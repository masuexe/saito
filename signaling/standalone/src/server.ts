import http from "node:http";
import crypto from "node:crypto";
import { WebSocketServer, WebSocket, type RawData } from "ws";

/**
 * Standalone saito signaling server (v1), protocol-compatible with
 * signaling/src/index.ts (Cloudflare Worker).
 */

interface IceServer {
	urls: string[];
	username?: string;
	credential?: string;
}

type Role = "host" | "join";
type Reason = "not-found" | "full" | "version" | "bad-request" | "busy";
type Kind = "invite" | "reconnect";

interface PeerMeta {
	roomKey: string;
	role: Role;
	replaced: boolean;
}

interface RoomState {
	key: string;
	kind: Kind;
	code: string;
	places: Partial<Record<Role, string>>;
	sockets: Partial<Record<Role, WebSocket>>;
	forgetTimer?: NodeJS.Timeout;
}

const VERSION = "1";
const ALPHABET = "ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const CODE_LENGTH = 6;
const CODE = /^[A-HJKMNP-Z2-9]{6}$/;
const RECONNECT_CODE = /^[0-9a-f]{32}$/;
const PEER = /^[A-Za-z0-9_-]{1,64}$/;
const FORGET_AFTER_MS: Record<Kind, number> = {
	invite: 10 * 60 * 1000,
	reconnect: 30 * 1000,
};
const CODE_TRIES = 8;
const TURN_TTL_SECONDS = 4 * 60 * 60;
const TURN_LIMIT_MAX = Number(process.env.TURN_LIMIT_PER_MINUTE ?? "10");
const TURN_LIMIT_WINDOW_MS = 60 * 1000;

const CLOSE: Record<Reason, number> = {
	full: 4000,
	version: 4002,
	"bad-request": 4003,
	"not-found": 4004,
	busy: 4005,
};

const PORT = Number(process.env.PORT ?? "8787");
const HOST = process.env.HOST ?? "0.0.0.0";
const STUN_URLS = (process.env.STUN_URLS ?? "stun:stun.miwifi.com:3478")
	.split(/[\s,]+/)
	.filter(Boolean);
const TURN_URLS = (process.env.TURN_URLS ?? "")
	.split(/[\s,]+/)
	.filter(Boolean);
const TURN_SECRET = process.env.TURN_SECRET ?? "";

const rooms = new Map<string, RoomState>();
const metaBySocket = new WeakMap<WebSocket, PeerMeta>();
const rateBuckets = new Map<string, number[]>();

function other(role: Role): Role {
	return role === "host" ? "join" : "host";
}

function newCode(): string {
	let code = "";
	while (code.length < CODE_LENGTH) {
		const bytes = crypto.randomBytes(2 * CODE_LENGTH);
		for (const b of bytes) {
			if (b < 248 && code.length < CODE_LENGTH) {
				code += ALPHABET[b % ALPHABET.length];
			}
		}
	}
	return code;
}

function clientIp(req: http.IncomingMessage): string {
	const xff = req.headers["x-forwarded-for"];
	const raw = Array.isArray(xff) ? xff[0] : xff?.split(",")[0]?.trim() || req.socket.remoteAddress || "";
	const ip = raw.startsWith("::ffff:") ? raw.slice(7) : raw;
	if (!ip.includes(":") || ip.includes(".")) {
		return ip;
	}
	const [head, tail] = ip.split("::");
	const front = head ? head.split(":") : [];
	const back = tail ? tail.split(":") : [];
	const groups =
		tail === undefined
			? front
			: [...front, ...Array(Math.max(0, 8 - front.length - back.length)).fill("0"), ...back];
	return `${groups
		.slice(0, 4)
		.map((g) => parseInt(g || "0", 16).toString(16))
		.join(":")}::/64`;
}

function allowTurn(client: string): boolean {
	if (TURN_LIMIT_MAX <= 0) return true;
	const now = Date.now();
	const cutoff = now - TURN_LIMIT_WINDOW_MS;
	const timestamps = (rateBuckets.get(client) ?? []).filter((t) => t > cutoff);
	if (timestamps.length >= TURN_LIMIT_MAX) {
		rateBuckets.set(client, timestamps);
		return false;
	}
	timestamps.push(now);
	rateBuckets.set(client, timestamps);
	return true;
}

// Periodically prune stale rate-limit entries
setInterval(() => {
	const cutoff = Date.now() - TURN_LIMIT_WINDOW_MS;
	for (const [k, list] of rateBuckets) {
		const kept = list.filter((t) => t > cutoff);
		if (kept.length === 0) {
			rateBuckets.delete(k);
		} else {
			rateBuckets.set(k, kept);
		}
	}
}, 5 * 60 * 1000).unref();

function matchTag(secret: string, roomKey: string): string {
	return crypto.createHmac("sha256", secret).update(roomKey).digest().subarray(0, 6).toString("hex");
}

