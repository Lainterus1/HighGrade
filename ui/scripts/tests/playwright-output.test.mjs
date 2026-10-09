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
