import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { deflateSync, inflateRawSync, inflateSync } from "node:zlib";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const packZip = process.env.ELOSIUM_PACK ?? resolve(root, "..", "pack", "Whimscape_1.20-1.20.1_r11.zip");
const betterZip = process.env.ELOSIUM_BETTER_LEAVES ?? resolve(root, "..", "pack", "Better-Leaves-9.6.zip");
const packRoot = resolve(root, "assets", "pack");
const textureRoot = resolve(packRoot, "assets/minecraft/textures");
const blockDir = resolve(textureRoot, "block");
const colormapDir = resolve(textureRoot, "colormap");

const zipIndexes = new Map();

function readZipIndex(zip) {
  if (zipIndexes.has(zip)) return zipIndexes.get(zip);
  if (!existsSync(zip)) throw new Error(`zip not found: ${zip}`);
  const buffer = readFileSync(zip);
  const minOffset = Math.max(0, buffer.length - 65557);
  let eocd = -1;
  for (let offset = buffer.length - 22; offset >= minOffset; offset--) {
    if (buffer.readUInt32LE(offset) === 0x06054b50) {
      eocd = offset;
      break;
    }
  }
  if (eocd < 0) throw new Error(`invalid zip: ${zip}`);

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
  zipIndexes.set(zip, { buffer, entries });
  return { buffer, entries };
}

function readZipEntryBuffer(zip, entry) {
  const { buffer, entries } = readZipIndex(zip);
  const info = entries.get(entry);
  if (!info) throw new Error(`${entry} not found in ${zip}`);
  const nameLength = buffer.readUInt16LE(info.localOffset + 26);
  const extraLength = buffer.readUInt16LE(info.localOffset + 28);
  const start = info.localOffset + 30 + nameLength + extraLength;
  const data = buffer.subarray(start, start + info.compressedSize);
  return info.method === 0 ? data : inflateRawSync(data);
}

function readZipEntry(zip, entry) {
  return readZipEntryBuffer(zip, entry);
}

function extractZipPrefixes(zip, prefixes, destination) {
  const { buffer, entries } = readZipIndex(zip);
  for (const entry of entries.keys()) {
    if (entry.endsWith("/")) continue;
    if (!prefixes.some((prefix) => entry.startsWith(prefix))) continue;
    const target = resolve(destination, entry);
    mkdirSync(dirname(target), { recursive: true });
    writeFileSync(target, readZipEntryBuffer(zip, entry));
  }
}

if (!existsSync(resolve(blockDir, "grass_block_top.png"))) {
  extractZipPrefixes(
    packZip,
    ["assets/minecraft/textures/block/", "assets/minecraft/textures/colormap/"],
    packRoot,
  );
}

function paeth(a, b, c) {
  const p = a + b - c;
  const pa = Math.abs(p - a);
  const pb = Math.abs(p - b);
  const pc = Math.abs(p - c);
  if (pa <= pb && pa <= pc) return a;
  if (pb <= pc) return b;
  return c;
}

