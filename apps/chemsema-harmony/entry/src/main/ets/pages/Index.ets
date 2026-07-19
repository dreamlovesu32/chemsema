import { webview } from '@kit.ArkWeb';
import { picker, fileIo } from '@kit.CoreFileKit';
import util from '@ohos.util';

interface HarmonyBridgeRequest {
  id?: string;
  command?: string;
  payload?: Record<string, Object>;
}

interface HarmonyBridgeResponse {
  ok: boolean;
  value?: Object | boolean | null;
  error?: string;
}

interface HarmonyPathResult {
  path: string;
  fileName: string;
}

interface HarmonyReadResult {
  path: string;
  fileName: string;
  format: string;
  text: string;
}

class ChemSemaHarmonyBridge {
  private controller: webview.WebviewController;

  constructor(controller: webview.WebviewController) {
    this.controller = controller;
  }

  postMessage(message: string): string {
    this.handleMessage(message);
    return 'true';
  }

  private async handleMessage(message: string): Promise<void> {
    let request: HarmonyBridgeRequest;
    try {
      request = JSON.parse(message) as HarmonyBridgeRequest;
    } catch (error) {
      console.error(`[ChemSemaBridge] invalid message: ${this.errorMessage(error)}`);
      return;
    }

    const id = request.id || '';
    if (!id) {
      console.error('[ChemSemaBridge] missing request id');
      return;
    }

    try {
      const value = await this.handleCommand(request.command || '', request.payload || {});
      this.complete(id, { ok: true, value });
    } catch (error) {
      const message = this.errorMessage(error);
      console.error(`[ChemSemaBridge] command failed: ${request.command || ''}: ${message}`);
      this.complete(id, { ok: false, error: message });
    }
  }

  private async handleCommand(command: string, payload: Record<string, Object>): Promise<Object | boolean | null> {
    if (command === 'chooseOpenPath') {
      return this.chooseOpenPath();
    }
    if (command === 'chooseSavePath' || command === 'chooseExportSavePath') {
      return this.chooseSavePath(this.stringPayload(payload, 'suggestedName'));
    }
    if (command === 'readPath') {
      return this.readPath(this.requiredStringPayload(payload, 'path'));
    }
    if (command === 'writePath' || command === 'writeTransientPath') {
      return this.writePath(
        this.requiredStringPayload(payload, 'path'),
        this.stringPayload(payload, 'content'),
      );
    }
    if (command === 'writeBase64') {
      return this.writeBase64(
        this.requiredStringPayload(payload, 'path'),
        this.stringPayload(payload, 'contentBase64'),
      );
    }
    if (command === 'writeClipboard') {
      return this.writeClipboard(payload['payload']);
    }
    if (command === 'readClipboard') {
      return this.readClipboard();
    }
    if (command === 'setWindowTitle') {
      console.info(`[ChemSemaBridge] title: ${this.stringPayload(payload, 'title')}`);
      return true;
    }
    if (command === 'traceEvent') {
      console.info(`[ChemSemaBridge] trace ${JSON.stringify(payload)}`);
      return true;
    }
    throw new Error(`Unsupported Harmony bridge command: ${command}`);
  }

  private async chooseOpenPath(): Promise<HarmonyPathResult | null> {
    const options = new picker.DocumentSelectOptions();
    options.maxSelectNumber = 1;
    options.fileSuffixFilters = [
      'ChemSema|.ccjz,.ccjs',
      'ChemDraw|.cdxml,.cdx',
      'Molecule|.mol,.sdf',
      'All documents|.*',
    ];
    const uris = await new picker.DocumentViewPicker().select(options);
    const path = uris.length > 0 ? uris[0] : '';
    return path ? this.pathResult(path) : null;
  }

