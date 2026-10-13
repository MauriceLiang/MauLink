import fs from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { channelForReleaseVersion, compareReleaseVersions } from './release-channel.mjs';

export const UPDATE_TARGETS = {
  'darwin-aarch64': (version) => `MauLink_${version}_aarch64-apple-darwin.app.tar.gz`,
  'darwin-x86_64': (version) => `MauLink_${version}_x86_64-apple-darwin.app.tar.gz`,
  'windows-x86_64': (version) => `MauLink_${version}_x86_64-pc-windows-msvc.msi`,
};

function parseHttpsUrl(value, description) {
  let url;
  try {
    url = new URL(value);
  } catch {
    throw new Error(`${description} must be an absolute HTTPS URL`);
  }
  if (url.protocol !== 'https:' || url.username || url.password || url.search || url.hash) {
    throw new Error(`${description} must be HTTPS without credentials, query, or fragment`);
  }
  return url;
}

function normalizeHttpsBaseUrl(value, description) {
  const url = parseHttpsUrl(value, description);
  if (!url.pathname.endsWith('/')) url.pathname += '/';
  return url;
}

function assertManifest(manifest, channel, description) {
  if (!manifest || typeof manifest !== 'object' || Array.isArray(manifest)) {
    throw new Error(`${description} must be a JSON object`);
  }
  if (typeof manifest.version !== 'string' || channelForReleaseVersion(manifest.version) !== channel) {
    throw new Error(`${description} version does not belong to the ${channel} channel`);
  }
  if (!manifest.platforms || typeof manifest.platforms !== 'object' || Array.isArray(manifest.platforms)) {
    throw new Error(`${description} has no platform map`);
  }
  const actualTargets = Object.keys(manifest.platforms).sort();
  const expectedTargets = Object.keys(UPDATE_TARGETS).sort();
  if (JSON.stringify(actualTargets) !== JSON.stringify(expectedTargets)) {
    throw new Error(`${description} must contain exactly the supported macOS and Windows targets`);
  }
  for (const [target, platform] of Object.entries(manifest.platforms)) {
    if (!platform || typeof platform !== 'object') throw new Error(`${description} has invalid ${target} data`);
    if (typeof platform.signature !== 'string' || !platform.signature.trim()) {
      throw new Error(`${description} has no ${target} signature`);
    }
    let url;
    try {
      url = new URL(platform.url);
    } catch {
      throw new Error(`${description} has an invalid ${target} URL`);
    }
    if (url.protocol !== 'https:') throw new Error(`${description} ${target} URL must use HTTPS`);
  }
}

export function buildUpdateManifest({ version, tag, releaseUrlBase, signatures, publishedAt }) {
  const channel = channelForReleaseVersion(version);
  if (tag !== `v${version}`) throw new Error(`Release tag/version mismatch: ${tag} != v${version}`);
  const releaseBase = normalizeHttpsBaseUrl(releaseUrlBase, 'Release asset base URL');

  const platforms = Object.fromEntries(Object.entries(UPDATE_TARGETS).map(([target, filenameFor]) => {
    const filename = filenameFor(version);
    const signature = signatures[target];
    if (typeof signature !== 'string' || !signature.trim()) throw new Error(`Missing signature for ${target}`);
    return [target, {
      url: new URL(encodeURIComponent(filename), releaseBase).toString(),
      signature: signature.trim(),
    }];
  }));
  const manifest = { version, platforms };
  if (publishedAt) manifest.pub_date = new Date(publishedAt).toISOString();
  assertManifest(manifest, channel, 'Generated update index');
  return { channel, manifest };
}

export function assertIndexCanAdvance(existing, candidate, channel) {
  assertManifest(candidate, channel, 'Candidate update index');
  if (!existing) return;
  assertManifest(existing, channel, 'Existing update index');
  const order = compareReleaseVersions(candidate.version, existing.version);
  if (order < 0) {
    throw new Error(`Refusing to move ${channel} index backwards from ${existing.version} to ${candidate.version}`);
  }
  if (order === 0 && JSON.stringify(candidate.platforms) !== JSON.stringify(existing.platforms)) {
    throw new Error(`${channel} release ${candidate.version} already has different signed assets`);
  }
}

async function readExistingIndex(endpoint) {
  const url = parseHttpsUrl(endpoint, 'Update index endpoint');
  url.searchParams.set('_cache_bust', Date.now().toString());
  const response = await fetch(url, { headers: { accept: 'application/json' } });
  if (response.status === 404 || response.status === 410) return null;
  if (!response.ok) throw new Error(`Could not read existing update index (${response.status})`);
  try {
    return await response.json();
  } catch {
    throw new Error('Existing update index is not valid JSON');
  }
}

