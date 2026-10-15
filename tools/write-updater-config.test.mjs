import assert from 'node:assert/strict';
import test from 'node:test';
import {
  createUpdaterReleaseConfig,
  DEFAULT_UPDATE_INDEX_BASE_URL,
} from './write-updater-config.mjs';

test('creates signed updater config with separate GitHub Pages channel indexes', () => {
  const config = createUpdaterReleaseConfig({ publicKey: ' public-key ' });
  assert.equal(config.bundle.createUpdaterArtifacts, true);
  assert.equal(config.plugins.updater.pubkey, 'public-key');
  assert.deepEqual(config.plugins.updater.endpoints, [
    new URL('stable.json', DEFAULT_UPDATE_INDEX_BASE_URL).toString(),
    new URL('preview.json', DEFAULT_UPDATE_INDEX_BASE_URL).toString(),
  ]);
});

test('allows moving both channels to a future HTTPS update host', () => {
  const config = createUpdaterReleaseConfig({
    publicKey: 'public-key',
    indexBaseUrl: 'https://updates.maulink.example/releases',
  });
  assert.deepEqual(config.plugins.updater.endpoints, [
    'https://updates.maulink.example/releases/stable.json',
    'https://updates.maulink.example/releases/preview.json',
  ]);
});

test('rejects missing keys and insecure or malformed index bases', () => {
  assert.throws(() => createUpdaterReleaseConfig({ publicKey: '' }), /MAULINK_UPDATER_PUBLIC_KEY/);
  for (const indexBaseUrl of [
    'http://updates.test/',
    'https://user:password@updates.test/',
    'https://updates.test/?channel=stable',
    'not a URL',
  ]) {
    assert.throws(
      () => createUpdaterReleaseConfig({ publicKey: 'public-key', indexBaseUrl }),
      /HTTPS|absolute/,
      indexBaseUrl,
    );
  }
});