  private async chooseSavePath(suggestedName: string): Promise<HarmonyPathResult | null> {
    const options = new picker.DocumentSaveOptions();
    options.newFileNames = [suggestedName || 'chemsema-document.ccjz'];
    options.fileSuffixChoices = ['.ccjz', '.ccjs', '.cdxml', '.svg', '.pdf'];
    const uris = await new picker.DocumentViewPicker().save(options);
    const path = uris.length > 0 ? uris[0] : '';
    return path ? this.pathResult(path) : null;
  }

  private async readPath(path: string): Promise<HarmonyReadResult> {
    try {
      const text = await fileIo.readText(path);
      const fileName = this.fileNameFromUri(path);
      return {
        path,
        fileName,
        format: this.formatFromName(fileName),
        text,
      };
    } catch (error) {
      throw new Error(`Failed to read file: ${this.errorMessage(error)}`);
    }
  }

  private async writePath(path: string, content: string): Promise<HarmonyPathResult> {
    let file: fileIo.File | null = null;
    try {
      file = fileIo.openSync(
        path,
        fileIo.OpenMode.READ_WRITE | fileIo.OpenMode.CREATE | fileIo.OpenMode.TRUNC,
      );
      await fileIo.write(file.fd, content);
    } catch (error) {
      throw new Error(`Failed to write file: ${this.errorMessage(error)}`);
    } finally {
      this.closeFile(file);
    }
    return this.pathResult(path);
  }

  private async writeBase64(path: string, contentBase64: string): Promise<HarmonyPathResult> {
    let file: fileIo.File | null = null;
    try {
      const bytes = new util.Base64Helper().decodeSync(contentBase64);
      file = fileIo.openSync(
        path,
        fileIo.OpenMode.READ_WRITE | fileIo.OpenMode.CREATE | fileIo.OpenMode.TRUNC,
      );
      await fileIo.write(file.fd, bytes.buffer);
    } catch (error) {
      throw new Error(`Failed to write binary file: ${this.errorMessage(error)}`);
    } finally {
      this.closeFile(file);
    }
    return this.pathResult(path);
  }

  private async writeClipboard(payload: Object | undefined): Promise<boolean> {
    return true;
  }

  private async readClipboard(): Promise<Object | null> {
    return null;
  }

  private complete(id: string, response: HarmonyBridgeResponse): void {
    const responseJson = JSON.stringify(response);
    const script = `globalThis.__chemsemaHarmonyResolve && globalThis.__chemsemaHarmonyResolve(${JSON.stringify(id)}, ${JSON.stringify(responseJson)});`;
    this.controller.runJavaScript(script).catch((error: Error) => {
      console.error(`[ChemSemaBridge] callback failed: ${this.errorMessage(error)}`);
    });
  }

  private pathResult(path: string): HarmonyPathResult {
    return {
      path,
      fileName: this.fileNameFromUri(path),
    };
  }

  private fileNameFromUri(path: string): string {
    const decoded = decodeURIComponent(path || '');
    const segments = decoded.split(/[\\/]/).filter((segment) => segment.length > 0);
    const last = segments.length > 0 ? segments[segments.length - 1] : '';
    return last.split('?')[0].split('#')[0] || 'Untitled';
  }

  private formatFromName(fileName: string): string {
    const parts = fileName.toLowerCase().split('.');
    return parts.length > 1 ? parts[parts.length - 1] : '';
  }

  private closeFile(file: fileIo.File | null): void {
    if (!file) {
      return;
    }
    try {
      fileIo.closeSync(file);
    } catch (error) {
      console.error(`[ChemSemaBridge] failed to close file: ${this.errorMessage(error)}`);
    }
  }

  private stringPayload(payload: Record<string, Object>, key: string): string {
    const value = payload[key];
    return typeof value === 'string' ? value : '';
  }

  private requiredStringPayload(payload: Record<string, Object>, key: string): string {
    const value = this.stringPayload(payload, key);
    if (!value) {
      throw new Error(`Missing Harmony bridge payload: ${key}`);
    }
    return value;
  }

