export function createDocumentTabInteractions(options) {
  let activeDrag = null;
  let detachingTabId = null;
  let suppressNextClick = false;

  function tabElement(tabId) {
    if (!options.root || !tabId) {
      return null;
    }
    return Array.from(options.root.querySelectorAll("[data-document-tab-id]"))
      .find((element) => element.dataset.documentTabId === tabId) || null;
  }

  function setDetachingTabId(tabId) {
    if (detachingTabId === tabId) {
      return;
    }
    tabElement(detachingTabId)?.classList.remove("is-dragging");
    detachingTabId = tabId;
    tabElement(detachingTabId)?.classList.add("is-dragging");
  }

  function clearDrag() {
    activeDrag = null;
    setDetachingTabId(null);
  }

  function bind() {
    if (!options.root) {
      return;
    }
    options.root.addEventListener("click", options.uiActions.listener(
      "document-tab.click",
      async (event) => {
        if (suppressNextClick) {
          suppressNextClick = false;
          event.preventDefault();
          event.stopPropagation();
          return;
        }
        const close = event.target.closest("[data-document-tab-close]");
        if (close) {
          event.stopPropagation();
          await options.closeDocumentTab(close.dataset.documentTabClose);
          return;
        }
        const tab = event.target.closest("[data-document-tab-id]");
        if (tab) {
          await options.activateDocumentTab(tab.dataset.documentTabId);
        }
      },
    ));
    options.root.addEventListener("keydown", options.uiActions.listener(
      "document-tab.keyboard",
      async (event) => {
        if (event.key !== "Enter" && event.key !== " ") {
          return;
        }
        const tab = event.target.closest("[data-document-tab-id]");
        if (!tab) {
          return;
        }
        event.preventDefault();
        await options.activateDocumentTab(tab.dataset.documentTabId);
      },
    ));
    options.root.addEventListener("pointerdown", (event) => {
      if (!options.detachEnabled()
        || event.button !== 0
        || event.target.closest("[data-document-tab-close]")) {
        return;
      }
      const tab = event.target.closest("[data-document-tab-id]");
      if (!tab) {
        return;
      }
      activeDrag = {
        tabId: tab.dataset.documentTabId,
        pointerId: event.pointerId,
        startX: event.clientX,
        startY: event.clientY,
        screenX: event.screenX,
        screenY: event.screenY,
        dragging: false,
      };
      tab.setPointerCapture?.(event.pointerId);
    });
    options.root.addEventListener("pointermove", (event) => {
      const drag = activeDrag;
      if (!drag || drag.pointerId !== event.pointerId) {
        return;
      }
      drag.screenX = event.screenX;
      drag.screenY = event.screenY;
      const dx = event.clientX - drag.startX;
      const dy = event.clientY - drag.startY;
      if (!drag.dragging && Math.hypot(dx, dy) >= 8) {
        drag.dragging = true;
      }
      const titlebarBottom = options.titlebar?.getBoundingClientRect().bottom || 42;
      const shouldDetach = drag.dragging && event.clientY > titlebarBottom + 18;
      setDetachingTabId(shouldDetach ? drag.tabId : null);
    });
    options.root.addEventListener("pointerup", options.uiActions.listener(
      "document-tab.detach",
      async (event) => {
        const drag = activeDrag;
        if (!drag || drag.pointerId !== event.pointerId) {
          return;
        }
        activeDrag = null;
        const shouldDetach = detachingTabId === drag.tabId;
        setDetachingTabId(null);
        if (!shouldDetach) {
          return;
        }
        suppressNextClick = true;
        event.preventDefault();
        event.stopPropagation();
        await options.detachDocumentTab(drag.tabId, drag.screenX, drag.screenY);
      },
      { recover: clearDrag },
    ));
    options.root.addEventListener("pointercancel", clearDrag);
  }

  return {
    bind,
    isDetaching: (tabId) => detachingTabId === tabId,
  };
}
