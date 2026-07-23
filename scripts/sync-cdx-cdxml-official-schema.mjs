import { createHash } from "node:crypto";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const outputPath = join(rootDir, "schemas", "cdx-cdxml-official-v1.json");
const engineOutputPath = join(rootDir, "crates", "chemsema-engine", "schemas", "cdx-cdxml-official-v1.json");
const sources = {
  properties: "https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/TableOfProperties.htm",
  objects: "https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/AllCDXObjects.htm",
  dataTypes: "https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk/DataTypes.htm",
  dtd: "https://static.chemistry.revvitycloud.com/cdxml/CDXML.dtd",
};

const decodeHtml = (value) => value
  .replace(/<[^>]+>/g, " ")
  .replace(/&nbsp;/gi, " ")
  .replace(/&amp;/gi, "&")
  .replace(/&lt;/gi, "<")
  .replace(/&gt;/gi, ">")
  .replace(/&quot;/gi, "\"")
  .replace(/&#39;/gi, "'")
  .replace(/\s+/g, " ")
  .trim();

async function fetchText(url) {
  const response = await fetch(url, { redirect: "follow" });
  if (!response.ok) throw new Error(`${response.status} ${response.statusText}: ${url}`);
  return response.text();
}

function tableRows(html) {
  return [...html.matchAll(/<tr\b[^>]*>([\s\S]*?)<\/tr>/gi)].map((match) =>
    [...match[1].matchAll(/<td\b[^>]*>([\s\S]*?)<\/td>/gi)].map((cell) => ({
      html: cell[1],
      text: decodeHtml(cell[1]),
    })),
  );
}

function parseProperties(html) {
  const rows = tableRows(html);
  const properties = [];
  for (let index = 0; index < rows.length; index += 1) {
    const cells = rows[index];
    if (cells.length !== 4 || !/^0x[0-9a-f]{4}$/i.test(cells[0].text)) continue;
    const href = cells[1].html.match(/href=["']([^"']+)["']/i)?.[1] ?? null;
    const description = rows[index + 1]?.length === 2 ? rows[index + 1][1].text : "";
    properties.push({
      tag: cells[0].text.toUpperCase().replace("0X", "0x"),
      sdkName: cells[1].text,
      cdxmlName: cells[2].text === "(not used)" ? null : cells[2].text,
      cdxType: cells[3].text || "varies",
      description,
      detailUrl: href ? new URL(href, sources.properties).href : null,
    });
  }
  // The archived SDK table contains three internally contradictory cells.
  // Their SDK constant names/descriptions and the current Revvity DTD agree
  // on the corrected CDXML names; CurveSpacing already occupies 0x0A38, so
  // Closed continues at 0x0A39.  Keep these corrections explicit and
  // machine-visible instead of silently teaching each importer a different
  // alias.
  const corrections = new Map([
    ["kCDXProp_3DMajorAxisEnd", { cdxmlName: "MajorAxisEnd3D" }],
    ["kCDXProp_3DMinorAxisEnd", { cdxmlName: "MinorAxisEnd3D" }],
    ["kCDXProp_Closed", { tag: "0x0A39", cdxType: "CDXBooleanImplied" }],
  ]);
  return properties.map((property) => ({
    ...property,
    ...(corrections.get(property.sdkName) ?? {}),
  }));
}

function parseObjects(html) {
  return tableRows(html)
    .filter((cells) => cells.length === 4 && /^0x[0-9a-f]{4}$/i.test(cells[1].text))
    .map((cells) => ({
      object: cells[0].text,
      tag: cells[1].text.toUpperCase().replace("0X", "0x"),
      sdkName: cells[2].text,
      cdxmlName: cells[3].text,
    }));
}

function expandParameterEntities(dtd) {
  const entities = new Map();
  for (const match of dtd.matchAll(/<!ENTITY\s+%\s+([\w.-]+)\s+["']([\s\S]*?)["']\s*>/g)) {
    entities.set(match[1], match[2]);
  }
  const expand = (value, seen = new Set()) => value.replace(/%([\w.-]+);/g, (token, name) => {
    if (!entities.has(name) || seen.has(name)) return token;
    const nextSeen = new Set(seen).add(name);
    return expand(entities.get(name), nextSeen);
  });
  return { entities, expand };
}

function parseDtd(dtd) {
  const { entities, expand } = expandParameterEntities(dtd);
  const elements = new Map();
  for (const match of dtd.matchAll(/<!ELEMENT\s+([^\s>]+)\s+([\s\S]*?)>/g)) {
    elements.set(match[1], { name: match[1], contentModel: decodeHtml(match[2]), attributes: [] });
  }
  const attributePattern = /([A-Za-z_:][\w:.-]*)\s+(\([^)]*\)|CDATA|IDREFS?|NMTOKENS?|ENTIT(?:Y|IES)|NOTATION\s+\([^)]*\))\s+(#REQUIRED|#IMPLIED|#FIXED\s+(?:"[^"]*"|'[^']*')|"[^"]*"|'[^']*')/g;
  for (const match of dtd.matchAll(/<!ATTLIST\s+([^\s>]+)\s+([\s\S]*?)>/g)) {
    const name = match[1];
    const body = expand(match[2]);
    const element = elements.get(name) ?? { name, contentModel: null, attributes: [] };
    for (const attribute of body.matchAll(attributePattern)) {
      const type = attribute[2].replace(/\s+/g, " ").trim();
      const defaultDeclaration = attribute[3].replace(/\s+/g, " ").trim();
      element.attributes.push({
        name: attribute[1],
        type,
        required: defaultDeclaration === "#REQUIRED",
        implied: defaultDeclaration === "#IMPLIED",
        fixed: defaultDeclaration.startsWith("#FIXED"),
        default: defaultDeclaration.startsWith("#")
          ? defaultDeclaration.match(/["']([^"']*)["']/)?.[1] ?? null
          : defaultDeclaration.slice(1, -1),
        values: type.startsWith("(")
          ? type.slice(1, -1).split("|").map((value) => value.trim()).filter(Boolean)
          : null,
      });
    }
    element.attributes.sort((left, right) => left.name.localeCompare(right.name));
    elements.set(name, element);
  }
  return {
    parameterEntityCount: entities.size,
    elements: [...elements.values()].sort((left, right) => left.name.localeCompare(right.name)),
  };
}

const [propertyHtml, objectHtml, dataTypeHtml, dtd] = await Promise.all([
  fetchText(sources.properties),
  fetchText(sources.objects),
  fetchText(sources.dataTypes),
  fetchText(sources.dtd),
]);
const properties = parseProperties(propertyHtml);
const objects = parseObjects(objectHtml);
const cdxml = parseDtd(dtd);
if (properties.length < 200 || objects.length < 30 || cdxml.elements.length < 20) {
  throw new Error(`Incomplete official schema parse: ${properties.length} properties, ${objects.length} objects, ${cdxml.elements.length} elements`);
}

const schema = {
  schema: "chemsema.cdx-cdxml-official-schema.v1",
  generatedAt: new Date().toISOString(),
  sources,
  sourceHashes: {
    propertiesSha256: createHash("sha256").update(propertyHtml).digest("hex"),
    objectsSha256: createHash("sha256").update(objectHtml).digest("hex"),
    dataTypesSha256: createHash("sha256").update(dataTypeHtml).digest("hex"),
    dtdSha256: createHash("sha256").update(dtd).digest("hex"),
  },
  errata: [
    {
      sdkName: "kCDXProp_3DMajorAxisEnd",
      published: { cdxmlName: "Center3D" },
      corrected: { cdxmlName: "MajorAxisEnd3D" },
      basis: "SDK constant name, description, and current Revvity DTD",
    },
    {
      sdkName: "kCDXProp_3DMinorAxisEnd",
      published: { cdxmlName: "Center3D" },
      corrected: { cdxmlName: "MinorAxisEnd3D" },
      basis: "SDK constant name, description, and current Revvity DTD",
    },
    {
      sdkName: "kCDXProp_Closed",
      published: { tag: "0x0A38", cdxType: "CDXBoolean" },
      corrected: { tag: "0x0A39", cdxType: "CDXBooleanImplied" },
      basis: "0x0A38 is CurveSpacing; CDX sequence and empty-property encoding",
    },
  ],
  counts: {
    cdxProperties: properties.length,
    cdxObjects: objects.length,
    cdxmlElements: cdxml.elements.length,
    cdxmlAttributes: new Set(cdxml.elements.flatMap((element) => element.attributes.map((attribute) => attribute.name))).size,
  },
  cdx: { objects, properties },
  cdxml,
};

mkdirSync(dirname(outputPath), { recursive: true });
const serialized = `${JSON.stringify(schema, null, 2)}\n`;
writeFileSync(outputPath, serialized);
mkdirSync(dirname(engineOutputPath), { recursive: true });
writeFileSync(engineOutputPath, serialized);
console.log(`Wrote ${outputPath}`);
console.log(`Wrote ${engineOutputPath}`);
console.log(JSON.stringify(schema.counts));
