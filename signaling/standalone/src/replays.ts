import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import type http from "node:http";

export type ReplayResult = "p1" | "p2" | "draw" | "unfinished";

export interface ReplayRecord {
	id: string;
	game: string;
	engine: string;
	content_hash: string;
	played_at: number;
	uploaded_at: number;
	names: [string, string];
	score: [number, number];
	rounds: number;
	ticks: number;
	result: ReplayResult;
	size: number;
}

export interface ReplayStoreConfig {
	dir: string;
	maxAgeDays: number;
	maxCount: number;
	maxBytes: number;
	uploadLimitPerMinute: number;
}

const REPLAY_ID = /^[0-9a-f]{16,64}$/;
const REPLAY_MAGIC = Buffer.from([0x53, 0x54, 0x52, 0x50, 0x53]); // "STRP" + layout 0x53
const UPLOAD_WINDOW_MS = 60 * 1000;
const PRUNE_INTERVAL_MS = 30 * 60 * 1000;

const CORS_HEADERS: Record<string, string> = {
	"Access-Control-Allow-Origin": "*",
	"Access-Control-Allow-Methods": "GET, POST, OPTIONS",
	"Access-Control-Allow-Headers": "Content-Type",
};

function sendJson(res: http.ServerResponse, status: number, body: unknown): void {
	const text = JSON.stringify(body);
	res.writeHead(status, {
		...CORS_HEADERS,
		"Content-Type": "application/json; charset=utf-8",
		"Cache-Control": "no-store",
	});
	res.end(text);
}

function readBody(req: http.IncomingMessage, maxLen: number): Promise<Buffer> {
	return new Promise((resolve, reject) => {
		const chunks: Buffer[] = [];
		let total = 0;
		let done = false;
		req.on("data", (chunk: Buffer) => {
			if (done) return;
			total += chunk.length;
			if (total > maxLen) {
				done = true;
				reject(new Error("payload too large"));
				return;
			}
			chunks.push(chunk);
		});
		req.on("end", () => {
			if (done) return;
			done = true;
			resolve(Buffer.concat(chunks));
		});
		req.on("error", (err) => {
			if (done) return;
			done = true;
			reject(err);
		});
	});
}

function clampInt(raw: unknown, min: number, max: number, fallback: number): number {
	if (typeof raw !== "number" || !Number.isFinite(raw)) return fallback;
	return Math.max(min, Math.min(max, Math.floor(raw)));
}

function cleanStr(raw: unknown, maxLen: number): string {
	if (typeof raw !== "string") return "";
	return raw.replace(/[\u0000-\u001f\u007f]/g, "").trim().slice(0, maxLen);
}

function cleanResult(raw: unknown): ReplayResult {
	if (raw === "p1" || raw === "p2" || raw === "draw" || raw === "unfinished") {
		return raw;
	}
	return "unfinished";
}

export class ReplayStore {
	private readonly config: ReplayStoreConfig;
	private readonly enabled: boolean;
	private readonly indexPath: string;
	private records = new Map<string, ReplayRecord>();
	private uploadBuckets = new Map<string, number[]>();
	private pruneTimer?: NodeJS.Timeout;

	constructor(config: ReplayStoreConfig) {
		this.config = config;
		const trimmed = config.dir.trim();
		this.enabled = trimmed.length > 0 && trimmed.toLowerCase() !== "off";
		this.indexPath = path.join(trimmed, "index.json");
		if (this.enabled) {
			fs.mkdirSync(trimmed, { recursive: true });
			this.load();
			this.prune();
			this.pruneTimer = setInterval(() => this.prune(), PRUNE_INTERVAL_MS);
			this.pruneTimer.unref();
		}
	}

	isEnabled(): boolean {
		return this.enabled;
	}

	close(): void {
		if (this.pruneTimer) {
			clearInterval(this.pruneTimer);
			this.pruneTimer = undefined;
		}
	}

	private filePath(id: string): string {
		return path.join(this.config.dir, `${id}.saito`);
	}

	private load(): void {
		if (!fs.existsSync(this.indexPath)) return;
		try {
			const raw = fs.readFileSync(this.indexPath, "utf8");
			const parsed = JSON.parse(raw) as { replays?: unknown[] };
			if (!Array.isArray(parsed.replays)) return;
			for (const item of parsed.replays) {
				if (!item || typeof item !== "object") continue;
				const r = item as Partial<ReplayRecord>;
				if (typeof r.id !== "string" || !REPLAY_ID.test(r.id)) continue;
				if (!fs.existsSync(this.filePath(r.id))) continue;
				const names: [string, string] =
					Array.isArray(r.names) && r.names.length === 2
						? [cleanStr(r.names[0], 48), cleanStr(r.names[1], 48)]
						: ["", ""];
				const score: [number, number] =
					Array.isArray(r.score) && r.score.length === 2
						? [clampInt(r.score[0], 0, 99, 0), clampInt(r.score[1], 0, 99, 0)]
						: [0, 0];
				this.records.set(r.id, {
					id: r.id,
					game: cleanStr(r.game, 32),
					engine: cleanStr(r.engine, 32),
					content_hash: cleanStr(r.content_hash, 32),
					played_at: clampInt(r.played_at, 0, Number.MAX_SAFE_INTEGER, 0),
					uploaded_at: clampInt(r.uploaded_at, 0, Number.MAX_SAFE_INTEGER, 0),
					names,
					score,
					rounds: clampInt(r.rounds, 0, 99, 0),
					ticks: clampInt(r.ticks, 0, 10_000_000, 0),
					result: cleanResult(r.result),
					size: clampInt(r.size, 0, this.config.maxBytes, 0),
				});
			}
		} catch (err) {
			console.warn("failed to load replay index, starting fresh:", err);
		}
	}

