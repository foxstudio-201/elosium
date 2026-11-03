import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { inflateRawSync } from "node:zlib";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const jar =
  process.env.ELOSIUM_MC_JAR ??
  "C:/Users/neo/AppData/Roaming/lunaspace-launcher/shared/versions/26.3/26.3.jar";

if (!existsSync(jar)) {
  throw new Error(`Minecraft jar not found: ${jar}`);
}

const buffer = readFileSync(jar);
let eocd = -1;
for (let offset = buffer.length - 22; offset >= Math.max(0, buffer.length - 65557); offset--) {
  if (buffer.readUInt32LE(offset) === 0x06054b50) {
    eocd = offset;
    break;
  }
}
if (eocd < 0) {
  throw new Error(`invalid zip: ${jar}`);
}
const count = buffer.readUInt16LE(eocd + 10);
let cursor = buffer.readUInt32LE(eocd + 16);
const entries = new Map();
for (let i = 0; i < count; i++) {
  if (buffer.readUInt32LE(cursor) !== 0x02014b50) break;
  const method = buffer.readUInt16LE(cursor + 10);
  const compressedSize = buffer.readUInt32LE(cursor + 20);
  const nameLength = buffer.readUInt16LE(cursor + 28);
  const extraLength = buffer.readUInt16LE(cursor + 30);
  const commentLength = buffer.readUInt16LE(cursor + 32);
  const localOffset = buffer.readUInt32LE(cursor + 42);
  const name = buffer.toString("utf8", cursor + 46, cursor + 46 + nameLength);
  entries.set(name, { method, compressedSize, localOffset });
  cursor += 46 + nameLength + extraLength + commentLength;
}

function readEntry(name) {
  const info = entries.get(name);
  if (!info) return null;
  const nameLength = buffer.readUInt16LE(info.localOffset + 26);
  const extraLength = buffer.readUInt16LE(info.localOffset + 28);
  const start = info.localOffset + 30 + nameLength + extraLength;
  const data = buffer.subarray(start, start + info.compressedSize);
  return info.method === 0 ? data : inflateRawSync(data);
}

const prefixes = ["data/minecraft/worldgen/", "data/minecraft/tags/worldgen/"];
let written = 0;
let bytes = 0;
for (const name of entries.keys()) {
  if (name.endsWith("/")) continue;
  if (!prefixes.some((prefix) => name.startsWith(prefix))) continue;
  const data = readEntry(name);
  if (!data) continue;
  const target = resolve(root, "assets", "worldgen", name);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, data);
  written += 1;
  bytes += data.length;
}

console.log(
  `worldgen data: ${written} files, ${(bytes / 1048576).toFixed(2)} MB -> assets/worldgen`,
);
