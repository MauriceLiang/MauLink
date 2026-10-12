import fs from 'node:fs';
import { toMsiProductVersion } from './msi-version.mjs';

const tag = process.argv[2] ?? '';
const numeric = '(?:0|[1-9]\\d*)';
const prereleaseIdentifier = '(?:0|[1-9]\\d*|\\d*[A-Za-z-][0-9A-Za-z-]*)';
const semver = new RegExp(
  `^v${numeric}\\.${numeric}\\.${numeric}(?:-${prereleaseIdentifier}(?:\\.${prereleaseIdentifier})*)?$`,
);

function fail(message) {
  console.error(message);
  process.exitCode = 1;
}

if (!semver.test(tag)) {
  fail(`Invalid release tag: ${tag || '<missing>'}`);
} else {
  const expected = tag.slice(1);
  const tauri = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8'));
  const frontend = JSON.parse(fs.readFileSync('frontend/package.json', 'utf8'));
  const lock = JSON.parse(fs.readFileSync('frontend/package-lock.json', 'utf8'));
  const workspace = fs.readFileSync('Cargo.toml', 'utf8');
  const section = workspace.match(/^\[workspace\.package\]([\s\S]*?)(?=^\[|$(?![\s\S]))/m)?.[1];
  const rustVersion = section?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  const lockVersion = lock.packages?.['']?.version ?? lock.version;
  const checks = {
    'Rust workspace version': rustVersion,
    'Tauri app version': tauri.version,
    'Frontend version': frontend.version,
    'npm lock root version': lockVersion,
  };

  let hasMismatch = false;
  for (const [name, actual] of Object.entries(checks)) {
    if (actual !== expected) {
      console.error(`${name}: expected ${expected}; found ${actual ?? '<missing>'}`);
      hasMismatch = true;
    }
  }

  if (tauri.identifier !== 'io.maulink.desktop') {
    console.error(`Unexpected app identifier: ${tauri.identifier ?? '<missing>'}`);
    hasMismatch = true;
  }

  try {
    console.log('Windows MSI ProductVersion: ' + toMsiProductVersion(tag));
  } catch (error) {
    console.error('MSI version validation failed: ' + error.message);
    hasMismatch = true;
  }

  if (hasMismatch) {
    process.exitCode = 1;
  } else {
    console.log(`Release version checks passed: ${tag}`);
  }
}
