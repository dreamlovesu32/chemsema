export function createTextEditorLayoutHost(scope) {
  const { getActiveTextEditor, editorState, routeEditorPointerEvents, state, svgPointFromEvent, currentEditorRenderList, renderEditorOverlay, openTextEditorAt, createEditorSourceRunsFromSession, normalizeEditorSourceRuns, editorRootFontFamily, cssColorToHex, renderSecondaryToolbar, worldToLayerPoint, getSharedGlyphProfiles, zoomScale, textLength, setActiveEditorSelection, renderActiveTextEditorFromModel, currentEditorSelectionOffsets, document } = scope;

  function commandResultForTextEditorTarget(target) {
    if (!target) {
      return null;
    }
    if (target.kind === "text-object") {
      const objectId = target.objectId || target.object_id || null;
      return objectId ? { changed: true, targets: { objects: [objectId] } } : null;
    }
    if (target.kind === "endpoint-label") {
      const nodeId = target.nodeId || target.node_id || null;
      return nodeId ? { changed: true, targets: { nodes: [nodeId] } } : null;
    }
    return null;
  }

  function textEditPrimitiveNodeId(primitive) {
    return primitive?.nodeId || primitive?.node_id || null;
  }

  function textEditPrimitiveObjectId(primitive) {
    return primitive?.objectId || primitive?.object_id || null;
  }

  function textEditHoverPrimitiveFromRenderList(renderList) {
    const hoverRoles = new Set(["hover-text-box", "hover-label-glyph", "hover-endpoint"]);
    return (renderList || []).find((primitive) => hoverRoles.has(primitive?.role)) || null;
  }

  function activeTextEditorTargetMatchesHoverPrimitive(primitive) {
    const target = getActiveTextEditor()?.session?.target;
    if (!target || !primitive) {
      return false;
    }
    const role = primitive.role;
    if (role === "hover-text-box" || role === "hover-label-glyph") {
      const objectId = textEditPrimitiveObjectId(primitive);
      if (target.kind === "text-object" && objectId) {
        return objectId === (target.objectId || target.object_id || null);
      }
      const nodeId = textEditPrimitiveNodeId(primitive);
      if (target.kind === "endpoint-label" && nodeId) {
        return nodeId === (target.nodeId || target.node_id || null);
      }
      return false;
    }
    if (role === "hover-endpoint" && target.kind === "endpoint-label" && primitive.center) {
      const dx = Number(primitive.center.x) - Number(target.x);
      const dy = Number(primitive.center.y) - Number(target.y);
      return Math.hypot(dx, dy) <= 0.001;
    }
    return false;
  }

  async function updateTextToolHoverFromPointerEvent(event) {
    if (!routeEditorPointerEvents() || editorState.activeTool !== "text" || !state.editorEngine?.pointerMove) {
      return null;
    }
    const point = svgPointFromEvent(event);
    await state.editorEngine.pointerMove(point.x, point.y, event.altKey);
    const renderList = currentEditorRenderList();
    renderEditorOverlay(renderList);
    positionActiveTextEditor();
    return textEditHoverPrimitiveFromRenderList(renderList);
  }

  async function openHoveredTextEditTargetFromPointerEvent(event) {
    const hoverPrimitive = await updateTextToolHoverFromPointerEvent(event);
    if (!hoverPrimitive || activeTextEditorTargetMatchesHoverPrimitive(hoverPrimitive)) {
      return false;
    }
    event.preventDefault();
    event.stopPropagation();
    await openTextEditorAt(svgPointFromEvent(event));
    return true;
  }

  function editorSourceRunsFromSession(session, root) {
    return createEditorSourceRunsFromSession(session, root, {
      defaultFontFamily: editorState.textFontFamily,
      defaultFontSize: editorState.textFontSize,
      defaultTextColor: editorState.textColor,
      normalizeRuns: normalizeEditorSourceRuns,
      baseStyle: editorRootBaseStyle,
    });
  }

  function editorRootBaseStyle(root) {
    const baseFontSize = Number.parseFloat(root?.dataset?.baseFontSize || `${editorState.textFontSize}`)
      || editorState.textFontSize;
    return {
      fontFamily: editorRootFontFamily(root),
      fontSize: baseFontSize,
      fill: cssColorToHex(root.style.color || editorState.textColor),
      fontWeight: 400,
      fontStyle: "normal",
      underline: false,
      outline: false,
      shadow: false,
      script: root.dataset.defaultChemical === "true" ? "chemical" : "normal",
    };
  }

  function syncTextToolbarStateFromSession(session) {
    const firstRun = Array.isArray(session.sourceRuns) ? session.sourceRuns[0] : null;
    editorState.textFontFamily = firstRun?.fontFamily || session.fontFamily || editorState.textFontFamily;
    const fontSize = Number(firstRun?.fontSize || session.fontSize);
    if (Number.isFinite(fontSize) && fontSize > 0) {
      editorState.textFontSize = fontSize;
    }
    editorState.textColor = firstRun?.fill || session.fill || editorState.textColor;
    editorState.textAlign = session.align || "left";
    editorState.textScript = firstRun?.script || (session.defaultChemical ? "chemical" : "normal");
    editorState.textBold = Number(firstRun?.fontWeight || 400) >= 600;
    editorState.textItalic = firstRun?.fontStyle === "italic";
    editorState.textUnderline = Boolean(firstRun?.underline);
    editorState.textOutline = Boolean(firstRun?.outline);
    editorState.textShadow = Boolean(firstRun?.shadow);
    renderSecondaryToolbar();
  }

  function positionActiveTextEditor() {
    if (!getActiveTextEditor()?.root) {
      return;
    }
    const { target } = getActiveTextEditor().session;
    const point = worldToLayerPoint({ x: target.x, y: target.y });
    if (!point) {
      return;
    }
    const root = getActiveTextEditor().root;
    const align = root.style.textAlign || "left";
    const anchorOffset = getActiveTextEditor().layout?.anchorOffset || { x: 0, y: 0 };
    const scale = editorDisplayScale();
    root.style.left = `${point.x}px`;
    root.style.top = `${point.y}px`;
    root.style.transform = `translate(${-anchorOffset.x * scale}px, ${-anchorOffset.y * scale}px) scale(${scale})`;
    root.dataset.anchor = align === "right"
      ? "end"
      : align === "center"
        ? "middle"
        : "start";
  }

  function syncEditorVisualMetrics() {
    if (!getActiveTextEditor()?.root) {
      return;
    }
    const root = getActiveTextEditor().root;
    const baseFontSize = Number.parseFloat(root.dataset.baseFontSize || `${editorState.textFontSize}`)
      || editorState.textFontSize;
    const baseLineHeight = Number.parseFloat(root.dataset.baseLineHeight || `${defaultTextEditorLineHeight(baseFontSize)}`)
      || defaultTextEditorLineHeight(baseFontSize);
    root.style.fontSize = `${baseFontSize}px`;
    root.style.lineHeight = `${baseLineHeight}px`;
    root.style.minHeight = `${baseLineHeight}px`;
  }

  function syncTextEditorSize() {
    if (!getActiveTextEditor()?.root) {
      return;
    }
    syncEditorVisualMetrics();
    const root = getActiveTextEditor().root;
    const display = getActiveTextEditor().display || root;
    const layout = getActiveTextEditor().layout;
    const width = Math.max(8, Math.ceil(Number(layout?.width || 0)));
    const height = Math.max(
      Number.parseFloat(root.style.minHeight || "15"),
      Math.ceil(Number(layout?.height || 0) || 0),
    );
    root.dataset.renderWidth = String(width);
    root.dataset.renderOffsetX = "0";
    root.dataset.renderOffsetY = "0";
    getActiveTextEditor().renderOffset = { x: 0, y: 0 };
    root.style.width = `${width}px`;
    root.style.height = `${height}px`;
    display.style.width = `${width}px`;
    display.style.height = `${height}px`;
    const svg = display.querySelector?.('svg[data-editor-text-svg="true"]');
    if (svg) {
      svg.setAttribute("width", String(width));
      svg.setAttribute("height", String(height));
      svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
    }
    updateCustomEditorChrome();
    positionActiveTextEditor();
  }

  function defaultTextEditorLineHeight(fontSize) {
    const size = Number(fontSize || editorState.textFontSize) || editorState.textFontSize;
    return Math.max(size, size * 1.05);
  }

  function editorDisplayScale() {
    return Math.max(0.01, zoomScale());
  }

  function editorGlyphProfiles() {
    if (!getSharedGlyphProfiles()) {
      throw new Error("Shared glyph profiles have not loaded yet");
    }
    return getSharedGlyphProfiles();
  }

  function editorGlyphLayoutConfig() {
    return editorGlyphProfiles().layout;
  }

  function buildEditorTextLayout() {
    return getActiveTextEditor()?.layout || null;
  }

  function placeCaretAtEnd(element) {
    if (!getActiveTextEditor()) {
      return;
    }
    const offset = textLength(getActiveTextEditor().plainText);
    setActiveEditorSelection({ anchor: offset, focus: offset }, false);
    renderActiveTextEditorFromModel();
  }

  function selectAllEditorText(element) {
    if (!getActiveTextEditor()) {
      return;
    }
    setActiveEditorSelection({ anchor: 0, focus: textLength(getActiveTextEditor().plainText) }, false);
    renderActiveTextEditorFromModel();
  }

  function captureEditorCaretOffset(root) {
    const selectionOffsets = currentEditorSelectionOffsets();
    if (!selectionOffsets || !selectionOffsets.collapsed) {
      return null;
    }
    return selectionOffsets.anchor;
  }

  function restoreEditorCaretOffset(root, offset) {
    if (!Number.isFinite(offset)) {
      placeCaretAtEnd(root);
      return;
    }
    setActiveEditorSelection({ anchor: offset, focus: offset }, true);
  }

  function updateCustomEditorChrome() {
    if (!getActiveTextEditor()?.root || !getActiveTextEditor().display || !getActiveTextEditor().caret || !getActiveTextEditor().input) {
      return;
    }
    const selection = currentEditorSelectionOffsets();
    const caret = getActiveTextEditor().caret;
    const selectionLayer = getActiveTextEditor().selectionLayer;
    const input = getActiveTextEditor().input;
    if (selectionLayer) {
      selectionLayer.replaceChildren();
    }
    if (!selection || !selection.collapsed) {
      caret.style.display = "none";
      renderEditorSelectionSegments(selection, selectionLayer);
      const focusRect = measureEditorCaretRect(selection?.focus ?? textLength(getActiveTextEditor().plainText));
      positionHiddenEditorInput(focusRect);
      return;
    }
    const caretRect = measureEditorCaretRect(selection.focus);
    if (!caretRect) {
      caret.style.display = "none";
      positionHiddenEditorInput(null);
      return;
    }
    caret.style.display = "block";
    caret.style.left = `${caretRect.x}px`;
    caret.style.top = `${caretRect.y}px`;
    caret.style.height = `${caretRect.height}px`;
    positionHiddenEditorInput(caretRect);
  }

  function renderEditorSelectionSegments(selection, selectionLayer) {
    if (!selection || selection.collapsed || !selectionLayer) {
      return;
    }
    const layout = buildEditorTextLayout();
    if (!layout) {
      return;
    }
    for (const segment of layout.selectionRects || []) {
      const node = document.createElement("div");
      node.className = "text-editor-selection-segment";
      node.style.left = `${segment.x}px`;
      node.style.top = `${segment.y}px`;
      node.style.width = `${Math.max(1, segment.width)}px`;
      node.style.height = `${Math.max(1, segment.height)}px`;
      selectionLayer.appendChild(node);
    }
  }

  function positionHiddenEditorInput(caretRect) {
    if (!getActiveTextEditor()?.input) {
      return;
    }
    const input = getActiveTextEditor().input;
    if (!caretRect) {
      input.style.left = "0px";
      input.style.top = "0px";
      return;
    }
    input.style.left = `${caretRect.x}px`;
    input.style.top = `${caretRect.y}px`;
    input.style.height = `${Math.max(1, caretRect.height)}px`;
  }

  function measureEditorCaretRect(offset) {
    const layout = buildEditorTextLayout();
    if (!layout) {
      return null;
    }
    const caret = layout.caretPositions?.find((entry) => entry.offset === offset)
      || layout.caretPositions?.[Math.max(0, Math.min((layout.caretPositions?.length || 1) - 1, offset))];
    if (!caret) {
      return null;
    }
    return {
      x: caret.x,
      y: caret.y,
      width: 0,
      height: caret.height,
    };
  }

  function buildEditorCaretLayout() {
    const layout = buildEditorTextLayout();
    if (!layout) {
      return null;
    }
    return layout;
  }

  function editorLineIndexForOffset(offset) {
    const layout = buildEditorCaretLayout();
    if (!layout) {
      return -1;
    }
    for (let index = 0; index < layout.lines.length; index += 1) {
      const line = layout.lines[index];
      if (line.caretOffsets?.some((entry) => entry.offset === offset)) {
        return index;
      }
    }
    return layout.lines.length - 1;
  }

  function nearestOffsetOnLine(line, targetX) {
    if (!line?.caretOffsets?.length) {
      return 0;
    }
    return line.caretOffsets.reduce((best, entry) => {
      const bestDistance = Math.abs(best.x - targetX);
      const nextDistance = Math.abs(entry.x - targetX);
      if (nextDistance < bestDistance) {
        return entry;
      }
      return best;
    }).offset;
  }

  function editorOffsetFromPointerEvent(event) {
    const layout = buildEditorCaretLayout();
    if (!getActiveTextEditor()?.display || !layout) {
      return 0;
    }
    const rect = getActiveTextEditor().display.getBoundingClientRect();
    const scale = editorDisplayScale();
    const localX = (event.clientX - rect.left) / scale;
    const localY = (event.clientY - rect.top) / scale;
    let line = layout.lines[0];
    let bestDistance = Number.POSITIVE_INFINITY;
    for (const candidate of layout.lines) {
      const centerY = candidate.y + candidate.height * 0.5;
      const distance = Math.abs(centerY - localY);
      if (distance < bestDistance) {
        bestDistance = distance;
        line = candidate;
      }
    }
    if (!line) {
      return 0;
    }
    return nearestOffsetOnLine(line, localX);
  }

  return { commandResultForTextEditorTarget, textEditPrimitiveNodeId, textEditPrimitiveObjectId, textEditHoverPrimitiveFromRenderList, activeTextEditorTargetMatchesHoverPrimitive, updateTextToolHoverFromPointerEvent, openHoveredTextEditTargetFromPointerEvent, editorSourceRunsFromSession, editorRootBaseStyle, syncTextToolbarStateFromSession, positionActiveTextEditor, syncEditorVisualMetrics, syncTextEditorSize, defaultTextEditorLineHeight, editorDisplayScale, editorGlyphProfiles, editorGlyphLayoutConfig, buildEditorTextLayout, placeCaretAtEnd, selectAllEditorText, captureEditorCaretOffset, restoreEditorCaretOffset, updateCustomEditorChrome, renderEditorSelectionSegments, positionHiddenEditorInput, measureEditorCaretRect, buildEditorCaretLayout, editorLineIndexForOffset, nearestOffsetOnLine, editorOffsetFromPointerEvent };
}