function decodePng(buffer) {
  let offset = 8;
  let width = 0;
  let height = 0;
  let bitDepth = 8;
  let colorType = 6;
  let interlace = 0;
  let palette = null;
  let transparency = null;
  const idat = [];

  while (offset < buffer.length) {
    const length = buffer.readUInt32BE(offset);
    const type = buffer.toString("ascii", offset + 4, offset + 8);
    const data = buffer.subarray(offset + 8, offset + 8 + length);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      bitDepth = data[8];
      colorType = data[9];
      interlace = data[12];
    } else if (type === "PLTE") {
      palette = data;
    } else if (type === "tRNS") {
      transparency = data;
    } else if (type === "IDAT") {
      idat.push(data);
    } else if (type === "IEND") {
      break;
    }
    offset += 12 + length;
  }

  if (interlace !== 0) throw new Error("interlaced PNG is not supported");
  if (![2, 3, 4, 6].includes(colorType)) throw new Error(`unsupported PNG color type ${colorType}`);

  const channels = colorType === 6 ? 4 : colorType === 2 ? 3 : colorType === 4 ? 2 : 1;
  const bitsPerPixel = channels * bitDepth;
  const bytesPerPixel = Math.max(1, Math.ceil(bitsPerPixel / 8));
  const stride = Math.ceil((width * bitsPerPixel) / 8);
  const raw = inflateSync(Buffer.concat(idat));
  const filtered = Buffer.alloc(stride * height);

  let cursor = 0;
  for (let y = 0; y < height; y++) {
    const filter = raw[cursor++];
    for (let x = 0; x < stride; x++) {
      const value = raw[cursor++];
      const left = x >= bytesPerPixel ? filtered[y * stride + x - bytesPerPixel] : 0;
      const up = y > 0 ? filtered[(y - 1) * stride + x] : 0;
      const upLeft = x >= bytesPerPixel && y > 0 ? filtered[(y - 1) * stride + x - bytesPerPixel] : 0;
      let result;
      switch (filter) {
        case 0:
          result = value;
          break;
        case 1:
          result = value + left;
          break;
        case 2:
          result = value + up;
          break;
        case 3:
          result = value + ((left + up) >> 1);
          break;
        case 4:
          result = value + paeth(left, up, upLeft);
          break;
        default:
          throw new Error(`unsupported PNG filter ${filter}`);
      }
      filtered[y * stride + x] = result & 0xff;
    }
  }

  const rgba = Buffer.alloc(width * height * 4);
  const maxIndex = (1 << bitDepth) - 1;

  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      let r = 0;
      let g = 0;
      let b = 0;
      let a = 255;
      if (colorType === 3) {
        const bitPosition = x * bitDepth;
        const byte = filtered[y * stride + (bitPosition >> 3)];
        const shift = 8 - bitDepth - (bitPosition & 7);
        const index = Math.min((byte >> shift) & maxIndex, Math.floor(palette.length / 3) - 1);
        r = palette[index * 3];
        g = palette[index * 3 + 1];
        b = palette[index * 3 + 2];
        if (transparency && index < transparency.length) a = transparency[index];
      } else if (colorType === 6) {
        const p = (y * width + x) * 4;
        r = filtered[p];
        g = filtered[p + 1];
        b = filtered[p + 2];
        a = filtered[p + 3];
      } else if (colorType === 2) {
        const p = (y * width + x) * 3;
        r = filtered[p];
        g = filtered[p + 1];
        b = filtered[p + 2];
      } else {
        const p = y * width + x;
        r = g = b = filtered[p * 2];
        a = filtered[p * 2 + 1];
      }
      const out = (y * width + x) * 4;
      rgba[out] = r;
      rgba[out + 1] = g;
      rgba[out + 2] = b;
      rgba[out + 3] = a;
    }
  }

  return { width, height, data: rgba };
}

function loadTexture(name) {
  const local = resolve(blockDir, `${name}.png`);
  if (existsSync(local)) return decodePng(readFileSync(local));
  const jar =
    process.env.ELOSIUM_MC_JAR ??
    "C:/Users/neo/AppData/Roaming/lunaspace-launcher/shared/versions/26.3/26.3.jar";
  return decodePng(
    readZipEntry(jar, `assets/minecraft/textures/block/${name}.png`),
  );
}

const CRC_TABLE = new Uint32Array(256);
for (let n = 0; n < 256; n++) {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  CRC_TABLE[n] = c >>> 0;
}

function crc32(buffer) {
  let c = 0xffffffff;
  for (const byte of buffer) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const name = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([name, data])), 0);
  return Buffer.concat([length, name, data, crc]);
}

