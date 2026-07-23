export function createBracketHitGeometry(scope) {
  const { pointDistance } = scope;

  function rotatePointAround(point, center, degrees) {
    const radians = degrees * Math.PI / 180;
    const cos = Math.cos(radians);
    const sin = Math.sin(radians);
    const dx = point.x - center.x;
    const dy = point.y - center.y;
    return {
      x: center.x + dx * cos - dy * sin,
      y: center.y + dx * sin + dy * cos,
    };
  }

  function pointToSegmentDistance(point, start, end) {
    const dx = end.x - start.x;
    const dy = end.y - start.y;
    const lengthSq = dx * dx + dy * dy;
    if (lengthSq <= 1e-9) {
      return pointDistance(point, start);
    }
    const t = Math.max(0, Math.min(1, (((point.x - start.x) * dx) + ((point.y - start.y) * dy)) / lengthSq));
    return pointDistance(point, {
      x: start.x + dx * t,
      y: start.y + dy * t,
    });
  }

  function bracketPairLip(width, height) {
    return Math.max(0, Math.min(height * 0.07248, width * 0.22));
  }

  function bracketPairDepth(width, height, kind) {
    if (kind === "curly") {
      return Math.max(0, Math.min(height * 0.14423, width * 0.24));
    }
    return Math.max(0, Math.min(height * (1 - Math.sqrt(3) * 0.5), width * 0.22));
  }

  function bracketStrokeHitPadding(object) {
    const strokeWidth = Number(object?.payload?.strokeWidth ?? object?.payload?.extra?.strokeWidth ?? 1);
    return Number.isFinite(strokeWidth) && strokeWidth > 0 ? strokeWidth * 0.5 : 0.5;
  }

  function bracketSideHandleX(kind, side, width) {
    if (kind === "round") {
      return side === "right" ? 0 : width;
    }
    return side === "right" ? width : 0;
  }

  function squareBracketSideLocalHit(point, x, y, width, height, side, pad) {
    const right = x + width;
    const bottom = y + height;
    if (side === "right") {
      return pointToSegmentDistance(point, { x: right, y }, { x: right, y: bottom }) <= pad
        || pointToSegmentDistance(point, { x, y }, { x: right, y }) <= pad
        || pointToSegmentDistance(point, { x, y: bottom }, { x: right, y: bottom }) <= pad;
    }
    return pointToSegmentDistance(point, { x, y }, { x, y: bottom }) <= pad
      || pointToSegmentDistance(point, { x, y }, { x: right, y }) <= pad
      || pointToSegmentDistance(point, { x, y: bottom }, { x: right, y: bottom }) <= pad;
  }

  function roundBracketSidePolyline(x, y, width, height, side) {
    const chordHalf = height * 0.5;
    const base = Math.sqrt(Math.max(0, height * height - chordHalf * chordHalf));
    const sampleCount = 24;
    const points = [];
    for (let index = 0; index <= sampleCount; index += 1) {
      const t = index / sampleCount;
      const dy = (t - 0.5) * height;
      const sagitta = Math.max(0, Math.sqrt(Math.max(0, height * height - dy * dy)) - base);
      const clampedSagitta = Math.min(width, sagitta);
      points.push({
        x: side === "right" ? x + clampedSagitta : x + width - clampedSagitta,
        y: y + height * t,
      });
    }
    return points;
  }

  function cubicPoint(p0, p1, p2, p3, t) {
    const mt = 1 - t;
    const mt2 = mt * mt;
    const t2 = t * t;
    return {
      x: p0.x * mt2 * mt + p1.x * 3 * mt2 * t + p2.x * 3 * mt * t2 + p3.x * t2 * t,
      y: p0.y * mt2 * mt + p1.y * 3 * mt2 * t + p2.y * 3 * mt * t2 + p3.y * t2 * t,
    };
  }

  function appendCubicSamples(points, p0, p1, p2, p3) {
    const sampleCount = 8;
    const start = points.length ? 1 : 0;
    for (let index = start; index <= sampleCount; index += 1) {
      points.push(cubicPoint(p0, p1, p2, p3, index / sampleCount));
    }
  }

  function curlyBracketSidePolyline(x, y, width, height, side) {
    const right = x + width;
    const bottom = y + height;
    const halfDepth = width * 0.5;
    const middle = y + height * 0.5;
    const cLarge = height * 0.039805;
    const cSmall = height * 0.032308;
    const topInner = y + halfDepth;
    const bottomInner = bottom - halfDepth;
    const points = [];
    if (side === "right") {
      const re = x;
      const rm = x + halfDepth;
      appendCubicSamples(points, { x: re, y: bottom }, { x: re + cLarge, y: bottom }, { x: rm, y: bottom - cSmall }, { x: rm, y: bottomInner });
      appendCubicSamples(points, { x: rm, y: bottomInner }, { x: rm, y: bottomInner }, { x: rm, y: middle + halfDepth }, { x: rm, y: middle + halfDepth });
      appendCubicSamples(points, { x: rm, y: middle + halfDepth }, { x: rm, y: middle + halfDepth - cLarge }, { x: rm + cSmall, y: middle }, { x: right, y: middle });
      appendCubicSamples(points, { x: right, y: middle }, { x: rm + cSmall, y: middle }, { x: rm, y: middle - halfDepth + cLarge }, { x: rm, y: middle - halfDepth });
      appendCubicSamples(points, { x: rm, y: middle - halfDepth }, { x: rm, y: middle - halfDepth }, { x: rm, y: y + cSmall }, { x: re + cLarge, y });
      appendCubicSamples(points, { x: re + cLarge, y }, { x: re, y }, { x: re, y }, { x: re, y });
      return points;
    }
    const le = right;
    const lm = x + halfDepth;
    appendCubicSamples(points, { x: le, y }, { x: le - cLarge, y }, { x: lm, y: y + cSmall }, { x: lm, y: topInner });
    appendCubicSamples(points, { x: lm, y: topInner }, { x: lm, y: topInner }, { x: lm, y: middle - halfDepth }, { x: lm, y: middle - halfDepth });
    appendCubicSamples(points, { x: lm, y: middle - halfDepth }, { x: lm, y: middle - halfDepth + cLarge }, { x: lm - cSmall, y: middle }, { x, y: middle });
    appendCubicSamples(points, { x, y: middle }, { x: lm - cSmall, y: middle }, { x: lm, y: middle + halfDepth - cLarge }, { x: lm, y: middle + halfDepth });
    appendCubicSamples(points, { x: lm, y: middle + halfDepth }, { x: lm, y: middle + halfDepth }, { x: lm, y: bottom - cSmall }, { x: le - cLarge, y: bottom });
    appendCubicSamples(points, { x: le - cLarge, y: bottom }, { x: le, y: bottom }, { x: le, y: bottom }, { x: le, y: bottom });
    return points;
  }

  function pointToPolylineDistance(point, points) {
    let distance = Infinity;
    for (let index = 1; index < points.length; index += 1) {
      distance = Math.min(distance, pointToSegmentDistance(point, points[index - 1], points[index]));
    }
    return distance;
  }

  function bracketSideLocalHit(point, x, y, width, height, kind, side, pad) {
    if (width <= 0 || height <= 0) {
      return false;
    }
    if (kind === "square") {
      return squareBracketSideLocalHit(point, x, y, width, height, side, pad);
    }
    const points = kind === "curly"
      ? curlyBracketSidePolyline(x, y, width, height, side)
      : roundBracketSidePolyline(x, y, width, height, side);
    return pointToPolylineDistance(point, points) <= pad;
  }

  function bracketPairLocalHit(point, x, y, width, height, kind, pad) {
    const right = x + width;
    const bottom = y + height;
    if (kind === "square") {
      const lip = bracketPairLip(width, height);
      return pointToSegmentDistance(point, { x, y }, { x, y: bottom }) <= pad
        || pointToSegmentDistance(point, { x: right, y }, { x: right, y: bottom }) <= pad
        || pointToSegmentDistance(point, { x, y }, { x: x + lip, y }) <= pad
        || pointToSegmentDistance(point, { x, y: bottom }, { x: x + lip, y: bottom }) <= pad
        || pointToSegmentDistance(point, { x: right - lip, y }, { x: right, y }) <= pad
        || pointToSegmentDistance(point, { x: right - lip, y: bottom }, { x: right, y: bottom }) <= pad;
    }
    const depth = bracketPairDepth(width, height, kind);
    const leftX = kind === "round" ? x - depth : x;
    const rightX = kind === "round" ? right : right - depth;
    return bracketSideLocalHit(point, leftX, y, depth, height, kind, "left", pad)
      || bracketSideLocalHit(point, rightX, y, depth, height, kind, "right", pad);
  }

  return { rotatePointAround, pointToSegmentDistance, bracketPairLip, bracketPairDepth, bracketStrokeHitPadding, bracketSideHandleX, squareBracketSideLocalHit, roundBracketSidePolyline, cubicPoint, appendCubicSamples, curlyBracketSidePolyline, pointToPolylineDistance, bracketSideLocalHit, bracketPairLocalHit };
}
