import fs from "node:fs/promises";
import path from "node:path";

function parseArgs(argv) {
  const args = {
    file: null,
    records: [],
    range: null,
    matchText: [],
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--records") {
      args.records = argv[++i]
        .split(",")
        .map((value) => Number.parseInt(value.trim(), 10))
        .filter(Number.isFinite);
    } else if (arg === "--range") {
      const [start, end] = argv[++i]
        .split(":")
        .map((value) => Number.parseInt(value.trim(), 10));
      args.range = [start, end];
    } else if (arg === "--match-text") {
      args.matchText = argv[++i]
        .split(",")
        .map((value) => value);
    } else if (!args.file) {
      args.file = arg;
    }
  }
  return args;
}

function u16(buf, off) {
  return off + 2 <= buf.length ? buf.readUInt16LE(off) : null;
}

function u32(buf, off) {
  return off + 4 <= buf.length ? buf.readUInt32LE(off) : null;
}

function f32(buf, off) {
  return off + 4 <= buf.length ? buf.readFloatLE(off) : null;
}

function decodeComment(buffer, record) {
  const parts = [];
  const dataSize = u32(buffer, record.offset + 8) ?? 0;
  let cursor = record.offset + 16;
  const end = Math.min(record.offset + record.size, record.offset + 12 + dataSize);
  while (cursor + 12 <= end) {
    const type = u16(buffer, cursor);
    const flags = u16(buffer, cursor + 2);
    const size = u32(buffer, cursor + 4);
    const payloadSize = u32(buffer, cursor + 8);
    if (!size || size < 12 || cursor + size > end) break;
    const entry = {
      offsetInComment: cursor - record.offset,
      type,
      flags,
      size,
      payloadSize,
    };
    if (type === 0x4008) {
      entry.kind = "Object";
      entry.objectType = (flags >> 8) & 0xff;
      entry.objectId = flags & 0xff;
      entry.rawHex = buffer
        .subarray(cursor + 12, cursor + 12 + Math.min(payloadSize, 32))
        .toString("hex");
    } else if (type === 0x401c) {
      entry.kind = "DrawString";
      entry.brushId = u32(buffer, cursor + 12);
      entry.formatId = u32(buffer, cursor + 16);
      entry.charCount = u32(buffer, cursor + 20);
      entry.rect = {
        x: f32(buffer, cursor + 24),
        y: f32(buffer, cursor + 28),
        width: f32(buffer, cursor + 32),
        height: f32(buffer, cursor + 36),
      };
      entry.text = buffer.toString(
        "utf16le",
        cursor + 40,
        cursor + 40 + (entry.charCount ?? 0) * 2
      );
    }
    parts.push(entry);
    cursor += size;
  }
  return parts;
}

function interestingRecord(record, matchTextSet) {
  if (record.name === "EMR_GDICOMMENT") return true;
  if (record.name === "EMR_EXTTEXTOUTW") {
    if (!matchTextSet.size) return true;
    return matchTextSet.has(record?.text?.text ?? "");
  }
  return false;
}

function printRecord(record, buffer) {
  if (!record) return;
  if (record.name === "EMR_EXTTEXTOUTW") {
    console.log(
      `${record.index}: ${record.name} text=${JSON.stringify(
        record?.text?.text ?? ""
      )} ref=(${record?.text?.reference?.x},${record?.text?.reference?.y})`
    );
    return;
  }
  console.log(`${record.index}: ${record.name} size=${record.size}`);
  if (record.name === "EMR_GDICOMMENT") {
    for (const part of decodeComment(buffer, record)) {
      if (part.kind === "DrawString") {
        console.log(
          `  sub 0x${part.type.toString(16)} flags=0x${part.flags.toString(
            16
          )} text=${JSON.stringify(part.text)} formatId=${part.formatId} brushId=${
            part.brushId
          } rect=(${part.rect.x},${part.rect.y},${part.rect.width},${part.rect.height})`
        );
      } else if (part.kind === "Object") {
        console.log(
          `  sub 0x${part.type.toString(16)} flags=0x${part.flags.toString(
            16
          )} objectType=${part.objectType} objectId=${part.objectId} raw=${part.rawHex}`
        );
      } else {
        console.log(
          `  sub 0x${part.type.toString(16)} flags=0x${part.flags.toString(
            16
          )} size=${part.size}`
        );
      }
    }
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.file) {
    console.error(
      "Usage: node scripts/emf-text-trace.mjs <file.emf> [--records 1,2,3] [--range 100:120] [--match-text a,b]"
    );
    process.exit(1);
  }
  const emfPath = path.resolve(args.file);
  const recordsPath = `${emfPath}.records.json`;
  const [buffer, jsonText] = await Promise.all([
    fs.readFile(emfPath),
    fs.readFile(recordsPath, "utf8"),
  ]);
  const inspection = JSON.parse(jsonText);
  const records = inspection.records ?? [];
  const matchTextSet = new Set(args.matchText);

  if (args.records.length) {
    for (const index of args.records) {
      printRecord(records[index], buffer);
    }
    return;
  }

  if (args.range) {
    const [start, end] = args.range;
    for (let index = start; index <= end; index += 1) {
      const record = records[index];
      if (!record) continue;
      if (!interestingRecord(record, matchTextSet)) continue;
      printRecord(record, buffer);
    }
    return;
  }

  for (const record of records) {
    if (!interestingRecord(record, matchTextSet)) continue;
    printRecord(record, buffer);
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});

