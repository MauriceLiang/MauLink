import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import {
  assertIndexCanAdvance,
  buildUpdateManifest,
  UPDATE_TARGETS,
  verifyUpdateSiteReadback,
  writeUpdateSite,
} from './build-update-index.mjs';

function manifest(version) {
  const signatures = Object.fromEntries(Object.keys(UPDATE_TARGETS).map((target) => [target, `signed-${version}-${target}`]));
  return buildUpdateManifest({
    version,
    tag: `v${version}`,
    releaseUrlBase: `https://github.com/MauriceLiang/MauLink/releases/download/v${version}/`,
    signatures,
    publishedAt: '2026-10-12T12:00:00Z',
  }).manifest;
}

test('builds a three-platform signed index for the matching release channel and tag', () => {
  const { channel, manifest: candidate } = buildUpdateManifest({
    version: '0.1.2-rc.10',
    tag: 'v0.1.2-rc.10',
    releaseUrlBase: 'https://github.com/MauriceLiang/MauLink/releases/download/v0.1.2-rc.10/',
    signatures: Object.fromEntries(Object.keys(UPDATE_TARGETS).map((target) => [target, `signed-${target}`])),
    publishedAt: '2026-10-12T12:00:00Z',
  });
  assert.equal(channel, 'preview');
  assert.deepEqual(Object.keys(candidate.platforms).sort(), Object.keys(UPDATE_TARGETS).sort());
  assert.equal(candidate.platforms['darwin-aarch64'].url,
    'https://github.com/MauriceLiang/MauLink/releases/download/v0.1.2-rc.10/MauLink_0.1.2-rc.10_aarch64-apple-darwin.app.tar.gz');
  assert.equal(candidate.platforms['windows-x86_64'].signature, 'signed-windows-x86_64');
  assert.equal(candidate.pub_date, '2026-10-12T12:00:00.000Z');
});

test('requires an exact version tag, HTTPS assets, and signatures for every target', () => {
  assert.throws(() => buildUpdateManifest({
    version: '0.1.2', tag: 'v0.1.1', releaseUrlBase: 'https://github.com/x/y/', signatures: {},
  }), /tag\/version mismatch/);
  assert.throws(() => buildUpdateManifest({
    version: '0.1.2', tag: 'v0.1.2', releaseUrlBase: 'http://github.com/x/y/',
    signatures: Object.fromEntries(Object.keys(UPDATE_TARGETS).map((target) => [target, 'signature'])),
  }), /HTTPS/);
  for (const releaseUrlBase of [
    'https://user:password@github.com/x/y/',
    'https://github.com/x/y/?token=secret',
    'https://github.com/x/y/#fragment',
  ]) {
    assert.throws(() => buildUpdateManifest({
      version: '0.1.2', tag: 'v0.1.2', releaseUrlBase,
      signatures: Object.fromEntries(Object.keys(UPDATE_TARGETS).map((target) => [target, 'signature'])),
    }), /credentials, query, or fragment/);
  }
  assert.throws(() => buildUpdateManifest({
    version: '0.1.2', tag: 'v0.1.2', releaseUrlBase: 'https://github.com/x/y/', signatures: {},
  }), /Missing signature/);
});

test('advances a channel by SemVer and rejects stale or mutated same-version indexes', () => {
  assertIndexCanAdvance(null, manifest('0.1.2-rc.1'), 'preview');
  assertIndexCanAdvance(manifest('0.1.2-rc.9'), manifest('0.1.2-rc.10'), 'preview');
  assert.throws(() => assertIndexCanAdvance(manifest('0.1.2-rc.10'), manifest('0.1.2-rc.9'), 'preview'), /backwards/);
  const changedSameVersion = manifest('0.1.2-rc.10');
  changedSameVersion.platforms['darwin-aarch64'].signature = 'different-signature';
  assert.throws(() => assertIndexCanAdvance(manifest('0.1.2-rc.10'), changedSameVersion, 'preview'), /different signed assets/);
  assert.throws(() => assertIndexCanAdvance(manifest('0.1.2'), manifest('0.1.2-rc.10'), 'stable'), /stable channel/);
});