  private errorMessage(error: Object): string {
    if (error instanceof Error) {
      return error.message || String(error);
    }
    return String(error);
  }
}

@Entry
@Component
struct Index {
  private readonly localOrigin: string = 'https://chemsema.local';
  private controller: webview.WebviewController = new webview.WebviewController();
  private bridge: ChemSemaHarmonyBridge = new ChemSemaHarmonyBridge(this.controller);

  private responseForRawfile(url: string): WebResourceResponse | null {
    if (!url.startsWith(`${this.localOrigin}/`)) {
      return null;
    }

    let relativePath = url.substring(this.localOrigin.length + 1);
    const queryIndex = relativePath.indexOf('?');
    if (queryIndex >= 0) {
      relativePath = relativePath.substring(0, queryIndex);
    }
    const hashIndex = relativePath.indexOf('#');
    if (hashIndex >= 0) {
      relativePath = relativePath.substring(0, hashIndex);
    }
    if (relativePath.length === 0) {
      relativePath = 'index.html';
    }
    if (relativePath.startsWith('/') || relativePath.includes('..')) {
      return null;
    }

    const response = new WebResourceResponse();
    response.setResponseData($rawfile(`chemsema/${relativePath}`));
    if (!relativePath.endsWith('.wasm')) {
      response.setResponseEncoding('utf-8');
    }
    response.setResponseMimeType(this.mimeTypeForPath(relativePath));
    response.setResponseCode(200);
    response.setReasonMessage('OK');
    response.setResponseIsReady(true);
    return response;
  }

  private mimeTypeForPath(path: string): string {
    if (path.endsWith('.html')) {
      return 'text/html';
    }
    if (path.endsWith('.js') || path.endsWith('.mjs')) {
      return 'text/javascript';
    }
    if (path.endsWith('.css')) {
      return 'text/css';
    }
    if (path.endsWith('.wasm')) {
      return 'application/wasm';
    }
    if (path.endsWith('.svg')) {
      return 'image/svg+xml';
    }
    if (path.endsWith('.png')) {
      return 'image/png';
    }
    if (path.endsWith('.jpg') || path.endsWith('.jpeg')) {
      return 'image/jpeg';
    }
    if (path.endsWith('.json')) {
      return 'application/json';
    }
    return 'application/octet-stream';
  }

  build() {
    Column() {
      Web({
        src: `${this.localOrigin}/index.html`,
        controller: this.controller
      })
        .javaScriptAccess(true)
        .javaScriptProxy({
          object: this.bridge,
          name: 'chemsemaHarmony',
          methodList: ['postMessage'],
          controller: this.controller
        })
        .domStorageAccess(true)
        .fileAccess(true)
        .imageAccess(true)
        .onInterceptRequest((event) => {
          return this.responseForRawfile(event.request.getRequestUrl());
        })
        .onPageBegin((event) => {
          console.info(`[ChemSemaWeb] page begin: ${event.url}`);
        })
        .onPageEnd((event) => {
          console.info(`[ChemSemaWeb] page end: ${event.url}`);
        })
        .onErrorReceive((event) => {
          console.error(`[ChemSemaWeb] load error ${event.error.getErrorCode()}: ${event.error.getErrorInfo()} (${event.request.getRequestUrl()})`);
        })
        .onConsole((event) => {
          const message = event.message;
          console.info(`[ChemSemaWeb] console ${message.getMessageLevel()} ${message.getSourceId()}:${message.getLineNumber()} ${message.getMessage()}`);
          return false;
        })
        .onDetectedBlankScreen((result) => {
          console.error(`[ChemSemaWeb] blank screen detected: ${JSON.stringify(result)}`);
        })
        .onFirstScreenPaint(() => {
          console.info('[ChemSemaWeb] first screen painted');
        })
        .backgroundColor('#F7F8FA')
        .width('100%')
        .height('100%')
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#F7F8FA')
  }
}
