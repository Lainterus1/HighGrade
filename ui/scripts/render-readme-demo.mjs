/**
 * Rebuild and capture isolated production UI stories, then render a deterministic GIF.
 * From the repository root: node ui/scripts/render-readme-demo.mjs
 * Requires ui/npm dependencies, installed Playwright Chromium, and ffmpeg/ffprobe in PATH.
 * Does not open real project data, write backend state, or fetch external resources.
 */
import {chromium} from '@playwright/test';
import {createServer} from 'node:http';
import {readFile,writeFile,mkdir,mkdtemp,readdir,rm} from 'node:fs/promises';
import {openSync,closeSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const args=process.argv.slice(2);
if(args.length&&!(args.length===2&&args[0]==='--evidence'))throw new Error('Usage: node ui/scripts/render-readme-demo.mjs [--evidence .highgrade/local/...]');
const evidence=path.resolve(root,args[1]||'.highgrade/local/reviews/readme-demo');
if(!path.relative(path.join(root,'.highgrade/local'),evidence)||path.relative(path.join(root,'.highgrade/local'),evidence).startsWith('..'))throw new Error('Evidence must be inside .highgrade/local/');
const output=path.join(root,'docs/assets/highgrade-overview.gif');
const poster=path.join(root,'docs/assets/highgrade-overview.png');
const scratchRoot=path.join(root,'target/highgrade/tmp');
await mkdir(scratchRoot,{recursive:true});await mkdir(evidence,{recursive:true});await mkdir(path.dirname(output),{recursive:true});
const scratch=await mkdtemp(path.join(scratchRoot,'readme-demo-'));
const frames=path.join(scratch,'frames');await mkdir(frames);
const startedAt=new Date().toISOString();
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
async function files(dir){const out=[];for(const e of await readdir(dir,{withFileTypes:true})){const p=path.join(dir,e.name);if(e.isDirectory())out.push(...await files(p));else out.push(p)}return out.sort()}
async function fingerprint(){
 const inputs=[...await files(path.join(root,'ui/src')),...await files(path.join(root,'ui/.storybook')),path.join(root,'ui/package.json'),path.join(root,'ui/package-lock.json'),fileURLToPath(import.meta.url),path.join(root,'ui/scripts/readme-demo.html')];
 return Object.fromEntries(await Promise.all(inputs.map(async p=>[path.relative(root,p).split(path.sep).join('/'),hash(await readFile(p))])));
}
const before=await fingerprint();
const buildLog=openSync(path.join(evidence,'storybook-build.log'),'w');
try{execFileSync('npm',['run','build-storybook'],{cwd:path.join(root,'ui'),stdio:['ignore',buildLog,'pipe']})}finally{closeSync(buildLog)}
const captures=new Map();const pageErrors=[];const forbiddenRequests=[];const actions=[];
const theme=await readFile(path.join(root,'ui/src/styles/theme.css'),'utf8');
const tokens=theme.match(/:root\s*\{([^}]+)\}/)?.[1];if(!tokens)throw new Error('Production theme tokens not found');
const mime={'.html':'text/html; charset=utf-8','.js':'text/javascript','.mjs':'text/javascript','.css':'text/css','.json':'application/json','.svg':'image/svg+xml','.png':'image/png','.gif':'image/gif','.ttf':'font/ttf','.woff2':'font/woff2'};
const server=createServer(async(req,res)=>{
 try{
  const url=new URL(req.url,'http://localhost');const route=decodeURIComponent(url.pathname);
  if(req.method!=='GET'){res.writeHead(405);res.end();return}
  if(route==='/demo-tokens.css'){res.writeHead(200,{'Content-Type':mime['.css']});res.end(':root{'+tokens+'}');return}
  if(route.startsWith('/capture/')){const data=captures.get(route.slice(9));if(!data)throw new Error('Missing capture');res.writeHead(200,{'Content-Type':'image/png'});res.end(data);return}
  let base,relative;
  if(route==='/demo.html'){base=path.join(root,'ui/scripts');relative='readme-demo.html'}
  else if(route.startsWith('/assets/')){base=path.join(root,'ui/src/assets');relative=route.slice(8)}
  else if(route.startsWith('/storybook/')){base=path.join(root,'ui/storybook-static');relative=route.slice(11)||'index.html'}
  else {res.writeHead(404);res.end();return}
  const file=path.resolve(base,relative);const rel=path.relative(base,file);
  if(rel.startsWith('..')||path.isAbsolute(rel))throw new Error('Invalid path');
  const bytes=await readFile(file);res.writeHead(200,{'Content-Type':mime[path.extname(file)]||'application/octet-stream'});res.end(bytes);
 }catch{res.writeHead(404);res.end()}
});
let browser;
try{
 await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve)});
 const origin=`http://127.0.0.1:${server.address().port}`;
 browser=await chromium.launch({headless:true});
 const context=await browser.newContext({viewport:{width:1088,height:360},deviceScaleFactor:1,reducedMotion:'reduce'});
 await context.route('**/*',async route=>{
  const req=route.request(),url=req.url();
  if((!url.startsWith(origin+'/')&&!url.startsWith('data:')&&!url.startsWith('blob:'))||req.method()!=='GET'||url.includes('/api/')){
   forbiddenRequests.push({url,method:req.method()});await route.abort();
  }else await route.continue();
 });
 const ui=await context.newPage();ui.on('pageerror',e=>pageErrors.push(String(e)));
 const index=JSON.parse(await readFile(path.join(root,'ui/storybook-static/index.json'),'utf8'));
 const story=(file,name)=>Object.values(index.entries).find(e=>e.importPath.endsWith(file)&&e.exportName===name)?.id;
 async function openStory(file,name){
  const id=story(file,name);if(!id)throw new Error('Story not found: '+file+' '+name);
  await ui.goto(origin+'/storybook/iframe.html?id='+encodeURIComponent(id)+'&viewMode=story');
  await ui.locator('.document').waitFor();await ui.evaluate(()=>document.fonts.ready);
  await ui.addStyleTag({content:'*{caret-color:transparent!important;animation:none!important;transition:none!important}'});
 }
 async function capture(name,action){captures.set(name+'.png',await ui.screenshot());actions.push({capture:name,action,url:ui.url().replace(origin,'<loopback>')})}
 await openStory('SpecWorkspace.stories.tsx','Reading');await capture('overview','Открыть рабочее пространство на HG-DEMO-01');
 await ui.getByRole('textbox',{name:'Поиск по спецификациям'}).fill('Интерфейс');
 if(await ui.locator('.spec-row').count()!==1)throw new Error('Search did not filter the demo');
 await capture('search','Ввести «Интерфейс» в поиск; остаётся одна демо-спека');
 await ui.getByRole('button',{name:'Документ остаётся источником правды'}).click();
 await ui.locator('.spec-changes').scrollIntoViewIfNeeded();
 await capture('expanded','Раскрыть требование в production NativeDocument');
 await openStory('SpecEditor.stories.tsx','Editing');
 await ui.getByRole('textbox',{name:'Цель',exact:true}).fill('Просматривать и уточнять спецификации проекта в локальном интерфейсе.');
 await ui.evaluate(()=>{const goal=document.getElementById('edit-goal');window.scrollTo(0,window.scrollY+goal.getBoundingClientRect().top-90)});
 await capture('editor','Уточнить цель в production SpecEditor и прокрутить к полю; не сохранять, транспорт только в памяти');
 await ui.close();
 const page=await context.newPage();await page.setViewportSize({width:1200,height:675});page.on('pageerror',e=>pageErrors.push(String(e)));
 await page.goto(origin+'/demo.html');await page.evaluate(()=>document.fonts.ready);
 // Preload every screenshot so a transition cannot capture an undecoded image.
 await page.evaluate(async()=>Promise.all(['overview','search','expanded','editor'].map(name=>new Promise((resolve,reject)=>{const i=new Image();i.onload=resolve;i.onerror=reject;i.src='/capture/'+name+'.png'}))));
 const controls=[0,5,6.5,9,11.5,15.5,19.5,23.5,27];
 const layout=[];
 for(const t of controls){
  await page.evaluate(t=>window.setFrame(t),t);
  layout.push(await page.evaluate(time=>{
   const scene=[...document.querySelectorAll('.scene')].find(el=>getComputedStyle(el).display!=='none');
   const footer=document.querySelector('.footer').getBoundingClientRect();
   const nodes=[...scene.querySelectorAll('h1,h2,p,.flow,.rows,.route,.trace,.cta')];
   return {time,scene:scene.id,overlaps:nodes.filter(el=>el.getBoundingClientRect().bottom>footer.top-8).map(el=>el.textContent),headlinePixels:parseFloat(getComputedStyle(scene.querySelector('h1,h2')).fontSize)};
  },t));
 }
 if(layout.some(x=>x.overlaps.length))throw new Error('Text overlaps footer: '+JSON.stringify(layout));
 for(let i=0;i<290;i++){
  await page.evaluate(async t=>{setFrame(t);await Promise.all([...document.images].map(i=>i.decode()));await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))},i/10);
  await page.screenshot({path:path.join(frames,String(i).padStart(4,'0')+'.png')});
  if(i%50===0)console.log(`Frames ${i}/290`);
 }
 await writeFile(poster,await readFile(path.join(frames,'0000.png')));
 execFileSync('ffmpeg',['-hide_banner','-loglevel','error','-framerate','10','-i',path.join(frames,'%04d.png'),'-filter_complex','split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=3:diff_mode=rectangle','-loop','0','-gifflags','+transdiff','-y',output]);
 const probe=JSON.parse(execFileSync('ffprobe',['-v','error','-show_entries','stream=width,height,nb_frames,r_frame_rate:format=duration,size','-of','json',output],{encoding:'utf8'}));
 const stream=probe.streams[0];
 if(stream.width!==1200||stream.height!==675||Number(probe.format.duration)!==29||Number(probe.format.size)>5*1024*1024)throw new Error('GIF limits exceeded: '+JSON.stringify(probe));
 execFileSync('ffmpeg',['-v','error','-i',output,'-f','null','-']);
 const gifBytes=await readFile(output);if(!gifBytes.includes(Buffer.from('NETSCAPE2.0')))throw new Error('Missing GIF loop extension');
 const firstHash=hash(await readFile(path.join(frames,'0000.png'))),lastHash=hash(await readFile(path.join(frames,'0289.png')));
 if(firstHash!==lastHash)throw new Error('Loop endpoints differ');
 for(const t of controls){const name=`scene-${String(t).replace('.','-')}.png`;await writeFile(path.join(evidence,name),await readFile(path.join(frames,String(Math.round(t*10)).padStart(4,'0')+'.png')))}
 // Contact sheets show control frames at README desktop and narrow-screen widths.
 for(const width of [600,390]){
  const board=await context.newPage();await board.setViewportSize({width:width*2,height:Math.ceil(controls.length/2)*(Math.round(width*675/1200)+34)});
  await board.setContent('<style>body{margin:0;background:#f8f6f0;font:16px sans-serif}.grid{display:grid;grid-template-columns:repeat(2,1fr)}figure{margin:0}img{display:block;width:100%}figcaption{height:34px;padding:6px 12px}</style><div class="grid">'+controls.map(t=>`<figure><img data-time="${t}"><figcaption>${t} s</figcaption></figure>`).join('')+'</div>');
  const images=await Promise.all(controls.map(t=>readFile(path.join(frames,String(Math.round(t*10)).padStart(4,'0')+'.png'))));
  await board.evaluate(async data=>{await Promise.all([...document.images].map(async(i,n)=>{i.src='data:image/png;base64,'+data[n];await i.decode()}))},images.map(b=>b.toString('base64')));
  await board.screenshot({path:path.join(evidence,`contact-${width}.png`),fullPage:true});await board.close();
 }
 const after=await fingerprint();if(JSON.stringify(before)!==JSON.stringify(after))throw new Error('Source changed during render');
 if(pageErrors.length||forbiddenRequests.length)throw new Error('Unexpected browser activity: '+JSON.stringify({pageErrors,forbiddenRequests}));
 const report={status:'passed',startedAt,finishedAt:new Date().toISOString(),command:'node ui/scripts/render-readme-demo.mjs '+args.join(' '),node:process.version,chromium:browser.version(),ffmpeg:execFileSync('ffmpeg',['-version'],{encoding:'utf8'}).split('\n')[0],actions,sourceHashes:before,storybookHash:hash(JSON.stringify(await Promise.all((await files(path.join(root,'ui/storybook-static'))).map(async p=>[path.relative(root,p),hash(await readFile(p))])))),probe,loop:'infinite',endpointsIdentical:firstHash===lastHash,poster:path.relative(root,poster),gif:{path:path.relative(root,output),sha256:hash(gifBytes),bytes:gifBytes.length},pageErrors,forbiddenRequests,layout,controlTimes:controls};
 await writeFile(path.join(evidence,'render.json'),JSON.stringify(report,null,2)+'\n');
 console.log(JSON.stringify({status:report.status,gif:report.gif,probe,loop:report.loop,endpointsIdentical:report.endpointsIdentical,evidence:path.relative(root,evidence)},null,2));
 await rm(scratch,{recursive:true,force:true});
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