function buildIceServers(client: string, roomKey: string, role: Role): IceServer[] {
	const base: IceServer[] = STUN_URLS.length > 0 ? [{ urls: STUN_URLS }] : [];
	if (TURN_URLS.length === 0 || !TURN_SECRET) {
		return base;
	}
	if (!allowTurn(client)) {
		console.warn(`no TURN credentials: rate limit reached for ${client}`);
		return base;
	}
	const match = matchTag(TURN_SECRET, roomKey);
	const expiry = Math.floor(Date.now() / 1000) + TURN_TTL_SECONDS;
	const username = `${expiry}:${match}-${role}`;
	const credential = crypto.createHmac("sha1", TURN_SECRET).update(username).digest("base64");
	return [...base, { urls: TURN_URLS, username, credential }];
}

function tell(ws: WebSocket | undefined, text: string): void {
	if (!ws || ws.readyState !== WebSocket.OPEN) return;
	try {
		ws.send(text);
	} catch {
		// Socket closed concurrently
	}
}

function scheduleForget(room: RoomState): void {
	if (room.forgetTimer) {
		clearTimeout(room.forgetTimer);
	}
	room.forgetTimer = setTimeout(() => {
		const hostOpen = room.sockets.host?.readyState === WebSocket.OPEN;
		const joinOpen = room.sockets.join?.readyState === WebSocket.OPEN;
		if (!hostOpen && !joinOpen) {
			rooms.delete(room.key);
		}
	}, FORGET_AFTER_MS[room.kind]);
}

function cancelForget(room: RoomState): void {
	if (room.forgetTimer) {
		clearTimeout(room.forgetTimer);
		room.forgetTimer = undefined;
	}
}

function forgetRoom(room: RoomState): void {
	cancelForget(room);
	rooms.delete(room.key);
}

const wss = new WebSocketServer({ noServer: true });

function refuseUpgrade(req: http.IncomingMessage, socket: import("node:stream").Duplex, head: Buffer, reason: Reason, message: string): void {
	wss.handleUpgrade(req, socket, head, (ws) => {
		tell(ws, JSON.stringify({ type: "error", reason, message }));
		ws.close(CLOSE[reason], message);
	});
}

function rejectHttp(socket: import("node:stream").Duplex, status: number, body: string): void {
	socket.write(
		`HTTP/1.1 ${status} ${http.STATUS_CODES[status] ?? ""}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: ${Buffer.byteLength(body)}\r\nConnection: close\r\n\r\n${body}`,
	);
	socket.destroy();
}

const server = http.createServer((req, res) => {
	const url = new URL(req.url ?? "/", `http://${req.headers.host ?? "localhost"}`);
	if (url.pathname === "/health") {
		res.writeHead(200, { "Content-Type": "text/plain" });
		res.end("ok\n");
		return;
	}
	const invite = url.pathname.match(/^\/rooms(?:\/([^/]*))?\/?$/);
	const reconnect = url.pathname.match(/^\/reconnect\/([^/]*)\/?$/);
	if (!invite && !reconnect) {
		res.writeHead(404, { "Content-Type": "text/plain" });
		res.end("a new room is /rooms?peer=ID&v=1, a room /rooms/CODE?peer=ID&v=1, a reconnect room /reconnect/CODE?peer=ID&v=1\n");
		return;
	}
	res.writeHead(426, { "Content-Type": "text/plain" });
	res.end("a room is a WebSocket\n");
});

