import http from "node:http";
import crypto from "node:crypto";
import { WebSocketServer, WebSocket, type RawData } from "ws";
import { ReplayStore } from "./replays.js";

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
type Reason = "not-found" | "full" | "version" | "bad-request" | "busy" | "disabled";
type Kind = "invite" | "reconnect";
type Visibility = "off" | "code-only" | "public";
type RoomStage = "lobby" | "loading" | "playing";

interface RoomMeta {
	visibility: Visibility;
	delay_secs: number;
	game: string;
	engine: string;
	content_hash: string;
	rules: string;
	names: [string, string];
	stage: RoomStage;
	score: [number, number];
}

interface SpectateChunk {
	from_tick: number;
	count: number;
	data_b64: string;
	releaseAt: number;
	released: boolean;
}

interface RoundCheckpoint {
	from_tick: number;
	stage: number;
	background: number;
	effects: number;
	wins: number;
	losses: number;
	round: number;
	max_combo: number;
	console_rng: [number, number];
	console_frames: [number, number];
	prev_inputs: [number, number];
	prev_delays: [number, number];
}

interface DelayedCheckpoint {
	checkpoint: RoundCheckpoint;
	releaseAt: number;
	released: boolean;
}

interface SpectateSet {
	setId: number;
	headerB64: string;
	releaseAt: number;
	released: boolean;
	checkpoints: DelayedCheckpoint[];
	chunks: SpectateChunk[];
	ended: boolean;
	endReason: string;
	endReleaseAt?: number;
	endReleased: boolean;
}

interface SpectateRoom {
	code: string;
	hostPeer: string;
	hostWs?: WebSocket;
	hostGraceTimer?: NodeJS.Timeout;
	meta: RoomMeta;
	watchers: Map<string, WebSocket>;
	currentSet?: SpectateSet;
	lastSet?: SpectateSet;
}

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
	joinReleaseTimer?: NodeJS.Timeout;
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
const JOIN_GRACE_MS = Math.max(0, Number(process.env.JOIN_GRACE_MS ?? String(10 * 1000)));
const WS_HEARTBEAT_INTERVAL_MS = Math.max(1000, Number(process.env.WS_HEARTBEAT_INTERVAL_MS ?? String(15 * 1000)));
const WS_IDLE_TIMEOUT_MS = Math.max(5000, Number(process.env.WS_IDLE_TIMEOUT_MS ?? String(45 * 1000)));
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
	disabled: 4006,
};

const MAX_SPECTATORS = Number(process.env.MAX_SPECTATORS ?? "30");
const SPECTATE_HOST_GRACE_MS = 30 * 1000;
const MAX_DELAY_SECS = 60;
const MAX_HEADER_B64_BYTES = 128 * 1024;
const MAX_CHUNK_B64_BYTES = 32 * 1024;
const MAX_CHUNKS_PER_SET = 20_000;

const PORT = Number(process.env.PORT ?? "8787");
const HOST = process.env.HOST ?? "0.0.0.0";
const STUN_URLS = (process.env.STUN_URLS ?? "stun:stun.miwifi.com:3478")
	.split(/[\s,]+/)
	.filter(Boolean);
const TURN_URLS = (process.env.TURN_URLS ?? "")
	.split(/[\s,]+/)
	.filter(Boolean);
const TURN_SECRET = process.env.TURN_SECRET ?? "";
const REPLAYS_DIR = process.env.REPLAYS_DIR ?? "/data/replays";
const REPLAYS_MAX_AGE_DAYS = Number(process.env.REPLAYS_MAX_AGE_DAYS ?? "7");
const REPLAYS_MAX_COUNT = Number(process.env.REPLAYS_MAX_COUNT ?? "200");
const REPLAYS_MAX_BYTES = Number(process.env.REPLAYS_MAX_BYTES ?? String(512 * 1024));
const REPLAYS_UPLOAD_LIMIT_PER_MINUTE = Number(process.env.REPLAYS_UPLOAD_LIMIT_PER_MINUTE ?? "10");

