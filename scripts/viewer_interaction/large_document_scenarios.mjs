export function createLargeDocumentInteractionScenarios(scope) {
  const { assert, openLargeCdxmlViewer, largeCdxml, existsSync, basename } = scope;

  function largeFileTargetFinder() {
    const doc = window.__chemsemaDebug.document;
    const visit = (object, out = []) => {
      if (!object) {
        return out;
      }
      out.push(object);
      for (const child of object.children || []) {
        visit(child, out);
      }
      return out;
    };
    const objectType = (object) => object?.type || object?.objectType || object?.object_type;
    const allObjects = (doc.objects || []).flatMap((candidate) => visit(candidate, []));
    const objectClientRect = (objectId) => {
      const elements = [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`)]
        .filter((element) => !element.classList.contains("document-diagnostic-marker"));
      if (!elements.length) {
        return null;
      }
      const rects = elements.map((element) => element.getBoundingClientRect())
        .filter((rect) => rect.width > 0 && rect.height > 0);
      if (!rects.length) {
        return null;
      }
      const left = Math.min(...rects.map((rect) => rect.left));
      const top = Math.min(...rects.map((rect) => rect.top));
      const right = Math.max(...rects.map((rect) => rect.right));
      const bottom = Math.max(...rects.map((rect) => rect.bottom));
      return {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
        centerX: (left + right) * 0.5,
        centerY: (top + bottom) * 0.5,
      };
    };
    const entries = [];
    const bondEntries = [];
    for (const object of allObjects) {
      if (objectType(object) !== "molecule") {
        continue;
      }
      const resourceRef = object.payload?.resourceRef || object.payload?.resource_ref;
      const fragment = resourceRef ? doc.resources?.[resourceRef]?.data : object.payload?.fragment;
      if (!fragment?.nodes?.length) {
        continue;
      }
      const degree = new Map();
      for (const bond of fragment.bonds || []) {
        degree.set(bond.begin, (degree.get(bond.begin) || 0) + 1);
        degree.set(bond.end, (degree.get(bond.end) || 0) + 1);
      }
      const translate = object.transform?.translate || [0, 0];
      const nodePositions = new Map();
      for (const node of fragment.nodes || []) {
        if (!Array.isArray(node.position) || !degree.get(node.id)) {
          continue;
        }
        const x = Number(translate[0] || 0) + Number(node.position[0] || 0);
        const y = Number(translate[1] || 0) + Number(node.position[1] || 0);
        nodePositions.set(node.id, { x, y });
        const client = window.__chemsemaDebug.worldToClient(x, y);
        if (!client
          || client.x <= 80
          || client.x >= innerWidth - 80
          || client.y <= 120
          || client.y >= innerHeight - 80) {
          continue;
        }
        entries.push({
          id: node.id,
          x: client.x,
          y: client.y,
          worldX: x,
          worldY: y,
          objectId: object.id,
          label: node.label?.text || node.label?.sourceText || "",
          element: node.element || "",
          degree: degree.get(node.id) || 0,
        });
      }
      for (const bond of fragment.bonds || []) {
        const begin = nodePositions.get(bond.begin);
        const end = nodePositions.get(bond.end);
        if (!begin || !end) {
          continue;
        }
        const client = window.__chemsemaDebug.worldToClient(
          (begin.x + end.x) * 0.5,
          (begin.y + end.y) * 0.5,
        );
        if (!client
          || client.x <= 80
          || client.x >= innerWidth - 80
          || client.y <= 120
          || client.y >= innerHeight - 80) {
          continue;
        }
        bondEntries.push({
          id: bond.id,
          x: client.x,
          y: client.y,
        });
      }
    }
    const visibleObjectTarget = (type) => allObjects
      .filter((object) => objectType(object) === type && object.visible !== false)
      .map((object) => {
        const rect = objectClientRect(object.id);
        const bbox = object.payload?.bbox;
        const translate = object.transform?.translate || [0, 0];
        const clientToWorld = (client) => {
          const matrix = document.querySelector("#viewer-svg")?.getScreenCTM?.()?.inverse?.();
          if (!matrix || !client) {
            return null;
          }
          const world = new DOMPoint(client.x, client.y).matrixTransform(matrix);
          return { x: world.x, y: world.y };
        };
        const boundsCenter = Array.isArray(bbox)
          ? {
            client: window.__chemsemaDebug.worldToClient(
              Number(translate[0] || 0) + Number(bbox[0] || 0) + Number(bbox[2] || 0) * 0.5,
              Number(translate[1] || 0) + Number(bbox[1] || 0) + Number(bbox[3] || 0) * 0.5,
            ),
            world: {
              x: Number(translate[0] || 0) + Number(bbox[0] || 0) + Number(bbox[2] || 0) * 0.5,
              y: Number(translate[1] || 0) + Number(bbox[1] || 0) + Number(bbox[3] || 0) * 0.5,
            },
          }
          : null;
        const domCenters = [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(object.id)}"]`)]
          .filter((element) => !element.classList.contains("document-diagnostic-marker"))
          .map((element) => {
            const candidateRect = element.getBoundingClientRect();
            if (candidateRect.width <= 0 || candidateRect.height <= 0) {
              return null;
            }
            const client = {
              x: candidateRect.left + candidateRect.width * 0.5,
              y: candidateRect.top + candidateRect.height * 0.5,
            };
            return { client, world: clientToWorld(client) };
          })
          .filter(Boolean);
        const unionCenter = rect
          ? {
            client: { x: rect.centerX, y: rect.centerY },
            world: clientToWorld({ x: rect.centerX, y: rect.centerY }),
          }
          : null;
        const hitTarget = [boundsCenter, unionCenter, ...domCenters]
          .filter((candidate) => candidate?.client && candidate?.world)
          .filter((candidate) => candidate.client.x > 80
            && candidate.client.x < innerWidth - 80
            && candidate.client.y > 120
            && candidate.client.y < innerHeight - 80)
          .find((candidate) => {
            const hit = JSON.parse(window.__chemsemaDebug.state.editorEngine.contextHitTestJson?.(candidate.world.x, candidate.world.y) || "null");
            return hit?.objectId === object.id;
          });
        if (!rect
          || !hitTarget) {
          return null;
        }
        return {
          id: object.id,
          x: hitTarget.client.x,
          y: hitTarget.client.y,
          rect,
        };
      })
      .find(Boolean) || null;
    const bracket = allObjects
      .filter((object) => objectType(object) === "bracket" && object.visible !== false)
      .map((object) => {
        const rect = objectClientRect(object.id);
        const bbox = object.payload?.bbox;
        const translate = object.transform?.translate || [0, 0];
        if (!rect || !Array.isArray(bbox)) {
          return null;
        }
        const tx = Number(translate[0] || 0) + Number(bbox[0] || 0);
        const ty = Number(translate[1] || 0) + Number(bbox[1] || 0);
        const width = Number(bbox[2] || 0);
        const height = Number(bbox[3] || 0);
        const side = object.payload?.side || object.payload?.extra?.side || "";
        const xCandidates = side
          ? [side === "right" ? tx + width : tx, tx + width * 0.5, side === "right" ? tx : tx + width]
          : [tx, tx + width, tx + width * 0.5];
        const yCandidates = [ty + height * 0.25, ty + height * 0.5, ty + height * 0.75, ty, ty + height];
        for (const x of xCandidates) {
          for (const y of yCandidates) {
            const hover = window.__chemsemaDebug.worldToClient(x, y);
            if (!hover
              || hover.x <= 80
              || hover.x >= innerWidth - 80
              || hover.y <= 120
              || hover.y >= innerHeight - 80) {
              continue;
            }
            const hit = JSON.parse(window.__chemsemaDebug.state.editorEngine.contextHitTestJson?.(x, y) || "null");
            if (hit?.objectId !== object.id) {
              continue;
            }
            return {
              id: object.id,
              x: hover.x,
              y: hover.y,
              rect,
            };
          }
        }
        return null;
      })
      .find(Boolean) || null;
    const bracketCount = allObjects
      .filter((object) => objectType(object) === "bracket" && object.visible !== false)
      .length;
    const domHover = [...document.querySelectorAll("[data-node-id]")]
      .map((element) => {
        const rect = element.getBoundingClientRect();
        return {
          id: element.getAttribute("data-node-id"),
          x: rect.x + rect.width / 2,
          y: rect.y + rect.height / 2,
          w: rect.width,
          h: rect.height,
          diagnostic: element.classList.contains("document-diagnostic-marker"),
        };
      })
      .filter((entry) => !entry.diagnostic)
      .filter((entry) => entry.w >= 3
        && entry.h >= 2
        && entry.x > 80
        && entry.x < innerWidth - 80
        && entry.y > 120
        && entry.y < innerHeight - 80)[0] || null;
    const hover = domHover
      || entries.find((entry) => entry.degree > 0)
      || bondEntries[0]
      || null;
    const bondTarget = [...document.querySelectorAll("[data-bond-id]")]
      .map((element) => {
        const rect = element.getBoundingClientRect();
        return {
          id: element.getAttribute("data-bond-id"),
          x: rect.x + rect.width / 2,
          y: rect.y + rect.height / 2,
          w: rect.width,
          h: rect.height,
        };
      })
      .filter((entry) => entry.id
        && entry.w >= 3
        && entry.h >= 3
        && entry.x > 80
        && entry.x < innerWidth - 80
        && entry.y > 120
        && entry.y < innerHeight - 80)[0] || null;
    const invalidDiagnostic = [...document.querySelectorAll(".document-diagnostic-marker[data-node-id]")]
      .map((marker) => {
        const id = marker.getAttribute("data-node-id");
        const nodeEntry = entries.find((entry) => entry.id === id);
        const anchor = [...document.querySelectorAll(`[data-node-id="${CSS.escape(id)}"]`)]
          .find((element) => element !== marker && !element.classList.contains("document-diagnostic-marker"));
        if (!anchor && !nodeEntry) {
          return null;
        }
        const markerRect = marker.getBoundingClientRect();
        const anchorRect = anchor?.getBoundingClientRect();
        return {
          id,
          x: nodeEntry?.x ?? (anchorRect.x + anchorRect.width / 2),
          y: nodeEntry?.y ?? (anchorRect.y + anchorRect.height / 2),
          markerX: markerRect.x + markerRect.width / 2,
          markerY: markerRect.y + markerRect.height / 2,
        };
      })
      .filter(Boolean)
      .find((entry) => entry.x > 80
        && entry.x < innerWidth - 80
        && entry.y > 120
        && entry.y < innerHeight - 80) || null;
    return {
      hover,
      bond: bondEntries[0] || bondTarget,
      label: entries.find((entry) => entry.label && entry.degree > 0) || null,
      atom: entries.find((entry) => !entry.label && (!entry.element || entry.element === "C") && entry.degree > 0) || null,
      bracket,
      bracketCount,
      textObject: visibleObjectTarget("text"),
      invalidDiagnostic,
    };
  }
  
  async function verifyLargeDragTarget(page, target, kind) {
    await page.keyboard.press("Escape").catch(() => {});
    await page.evaluate(() => {
      window.__chemsemaDebug.state.editorEngine.clearSelection?.();
      window.__chemsemaDebug.state.editorEngine.clearInteraction?.();
      window.__chemsemaDebug.clearActiveSelectionGesture?.();
      document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
    });
    await page.locator('button[data-tool="select"]').click();
    await page.mouse.move(target.x, target.y);
    await page.waitForTimeout(180);
    await page.mouse.move(target.x, target.y);
    const beforeNodePosition = await page.evaluate((nodeId) => {
      const rects = [...document.querySelectorAll(`[data-layer="document-content"] [data-node-id="${CSS.escape(nodeId)}"]`)]
        .filter((element) => getComputedStyle(element).visibility !== "hidden")
        .map((element) => element.getBoundingClientRect())
        .filter((rect) => rect.width > 0 && rect.height > 0);
      if (!rects.length) {
        return null;
      }
      const left = Math.min(...rects.map((rect) => rect.left));
      const top = Math.min(...rects.map((rect) => rect.top));
      const right = Math.max(...rects.map((rect) => rect.right));
      const bottom = Math.max(...rects.map((rect) => rect.bottom));
      return { x: (left + right) * 0.5, y: (top + bottom) * 0.5 };
    }, target.id);
    await page.mouse.down();
    await page.mouse.move(target.x + 24, target.y + 12, { steps: 6 });
    const backendDomMatches = (nodeId) => {
      const doc = window.__chemsemaDebug.document;
      const connectedBonds = new Set();
      const visit = (object, out = []) => {
        if (!object) {
          return out;
        }
        out.push(object);
        for (const child of object.children || []) {
          visit(child, out);
        }
        return out;
      };
      for (const object of (doc.objects || []).flatMap((candidate) => visit(candidate, []))) {
        const resourceRef = object.payload?.resourceRef || object.payload?.resource_ref;
        const fragment = resourceRef ? doc.resources?.[resourceRef]?.data : object.payload?.fragment;
        for (const bond of fragment?.bonds || []) {
          if (bond.begin === nodeId || bond.end === nodeId) {
            connectedBonds.add(bond.id);
          }
        }
      }
      const renderList = JSON.parse(window.__chemsemaDebug.state.editorEngine.renderTargetsJson(JSON.stringify({
        nodes: [nodeId],
        bonds: [...connectedBonds],
      })));
      const backendCount = renderList
        .filter((primitive) => (
          primitive.role !== "document-knockout"
          && primitive.role !== "document_knockout"
          && (
            primitive.nodeId === nodeId
            || primitive.node_id === nodeId
            || connectedBonds.has(primitive.bondId || primitive.bond_id)
          )
        ))
        .length;
      const selectors = [
        `[data-node-id="${CSS.escape(nodeId)}"]`,
        ...[...connectedBonds].map((bondId) => `[data-bond-id="${CSS.escape(bondId)}"]`),
      ];
      const domCount = [...document.querySelectorAll(`[data-layer="document-content"] ${selectors.join(",")}`)].length;
      return {
        connectedBonds: [...connectedBonds],
        backendCount,
        domCount,
        matches: backendCount > 0 && backendCount === domCount,
        partialChildren: document.querySelector('[data-layer="document-partial-bond-preview"]')?.childElementCount || 0,
        gesture: window.__chemsemaDebug.activeSelectionGesture || null,
      };
    };
    await page.evaluate((source) => {
      window.__viewerSmokeBackendDomMatches = eval(`(${source})`);
    }, backendDomMatches.toString());
    try {
      await page.waitForFunction((nodeId) => {
        return (window.__viewerSmokeBackendDomMatches || (() => ({ matches: false })))(nodeId).matches;
      }, target.id, { timeout: 5000 });
    } catch (error) {
      const diagnostics = await page.evaluate(
        ([nodeId, source]) => {
          window.__viewerSmokeBackendDomMatches = eval(`(${source})`);
          return window.__viewerSmokeBackendDomMatches(nodeId);
        },
        [target.id, backendDomMatches.toString()],
      );
      throw new Error(`${kind} backend DOM did not match: ${JSON.stringify(diagnostics).slice(0, 1600)}`);
    }
    const duringNodePosition = await page.evaluate(([nodeId, before]) => {
      const elements = [...document.querySelectorAll(`[data-layer="document-content"] [data-node-id="${CSS.escape(nodeId)}"]`)]
        .filter((element) => getComputedStyle(element).visibility !== "hidden");
      const rects = elements
        .map((element) => element.getBoundingClientRect())
        .filter((rect) => rect.width > 0 && rect.height > 0);
      if (!rects.length) {
        return { count: 0, oldVisibleCount: 0, x: null, y: null };
      }
      const left = Math.min(...rects.map((rect) => rect.left));
      const top = Math.min(...rects.map((rect) => rect.top));
      const right = Math.max(...rects.map((rect) => rect.right));
      const bottom = Math.max(...rects.map((rect) => rect.bottom));
      const oldVisibleCount = before
        ? rects.filter((rect) => {
          const cx = rect.left + rect.width * 0.5;
          const cy = rect.top + rect.height * 0.5;
          return Math.hypot(cx - before.x, cy - before.y) < 3;
        }).length
        : 0;
      return {
        count: rects.length,
        oldVisibleCount,
        x: (left + right) * 0.5,
        y: (top + bottom) * 0.5,
      };
    }, [target.id, beforeNodePosition]);
    const previewMoved = beforeNodePosition && duringNodePosition.x != null
      ? Math.hypot(duringNodePosition.x - beforeNodePosition.x, duringNodePosition.y - beforeNodePosition.y)
      : 0;
    assert(
      previewMoved > 6,
      `${kind} did not visually follow drag before mouseup: ${JSON.stringify({ target, beforeNodePosition, duringNodePosition, previewMoved })}`,
    );
    assert(
      duringNodePosition.oldVisibleCount === 0,
      `${kind} left a visible stale node primitive at the drag origin: ${JSON.stringify({ target, beforeNodePosition, duringNodePosition })}`,
    );
    const during = await page.evaluate(() => {
      const partial = document.querySelector('[data-layer="document-partial-bond-preview"]');
      return {
        partialChildren: partial?.childElementCount || 0,
        transformed: document.querySelectorAll(".is-preview-transforming").length,
      };
    });
    await page.mouse.up();
    await page.waitForTimeout(250);
    const after = await page.evaluate(() => {
      const overlay = document.querySelector('[data-layer="editor-overlay"]');
      return {
        previews: overlay?.querySelectorAll('[data-role^="preview-"]').length || 0,
        partial: !!document.querySelector('[data-layer="document-partial-bond-preview"]'),
        transformed: document.querySelectorAll(".is-preview-transforming").length,
        gesture: window.__chemsemaDebug.activeSelectionGesture || null,
      };
    });
    assert(during.partialChildren === 0, `${kind} drag used front-end partial bond preview.`);
    assert(!after.partial, `${kind} drag left partial bond preview behind.`);
    assert(after.transformed === 0, `${kind} drag left transformed document nodes behind.`);
    assert(after.previews === 0, `${kind} drag left preview overlay behind.`);
    assert(after.gesture === null, `${kind} drag left an active selection gesture behind.`);
    const commandTargets = await page.evaluate((nodeId) => {
      const raw = window.__chemsemaDebug.state.editorEngine.lastCommandResultJson?.() || "null";
      const result = JSON.parse(raw);
      const targetNodes = new Set([
        ...(result?.targets?.nodes || []),
        ...(result?.updated?.nodes || []),
        ...(result?.created?.nodes || []),
        ...(result?.deleted?.nodes || []),
      ]);
      return {
        changed: !!result?.changed,
        nodeIncluded: targetNodes.has(nodeId),
        nodes: [...targetNodes].slice(0, 20),
        command: result?.command || null,
      };
    }, target.id);
    assert(
      commandTargets.changed && commandTargets.nodeIncluded,
      `${kind} drag commit did not report the moved node for incremental rendering: ${JSON.stringify({ target, commandTargets })}`,
    );
    const documentSync = await page.evaluate((nodeId) => {
      const worldPositionForNode = (doc, wantedNodeId) => {
        const objectType = (object) => object?.type || object?.objectType || object?.object_type;
        const visit = (object, inheritedTranslate = [0, 0]) => {
          if (!object) {
            return null;
          }
          const translate = object.transform?.translate || object.transform?.translation || [0, 0];
          const combinedTranslate = [
            Number(inheritedTranslate[0] || 0) + Number(translate[0] || 0),
            Number(inheritedTranslate[1] || 0) + Number(translate[1] || 0),
          ];
          if (objectType(object) === "molecule") {
            const resourceRef = object.payload?.resourceRef || object.payload?.resource_ref;
            const fragment = resourceRef ? doc.resources?.[resourceRef]?.data : object.payload?.fragment;
            const node = fragment?.nodes?.find((candidate) => candidate.id === wantedNodeId);
            if (node?.position) {
              return {
                x: Number(node.position[0] || 0) + combinedTranslate[0],
                y: Number(node.position[1] || 0) + combinedTranslate[1],
              };
            }
          }
          for (const child of object.children || []) {
            const found = visit(child, combinedTranslate);
            if (found) {
              return found;
            }
          }
          return null;
        };
        for (const object of doc?.objects || []) {
          const found = visit(object);
          if (found) {
            return found;
          }
        }
        for (const resource of Object.values(doc?.resources || {})) {
          for (const node of resource?.data?.nodes || []) {
            if (node.id === wantedNodeId && node.position) {
              return { x: Number(node.position[0] || 0), y: Number(node.position[1] || 0) };
            }
          }
        }
        return null;
      };
      const engineDoc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson?.() || "null");
      const frontendDoc = window.__chemsemaDebug.document;
      const engine = worldPositionForNode(engineDoc, nodeId);
      const frontend = worldPositionForNode(frontendDoc, nodeId);
      const distance = engine && frontend ? Math.hypot(engine.x - frontend.x, engine.y - frontend.y) : null;
      return { engine, frontend, distance };
    }, target.id);
    assert(
      documentSync.distance !== null && documentSync.distance < 0.01,
      `${kind} drag left the front-end document model stale after commit: ${JSON.stringify({ target, documentSync })}`,
    );
  }
  
  async function verifyLargeRegionSelectionDoesNotDragGroup(page, target) {
    await page.keyboard.press("Escape").catch(() => {});
    await page.evaluate(() => {
      window.__chemsemaDebug.state.editorEngine.clearSelection?.();
      window.__chemsemaDebug.state.editorEngine.clearInteraction?.();
      window.__chemsemaDebug.clearActiveSelectionGesture?.();
      document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
    });
    await page.locator('button[data-tool="select"]').click();
    const selected = await page.evaluate((regionTarget) => {
      const engine = window.__chemsemaDebug.state.editorEngine;
      const doc = JSON.parse(engine.documentJson());
      const objectTypeById = new Map();
      const visit = (object) => {
        if (!object) {
          return;
        }
        objectTypeById.set(object.id, object.type || object.objectType || object.object_type);
        for (const child of object.children || []) {
          visit(child);
        }
      };
      for (const object of doc.objects || []) {
        visit(object);
      }
      engine.selectInRect(
        regionTarget.worldX - 24,
        regionTarget.worldY - 24,
        regionTarget.worldX + 24,
        regionTarget.worldY + 24,
        false,
      );
      const selection = JSON.parse(engine.stateJson()).selection || {};
      return {
        selection,
        selectedGroups: (selection.arrowObjects || [])
          .filter((objectId) => objectTypeById.get(objectId) === "group"),
      };
    }, target);
    assert(
      selected.selectedGroups.length === 0,
      `Large CDXML region selection captured parent groups: ${JSON.stringify({ target, selected })}`,
    );
    assert(
      (selected.selection.nodes || []).includes(target.id)
        || (selected.selection.labelNodes || []).includes(target.id),
      `Large CDXML region selection did not include target node: ${JSON.stringify({ target, selected })}`,
    );
  
    const moved = await page.evaluate((regionTarget) => {
      const engine = window.__chemsemaDebug.state.editorEngine;
      const nodePosition = (doc, nodeId) => {
        for (const resource of Object.values(doc.resources || {})) {
          for (const node of resource?.data?.nodes || []) {
            if (node.id === nodeId) {
              return node.position;
            }
          }
        }
        return null;
      };
      const groupTransforms = (doc) => {
        const out = {};
        const visit = (object) => {
          if (!object) {
            return;
          }
          const type = object.type || object.objectType || object.object_type;
          if (type === "group") {
            out[object.id] = object.transform?.translate || [0, 0];
          }
          for (const child of object.children || []) {
            visit(child);
          }
        };
        for (const object of doc.objects || []) {
          visit(object);
        }
        return out;
      };
      const beforeDoc = JSON.parse(engine.documentJson());
      const beforeNode = nodePosition(beforeDoc, regionTarget.id);
      const beforeGroups = groupTransforms(beforeDoc);
      const began = engine.beginSelectionMove(regionTarget.worldX, regionTarget.worldY, false, false);
      const updated = engine.updateSelectionMove(regionTarget.worldX + 10, regionTarget.worldY, false);
      const finished = engine.finishSelectionMove(regionTarget.worldX + 10, regionTarget.worldY, false);
      const afterDoc = JSON.parse(engine.documentJson());
      const afterNode = nodePosition(afterDoc, regionTarget.id);
      const afterGroups = groupTransforms(afterDoc);
      const command = JSON.parse(engine.lastCommandResultJson?.() || "null");
      return { began, updated, finished, beforeNode, afterNode, beforeGroups, afterGroups, command };
    }, target);
    const dx = (moved.afterNode?.[0] ?? NaN) - (moved.beforeNode?.[0] ?? NaN);
    assert(
      moved.began && moved.updated && moved.finished && Math.abs(dx - 10) < 0.01,
      `Large CDXML region-selected molecule node did not move correctly: ${JSON.stringify({ target, moved })}`,
    );
    assert(
      JSON.stringify(moved.beforeGroups) === JSON.stringify(moved.afterGroups),
      `Large CDXML region-selected molecule moved parent group transforms: ${JSON.stringify({ target, moved })}`,
    );
    assert(
      (moved.command?.targets?.nodes || []).includes(target.id),
      `Large CDXML region-selected molecule drag did not report moved node target: ${JSON.stringify({ target, moved })}`,
    );
    await page.evaluate(async () => {
      window.__chemsemaDebug.state.editorEngine.clearSelection?.();
      window.__chemsemaDebug.state.editorEngine.clearInteraction?.();
      await window.__chemsemaDebug.syncDocument?.();
    });
  }
  
  function selectionItemCount(selection) {
    if (!selection) {
      return 0;
    }
    return (selection.textObjects?.length || 0)
      + (selection.arrowObjects?.length || 0)
      + (selection.labelNodes?.length || 0)
      + (selection.nodes?.length || 0)
      + (selection.bonds?.length || 0);
  }
  
  async function verifyLargeFileSelectionLatency(page, target) {
    await page.locator('button[data-tool="select"]').click();
    await page.waitForFunction(() => getComputedStyle(document.querySelector("#viewer-svg")).pointerEvents === "none");
    const blank = { x: 1180, y: 820 };
  
    await page.mouse.move(target.x, target.y);
    let stepStarted = Date.now();
    await page.mouse.down();
    const selectDownMs = Date.now() - stepStarted;
    stepStarted = Date.now();
    await page.mouse.up();
    const selectUpMs = Date.now() - stepStarted;
    await page.waitForFunction(() => {
      const selection = window.__chemsemaDebug.engineState?.selection;
      const count = (selection?.textObjects?.length || 0)
        + (selection?.arrowObjects?.length || 0)
        + (selection?.labelNodes?.length || 0)
        + (selection?.nodes?.length || 0)
        + (selection?.bonds?.length || 0);
      return count > 0 && (document.querySelector('[data-layer="editor-overlay"]')?.childElementCount || 0) > 0;
    }, null, { timeout: 1000 });
    const selected = await page.evaluate(() => ({
      overlayChildren: document.querySelector('[data-layer="editor-overlay"]')?.childElementCount || 0,
      selection: window.__chemsemaDebug.engineState?.selection || null,
    }));
    assert(selectionItemCount(selected.selection) > 0 && selected.overlayChildren > 0, `Large CDXML selection box did not appear: ${JSON.stringify(selected)}`);
    assert(
      selectDownMs + selectUpMs < 500,
      `Large CDXML selection box appeared too slowly: ${JSON.stringify({ selectDownMs, selectUpMs, selected })}`,
    );
  
    await page.mouse.move(blank.x, blank.y);
    stepStarted = Date.now();
    await page.mouse.down();
    const clearDownMs = Date.now() - stepStarted;
    stepStarted = Date.now();
    await page.mouse.up();
    const clearUpMs = Date.now() - stepStarted;
    await page.waitForFunction(() => {
      const selection = window.__chemsemaDebug.engineState?.selection;
      const count = (selection?.textObjects?.length || 0)
        + (selection?.arrowObjects?.length || 0)
        + (selection?.labelNodes?.length || 0)
        + (selection?.nodes?.length || 0)
        + (selection?.bonds?.length || 0);
      return count === 0 && (document.querySelector('[data-layer="editor-overlay"]')?.childElementCount || 0) === 0;
    }, null, { timeout: 1000 });
    const cleared = await page.evaluate(() => ({
      overlayChildren: document.querySelector('[data-layer="editor-overlay"]')?.childElementCount || 0,
      selection: window.__chemsemaDebug.engineState?.selection || null,
    }));
    assert(selectionItemCount(cleared.selection) === 0 && cleared.overlayChildren === 0, `Large CDXML blank click did not clear selection: ${JSON.stringify(cleared)}`);
    assert(
      clearDownMs + clearUpMs < 350,
      `Large CDXML blank click cleared selection too slowly: ${JSON.stringify({ clearDownMs, clearUpMs, cleared })}`,
    );
    await page.keyboard.press("Escape").catch(() => {});
    await page.evaluate(() => {
      window.__chemsemaDebug.state.editorEngine.clearSelection?.();
      window.__chemsemaDebug.state.editorEngine.clearInteraction?.();
      window.__chemsemaDebug.clearActiveSelectionGesture?.();
      document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
    });
    await page.mouse.move(blank.x, blank.y);
    await page.waitForTimeout(30);
  }
  
  async function verifyDiagnosticMarkerHidesDuringDrag(page, target, dragTarget = target) {
    if (!target || !dragTarget) {
      return;
    }
    await page.keyboard.press("Escape").catch(() => {});
    await page.evaluate(() => {
      window.__chemsemaDebug.state.editorEngine.clearSelection?.();
      window.__chemsemaDebug.state.editorEngine.clearInteraction?.();
      window.__chemsemaDebug.clearActiveSelectionGesture?.();
      document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
    });
    await page.locator('button[data-tool="select"]').click();
    const before = await page.evaluate((nodeId) => {
      const markers = [...document.querySelectorAll(`.document-diagnostic-marker[data-node-id="${CSS.escape(nodeId)}"]`)];
      return {
        count: markers.length,
        totalDiagnostics: document.querySelectorAll(".document-diagnostic-marker").length,
        visibleDiagnostics: [...document.querySelectorAll(".document-diagnostic-marker")]
          .filter((element) => getComputedStyle(element).visibility !== "hidden").length,
      };
    }, target.id);
    if (!before.count) {
      return;
    }
    await page.mouse.click(dragTarget.x, dragTarget.y);
    await page.waitForFunction((id) => {
      const selection = window.__chemsemaDebug.engineState?.selection || window.__chemsemaDebug.getEngineState?.()?.selection || {};
      return (selection.nodes || []).includes(id)
        || (selection.labelNodes || []).includes(id)
        || (selection.textObjects || []).includes(id)
        || (selection.arrowObjects || []).includes(id);
    }, dragTarget.id, { timeout: 1200 });
    await page.mouse.move(dragTarget.x, dragTarget.y);
    await page.waitForTimeout(180);
    await page.mouse.down();
    await page.mouse.move(dragTarget.x + 42, dragTarget.y + 18, { steps: 6 });
    const during = await page.evaluate((nodeId) => {
      const markers = [...document.querySelectorAll(`.document-diagnostic-marker[data-node-id="${CSS.escape(nodeId)}"]`)];
      return {
        count: markers.length,
        totalDiagnostics: document.querySelectorAll(".document-diagnostic-marker").length,
        visibleDiagnostics: [...document.querySelectorAll(".document-diagnostic-marker")]
          .filter((element) => getComputedStyle(element).visibility !== "hidden").length,
        previewDiagnostics: document.querySelectorAll('[data-layer="document-partial-bond-preview"] .document-diagnostic-marker').length,
        gesture: window.__chemsemaDebug.getActiveSelectionGesture?.() || null,
        selection: window.__chemsemaDebug.engineState?.selection || window.__chemsemaDebug.getEngineState?.()?.selection || null,
        previewStats: window.__chemsemaDebug.backendMovePreviewStats?.last || null,
      };
    }, target.id);
    await page.mouse.up();
    await page.waitForFunction(() => [...document.querySelectorAll(".document-diagnostic-marker")]
      .some((element) => getComputedStyle(element).visibility !== "hidden"), null, { timeout: 1000 });
    const after = await page.evaluate(() => ({
      totalDiagnostics: document.querySelectorAll(".document-diagnostic-marker").length,
      visibleDiagnostics: [...document.querySelectorAll(".document-diagnostic-marker")]
        .filter((element) => getComputedStyle(element).visibility !== "hidden").length,
    }));
    assert(during.count === before.count, `Diagnostic marker duplicated during drag: ${JSON.stringify({ before, during, target })}`);
    assert(during.totalDiagnostics <= before.totalDiagnostics + 2, `Diagnostic marker count ballooned during drag: ${JSON.stringify({ before, during, target })}`);
    assert(during.previewDiagnostics === 0, `Diagnostic markers were drawn into partial preview layer: ${JSON.stringify({ before, during, target })}`);
    assert(during.visibleDiagnostics === 0, `Diagnostic markers remained visible during drag: ${JSON.stringify({ before, during, target, dragTarget })}`);
    assert(after.totalDiagnostics <= before.totalDiagnostics + 2 && after.visibleDiagnostics > 0, `Diagnostic markers did not restore after drag: ${JSON.stringify({ before, during, after, target })}`);
  }
  
  async function verifyBracketHoverFocus(page, target) {
    if (!target) {
      return;
    }
    await page.keyboard.press("Escape").catch(() => {});
    await page.evaluate(() => {
      window.__chemsemaDebug.state.editorEngine.clearSelection?.();
      window.__chemsemaDebug.state.editorEngine.clearInteraction?.();
      window.__chemsemaDebug.clearActiveSelectionGesture?.();
      document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
    });
    await page.locator('button[data-tool="select"]').click();
    await page.mouse.move(target.x, target.y);
    const started = Date.now();
    await page.waitForFunction(() => {
      const overlay = document.querySelector('[data-layer="editor-overlay"]');
      return (overlay?.querySelectorAll('[data-role="hover-shape-handle"]').length || 0) > 0;
    }, null, { timeout: 800 });
    const elapsed = Date.now() - started;
    const debug = await page.evaluate(() => ({
      fastHover: window.__chemsemaDebug.fastSelectHoverStats || null,
      overlayChildren: document.querySelector('[data-layer="editor-overlay"]')?.childElementCount || 0,
      handles: document.querySelectorAll('[data-role="hover-shape-handle"]').length,
      handleStyle: (() => {
        const handle = document.querySelector('.editor-object-control-handle[data-role="hover-shape-handle"]');
        if (!handle) {
          return null;
        }
        const matrix = handle.ownerSVGElement?.getScreenCTM?.();
        const scale = Math.max(Math.abs(matrix?.a || 1), Math.abs(matrix?.d || 1));
        const style = getComputedStyle(handle);
        return {
          tagName: handle.tagName.toLowerCase(),
          radiusPx: Number(handle.getAttribute("r") || 0) * scale,
          fill: style.fill,
        };
      })(),
    }));
    assert(elapsed < 350, `Large CDXML bracket hover focus was delayed: ${JSON.stringify({ elapsed, target, debug })}`);
    assert(
      debug.handleStyle?.tagName === "circle"
        && Math.abs(debug.handleStyle.radiusPx - 1.5) < 0.2
        && (debug.handleStyle.fill === "none" || debug.handleStyle.fill === "rgba(0, 0, 0, 0)"),
      `Large CDXML bracket hover control handle style was not unified: ${JSON.stringify({ target, debug })}`,
    );
  }
  
  async function verifyAllSquareBracketsHover(page) {
    await page.keyboard.press("Escape").catch(() => {});
    await page.evaluate(() => {
      window.__chemsemaDebug.state.editorEngine.clearSelection?.();
      window.__chemsemaDebug.state.editorEngine.clearInteraction?.();
      window.__chemsemaDebug.clearActiveSelectionGesture?.();
      document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
    });
    await page.locator('button[data-tool="select"]').click();
    const targets = await page.evaluate(() => {
      const documentData = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson?.() || "null")
        || window.__chemsemaDebug.document;
      const visit = (object, out = []) => {
        if (!object) {
          return out;
        }
        out.push(object);
        for (const child of object.children || []) {
          visit(child, out);
        }
        return out;
      };
      const objectType = (object) => object?.type || object?.objectType || object?.object_type;
      const bracketKind = (object) => object?.payload?.kind || object?.payload?.extra?.kind || "round";
      const bracketSide = (object) => object?.payload?.side || object?.payload?.extra?.side || "";
      return (documentData.objects || [])
        .flatMap((object) => visit(object, []))
        .filter((object) => objectType(object) === "bracket" && object.visible !== false && bracketKind(object) === "square")
        .map((object) => {
          const bbox = object.payload?.bbox || [];
          const translate = object.transform?.translate || [0, 0];
          const tx = Number(translate[0] || 0) + Number(bbox[0] || 0);
          const ty = Number(translate[1] || 0) + Number(bbox[1] || 0);
          const width = Number(bbox[2] || 0);
          const height = Number(bbox[3] || 0);
          const side = bracketSide(object);
          const xCandidates = side
            ? [side === "right" ? tx + width : tx, tx + width * 0.5, side === "right" ? tx : tx + width]
            : [tx, tx + width, tx + width * 0.5];
          const yCandidates = [ty + height * 0.25, ty + height * 0.5, ty + height * 0.75, ty, ty + height];
          for (const x of xCandidates) {
            for (const y of yCandidates) {
              const client = window.__chemsemaDebug.worldToClient(x, y);
              if (!client
                || client.x <= 80
                || client.x >= innerWidth - 80
                || client.y <= 120
                || client.y >= innerHeight - 80) {
                continue;
              }
              const hit = JSON.parse(window.__chemsemaDebug.state.editorEngine.contextHitTestJson?.(x, y) || "null");
              if (hit?.objectId === object.id) {
                return {
                  id: object.id,
                  x: client.x,
                  y: client.y,
                  worldX: x,
                  worldY: y,
                  side,
                };
              }
            }
          }
          return null;
        })
        .filter(Boolean);
    });
    if (!targets.length) {
      console.log("[viewer-interaction-smoke] skipping square bracket hover; no visible square bracket targets");
      return;
    }
    const failures = [];
    for (const target of targets) {
      await page.evaluate(() => {
        window.__chemsemaDebug.state.editorEngine.clearInteraction?.();
        document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
      });
      await page.mouse.move(Math.max(1, target.x - 30), Math.max(1, target.y - 30));
      await page.waitForTimeout(20);
      await page.mouse.move(target.x, target.y);
      try {
        await page.waitForFunction(() => {
          const overlay = document.querySelector('[data-layer="editor-overlay"]');
          return (overlay?.querySelectorAll('[data-role="hover-shape-handle"]').length || 0) > 0;
        }, null, { timeout: 800 });
      } catch {
        const debug = await page.evaluate((probe) => ({
          target: probe,
          hit: JSON.parse(window.__chemsemaDebug.state.editorEngine.contextHitTestJson?.(probe.worldX, probe.worldY) || "null"),
          interaction: JSON.parse(window.__chemsemaDebug.state.editorEngine.interactionRenderListJson?.() || "[]"),
          handles: document.querySelectorAll('[data-role="hover-shape-handle"]').length,
          overlayChildren: document.querySelector('[data-layer="editor-overlay"]')?.childElementCount || 0,
        }), target);
        failures.push(debug);
      }
    }
    assert(!failures.length, `Large CDXML square bracket hover failures: ${JSON.stringify(failures.slice(0, 5))}`);
  }
  
  async function verifyImportedBracketSideDragIsolation(page) {
    await page.keyboard.press("Escape").catch(() => {});
    await page.evaluate(() => {
      window.__chemsemaDebug.state.editorEngine.clearSelection?.();
      window.__chemsemaDebug.state.editorEngine.clearInteraction?.();
      window.__chemsemaDebug.clearActiveSelectionGesture?.();
      document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
    });
    await page.locator('button[data-tool="select"]').click();
    const target = await page.evaluate(() => {
      const documentData = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson?.() || "null")
        || window.__chemsemaDebug.document;
      const objectType = (object) => object?.type || object?.objectType || object?.object_type;
      const payloadValue = (object, key) => object?.payload?.[key] || object?.payload?.extra?.[key] || "";
      const hitTargetForSide = (object) => {
        const bbox = object.payload?.bbox || [];
        const translate = object.transform?.translate || [0, 0];
        const tx = Number(translate[0] || 0) + Number(bbox[0] || 0);
        const ty = Number(translate[1] || 0) + Number(bbox[1] || 0);
        const width = Number(bbox[2] || 0);
        const height = Number(bbox[3] || 0);
        const side = payloadValue(object, "side");
        const xCandidates = side === "right"
          ? [tx + width - 0.5, tx + width, tx + width * 0.5]
          : [tx + 0.5, tx, tx + width * 0.5];
        const yCandidates = [ty + height * 0.5, ty + height * 0.25, ty + height * 0.75];
        for (const x of xCandidates) {
          for (const y of yCandidates) {
            const client = window.__chemsemaDebug.worldToClient(x, y);
            if (!client
              || client.x <= 80
              || client.x >= innerWidth - 80
              || client.y <= 120
              || client.y >= innerHeight - 80) {
              continue;
            }
            const hit = JSON.parse(window.__chemsemaDebug.state.editorEngine.contextHitTestJson?.(x, y) || "null");
            if (hit?.objectId === object.id) {
              return { x: client.x, y: client.y, worldX: x, worldY: y };
            }
          }
        }
        return null;
      };
      const visit = (object, out = []) => {
        if (!object) {
          return out;
        }
        out.push(object);
        for (const child of object.children || []) {
          visit(child, out);
        }
        return out;
      };
      const allObjects = (documentData.objects || []).flatMap((object) => visit(object, []));
      const groups = allObjects.filter((object) => (
        objectType(object) === "group"
        && (object.meta?.kind || object.meta?.extra?.kind) === "bracket-group"
      ));
      for (const group of groups) {
        const left = (group.children || []).find((object) => objectType(object) === "bracket" && payloadValue(object, "side") === "left");
        const right = (group.children || []).find((object) => objectType(object) === "bracket" && payloadValue(object, "side") === "right");
        if (!left || !right) {
          continue;
        }
        for (const [sideObject, siblingObject] of [[left, right], [right, left]]) {
          const point = hitTargetForSide(sideObject);
          if (!point) {
            continue;
          }
          return {
            groupId: group.id,
            sideObjectId: sideObject.id,
            siblingObjectId: siblingObject.id,
            x: point.x,
            y: point.y,
            worldX: point.worldX,
            worldY: point.worldY,
          };
        }
      }
      return null;
    });
    if (!target) {
      console.log("[viewer-interaction-smoke] skipping imported bracket side drag; no visible imported bracket pair");
      return;
    }
    const before = await page.evaluate(({ sideObjectId, siblingObjectId }) => {
      const documentData = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson?.() || "null")
        || window.__chemsemaDebug.document;
      const visit = (object, out = []) => {
        if (!object) {
          return out;
        }
        out.push(object);
        for (const child of object.children || []) {
          visit(child, out);
        }
        return out;
      };
      const byId = new Map((documentData.objects || []).flatMap((object) => visit(object, [])).map((object) => [object.id, object]));
      return {
        sideTranslate: byId.get(sideObjectId)?.transform?.translate || null,
        siblingTranslate: byId.get(siblingObjectId)?.transform?.translate || null,
      };
    }, target);
    await page.mouse.move(target.x, target.y);
    await page.waitForTimeout(60);
    await page.mouse.down();
    await page.mouse.move(target.x + 18, target.y + 9, { steps: 4 });
    await page.mouse.up();
    await page.waitForTimeout(250);
    const after = await page.evaluate(({ sideObjectId, siblingObjectId }) => {
      const documentData = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson?.() || "null")
        || window.__chemsemaDebug.document;
      const visit = (object, out = []) => {
        if (!object) {
          return out;
        }
        out.push(object);
        for (const child of object.children || []) {
          visit(child, out);
        }
        return out;
      };
      const byId = new Map((documentData.objects || []).flatMap((object) => visit(object, [])).map((object) => [object.id, object]));
      const selection = window.__chemsemaDebug.engineState?.selection
        || window.__chemsemaDebug.getEngineState?.()?.selection
        || {};
      const arrowObjects = selection.arrowObjects || selection.arrow_objects || [];
      return {
        sideTranslate: byId.get(sideObjectId)?.transform?.translate || null,
        siblingTranslate: byId.get(siblingObjectId)?.transform?.translate || null,
        arrowObjects,
      };
    }, target);
    assert(
      JSON.stringify(after.sideTranslate) !== JSON.stringify(before.sideTranslate),
      `Imported bracket side did not move: ${JSON.stringify({ target, before, after })}`,
    );
    assert(
      JSON.stringify(after.siblingTranslate) === JSON.stringify(before.siblingTranslate),
      `Imported bracket side drag moved its sibling: ${JSON.stringify({ target, before, after })}`,
    );
    assert(
      after.arrowObjects.length === 1 && after.arrowObjects[0] === target.sideObjectId,
      `Imported bracket side drag did not leave only the dragged side selected: ${JSON.stringify({ target, after })}`,
    );
  }
  
  async function verifyMixedObjectFollowsStructureDrag(page, structureTarget, objectTarget, kind) {
    if (!structureTarget || !objectTarget) {
      return;
    }
    await page.keyboard.press("Escape").catch(() => {});
    await page.evaluate(() => {
      window.__chemsemaDebug.state.editorEngine.clearSelection?.();
      window.__chemsemaDebug.state.editorEngine.clearInteraction?.();
      window.__chemsemaDebug.clearActiveSelectionGesture?.();
      document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
    });
    await page.locator('button[data-tool="select"]').click();
    await page.mouse.click(structureTarget.x, structureTarget.y);
    await page.keyboard.down("Shift");
    await page.mouse.click(objectTarget.x, objectTarget.y);
    await page.keyboard.up("Shift");
    try {
      await page.waitForFunction((objectId) => {
        const selection = window.__chemsemaDebug.engineState?.selection || {};
        const hasStructure = (selection.nodes || []).length > 0
          || (selection.labelNodes || []).length > 0
          || (selection.bonds || []).length > 0;
        return hasStructure
          && ((selection.textObjects || []).includes(objectId) || (selection.arrowObjects || []).includes(objectId));
      }, objectTarget.id, { timeout: 1200 });
    } catch (error) {
      const diagnostics = await page.evaluate((target) => {
        const matrix = document.querySelector("#viewer-svg")?.getScreenCTM?.();
        const world = matrix
          ? new DOMPoint(target.x, target.y).matrixTransform(matrix.inverse())
          : null;
        return {
          selection: window.__chemsemaDebug.engineState?.selection || window.__chemsemaDebug.getEngineState?.()?.selection || null,
          world: world ? { x: world.x, y: world.y } : null,
          contextHit: world ? window.__chemsemaDebug.state.editorEngine.contextHitTestJson?.(world.x, world.y) : null,
          object: window.__chemsemaDebug.document?.objects
            ?.flatMap((object) => {
              const out = [];
              const visit = (candidate) => {
                if (!candidate) return;
                out.push(candidate);
                for (const child of candidate.children || []) visit(child);
              };
              visit(object);
              return out;
            })
            .find((object) => object.id === target.id) || null,
        };
      }, objectTarget);
      throw new Error(`${kind} mixed selection did not include both targets: ${JSON.stringify({ ...diagnostics, structureTarget, objectTarget })}`);
    }
    const dragPoint = objectTarget;
    const before = await page.evaluate((objectId) => {
      const rects = [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`)]
        .filter((element) => !element.classList.contains("document-diagnostic-marker"))
        .map((element) => element.getBoundingClientRect())
        .filter((rect) => rect.width > 0 && rect.height > 0);
      const left = Math.min(...rects.map((rect) => rect.left));
      const top = Math.min(...rects.map((rect) => rect.top));
      const right = Math.max(...rects.map((rect) => rect.right));
      const bottom = Math.max(...rects.map((rect) => rect.bottom));
      return {
        count: rects.length,
        x: (left + right) * 0.5,
        y: (top + bottom) * 0.5,
        centers: rects.slice(0, 200).map((rect) => ({
          x: rect.left + rect.width * 0.5,
          y: rect.top + rect.height * 0.5,
        })),
      };
    }, objectTarget.id);
    await page.mouse.move(dragPoint.x, dragPoint.y);
    await page.mouse.down();
    await page.mouse.move(dragPoint.x + 42, dragPoint.y + 18, { steps: 6 });
    try {
      await page.waitForFunction((objectId) => {
        const elements = [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`)]
          .filter((element) => !element.classList.contains("document-diagnostic-marker"));
        return elements.some((element) => element.classList.contains("is-preview-transforming"));
      }, objectTarget.id, { timeout: 700 });
    } catch (error) {
      const diagnostics = await page.evaluate((objectId) => ({
        gesture: window.__chemsemaDebug.getActiveSelectionGesture?.() || null,
        selection: window.__chemsemaDebug.engineState?.selection || window.__chemsemaDebug.getEngineState?.()?.selection || null,
        previewStats: window.__chemsemaDebug.backendMovePreviewStats?.last || null,
        schedulerStats: window.__chemsemaDebug.backendPreviewSchedulerStats || null,
        elements: [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`)]
          .map((element) => ({
            tag: element.tagName,
            classes: element.getAttribute("class"),
            transform: element.getAttribute("transform"),
            styleTransform: element.style.transform,
          })),
      }), objectTarget.id);
      throw new Error(`${kind} mixed drag did not apply preview transform: ${JSON.stringify({ diagnostics, structureTarget, dragPoint, objectTarget })}`);
    }
    const during = await page.evaluate(([objectId, beforeCenters]) => {
      const rects = [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`)]
        .filter((element) => !element.classList.contains("document-diagnostic-marker"))
        .map((element) => element.getBoundingClientRect())
        .filter((rect) => rect.width > 0 && rect.height > 0);
      const left = Math.min(...rects.map((rect) => rect.left));
      const top = Math.min(...rects.map((rect) => rect.top));
      const right = Math.max(...rects.map((rect) => rect.right));
      const bottom = Math.max(...rects.map((rect) => rect.bottom));
      const staleCenters = rects.filter((rect) => {
        const cx = rect.left + rect.width * 0.5;
        const cy = rect.top + rect.height * 0.5;
        return (beforeCenters || []).some((before) => Math.hypot(cx - before.x, cy - before.y) < 2);
      }).length;
      return {
        count: rects.length,
        x: (left + right) * 0.5,
        y: (top + bottom) * 0.5,
        staleCenters,
        transforming: [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`)]
          .filter((element) => element.classList.contains("is-preview-transforming")).length,
      };
    }, [objectTarget.id, before.centers]);
    await page.mouse.up();
    await page.waitForTimeout(150);
    const moved = Math.hypot(during.x - before.x, during.y - before.y);
    assert(before.count > 0 && during.count > 0, `${kind} mixed drag target was not rendered: ${JSON.stringify({ before, during, objectTarget })}`);
    assert(moved > 12, `${kind} did not follow mixed molecule drag preview: ${JSON.stringify({ before, during, moved, structureTarget, objectTarget })}`);
    assert(during.transforming > 0, `${kind} mixed drag did not use object preview transform: ${JSON.stringify({ before, during, objectTarget })}`);
    assert(during.staleCenters === 0, `${kind} left stale object primitives at the drag origin: ${JSON.stringify({ before, during, structureTarget, objectTarget })}`);
    const documentSync = await page.evaluate((objectId) => {
      const translateForObject = (doc, wantedObjectId) => {
        const visit = (object) => {
          if (!object) {
            return null;
          }
          if (object.id === wantedObjectId) {
            const translate = object.transform?.translate || object.transform?.translation || [0, 0];
            return {
              x: Number(translate[0] || 0),
              y: Number(translate[1] || 0),
            };
          }
          for (const child of object.children || []) {
            const found = visit(child);
            if (found) {
              return found;
            }
          }
          return null;
        };
        for (const object of doc?.objects || []) {
          const found = visit(object);
          if (found) {
            return found;
          }
        }
        return null;
      };
      const engineDoc = JSON.parse(window.__chemsemaDebug.state.editorEngine.documentJson?.() || "null");
      const frontendDoc = window.__chemsemaDebug.document;
      const engine = translateForObject(engineDoc, objectId);
      const frontend = translateForObject(frontendDoc, objectId);
      const distance = engine && frontend ? Math.hypot(engine.x - frontend.x, engine.y - frontend.y) : null;
      return { engine, frontend, distance };
    }, objectTarget.id);
    assert(
      documentSync.distance !== null && documentSync.distance < 0.01,
      `${kind} drag left the front-end object model stale after commit: ${JSON.stringify({ structureTarget, objectTarget, documentSync })}`,
    );
  }
  
  async function verifyObjectOnlySelectionDragPreview(page, target, kind) {
    if (!target) {
      return;
    }
    await page.keyboard.press("Escape").catch(() => {});
    await page.evaluate(() => {
      window.__chemsemaDebug.state.editorEngine.clearSelection?.();
      window.__chemsemaDebug.state.editorEngine.clearInteraction?.();
      window.__chemsemaDebug.clearActiveSelectionGesture?.();
      document.querySelector('[data-layer="editor-overlay"]')?.replaceChildren();
    });
    await page.locator('button[data-tool="select"]').click();
    await page.mouse.click(target.x, target.y);
    const selected = await page.waitForFunction((targetId) => {
      const selection = window.__chemsemaDebug.engineState?.selection
        || window.__chemsemaDebug.getEngineState?.()?.selection
        || {};
      const objectId = (selection.textObjects || []).includes(targetId)
        ? targetId
        : (selection.arrowObjects || []).includes(targetId)
        ? targetId
        : (selection.textObjects || [])[0] || (selection.arrowObjects || [])[0] || "";
      return objectId ? { selection, objectId } : false;
    }, target.id, { timeout: 1200 }).then((handle) => handle.jsonValue());
    const before = await page.evaluate((objectId) => {
      const bounds = JSON.parse(window.__chemsemaDebug.state.editorEngine.selectionBoundsJson?.() || "null");
      const center = bounds
        ? window.__chemsemaDebug.worldToClient((bounds.minX + bounds.maxX) * 0.5, (bounds.minY + bounds.maxY) * 0.5)
        : null;
      const rects = [...document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`)]
        .filter((element) => !element.classList.contains("document-diagnostic-marker"))
        .map((element) => element.getBoundingClientRect())
        .filter((rect) => rect.width > 0 || rect.height > 0);
      const left = rects.length ? Math.min(...rects.map((rect) => rect.left)) : null;
      const top = rects.length ? Math.min(...rects.map((rect) => rect.top)) : null;
      const right = rects.length ? Math.max(...rects.map((rect) => rect.right)) : null;
      const bottom = rects.length ? Math.max(...rects.map((rect) => rect.bottom)) : null;
      return {
        bounds,
        dragStart: center,
        count: rects.length,
        x: rects.length ? (left + right) * 0.5 : null,
        y: rects.length ? (top + bottom) * 0.5 : null,
        centers: rects.slice(0, 240).map((rect) => ({
          x: rect.left + rect.width * 0.5,
          y: rect.top + rect.height * 0.5,
        })),
      };
    }, selected.objectId);
    assert(before.count > 0 && before.dragStart, `${kind} did not expose a selected object drag target: ${JSON.stringify({ target, selected, before })}`);
  
    await page.mouse.move(before.dragStart.x, before.dragStart.y);
    await page.waitForTimeout(140);
    const cursor = await page.evaluate(() => ({
      svg: getComputedStyle(document.querySelector("#viewer-svg")).cursor,
      container: getComputedStyle(document.querySelector("#viewer-container")).cursor,
      shield: getComputedStyle(document.querySelector(".canvas-pointer-shield")).cursor,
    }));
    assert(
      [cursor.svg, cursor.container, cursor.shield].includes("grab"),
      `${kind} selection box interior did not show grab cursor: ${JSON.stringify({ target, selected, before, cursor })}`,
    );
  
    await page.mouse.down();
    await page.mouse.move(before.dragStart.x + 70, before.dragStart.y + 35, { steps: 8 });
    await page.waitForTimeout(160);
    const during = await page.evaluate(([objectId, beforeCenters]) => {
      const elements = [...document.querySelectorAll(
        `[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"], [data-layer="document-batch-preview"] [data-object-id="${CSS.escape(objectId)}"]`,
      )].filter((element) => !element.classList.contains("document-diagnostic-marker"));
      const rects = elements
        .filter((element) => {
          const style = getComputedStyle(element);
          return style.visibility !== "hidden" && style.display !== "none";
        })
        .map((element) => element.getBoundingClientRect())
        .filter((rect) => rect.width > 0 || rect.height > 0);
      const left = rects.length ? Math.min(...rects.map((rect) => rect.left)) : null;
      const top = rects.length ? Math.min(...rects.map((rect) => rect.top)) : null;
      const right = rects.length ? Math.max(...rects.map((rect) => rect.right)) : null;
      const bottom = rects.length ? Math.max(...rects.map((rect) => rect.bottom)) : null;
      const staleCenters = rects.filter((rect) => {
        const cx = rect.left + rect.width * 0.5;
        const cy = rect.top + rect.height * 0.5;
        return (beforeCenters || []).some((before) => Math.hypot(cx - before.x, cy - before.y) < 2);
      }).length;
      return {
        gesture: window.__chemsemaDebug.getActiveSelectionGesture?.() || null,
        count: rects.length,
        x: rects.length ? (left + right) * 0.5 : null,
        y: rects.length ? (top + bottom) * 0.5 : null,
        staleCenters,
        transforming: elements.filter((element) => element.classList.contains("is-preview-transforming")).length,
        transformSamples: elements
          .filter((element) => element.classList.contains("is-preview-transforming"))
          .map((element) => element.getAttribute("transform") || "")
          .slice(0, 8),
        batchChildren: document.querySelector('[data-layer="document-batch-preview"]')?.childElementCount || 0,
      };
    }, [selected.objectId, before.centers]);
    await page.mouse.up();
    await page.waitForTimeout(120);
    const moved = before.x != null && during.x != null
      ? Math.hypot(during.x - before.x, during.y - before.y)
      : 0;
    assert(
      !during.transformSamples.some((transform) => /\bscale\(/.test(transform)),
      `${kind} selection box drag used resize/scale preview instead of move: ${JSON.stringify({ target, selected, before, during })}`,
    );
    assert(during.count > 0 && moved > 12, `${kind} object-only drag did not visually move: ${JSON.stringify({ target, selected, before, during, moved })}`);
    assert(during.transforming > 0 || during.batchChildren > 0, `${kind} object-only drag did not use document preview transform: ${JSON.stringify({ target, selected, before, during })}`);
    assert(during.staleCenters === 0, `${kind} left visible object primitives at the drag origin: ${JSON.stringify({ target, selected, before, during })}`);
  }
  
  async function resetViewerUi(page) {
    await page.keyboard.press("Escape").catch(() => {});
    await page.keyboard.press("Escape").catch(() => {});
    await page.waitForFunction(() => !window.__chemsemaDebug?.activeTextEditor, null, { timeout: 800 }).catch(() => {});
    await page.waitForTimeout(30);
  }
  
  async function measureCommitLatency(page, label, action, predicate, predicateArg = null, thresholdMs = 350) {
    const started = await page.evaluate(() => performance.now());
    const actionStarted = Date.now();
    await action();
    const actionMs = Date.now() - actionStarted;
    const waitStarted = Date.now();
    await page.waitForFunction(predicate, predicateArg, { timeout: 1500 });
    const waitMs = Date.now() - waitStarted;
    const elapsed = await page.evaluate((start) => performance.now() - start, started);
    if (elapsed >= thresholdMs) {
      const diagnostics = await page.evaluate((start) => ({
        measureStartedAt: start,
        measureEndedAt: performance.now(),
        commitTiming: window.__chemsemaDebug?.creationCommitStats?.last || null,
        lastCommandResult: JSON.parse(window.__chemsemaDebug?.state?.editorEngine?.lastCommandResultJson?.() || "null"),
      }), started);
      assert(false, `${label} committed too slowly: ${elapsed.toFixed(1)}ms (action=${actionMs}ms wait=${waitMs}ms) ${JSON.stringify(diagnostics)}`);
    }
    return elapsed;
  }
  
  async function verifyLargeFileCommitLatency(page) {
    const box = await page.locator("#viewer-container").boundingBox();
    const bracketStart = { x: box.x + box.width - 360, y: box.y + box.height - 330 };
    const bracketEnd = { x: bracketStart.x + 130, y: bracketStart.y + 120 };
    await resetViewerUi(page);
    await page.locator('button[data-tool="bracket"]').click();
    await page.waitForFunction(() => getComputedStyle(document.querySelector("#viewer-svg")).pointerEvents === "none");
    await page.evaluate(() => {
      const engine = window.__chemsemaDebug.state.editorEngine;
      window.__viewerSmokeEngineTimings = [];
      for (const name of ["pointerMove", "interactionRenderListJson"]) {
        const original = engine?.[name];
        if (typeof original !== "function" || original.__viewerSmokeWrapped) {
          continue;
        }
        const wrapped = function (...args) {
          const start = performance.now();
          const result = original.apply(this, args);
          window.__viewerSmokeEngineTimings.push({
            name,
            ms: performance.now() - start,
          });
          return result;
        };
        wrapped.__viewerSmokeWrapped = true;
        engine[name] = wrapped;
      }
    });
    const bracketBefore = await page.evaluate(() => ({
      activeTool: window.__chemsemaDebug.engineState?.tool?.activeTool
        || window.__chemsemaDebug.engineState?.tool?.active_tool
        || null,
      selection: window.__chemsemaDebug.engineState?.selection || null,
      activeGesture: window.__chemsemaDebug.activeSelectionGesture || null,
      overlayChildren: document.querySelector('[data-layer="editor-overlay"]')?.childElementCount || 0,
      documentChildren: document.querySelector('[data-layer="document-content"]')?.childElementCount || 0,
      documentPointerEvents: document.querySelector('[data-layer="document-content"]')?.getAttribute("pointer-events") || getComputedStyle(document.querySelector('[data-layer="document-content"]')).pointerEvents,
      totalSvgElements: document.querySelectorAll("#viewer-svg *").length,
    }));
    const bracketMs = await measureCommitLatency(
      page,
      "Large CDXML bracket label editor",
      async () => {
        let stepStarted = Date.now();
        await page.mouse.move(bracketStart.x, bracketStart.y);
        const moveMs = Date.now() - stepStarted;
        stepStarted = Date.now();
        await page.mouse.down();
        const shieldAfterDown = await page.evaluate(() => document.querySelector(".canvas-pointer-shield")?.className || "");
        const downMs = Date.now() - stepStarted;
        stepStarted = Date.now();
        await page.mouse.move(bracketEnd.x, bracketEnd.y);
        const dragMs = Date.now() - stepStarted;
        stepStarted = Date.now();
        await page.mouse.up();
        const upMs = Date.now() - stepStarted;
        await page.evaluate((timing) => {
          window.__viewerSmokeBracketTiming = timing;
        }, { moveMs, downMs, dragMs, upMs, shieldAfterDown });
      },
      () => !!window.__chemsemaDebug.activeTextEditor?.bracketLabelObjectId,
      null,
      60000,
    );
    const bracketTiming = await page.evaluate(() => window.__viewerSmokeBracketTiming || null);
    const engineTimings = await page.evaluate(() => window.__viewerSmokeEngineTimings || []);
    const bracketActiveMs = (bracketTiming?.downMs || 0) + (bracketTiming?.dragMs || 0) + (bracketTiming?.upMs || 0);
    assert(bracketActiveMs < 1500, `Large CDXML bracket label editor committed too slowly: ${bracketMs.toFixed(1)}ms ${JSON.stringify({ bracketTiming, bracketActiveMs, bracketBefore, engineTimings: engineTimings.slice(-20) })}`);
  
    await resetViewerUi(page);
    await page.locator('button[data-tool="symbol"]').click();
    await page.waitForFunction(() => getComputedStyle(document.querySelector("#viewer-svg")).pointerEvents === "none");
    const symbolPoint = { x: bracketStart.x - 70, y: bracketStart.y + 35 };
    const symbolMs = await measureCommitLatency(
      page,
      "Large CDXML charge symbol",
      async () => {
        await page.mouse.click(symbolPoint.x, symbolPoint.y);
      },
      () => {
        const result = JSON.parse(window.__chemsemaDebug.state.editorEngine.lastCommandResultJson?.() || "null");
        const objectId = [
          ...(result?.targets?.objects || []),
          ...(result?.created?.objects || []),
          ...(result?.updated?.objects || []),
        ].find((id) => String(id || "").startsWith("obj_symbol")) || "";
        return result?.changed
          && objectId
          && document.querySelectorAll(`[data-object-id="${CSS.escape(objectId)}"]`).length > 0;
      },
    );
  
    await resetViewerUi(page);
    await page.locator('button[data-tool="bond"]').click();
    await page.waitForFunction(() => getComputedStyle(document.querySelector("#viewer-svg")).pointerEvents === "none");
    const bondStart = { x: bracketStart.x - 160, y: bracketStart.y + 170 };
    const bondEnd = { x: bondStart.x + 115, y: bondStart.y };
    let bondPreviewBeforeUp = null;
    let bondCleanupBeforeCommit = null;
    await page.evaluate(() => {
      if (window.__chemsemaDebug) {
        window.__chemsemaDebug.creationPreviewClearedBeforeCommitAt = null;
      }
    });
    const bondMs = await measureCommitLatency(
      page,
      "Large CDXML bond hover cleanup",
      async () => {
        await page.mouse.move(bondStart.x, bondStart.y);
        await page.mouse.down();
        await page.mouse.move(bondEnd.x, bondEnd.y);
        bondPreviewBeforeUp = await page.evaluate(() => ({
          dragChildren: document.querySelector(".canvas-drag-preview-svg")?.childElementCount || 0,
          dragRoles: [...document.querySelectorAll(".canvas-drag-preview-svg [data-role]")]
            .map((node) => node.getAttribute("data-role") || ""),
          creationPreviewChildren: document.querySelector('[data-layer="document-bond-creation-preview"]')?.childElementCount || 0,
          creationPreviewBond: !!document.querySelector('[data-layer="document-bond-creation-preview"] [data-bond-id="__preview_bond"]'),
        }));
        const started = await page.evaluate(() => performance.now());
        const upPromise = page.mouse.up();
        const cleanupHandle = await page.waitForFunction((startTime) => {
          const dragChildren = document.querySelector(".canvas-drag-preview-svg")?.childElementCount || 0;
          const creationPreview = document.querySelector('[data-layer="document-bond-creation-preview"]');
          if (dragChildren !== 0 || creationPreview) {
            return false;
          }
          const stats = window.__chemsemaDebug?.creationCommitStats?.last || null;
          return {
            elapsedMs: performance.now() - startTime,
            commitAlreadyRecorded: !!stats && Number(stats.commitStartedAt || 0) >= startTime,
            commitStartedAt: stats?.commitStartedAt || null,
            previewClearedAt: window.__chemsemaDebug?.creationPreviewClearedBeforeCommitAt || null,
          };
        }, started, { timeout: 1000 });
        bondCleanupBeforeCommit = await cleanupHandle.jsonValue();
        await upPromise;
      },
      () => {
        const overlay = document.querySelector('[data-layer="editor-overlay"]');
        return !overlay?.querySelector('[data-role^="preview-"], [data-role^="hover-"]');
      },
      null,
      3000,
    );
    assert(
      bondPreviewBeforeUp?.dragChildren > 0 || bondPreviewBeforeUp?.creationPreviewChildren > 0,
      `Large CDXML bond drag did not expose a preview before pointerup: ${JSON.stringify(bondPreviewBeforeUp)}`,
    );
    assert(
      bondCleanupBeforeCommit
        && bondCleanupBeforeCommit.elapsedMs < 120,
      `Large CDXML bond preview did not clear promptly after commit: ${JSON.stringify({ bondCleanupBeforeCommit, bondPreviewBeforeUp })}`,
    );
  
    await resetViewerUi(page);
    await page.locator('button[data-tool="orbital"]').click();
    await page.waitForFunction(() => getComputedStyle(document.querySelector("#viewer-svg")).pointerEvents === "none");
    const orbitalStart = { x: bondStart.x + 210, y: bondStart.y + 70 };
    const orbitalEnd = { x: orbitalStart.x + 90, y: orbitalStart.y + 85 };
    await page.mouse.move(orbitalStart.x, orbitalStart.y);
    await page.mouse.down();
    await page.mouse.move(orbitalEnd.x, orbitalEnd.y, { steps: 6 });
    await page.mouse.up();
    await page.waitForFunction(() => {
      const command = JSON.parse(window.__chemsemaDebug.state.editorEngine.lastCommandResultJson?.() || "null");
      return !!(command?.targets?.objects?.[0] || command?.created?.objects?.[0]);
    }, null, { timeout: 1500 });
    const orbitalObjectId = await page.evaluate(() => {
      const command = JSON.parse(window.__chemsemaDebug.state.editorEngine.lastCommandResultJson?.() || "null");
      return command?.targets?.objects?.[0] || command?.created?.objects?.[0] || "";
    });
    await page.locator('button[data-tool="select"]').click();
    await page.waitForTimeout(160);
    const orbitalSelectState = await page.evaluate((objectId) => ({
      objectDomCount: document.querySelectorAll(`[data-layer="document-content"] [data-object-id="${CSS.escape(objectId)}"]`).length,
      selectionCount: document.querySelectorAll('[data-layer="editor-overlay"] [data-role^="selection-"]').length,
      hoverCount: document.querySelectorAll('[data-layer="editor-overlay"] [data-role^="hover-"]').length,
      previewCount: document.querySelectorAll('[data-layer="editor-overlay"] [data-role^="preview-"]').length,
    }), orbitalObjectId);
    assert(orbitalSelectState.objectDomCount > 0, `Large CDXML orbital disappeared after switching to select: ${JSON.stringify({ orbitalObjectId, orbitalSelectState })}`);
    assert(
      orbitalSelectState.hoverCount === 0 && orbitalSelectState.previewCount === 0,
      `Large CDXML orbital switch to select left hover/preview overlay: ${JSON.stringify({ orbitalObjectId, orbitalSelectState })}`,
    );
  
    return { bracketMs, symbolMs, bondMs };
  }
  
  async function verifyLargeFileHoverAndDrag(browser) {
    if (!largeCdxml || !existsSync(largeCdxml)) {
      const reason = largeCdxml
        ? `missing configured private file ${basename(largeCdxml)}`
        : "set CHEMSEMA_STABILITY_PRIVATE_CDXML to enable";
      console.log(`[viewer-interaction-smoke] skipping private large-file hover; ${reason}`);
      return;
    }
    const { page, errors, sourcePage } = await openLargeCdxmlViewer(browser);
    await page.locator('button[data-tool="select"]').click();
    let targets = await page.evaluate(largeFileTargetFinder);
    assert(targets.hover, "Large CDXML did not expose a visible hover target.");
    assert(targets.label, "Large CDXML did not expose a draggable label node target.");
    assert(targets.atom, "Large CDXML did not expose a draggable atom node target.");
  
    await page.mouse.move(targets.hover.x, targets.hover.y);
    await page.waitForTimeout(250);
    const hover = await page.evaluate(() => {
      const overlay = document.querySelector('[data-layer="editor-overlay"]');
      return overlay?.querySelectorAll('[data-role^="hover-"]').length || 0;
    });
    assert(hover > 0, "Large CDXML select hover did not render a hover overlay.");
  
    await verifyLargeFileSelectionLatency(page, targets.hover);
    await verifyDiagnosticMarkerHidesDuringDrag(page, targets.invalidDiagnostic, targets.textObject || targets.bracket || targets.label || targets.atom);
    targets = await page.evaluate(largeFileTargetFinder);
    if (targets.bracket) {
      await verifyBracketHoverFocus(page, targets.bracket);
      await verifyAllSquareBracketsHover(page);
      await verifyImportedBracketSideDragIsolation(page);
    } else if (targets.bracketCount > 0) {
      throw new Error(`Large CDXML contains ${targets.bracketCount} bracket objects, but none exposed a visible hit target.`);
    } else {
      console.log("[viewer-interaction-smoke] skipping bracket-specific large-file checks; no visible bracket target");
    }
    await verifyLargeRegionSelectionDoesNotDragGroup(page, targets.atom);
    targets = await page.evaluate(largeFileTargetFinder);
    await verifyLargeDragTarget(page, targets.label, "Label");
    targets = await page.evaluate(largeFileTargetFinder);
    await verifyMixedObjectFollowsStructureDrag(page, targets.atom || targets.label, targets.textObject || targets.bracket, "Large CDXML text/bracket");
    targets = await page.evaluate(largeFileTargetFinder);
    await verifyObjectOnlySelectionDragPreview(page, targets.textObject, "Large CDXML text object");
    targets = await page.evaluate(largeFileTargetFinder);
    await verifyObjectOnlySelectionDragPreview(page, targets.bracket, "Large CDXML bracket object");
    await page.close();
    await sourcePage?.close().catch(() => {});
    const { page: latencyPage, errors: latencyErrors, sourcePage: latencySourcePage } = await openLargeCdxmlViewer(browser);
    const latency = await verifyLargeFileCommitLatency(latencyPage);
    await latencyPage.close();
    await latencySourcePage?.close().catch(() => {});
    console.log(`[viewer-interaction-smoke] large commit latency bracket=${latency.bracketMs.toFixed(1)}ms symbol=${latency.symbolMs.toFixed(1)}ms bond=${latency.bondMs.toFixed(1)}ms`);
    assert(!errors.length && !latencyErrors.length, `Viewer console errors during large-file hover: ${[...errors, ...latencyErrors].join("\n")}`);
  }

  return { verifyLargeFileHoverAndDrag };
}
