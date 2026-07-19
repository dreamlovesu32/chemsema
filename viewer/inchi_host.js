export function createInchiHost() {
  let modulePromise = null;

  async function module() {
    if (!modulePromise) {
      modulePromise = loadInchiScript().then(() => {
        if (typeof globalThis.inchiModule1075 !== "function") {
          throw new Error("Official InChI WebAssembly factory is unavailable.");
        }
        return globalThis.inchiModule1075({
          locateFile: (path) => new URL("./inchi/" + path, import.meta.url).href,
        });
      });
    }
    return modulePromise;
  }

  async function analyzeMolfile(molfile) {
    const inchiModule = await module();
    const inchiPayload = callJson(
      inchiModule,
      "inchi_from_molfile",
      ["string", "string"],
      [molfile, ""],
    );
    if (!inchiPayload.inchi) {
      throw new Error(inchiPayload.message || inchiPayload.log || "InChI generation failed.");
    }
    const keyPayload = callJson(
      inchiModule,
      "inchikey_from_inchi",
      ["string"],
      [inchiPayload.inchi],
    );
    if (!keyPayload.inchikey) {
      throw new Error(keyPayload.message || "InChIKey generation failed.");
    }
    return {
      inchi: inchiPayload.inchi,
      inchikey: keyPayload.inchikey,
      auxiliaryInfo: inchiPayload.auxinfo || null,
      provider: "IUPAC InChI",
      providerVersion: "1.07.5",
    };
  }

  return { analyzeMolfile };
}

function callJson(module, functionName, argumentTypes, values) {
  const pointer = module.ccall(functionName, "number", argumentTypes, values);
  if (!pointer) {
    throw new Error("Official InChI WebAssembly returned no result.");
  }
  try {
    return JSON.parse(module.UTF8ToString(pointer));
  } finally {
    module._free(pointer);
  }
}

function loadInchiScript() {
  if (typeof globalThis.inchiModule1075 === "function") {
    return Promise.resolve();
  }
  return new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = new URL("./inchi/inchi-web-1075.js", import.meta.url).href;
    script.async = true;
    script.onload = resolve;
    script.onerror = () => reject(new Error("Could not load official InChI WebAssembly."));
    document.head.appendChild(script);
  });
}