const replayStore = new ReplayStore({
	dir: REPLAYS_DIR,
	maxAgeDays: REPLAYS_MAX_AGE_DAYS,
	maxCount: REPLAYS_MAX_COUNT,
	maxBytes: REPLAYS_MAX_BYTES,
	uploadLimitPerMinute: REPLAYS_UPLOAD_LIMIT_PER_MINUTE,
});

const rooms = new Map<string, RoomState>();
const metaBySocket = new WeakMap<WebSocket, PeerMeta>();
const lastSeenBySocket = new WeakMap<WebSocket, number>();
const rateBuckets = new Map<string, number[]>();
const spectateRooms = new Map<string, SpectateRoom>();
const listSubscribers = new Set<WebSocket>();

function touchSocket(ws: WebSocket): void {
	lastSeenBySocket.set(ws, Date.now());
}

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
			forgetRoom(room);
		}
	}, FORGET_AFTER_MS[room.kind]);
}

function cancelForget(room: RoomState): void {
	if (room.forgetTimer) {
		clearTimeout(room.forgetTimer);
		room.forgetTimer = undefined;
	}
}

function cancelJoinRelease(room: RoomState): void {
	if (room.joinReleaseTimer) {
		clearTimeout(room.joinReleaseTimer);
		room.joinReleaseTimer = undefined;
	}
}

function scheduleJoinRelease(room: RoomState, peer: string): void {
	if (room.kind !== "invite") return;
	cancelJoinRelease(room);
	room.joinReleaseTimer = setTimeout(() => {
		room.joinReleaseTimer = undefined;
		if (rooms.get(room.key) !== room) return;
		if (room.sockets.join?.readyState === WebSocket.OPEN) return;
		if (room.places.join !== peer) return;
		delete room.places.join;
		if (room.places.host === undefined && room.places.join === undefined) {
			forgetRoom(room);
		}
	}, JOIN_GRACE_MS);
}

function forgetRoom(room: RoomState): void {
	cancelForget(room);
	cancelJoinRelease(room);
	rooms.delete(room.key);
}

// Periodically ping open signaling & spectate-host sockets and terminate
// half-open zombies (e.g. mobile sleep / Wi-Fi drop without TCP FIN/RST).
setInterval(() => {
	const now = Date.now();
	const check = (ws: WebSocket | undefined) => {
		if (!ws || ws.readyState !== WebSocket.OPEN) return;
		const seen = lastSeenBySocket.get(ws) ?? now;
		if (now - seen >= WS_IDLE_TIMEOUT_MS) {
			try {
				ws.terminate();
			} catch {
				// Ignore
			}
			return;
		}
		try {
			ws.ping();
		} catch {
			// Ignore
		}
	};
	for (const room of rooms.values()) {
		check(room.sockets.host);
		check(room.sockets.join);
	}
	for (const room of spectateRooms.values()) {
		check(room.hostWs);
	}
}, WS_HEARTBEAT_INTERVAL_MS).unref();

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

function defaultMeta(): RoomMeta {
	return {
		visibility: "code-only",
		delay_secs: 0,
		game: "",
		engine: "",
		content_hash: "",
		rules: "",
		names: ["", ""],
		stage: "lobby",
		score: [0, 0],
	};
}

function sanitizeMeta(raw: unknown): RoomMeta {
	const base = defaultMeta();
	if (!raw || typeof raw !== "object") return base;
	const m = raw as Record<string, unknown>;
	if (m.visibility === "off" || m.visibility === "code-only" || m.visibility === "public") {
		base.visibility = m.visibility;
	}
	if (typeof m.delay_secs === "number" && Number.isFinite(m.delay_secs)) {
		base.delay_secs = Math.max(0, Math.min(MAX_DELAY_SECS, Math.floor(m.delay_secs)));
	}
	if (typeof m.game === "string") base.game = m.game.slice(0, 32);
	if (typeof m.engine === "string") base.engine = m.engine.slice(0, 32);
	if (typeof m.content_hash === "string") base.content_hash = m.content_hash.slice(0, 32);
	if (typeof m.rules === "string") base.rules = m.rules.slice(0, 64);
	if (Array.isArray(m.names) && m.names.length === 2) {
		base.names = [
			typeof m.names[0] === "string" ? m.names[0].slice(0, 48) : "",
			typeof m.names[1] === "string" ? m.names[1].slice(0, 48) : "",
		];
	}
	if (m.stage === "lobby" || m.stage === "loading" || m.stage === "playing") {
		base.stage = m.stage;
	}
	if (Array.isArray(m.score) && m.score.length === 2) {
		base.score = [
			typeof m.score[0] === "number" ? Math.max(0, Math.min(99, Math.floor(m.score[0]))) : 0,
			typeof m.score[1] === "number" ? Math.max(0, Math.min(99, Math.floor(m.score[1]))) : 0,
		];
	}
	return base;
}