function encodePng(width, height, rgba) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y++) {
    raw[y * (width * 4 + 1)] = 0;
    rgba.copy(raw, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

function rgbaAt(image, x, y) {
  const p = (y * image.width + x) * 4;
  return [image.data[p], image.data[p + 1], image.data[p + 2], image.data[p + 3]];
}

const grassMap = decodePng(readFileSync(resolve(colormapDir, "grass.png")));
const foliageMap = decodePng(readFileSync(resolve(colormapDir, "foliage.png")));

function sampleMap(image, u, v) {
  const x = Math.min(image.width - 1, Math.max(0, Math.round(u * (image.width - 1))));
  const y = Math.min(image.height - 1, Math.max(0, Math.round(v * (image.height - 1))));
  return rgbaAt(image, x, y);
}

console.log(`grass colormap corners TL ${sampleMap(grassMap, 0, 0)} BR ${sampleMap(grassMap, 1, 1)}`);

const TILE = 32;
const COLUMNS = 32;

const ruZip =
  process.env.ELOSIUM_RU ??
  "C:/Users/neo/Documents/LunarSpace/instances/Forge-1.20.1/mods/RegionsUnexploredForge-0.5.6+1.20.1.jar";
const bygZip =
  process.env.ELOSIUM_BYG ??
  "C:/Users/neo/Documents/LunarSpace/instances/Forge-1.20.1/mods/Oh-The-Biomes-Weve-Gone-Forge-1.8.2.jar";
const bopZip =
  process.env.ELOSIUM_BOP ??
  "C:/Users/neo/Documents/LunarSpace/instances/Forge-1.20.1/mods/BiomesOPlenty-forge-1.20.1-19.0.0.96.jar";

const leafTexture = decodePng(readZipEntry(betterZip, "assets/minecraft/textures/block/oak_leaves.png"));

const VANILLA = (name) => [`TEX_${name.toUpperCase()}`, loadTexture(name)];
const RU = (name) => [
  `TEX_${name.toUpperCase()}`,
  decodePng(readZipEntry(ruZip, `assets/regions_unexplored/textures/block/${name}.png`)),
];
const BYG = (wood, file) => [
  `TEX_${wood}_${file}`.toUpperCase(),
  decodePng(readZipEntry(bygZip, `assets/biomeswevegone/textures/block/${wood}/${file}.png`)),
];
const BOP = (name) => [
  `TEX_${name.toUpperCase()}`,
  decodePng(readZipEntry(bopZip, `assets/biomesoplenty/textures/block/${name}.png`)),
];

const tiles = [];

tiles.push(
  VANILLA("grass_block_top"),
  VANILLA("grass_block_side"),
  VANILLA("grass_block_side_overlay"),
  VANILLA("dirt"),
  VANILLA("coarse_dirt"),
  VANILLA("stone"),
  VANILLA("granite"),
  VANILLA("diorite"),
  VANILLA("andesite"),
  VANILLA("deepslate"),
  VANILLA("deepslate_top"),
  VANILLA("tuff"),
  VANILLA("calcite"),
  VANILLA("gravel"),
  VANILLA("sand"),
  VANILLA("red_sand"),
  VANILLA("sandstone"),
  VANILLA("red_sandstone"),
  VANILLA("snow"),
  VANILLA("ice"),
  VANILLA("packed_ice"),
  VANILLA("clay"),
  VANILLA("moss_block"),
  VANILLA("dripstone_block"),
  VANILLA("bedrock"),
  VANILLA("podzol_top"),
  VANILLA("podzol_side"),
  VANILLA("mycelium_top"),
  VANILLA("mycelium_side"),
  VANILLA("mud"),
  VANILLA("water_still"),
  VANILLA("lava_still"),
  VANILLA("seagrass"),
  VANILLA("kelp"),
  VANILLA("kelp_plant"),
  VANILLA("coal_ore"),
  VANILLA("deepslate_coal_ore"),
  VANILLA("iron_ore"),
  VANILLA("deepslate_iron_ore"),
  VANILLA("copper_ore"),
  VANILLA("deepslate_copper_ore"),
  VANILLA("gold_ore"),
  VANILLA("deepslate_gold_ore"),
  VANILLA("redstone_ore"),
  VANILLA("deepslate_redstone_ore"),
  VANILLA("lapis_ore"),
  VANILLA("deepslate_lapis_ore"),
  VANILLA("diamond_ore"),
  VANILLA("deepslate_diamond_ore"),
  VANILLA("emerald_ore"),
  VANILLA("deepslate_emerald_ore"),
  VANILLA("grass"),
  VANILLA("fern"),
  VANILLA("dead_bush"),
  VANILLA("dandelion"),
  VANILLA("poppy"),
  VANILLA("cornflower"),
  VANILLA("oxeye_daisy"),
  VANILLA("allium"),
  VANILLA("azure_bluet"),
  VANILLA("orange_tulip"),
  VANILLA("red_tulip"),
  VANILLA("pink_tulip"),
  VANILLA("white_tulip"),
  VANILLA("lily_of_the_valley"),
  VANILLA("red_mushroom"),
  VANILLA("brown_mushroom"),
  VANILLA("cactus_side"),
  VANILLA("sugar_cane"),
);

const WOODS = [
  ["oak", false],
  ["birch", false],
  ["spruce", false],
  ["jungle", false],
  ["acacia", false],
  ["dark_oak", false],
  ["cherry", false],
  ["baobab", true],
  ["blackwood", true],
  ["eucalyptus", true],
  ["magnolia", true],
  ["maple", true],
  ["pine", true],
  ["redwood", true],
  ["willow", true],
  ["dead", true],
];

for (const [id, ru] of WOODS) {
  const load = ru ? RU : VANILLA;
  tiles.push(load(`${id}_log`), load(`${id}_log_top`));
  if (id !== "dead") tiles.push(load(`${id}_leaves`));
}

tiles.push(
  RU("chalk"),
  RU("chalk_grass_side"),
  RU("silt_dirt"),
  RU("clover"),
  RU("daisy"),
  RU("cattail_top"),
  RU("palm_log"),
  RU("palm_log_top"),
  RU("palm_leaves_side"),
  RU("joshua_log"),
  RU("joshua_log_top"),
  RU("joshua_leaves_top"),
  RU("larch_log"),
  RU("larch_log_top"),
  RU("larch_leaves"),
);

const BYG_WOODS = [
  "fir",
  "cypress",
  "aspen",
  "jacaranda",
  "mahogany",
  "ebony",
  "white_mangrove",
  "cika",
  "skyris",
  "holly",
];

for (const wood of BYG_WOODS) {
  tiles.push(BYG(wood, "log"), BYG(wood, "log_top"), BYG(wood, "leaves"));
}

const BOP_PLANTS = [
  "lavender",
  "rose",
  "pink_daffodil",
  "high_grass_plant",
  "desert_grass",
  "dune_grass",
  "dead_grass",
  "glowflower",
  "sprout",
];

const BYG_PLANTS = [
  "amaranth",
  "cyan_amaranth",
  "black_rose",
  "crocus",
  "incan_lily",
  "japanese_orchid",
  "blue_sage",
  "prairie_grass",
  "beach_grass",
  "hydrangea_bush",
  "firecracker_flower_bush",
  "pink_allium_flower_bush",
];

for (const name of BOP_PLANTS) tiles.push(BOP(name));
for (const name of BYG_PLANTS) tiles.push(BYG_PLANT(name));

function BYG_PLANT(name) {
  return [
    `TEX_${name.toUpperCase()}`,
    decodePng(readZipEntry(bygZip, `assets/biomeswevegone/textures/block/${name}.png`)),
  ];
}


const FLORA_NAMES = new Set([
  "TEX_GRASS",
  "TEX_FERN",
  "TEX_DEAD_BUSH",
  "TEX_DANDELION",
  "TEX_POPPY",
  "TEX_CORNFLOWER",
  "TEX_OXEYE_DAISY",
  "TEX_ALLIUM",
  "TEX_AZURE_BLUET",
  "TEX_ORANGE_TULIP",
  "TEX_RED_TULIP",
  "TEX_PINK_TULIP",
  "TEX_WHITE_TULIP",
  "TEX_LILY_OF_THE_VALLEY",
  "TEX_RED_MUSHROOM",
  "TEX_BROWN_MUSHROOM",
  "TEX_SUGAR_CANE",
  "TEX_CLOVER",
  "TEX_DAISY",
  "TEX_CATTAIL_TOP",
  "TEX_LAVENDER",
  "TEX_ROSE",
  "TEX_PINK_DAFFODIL",
  "TEX_HIGH_GRASS_PLANT",
  "TEX_DESERT_GRASS",
  "TEX_DUNE_GRASS",
  "TEX_DEAD_GRASS",
  "TEX_GLOWFLOWER",
  "TEX_SPROUT",
  "TEX_AMARANTH",
  "TEX_CYAN_AMARANTH",
  "TEX_BLACK_ROSE",
  "TEX_CROCUS",
  "TEX_INCAN_LILY",
  "TEX_JAPANESE_ORCHID",
  "TEX_BLUE_SAGE",
  "TEX_PRAIRIE_GRASS",
  "TEX_BEACH_GRASS",
  "TEX_HYDRANGEA_BUSH",
  "TEX_FIRECRACKER_FLOWER_BUSH",
  "TEX_PINK_ALLIUM_FLOWER_BUSH",
  "TEX_SEAGRASS",
  "TEX_KELP",
  "TEX_KELP_PLANT",
]);

const isLeafName = (name) =>
  name.endsWith("_LEAVES") || name === "TEX_PALM_LEAVES_SIDE" || name === "TEX_JOSHUA_LEAVES_TOP";

const atlases = {
  block: { tiles: [], width: 1024, file: "atlas.png" },
  flora: { tiles: [], width: 512, file: "atlas_flora.png" },
  leaf: { tiles: [], width: 1024, file: "atlas_leaves.png" },
};

for (const entry of tiles) {
  const [name] = entry;
  const group = isLeafName(name) ? "leaf" : FLORA_NAMES.has(name) ? "flora" : "block";
  atlases[group].tiles.push(entry);
}

function buildAtlas(group) {
  const width = group.width;
  const pad = 1;
  const sizes = group.tiles.map(([, image]) => Math.min(image.width, image.height) + pad * 2);
  sizes.push(32 + pad * 2);
  let cursorX = 0;
  let cursorY = 0;
  let shelf = 0;
  const places = sizes.map((padded) => {
    if (cursorX + padded > width) {
      cursorX = 0;
      cursorY += shelf;
      shelf = 0;
    }
    const place = { x: cursorX, y: cursorY, padded, size: padded - pad * 2 };
    cursorX += padded;
    shelf = Math.max(shelf, padded);
    return place;
  });
  const used = cursorY + shelf;
  const height = 1 << Math.ceil(Math.log2(Math.max(32, used)));
  const atlas = Buffer.alloc(width * height * 4);
  const blit = (image, place) => {
    const frame = Math.min(image.width, image.height);
    for (let y = -pad; y < place.size + pad; y++) {
      for (let x = -pad; x < place.size + pad; x++) {
        const contentX = Math.max(0, Math.min(place.size - 1, x));
        const contentY = Math.max(0, Math.min(place.size - 1, y));
        const sourceX = Math.min(image.width - 1, Math.floor((contentX * image.width) / place.size));
        const sourceY = Math.min(frame - 1, Math.floor((contentY * frame) / place.size));
        const source = rgbaAt(image, sourceX, sourceY);
        const destination = ((place.y + y + pad) * width + place.x + x + pad) * 4;
        atlas[destination] = source[0];
        atlas[destination + 1] = source[1];
        atlas[destination + 2] = source[2];
        atlas[destination + 3] = source[3];
      }
    }
  };
  group.tiles.forEach(([, image], index) => blit(image, places[index]));
  const missing = places[places.length - 1];
  for (let y = 0; y < missing.size; y++) {
    for (let x = 0; x < missing.size; x++) {
      const checker = ((x >> 3) + (y >> 3)) % 2 === 0;
      const destination = ((missing.y + pad + y) * width + missing.x + pad + x) * 4;
      atlas[destination] = checker ? 255 : 0;
      atlas[destination + 1] = 0;
      atlas[destination + 2] = checker ? 255 : 0;
      atlas[destination + 3] = 255;
    }
  }
  group.height = height;
  group.uvs = places.map((place) => [
    (place.x + pad) / width,
    (place.y + pad) / height,
    (place.x + pad + place.size) / width,
    (place.y + pad + place.size) / height,
  ]);
  group.missing = group.tiles.length;
  mkdirSync(resolve(root, "assets", "textures"), { recursive: true });
  writeFileSync(resolve(root, "assets", "textures", group.file), encodePng(width, height, atlas));
  console.log(`${group.file}: ${width}x${height}, ${group.tiles.length} sprites at native size`);
}

buildAtlas(atlases.block);
buildAtlas(atlases.flora);
buildAtlas(atlases.leaf);

const waterStill = loadTexture("water_still");
const waterFrame = waterStill.width;
const waterFrames = Math.floor(waterStill.height / waterFrame);
const waterSheet = Buffer.alloc(waterFrames * waterFrame * waterFrame * 4);
for (let frameIndex = 0; frameIndex < waterFrames; frameIndex++) {
  for (let y = 0; y < waterFrame; y++) {
    for (let x = 0; x < waterFrame; x++) {
      const source = rgbaAt(waterStill, x, frameIndex * waterFrame + y);
      const destination = (y * waterFrames * waterFrame + frameIndex * waterFrame + x) * 4;
      waterSheet[destination] = source[0];
      waterSheet[destination + 1] = source[1];
      waterSheet[destination + 2] = source[2];
      waterSheet[destination + 3] = source[3];
    }
  }
}
writeFileSync(
  resolve(root, "assets", "textures", "water_frames.png"),
  encodePng(waterFrames * waterFrame, waterFrame, waterSheet),
);
console.log(`water frames: ${waterFrames}`);

const LUT_SIZE = 64;

function bakeLut(image) {
  const lut = Buffer.alloc(LUT_SIZE * LUT_SIZE * 3);
  for (let y = 0; y < LUT_SIZE; y++) {
    for (let x = 0; x < LUT_SIZE; x++) {
      const [r, g, b] = sampleMap(image, (x + 0.5) / LUT_SIZE, (y + 0.5) / LUT_SIZE);
      const p = (y * LUT_SIZE + x) * 3;
      lut[p] = r;
      lut[p + 1] = g;
      lut[p + 2] = b;
    }
  }
  return lut;
}

mkdirSync(resolve(root, "assets", "data"), { recursive: true });
writeFileSync(resolve(root, "assets", "data", "grass_lut.bin"), bakeLut(grassMap));
writeFileSync(resolve(root, "assets", "data", "foliage_lut.bin"), bakeLut(foliageMap));

function rotateY(x, z, angleDegrees, originX, originZ) {
  const angle = (angleDegrees * Math.PI) / 180;
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  const dx = x - originX;
  const dz = z - originZ;
  return [originX + dx * cos - dz * sin, originZ + dx * sin + dz * cos];
}

function bakeLeafVariant(index) {
  const model = JSON.parse(
    readZipEntry(betterZip, `assets/betterleaves/models/block/leaves${index}.json`).toString("utf8"),
  );
  const quads = [];
  for (const element of model.elements.slice(1)) {
    const [x1, y1, z1] = element.from;
    const [x2, y2, z2] = element.to;
    const faceName = Object.keys(element.faces).find((name) =>
      ["north", "south", "east", "west"].includes(name),
    );
    if (!faceName) continue;
    const [u1, v1, u2, v2] = element.faces[faceName].uv;
    let corners;
    if (Math.abs(z1 - z2) < 1e-6) {
      corners = [[x1, y1, z1], [x2, y1, z1], [x2, y2, z1], [x1, y2, z1]];
    } else if (Math.abs(x1 - x2) < 1e-6) {
      corners = [[x1, y1, z1], [x1, y1, z2], [x1, y2, z2], [x1, y2, z1]];
    } else {
      continue;
    }
    if (element.rotation && element.rotation.axis === "y") {
      const { angle, origin } = element.rotation;
      corners = corners.map(([x, y, z]) => {
        const [rotatedX, rotatedZ] = rotateY(x, z, angle, origin[0], origin[2]);
        return [rotatedX, y, rotatedZ];
      });
    }
    quads.push({
      positions: corners.map(([x, y, z]) => [x / 16, y / 16, z / 16]),
      uvs: [
        [u1 / 16, v2 / 16],
        [u2 / 16, v2 / 16],
        [u2 / 16, v1 / 16],
        [u1 / 16, v1 / 16],
      ],
    });
  }
  return quads;
}

const leafVariants = [1, 2, 3, 4].map(bakeLeafVariant);

const lines = [
  `#![allow(dead_code)]`,
  `pub const BLOCK_ATLAS_SIZE: [u32; 2] = [${atlases.block.width}, ${atlases.block.height}];`,
  `pub const FLORA_ATLAS_SIZE: [u32; 2] = [${atlases.flora.width}, ${atlases.flora.height}];`,
  `pub const LEAF_ATLAS_SIZE: [u32; 2] = [${atlases.leaf.width}, ${atlases.leaf.height}];`,
  `pub const LUT_SIZE: usize = ${LUT_SIZE};`,
  `pub const GRASS_LUT: &[u8] = include_bytes!("../assets/data/grass_lut.bin");`,
  `pub const FOLIAGE_LUT: &[u8] = include_bytes!("../assets/data/foliage_lut.bin");`,
];
for (const group of [atlases.block, atlases.flora, atlases.leaf]) {
  group.tiles.forEach(([name], index) => lines.push(`pub const ${name}: u8 = ${index};`));
}
lines.push(`pub const TEX_MISSING: u8 = ${atlases.block.missing};`);
for (const [name, group] of [
  ["BLOCK", atlases.block],
  ["FLORA", atlases.flora],
  ["LEAF", atlases.leaf],
]) {
  lines.push(`pub const ${name}_UV_RECTS: [[f32; 4]; ${group.uvs.length}] = [`);
  for (const [u0, v0, u1, v1] of group.uvs) {
    lines.push(`    [${u0.toFixed(6)}, ${v0.toFixed(6)}, ${u1.toFixed(6)}, ${v1.toFixed(6)}],`);
  }
  lines.push("];");
}
lines.push("");
lines.push("#[derive(Clone, Copy)]");
lines.push("pub struct LeafQuad {");
lines.push("    pub positions: [[f32; 3]; 4],");
lines.push("    pub uvs: [[f32; 2]; 4],");
lines.push("}");
leafVariants.forEach((quads, index) => {
  lines.push(`static LEAF_VARIANTS_${index}: [LeafQuad; ${quads.length}] = [`);
  for (const quad of quads) {
    lines.push("    LeafQuad {");
    lines.push(
      `        positions: [${quad.positions.map(([x, y, z]) => `[${x.toFixed(5)}, ${y.toFixed(5)}, ${z.toFixed(5)}]`).join(", ")}],`,
    );
    lines.push(
      `        uvs: [${quad.uvs.map(([u, v]) => `[${u.toFixed(5)}, ${v.toFixed(5)}]`).join(", ")}],`,
    );
    lines.push("    },");
  }
  lines.push("];");
});
lines.push(
  `pub static LEAF_VARIANTS: [&[LeafQuad]; ${leafVariants.length}] = [${leafVariants.map((_, index) => `&LEAF_VARIANTS_${index}`).join(", ")}];`,
);
writeFileSync(resolve(root, "src", "pack_atlas.rs"), `${lines.join("\n")}\n`);

console.log(`better leaves quads: ${leafVariants.map((variant) => variant.length).join(", ")}`);
