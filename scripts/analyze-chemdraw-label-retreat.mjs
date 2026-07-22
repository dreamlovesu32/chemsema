import fs from "node:fs";
import path from "node:path";

function readMeasurements(directory) {
  return JSON.parse(fs.readFileSync(path.join(directory, "measurements.json"), "utf8")).measurements;
}

function readOptionalResult(directory) {
  const resultPath = path.join(directory, "measurements.json");
  return fs.existsSync(resultPath) ? JSON.parse(fs.readFileSync(resultPath, "utf8")) : null;
}

function quantile(values, fraction) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.floor((sorted.length - 1) * fraction)];
}

function errorSummary(errors) {
  const absolute = errors.map(Math.abs);
  return {
    count: errors.length,
    maePt: absolute.reduce((sum, value) => sum + value, 0) / absolute.length,
    p50Pt: quantile(absolute, 0.5),
    p95Pt: quantile(absolute, 0.95),
    maxPt: Math.max(...absolute),
    biasPt: errors.reduce((sum, value) => sum + value, 0) / errors.length,
  };
}

function valueSummary(values) {
  return {
    count: values.length,
    mean: values.reduce((sum, value) => sum + value, 0) / values.length,
    p50: quantile(values, 0.5),
    p95: quantile(values, 0.95),
    max: Math.max(...values),
  };
}

function matches(entry, query) {
  return Object.entries(query).every(([key, value]) => entry[key] === value);
}

function indexBy(entries, fields) {
  return new Map(entries.map((entry) => [fields.map((field) => entry[field]).join("\u0000"), entry]));
}

function matchedDifferences(reference, candidates, fields) {
  const referenceIndex = indexBy(reference, fields);
  return candidates.flatMap((candidate) => {
    const key = fields.map((field) => candidate[field]).join("\u0000");
    const baseline = referenceIndex.get(key);
    return baseline ? [candidate.retreat - baseline.retreat] : [];
  });
}

function matchedFieldDifferences(reference, candidates, fields, valueField) {
  const referenceIndex = indexBy(reference, fields);
  return candidates.flatMap((candidate) => {
    const key = fields.map((field) => candidate[field]).join("\u0000");
    const baseline = referenceIndex.get(key);
    return baseline ? [candidate[valueField] - baseline[valueField]] : [];
  });
}

function interpolate(entries, field, value) {
  const unique = [...new Map(entries.map((entry) => [entry[field], entry])).values()]
    .sort((left, right) => left[field] - right[field]);
  if (unique.length < 2) throw new Error(`Need two ${field} samples, found ${unique.length}`);
  let rightIndex = unique.findIndex((entry) => entry[field] >= value);
  if (rightIndex < 0) rightIndex = unique.length - 1;
  else if (rightIndex === 0) rightIndex = 1;
  const left = unique[rightIndex - 1];
  const right = unique[rightIndex];
  const ratio = (value - left[field]) / (right[field] - left[field]);
  return left.retreat * (1 - ratio) + right.retreat * ratio;
}

function marginPrediction(training, target, overrides = {}) {
  const query = {
    glyph: target.glyph,
    angleDeg: target.angleDeg,
    font: target.font,
    face: target.face,
    size: overrides.size ?? target.size,
    lineWidth: overrides.lineWidth ?? target.lineWidth,
  };
  return interpolate(training.filter((entry) => matches(entry, query)), "marginWidth", overrides.marginWidth ?? target.marginWidth);
}

function lineWidthPrediction(training, target) {
  const query = {
    glyph: target.glyph,
    angleDeg: target.angleDeg,
    font: target.font,
    face: target.face,
    size: target.size,
    marginWidth: target.marginWidth,
  };
  return interpolate(training.filter((entry) => matches(entry, query)), "lineWidth", target.lineWidth);
}

const root = process.cwd();
const thinDir = path.resolve(process.argv[2] ?? path.join(root, "tmp/chemdraw-label-retreat-thin"));
const fineDir = path.resolve(process.argv[3] ?? path.join(root, "tmp/chemdraw-label-retreat-fine"));
const surveyDir = path.resolve(process.argv[4] ?? path.join(root, "tmp/chemdraw-label-retreat-survey"));
const holdoutDir = path.resolve(process.argv[5] ?? path.join(root, "tmp/chemdraw-label-retreat-holdout"));
const outputPath = path.resolve(process.argv[6] ?? path.join(holdoutDir, "analysis.json"));
const comprehensiveDir = path.resolve(process.argv[7] ?? path.join(root, "tmp/chemdraw-label-retreat-comprehensive"));
const directionalDir = path.resolve(process.argv[8] ?? path.join(root, "tmp/chemdraw-label-retreat-directional"));
const anchoredDirectionalDir = path.resolve(process.argv[9] ?? path.join(root, "tmp/chemdraw-label-retreat-anchored-directional"));

const thin = readMeasurements(thinDir);
const fine = readMeasurements(fineDir);
const survey = readMeasurements(surveyDir);
const holdout = readMeasurements(holdoutDir);
const comprehensiveResult = readOptionalResult(comprehensiveDir);
const comprehensive = comprehensiveResult?.measurements ?? [];
const directionalResult = readOptionalResult(directionalDir);
const directional = directionalResult?.measurements ?? [];
const anchoredDirectionalResult = readOptionalResult(anchoredDirectionalDir);
const anchoredDirectional = anchoredDirectionalResult?.measurements ?? [];

