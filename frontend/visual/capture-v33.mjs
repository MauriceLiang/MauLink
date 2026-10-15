import { mkdir, readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { captureCases, visualCasesV33 } from './capture.mjs';

const matrix = JSON.parse(await readFile(new URL('./matrix-v33.json', import.meta.url), 'utf8'));

export async function captureMatrixV33({ tab, viewport, outputRoot, baseUrl, conditions = matrix }) {
  const root = resolve(outputRoot);
  await mkdir(root, { recursive: true });
  const results = [];
  for (const condition of conditions) {
    const { id, ...options } = condition;
    if (!id) throw new Error('Every V3.3 matrix condition needs an id');
    const outputDir = join(root, id);
    const records = await captureCases({
      tab, viewport, outputDir, ...options, manifestId: id,
      caseSet: visualCasesV33, update: true, resume: true, baseUrl,
    });
    results.push({ id, count: records.length, pages: records.map(record => record.page) });
  }
  return results;
}

export { matrix as matrixV33 };
