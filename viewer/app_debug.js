export function registerChemcoreDebug({
  state,
  getEngineState,
  getActiveTextEditor,
  getActiveSelectionGesture,
  getDisplayMetrics,
  engineHost,
  desktopFileHost,
  commandEngine,
  insertEditorText,
  syncDocument,
  loadDocumentForTest,
  renderStats,
  getRenderListJson,
  worldToClient,
}) {
  if (typeof window === "undefined") {
    return;
  }

  window.__chemcoreDebug = {
    state,
    get document() {
      return state.currentDocument;
    },
    get engineState() {
      return getEngineState();
    },
    get activeTextEditor() {
      return getActiveTextEditor();
    },
    get activeSelectionGesture() {
      return getActiveSelectionGesture();
    },
    get displayMetrics() {
      return getDisplayMetrics();
    },
    get engineHost() {
      return engineHost;
    },
    get desktopFileHost() {
      return desktopFileHost;
    },
    get commandEngine() {
      return commandEngine;
    },
    insertEditorText,
    syncDocument,
    loadDocumentForTest,
    renderStats,
    getRenderListJson,
    worldToClient,
  };
}
