import { readFile, readdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { basename, resolve } from 'node:path';

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

const root = resolve(process.argv[2] ?? 'docs/refactor/screenshots/color-v1-review');
const names = await readdir(root);
const manifests = names.filter(name => /^capture-(light|dark)-(zh-CN|en)-\d+x\d+.*\.json$/.test(name));
if (manifests.length !== 5) throw new Error(`Expected 5 condition manifests, got ${manifests.length}`);

const expectedConditions = new Map([
  ['dark-en-860x640', 6],
  ['dark-zh-CN-1440x920', 23],
  ['light-en-860x640', 6],
  ['light-zh-CN-1440x920', 23],
]);
const files = new Set();
const pagesByCondition = new Map();
for (const name of manifests) {
  const [, theme, locale, viewport] = name.match(/^capture-(light|dark)-(zh-CN|en)-(\d+x\d+)/);
  const condition = `${theme}-${locale}-${viewport}`;
  if (!expectedConditions.has(condition)) throw new Error(`${name}: unexpected capture condition`);
  const records = JSON.parse(await readFile(`${root}/${name}`, 'utf8'));
  if (!Array.isArray(records) || records.length === 0) throw new Error(`${name}: empty or invalid manifest`);
  const pageSet = pagesByCondition.get(condition) ?? new Set();
  for (const record of records) {
    if (basename(record.file) !== record.file || !record.file.endsWith('.jpg')) throw new Error(`${name}: invalid screenshot path`);
    if (record.theme !== theme || record.locale !== locale || `${record.width}x${record.height}` !== viewport) throw new Error(`${record.file}: manifest condition mismatch`);
    if (!record.page || pageSet.has(record.page)) throw new Error(`${record.file}: duplicate or missing page record`);
    const bytes = await readFile(`${root}/${record.file}`);
    const sha256 = createHash('sha256').update(bytes).digest('hex');
    const size = dimensions(bytes);
    if (!record.identical || sha256 !== record.sha256 || sha256 !== record.repeatSha256) throw new Error(`${record.file}: invalid capture hash`);
    if (record.dpr !== 1 || record.fonts !== 'loaded' || record.overflow) throw new Error(`${record.file}: invalid capture conditions`);
    if (size.width !== record.width * record.dpr || size.height !== record.height * record.dpr) throw new Error(`${record.file}: invalid JPEG dimensions`);
    if (files.has(record.file)) throw new Error(`${record.file}: duplicated across manifests`);
    files.add(record.file);
    pageSet.add(record.page);
  }
  pagesByCondition.set(condition, pageSet);
}

for (const [condition, expectedCount] of expectedConditions) {
  const actualCount = pagesByCondition.get(condition)?.size ?? 0;
  if (actualCount !== expectedCount) throw new Error(`${condition}: expected ${expectedCount} pages, got ${actualCount}`);
}
if (files.size !== 58) throw new Error(`Expected 58 unique baselines, got ${files.size}`);
const extras = names.filter(name => name.endsWith('.jpg') && !files.has(name));
if (extras.length) throw new Error(`Unmanifested or unreviewed screenshots: ${extras.join(', ')}`);

console.log(JSON.stringify({ manifests: manifests.length, screenshots: files.size, dpr: 1, independentRepeat: 'byte-identical' }, null, 2));
