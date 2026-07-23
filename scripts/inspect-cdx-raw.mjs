import fs from "node:fs";

const input = process.argv[2];
if (!input) throw new Error("Usage: node scripts/inspect-cdx-raw.mjs input.cdx [tag ...]");
const showObjects = process.argv.includes("--objects");
const selected = new Set(
  (process.argv.slice(3).filter((value) => value !== "--objects").length
    ? process.argv.slice(3).filter((value) => value !== "--objects")
    : ["0700", "0706", "0804", "0805", "0806", "0807", "0808", "0809", "080a", "080b"])
    .map((value) => Number.parseInt(value.replace(/^0x/i, ""), 16)),
);
const bytes = fs.readFileSync(input);
let offset = 22;
const stack = [];
while (offset + 2 <= bytes.length) {
  const propertyOffset = offset;
  const tag = bytes.readUInt16LE(offset);
  offset += 2;
  if (tag === 0) {
    stack.pop();
    continue;
  }
  if (tag >= 0x8000) {
    const id = bytes.readUInt32LE(offset);
    offset += 4;
    if (showObjects) {
      console.log([
        propertyOffset.toString(16).padStart(8, "0"),
        `0x${tag.toString(16).padStart(4, "0")}`,
        "object",
        id,
        stack.join("/"),
      ].join("\t"));
    }
    stack.push(`${tag.toString(16).padStart(4, "0")}:${id}`);
    continue;
  }
  let length = bytes.readUInt16LE(offset);
  offset += 2;
  if (length === 0xffff) {
    length = bytes.readUInt32LE(offset);
    offset += 4;
  }
  const data = bytes.subarray(offset, offset + length);
  offset += length;
  if (selected.has(tag)) {
    console.log([
      propertyOffset.toString(16).padStart(8, "0"),
      `0x${tag.toString(16).padStart(4, "0")}`,
      length,
      data.toString("hex"),
      stack.join("/"),
    ].join("\t"));
  }
}