function roomSummary(room: SpectateRoom) {
	return {
		code: room.code,
		...room.meta,
		spectators: room.watchers.size,
	};
}

function publicRoomsList() {
	const stageRank: Record<RoomStage, number> = { playing: 0, loading: 1, lobby: 2 };
	return Array.from(spectateRooms.values())
		.filter((r) => r.meta.visibility === "public")
		.sort((a, b) => {
			const sd = stageRank[a.meta.stage] - stageRank[b.meta.stage];
			if (sd !== 0) return sd;
			const wd = b.watchers.size - a.watchers.size;
			if (wd !== 0) return wd;
			return a.code.localeCompare(b.code);
		})
		.map(roomSummary);
}

function broadcastPublicList(): void {
	if (listSubscribers.size === 0) return;
	const payload = JSON.stringify({ type: "rooms", rooms: publicRoomsList() });
	for (const ws of listSubscribers) {
		tell(ws, payload);
	}
}

function broadcastWatchers(room: SpectateRoom, payload: string): void {
	for (const ws of room.watchers.values()) {
		tell(ws, payload);
	}
}

function notifySpectatorCount(room: SpectateRoom): void {
	const count = room.watchers.size;
	const msg = JSON.stringify({ type: "spectators", count });
	tell(room.hostWs, msg);
	broadcastWatchers(room, msg);
	if (room.meta.visibility === "public") {
		broadcastPublicList();
	}
}

function kickAllWatchers(room: SpectateRoom, reason: string): void {
	const msg = JSON.stringify({ type: "closed", reason });
	for (const ws of room.watchers.values()) {
		tell(ws, msg);
		try {
			ws.close(1000, reason);
		} catch {
			// Ignore
		}
	}
	room.watchers.clear();
}

function closeSpectateRoom(room: SpectateRoom, reason: string): void {
	if (room.hostGraceTimer) {
		clearTimeout(room.hostGraceTimer);
		room.hostGraceTimer = undefined;
	}
	const wasPublic = room.meta.visibility === "public";
	spectateRooms.delete(room.code);
	kickAllWatchers(room, reason);
	if (wasPublic) {
		broadcastPublicList();
	}
}

function flushDelayedSet(room: SpectateRoom, now: number): void {
	const set = room.currentSet;
	if (!set) return;
	if (!set.released) {
		if (now < set.releaseAt) return;
		set.released = true;
		broadcastWatchers(room, JSON.stringify({ type: "set_start", set_id: set.setId, header_b64: set.headerB64 }));
	}
	for (const cp of set.checkpoints) {
		if (!cp.released) {
			if (now < cp.releaseAt) break;
			cp.released = true;
		}
	}
	for (const chunk of set.chunks) {
		if (!chunk.released) {
			if (now < chunk.releaseAt) return;
			chunk.released = true;
			broadcastWatchers(
				room,
				JSON.stringify({
					type: "ticks",
					set_id: set.setId,
					from_tick: chunk.from_tick,
					count: chunk.count,
					data_b64: chunk.data_b64,
				}),
			);
		}
	}
	if (set.ended && !set.endReleased && (set.endReleaseAt === undefined || now >= set.endReleaseAt)) {
		set.endReleased = true;
		broadcastWatchers(room, JSON.stringify({ type: "set_end", set_id: set.setId, reason: set.endReason }));
	}
}

