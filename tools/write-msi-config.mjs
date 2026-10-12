import fs from 'node:fs';
import path from 'node:path';
import { windowsReleaseConfig } from './msi-config.mjs';

const mainPath = path.resolve('src-tauri/tauri.conf.json');
const outputPath = path.resolve('src-tauri/tauri.release-windows.conf.json');

fs.rmSync(outputPath, { force: true });

const app = JSON.parse(fs.readFileSync(mainPath, 'utf8'));
const requested = process.argv[2] ?? app.version;
const normalized = requested.startsWith('v') ? requested.slice(1) : requested;
if (app.version !== normalized) {
  throw new Error('Tag/app version mismatch: ' + requested + ' != ' + app.version);
}

const overlay = windowsReleaseConfig(normalized);
const productVersion = overlay.bundle.windows.wix.version;
fs.writeFileSync(outputPath, JSON.stringify(overlay, null, 2) + '\n', { flag: 'wx' });
console.log('MSI mapping: ' + normalized + ' => ' + productVersion);
console.log('Temporary Tauri override: ' + outputPath);
