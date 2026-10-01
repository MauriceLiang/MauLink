import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { saveCapture } from './capture.mjs';
test('preserves baseline on changed, missing or non-repeatable captures; updates only explicitly', async () => {
  const directory=await mkdtemp(join(tmpdir(),'maulink-visual-')); const original=Buffer.from('original'),changed=Buffer.from('changed');
  try {
    await assert.rejects(saveCapture(directory,'missing.jpg',[original,original]),/baseline missing/);
    await assert.rejects(readFile(join(directory,'missing.jpg')),error=>error.code==='ENOENT');
    await writeFile(join(directory,'page.jpg'),original);
    await assert.rejects(saveCapture(directory,'page.jpg',[changed,changed]),/baseline changed/);
    assert.deepEqual(await readFile(join(directory,'page.jpg')),original);
    await assert.rejects(saveCapture(directory,'page.jpg',[original,changed],true),/independent reload screenshots differ/);
    assert.deepEqual(await readFile(join(directory,'page.jpg')),original);
    assert.equal((await saveCapture(directory,'page.jpg',[original,original])).identical,true);
    await saveCapture(directory,'page.jpg',[changed,changed],true);
    assert.deepEqual(await readFile(join(directory,'page.jpg')),changed);
  } finally { await rm(directory,{recursive:true,force:true}); }
});