function buildSetSnapshot(room: SpectateRoom) {
	const set = room.currentSet;
	if (!set || !set.released || set.endReleased) return undefined;
	let latestCheckpoint: RoundCheckpoint | undefined;
	for (const cp of set.checkpoints) {
		if (cp.released) {
			latestCheckpoint = cp.checkpoint;
		}
	}
	const minTick = latestCheckpoint ? latestCheckpoint.from_tick : 0;
	return {
		set_id: set.setId,
		header_b64: set.headerB64,
		...(latestCheckpoint ? { checkpoint: latestCheckpoint } : {}),
		chunks: set.chunks
			.filter((c) => c.released && c.from_tick >= minTick)
			.map((c) => ({ from_tick: c.from_tick, count: c.count, data_b64: c.data_b64 })),
	};
}

// Periodically release delayed spectator frames
setInterval(() => {
	const now = Date.now();
	for (const room of spectateRooms.values()) {
		if (room.currentSet) {
			flushDelayedSet(room, now);
		}
	}
}, 50).unref();

function geoCountry(req: http.IncomingMessage): string {
	const raw = req.headers["cf-ipcountry"];
	const val = (Array.isArray(raw) ? raw[0] : raw ?? "").trim().toUpperCase();
	if (!val || val === "XX" || !/^[A-Z0-9]{2}$/.test(val)) {
		return "";
	}
	return val;
}

const server = http.createServer(async (req, res) => {
	const url = new URL(req.url ?? "/", `http://${req.headers.host ?? "localhost"}`);
	if (url.pathname === "/health") {
		res.writeHead(200, { "Content-Type": "text/plain" });
		res.end("ok\n");
		return;
	}
	if (url.pathname === "/geo" && (req.method === "GET" || req.method === "HEAD")) {
		const country = geoCountry(req);
		const headers = {
			"Content-Type": "application/json; charset=utf-8",
			"Access-Control-Allow-Origin": "*",
			"Cache-Control": "no-store",
		};
		if (!country) {
			res.writeHead(503, headers);
			res.end(req.method === "HEAD" ? undefined : JSON.stringify({ error: "geolocation unavailable" }));
			return;
		}
		const body = JSON.stringify({ country, mainland: country === "CN" });
		res.writeHead(200, headers);
		res.end(req.method === "HEAD" ? undefined : body);
		return;
	}
	if (url.pathname === "/spectate/rooms" && (req.method === "GET" || req.method === "HEAD")) {
		const body = JSON.stringify({ rooms: publicRoomsList() });
		res.writeHead(200, {
			"Content-Type": "application/json; charset=utf-8",
			"Access-Control-Allow-Origin": "*",
			"Cache-Control": "no-store",
		});
		res.end(req.method === "HEAD" ? undefined : body);
		return;
	}
	if (await replayStore.handleHttp(req, res, url, clientIp(req))) {
		return;
	}
	const invite = url.pathname.match(/^\/rooms(?:\/([^/]*))?\/?$/);
	const reconnect = url.pathname.match(/^\/reconnect\/([^/]*)\/?$/);
	const spectate = url.pathname.match(/^\/spectate\/(host|watch|list)(?:\/([^/]*))?\/?$/);
	if (!invite && !reconnect && !spectate) {
		res.writeHead(404, { "Content-Type": "text/plain" });
		res.end("a new room is /rooms?peer=ID&v=1, a room /rooms/CODE?peer=ID&v=1, a reconnect room /reconnect/CODE?peer=ID&v=1\n");
		return;
	}
	res.writeHead(426, { "Content-Type": "text/plain" });
	res.end("a room is a WebSocket\n");
});