	private save(): void {
		if (!this.enabled) return;
		const sorted = this.sortedRecords();
		const tmp = `${this.indexPath}.tmp`;
		fs.writeFileSync(tmp, JSON.stringify({ replays: sorted }), "utf8");
		fs.renameSync(tmp, this.indexPath);
	}

	private sortedRecords(): ReplayRecord[] {
		return Array.from(this.records.values()).sort((a, b) => {
			if (b.uploaded_at !== a.uploaded_at) return b.uploaded_at - a.uploaded_at;
			if (b.played_at !== a.played_at) return b.played_at - a.played_at;
			return a.id.localeCompare(b.id);
		});
	}

	prune(nowSec = Math.floor(Date.now() / 1000)): void {
		if (!this.enabled) return;
		const maxAgeSec = Math.max(1, this.config.maxAgeDays) * 24 * 60 * 60;
		const cutoff = nowSec - maxAgeSec;
		let changed = false;

		for (const [id, rec] of this.records) {
			if (rec.uploaded_at < cutoff) {
				this.records.delete(id);
				this.unlinkQuietly(this.filePath(id));
				changed = true;
			}
		}

		const sorted = this.sortedRecords();
		if (sorted.length > this.config.maxCount) {
			for (const rec of sorted.slice(this.config.maxCount)) {
				this.records.delete(rec.id);
				this.unlinkQuietly(this.filePath(rec.id));
				changed = true;
			}
		}

		// Also remove orphan .saito files not present in index
		try {
			for (const entry of fs.readdirSync(this.config.dir)) {
				if (!entry.endsWith(".saito")) continue;
				const id = entry.slice(0, -".saito".length);
				if (!this.records.has(id)) {
					this.unlinkQuietly(path.join(this.config.dir, entry));
				}
			}
		} catch {
			// Ignore directory scan errors
		}

		// Prune stale upload rate-limit buckets
		const rateCutoff = Date.now() - UPLOAD_WINDOW_MS;
		for (const [k, times] of this.uploadBuckets) {
			const kept = times.filter((t) => t > rateCutoff);
			if (kept.length === 0) {
				this.uploadBuckets.delete(k);
			} else {
				this.uploadBuckets.set(k, kept);
			}
		}

		if (changed) {
			this.save();
		}
	}

	private unlinkQuietly(file: string): void {
		try {
			fs.unlinkSync(file);
		} catch {
			// Ignore already-removed files
		}
	}

	private allowUpload(client: string): boolean {
		if (this.config.uploadLimitPerMinute <= 0) return true;
		const now = Date.now();
		const cutoff = now - UPLOAD_WINDOW_MS;
		const list = (this.uploadBuckets.get(client) ?? []).filter((t) => t > cutoff);
		if (list.length >= this.config.uploadLimitPerMinute) {
			this.uploadBuckets.set(client, list);
			return false;
		}
		list.push(now);
		this.uploadBuckets.set(client, list);
		return true;
	}

	list(game?: string, limit = 50): ReplayRecord[] {
		if (!this.enabled) return [];
		const cap = Math.max(1, Math.min(this.config.maxCount, Math.floor(limit) || 50));
		const wanted = game ? cleanStr(game, 32) : "";
		return this.sortedRecords()
			.filter((r) => !wanted || r.game === wanted)
			.slice(0, cap);
	}

