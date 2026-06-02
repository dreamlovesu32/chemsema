import { makeSvgNode } from "./render_support.js";
import {
  renderLineObject,
  renderShapeObject,
  renderTextObject,
} from "./object_fallbacks.js";
import { renderCorePrimitive } from "./primitive_dom_renderer.js";

function buildRenderList(documentData) {
  return [...documentData.objects].sort((a, b) => {
    if (a.zIndex !== b.zIndex) {
      return a.zIndex - b.zIndex;
    }
    return a.id.localeCompare(b.id);
  });
}

function sortedSceneChildren(children = []) {
  return [...children].sort((a, b) => {
    if (a.zIndex !== b.zIndex) {
      return a.zIndex - b.zIndex;
    }
    return a.id.localeCompare(b.id);
  });
}

export function createSceneRenderer(options) {
  const legacyFallbackWarnings = new Set();

  function shouldRenderSceneObject(object) {
    if (!object.visible) {
      return false;
    }
    if (object.type === "molecule" && options.toggleMolecules?.() === false) {
      return false;
    }
    if (object.type === "line" && options.toggleLines?.() === false) {
      return false;
    }
    if (object.type === "text" && options.toggleTexts?.() === false) {
      return false;
    }
    if (options.labelDebugMode && object.type !== "molecule" && object.type !== "group") {
      return false;
    }
    return true;
  }

  function documentUsesCorePrimitivePipeline(documentData) {
    return Boolean(documentData && options.hasCoreRenderList?.());
  }

  function warnUnexpectedLegacyFallback(object, documentData) {
    if (!documentUsesCorePrimitivePipeline(documentData)) {
      return;
    }
    const kind = object?.payload?.kind || object?.type || "object";
    const key = `${object?.id || "unknown"}:${kind}`;
    if (legacyFallbackWarnings.has(key)) {
      return;
    }
    legacyFallbackWarnings.add(key);
    console.warn("[chemcore] unexpected legacy fallback render", {
      id: object?.id || null,
      type: object?.type || null,
      kind,
      meta: object?.meta || null,
    });
  }

  function renderObjectCorePrimitives(objectLayer, objectId) {
    const corePrimitives = options.corePrimitivesForObject(objectId);
    if (!corePrimitives.length) {
      return false;
    }
    objectLayer.setAttribute("data-renderer", "core");
    corePrimitives.forEach((primitive) => {
      renderCorePrimitive(objectLayer, primitive, options.corePrimitiveRenderOptions());
    });
    return true;
  }

  function renderObjectWithCoreFallback(objectLayer, object, documentData, fallbackRenderer) {
    if (renderObjectCorePrimitives(objectLayer, object.id)) {
      return;
    }
    objectLayer.setAttribute("data-renderer", "fallback");
    warnUnexpectedLegacyFallback(object, documentData);
    fallbackRenderer();
  }

  function renderSceneObject(parentLayer, object, documentData) {
    if (!shouldRenderSceneObject(object)) {
      return;
    }

    const objectLayer = makeSvgNode("g", {
      "data-object-id": object.id,
      "data-object-type": object.type,
    });

    if (object.type === "group") {
      for (const child of sortedSceneChildren(object.children || [])) {
        renderSceneObject(objectLayer, child, documentData);
      }
    } else if (object.type === "molecule") {
      renderObjectCorePrimitives(objectLayer, object.id);
    } else if (object.type === "shape") {
      renderObjectWithCoreFallback(objectLayer, object, documentData, () => {
        renderShapeObject(objectLayer, object, documentData.styles);
      });
    } else if (object.type === "line") {
      renderObjectWithCoreFallback(objectLayer, object, documentData, () => {
        renderLineObject(objectLayer, object, documentData.styles);
      });
    } else if (object.type === "text") {
      renderObjectWithCoreFallback(objectLayer, object, documentData, () => {
        renderTextObject(objectLayer, object);
      });
    } else if (object.type === "bracket" || object.type === "symbol") {
      renderObjectCorePrimitives(objectLayer, object.id);
    }

    if (objectLayer.childNodes.length) {
      parentLayer.appendChild(objectLayer);
    }
  }

  return {
    buildRenderList,
    renderSceneObject,
  };
}
