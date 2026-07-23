export function createImageImportHost(scope) {
  const { svgPointFromEvent, activeViewBox, getPendingImageInsertWorldPoint, setPendingImageInsertWorldPoint, imageFileInput, isEditingRustDocument, commandEngine, activateEditorTool, renderDocumentChange, desktopFileHost } = scope;

  function imageMimeTypeFromName(name, declaredMimeType = "") {
    const normalized = String(declaredMimeType || "").toLowerCase();
    if (["image/png", "image/jpeg", "image/gif", "image/bmp"].includes(normalized)) {
      return normalized;
    }
    const extension = String(name || "").split(".").pop()?.toLowerCase();
    return {
      png: "image/png",
      jpg: "image/jpeg",
      jpeg: "image/jpeg",
      gif: "image/gif",
      bmp: "image/bmp",
    }[extension] || "";
  }

  function blobDataBase64(blob) {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onerror = () => reject(reader.error || new Error("Failed to read image."));
      reader.onload = () => {
        const value = String(reader.result || "");
        const comma = value.indexOf(",");
        if (comma < 0) {
          reject(new Error("Image data is not a valid data URL."));
          return;
        }
        resolve(value.slice(comma + 1));
      };
      reader.readAsDataURL(blob);
    });
  }

  async function decodedImageDimensions(blob) {
    if (typeof createImageBitmap === "function") {
      const bitmap = await createImageBitmap(blob);
      try {
        return { width: bitmap.width, height: bitmap.height };
      } finally {
        bitmap.close?.();
      }
    }
    const url = URL.createObjectURL(blob);
    try {
      const image = new Image();
      image.src = url;
      await image.decode();
      return { width: image.naturalWidth, height: image.naturalHeight };
    } finally {
      URL.revokeObjectURL(url);
    }
  }

  function droppedImageCenter(clientPoint, index) {
    const base = clientPoint?.worldPoint
      && Number.isFinite(clientPoint.worldPoint.x)
      && Number.isFinite(clientPoint.worldPoint.y)
      ? clientPoint.worldPoint
      : clientPoint && Number.isFinite(clientPoint.clientX) && Number.isFinite(clientPoint.clientY)
        ? svgPointFromEvent(clientPoint)
        : (() => {
          const box = activeViewBox();
          return { x: box.x + box.width * 0.5, y: box.y + box.height * 0.5 };
        })();
    return { x: base.x + index * 12, y: base.y + index * 12 };
  }

  function openImageFilePickerAt(worldPoint = null) {
    setPendingImageInsertWorldPoint(worldPoint
      && Number.isFinite(worldPoint.x)
      && Number.isFinite(worldPoint.y)
      ? { x: worldPoint.x, y: worldPoint.y }
      : null);
    imageFileInput.value = "";
    imageFileInput.click();
  }

  function consumeImageInsertWorldPoint() {
    const point = getPendingImageInsertWorldPoint();
    setPendingImageInsertWorldPoint(null);
    return point;
  }

  async function insertImagePayload(image, clientPoint, index = 0, source = "image-import") {
    if (!isEditingRustDocument()) {
      throw new Error("Images can only be inserted into an editable document.");
    }
    const mimeType = imageMimeTypeFromName(image.fileName, image.mimeType);
    if (!mimeType) {
      throw new Error(`Unsupported image type: ${image.fileName || "image"}`);
    }
    const blob = image.blob || await fetch(`data:${mimeType};base64,${image.dataBase64}`).then((response) => response.blob());
    if (!blob.size || blob.size > 64 * 1024 * 1024) {
      throw new Error(`${image.fileName || "Image"} exceeds the 64 MiB image limit.`);
    }
    const dimensions = await decodedImageDimensions(blob);
    if (!dimensions?.width || !dimensions?.height) {
      throw new Error(`${image.fileName || "Image"} cannot be decoded by the current renderer.`);
    }
    const dataBase64 = image.dataBase64 || await blobDataBase64(blob);
    const naturalWidthPt = dimensions.width * 0.75;
    const naturalHeightPt = dimensions.height * 0.75;
    const maxInitialSidePt = 320;
    const initialScale = Math.min(1, maxInitialSidePt / Math.max(naturalWidthPt, naturalHeightPt));
    const center = droppedImageCenter(clientPoint, index);
    const result = await commandEngine.executeCommand({
      type: "add-image",
      mimeType,
      dataBase64,
      pixelWidth: dimensions.width,
      pixelHeight: dimensions.height,
      position: center,
      width: Math.max(1, naturalWidthPt * initialScale),
      height: Math.max(1, naturalHeightPt * initialScale),
      sourceName: image.fileName || null,
    }, { source, label: "Insert image" });
    if (!result?.changed) {
      throw new Error(`Failed to insert ${image.fileName || "image"}.`);
    }
    activateEditorTool("select");
    renderDocumentChange(result);
    return result;
  }

  async function insertDroppedImageFiles(files, clientPoint, source = "file-drop") {
    let index = 0;
    for (const file of files) {
      await insertImagePayload({
        fileName: file.name,
        mimeType: file.type,
        blob: file,
      }, clientPoint, index++, source);
    }
  }

  async function insertDroppedImagePaths(paths, clientPoint, source = "file-drop") {
    let index = 0;
    for (const path of paths) {
      const file = await desktopFileHost.readBinaryPath(path);
      await insertImagePayload({
        fileName: file.fileName,
        mimeType: file.mimeType,
        dataBase64: file.dataBase64,
      }, clientPoint, index++, source);
    }
  }

  return { imageMimeTypeFromName, blobDataBase64, decodedImageDimensions, droppedImageCenter, openImageFilePickerAt, consumeImageInsertWorldPoint, insertImagePayload, insertDroppedImageFiles, insertDroppedImagePaths };
}
