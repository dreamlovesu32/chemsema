// Browser layout uses CSS pixels. OS/browser scaling affects device pixels via
// devicePixelRatio, while CSS geometry remains 96 CSS px per CSS inch.
export const CSS_PX_PER_INCH = 96.0;
export const CM_PER_INCH = 2.54;

export const CSS_PX_PER_CM = CSS_PX_PER_INCH / CM_PER_INCH;
export const CM_PER_CSS_PX = CM_PER_INCH / CSS_PX_PER_INCH;

export const devicePixelRatioValue = () =>
  Number(globalThis.window?.devicePixelRatio || globalThis.devicePixelRatio || 1) || 1;

export const devicePxPerInch = () => CSS_PX_PER_INCH * devicePixelRatioValue();
export const devicePxPerCm = () => CSS_PX_PER_CM * devicePixelRatioValue();

export const cmToCssPx = (cm) => cm * CSS_PX_PER_CM;
export const cssPxToCm = (px) => px * CM_PER_CSS_PX;

export const cmToDevicePx = (cm) => cmToCssPx(cm) * devicePixelRatioValue();
export const devicePxToCm = (px) => cssPxToCm(px / devicePixelRatioValue());

export const displayMetrics = () => {
  const devicePixelRatio = devicePixelRatioValue();
  return {
    cssPxPerInch: CSS_PX_PER_INCH,
    cssPxPerCm: CSS_PX_PER_CM,
    cmPerCssPx: CM_PER_CSS_PX,
    devicePixelRatio,
    displayScalePercent: devicePixelRatio * 100,
    devicePxPerInch: CSS_PX_PER_INCH * devicePixelRatio,
    devicePxPerCm: CSS_PX_PER_CM * devicePixelRatio,
  };
};

export const cmToPx = cmToCssPx;
export const pxToCm = cssPxToCm;

export const mapLengthArray = (values, convert) =>
  Array.isArray(values) ? values.map((value) => convert(Number(value || 0))) : values;
