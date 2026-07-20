import assert from "node:assert/strict";
import test from "node:test";
import { boundedLocalTopologyEquivalent } from "../public-cdxml-visual-gate.mjs";

function metrics({
  coverage = 0.98,
  missingSpan = 18,
  extraSpan = 18,
  componentDelta = 3,
  relativeCoverage = 0.95,
} = {}) {
  return {
    referenceCoverage: coverage,
    candidateCoverage: coverage,
    largestMissing: { span: missingSpan },
    largestExtra: { span: extraSpan },
    detailFeatures: {
      componentCountDelta: componentDelta,
      relativeComponentMatchCoverage: relativeCoverage,
    },
  };
}

test("bounded local topology accepts small fixed-coordinate defects", () => {
  assert.equal(boundedLocalTopologyEquivalent(metrics()), true);
});

test("bounded local topology is not diluted by a large image", () => {
  const coarse = metrics({ missingSpan: 33, extraSpan: 1 });
  coarse.totals = { referenceInk: 10_000_000, candidateInk: 10_000_000 };
  assert.equal(boundedLocalTopologyEquivalent(coarse), false);
});

test("bounded local topology rejects weak relative structure agreement", () => {
  assert.equal(boundedLocalTopologyEquivalent(metrics({
    componentDelta: 6,
    relativeCoverage: 0.92,
  })), false);
});

test("very tight defects allow a small bounded component mismatch", () => {
  assert.equal(boundedLocalTopologyEquivalent(metrics({
    missingSpan: 16,
    extraSpan: 17,
    componentDelta: 5,
    relativeCoverage: 0.89,
  })), true);
});
