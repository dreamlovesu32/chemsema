import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { generateChemDrawOracle } from "./chemdraw-oracle.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const schema = JSON.parse(await fs.readFile(path.join(root, "schemas", "cdx-cdxml-official-v1.json"), "utf8"));
const args = process.argv.slice(2);
const outArg = args.indexOf("--out");
const outDir = path.resolve(root, outArg >= 0 ? args[outArg + 1] : "tmp/chemdraw-cdxml-field-probe");
const refresh = args.includes("--refresh");
const elementsArg = args.indexOf("--refresh-elements");
const refreshElements = new Set(elementsArg >= 0 ? args[elementsArg + 1].split(",").filter(Boolean) : []);

const binaryAttributes = new Set([
  "BMP", "CartridgeData", "CompressedEnhancedMetafile", "CompressedOLEObject",
  "CompressedWindowsMetafile", "EnhancedMetafile", "GIF", "JPEG", "MacPICT",
  "MacPrintInfo", "OLEObject", "PDF", "PNG", "TIFF", "WinPrintInfo", "WindowsMetafile",
]);

const pointAttributes = new Set([
  "p", "WindowPosition", "WindowSize", "PositioningOffset", "FixInPlaceExtent", "FixInPlaceGap",
]);
const point3dAttributes = new Set([
  "xyz", "Head3D", "Tail3D", "Center3D", "MajorAxisEnd3D", "MinorAxisEnd3D",
]);
const rectangleAttributes = new Set([
  "BoundingBox", "BoundsInParent", "TextFrame", "PrintMargins",
]);
const objectIdArrays = new Set([
  "Attachments", "BasisObjects", "BondCircularOrdering", "BondOrdering", "BracketedObjectIDs",
  "ConnectionOrder", "CrossingBonds", "CrossingBondss", "ExternalBonds", "ReactionStepArrows",
  "ReactionStepObjectsAboveArrow", "ReactionStepObjectsBelowArrow", "ReactionStepPlusses",
  "ReactionStepProducts", "ReactionStepReactants", "SplitterPositions",
]);
const textAttributes = new Set([
  "AS", "AtomNumber", "Caption", "Class", "Comment", "Content", "CreationProgram",
  "CreationUserName", "CrossReferenceContainer", "CrossReferenceDocument",
  "CrossReferenceIdentifier", "CrossReferenceSequence", "DisplayName", "Edition", "EditionAlias",
  "Footer", "Formula", "GenericList", "GenericNickname", "Header", "Keyword", "LabelText",
  "Name", "RegistryAuthority", "RegistryNumber", "RelationValue", "SequenceIdentifier",
  "SGDataType", "SGDataValue", "SGPropertyType", "SRULabel", "Value", "Warning",
  "XAxisLabel", "YAxisLabel", "name",
]);

