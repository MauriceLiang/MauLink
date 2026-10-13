import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { verifyUpdateSiteReadback } from './build-update-index.mjs';

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const [indexDir, indexBaseUrl] = process.argv.slice(2);
  if (!indexDir || !indexBaseUrl) {
    throw new Error('Usage: node tools/verify-update-index.mjs <index-directory> <https-index-base-url>');
  }
  await verifyUpdateSiteReadback({ indexDir, indexBaseUrl });
  console.log('Deployed update index bytes verified over HTTPS');
}
