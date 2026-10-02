// Execute inside cua_repl with its documented Tab and viewport capability; no standalone browser driver.
import { mkdir, readFile, writeFile, unlink } from 'node:fs/promises';
import { createHash } from 'node:crypto';
const cases = JSON.parse(await readFile(new URL('./cases.json', import.meta.url), 'utf8'));
export { cases };
export async function saveCapture(outputDir, name, screenshots, update = false) {
  const sha256 = hash(screenshots[0]); const repeatSha256 = hash(screenshots[1]);
  if (sha256 !== repeatSha256) {
    await writeFile(`${outputDir}/${name.replace('.jpg','-candidate.jpg')}`,screenshots[0]);
    await writeFile(`${outputDir}/${name.replace('.jpg','-repeat.jpg')}`,screenshots[1]);
    throw new Error(`${name}: independent reload screenshots differ`);
  }
  let baseline;
  try { baseline = await readFile(`${outputDir}/${name}`); } catch(error) { if(error.code !== 'ENOENT') throw error; }
  if (!update && (!baseline || hash(baseline) !== sha256)) {
    await writeFile(`${outputDir}/${name.replace('.jpg','-candidate.jpg')}`,screenshots[0]);
    throw new Error(`${name}: ${baseline?'baseline changed':'baseline missing'}; review candidate before update:true`);
  }
  if(update) await writeFile(`${outputDir}/${name}`,screenshots[0]);
  return {sha256,repeatSha256,identical:true};
}
const hash = value => createHash('sha256').update(value).digest('hex');
const labels = {
  add: ['添加服务器', 'Add Server'], 'server-menu': ['更多操作 Web-01', 'More actions Web-01'],
  edit: ['编辑服务器 Web-01', 'Edit server Web-01'], 'delete-server': ['删除服务器 Web-01', 'Delete server Web-01'],
  select: ['查看 Web-01', 'View Web-01'], connect: ['连接', 'Connect'], focus: ['专注', 'Focus'],
  files: ['文件', 'Files'], monitor: ['监控', 'Monitor'], settings: ['设置', 'Settings'],
  general: ['通用', 'General'], appearance: ['外观', 'Appearance'], 'terminal-section': ['终端', 'Terminal'],
  'terminal-settings': ['终端设置', 'Terminal settings'], language: ['语言', 'Language'],
  palette: ['命令面板', 'Command palette'], 'file-menu': ['package.json 操作', 'package.json Actions'],
  'delete-file': ['删除', 'Delete'], 'save-settings': ['保存设置', 'Save settings'],
};
async function prepare(tab, item, locale) {
  const index = locale === 'en' ? 1 : 0;
  await tab.playwright.getByRole('button', { name: labels.settings[index], exact: true }).waitFor({state:'visible'});
  let dom = await tab.playwright.domSnapshot();
  for (const step of item.steps) {
    const name = labels[step][index];
    // Read state before every action. All actions target actual rendered controls.
    if (!dom.includes(`"${name}"`)) throw new Error(`${item.page}: ${step} not present in current DOM (${name})`);
    const role = ['edit', 'delete-server', 'delete-file'].includes(step) ? 'menuitem' : 'button';
    let control = tab.playwright.getByRole(role, { name, exact: true });
    if (step === 'add') control = tab.playwright.getByRole('banner').getByRole('button', { name, exact: true });
    if (step === 'terminal-section' || step === 'general' || step === 'appearance' || step === 'language') control = tab.playwright.getByRole('dialog').getByRole('button', { name, exact: true });
    await control.click();
    dom = await tab.playwright.domSnapshot();
    if (step === 'connect' && !['host-key','host-key-changed','authentication','connection-error'].includes(item.page)) {
      await tab.playwright.getByText(index ? 'Shell ready' : 'Shell 就绪', {exact:true}).waitFor({state:'visible'});
      await tab.playwright.locator('.xterm-rows').filter({hasText:'Release 2.14.0 activated'}).waitFor({state:'visible'});
      dom = await tab.playwright.domSnapshot();
    }
    if (step === 'files') {
      await tab.playwright.locator('.files-table-scroll[aria-busy="false"]').waitFor({state:'visible'});
      await tab.playwright.getByRole('button', {name:'package.json',exact:true}).waitFor({state:'visible'});
      dom = await tab.playwright.domSnapshot();
    }
    if (step === 'save-settings') { await tab.playwright.getByRole('dialog').waitFor({state:'hidden'}); dom = await tab.playwright.domSnapshot(); }
  }
  // Verify the requested screen, rather than archiving a transient/loading page.
  const expected = {
    'add-server': ['添加服务器','Add server'], 'edit-server':['编辑服务器','Edit server'],
    'delete-server':['删除服务器？','Delete server?'], 'host-key':['确认服务器身份','Verify server identity'],
    'host-key-changed':['服务器身份发生变化','Server identity changed'], authentication:['输入 SSH 密码','Enter SSH password'],
    'connection-error':['服务器拒绝连接','Connection refused'], 'file-delete':['确认删除？','Confirm deletion?'],
    'settings-general':['断开连接前确认','Confirm before disconnecting'],
    'settings-appearance':['应用图标样式','App icon style'], 'settings-terminal':['滚动缓冲行数','Scrollback lines'],
    'settings-language':['中文','English'], palette:['combobox','combobox'],
    transfer:['config.yml','config.yml'], monitor:['24.5%','24.5%'], 'monitor-unavailable':['不支持','Unsupported'],
    toast:['设置已保存。','Settings saved.'],
  }[item.page];
  if (expected && !dom.includes(expected[index])) throw new Error(`${item.page}: expected screen marker ${expected[index]} missing`);
  return dom;
}
export async function captureCases({ tab, viewport, outputDir, width, height, theme, locale, pages, update = false, baseUrl = 'http://127.0.0.1:1420/' }) {
  await mkdir(outputDir, {recursive:true}); await viewport.set({width,height});
  const results = [];
  for (const item of cases.filter(item => !pages || pages.includes(item.page))) {
    const url = `${baseUrl}?harness=visual&page=${item.page}&theme=${theme}&locale=${locale}`;
    const screenshots = []; const observations = [];
    for (let run = 0; run < 2; run++) {
      if (await tab.url() === url) await tab.reload(); else await tab.goto(url);
      await viewport.set({width,height});
      await prepare(tab, item, locale);
      observations.push(await tab.playwright.evaluate(() => ({
        width: innerWidth, height: innerHeight, dpr: devicePixelRatio,
        theme: document.documentElement.dataset.theme, locale: document.documentElement.lang,
        overflow: document.documentElement.scrollWidth > innerWidth,
        fonts: document.fonts.status,
      })));
      const observed = observations.at(-1);
      if (observed.width !== width || observed.height !== height || observed.theme !== theme || observed.locale !== locale || observed.overflow || observed.fonts !== 'loaded') throw new Error(`${item.page}: capture condition mismatch ${JSON.stringify(observed)}`);
      screenshots.push(await tab.screenshot({fullPage:false}));
    }
    const name = `${item.page}-${theme}-${locale}-${width}x${height}.jpg`;
    const comparison = await saveCapture(outputDir,name,screenshots,update);
    const result = { page:item.page, file:name, url, ...observations[0], ...comparison };
    results.push(result);
    // Persist a checkpoint after every page so interrupted runs remain reviewable.
    await writeFile(`${outputDir}/.capture-${theme}-${locale}-${width}x${height}.json`, JSON.stringify(results,null,2)+'\n');
  }
  const suffix=pages?'-'+pages.join('_'):'';
  await writeFile(`${outputDir}/capture-${theme}-${locale}-${width}x${height}${suffix}.json`,JSON.stringify(results,null,2)+'\n');
  await unlink(`${outputDir}/.capture-${theme}-${locale}-${width}x${height}.json`);
  return results;
}