function escapeXml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll('"', "&quot;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

function sampleValue(attribute) {
  if (attribute.values?.length) {
    const preferred = {
      ArrowheadHead: "Full", ArrowheadTail: "Full", ArrowheadType: "Hollow",
      ArrowType: "FullHead", BracketType: "RoundPair", ConstraintType: "Distance",
      Display: "Dash", Display2: "Dash", FillType: "Solid", LabelDisplay: "Right",
      LineType: "Dashed", NoGo: "Cross", NodeType: "GenericNickname",
      OrbitalType: "p", OvalType: "Circle", RectangleType: "RoundEdge",
      Side: "top",
    }[attribute.name];
    if (preferred && attribute.values.includes(preferred)) return preferred;
    const weakValues = new Set(["Unspecified", "Undefined", "Unknown", "None"]);
    return attribute.values.find((value) => value !== attribute.default && !weakValues.has(value))
      ?? attribute.values.find((value) => value !== attribute.default)
      ?? attribute.values[0];
  }
  const name = attribute.name;
  if (binaryAttributes.has(name)) return null;
  if (name === "id") return "900001";
  if (["B", "BeginExternalNum", "BondID", "InnerAtomID", "object"].includes(name)) return "101";
  if (["E", "EndExternalNum", "AtomID"].includes(name)) return "102";
  if (name === "attribute") return "Charge";
  if (pointAttributes.has(name)) return "100 100";
  if (point3dAttributes.has(name)) return "100 100 0";
  if (rectangleAttributes.has(name)) return "80 80 120 120";
  if (name === "extent") return "240 200";
  if (name === "CurvePoints") return "3 80 100 100 80 120 100";
  if (name === "CurvePoints3D") return "3 80 100 0 100 80 0 120 100 0";
  if (name === "ElementList") return "6 7 8";
  if (name === "LineStarts") return "2 4";
  if (name === "ReactionStepAtomMap" || name === "ReactionStepAtomMapAuto" || name === "ReactionStepAtomMapManual") return "101 102";
  if (objectIdArrays.has(name)) return "101 102";
  if (/Date$/.test(name)) return "2026 7 23 12 34 56 4";
  if (/Color$/.test(name) || name === "color" || name === "bgcolor" || name === "font") return "2";
  if (/Size$/.test(name) || /Width$/.test(name) || /Height$/.test(name) || /Length$/.test(name)
      || /Offset$/.test(name) || /Angle$/.test(name) || /Spacing$/.test(name)
      || /Position$/.test(name) || /Radius$/.test(name) || /Percent$/.test(name)
      || /Scale$/.test(name) || /Low$/.test(name) || /Count/.test(name)
      || /Number/.test(name) || /Pages$/.test(name) || /Start$/.test(name) || /End$/.test(name)
      || /Order$/.test(name) || /ID$/.test(name) || /Type$/.test(name) || /Value$/.test(name)
      || /alpha$/i.test(name) || name === "Z" || name === "Weight" || name === "Valence") return "2";
  if (name === "r") return "0.25";
  if (name === "g") return "0.5";
  if (name === "b") return "0.75";
  if (textAttributes.has(name)) return `ChemSemaProbe-${name}`;
  return "2";
}

function sampleValueFor(element, attribute) {
  if (element.name === "represent" && attribute.name === "object") return "901";
  if (element.name === "represent" && attribute.name === "attribute") return "Charge";
  if (element.name === "chemicalproperty" && attribute.name === "ChemicalPropertyDisplayID") return "801";
  if (element.name === "step" && attribute.name === "ReactionStepPlusses") return "820";
  return sampleValue(attribute);
}

function attributesFor(element, target) {
  const values = new Map();
  const semanticBaselines = {
    annotation: { Content: "ChemSemaProbe" },
    arrow: { Tail3D: "80 100 0", Head3D: "140 100 0", ArrowheadHead: "Full", ArrowheadType: "Solid" },
    bioshape: { BioShapeType: "1SubstrateEnzyme", BoundingBox: "80 80 160 140" },
    bracketedgroup: { BracketedObjectIDs: "101 102", BracketUsage: "SRU" },
    constraint: { ConstraintType: "Distance", BasisObjects: "101 102" },
    geometry: { GeometricFeature: "LineFromPoints", BasisObjects: "101 102" },
    chemicalproperty: { BasisObjects: "101 102", ChemicalPropertyDisplayID: "801", ChemicalPropertyType: "Name", Name: "ChemSemaProbe" },
    coloredmoleculararea: { BasisObjects: "101 102" },
    curve: { CurvePoints: "3 80 100 100 80 120 100" },
    embeddedobject: { BoundingBox: "80 80 120 120", PNG: "89504E470D0A1A0A0000000D49484452000000010000000108060000001F15C4890000000D4944415408D763F8FFFF3F030008FC02FEA7A6A00000000049454E44AE426082" },
    graphic: { GraphicType: "Line", BoundingBox: "80 100 140 100" },
    n: { p: "100 100" },
    objecttag: { Name: "ChemSemaProbe", TagType: "String", Value: "ChemSemaProbeValue", Persistent: "yes", Visible: "yes" },
    plasmidregion: { RegionStart: "0", RegionEnd: "90", RegionOffset: "0", Head3D: "140 100 0", Tail3D: "100 140 0", Center3D: "100 100 0", MajorAxisEnd3D: "140 100 0", MinorAxisEnd3D: "100 140 0" },
    templategrid: { extent: "240 200", PaneHeight: "200", NumRows: "1", NumColumns: "1" },
    tlcplate: { BoundingBox: "80 80 140 160" },
    gepplate: { BoundingBox: "80 80 140 160" },
    plasmidmap: { BoundingBox: "80 80 160 160" },
  };
  for (const [name, value] of Object.entries(semanticBaselines[element.name] ?? {})) {
    if (element.attributes.some((attribute) => attribute.name === name)) values.set(name, value);
  }
  for (const attribute of element.attributes) {
    if (attribute.required) values.set(attribute.name, sampleValueFor(element, attribute) ?? "1");
  }
  if (element.attributes.some((attribute) => attribute.name === "id") && target.name !== "id") {
    values.set("id", element.name === "templategrid" ? "0" : "900001");
  }
  values.set(target.name, target.value);
  return [...values].map(([name, value]) => `${name}="${escapeXml(value)}"`).join(" ");
}

function elementXml(element, target) {
  const attrs = attributesFor(element, target);
  const tag = element.name;
  if (tag === "s") return `<s ${attrs}>ChemSemaProbe</s>`;
  if (tag === "t") return `<t ${attrs}><s font="3" size="10" color="0">ChemSemaProbe</s></t>`;
  if (tag === "n") {
    const labelFields = new Set(["GenericNickname", "LabelDisplay", "LabelFace", "LabelFont", "LabelSize", "NodeType"]);
    const nodeType = labelFields.has(target.name) && target.name !== "NodeType" ? ' NodeType="GenericNickname"' : "";
    return `<n ${attrs}${nodeType}><t p="100 100"><s font="3" size="10" color="0">Probe</s></t></n>`;
  }
  if (tag === "spectrum") return `<spectrum ${attrs}>1 2 3 4</spectrum>`;
  if (tag === "altgroup") return `<altgroup ${attrs}><t><s font="3" size="10" color="0">Probe</s></t></altgroup>`;
  if (tag === "bracketedgroup") return `<bracketedgroup ${attrs}><bracketattachment GraphicID="810"/><bracketattachment GraphicID="811"/></bracketedgroup>`;
  if (tag === "colortable") return `<colortable ${attrs}><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>`;
  if (tag === "fonttable") return `<fonttable ${attrs}><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>`;
  if (tag === "scheme") return `<scheme ${attrs}><step /></scheme>`;
  if (tag === "objecttag") return `<objecttag ${attrs}><t><s font="3" size="10" color="0">Probe</s></t></objecttag>`;
  if (tag === "sequence") return `<sequence ${attrs}><t><s font="3" size="10" color="0">Sequence Probe</s></t></sequence>`;
  if (tag === "crossreference") return `<crossreference ${attrs}><t><s font="3" size="10" color="0">Cross Reference Probe</s></t></crossreference>`;
  if (tag === "rlogic") return `<rlogic ${attrs}><rlogicitem /></rlogic>`;
  if (tag === "group") return `<group ${attrs}><t id="812" p="100 100"><s font="3" size="10" color="0">Grouped</s></t></group>`;
  return `<${tag}${attrs ? ` ${attrs}` : ""}/>`;
}

function wrapPageTarget(element, xml) {
  switch (element.name) {
    case "n": return `<fragment id="500"><n id="101" p="80 100"/><n id="102" p="120 100"/>${xml}</fragment>`;
    case "b": return `<fragment id="500"><n id="101" p="80 100"/><n id="102" p="120 100"/>${xml}</fragment>`;
    case "s": return `<t p="100 100">${xml}</t>`;
    case "color": return null;
    case "font": return null;
    case "represent": return `<fragment id="790"><n id="901" p="100 100" Element="8" Charge="-1"/><graphic id="902" BoundingBox="104 96 110 102" GraphicType="Symbol" SymbolType="Minus">${xml}</graphic></fragment>`;
    case "regnum": return `<fragment id="750"><n id="751" p="100 100"/>${xml}</fragment>`;
    case "objecttag": return `<fragment id="760"><n id="761" p="100 100">${xml}</n></fragment>`;
    case "crossreference": return `<sequence id="770" SequenceIdentifier="ChemSemaProbe-Sequence"><t><s font="3" size="10" color="0">Sequence Probe</s></t></sequence>${xml}`;
    case "chemicalproperty": return `<t id="801" p="100 140"><s font="3" size="10" color="0">Property Probe</s></t>${xml}`;
    case "border": return `<table id="780"><page id="781" BoundingBox="80 80 160 160">${xml}</page></table>`;
    case "bracketedgroup": return `<graphic id="810" BoundingBox="30 30 30 70" GraphicType="Bracket" BracketType="Square"/><graphic id="811" BoundingBox="70 70 70 30" GraphicType="Bracket" BracketType="Square"/>${xml}`;
    case "crossingbond": return `<bracketedgroup><bracketattachment>${xml}</bracketattachment></bracketedgroup>`;
    case "bracketattachment": return `<bracketedgroup>${xml}</bracketedgroup>`;
    case "tlclane": return `<tlcplate id="710" BoundingBox="80 80 140 160">${xml}</tlcplate>`;
    case "tlcspot": return `<tlcplate id="710" BoundingBox="80 80 140 160"><tlclane>${xml}</tlclane></tlcplate>`;
    case "geplane": return `<gepplate id="720" BoundingBox="80 80 140 160">${xml}</gepplate>`;
    case "gepband": return `<gepplate id="720" BoundingBox="80 80 140 160"><geplane>${xml}</geplane></gepplate>`;
    case "marker": return `<gepplate id="720" BoundingBox="80 80 140 160"><geplane><gepband>${xml}</gepband></geplane></gepplate>`;
    case "plasmidregion": return `<plasmidmap id="730" BoundingBox="80 80 160 160">${xml}</plasmidmap>`;
    case "plasmidmarker": return `<plasmidmap id="730" BoundingBox="80 80 160 160">${xml}</plasmidmap>`;
    case "rlogicitem": return `<rlogic>${xml}</rlogic>`;
    case "sgcomponent": return `<stoichiometrygrid id="740" BoundingBox="80 80 160 160">${xml}</stoichiometrygrid>`;
    case "sgdatum": return `<stoichiometrygrid id="740" BoundingBox="80 80 160 160"><sgcomponent>${xml}</sgcomponent></stoichiometrygrid>`;
    case "step": return `<graphic id="820" BoundingBox="90 90 110 110" GraphicType="Symbol" SymbolType="Plus"/><scheme>${xml}</scheme>`;
    default: return xml;
  }
}

function documentXml(element, target) {
  const xml = elementXml(element, target);
  const rootDefaults = new Map([
    ["CreationProgram", "ChemSema field probe"], ["BoundingBox", "0 0 240 200"],
    ["BondLength", "14.4"], ["LineWidth", "0.6"], ["BoldWidth", "2"],
    ["HashSpacing", "2.5"], ["MarginWidth", "1.6"], ["LabelFont", "3"],
    ["LabelSize", "10"], ["CaptionFont", "3"], ["CaptionSize", "10"],
  ]);
  if (element.name === "CDXML") rootDefaults.delete(target.name);
  const rootAttrs = [...rootDefaults].map(([name, value]) => `${name}="${value}"`).join(" ");
  const colors = '<colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>';
  const fonts = '<fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>';
  if (element.name === "CDXML") return `<?xml version="1.0" encoding="UTF-8"?><CDXML ${rootAttrs} ${attributesFor(element, target)}>${colors}${fonts}<page id="1" BoundingBox="0 0 240 200"/></CDXML>`;
  if (element.name === "page") return `<?xml version="1.0" encoding="UTF-8"?><CDXML ${rootAttrs}>${colors}${fonts}<page ${attributesFor(element, target)}/></CDXML>`;
  if (element.name === "colortable") return `<?xml version="1.0" encoding="UTF-8"?><CDXML ${rootAttrs}>${xml}${fonts}<page id="1" BoundingBox="0 0 240 200"/></CDXML>`;
  if (element.name === "fonttable") return `<?xml version="1.0" encoding="UTF-8"?><CDXML ${rootAttrs}>${colors}${xml}<page id="1" BoundingBox="0 0 240 200"/></CDXML>`;
  if (element.name === "templategrid") return `<?xml version="1.0" encoding="UTF-8"?><CDXML ${rootAttrs}>${colors}${fonts}<page id="1" BoundingBox="0 0 240 200"/>${xml}</CDXML>`;
  if (element.name === "color") return `<?xml version="1.0" encoding="UTF-8"?><CDXML ${rootAttrs}><colortable>${xml}<color r="0" g="0" b="0"/></colortable>${fonts}<page id="1" BoundingBox="0 0 240 200"/></CDXML>`;
  if (element.name === "font") return `<?xml version="1.0" encoding="UTF-8"?><CDXML ${rootAttrs}>${colors}<fonttable>${xml}</fonttable><page id="1" BoundingBox="0 0 240 200"/></CDXML>`;
  const body = wrapPageTarget(element, xml);
  return `<?xml version="1.0" encoding="UTF-8"?><CDXML ${rootAttrs}>${colors}${fonts}<page id="1" BoundingBox="0 0 240 200"><fragment id="600"><n id="101" p="40 40"/><n id="102" p="60 40"/><b id="103" B="101" E="102"/></fragment>${body}</page></CDXML>`;
}

function stem(element, attribute) {
  return `${element}--${attribute}`.replace(/[^A-Za-z0-9_.-]/g, "_");
}

function findTargetTag(xml, elementName, attributeName) {
  const tags = [...xml.matchAll(new RegExp(`<${elementName}\\b[^>]*>`, "gi")), ...xml.matchAll(new RegExp(`<${elementName}\\b[^>]*/>`, "gi"))]
    .map((match) => match[0]);
  return tags.find((tag) => new RegExp(`\\b${attributeName}="`, "i").test(tag)) ?? tags[0] ?? null;
}

function readAttribute(tag, attributeName) {
  if (!tag) return null;
  return new RegExp(`\\b${attributeName}="([^"]*)"`, "i").exec(tag)?.[1] ?? null;
}

await fs.mkdir(outDir, { recursive: true });
const inputDir = path.join(outDir, "inputs");
const oracleDir = path.join(outDir, "chemdraw");
await fs.mkdir(inputDir, { recursive: true });
await fs.mkdir(oracleDir, { recursive: true });

const cases = [];
for (const element of schema.cdxml.elements) {
  for (const attribute of element.attributes) {
    const value = sampleValueFor(element, attribute);
    if (value == null) {
      cases.push({ element: element.name, attribute: attribute.name, status: "opaque-needs-real-payload", sourceValue: null, declaredDefault: attribute.default });
      continue;
    }
    const name = stem(element.name, attribute.name);
    const input = path.join(inputDir, `${name}.cdxml`);
    await fs.writeFile(input, documentXml(element, { name: attribute.name, value }), "utf8");
    cases.push({ element: element.name, attribute: attribute.name, status: "pending", sourceValue: value, declaredDefault: attribute.default, input, name });
  }
}

const runnable = cases.filter((entry) => entry.status === "pending");
const outputsExist = await Promise.all(runnable.map(async (entry) => {
  try { await fs.access(path.join(oracleDir, `${entry.name}.chemdraw.cdxml`)); return true; } catch { return false; }
}));
const generationErrors = new Map();
async function generateBatch(entries) {
  if (!entries.length) return;
  try {
    await generateChemDrawOracle({
      inputs: entries.map((entry) => entry.input),
      outputNames: entries.map((entry) => entry.name),
      outDir: oracleDir,
      formats: ["cdxml"],
    });
  } catch (error) {
    if (entries.length === 1) {
      generationErrors.set(entries[0].name, error instanceof Error ? error.message : String(error));
      return;
    }
    const middle = Math.floor(entries.length / 2);
    await generateBatch(entries.slice(0, middle));
    await generateBatch(entries.slice(middle));
  }
}
const needsGeneration = runnable.filter((entry, index) => refresh || refreshElements.has(entry.element) || !outputsExist[index]);
for (let index = 0; index < needsGeneration.length; index += 20) {
  await generateBatch(needsGeneration.slice(index, index + 20));
}

for (const entry of runnable) {
  const output = path.join(oracleDir, `${entry.name}.chemdraw.cdxml`);
  try {
    const xml = await fs.readFile(output, "utf8");
    const tag = findTargetTag(xml, entry.element, entry.attribute);
    const outputValue = readAttribute(tag, entry.attribute);
    entry.output = output;
    entry.objectPresent = tag != null;
    entry.outputValue = outputValue;
    entry.status = outputValue == null
      ? entry.sourceValue === entry.declaredDefault ? "default-omitted" : "removed"
      : outputValue === entry.sourceValue ? "retained" : "normalized";
  } catch (error) {
    entry.status = generationErrors.has(entry.name) ? "open-failed" : "failed";
    entry.error = generationErrors.get(entry.name) ?? (error instanceof Error ? error.message : String(error));
  }
}

const summary = Object.fromEntries([...new Set(cases.map((entry) => entry.status))]
  .sort().map((status) => [status, cases.filter((entry) => entry.status === status).length]));
const report = { schema: "chemsema.chemdraw-cdxml-field-probe.v1", generatedAt: new Date().toISOString(), summary, cases };
const reportPath = path.join(outDir, "report.json");
await fs.writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
console.log(JSON.stringify({ reportPath, summary }));
