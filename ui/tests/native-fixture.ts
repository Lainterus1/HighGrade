import {spawn,spawnSync,type ChildProcess} from 'node:child_process';
import {mkdtempSync,mkdirSync,readFileSync,writeFileSync,readdirSync,rmSync} from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
const repository=path.resolve('..');
export const exe=process.env.HIGHGRADE_TEST_EXE??path.join(repository,'target/debug',process.platform==='win32'?'highgrade.exe':'highgrade');
export function fixture(executable=exe){
 const parent=path.join(repository,'target/highgrade/tmp');mkdirSync(parent,{recursive:true});const root=mkdtempSync(path.join(parent,'native-ui-'));
 function cli(operation:string,...args:string[]):any{for(let attempt=0;;attempt++){const p=spawnSync(executable,[operation,'--root',root,...args],{encoding:'utf8',windowsHide:true});const report=JSON.parse(p.stdout);if(!p.status)return report.result;if(attempt<10&&report.findings?.[0]?.message.startsWith('StoreBusy:')){Atomics.wait(new Int32Array(new SharedArrayBuffer(4)),0,0,30);continue}throw Error(p.stdout)}}
 const sha=()=>cli('spec-list').store_sha256 as string;
 const input=(value:unknown)=>{writeFileSync(path.join(root,'input.json'),JSON.stringify(value));return 'input.json'};
 const edit=(id:string,value:unknown)=>cli('spec-edit','--id',id,'--expected',sha(),'--input',input(value));
 const row=(id:string)=>cli('spec-ui').changes.find((r:any)=>r.id===id);
 const attention=(id:string,value:unknown)=>cli('spec-attention','--id',id,'--expected',sha(),'--input',input(value));
 const handoff=(id:string,kind:string,key=kind,verified_revision?:string)=>attention(id,{action:'request',request_id:key,kind,verified_revision,reason:kind==='question'?'Как сохранить правки?':kind==='requirements'?'Проверьте требования':'Проверьте результат',content_sha256:row(id).content_sha256});
 function ready(id:string){
  const c=cli('spec-read','--id',id).change;edit(id,{tasks:c.tasks.map((t:any)=>({...t,done:true}))});
  writeFileSync(path.join(root,'proof.txt'),'Actual isolated fixture for interface contract');
  cli('spec-evidence','--id',id,'--expected',sha(),'--input',input({command:'fixture preparation','captured_at':new Date().toISOString(),'method':'manual','scenario':`${id}-S1`,outcome:'passed',observation:'Controlled ready-result fixture, not project evidence.',inputs:['proof.txt'],report:'proof.txt'}));
  cli('spec-review','--id',id,'--expected',sha(),'--verdict','go','--reviewer','fixture','--conclusion','Controlled fixture review');
  cli('spec-integrate','--id',id,'--expected',sha());
 }
 for(let i=1;i<=7;i++){
  const id=`HG-${String(i).padStart(4,'0')}`;
  cli('spec-new','--title',`Спецификация ${i}`,'--expected',i===1?'absent':sha());
  edit(id,{goal:`Цель ${i}`,rationale:`Причина ${i}`,scope:'Только текущий проект',tasks:[{id:`${id}-T1`,description:'Выполнить условие',done:false}],operations:[{action:'add',requirement:{id:`${id}-R1`,title:`Требование ${i}`,statement:`Условие ${i}`,scenarios:[{id:`${id}-S1`,given:'Текст документа',when:'Пользователь читает',then:'Все поля видны',verification:'Сверка с источником'}]}}]});
 }
 cli('spec-tag-set','--id','interface','--title','Интерфейс','--description','Тема','--expected',sha());edit('HG-0002',{tags:['interface']});
 handoff('HG-0001','question');handoff('HG-0002','requirements');ready('HG-0003');handoff('HG-0003','result','result','fixture-build-v1');ready('HG-0004');handoff('HG-0004','result');
 const r=row('HG-0004');
 attention('HG-0004',{action:'respond',request_id:'result',content_sha256:r.content_sha256,decision:'accepted',author:'Fixture user',comment:'Test acceptance',verified_revision:'fixture-build-v1'});
 cli('spec-abandon','--id','HG-0007','--expected',sha(),'--reason','Отменённый пример');
 // Only test fixture mutation: model an older valid structural profile.
 const old=path.join(root,'specs/changes/HG-0006/spec.json');const v=JSON.parse(readFileSync(old,'utf8'));delete v.scoped_baseline;writeFileSync(old,JSON.stringify(v));
 function hash(){const files:Record<string,string>={};const walk=(folder:string)=>{for(const e of readdirSync(folder,{withFileTypes:true})){const p=path.join(folder,e.name);if(e.isDirectory())walk(p);else files[path.relative(root,p)]=createHash('sha256').update(readFileSync(p)).digest('hex')}};walk(path.join(root,'specs'));return files}
 let child:ChildProcess|undefined;
 async function start(runtime=executable){child=spawn(runtime,['ui','--root',root,'--no-open','true'],{env:{...process.env,PATH:''},stdio:['ignore','pipe','pipe'],windowsHide:true});return await new Promise<string>((resolve,reject)=>{child!.stdout!.once('data',d=>{const result=JSON.parse(String(d).split('\n')[0]);if(result.ui_url)resolve(result.ui_url);else reject(Error(JSON.stringify(result)))});child!.once('error',reject)})}
 async function close(){if(child&&child.exitCode===null){child.kill();await new Promise(resolve=>child!.once('exit',resolve))}if(path.dirname(root)!==parent)throw Error('unsafe fixture cleanup');rmSync(root,{recursive:true,force:true})}
 return {root,cli,sha,edit,row,handoff,attention,ready,hash,start,close};
}
