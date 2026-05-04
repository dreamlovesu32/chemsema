export function pointDistance(a, b) {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

export function boundsCenter(bounds) {
  return {
    x: (bounds.minX + bounds.maxX) / 2,
    y: (bounds.minY + bounds.maxY) / 2,
  };
}

export function boundsSize(bounds) {
  return {
    width: Math.max(0, bounds.maxX - bounds.minX),
    height: Math.max(0, bounds.maxY - bounds.minY),
  };
}

export function boundsToKey(bounds) {
  if (!bounds) {
    return "none";
  }
  return [
    bounds.minX,
    bounds.minY,
    bounds.maxX,
    bounds.maxY,
  ].map((value) => Number(value || 0).toFixed(3)).join(",");
}

export function rectContainsBounds(rect, bounds, epsilon = 0.001) {
  if (!rect || !bounds) {
    return false;
  }
  return bounds.minX >= rect.minX - epsilon
    && bounds.maxX <= rect.maxX + epsilon
    && bounds.minY >= rect.minY - epsilon
    && bounds.maxY <= rect.maxY + epsilon;
}

export function rectIntersectsBounds(rect, bounds, epsilon = 0.001) {
  if (!rect || !bounds) {
    return false;
  }
  return bounds.maxX >= rect.minX - epsilon
    && bounds.minX <= rect.maxX + epsilon
    && bounds.maxY >= rect.minY - epsilon
    && bounds.minY <= rect.maxY + epsilon;
}

export function intersectBounds(a, b) {
  if (!rectIntersectsBounds(a, b)) {
    return null;
  }
  return {
    minX: Math.max(a.minX, b.minX),
    minY: Math.max(a.minY, b.minY),
    maxX: Math.min(a.maxX, b.maxX),
    maxY: Math.min(a.maxY, b.maxY),
  };
}

export function paddedViewBoxFromBounds(bounds, paddingX, paddingY = paddingX, minWidth = 0, minHeight = 0) {
  const padded = {
    x: bounds.minX - paddingX,
    y: bounds.minY - paddingY,
    width: (bounds.maxX - bounds.minX) + paddingX * 2,
    height: (bounds.maxY - bounds.minY) + paddingY * 2,
  };
  if (padded.width < minWidth) {
    padded.x -= (minWidth - padded.width) / 2;
    padded.width = minWidth;
  }
  if (padded.height < minHeight) {
    padded.y -= (minHeight - padded.height) / 2;
    padded.height = minHeight;
  }
  return padded;
}
