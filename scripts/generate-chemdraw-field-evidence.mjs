import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const tmpDir = path.join(root, "tmp");
const probePath = path.join(tmpDir, "chemdraw-cdxml-field-probe", "report.json");
const outputPath = path.join(root, "schemas", "chemdraw-cdxml-field-evidence-v1.json");

async function walk(directory) {
  const files = [];
  for (const entry of await fs.readdir(directory, { withFileTypes: true })) {
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...await walk(fullPath));
    else files.push(fullPath);
  }
  return files;
}

function attributes(tag) {
  const result = [];
  for (const match of tag.matchAll(/([A-Za-z_][A-Za-z0-9_.:-]*)\s*=\s*"([^"]*)"/g)) {
    result.push([match[1], match[2]]);
  }
  return result;
}

const observedObjects = new Map();
const observedAttributes = new Map();
const versions = new Set();
const candidateFiles = (await walk(tmpDir)).filter((file) =>
  file.toLowerCase().endsWith(".cdxml")
  && !file.includes(`${path.sep}chemdraw-cdxml-field-probe${path.sep}`));
const oracleFiles = [];
for (const file of candidateFiles) {
  const xml = await fs.readFile(file, "utf8");
  // Public corpora are useful only when the document identifies ChemDraw as
  // its producer. This prevents hand-authored fixtures from being promoted to
  // behavioral evidence merely because they use a declared DTD field.
  const creationProgram = /<CDXML\b[^>]*\bCreationProgram\s*=\s*"([^"]*)"/i.exec(xml)?.[1];
  if (!file.toLowerCase().endsWith(".chemdraw.cdxml") && !creationProgram?.startsWith("ChemDraw")) continue;
  oracleFiles.push(file);
  const relativeFile = path.relative(root, file).replaceAll("\\", "/");
  for (const match of xml.matchAll(/<([A-Za-z_][A-Za-z0-9_.:-]*)\b[^>]*>/g)) {
    // ChemDraw has emitted a few element names with different casing across
    // releases (for example `ColoredMolecularArea` versus the lower-case DTD
    // declaration). XML itself is case-sensitive, but these spellings refer to
    // the same CDXML object in the official schema, so aggregate evidence by
    // the DTD's canonical lower-case object name.
    const element = match[1].toLowerCase();
    if (element.startsWith("?")) continue;
    const object = observedObjects.get(element) ?? { count: 0, files: new Set() };
    object.count += 1;
    if (object.files.size < 5) object.files.add(relativeFile);
    observedObjects.set(element, object);
    for (const [name, value] of attributes(match[0])) {
      const key = `${element}@${name}`;
      const attribute = observedAttributes.get(key) ?? { element, name, count: 0, values: new Set(), files: new Set() };
      attribute.count += 1;
      if (attribute.values.size < 8) attribute.values.add(value);
      if (attribute.files.size < 5) attribute.files.add(relativeFile);
      observedAttributes.set(key, attribute);
      if (element === "CDXML" && name === "CreationProgram" && value.startsWith("ChemDraw")) versions.add(value);
    }
  }
}

const probe = JSON.parse(await fs.readFile(probePath, "utf8"));

const sdkBase = "https://iupac.github.io/IUPAC-FAIRSpec/cdx_sdk";
const dtdSource = "https://static.chemistry.revvitycloud.com/cdxml/CDXML.dtd";

function explicitContextRule(element, attribute) {
  if (element === "regnum") return {
    kind: "official-not-read-or-written",
    rule: "The SDK marks the registry-number object as not read or written by ChemDraw; a modern open/save cycle removes it.",
    source: `${sdkBase}/RegistryNumber.htm`,
  };
  if (element === "splitter") return {
    kind: "official-reserved-for-future-use",
    rule: "The SDK says splitter is reserved for future compatibility and no public ChemDraw release reads or writes it.",
    source: `${sdkBase}/Splitter.htm`,
  };
  if (element === "templategrid") return {
    kind: "official-template-document-context",
    rule: "TemplateGrid is valid only for template documents; all four fields are required and page count must equal rows times columns. A normal-document save removes the grid.",
    source: `${sdkBase}/TemplateGrid.htm`,
  };
  if (element === "bracketedgroup" && ["ComponentOrder", "PolymerRepeatPattern"].includes(attribute)) return {
    kind: "official-write-only-object-contract",
    rule: "The SDK defines the bracket field but labels the BracketedGroup object as written-only; current ChemDraw also removes an incomplete synthetic form.",
    source: `${sdkBase}/BracketedGroup.htm`,
  };
  if (element === "group" && attribute === "Integral") return {
    kind: "official-semantic-with-save-normalization",
    rule: "Integral means the group is non-subdivisible when nonzero. ChemDraw flattens a synthetic one-child group on save, so the property disappears with its container.",
    source: `${sdkBase}/Group.htm`,
  };
  if (element === "step" && attribute === "ReactionStepPlusses") return {
    kind: "official-object-id-reference-with-context-cleanup",
    rule: "ReactionStepPlusses is an object-ID list of plus graphics in a reaction step. ChemDraw removes a step that has no valid reactant/product context, including this reference.",
    source: `${sdkBase}/ReactionStep.htm`,
  };
  if (["marker", "plasmidmarker", "plasmidregion", "sgcomponent", "stoichiometrygrid"].includes(element)) return {
    kind: "official-specialized-context-contract",
    rule: "The current official DTD defines this specialized object's parent/content model, lexical type, enumeration, and default. ChemDraw removes a context-free synthetic instance rather than retaining a partial object.",
    source: dtdSource,
  };
  return null;
}