server.on("upgrade", (req, socket, head) => {
	const url = new URL(req.url ?? "/", `http://${req.headers.host ?? "localhost"}`);
	const invite = url.pathname.match(/^\/rooms(?:\/([^/]*))?\/?$/);
	const reconnect = url.pathname.match(/^\/reconnect\/([^/]*)\/?$/);
	if (!invite && !reconnect) {
		rejectHttp(socket, 404, "not found\n");
		return;
	}
	if (url.searchParams.get("v") !== VERSION) {
		refuseUpgrade(req, socket, head, "version", `this server speaks version ${VERSION} of the signaling protocol`);
		return;
	}
	const peer = url.searchParams.get("peer") ?? "";
	if (!PEER.test(peer)) {
		refuseUpgrade(req, socket, head, "bad-request", "a peer's ID is 1 to 64 letters, digits, _ and -");
		return;
	}

	let kind: Kind;
	let code: string;
	let roomKey: string;
	let creatingInvite = false;

	if (reconnect) {
		kind = "reconnect";
		code = reconnect[1];
		if (!RECONNECT_CODE.test(code)) {
			refuseUpgrade(req, socket, head, "bad-request", "a reconnect code is 32 lowercase hex digits");
			return;
		}
		roomKey = `reconnect/${code}`;
	} else if (invite![1] === undefined || invite![1] === "") {
		kind = "invite";
		creatingInvite = true;
		let picked: string | undefined;
		for (let i = 0; i < CODE_TRIES; i++) {
			const candidate = newCode();
			if (!rooms.has(candidate)) {
				picked = candidate;
				break;
			}
		}
		if (!picked) {
			refuseUpgrade(req, socket, head, "busy", "no free code was found");
			return;
		}
		code = picked;
		roomKey = code;
	} else {
		kind = "invite";
		code = invite![1].toUpperCase();
		if (!CODE.test(code)) {
			refuseUpgrade(req, socket, head, "not-found", "no room has that code");
			return;
		}
		roomKey = code;
	}

	let room = rooms.get(roomKey);
	if (creatingInvite) {
		room = { key: roomKey, kind, code, places: {}, sockets: {} };
		rooms.set(roomKey, room);
	} else if (!room) {
		if (kind === "invite") {
			refuseUpgrade(req, socket, head, "not-found", "no room has that code");
			return;
		}
		room = { key: roomKey, kind, code, places: {}, sockets: {} };
		rooms.set(roomKey, room);
	}

	const roles: Role[] = ["host", "join"];
	const role = roles.find((r) => room.places[r] === peer) ?? roles.find((r) => room.places[r] === undefined);
	if (role === undefined) {
		refuseUpgrade(req, socket, head, "full", "the room is full");
		return;
	}

	room.places[role] = peer;
	cancelForget(room);

	const both = room.places.host !== undefined && room.places.join !== undefined;
	const client = clientIp(req);
	const otherRole = other(role);
	const otherWs = room.sockets[otherRole];
	const otherOpen = otherWs?.readyState === WebSocket.OPEN;

	const ice = both ? buildIceServers(client, room.key, role) : undefined;
	const theirs = both && otherOpen ? buildIceServers(client, room.key, otherRole) : undefined;

	wss.handleUpgrade(req, socket, head, (ws) => {
		const old = room.sockets[role];
		if (old && old !== ws) {
			const oldMeta = metaBySocket.get(old);
			if (oldMeta) oldMeta.replaced = true;
			try {
				old.close(4001, "replaced by a new connection");
			} catch {
				// Ignore
			}
		}

		room.sockets[role] = ws;
		metaBySocket.set(ws, { roomKey: room.key, role, replaced: false });

		const welcome =
			kind === "invite"
				? { type: "welcome", role, peer: otherOpen, code, ...(ice ? { iceServers: ice } : {}) }
				: { type: "welcome", peer: otherOpen, ...(ice ? { iceServers: ice } : {}) };
		tell(ws, JSON.stringify(welcome));

		if (otherOpen) {
			const present = { type: "peer", present: true, ...(theirs ? { iceServers: theirs } : {}) };
			tell(otherWs, JSON.stringify(present));
		}

		ws.on("message", (data: RawData, isBinary: boolean) => {
			if (isBinary) return;
			const text = typeof data === "string" ? data : data.toString("utf8");
			if (text === "ping") {
				tell(ws, "pong");
				return;
			}
			let said: { type?: string; data?: unknown };
			try {
				said = JSON.parse(text);
			} catch {
				return;
			}
			const meta = metaBySocket.get(ws);
			if (!meta) return;
			const currentRoom = rooms.get(meta.roomKey);
			if (!currentRoom) return;

			if (said.type === "signal") {
				tell(currentRoom.sockets[other(meta.role)], JSON.stringify({ type: "signal", data: said.data }));
			} else if (said.type === "leave") {
				delete currentRoom.places[meta.role];
				if (currentRoom.sockets[meta.role] === ws) {
					delete currentRoom.sockets[meta.role];
				}
				if (currentRoom.places.host === undefined && currentRoom.places.join === undefined) {
					forgetRoom(currentRoom);
				}
				if (!meta.replaced) {
					meta.replaced = true;
					tell(currentRoom.sockets[other(meta.role)], JSON.stringify({ type: "peer", present: false }));
				}
				ws.close(1000, "left");
			}
		});

		const handleGone = () => {
			const meta = metaBySocket.get(ws);
			if (!meta) return;
			const currentRoom = rooms.get(meta.roomKey);
			if (!currentRoom) return;

			if (currentRoom.sockets[meta.role] === ws) {
				delete currentRoom.sockets[meta.role];
			}
			if (!meta.replaced) {
				meta.replaced = true;
				tell(currentRoom.sockets[other(meta.role)], JSON.stringify({ type: "peer", present: false }));
			}
			const anyOpen =
				currentRoom.sockets.host?.readyState === WebSocket.OPEN ||
				currentRoom.sockets.join?.readyState === WebSocket.OPEN;
			if (!anyOpen && rooms.has(currentRoom.key)) {
				scheduleForget(currentRoom);
			}
		};

		ws.on("close", handleGone);
		ws.on("error", handleGone);
	});
});

server.listen(PORT, HOST, () => {
	console.log(`saito-signal listening on ${HOST}:${PORT} (STUN=${STUN_URLS.join(",")}, TURN=${TURN_URLS.join(",") || "disabled"})`);
});
