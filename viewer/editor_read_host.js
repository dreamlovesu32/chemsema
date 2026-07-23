export function createEditorReadHost(scope) {
  const { state, editorEngineReadCache, parseEngineJson, isEditingRustDocument, renderBoundsFromEngine, renderListFromEngine, resetDocumentEngine, maybeAutoExpandEditorViewport, isDocumentPreviewPrimitive, primitivesForObject } = scope;

  function currentEditableFragmentData() {
    const documentData = state.currentDocument;
    if (!documentData?.objects || !documentData?.resources) {
      return null;
    }
    const object = documentData.objects.find((candidate) => candidate.type === "molecule" || candidate.object_type === "molecule");
    const resourceRef = object?.payload?.resourceRef || object?.payload?.resource_ref;
    const fragment = resourceRef ? documentData.resources?.[resourceRef]?.data : null;
    if (!object || !fragment?.nodes || !fragment?.bonds) {
      return null;
    }
    return { object, fragment };
  }

  function worldPointForFragmentPosition(object, position) {
    if (!Array.isArray(position) || position.length < 2) {
      return null;
    }
    const translate = object?.transform?.translate || [0, 0];
    return {
      x: Number(translate[0] || 0) + Number(position[0] || 0),
      y: Number(translate[1] || 0) + Number(position[1] || 0),
    };
  }

  function worldPointForFragmentNode(object, node) {
    return worldPointForFragmentPosition(object, node?.position);
  }

  function selectionZoomCenterWorld() {
    const engineState = currentEditorEngineState();
    const selection = engineState?.selection;
    if (!selection || (!selection.nodes?.length && !selection.bonds?.length)) {
      return null;
    }
    const entry = currentEditableFragmentData();
    if (!entry) {
      return null;
    }
    const nodeById = new Map(entry.fragment.nodes.map((node) => [node.id, node]));
    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;
    let hasPoint = false;
  
    function includePoint(point) {
      if (!point) {
        return;
      }
      minX = Math.min(minX, point.x);
      minY = Math.min(minY, point.y);
      maxX = Math.max(maxX, point.x);
      maxY = Math.max(maxY, point.y);
      hasPoint = true;
    }
  
    function worldPointForNode(node) {
      return worldPointForFragmentNode(entry.object, node);
    }
  
    for (const nodeId of selection.nodes || []) {
      includePoint(worldPointForNode(nodeById.get(nodeId)));
    }
  
    for (const bondId of selection.bonds || []) {
      const bond = entry.fragment.bonds.find((candidate) => candidate.id === bondId);
      if (!bond) {
        continue;
      }
      includePoint(worldPointForNode(nodeById.get(bond.begin)));
      includePoint(worldPointForNode(nodeById.get(bond.end)));
    }
  
    if (!hasPoint) {
      return null;
    }
    return {
      x: (minX + maxX) / 2,
      y: (minY + maxY) / 2,
    };
  }

  function editorEngineRevision(engine) {
    if (!engine?.revision) {
      return 0;
    }
    return Number(engine.revision()) || 0;
  }

  function currentEditorEngineReadCache() {
    const engine = state.editorEngine;
    if (!engine) {
      return null;
    }
    const revision = editorEngineRevision(engine);
    const canTrustRevision = typeof engine.revision === "function";
    const stateJson = canTrustRevision ? null : engine.stateJson?.() || "";
    if (
      editorEngineReadCache.engine !== engine
      || editorEngineReadCache.revision !== revision
      || (!canTrustRevision && editorEngineReadCache.stateJson !== stateJson)
    ) {
      editorEngineReadCache.engine = engine;
      editorEngineReadCache.revision = revision;
      editorEngineReadCache.stateJson = stateJson;
      editorEngineReadCache.parsedState = undefined;
      editorEngineReadCache.renderListJson = null;
      editorEngineReadCache.renderList = null;
      editorEngineReadCache.interactionRenderListJson = null;
      editorEngineReadCache.interactionRenderList = null;
      editorEngineReadCache.documentJson = null;
      editorEngineReadCache.parsedDocument = undefined;
      editorEngineReadCache.boundsJsonByScope = new Map();
      editorEngineReadCache.boundsByScope = new Map();
    }
    return editorEngineReadCache;
  }

  function currentEditorEngineState() {
    const cache = currentEditorEngineReadCache();
    if (!cache) {
      return null;
    }
    if (cache.stateJson === null) {
      cache.stateJson = state.editorEngine.stateJson?.() || "";
    }
    if (cache.parsedState === undefined) {
      cache.parsedState = parseEngineJson(cache.stateJson, null);
    }
    return cache.parsedState;
  }

  function currentEditorDocumentData() {
    if (!isEditingRustDocument()) {
      return state.currentDocument;
    }
    const cache = currentEditorEngineReadCache();
    if (!cache) {
      return state.currentDocument;
    }
    if (cache.documentJson === null) {
      cache.documentJson = state.editorEngine.documentJson?.() || "";
    }
    if (cache.parsedDocument === undefined) {
      cache.parsedDocument = parseEngineJson(cache.documentJson, null);
    }
    return cache.parsedDocument || state.currentDocument;
  }

  function currentEditorRenderList() {
    const cache = currentEditorEngineReadCache();
    if (!cache) {
      return [];
    }
    if (!cache.renderList) {
      if (window.__chemsemaDebug?.renderStats) {
        window.__chemsemaDebug.renderStats.renderListJsonCount += 1;
        if (window.__chemsemaDebug.renderStats.captureRenderListStacks) {
          window.__chemsemaDebug.renderStats.lastRenderListJsonStack = new Error().stack || "";
        }
      }
      cache.renderListJson = state.editorEngine.renderListJson?.() || "[]";
      cache.renderList = parseEngineJson(cache.renderListJson, []) || [];
    }
    return cache.renderList;
  }

  function currentEditorInteractionRenderList() {
    const cache = currentEditorEngineReadCache();
    if (!cache) {
      return [];
    }
    if (!cache.interactionRenderList) {
      cache.interactionRenderListJson = state.editorEngine.interactionRenderListJson?.() || "[]";
      cache.interactionRenderList = parseEngineJson(cache.interactionRenderListJson, []) || [];
    }
    return cache.interactionRenderList;
  }

  function currentEditorRenderBounds(scope = "all") {
    const cache = currentEditorEngineReadCache();
    if (!cache) {
      return null;
    }
    if (!cache.boundsByScope.has(scope)) {
      const json = scope === "selection" && state.editorEngine.selectionBoundsJson
        ? state.editorEngine.selectionBoundsJson()
        : state.editorEngine.renderBoundsJson?.(scope);
      cache.boundsJsonByScope.set(scope, json || "null");
      cache.boundsByScope.set(scope, parseEngineJson(json || "null", null));
    }
    return cache.boundsByScope.get(scope);
  }

  function currentRenderBounds(scope = "all") {
    if (isEditingRustDocument()) {
      return currentEditorRenderBounds(scope);
    }
    return renderBoundsFromEngine(state.documentEngine, scope);
  }

  async function syncCoreRenderListFromCurrentDocument() {
    state.coreRenderList = null;
    if (!state.currentDocument) {
      return;
    }
    if (state.currentPath) {
      if (!state.documentEngine) {
        await resetDocumentEngine();
      }
      await state.documentEngine.ready?.();
      await state.documentEngine.loadDocumentJson(JSON.stringify(state.currentDocument));
      state.coreRenderList = renderListFromEngine(state.documentEngine);
      return;
    }
    if (state.editorEngine) {
      state.coreRenderList = currentEditorRenderList();
    }
  }

  function syncEditorRenderListFromEngine(options = {}) {
    if (!state.editorEngine) {
      return [];
    }
    const autoExpand = options.autoExpand ?? true;
    state.coreRenderList = currentEditorRenderList();
    if (autoExpand) {
      maybeAutoExpandEditorViewport(state.coreRenderList || []);
    }
    return state.coreRenderList || [];
  }

  function syncEditorSelectionRenderListFromEngine() {
    return currentEditorInteractionRenderList();
  }

  function currentEditorOverlayRenderList() {
    const renderList = currentEditorInteractionRenderList();
    return (renderList || []).filter((primitive) => !isDocumentPreviewPrimitive(primitive));
  }

  function currentSelectionItemCount(selection = currentEditorEngineState()?.selection) {
    if (!selection) {
      return 0;
    }
    return (selection.nodes?.length || 0)
      + (selection.bonds?.length || 0)
      + (selection.labelNodes?.length || selection.label_nodes?.length || 0)
      + (selection.textObjects?.length || selection.text_objects?.length || 0)
      + (selection.arrowObjects?.length || selection.arrow_objects?.length || 0);
  }

  function freshestPreviewSelection(cachedSelection = null) {
    const currentSelection = parseEngineJson(state.editorEngine?.stateJson?.() || "", null)?.selection || null;
    if (!cachedSelection) {
      return currentSelection;
    }
    if (!currentSelection) {
      return cachedSelection;
    }
    const cachedObjectIds = new Set([
      ...(cachedSelection.textObjects || cachedSelection.text_objects || []),
      ...(cachedSelection.arrowObjects || cachedSelection.arrow_objects || []),
    ]);
    const currentObjectIds = [
      ...(currentSelection.textObjects || currentSelection.text_objects || []),
      ...(currentSelection.arrowObjects || currentSelection.arrow_objects || []),
    ];
    if (currentObjectIds.some((objectId) => !cachedObjectIds.has(objectId))) {
      return currentSelection;
    }
    return currentSelectionItemCount(currentSelection) > currentSelectionItemCount(cachedSelection)
      ? currentSelection
      : cachedSelection;
  }

  function corePrimitivesForObject(objectId) {
    return primitivesForObject(state.coreRenderList, objectId);
  }

  return { currentEditableFragmentData, worldPointForFragmentPosition, worldPointForFragmentNode, selectionZoomCenterWorld, editorEngineRevision, currentEditorEngineReadCache, currentEditorEngineState, currentEditorDocumentData, currentEditorRenderList, currentEditorInteractionRenderList, currentEditorRenderBounds, currentRenderBounds, syncCoreRenderListFromCurrentDocument, syncEditorRenderListFromEngine, syncEditorSelectionRenderListFromEngine, currentEditorOverlayRenderList, currentSelectionItemCount, freshestPreviewSelection, corePrimitivesForObject };
}
