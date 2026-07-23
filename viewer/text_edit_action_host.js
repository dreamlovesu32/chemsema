export function createTextEditActionHost(scope) {
  const { getActiveTextEditor, editorState, syncTextEditorSize, positionActiveTextEditor, getTextEditorController, normalizeEditorSelectionOffsetsModel, state, focusActiveTextEditor, syncEngineToolState, renderSecondaryToolbar, syncCanvasCursor, setCanvasPointerShieldActive, canvasPointerShield, clearCanvasDragPreview, syncViewerSvgPointerEventMode, syncEditorPrimaryToolButtons } = scope;

  function applyTextAlignment(align) {
    if (!getActiveTextEditor()?.root) {
      return;
    }
    getActiveTextEditor().root.style.textAlign = align === "justify" ? "justify" : align;
    syncTextEditorSize();
    positionActiveTextEditor();
  }

  function currentEditorSelectionOffsets() {
    return getTextEditorController().currentEditorSelectionOffsets();
  }

  function restoreEditorSelectionOffsets(selectionOffsets) {
    setActiveEditorSelection(selectionOffsets, false);
  }

  function normalizeEditorSelectionOffsets(selectionOffsets) {
    if (!getActiveTextEditor()) {
      return null;
    }
    return normalizeEditorSelectionOffsetsModel(getActiveTextEditor().plainText, selectionOffsets);
  }

  function setActiveEditorSelection(selectionOffsets, syncDom = true) {
    return getTextEditorController().setActiveEditorSelection(selectionOffsets, syncDom);
  }

  function renderActiveTextEditorFromModel(selectionOffsets = null) {
    getTextEditorController().renderActiveTextEditorFromModel(selectionOffsets);
  }

  function syncPendingEditorStyleWithSelection() {
    getTextEditorController().syncPendingEditorStyleWithSelection();
  }

  function handleTextEditorBeforeInput(event, root) {
    getTextEditorController().handleTextEditorBeforeInput(event, root);
  }

  function applyTextFormatCommand(command) {
    getTextEditorController().applyTextFormatCommand(command);
  }

  function applyTextScript(script) {
    getTextEditorController().applyTextScript(script);
  }

  function applyTextInlineStyle(styles) {
    getTextEditorController().applyTextInlineStyle(styles);
  }

  function applyChemicalFormat() {
    getTextEditorController().applyChemicalFormat();
  }

  function insertTextAtSelection(text) {
    getTextEditorController().insertTextAtSelection(text);
  }

  function insertTextSymbol(character) {
    const symbol = String(character || "");
    if (!symbol) {
      return;
    }
    if (getActiveTextEditor()) {
      getTextEditorController().insertTextAtSelection(symbol);
      focusActiveTextEditor();
      return;
    }
    state.pendingTextSymbol = symbol;
    editorState.activeTool = "text";
    editorState.secondaryToolbarTool = "text";
    void syncEngineToolState();
    renderSecondaryToolbar();
    syncCanvasCursor();
  }

  function insertElementSymbol(symbol) {
    const text = String(symbol || "");
    if (!text) {
      return;
    }
    if (getActiveTextEditor()) {
      getTextEditorController().insertTextAtSelection(text);
      focusActiveTextEditor();
    }
  }

  function setElementPlacementActive(active) {
    const nextActive = Boolean(active) && !getActiveTextEditor();
    setCanvasPointerShieldActive(false);
    canvasPointerShield.classList.remove("is-active");
    clearCanvasDragPreview();
    syncViewerSvgPointerEventMode();
    if (editorState.elementPlacementActive === nextActive) {
      return;
    }
    editorState.elementPlacementActive = nextActive;
    syncEditorPrimaryToolButtons();
    syncCanvasCursor();
  }

  function handleQuickPaletteModeChange({ open, mode, keepElementPlacement = false } = {}) {
    setElementPlacementActive((open && mode === "element") || keepElementPlacement);
    void syncEngineToolState();
  }

  async function selectElementFromQuickPalette(symbol, atomicNumber = null) {
    const normalizedSymbol = String(symbol || "");
    if (!normalizedSymbol) {
      return false;
    }
    const nextAtomicNumber = Number(atomicNumber) || editorState.elementAtomicNumber || 15;
    const changed = editorState.elementSymbol !== normalizedSymbol
      || editorState.elementAtomicNumber !== nextAtomicNumber;
    editorState.elementSymbol = normalizedSymbol;
    editorState.elementAtomicNumber = nextAtomicNumber;
    if (getActiveTextEditor()) {
      insertElementSymbol(normalizedSymbol);
    } else {
      setElementPlacementActive(true);
    }
    await syncEngineToolState();
    renderSecondaryToolbar();
    syncCanvasCursor();
    focusActiveTextEditor();
    return changed;
  }

  return { applyTextAlignment, currentEditorSelectionOffsets, restoreEditorSelectionOffsets, normalizeEditorSelectionOffsets, setActiveEditorSelection, renderActiveTextEditorFromModel, syncPendingEditorStyleWithSelection, handleTextEditorBeforeInput, applyTextFormatCommand, applyTextScript, applyTextInlineStyle, applyChemicalFormat, insertTextAtSelection, insertTextSymbol, insertElementSymbol, setElementPlacementActive, handleQuickPaletteModeChange, selectElementFromQuickPalette };
}
