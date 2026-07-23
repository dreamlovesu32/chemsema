import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const readJson = (name) => JSON.parse(readFileSync(join(rootDir, "schemas", name), "utf8"));
const official = readJson("cdx-cdxml-official-v1.json");
const engineOfficial = JSON.parse(readFileSync(join(rootDir, "crates", "chemsema-engine", "schemas", "cdx-cdxml-official-v1.json"), "utf8"));
const verification = readJson("cdx-cdxml-verification-v1.json");
const evidence = readJson("chemdraw-cdxml-field-evidence-v1.json");
const ledger = readFileSync(join(rootDir, "docs", "cdx-cdxml-field-verification.zh-CN.md"), "utf8");
const cdx = readFileSync(join(rootDir, "crates", "chemsema-engine", "src", "cdx.rs"), "utf8");
const cdxml = readFileSync(join(rootDir, "crates", "chemsema-engine", "src", "cdxml.rs"), "utf8");
const document = readFileSync(join(rootDir, "crates", "chemsema-engine", "src", "document.rs"), "utf8");

const fail = [];
if (JSON.stringify(engineOfficial) !== JSON.stringify(official)) fail.push("embedded engine official schema is out of sync");
const objectKeys = new Set(verification.objects.map((entry) => `${entry.publishedTag ?? entry.tag}:${entry.cdxmlName}`));
for (const entry of official.cdx.objects) {
  if (!objectKeys.has(`${entry.tag}:${entry.cdxmlName}`)) fail.push(`missing CDX object ${entry.tag}:${entry.cdxmlName}`);
}
for (const entry of verification.objects.filter((entry) => entry.publishedTag)) {
  if (entry.schemaStatus !== "verified-with-erratum") fail.push(`runtime CDX object override lacks erratum status ${entry.tag}:${entry.cdxmlName}`);
}
const propertyKeys = new Set(verification.properties.map((entry) => `${entry.tag}:${entry.sdkName}`));
for (const entry of official.cdx.properties) {
  if (!propertyKeys.has(`${entry.tag}:${entry.sdkName}`)) fail.push(`missing CDX property ${entry.tag}:${entry.sdkName}`);
}
const elementNames = new Set(verification.cdxml.elements.map((entry) => entry.name));
for (const entry of official.cdxml.elements) {
  if (!elementNames.has(entry.name)) fail.push(`missing CDXML element ${entry.name}`);
}
const officialAttributes = new Set(official.cdxml.elements.flatMap((entry) => entry.attributes.map((attribute) => attribute.name)));
const verifiedAttributes = new Set(verification.cdxml.attributes.map((entry) => entry.name));
for (const name of officialAttributes) {
  if (!verifiedAttributes.has(name)) fail.push(`missing CDXML attribute ${name}`);
}

const officialDeclarations = new Set(official.cdxml.elements.flatMap((element) =>
  element.attributes.map((attribute) => `${element.name}@${attribute.name}`)));
const evidenceDeclarations = new Set(Object.keys(evidence.declarations));
for (const key of officialDeclarations) {
  if (!evidenceDeclarations.has(key)) fail.push(`missing CDXML declaration evidence ${key}`);
  else if (evidence.declarations[key].status !== "verified") fail.push(`CDXML declaration review is incomplete for ${key}`);
}
for (const key of evidenceDeclarations) {
  if (!officialDeclarations.has(key)) fail.push(`non-official CDXML declaration in evidence ${key}`);
}
if (evidenceDeclarations.size !== officialDeclarations.size) {
  fail.push(`CDXML declaration evidence count ${evidenceDeclarations.size} does not match official count ${officialDeclarations.size}`);
}

const behaviorStatuses = new Set(["verified", "in-review", "unverified", "unsupported"]);
for (const entry of [...verification.objects, ...verification.properties, ...verification.cdxml.attributes]) {
  if (!behaviorStatuses.has(entry.behaviorStatus)) fail.push(`invalid behavior status ${entry.behaviorStatus}`);
  if (entry.behaviorStatus !== "verified") fail.push(`behavior review is incomplete for ${entry.cdxmlName ?? entry.name ?? entry.sdkName}`);
  if (entry.storageStatus !== "verified") fail.push(`non-lossless storage status for ${entry.cdxmlName ?? entry.name}`);
}
if (verification.counts.losslessUncovered !== 0) fail.push("losslessUncovered must be zero");
if (verification.counts.cdxmlDeclarations !== officialDeclarations.size) fail.push("verification declaration count is stale");
if (verification.counts.verifiedBehaviorCdxmlDeclarations !== officialDeclarations.size) fail.push("not every CDXML declaration is behavior-verified");
if (evidence.oracleFileCount < 1 || evidence.probeCaseCount < 1) fail.push("ChemDraw field evidence is empty");

for (const marker of ["pub interchange: BTreeMap<String, InterchangeDocument>", "set_interchange_property("]) {
  if (!document.includes(marker)) fail.push(`missing CCJS interchange API marker: ${marker}`);
}
for (const marker of ["raw_base64", "official_property_info", "write_raw_interchange_object"]) {
  if (!cdx.includes(marker)) fail.push(`missing CDX lossless implementation marker: ${marker}`);
}
for (const marker of ["interchange_object_from_xml", "source_tree"]) {
  if (!cdxml.includes(marker)) fail.push(`missing CDXML lossless implementation marker: ${marker}`);
}
for (const expected of ["262 个 CDX 属性", "384 个唯一 CDXML 属性名", "无损未覆盖数：0"]) {
  if (!ledger.includes(expected)) fail.push(`ledger summary missing: ${expected}`);
}

if (fail.length) {
  console.error("CDX/CDXML field ledger check failed:");
  for (const entry of fail) console.error(`  ${entry}`);
  process.exit(1);
}
console.log(`CDX/CDXML ledger covers ${verification.objects.length} objects, ${verification.properties.length} CDX properties, ${verification.cdxml.elements.length} CDXML elements, and ${verification.cdxml.attributes.length} unique CDXML attributes; lossless gaps: 0.`);
