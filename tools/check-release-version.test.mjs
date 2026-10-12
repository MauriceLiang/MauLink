import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const script = fileURLToPath(new URL('./check-release-version.mjs', import.meta.url));

function runChecker({ tag, version = '0.1.0', frontendVersion = version, lockVersion = version, identifier = 'io.maulink.desktop' }) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'maulink-release-version-'));
  fs.mkdirSync(path.join(directory, 'src-tauri'), { recursive: true });
  fs.mkdirSync(path.join(directory, 'frontend'), { recursive: true });
  fs.writeFileSync(path.join(directory, 'Cargo.toml'), `[workspace.package]\nversion = "${version}"\n`);
  fs.writeFileSync(
    path.join(directory, 'src-tauri/tauri.conf.json'),
    JSON.stringify({ version, identifier }),
  );
  fs.writeFileSync(path.join(directory, 'frontend/package.json'), JSON.stringify({ version: frontendVersion }));
  fs.writeFileSync(
    path.join(directory, 'frontend/package-lock.json'),
    JSON.stringify({ packages: { '': { version: lockVersion } } }),
  );

  try {
    return spawnSync(process.execPath, [script, tag], { cwd: directory, encoding: 'utf8' });
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
}

test('accepts a stable release tag when all manifests match', () => {
  const result = runChecker({ tag: 'v0.1.0' });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /Release version checks passed: v0\.1\.0/);
});

test('accepts a release candidate tag when all manifests match', () => {
  const result = runChecker({ tag: 'v0.1.0-rc.1', version: '0.1.0-rc.1' });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /Windows MSI ProductVersion: 0\.1\.1/);
});

test('rejects a SemVer prerelease unsupported by the MSI mapping', () => {
  const version = '0.1.1-beta.1';
  const result = runChecker({
    tag: 'v' + version,
    version,
    frontendVersion: version,
    lockVersion: version,
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /MSI version validation failed: Unsupported MauLink MSI release version/);
});

test('rejects invalid release tag syntax', () => {
  const result = runChecker({ tag: 'v0.1' });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Invalid release tag/);
});

test('rejects a tag whose version differs from the manifests', () => {
  const result = runChecker({ tag: 'v0.1.1' });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Rust workspace version: expected 0\.1\.1; found 0\.1\.0/);
});

test('rejects a frontend version mismatch', () => {
  const result = runChecker({ tag: 'v0.1.0', frontendVersion: '0.2.0' });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Frontend version: expected 0\.1\.0; found 0\.2\.0/);
});

test('rejects an npm lock root version mismatch', () => {
  const result = runChecker({ tag: 'v0.1.0', lockVersion: '0.2.0' });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /npm lock root version: expected 0\.1\.0; found 0\.2\.0/);
});

test('rejects an unexpected Tauri application identifier', () => {
  const result = runChecker({ tag: 'v0.1.0', identifier: 'io.example.test' });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Unexpected app identifier: io\.example\.test/);
});
