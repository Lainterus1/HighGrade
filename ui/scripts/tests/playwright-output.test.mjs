import {test} from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import fs from 'node:fs';
import {randomUUID} from 'node:crypto';
import {fileURLToPath} from 'node:url';

const config=fileURLToPath(new URL('../../playwright.config.ts',import.meta.url));
let sequence=0;
async function read(env){
 const previous=Object.fromEntries(['HIGHGRADE_RUN_DIR','HIGHGRADE_RUN_TOKEN','HIGHGRADE_UI_REPORT'].map(key=>[key,process.env[key]]));
 Object.assign(process.env,{HIGHGRADE_RUN_DIR:'',HIGHGRADE_RUN_TOKEN:'',HIGHGRADE_UI_REPORT:'',...env});
 try{return (await import(new URL(`../../playwright.config.ts?test=${sequence++}`,import.meta.url).href)).default;}
 finally{for(const [key,value] of Object.entries(previous)){if(value===undefined)delete process.env[key];else process.env[key]=value;}}
}
function ownedRun(){
 const folder=path.resolve(path.dirname(config),'../target/highgrade/runs');
 fs.mkdirSync(folder,{recursive:true});
 const run=path.join(folder,'ui-fixture-'+randomUUID());
 fs.mkdirSync(run);
 const token=randomUUID();
 fs.writeFileSync(path.join(run,'run.json'),JSON.stringify({schema:1,state:'active',owner_pid:process.pid,run_token:token}));
 return {run,token};
}
test('managed browser checks write machine report and attachments inside their run',async()=>{
 const {run,token}=ownedRun();
 try{
 const value=await read({HIGHGRADE_RUN_DIR:run,HIGHGRADE_RUN_TOKEN:token});
 assert.equal(value.outputDir,path.join(run,'playwright'));
 assert.equal(value.reporter[1][1].outputFile,path.join(run,'playwright','report.json'));
 }finally{fs.rmSync(run,{recursive:true});}
});
test('foreign active browser context refuses without changing its report',async()=>{
 const {run}=ownedRun();
 const report=path.join(run,'playwright/report.json');
 fs.mkdirSync(path.dirname(report)); fs.writeFileSync(report,'preserve active report');
 try{
 for(const token of ['', 'other-run']){
  await assert.rejects(read({HIGHGRADE_RUN_DIR:run,HIGHGRADE_RUN_TOKEN:token}),/context/);
  assert.equal(fs.readFileSync(report,'utf8'),'preserve active report');
 }
 }finally{fs.rmSync(run,{recursive:true});}
});
test('explicit native report destination remains supported',async()=>{
 const destination=path.resolve('target/highgrade/reports/explicit.json');
 assert.equal((await read({HIGHGRADE_UI_REPORT:destination})).reporter[1][1].outputFile,destination);
});

test('invalid managed state and dead owner refuse without changing the report',async()=>{
 const {run,token}=ownedRun();
 const stateFile=path.join(run,'run.json'),report=path.join(run,'playwright/report.json');
 const initial=JSON.parse(fs.readFileSync(stateFile,'utf8'));
 fs.mkdirSync(path.dirname(report));fs.writeFileSync(report,'preserve report bytes');
 // kill(pid, 0) only queries existence; it never terminates a process.
 const deadPid=2147483647;
 assert.throws(()=>process.kill(deadPid,0),{code:'ESRCH'});
 try{
  for(const patch of [{schema:99},{state:'completed'},{owner_pid:0},{owner_pid:1.5},{owner_pid:deadPid}]){
   fs.writeFileSync(stateFile,JSON.stringify({...initial,...patch}));
   await assert.rejects(read({HIGHGRADE_RUN_DIR:run,HIGHGRADE_RUN_TOKEN:token}),patch.owner_pid===deadPid?/owner is not active/:/run is not active/);
   assert.equal(fs.readFileSync(report,'utf8'),'preserve report bytes');
  }
 }finally{fs.rmSync(run,{recursive:true});}
});

test('unknown managed location and linked output preserve their targets',async()=>{
 const {run,token}=ownedRun();
 const target=path.join(run,'protected'),output=path.join(run,'playwright');
 fs.mkdirSync(target);fs.writeFileSync(path.join(target,'report.json'),'protected target');
 try{
  await assert.rejects(read({HIGHGRADE_RUN_DIR:target,HIGHGRADE_RUN_TOKEN:token}),/Unknown managed report run/);
  fs.symlinkSync(target,output,process.platform==='win32'?'junction':'dir');
  await assert.rejects(read({HIGHGRADE_RUN_DIR:run,HIGHGRADE_RUN_TOKEN:token}),/Linked managed report output/);
  assert.equal(fs.readFileSync(path.join(target,'report.json'),'utf8'),'protected target');
 }finally{fs.rmSync(run,{recursive:true});}
});
