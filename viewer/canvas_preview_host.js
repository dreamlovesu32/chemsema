export function createCanvasPreviewHost(scope) {
  const { activeGestureUsesObjectEditPreview, currentEditorInteractionRenderList, syncObjectEditPreviewHiddenElements, editorOverlayRenderer, canvasDragPreviewSvg, window, makeSvgNode, viewerSvg, renderCorePrimitive, corePrimitiveRenderOptions } = scope;

  function renderEditorOverlay(renderList = null) {
    const objectEditPreviewActive = activeGestureUsesObjectEditPreview();
    const effectiveRenderList = objectEditPreviewActive && !renderList
      ? currentEditorInteractionRenderList()
      : renderList;
    syncObjectEditPreviewHiddenElements(objectEditPreviewActive ? effectiveRenderList || [] : []);
    editorOverlayRenderer.renderEditorOverlay(effectiveRenderList);
  }

  function syncCanvasDragPreviewViewport() {
    canvasDragPreviewSvg.style.left = "0";
    canvasDragPreviewSvg.style.top = "0";
    canvasDragPreviewSvg.style.width = "100vw";
    canvasDragPreviewSvg.style.height = "100vh";
    canvasDragPreviewSvg.setAttribute("viewBox", `0 0 ${window.innerWidth} ${window.innerHeight}`);
  }

  function clearCanvasDragPreview() {
    const hadPreview = canvasDragPreviewSvg.childElementCount > 0
      || canvasDragPreviewSvg.hasAttribute("viewBox");
    canvasDragPreviewSvg.replaceChildren();
    canvasDragPreviewSvg.removeAttribute("viewBox");
    return hadPreview;
  }

  function screenPointFromSvgMatrix(point, matrix) {
    return {
      x: Number(point?.x || 0) * matrix.a + Number(point?.y || 0) * matrix.c + matrix.e,
      y: Number(point?.x || 0) * matrix.b + Number(point?.y || 0) * matrix.d + matrix.f,
    };
  }

  function canvasScreenFeedbackPrimitiveNode(primitive, matrix) {
    if (!matrix || primitive?.kind !== "circle" || !primitive.center) {
      return null;
    }
    if (primitive.role !== "preview-end" && primitive.role !== "hover-endpoint") {
      return null;
    }
    const center = screenPointFromSvgMatrix(primitive.center, matrix);
    const scale = Math.hypot(matrix.a || 0, matrix.b || 0);
    const radius = Number(primitive.radius || 0) * scale;
    return makeSvgNode("circle", {
      cx: center.x,
      cy: center.y,
      r: radius,
      class: "editor-endpoint-halo",
      "data-role": primitive.role,
    });
  }

  function renderCanvasDragPreview(renderList = []) {
    canvasDragPreviewSvg.replaceChildren();
    if (!renderList?.length) {
      return;
    }
    syncCanvasDragPreviewViewport();
    const matrix = viewerSvg.getScreenCTM?.();
    const target = matrix
      ? makeSvgNode("g", {
          transform: `matrix(${matrix.a} ${matrix.b} ${matrix.c} ${matrix.d} ${matrix.e} ${matrix.f})`,
        })
      : canvasDragPreviewSvg;
    const screenFeedbackNodes = [];
    for (const primitive of renderList) {
      const feedbackNode = canvasScreenFeedbackPrimitiveNode(primitive, matrix);
      if (feedbackNode) {
        screenFeedbackNodes.push(feedbackNode);
        continue;
      }
      renderCorePrimitive(target, primitive, corePrimitiveRenderOptions());
    }
    if (target !== canvasDragPreviewSvg) {
      canvasDragPreviewSvg.appendChild(target);
    }
    for (const node of screenFeedbackNodes) {
      canvasDragPreviewSvg.appendChild(node);
    }
  }

  return { renderEditorOverlay, syncCanvasDragPreviewViewport, clearCanvasDragPreview, screenPointFromSvgMatrix, canvasScreenFeedbackPrimitiveNode, renderCanvasDragPreview };
}
