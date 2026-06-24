import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appDir = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(appDir, "..", "..");

const configPath = process.env.SYNC_IMAGE_BUILD_CONFIG
  ? path.resolve(process.env.SYNC_IMAGE_BUILD_CONFIG)
  : path.join(repoRoot, "examples", "config.toml");

const rawConfig = await readFile(configPath, "utf8");
const appTitle = readTopLevelString(rawConfig, "app_title")?.trim();

if (!appTitle) {
  throw new Error(`app_title is required in ${configPath}`);
}

const tauriConfigPath = path.join(appDir, "src-tauri", "tauri.conf.json");
const tauriConfig = JSON.parse(await readFile(tauriConfigPath, "utf8"));
tauriConfig.productName = appTitle;

for (const windowConfig of tauriConfig.app?.windows ?? []) {
  windowConfig.title = appTitle;
}

await writeFile(
  tauriConfigPath,
  `${JSON.stringify(tauriConfig, null, 2)}\n`,
  "utf8",
);

const indexPath = path.join(appDir, "index.html");
const indexHtml = await readFile(indexPath, "utf8");
const updatedIndexHtml = indexHtml.replace(
  /<title>.*?<\/title>/s,
  `<title>${escapeHtml(appTitle)}</title>`,
);

if (updatedIndexHtml === indexHtml && !indexHtml.includes(`<title>${appTitle}</title>`)) {
  throw new Error(`failed to update title in ${indexPath}`);
}

await writeFile(indexPath, updatedIndexHtml, "utf8");

function readTopLevelString(raw, key) {
  const pattern = new RegExp(`^${escapeRegExp(key)}\\s*=\\s*"((?:\\\\.|[^"\\\\])*)"\\s*$`, "m");
  const match = raw.match(pattern);

  if (!match) {
    return undefined;
  }

  return match[1].replace(/\\"/g, '"').replace(/\\\\/g, "\\");
}

function escapeHtml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