async function readSignature(assetDir, filename) {
  const value = (await fs.readFile(path.join(assetDir, `${filename}.sig`), 'utf8')).trim();
  if (!value) throw new Error(`Empty signature: ${filename}.sig`);
  return value;
}

export async function writeUpdateSite({ version, tag, assetDir, outputDir, indexBaseUrl, repository }) {
  const baseUrl = normalizeHttpsBaseUrl(indexBaseUrl, 'Update index base URL');
  const channel = channelForReleaseVersion(version);
  const existing = {};
  for (const targetChannel of ['stable', 'preview']) {
    existing[targetChannel] = await readExistingIndex(new URL(`${targetChannel}.json`, baseUrl).toString());
  }

  const signatures = {};
  for (const [target, filenameFor] of Object.entries(UPDATE_TARGETS)) {
    const filename = filenameFor(version);
    const artifactPath = path.join(assetDir, filename);
    const stat = await fs.stat(artifactPath);
    if (!stat.isFile() || stat.size === 0) throw new Error(`Missing or empty updater artifact: ${filename}`);
    signatures[target] = await readSignature(assetDir, filename);
  }

  const { manifest } = buildUpdateManifest({
    version,
    tag,
    releaseUrlBase: `https://github.com/${repository}/releases/download/${encodeURIComponent(tag)}/`,
    signatures,
    publishedAt: new Date().toISOString(),
  });
  assertIndexCanAdvance(existing[channel], manifest, channel);
  if (existing[channel] && existing[channel].version === version) {
    manifest.pub_date = existing[channel].pub_date;
  }

  await fs.mkdir(outputDir, { recursive: true });
  for (const targetChannel of ['stable', 'preview']) {
    const value = targetChannel === channel ? manifest : existing[targetChannel];
    if (!value) continue;
    assertManifest(value, targetChannel, `Published ${targetChannel} index`);
    await fs.writeFile(path.join(outputDir, `${targetChannel}.json`), `${JSON.stringify(value, null, 2)}\n`);
  }
  return { channel, manifest };
}

export async function verifyUpdateSiteReadback({ indexDir, indexBaseUrl, fetcher = fetch, attempts = 6, delayMs = 3000 }) {
  const baseUrl = normalizeHttpsBaseUrl(indexBaseUrl, 'Update index base URL');
  const files = (await fs.readdir(indexDir)).filter((filename) => filename.endsWith('.json')).sort();
  if (files.length === 0) throw new Error('No update indexes were generated for readback');

  for (const filename of files) {
    const expected = await fs.readFile(path.join(indexDir, filename));
    let verified = false;
    let lastStatus = 'no response';
    for (let attempt = 0; attempt < attempts; attempt += 1) {
      const url = new URL(filename, baseUrl);
      url.searchParams.set('_readback', `${Date.now()}-${attempt}`);
      try {
        const response = await fetcher(url, { headers: { accept: 'application/json', 'cache-control': 'no-cache' } });
        if (!response.ok) {
          lastStatus = `HTTP ${response.status}`;
        } else {
          const actual = Buffer.from(await response.arrayBuffer());
          if (actual.equals(expected)) {
            verified = true;
            break;
          }
          lastStatus = 'response bytes differ from the deployed index';
        }
      } catch (error) {
        lastStatus = error instanceof Error ? error.message : 'request failed';
      }
      if (attempt + 1 < attempts && delayMs > 0) {
        await new Promise((resolve) => setTimeout(resolve, delayMs));
      }
    }
    if (!verified) throw new Error(`HTTPS readback failed for ${filename}: ${lastStatus}`);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const [version, tag, assetDir, outputDir] = process.argv.slice(2);
  const indexBaseUrl = process.env.MAULINK_UPDATER_INDEX_BASE_URL || 'https://mauriceliang.github.io/MauLink/updates/';
  const repository = process.env.GITHUB_REPOSITORY || 'MauriceLiang/MauLink';
  if (!version || !tag || !assetDir || !outputDir) {
    throw new Error('Usage: node tools/build-update-index.mjs <version> <tag> <release-assets-dir> <output-dir>');
  }
  const result = await writeUpdateSite({ version, tag, assetDir, outputDir, indexBaseUrl, repository });
  console.log(`Prepared ${result.channel} update index for ${version}`);
}