const declarations = {};
for (const entry of probe.cases) {
  const key = `${entry.element}@${entry.attribute}`;
  const observed = observedAttributes.get(key);
  const accepted = entry.status === "retained" || entry.status === "normalized" || entry.status === "default-omitted";
  const removalObserved = entry.status === "removed" && entry.objectPresent;
  const opaqueByDefinition = entry.status === "opaque-needs-real-payload";
  const contextRule = explicitContextRule(entry.element, entry.attribute);
  declarations[key] = {
    status: observed || accepted || removalObserved || opaqueByDefinition || contextRule ? "verified" : "in-review",
    verificationKind: contextRule?.kind
      ?? (observed ? "chemdraw-output-observed"
        : accepted ? `probe-${entry.status}`
          : removalObserved ? "probe-field-removed-object-preserved"
            : opaqueByDefinition ? "official-opaque-payload" : "unresolved"),
    rule: contextRule?.rule ?? null,
    officialSource: contextRule?.source ?? null,
    evidence: [
      ...(observed ? ["chemdraw-output-observed"] : []),
      ...(accepted ? [`probe-${entry.status}`] : []),
      ...(removalObserved ? ["probe-field-removed-object-preserved"] : []),
      ...(opaqueByDefinition ? ["official-opaque-payload"] : []),
      ...(contextRule ? [contextRule.kind, `probe-${entry.status}`] : []),
      ...(!observed && !accepted && !removalObserved && !opaqueByDefinition && !contextRule ? [`probe-${entry.status}`] : []),
    ],
    sourceValue: entry.sourceValue,
    outputValue: entry.outputValue ?? null,
    observedCount: observed?.count ?? 0,
    observedValues: [...(observed?.values ?? [])],
    sampleFiles: [...(observed?.files ?? [])],
  };
}

const probedObjects = new Map();
for (const entry of probe.cases) {
  const value = probedObjects.get(entry.element) ?? { cases: 0, preservedCases: 0 };
  value.cases += 1;
  if (entry.objectPresent) value.preservedCases += 1;
  probedObjects.set(entry.element, value);
}
const objectNames = new Set([...observedObjects.keys(), ...probedObjects.keys()]);
const objects = Object.fromEntries([...objectNames].sort().map((name) => {
  const observed = observedObjects.get(name);
  const probed = probedObjects.get(name);
  return [name, {
    status: observed || probed?.preservedCases > 0 ? "verified" : "in-review",
    evidence: [...(observed ? ["chemdraw-output-observed"] : []), ...(probed?.preservedCases > 0 ? ["probe-object-preserved"] : [])],
    observedCount: observed?.count ?? 0,
    probeCases: probed?.cases ?? 0,
    preservedProbeCases: probed?.preservedCases ?? 0,
    sampleFiles: [...(observed?.files ?? [])],
  }];
}));
const uniqueAttributes = {};
for (const [key, entry] of Object.entries(declarations)) {
  const name = key.slice(key.indexOf("@") + 1);
  const aggregate = uniqueAttributes[name] ?? { declarations: 0, verifiedDeclarations: 0, evidence: new Set() };
  aggregate.declarations += 1;
  if (entry.status === "verified") aggregate.verifiedDeclarations += 1;
  entry.evidence.forEach((value) => aggregate.evidence.add(value));
  uniqueAttributes[name] = aggregate;
}
for (const aggregate of Object.values(uniqueAttributes)) {
  aggregate.status = aggregate.verifiedDeclarations > 0 ? "verified" : "in-review";
  aggregate.evidence = [...aggregate.evidence].sort();
}

const evidence = {
  schema: "chemsema.chemdraw-cdxml-field-evidence.v1",
  chemDrawVersions: [...versions].sort(),
  oracleFileCount: oracleFiles.length,
  probeCaseCount: probe.cases.length,
  objects,
  declarations: Object.fromEntries(Object.entries(declarations).sort(([a], [b]) => a.localeCompare(b))),
  attributes: Object.fromEntries(Object.entries(uniqueAttributes).sort(([a], [b]) => a.localeCompare(b))),
};
await fs.writeFile(outputPath, `${JSON.stringify(evidence, null, 2)}\n`, "utf8");
console.log(JSON.stringify({
  outputPath,
  oracleFileCount: oracleFiles.length,
  observedObjects: Object.keys(objects).length,
  verifiedDeclarations: Object.values(declarations).filter((entry) => entry.status === "verified").length,
  verifiedAttributes: Object.values(uniqueAttributes).filter((entry) => entry.status === "verified").length,
}));
