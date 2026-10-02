import {test,expect} from '@playwright/test';
import {fixture} from '../tests/native-fixture';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import path from 'node:path';
const output=process.env.HIGHGRADE_PERF_OUTPUT;
if(!output)throw Error('Set HIGHGRADE_PERF_OUTPUT to a new report directory');
for(const [name,count,operations] of [['ordinary',7,3],['catalog',250,3],['document',7,160]] as const){
 test(name,async({page,request,browserName})=>{
  const f=fixture();try{
   // Expand only isolated fixture files, never a working project's catalog.
   const specPath=path.join(f.root,'specs/changes/HG-0005/spec.json');
   const spec=JSON.parse(readFileSync(specPath,'utf8'));
   spec.operations=Array.from({length:operations},(_,i)=>({action:'add',requirement:{id:`HG-0005-R${i+1}`,title:`Требование ${i+1}`,statement:'Исходная формулировка требования. '.repeat(12),scenarios:[{id:`HG-0005-S${i+1}`,given:'Документ открыт',when:'Человек редактирует поле',then:`Условие готовности ${i+1}`,verification:'Сравнить сохранённый текст'}]}}));
   writeFileSync(specPath,JSON.stringify(spec));
   for(let n=8;n<=count;n++){
    const id=`HG-${String(n).padStart(4,'0')}`,dir=path.join(f.root,'specs/changes',id);mkdirSync(dir);
    const copy=JSON.parse(JSON.stringify(spec).replaceAll('HG-0005',id));copy.id=id;copy.title=`Спецификация ${n}`;
    writeFileSync(path.join(dir,'spec.json'),JSON.stringify(copy));
   }
   const cat=path.join(f.root,'specs/catalog.json'),catalog=JSON.parse(readFileSync(cat,'utf8'));catalog.next_number=Math.max(8,count+1);writeFileSync(cat,JSON.stringify(catalog));
   const url=await f.start(),before=f.hash();
   const samples:Record<string,number[]>={apiList:[],apiRead:[],firstDocument:[],switchDocument:[],search:[],inputTask:[],inputToPaint:[],idleTask:[]};
   const requests:{url:string;method:string}[]=[];page.on('request',r=>{if(r.url().includes('/api/'))requests.push({url:r.url().replace(url,''),method:r.method()})});
   const cdp=await page.context().newCDPSession(page);await cdp.send('Performance.enable');
   const cpu=async()=>{const r=await cdp.send('Performance.getMetrics');return r.metrics.find((m:{name:string})=>m.name==='TaskDuration')!.value*1000};
   const paint=()=>page.evaluate(()=>new Promise<void>(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>r()))));
   for(let i=0;i<7;i++){
    for(const [key,endpoint] of [['apiList','/api/specs'],['apiRead','/api/specs/HG-0005']]){const t=performance.now();const r=await request.get(url+endpoint,{headers:{'X-HighGrade-Api':'2'}});expect(r.ok()).toBeTruthy();await r.body();samples[key].push(performance.now()-t)}
    await page.goto('about:blank');await cdp.send('Network.clearBrowserCache');const t=performance.now();await page.goto(url);await expect(page.locator('.spec-document h1')).toBeVisible();await page.evaluate(()=>document.fonts.ready);await paint();samples.firstDocument.push(performance.now()-t);
   }
   await page.getByRole('button',{name:'Все спецификации'}).click();
   for(let i=0;i<20;i++){
    const id=i%2?'HG-0002':'HG-0005',title=i%2?'Спецификация 2':'Спецификация 5';const t=performance.now();await page.locator('.spec-row').filter({hasText:id}).click();await expect(page.locator('.spec-document h1')).toHaveText(title);await expect(page.getByRole('button',{name:'Обновить',exact:true})).toBeEnabled();await paint();samples.switchDocument.push(performance.now()-t);
   }
   for(let i=0;i<20;i++){const t=performance.now();await page.getByLabel('Поиск по спецификациям').fill(i%2?'':'HG-0005');await expect(page.locator('.spec-row')).toHaveCount(i%2?count-2:1);await paint();samples.search.push(performance.now()-t)}
   await page.locator('.spec-row').filter({hasText:'HG-0005'}).click();await expect(page.getByRole('button',{name:'Редактировать',exact:true})).toBeEnabled();await page.getByRole('button',{name:'Редактировать',exact:true}).click();
   if(name==='ordinary'){mkdirSync(output!,{recursive:true});await page.locator('.editor').screenshot({path:path.join(output!,'editor.png'),caret:'hide'});}
   const field=page.locator('#edit-requirement-0-statement');await field.focus();
   await page.evaluate(()=>{(window as any).__inputTimes=[];document.addEventListener('input',()=>{const t=performance.now();requestAnimationFrame(()=>{(window as any).__inputTimes.push(performance.now()-t)})},true)});
   for(let i=0;i<25;i++){const t=await cpu();await field.press('End');await field.press('x');await paint();samples.inputTask.push((await cpu())-t)}
   samples.inputToPaint=await page.evaluate(()=>(window as any).__inputTimes);
   const t=await cpu();const idleStart=requests.length;await page.waitForTimeout(11000);samples.idleTask.push((await cpu())-t);
   expect(f.hash()).toEqual(before);expect(requests.filter(r=>r.method!=='GET')).toEqual([]);
   const summary=Object.fromEntries(Object.entries(samples).map(([k,v])=>{const sorted=[...v].sort((a,b)=>a-b);return [k,{n:v.length,median:sorted[Math.floor(sorted.length/2)],p95:sorted[Math.min(sorted.length-1,Math.ceil(sorted.length*.95)-1)]}]}));
   mkdirSync(output!,{recursive:true});writeFileSync(path.join(output!,name+'.json'),JSON.stringify({name,count,operations,browserName,executable:process.env.HIGHGRADE_TEST_EXE??'target/debug/highgrade',captured_at:new Date().toISOString(),samples,summary,idleRequests:requests.slice(idleStart)},null,2)+'\n');console.log(name,summary);
  }finally{await f.close()}
 });
}
