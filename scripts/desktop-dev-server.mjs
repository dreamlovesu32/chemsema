import { createServer } from "node:http";
import { createReadStream, existsSync, statSync } from "node:fs";
import { extname, join, normalize, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const host = "127.0.0.1";
const port = Number(process.env.CHEMCORE_DESKTOP_DEV_PORT || 8767);

const contentTypes = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".ico": "image/x-icon",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".svg": "image/svg+xml; charset=utf-8",
  ".wasm": "application/wasm",
};

function filePathForUrl(url) {
  const parsedUrl = new URL(url, `http://${host}:${port}`);
  const decodedPath = decodeURIComponent(parsedUrl.pathname);
  const relativePath = normalize(decodedPath.replace(/^\/+/, ""));
  const candidate = resolve(rootDir, relativePath || "viewer/index.html");
  if (candidate !== rootDir && !candidate.startsWith(`${rootDir}${sep}`)) {
    return null;
  }
  if (existsSync(candidate) && statSync(candidate).isDirectory()) {
    return join(candidate, "index.html");
  }
  return candidate;
}

const server = createServer((request, response) => {
  const filePath = filePathForUrl(request.url || "/");
  if (!filePath || !existsSync(filePath) || !statSync(filePath).isFile()) {
    response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
    response.end("Not found");
    return;
  }

  const contentType = contentTypes[extname(filePath).toLowerCase()] || "application/octet-stream";
  response.writeHead(200, { "content-type": contentType });
  createReadStream(filePath).pipe(response);
});

server.on("error", (error) => {
  if (error.code === "EADDRINUSE") {
    console.log(`[desktop:dev] http://${host}:${port}/ is already in use; reusing it for Tauri dev.`);
    setInterval(() => {}, 2 ** 30);
    return;
  }
  throw error;
});

server.listen(port, host, () => {
  console.log(`[desktop:dev] serving ${rootDir} at http://${host}:${port}/`);
});