	async handleHttp(
		req: http.IncomingMessage,
		res: http.ServerResponse,
		url: URL,
		client: string,
	): Promise<boolean> {
		const isCollection = url.pathname === "/replays" || url.pathname === "/replays/";
		const itemMatch = url.pathname.match(/^\/replays\/([^/]+)\/?$/);
		if (!isCollection && !itemMatch) {
			return false;
		}

		if (req.method === "OPTIONS") {
			res.writeHead(204, CORS_HEADERS);
			res.end();
			return true;
		}

		if (!this.enabled) {
			sendJson(res, 503, { error: "replay storage is disabled on this server" });
			return true;
		}

		if (isCollection && (req.method === "GET" || req.method === "HEAD")) {
			const game = url.searchParams.get("game") ?? undefined;
			const rawLimit = Number(url.searchParams.get("limit") ?? "50");
			const limit = Number.isFinite(rawLimit) && rawLimit > 0 ? rawLimit : 50;
			const replays = this.list(game, limit);
			const body = JSON.stringify({ replays });
			res.writeHead(200, {
				...CORS_HEADERS,
				"Content-Type": "application/json; charset=utf-8",
				"Cache-Control": "no-store",
			});
			res.end(req.method === "HEAD" ? undefined : body);
			return true;
		}

		if (isCollection && req.method === "POST") {
			if (!this.allowUpload(client)) {
				sendJson(res, 429, { error: "upload rate limit reached" });
				return true;
			}
			const maxJsonLen = Math.ceil(this.config.maxBytes * 1.4) + 16_384;
			let raw: Buffer;
			try {
				raw = await readBody(req, maxJsonLen);
			} catch (err) {
				const msg = err instanceof Error ? err.message : "bad request";
				sendJson(res, msg === "payload too large" ? 413 : 400, { error: msg });
				return true;
			}
			let payload: Record<string, unknown>;
			try {
				const parsed = JSON.parse(raw.toString("utf8"));
				if (!parsed || typeof parsed !== "object") {
					throw new Error("expected JSON object");
				}
				payload = parsed as Record<string, unknown>;
			} catch {
				sendJson(res, 400, { error: "invalid JSON body" });
				return true;
			}

			const dataB64 = typeof payload.data_b64 === "string" ? payload.data_b64 : "";
			if (!dataB64) {
				sendJson(res, 400, { error: "missing data_b64" });
				return true;
			}
			const bytes = Buffer.from(dataB64, "base64");
			if (bytes.length < REPLAY_MAGIC.length + 12 || bytes.length > this.config.maxBytes) {
				sendJson(res, bytes.length > this.config.maxBytes ? 413 : 400, {
					error: `replay size out of bounds (${bytes.length} bytes)`,
				});
				return true;
			}
			if (!bytes.subarray(0, REPLAY_MAGIC.length).equals(REPLAY_MAGIC)) {
				sendJson(res, 400, { error: "invalid replay file header" });
				return true;
			}

			const game = cleanStr(payload.game, 32);
			const engine = cleanStr(payload.engine, 32);
			if (!game || !engine) {
				sendJson(res, 400, { error: "missing game or engine" });
				return true;
			}

			const rawKey = typeof payload.dedup_key === "string" ? payload.dedup_key.toLowerCase() : "";
			const id = REPLAY_ID.test(rawKey)
				? rawKey.slice(0, 32)
				: crypto.createHash("sha256").update(bytes).digest("hex").slice(0, 32);

			if (this.records.has(id)) {
				sendJson(res, 200, { ok: true, id, duplicate: true });
				return true;
			}

			const nowSec = Math.floor(Date.now() / 1000);
			const names: [string, string] =
				Array.isArray(payload.names) && payload.names.length === 2
					? [cleanStr(payload.names[0], 48), cleanStr(payload.names[1], 48)]
					: ["", ""];
			const score: [number, number] =
				Array.isArray(payload.score) && payload.score.length === 2
					? [clampInt(payload.score[0], 0, 99, 0), clampInt(payload.score[1], 0, 99, 0)]
					: [0, 0];
			const record: ReplayRecord = {
				id,
				game,
				engine,
				content_hash: cleanStr(payload.content_hash, 32),
				played_at: clampInt(payload.played_at, 0, nowSec + 3600, nowSec),
				uploaded_at: nowSec,
				names,
				score,
				rounds: clampInt(payload.rounds, 0, 99, 0),
				ticks: clampInt(payload.ticks, 0, 10_000_000, 0),
				result: cleanResult(payload.result),
				size: bytes.length,
			};

			try {
				const target = this.filePath(id);
				const tmp = `${target}.tmp`;
				fs.writeFileSync(tmp, bytes);
				fs.renameSync(tmp, target);
				this.records.set(id, record);
				this.prune(nowSec);
				this.save();
			} catch (err) {
				console.error("failed to save replay:", err);
				sendJson(res, 500, { error: "failed to store replay" });
				return true;
			}

			sendJson(res, 201, { ok: true, id, duplicate: false });
			return true;
		}

		if (itemMatch && (req.method === "GET" || req.method === "HEAD")) {
			const id = itemMatch[1].toLowerCase();
			if (!REPLAY_ID.test(id) || !this.records.has(id)) {
				sendJson(res, 404, { error: "replay not found" });
				return true;
			}
			const file = this.filePath(id);
			try {
				const data = fs.readFileSync(file);
				res.writeHead(200, {
					...CORS_HEADERS,
					"Content-Type": "application/octet-stream",
					"Content-Length": String(data.length),
					"Cache-Control": "public, max-age=3600",
				});
				res.end(req.method === "HEAD" ? undefined : data);
			} catch {
				this.records.delete(id);
				this.save();
				sendJson(res, 404, { error: "replay not found" });
			}
			return true;
		}

		sendJson(res, 405, { error: "method not allowed" });
		return true;
	}
}