function handleSpectateListUpgrade(req: http.IncomingMessage, socket: import("node:stream").Duplex, head: Buffer): void {
	wss.handleUpgrade(req, socket, head, (ws) => {
		listSubscribers.add(ws);
		tell(ws, JSON.stringify({ type: "rooms", rooms: publicRoomsList() }));
		ws.on("message", (data: RawData, isBinary: boolean) => {
			if (isBinary) return;
			const text = typeof data === "string" ? data : data.toString("utf8");
			if (text === "ping") {
				tell(ws, "pong");
			} else if (text === "refresh") {
				tell(ws, JSON.stringify({ type: "rooms", rooms: publicRoomsList() }));
			}
		});
		const cleanup = () => {
			listSubscribers.delete(ws);
		};
		ws.on("close", cleanup);
		ws.on("error", cleanup);
	});
}

function handleSpectateHostUpgrade(
	req: http.IncomingMessage,
	socket: import("node:stream").Duplex,
	head: Buffer,
	rawCode: string,
	peer: string,
): void {
	const code = rawCode.toUpperCase();
	if (!CODE.test(code)) {
		refuseUpgrade(req, socket, head, "bad-request", "invalid room code");
		return;
	}
	let room = spectateRooms.get(code);
	if (room && room.hostPeer !== peer) {
		refuseUpgrade(req, socket, head, "full", "room already has a spectator host");
		return;
	}
	if (!room) {
		room = {
			code,
			hostPeer: peer,
			meta: defaultMeta(),
			watchers: new Map(),
		};
		spectateRooms.set(code, room);
	}
	if (room.hostGraceTimer) {
		clearTimeout(room.hostGraceTimer);
		room.hostGraceTimer = undefined;
	}
	const targetRoom = room;
	wss.handleUpgrade(req, socket, head, (ws) => {
		if (targetRoom.hostWs && targetRoom.hostWs !== ws) {
			try {
				targetRoom.hostWs.close(4001, "replaced by a new connection");
			} catch {
				// Ignore
			}
		}
		targetRoom.hostWs = ws;
		touchSocket(ws);
		ws.on("pong", () => touchSocket(ws));
		tell(ws, JSON.stringify({ type: "welcome", code: targetRoom.code, spectators: targetRoom.watchers.size }));

		ws.on("message", (data: RawData, isBinary: boolean) => {
			touchSocket(ws);
			if (isBinary) return;
			const text = typeof data === "string" ? data : data.toString("utf8");
			if (text === "ping") {
				tell(ws, "pong");
				return;
			}
			let msg: Record<string, unknown>;
			try {
				msg = JSON.parse(text);
			} catch {
				return;
			}
			if (spectateRooms.get(code) !== targetRoom) return;
			const now = Date.now();
			const delayMs = targetRoom.meta.delay_secs * 1000;

			if (msg.type === "config") {
				const prevVisibility = targetRoom.meta.visibility;
				targetRoom.meta = sanitizeMeta(msg.meta);
				if (targetRoom.meta.visibility === "off") {
					if (targetRoom.watchers.size > 0) {
						kickAllWatchers(targetRoom, "disabled");
						tell(ws, JSON.stringify({ type: "spectators", count: 0 }));
					}
				} else {
					broadcastWatchers(targetRoom, JSON.stringify({ type: "room_update", room: roomSummary(targetRoom) }));
				}
				if (prevVisibility === "public" || targetRoom.meta.visibility === "public") {
					broadcastPublicList();
				}
			} else if (msg.type === "set_start") {
				const setId = typeof msg.set_id === "number" ? Math.floor(msg.set_id) : 0;
				const headerB64 = typeof msg.header_b64 === "string" ? msg.header_b64 : "";
				if (!headerB64 || headerB64.length > MAX_HEADER_B64_BYTES) return;
				if (targetRoom.currentSet) {
					if (targetRoom.currentSet.released && !targetRoom.currentSet.endReleased) {
						targetRoom.currentSet.endReleased = true;
						broadcastWatchers(
							targetRoom,
							JSON.stringify({ type: "set_end", set_id: targetRoom.currentSet.setId, reason: "next-set" }),
						);
					}
					targetRoom.lastSet = targetRoom.currentSet;
				}
				const released = delayMs === 0;
				targetRoom.currentSet = {
					setId,
					headerB64,
					releaseAt: now + delayMs,
					released,
					checkpoints: [],
					chunks: [],
					ended: false,
					endReason: "",
					endReleased: false,
				};
				if (released) {
					broadcastWatchers(targetRoom, JSON.stringify({ type: "set_start", set_id: setId, header_b64: headerB64 }));
				}
			} else if (msg.type === "round_checkpoint") {
				const set = targetRoom.currentSet;
				const setId = typeof msg.set_id === "number" ? Math.floor(msg.set_id) : -1;
				const cp = msg.checkpoint as Partial<RoundCheckpoint> | undefined;
				if (!set || set.setId !== setId || set.ended || !cp || typeof cp !== "object") return;
				if (
					typeof cp.from_tick !== "number" ||
					typeof cp.stage !== "number" ||
					typeof cp.background !== "number" ||
					typeof cp.effects !== "number" ||
					typeof cp.wins !== "number" ||
					typeof cp.losses !== "number" ||
					typeof cp.round !== "number" ||
					typeof cp.max_combo !== "number" ||
					!Array.isArray(cp.console_rng) ||
					cp.console_rng.length !== 2 ||
					!Array.isArray(cp.console_frames) ||
					cp.console_frames.length !== 2 ||
					!Array.isArray(cp.prev_inputs) ||
					cp.prev_inputs.length !== 2 ||
					!Array.isArray(cp.prev_delays) ||
					cp.prev_delays.length !== 2
				) {
					return;
				}
				if (set.checkpoints.length >= 64) return;
				const sanitized: RoundCheckpoint = {
					from_tick: Math.max(0, Math.floor(cp.from_tick)),
					stage: Math.max(0, Math.floor(cp.stage)),
					background: Math.max(0, Math.floor(cp.background)),
					effects: Math.max(0, Math.floor(cp.effects)),
					wins: Math.max(0, Math.min(255, Math.floor(cp.wins))),
					losses: Math.max(0, Math.min(255, Math.floor(cp.losses))),
					round: Math.max(0, Math.min(255, Math.floor(cp.round))),
					max_combo: Math.max(0, Math.min(255, Math.floor(cp.max_combo))),
					console_rng: [Math.floor(cp.console_rng[0]) >>> 0, Math.floor(cp.console_rng[1]) >>> 0],
					console_frames: [Math.floor(cp.console_frames[0]) >>> 0, Math.floor(cp.console_frames[1]) >>> 0],
					prev_inputs: [Math.floor(cp.prev_inputs[0]) >>> 0, Math.floor(cp.prev_inputs[1]) >>> 0],
					prev_delays: [Math.max(0, Math.min(255, Math.floor(cp.prev_delays[0]))), Math.max(0, Math.min(255, Math.floor(cp.prev_delays[1])))],
				};
				const released = delayMs === 0 && set.released;
				set.checkpoints.push({
					checkpoint: sanitized,
					releaseAt: now + delayMs,
					released,
				});
			} else if (msg.type === "ticks") {
				const set = targetRoom.currentSet;
				const setId = typeof msg.set_id === "number" ? Math.floor(msg.set_id) : -1;
				const fromTick = typeof msg.from_tick === "number" ? Math.floor(msg.from_tick) : 0;
				const count = typeof msg.count === "number" ? Math.floor(msg.count) : 0;
				const dataB64 = typeof msg.data_b64 === "string" ? msg.data_b64 : "";
				if (!set || set.setId !== setId || set.ended) return;
				if (count <= 0 || dataB64.length > MAX_CHUNK_B64_BYTES || set.chunks.length >= MAX_CHUNKS_PER_SET) return;
				const released = delayMs === 0 && set.released;
				set.chunks.push({
					from_tick: fromTick,
					count,
					data_b64: dataB64,
					releaseAt: now + delayMs,
					released,
				});
				if (released) {
					broadcastWatchers(
						targetRoom,
						JSON.stringify({ type: "ticks", set_id: setId, from_tick: fromTick, count, data_b64: dataB64 }),
					);
				}
			} else if (msg.type === "set_end") {
				const set = targetRoom.currentSet;
				const setId = typeof msg.set_id === "number" ? Math.floor(msg.set_id) : -1;
				const reason = typeof msg.reason === "string" ? msg.reason.slice(0, 64) : "";
				if (!set || set.setId !== setId || set.ended) return;
				set.ended = true;
				set.endReason = reason;
				set.endReleaseAt = now + delayMs;
				flushDelayedSet(targetRoom, now);
			} else if (msg.type === "close") {
				closeSpectateRoom(targetRoom, "host-left");
				try {
					ws.close(1000, "closed");
				} catch {
					// Ignore
				}
			}
		});

		const handleGone = () => {
			if (targetRoom.hostWs !== ws) return;
			targetRoom.hostWs = undefined;
			if (spectateRooms.get(code) !== targetRoom) return;
			targetRoom.hostGraceTimer = setTimeout(() => {
				if (targetRoom.hostWs === undefined && spectateRooms.get(code) === targetRoom) {
					closeSpectateRoom(targetRoom, "host-disconnected");
				}
			}, SPECTATE_HOST_GRACE_MS);
		};
		ws.on("close", handleGone);
		ws.on("error", handleGone);
	});
}

