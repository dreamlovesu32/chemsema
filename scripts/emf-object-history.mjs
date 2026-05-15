import fs from "node:fs/promises";
import path from "node:path";

function parseArgs(argv) {
  const args = {
    file: null,
    matchDraw: [],
    matchExt: [],
    range: null,
    historyLimit: 8,
    context: 0,
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--match-draw") {
      args.matchDraw = argv[++i]
        .split(",")
        .map((value) => value);
    } else if (arg === "--match-ext") {
      args.matchExt = argv[++i]
        .split(",")
        .map((value) => value);
    } else if (arg === "--range") {
      const [start, end] = argv[++i]
        .split(":")
        .map((value) => Number.parseInt(value.trim(), 10));
      args.range = [start, end];
    } else if (arg === "--history-limit") {
      args.historyLimit = Number.parseInt(argv[++i], 10);
    } else if (arg === "--context") {
      args.context = Number.parseInt(argv[++i], 10);
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
      payloadOffset: cursor + 12,
    };
    if (type === 0x4008) {
      entry.kind = "Object";
      entry.objectType = (flags >> 8) & 0xff;
      entry.objectId = flags & 0xff;
      entry.rawHex = buffer
        .subarray(cursor + 12, cursor + 12 + Math.min(payloadSize, 64))
        .toString("hex");
      entry.payloadWords = [];
      for (let wordOff = 0; wordOff + 4 <= Math.min(payloadSize, 32); wordOff += 4) {
        entry.payloadWords.push({
          off: wordOff,
          u32: u32(buffer, cursor + 12 + wordOff),
          f32: f32(buffer, cursor + 12 + wordOff),
        });
      }
    } else if (type === 0x401c) {
      entry.kind = "DrawString";
      entry.fontObjectId = flags & 0xff;
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

function matchesAny(text, candidates) {
  return candidates.length === 0 || candidates.includes(text);
}

function fmtMaybe(value) {
  if (value == null || Number.isNaN(value)) return "null";
  if (typeof value === "number" && Number.isFinite(value)) {
    return Number.isInteger(value) ? String(value) : value.toFixed(4);
  }
  return String(value);
}

function formatObjectHistory(history) {
  return history
    .map((item) => {
      const wordPreview = item.payloadWords
        .slice(0, 3)
        .map(
          (word) =>
            `+${word.off}:u32=${fmtMaybe(word.u32)} f32=${fmtMaybe(word.f32)}`
        )
        .join(" ");
      return `#${item.recordIndex}@${item.commentOffset} raw=${item.rawHex}${wordPreview ? ` ${wordPreview}` : ""}`;
    })
    .join("\n      ");
}

function printDrawMatch(match, objectHistory, historyLimit) {
  const entry = match.entry;
  console.log(
    `${match.recordIndex}: DrawString text=${JSON.stringify(entry.text)} flags=0x${entry.flags.toString(
      16
    )} fontId=${entry.fontObjectId} formatId=${entry.formatId} brushId=${entry.brushId} rect=(${fmtMaybe(
      entry.rect.x
    )},${fmtMaybe(entry.rect.y)},${fmtMaybe(entry.rect.width)},${fmtMaybe(entry.rect.height)})`
  );
  const relevant = [
    ["font", `6:${entry.fontObjectId}`],
    ["format", `7:${entry.formatId}`],
    ["brush", `1:${entry.brushId}`],
  ];
  for (const [label, key] of relevant) {
    const history = objectHistory.get(key) ?? [];
    console.log(
      `  ${label} ${key} definitions=${history.length}${
        history.length
          ? `\n      ${formatObjectHistory(history.slice(-historyLimit))}`
          : ""
      }`
    );
  }
}

function printExtMatch(record) {
  console.log(
    `${record.index}: EXTTEXTOUT text=${JSON.stringify(
      record?.text?.text ?? ""
    )} ref=(${record?.text?.reference?.x},${record?.text?.reference?.y}) bounds=(${record?.text?.bounds?.left},${record?.text?.bounds?.top},${record?.text?.bounds?.right},${record?.text?.bounds?.bottom})`
  );
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (!args.file) {
    console.error(
      "Usage: node scripts/emf-object-history.mjs <file.emf> [--match-draw a,b] [--match-ext a,b] [--range 10:40] [--history-limit 8]"
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

  const objectHistory = new Map();
  const drawMatches = [];
  const extMatches = [];

  const [rangeStart, rangeEnd] = args.range ?? [0, records.length - 1];

  for (const record of records) {
    if (!record || record.index < rangeStart || record.index > rangeEnd) continue;
    if (record.name === "EMR_GDICOMMENT") {
      const parts = decodeComment(buffer, record);
      for (const part of parts) {
        if (part.kind === "Object") {
          const key = `${part.objectType}:${part.objectId}`;
          const history = objectHistory.get(key) ?? [];
          history.push({
            recordIndex: record.index,
            commentOffset: part.offsetInComment,
            rawHex: part.rawHex,
            payloadWords: part.payloadWords,
          });
          objectHistory.set(key, history);
        } else if (part.kind === "DrawString" && matchesAny(part.text, args.matchDraw)) {
          drawMatches.push({
            recordIndex: record.index,
            entry: part,
          });
        }
      }
    } else if (
      record.name === "EMR_EXTTEXTOUTW" &&
      matchesAny(record?.text?.text ?? "", args.matchExt)
    ) {
      extMatches.push(record);
    }
  }

  console.log(`FILE ${emfPath}`);
  if (drawMatches.length) {
    console.log("DRAWSTRING MATCHES");
    for (const match of drawMatches) {
      printDrawMatch(match, objectHistory, args.historyLimit);
    }
  }
  if (extMatches.length) {
    console.log("EXTTEXTOUT MATCHES");
    for (const record of extMatches) {
      printExtMatch(record);
    }
  }
  if (!drawMatches.length && !extMatches.length) {
    console.log("No matches.");
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack ?? error.message : String(error));
  process.exit(1);
});
