import { chromium } from "playwright";

const defaultChannel = process.platform === "win32"
  ? (process.env.CHEMSEMA_PLAYWRIGHT_CHANNEL || "msedge")
  : process.env.CHEMSEMA_PLAYWRIGHT_CHANNEL;

const defaultExecutablePath = process.env.CHEMSEMA_PLAYWRIGHT_EXECUTABLE_PATH;

export function launchBrowser(options = {}) {
  const launchOptions = { ...options };
  if (defaultExecutablePath && launchOptions.executablePath === undefined) {
    launchOptions.executablePath = defaultExecutablePath;
  }
  if (
    defaultChannel
    && !launchOptions.executablePath
    && launchOptions.channel === undefined
  ) {
    launchOptions.channel = defaultChannel;
  }
  return chromium.launch(launchOptions);
}
