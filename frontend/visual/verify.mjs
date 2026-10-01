import { readFile, readdir, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
function dimensions(bytes) {
  if(bytes.readUInt16BE(0)!==0xffd8)throw new Error('Expected JPEG');
  for(let offset=2;offset<bytes.length;) {
    if(bytes[offset++]!==0xff)throw new Error('Invalid JPEG segment');
    const marker=bytes[offset++]; const length=bytes.readUInt16BE(offset);
    if([0xc0,0xc1,0xc2,0xc3,0xc5,0xc6,0xc7,0xc9,0xca,0xcb,0xcd,0xce,0xcf].includes(marker))return {height:bytes.readUInt16BE(offset+3),width:bytes.readUInt16BE(offset+5)};
    offset+=length;
  }
  throw new Error('JPEG has no size frame');
}
const root=resolve(process.argv[2]??'docs/refactor/screenshots/phase-10'); const summary={}; const gallery=[];
for(const [kind,expectedPages] of [['implementation',23],['reference',20]]) {
  const directory=`${root}/${kind}`; const names=await readdir(directory); const manifests=names.filter(name=>/^capture-(light|dark)-(zh-CN|en)-(1440x920|860x640)\.json$/.test(name));
  if(manifests.length!==8)throw new Error(`${kind}: expected 8 condition manifests, got ${manifests.length}`);
  const files=new Set(); const overflow=[];
  for(const name of manifests) {
    const records=JSON.parse(await readFile(`${directory}/${name}`,'utf8'));
    if(records.length!==expectedPages)throw new Error(`${name}: incomplete page coverage`);
    for(const record of records) {
      const bytes=await readFile(`${directory}/${record.file}`); const sha256=createHash('sha256').update(bytes).digest('hex'); const size=dimensions(bytes);
      if(!record.identical||sha256!==record.sha256||sha256!==record.repeatSha256||size.width!==record.width*record.dpr||size.height!==record.height*record.dpr)throw new Error(`${kind}/${record.file}: invalid capture evidence`);
      if(record.dpr!==1||record.fonts!=='loaded')throw new Error(`${kind}/${record.file}: inconsistent environment`);
      if(record.overflow)overflow.push(record.file);
      if(kind==='implementation'&&record.overflow)throw new Error(`Implementation overflow: ${record.file}`);
      files.add(record.file);gallery.push({kind,...record});
    }
  }
  const extras=names.filter(name=>name.endsWith('.jpg')&&!files.has(name));
  if(extras.length)throw new Error(`${kind}: unreviewed candidates/repeats: ${extras.join(', ')}`);
  summary[kind]={pages:expectedPages,conditions:manifests.length,screenshots:files.size,independentRepeat:'byte-identical',dpr:1,overflow};
}
await writeFile(`${root}/summary.json`,JSON.stringify(summary,null,2)+'\n');
const data=JSON.stringify(gallery).replaceAll('<','\\u003c');
const html=`<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>MauLink Phase 10 视觉对照</title><style>body{margin:24px;background:#f6f7f9;color:#171a21;font:14px system-ui}select{margin:0 16px 0 6px;padding:6px}article{margin:24px 0;padding:16px;background:white;border:1px solid #e5e7eb;border-radius:8px}section{display:grid;grid-template-columns:1fr 1fr;gap:16px}img{width:100%;border:1px solid #e5e7eb}figure{margin:0}figcaption{margin-bottom:8px}a{color:#5b5ce2}</style><h1>MauLink Phase 10 视觉对照</h1><p>固定 Mock IPC；184 张实现基线、160 张冻结原型参考图。每张均由两次独立加载生成一致截图。对照图用于审阅布局差异，不代表产品与原型像素相等，也不代表真实 SSH 或 Windows 验收。</p><label>主题<select id="theme"><option>light</option><option>dark</option></select></label><label>语言<select id="locale"><option>zh-CN</option><option>en</option></select></label><label>CSS 视口<select id="size"><option>1440x920</option><option>860x640</option></select></label><main></main><script>const records=${data};function render(){const [width,height]=document.querySelector('#size').value.split('x').map(Number),theme=document.querySelector('#theme').value,locale=document.querySelector('#locale').value;const selected=records.filter(r=>r.width===width&&r.height===height&&r.theme===theme&&r.locale===locale),implementation=selected.filter(r=>r.kind==='implementation');document.querySelector('main').innerHTML=implementation.map(r=>{const ref=selected.find(p=>p.kind==='reference'&&p.page===r.page);return '<article id="'+r.page+'"><h2>'+r.page+'</h2><section>'+[r,ref].map(p=>p?'<figure><figcaption>'+p.kind+' · CSS '+p.width+'×'+p.height+' · DPR '+p.dpr+(p.overflow?' · 原型存在横向溢出':'')+'</figcaption><a href="'+p.kind+'/'+p.file+'"><img loading="lazy" src="'+p.kind+'/'+p.file+'" alt="'+p.kind+' '+p.page+'"></a></figure>':'<p>原型无对应固定状态；见逐页记录。</p>').join('')+'</section></article>';}).join('');}document.querySelectorAll('select').forEach(select=>select.addEventListener('change',render));render();</script></html>`;
await writeFile(`${root}/index.html`,html);
console.log(JSON.stringify(summary,null,2));
