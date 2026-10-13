import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

export const DEFAULT_UPDATE_INDEX_BASE_URL = 'https://mauriceliang.github.io/MauLink/updates/';

function normalizeBaseUrl(value) {
  let url;
  try {
    url = new URL(value.endsWith('/') ? value : `${value}/`);
  } catch {
    throw new Error('Updater index base URL must be an absolute HTTPS URL');
  }
  if (url.protocol !== 'https:' || url.username || url.password || url.search || url.hash) {
    throw new Error('Updater index base URL must be HTTPS without credentials, query, or fragment');
  }
  return url;
}

export function createUpdaterReleaseConfig({ publicKey, indexBaseUrl = DEFAULT_UPDATE_INDEX_BASE_URL }) {
  if (typeof publicKey !== 'string' || !publicKey.trim()) {
    throw new Error('Missing MAULINK_UPDATER_PUBLIC_KEY');
  }

  const baseUrl = normalizeBaseUrl(indexBaseUrl.trim());
  const endpoints = ['stable.json', 'preview.json'].map((filename) => new URL(filename, baseUrl).toString());
  if (endpoints[0] === endpoints[1]) throw new Error('Stable and Preview updater endpoints must differ');

  return {
    bundle: { createUpdaterArtifacts: true },
    plugins: {
      updater: {
        pubkey: publicKey.trim(),
        endpoints,
      },
    },
  };
}

export function writeUpdaterReleaseConfig(outputPath, env = process.env) {
  const resolvedPath = path.resolve(outputPath);
  const config = createUpdaterReleaseConfig({
    publicKey: env.MAULINK_UPDATER_PUBLIC_KEY,
    indexBaseUrl: env.MAULINK_UPDATER_INDEX_BASE_URL || DEFAULT_UPDATE_INDEX_BASE_URL,
  });
  fs.writeFileSync(resolvedPath, `${JSON.stringify(config, null, 2)}\n`, { flag: 'wx' });
  return resolvedPath;
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const outputPath = process.argv[2];
  if (!outputPath) throw new Error('Usage: node tools/write-updater-config.mjs <output-json-path>');
  const writtenPath = writeUpdaterReleaseConfig(outputPath);
  console.log(`Temporary updater release config generated: ${writtenPath}`);
}
