import { readFile, writeFile, copyFile, readdir, mkdtemp, rm } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const cli = process.argv[2] ?? 'cargo-tauri';
const temporary = await mkdtemp(join(tmpdir(), 'maulink-rounded-icons-'));
try {
  for (const style of ['light', 'dark']) {
    const source = await readFile(join(root, `frontend/src/assets/maulink-logo-${style}.png`));
    const svg = join(temporary, `${style}.svg`);
    // Preserve the supplied artwork; add a transparent margin and rounded clipping path.
    await writeFile(svg, `<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="1024" height="1024" viewBox="0 0 1024 1024"><defs><clipPath id="rounded"><rect x="64" y="64" width="896" height="896" rx="200"/></clipPath></defs><image x="64" y="64" width="896" height="896" clip-path="url(#rounded)" xlink:href="data:image/png;base64,${source.toString('base64')}"/></svg>`);
    const output = join(temporary, style);
    const result = spawnSync(cli, ['icon', svg, '--output', output], { stdio: 'inherit' });
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`Icon generation failed for ${style}: ${result.status}`);
    if (style === 'light') {
      for (const name of await readdir(output)) {
        if (/\.(png|ico|icns)$/.test(name)) await copyFile(join(output, name), join(root, 'src-tauri/icons', name));
      }
    } else {
      await copyFile(join(output, '128x128@2x.png'), join(root, 'src-tauri/icons/app-icon-dark.png'));
    }
    await copyFile(join(output, '128x128@2x.png'), join(root, `frontend/src/assets/app-icon-${style}.png`));
  }
} finally {
  await rm(temporary, { recursive: true, force: true });
}
