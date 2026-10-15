import { createHash } from 'node:crypto';
import { readFile, readdir } from 'node:fs/promises';
import { basename, join, resolve } from 'node:path';

const matrix = JSON.parse(await readFile(new URL('./matrix-v33.json', import.meta.url), 'utf8'));
const baseCases = JSON.parse(await readFile(new URL('./cases.json', import.meta.url), 'utf8'));
const v33Cases = JSON.parse(await readFile(new URL('./cases-v33.json', import.meta.url), 'utf8'));
const allPages = [...baseCases, ...v33Cases].map(item => item.page);

function dimensions(bytes) {
  if (bytes.readUInt16BE(0) !== 0xffd8) throw new Error('Expected JPEG');
  for (let offset = 2; offset < bytes.length;) {
    if (bytes[offset++] !== 0xff) throw new Error('Invalid JPEG segment');
    const marker = bytes[offset++];
    const length = bytes.readUInt16BE(offset);
    if ([0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf].includes(marker)) {
      return { height: bytes.readUInt16BE(offset + 3), width: bytes.readUInt16BE(offset + 5) };
    }
    offset += length;
  }
  throw new Error('JPEG has no size frame');
}

async function verifyDirectory(root) {
  const seenFiles = new Set();
  let total = 0;
  const conditionResults = [];
  for (const condition of matrix) {
    const directory = join(root, condition.id);
    const pages = condition.pages ?? allPages;
    const manifest = `capture-${condition.theme}-${condition.locale}-${condition.width}x${condition.height}-${condition.id}.json`;
    const records = JSON.parse(await readFile(join(directory, manifest), 'utf8'));
    if (!Array.isArray(records) || records.length !== pages.length) throw new Error(`${condition.id}: expected ${pages.length} records, got ${records?.length}`);
    const names = await readdir(directory);
    const expectedFiles = new Set();
    const recordedPages = new Set();
    for (const record of records) {
      if (!record.page || !pages.includes(record.page) || recordedPages.has(record.page)) throw new Error(`${condition.id}: unexpected or duplicate page ${record.page}`);
      if (basename(record.file) !== record.file || !record.file.endsWith('.jpg')) throw new Error(`${condition.id}: invalid screenshot path ${record.file}`);
      if (record.themePreference !== condition.theme || record.locale !== condition.locale || record.width !== condition.width || record.height !== condition.height || record.density !== condition.uiDensity || record.accentColor !== condition.accentColor) throw new Error(`${condition.id}/${record.file}: manifest condition mismatch`);
      if (condition.theme !== 'system' && record.theme !== condition.theme) throw new Error(`${condition.id}/${record.file}: resolved theme mismatch`);
      if (condition.theme === 'system' && !['light', 'dark', 'system'].includes(record.theme)) throw new Error(`${condition.id}/${record.file}: invalid resolved system theme ${record.theme}`);
      if (record.customAccentColor !== (condition.accentColor === 'custom' ? condition.customAccentColor : null)) throw new Error(`${condition.id}/${record.file}: custom accent mismatch`);
      if (!record.identical || record.sha256 !== record.repeatSha256) throw new Error(`${condition.id}/${record.file}: independent reload hashes differ`);
      if (record.dpr !== 1 || record.fonts !== 'loaded' || record.overflow) throw new Error(`${condition.id}/${record.file}: invalid viewport/font/overflow condition`);
      const bytes = await readFile(join(directory, record.file));
      const sha256 = createHash('sha256').update(bytes).digest('hex');
      const size = dimensions(bytes);
      if (sha256 !== record.sha256 || size.width !== condition.width * record.dpr || size.height !== condition.height * record.dpr) throw new Error(`${condition.id}/${record.file}: hash or image dimensions mismatch`);
      if (seenFiles.has(`${condition.id}/${record.file}`)) throw new Error(`${condition.id}/${record.file}: duplicate screenshot`);
      seenFiles.add(`${condition.id}/${record.file}`);
      expectedFiles.add(record.file);
      recordedPages.add(record.page);
    }
    for (const page of pages) if (!recordedPages.has(page)) throw new Error(`${condition.id}: missing page ${page}`);
    const extraImages = names.filter(name => name.endsWith('.jpg') && !expectedFiles.has(name));
    if (extraImages.length) throw new Error(`${condition.id}: unreviewed candidate/repeat or unmanifested images: ${extraImages.join(', ')}`);
    total += records.length;
    conditionResults.push({ id: condition.id, screenshots: records.length });
  }
  return { root, conditions: conditionResults, screenshots: total };
}

export async function verifyV33(rootPaths) {
  if (!Array.isArray(rootPaths) || rootPaths.length < 1 || rootPaths.length > 2) throw new Error('Pass candidate root and optional baseline root');
  const results = [];
  for (const path of rootPaths) results.push(await verifyDirectory(resolve(path)));
  if (results.length === 2) {
    for (const condition of matrix) {
      const candidateDir = join(resolve(rootPaths[0]), condition.id);
      const baselineDir = join(resolve(rootPaths[1]), condition.id);
      const manifest = `capture-${condition.theme}-${condition.locale}-${condition.width}x${condition.height}-${condition.id}.json`;
      const candidate = JSON.parse(await readFile(join(candidateDir, manifest), 'utf8'));
      const baseline = JSON.parse(await readFile(join(baselineDir, manifest), 'utf8'));
      for (let index = 0; index < candidate.length; index++) {
        if (candidate[index].page !== baseline[index]?.page || candidate[index].sha256 !== baseline[index]?.sha256) throw new Error(`${condition.id}/${candidate[index].page}: baseline differs from reviewed candidate`);
      }
    }
  }
  return results;
}

if (process.argv[1] && resolve(process.argv[1]) === resolve(new URL(import.meta.url).pathname)) {
  const roots = process.argv.slice(2);
  if (!roots.length) throw new Error('Usage: node frontend/visual/verify-v33.mjs <candidate-root> [baseline-root]');
  console.log(JSON.stringify(await verifyV33(roots), null, 2));
}