test('keeps the other channel index intact when building a release site', async () => {
  const previousFetch = globalThis.fetch;
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'maulink-update-index-'));
  const assetDir = path.join(root, 'assets');
  const outputDir = path.join(root, 'site', 'updates');
  const stableIndex = manifest('0.1.1');
  try {
    await fs.mkdir(assetDir);
    for (const filenameFor of Object.values(UPDATE_TARGETS)) {
      const filename = filenameFor('0.1.2-rc.10');
      await fs.writeFile(path.join(assetDir, filename), 'artifact');
      await fs.writeFile(path.join(assetDir, `${filename}.sig`), `signed-${filename}`);
    }
    globalThis.fetch = async (input) => {
      const url = new URL(input);
      if (url.pathname.endsWith('/stable.json')) {
        return { status: 200, ok: true, json: async () => stableIndex };
      }
      return { status: 404, ok: false };
    };
    const result = await writeUpdateSite({
      version: '0.1.2-rc.10',
      tag: 'v0.1.2-rc.10',
      assetDir,
      outputDir,
      indexBaseUrl: 'https://mauriceliang.github.io/MauLink/updates/',
      repository: 'MauriceLiang/MauLink',
    });
    assert.equal(result.channel, 'preview');
    assert.deepEqual(JSON.parse(await fs.readFile(path.join(outputDir, 'stable.json'), 'utf8')), stableIndex);
    const previewIndex = JSON.parse(await fs.readFile(path.join(outputDir, 'preview.json'), 'utf8'));
    assert.equal(previewIndex.version, '0.1.2-rc.10');
    assert.equal(previewIndex.platforms['darwin-aarch64'].signature, 'signed-MauLink_0.1.2-rc.10_aarch64-apple-darwin.app.tar.gz');
  } finally {
    globalThis.fetch = previousFetch;
    await fs.rm(root, { recursive: true, force: true });
  }
});

test('does not write over a newer published channel index', async () => {
  const previousFetch = globalThis.fetch;
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'maulink-update-index-'));
  const assetDir = path.join(root, 'assets');
  const outputDir = path.join(root, 'site', 'updates');
  try {
    await fs.mkdir(assetDir);
    for (const filenameFor of Object.values(UPDATE_TARGETS)) {
      const filename = filenameFor('0.1.2-rc.9');
      await fs.writeFile(path.join(assetDir, filename), 'artifact');
      await fs.writeFile(path.join(assetDir, `${filename}.sig`), `signed-${filename}`);
    }
    globalThis.fetch = async (input) => new URL(input).pathname.endsWith('/preview.json')
      ? { status: 200, ok: true, json: async () => manifest('0.1.2-rc.10') }
      : { status: 404, ok: false };
    await assert.rejects(writeUpdateSite({
      version: '0.1.2-rc.9',
      tag: 'v0.1.2-rc.9',
      assetDir,
      outputDir,
      indexBaseUrl: 'https://mauriceliang.github.io/MauLink/updates/',
      repository: 'MauriceLiang/MauLink',
    }), /move preview index backwards/);
    await assert.rejects(fs.stat(outputDir));
  } finally {
    globalThis.fetch = previousFetch;
    await fs.rm(root, { recursive: true, force: true });
  }
});

test('verifies the published index bytes over HTTPS', async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'maulink-update-readback-'));
  const indexPath = path.join(root, 'preview.json');
  const expected = Buffer.from('{"version":"0.1.2-rc.10"}\n');
  try {
    await fs.writeFile(indexPath, expected);
    await verifyUpdateSiteReadback({
      indexDir: root,
      indexBaseUrl: 'https://updates.example.test/updates/',
      attempts: 1,
      fetcher: async (url) => {
        assert.equal(new URL(url).pathname, '/updates/preview.json');
        return new Response(expected, { status: 200 });
      },
    });
    await assert.rejects(verifyUpdateSiteReadback({
      indexDir: root,
      indexBaseUrl: 'https://updates.example.test/updates/',
      attempts: 1,
      fetcher: async () => new Response('{"version":"old"}', { status: 200 }),
    }), /response bytes differ/);
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});

test('rejects updater index URLs with credentials, query, or fragment', async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'maulink-update-url-'));
  try {
    for (const indexBaseUrl of [
      'http://updates.example.test/updates/',
      'https://user:password@updates.example.test/updates/',
      'https://updates.example.test/updates/?token=secret',
      'https://updates.example.test/updates/#fragment',
    ]) {
      await assert.rejects(verifyUpdateSiteReadback({
        indexDir: root,
        indexBaseUrl,
        fetcher: async () => { throw new Error('fetch should not run'); },
      }), /HTTPS|credentials, query, or fragment/);
    }
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
});
