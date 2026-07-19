import { copyFileSync, existsSync } from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";

const defaultDevEcoHome = "D:\\Huawei\\DevEco Studio";
const devEcoHome = process.env.DEVECOSTUDIO_HOME || defaultDevEcoHome;
const sdkHome = process.env.DEVECO_SDK_HOME || path.join(devEcoHome, "sdk");
const openHarmonySdkHome = process.env.OHOS_BASE_SDK_HOME || path.join(sdkHome, "default", "openharmony");

const hvigorw = path.join(devEcoHome, "tools", "hvigor", "bin", "hvigorw.js");
const nodeBin = path.join(devEcoHome, "tools", "node");
const devEcoNode = path.join(nodeBin, "node.exe");
const corepackShims = path.join(nodeBin, "node_modules", "corepack", "shims");
const jbrBin = path.join(devEcoHome, "jbr", "bin");
const ohpmBin = path.join(devEcoHome, "tools", "ohpm", "bin");
const openHarmonyToolchains = path.join(openHarmonySdkHome, "toolchains");
const hmsToolchains = path.join(sdkHome, "default", "hms", "toolchains");
const harmonyProjectDir = path.join(process.cwd(), "apps", "chemsema-harmony");
const buildProfile = path.join(harmonyProjectDir, "build-profile.json5");
const exampleBuildProfile = path.join(harmonyProjectDir, "build-profile.example.json5");

for (const requiredPath of [hvigorw, devEcoNode, nodeBin, jbrBin, openHarmonySdkHome]) {
  if (!existsSync(requiredPath)) {
    console.error(`Missing DevEco path: ${requiredPath}`);
    process.exit(1);
  }
}

if (!existsSync(buildProfile)) {
  copyFileSync(exampleBuildProfile, buildProfile);
}

const env = {
  ...process.env,
  DEVECO_SDK_HOME: sdkHome,
  OHOS_BASE_SDK_HOME: openHarmonySdkHome,
  Path: [
    jbrBin,
    nodeBin,
    corepackShims,
    path.dirname(hvigorw),
    ohpmBin,
    openHarmonyToolchains,
    hmsToolchains,
    process.env.Path || process.env.PATH || "",
  ].join(path.delimiter),
};

const hvigorArgs = process.argv.slice(2);
const command = process.platform === "win32" ? devEcoNode : "node";
const args = [hvigorw, ...hvigorArgs];

const child = spawn(command, args, {
  cwd: harmonyProjectDir,
  env,
  shell: false,
  stdio: "inherit",
});

child.on("exit", (code, signal) => {
  if (signal) {
    console.error(`hvigorw exited with signal ${signal}`);
    process.exit(1);
  }
  process.exit(code ?? 1);
});
