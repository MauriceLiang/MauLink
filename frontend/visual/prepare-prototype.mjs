// Freeze only the local reference's demo clock/animation/storage. Do not edit the original prototype or product.
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
const source = new URL('../../UI/MauLink_prototype_local.html', import.meta.url);
const html = await readFile(source, 'utf8');
const freeze = `<script>
Math.random = () => 0.5;
Date.now = () => 1790812800000;
window.setInterval = () => 0;
Object.defineProperty(window, 'localStorage', { value: {getItem: () => null, setItem: () => {}, removeItem: () => {}} });
</script><style>*{animation:none!important;transition:none!important;caret-color:transparent!important}</style>`;
if (!html.includes('<script>') || !html.includes('/* boot */')) throw new Error('Prototype structure changed; inspect before preparing reference');
// Align server count with the fixture; preserve the original layout, styles and functionality.
const originalServer = '  mkServer({id:"staging-01",name:"Staging-01",host:"10.0.0.14",group:"development",known:true}),';
if (!html.includes(originalServer)) throw new Error('Prototype demo seed changed');
const files = JSON.parse(await readFile(new URL('./files.json',import.meta.url),'utf8')).map(({name,fileType,sizeBytes})=>({n:name,type:fileType==='directory'?'folder':'file',size:sizeBytes===null?'—':Number(sizeBytes)<1024?`${sizeBytes} B`:`${(Number(sizeBytes)/1024).toFixed(1)} KB`,md:'Oct 1 08:00'}));
const bootMarker = ' if(hp.get("palette")==="1")';
if (!html.includes(bootMarker)) throw new Error('Prototype boot marker changed');
const align = ` state.files=${JSON.stringify(files)};
 if(hp.get('transfers')!=='1')state.transfers=[];
 if(state.modal&&state.modal.type==='add'){state.modal.name='';state.modal.host='';}
 if(state.modal&&state.modal.type==='edit'){state.modal.name='Web-01';}
 if(state.modal&&state.modal.type==='filedel'){state.modal.file={n:'package.json'};}
`;
const reference = html.replace('<script>',freeze+'<script>').replace(originalServer,'').replace(bootMarker,align+bootMarker);
const directory = process.argv[2] ?? '/private/tmp/maulink-phase10-prototype';
await mkdir(directory,{recursive:true});
await writeFile(`${directory}/MauLink_prototype_visual.html`,reference);
await writeFile(`${directory}/source.json`,JSON.stringify({source:source.pathname,sha256:createHash('sha256').update(html).digest('hex'),changes:['fixed demo random value/time','disabled demo intervals and CSS animation','isolated preferences','aligned server count, form defaults, files and transfer presence with fixture']},null,2)+'\n');
console.log(`${directory}/MauLink_prototype_visual.html`);