export async function captureReferences({tab,viewport,outputDir,width,height,theme,locale,pages,update=false,baseUrl='http://127.0.0.1:8080/MauLink_prototype_visual.html'}) {
  await mkdir(outputDir,{recursive:true}); await viewport.set({width,height}); const results=[];
  for (const item of cases.filter(item=>item.reference && (!pages || pages.includes(item.page)))) {
    const url=`${baseUrl}#theme=${theme}&lang=${locale==='en'?'en':'zh'}&${item.reference}`;
    const screenshots=[]; let observation;
    for(let run=0;run<2;run++) {
      if(await tab.url()!==url) await tab.goto(url);
      // The reference reads its hash only at boot; hash navigation alone retains the previous page.
      await tab.reload();
      await viewport.set({width,height});
      const dom=await tab.playwright.domSnapshot(); if(!dom.includes('MauLink')) throw new Error(`${item.page}: reference did not render`);
      if(item.page.startsWith('settings-')) {
        const expected={ 'settings-general':['通用','General'], 'settings-appearance':['外观','Appearance'], 'settings-terminal':['终端','Terminal'], 'settings-language':['语言','Language'] }[item.page][locale==='en'?1:0];
        if(!dom.includes('heading "'+expected+'"')) throw new Error(`${item.page}: reference heading mismatch`);
      }
      observation=await tab.playwright.evaluate(()=>({width:innerWidth,height:innerHeight,dpr:devicePixelRatio,theme:document.documentElement.dataset.theme,locale:document.documentElement.lang,overflow:document.documentElement.scrollWidth>innerWidth,fonts:document.fonts.status}));
      if(observation.width!==width||observation.height!==height||observation.theme!==theme||observation.locale!==locale||observation.fonts!=='loaded') throw new Error(`${item.page}: reference condition mismatch ${JSON.stringify(observation)}`);
      screenshots.push(await tab.screenshot({fullPage:false}));
    }
    const name=`${item.page}-${theme}-${locale}-${width}x${height}.jpg`;
    const comparison=await saveCapture(outputDir,name,screenshots,update);
    results.push({page:item.page,file:name,url,...observation,...comparison});
    await writeFile(`${outputDir}/.capture-${theme}-${locale}-${width}x${height}.json`,JSON.stringify(results,null,2)+'\n');
  }
  const suffix=pages?'-'+pages.join('_'):'';
  await writeFile(`${outputDir}/capture-${theme}-${locale}-${width}x${height}${suffix}.json`,JSON.stringify(results,null,2)+'\n');
  await unlink(`${outputDir}/.capture-${theme}-${locale}-${width}x${height}.json`);
  return results;
}