function handleSpectateWatchUpgrade(
	req: http.IncomingMessage,
	socket: import("node:stream").Duplex,
	head: Buffer,
	rawCode: string,
	peer: string,
): void {
	const code = rawCode.toUpperCase();
	if (!CODE.test(code)) {
		refuseUpgrade(req, socket, head, "not-found", "no room has that code");
		return;
	}
	const room = spectateRooms.get(code);
	if (!room) {
		refuseUpgrade(req, socket, head, "not-found", "no spectatable room has that code");
		return;
	}
	if (room.meta.visibility === "off") {
		refuseUpgrade(req, socket, head, "disabled", "spectating is disabled for this room");
		return;
	}
	const existing = room.watchers.get(peer);
	if (!existing && room.watchers.size >= MAX_SPECTATORS) {
		refuseUpgrade(req, socket, head, "full", "the spectator limit has been reached");
		return;
	}
	wss.handleUpgrade(req, socket, head, (ws) => {
		if (existing && existing !== ws) {
			try {
				existing.close(4001, "replaced by a new connection");
			} catch {
				// Ignore
			}
		}
		room.watchers.set(peer, ws);
		flushDelayedSet(room, Date.now());
		const setSnap = buildSetSnapshot(room);
		tell(
			ws,
			JSON.stringify({
				type: "snapshot",
				room: roomSummary(room),
				...(setSnap ? { set: setSnap } : {}),
			}),
		);
		notifySpectatorCount(room);

		ws.on("message", (data: RawData, isBinary: boolean) => {
			if (isBinary) return;
			const text = typeof data === "string" ? data : data.toString("utf8");
			if (text === "ping") {
				tell(ws, "pong");
				return;
			}
			let msg: Record<string, unknown>;
			try {
				msg = JSON.parse(text);
			} catch {
				return;
			}
			if (msg.type === "request_history") {
				const reqSetId = typeof msg.set_id === "number" ? Math.floor(msg.set_id) : -1;
				flushDelayedSet(room, Date.now());
				const targetSet =
					room.currentSet && room.currentSet.setId === reqSetId
						? room.currentSet
						: room.lastSet && room.lastSet.setId === reqSetId
							? room.lastSet
							: undefined;
				if (!targetSet || !targetSet.released) return;
				const releasedChunks = targetSet.chunks
					.filter((c) => c.released)
					.map((c) => ({ from_tick: c.from_tick, count: c.count, data_b64: c.data_b64 }));
				tell(
					ws,
					JSON.stringify({
						type: "set_history",
						set_id: targetSet.setId,
						header_b64: targetSet.headerB64,
						chunks: releasedChunks,
					}),
				);
			}
		});
		const handleGone = () => {
			if (room.watchers.get(peer) === ws) {
				room.watchers.delete(peer);
				if (spectateRooms.get(code) === room) {
					notifySpectatorCount(room);
				}
			}
		};
		ws.on("close", handleGone);
		ws.on("error", handleGone);
	});
}

