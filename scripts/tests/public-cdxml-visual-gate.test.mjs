import assert from "node:assert/strict";
import test from "node:test";
import {
  boundedLocalTopologyEquivalent,
  nearExactFixedDefectEquivalent,
} from "../public-cdxml-visual-gate.mjs";

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

test("near-exact fixed defects ignore sparse-window percentages", () => {
  const coarse = metrics({ coverage: 0.994, missingSpan: 15, extraSpan: 15 });
  coarse.largestMissing.area = 18;
  coarse.largestExtra.area = 18;
  assert.equal(nearExactFixedDefectEquivalent(coarse), true);
});

test("near-exact defects remain bounded independently of image size", () => {
  const coarse = metrics({ coverage: 0.9999, missingSpan: 15.01, extraSpan: 1 });
  coarse.largestMissing.area = 1;
  coarse.largestExtra.area = 1;
  coarse.totals = { referenceInk: 10_000_000, candidateInk: 10_000_000 };
  assert.equal(nearExactFixedDefectEquivalent(coarse), false);
});