const marginTargets = holdout.filter((entry) => (
  entry.font === "Arial"
  && entry.face === 0
  && entry.size === 10
  && entry.lineWidth === 0.05
));
const marginErrors = marginTargets.map((target) => marginPrediction(thin, target) - target.retreat);

const lineTraining = [...thin, ...fine, ...survey];
const lineTargets = holdout.filter((entry) => (
  entry.font === "Arial"
  && entry.face === 0
  && entry.size === 10
  && [0, 2].includes(entry.marginWidth)
  && [0.25, 0.75, 1.5, 3].includes(entry.lineWidth)
));
const lineErrorsByMargin = Object.fromEntries([0, 2].map((marginWidth) => {
  const targets = lineTargets.filter((entry) => entry.marginWidth === marginWidth);
  return [marginWidth, errorSummary(targets.map((target) => lineWidthPrediction(lineTraining, target) - target.retreat))];
}));

// Similarity law: R(s,m,w,theta) = s * F(m/s,w/s,theta).  Map the held-out
// 14pt cases to equivalent 10pt parameters and interpolate the two measured
// line-width planes.  Survey angles are 15 degrees, so use their common 30
// degree subset for this independent check.
const sizeTargets = holdout.filter((entry) => entry.size === 14 && entry.angleDeg % 30 === 0);
const scale = 14 / 10;
const equivalentMargin = 2 / scale;
const equivalentLineWidth = 1 / scale;
const lineMix = (equivalentLineWidth - 0.05) / (1 - 0.05);
const sizeErrors = sizeTargets.map((target) => {
  const thinPrediction = marginPrediction(thin, target, { size: 10, lineWidth: 0.05, marginWidth: equivalentMargin });
  const lineOnePrediction = marginPrediction(survey, target, { size: 10, lineWidth: 1, marginWidth: equivalentMargin });
  const prediction = (thinPrediction * (1 - lineMix) + lineOnePrediction * lineMix) * scale;
  return prediction - target.retreat;
});

const fontComparisonFields = ["size", "face", "marginWidth", "lineWidth", "glyph", "angleDeg"];
const fontGrid = comprehensive.filter((entry) => (
  entry.face === 0
  && [8, 14, 24].includes(entry.size)
  && [0.75, 2.5].includes(entry.marginWidth)
  && entry.lineWidth === 1
));
const fontBaseline = fontGrid.filter((entry) => entry.font === "Arial");
const fontDifferencesFromArial = Object.fromEntries(
  [...new Set(fontGrid.map((entry) => entry.font))]
    .filter((font) => font !== "Arial")
    .map((font) => [font, errorSummary(matchedDifferences(
      fontBaseline,
      fontGrid.filter((entry) => entry.font === font),
      fontComparisonFields,
    ))]),
);

const faceComparisonFields = ["font", "size", "marginWidth", "lineWidth", "glyph", "angleDeg"];
const faceGrid = comprehensive.filter((entry) => (
  entry.size === 10 && entry.marginWidth === 1.6 && entry.lineWidth === 1
));
const faceDifferencesFromRegular = Object.fromEntries(
  [...new Set(faceGrid.map((entry) => entry.font))].map((font) => {
    const family = faceGrid.filter((entry) => entry.font === font);
    const regular = family.filter((entry) => entry.face === 0);
    return [font, Object.fromEntries([1, 2, 3].map((face) => [face, errorSummary(matchedDifferences(
      regular,
      family.filter((entry) => entry.face === face),
      faceComparisonFields,
    ))]))];
  }),
);

const anchorComparisonFields = ["glyph", "font", "size", "face", "marginWidth", "lineWidth", "angleDeg"];
const leftAnchors = anchoredDirectional.filter((entry) => entry.anchorPosition === "left");
const anchorDifferencesFromLeft = Object.fromEntries(["middle", "right"].map((anchorPosition) => [
  anchorPosition,
  errorSummary(matchedFieldDifferences(
    leftAnchors,
    anchoredDirectional.filter((entry) => entry.anchorPosition === anchorPosition),
    anchorComparisonFields,
    "effectiveEndpointDisplacement",
  )),
]));
const anchorAngularDeflection = Object.fromEntries(["left", "middle", "right"].map((anchorPosition) => [
  anchorPosition,
  valueSummary(anchoredDirectional
    .filter((entry) => entry.anchorPosition === anchorPosition)
    .map((entry) => entry.angularDeflectionDeg)),
]));

const report = {
  schema: "chemsema.chemdraw-label-retreat-analysis.v1",
  sampleCounts: {
    survey: survey.length,
    fine: fine.length,
    thin: thin.length,
    holdout: holdout.length,
    total: survey.length + fine.length + thin.length + holdout.length,
    comprehensive: comprehensive.length,
    directional: directional.length,
    anchoredDirectional: anchoredDirectional.length,
    grandTotal: survey.length + fine.length + thin.length + holdout.length + comprehensive.length
      + directional.length + anchoredDirectional.length,
  },
  marginInterpolationHoldout: errorSummary(marginErrors),
  lineWidthInterpolationHoldoutByMargin: lineErrorsByMargin,
  normalizedSize14Holdout: errorSummary(sizeErrors),
  comprehensiveCoverage: comprehensiveResult?.coverage ?? null,
  directionalCoverage: directionalResult?.coverage ?? null,
  anchoredDirectionalCoverage: anchoredDirectionalResult?.coverage ?? null,
  fontDifferencesFromArial,
  faceDifferencesFromRegular,
  anchorDifferencesFromLeft,
  anchorAngularDeflection,
};

fs.writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify({ outputPath, ...report }, null, 2));