server.on("upgrade", (req, socket, head) => {
	const url = new URL(req.url ?? "/", `http://${req.headers.host ?? "localhost"}`);
	const spectate = url.pathname.match(/^\/spectate\/(host|watch|list)(?:\/([^/]*))?\/?$/);
	if (spectate) {
		if (url.searchParams.get("v") !== VERSION) {
			refuseUpgrade(req, socket, head, "version", `this server speaks version ${VERSION} of the signaling protocol`);
			return;
		}
		const action = spectate[1];
		if (action === "list") {
			handleSpectateListUpgrade(req, socket, head);
			return;
		}
		const peer = url.searchParams.get("peer") ?? "";
		if (!PEER.test(peer)) {
			refuseUpgrade(req, socket, head, "bad-request", "a peer's ID is 1 to 64 letters, digits, _ and -");
			return;
		}
		const rawCode = spectate[2] ?? "";
		if (action === "host") {
			handleSpectateHostUpgrade(req, socket, head, rawCode, peer);
		} else {
			handleSpectateWatchUpgrade(req, socket, head, rawCode, peer);
		}
		return;
	}
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

	const existingRoom = rooms.get(roomKey);
	if (!creatingInvite) {
		if (!existingRoom && kind === "invite") {
			refuseUpgrade(req, socket, head, "not-found", "no room has that code");
			return;
		}
		if (existingRoom) {
			const roles: Role[] = ["host", "join"];
			const preRole =
				roles.find((r) => existingRoom.places[r] === peer) ??
				roles.find((r) => existingRoom.places[r] === undefined);
			if (preRole === undefined) {
				refuseUpgrade(req, socket, head, "full", "the room is full");
				return;
			}
		}
	}

	const client = clientIp(req);

	wss.handleUpgrade(req, socket, head, (ws) => {
		let room = rooms.get(roomKey);
		if (creatingInvite) {
			if (room) {
				tell(ws, JSON.stringify({ type: "error", reason: "busy", message: "room code collision" }));
				ws.close(CLOSE.busy, "room code collision");
				return;
			}
			room = { key: roomKey, kind, code, places: {}, sockets: {} };
			rooms.set(roomKey, room);
		} else if (!room) {
			if (kind === "invite") {
				tell(ws, JSON.stringify({ type: "error", reason: "not-found", message: "no room has that code" }));
				ws.close(CLOSE["not-found"], "no room has that code");
				return;
			}
			room = { key: roomKey, kind, code, places: {}, sockets: {} };
			rooms.set(roomKey, room);
		}

		const roles: Role[] = ["host", "join"];
		const role = roles.find((r) => room.places[r] === peer) ?? roles.find((r) => room.places[r] === undefined);
		if (role === undefined) {
			tell(ws, JSON.stringify({ type: "error", reason: "full", message: "the room is full" }));
			ws.close(CLOSE.full, "the room is full");
			return;
		}

		room.places[role] = peer;
		cancelForget(room);
		if (role === "join") {
			cancelJoinRelease(room);
		}

		const both = room.places.host !== undefined && room.places.join !== undefined;
		const otherRole = other(role);
		const otherWs = room.sockets[otherRole];
		const otherOpen = otherWs?.readyState === WebSocket.OPEN;

		const ice = both ? buildIceServers(client, room.key, role) : undefined;
		const theirs = both && otherOpen ? buildIceServers(client, room.key, otherRole) : undefined;

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
		touchSocket(ws);
		ws.on("pong", () => touchSocket(ws));

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
			touchSocket(ws);
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
				if (meta.role === "join") {
					cancelJoinRelease(currentRoom);
				}
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
				if (currentRoom.kind === "invite" && meta.role === "join" && currentRoom.places.join === peer) {
					scheduleJoinRelease(currentRoom, peer);
				}
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
	console.log(
		`saito-signal listening on ${HOST}:${PORT} (STUN=${STUN_URLS.join(",")}, TURN=${TURN_URLS.join(",") || "disabled"}, REPLAYS=${replayStore.isEnabled() ? REPLAYS_DIR : "disabled"})`,
	);
});
