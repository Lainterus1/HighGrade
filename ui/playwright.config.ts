import {defineConfig} from '@playwright/test';
import path from 'node:path';
import fs from 'node:fs';

function reportOutputDir(){
 const run=process.env.HIGHGRADE_RUN_DIR;
 if(!run)return '../target/highgrade/ui-playwright';
 const root=path.resolve(import.meta.dirname,'..');
 const relative=path.relative(root,run);
 if(!/^target\/highgrade\/runs\/[a-z0-9-]+$/.test(relative.split(path.sep).join('/')))throw new Error('Unknown managed report run');
 let current=root;
 for(const part of [...relative.split(path.sep),'run.json']){
  current=path.join(current,part);
  if(fs.lstatSync(current).isSymbolicLink())throw new Error('Linked managed report run');
 }
 const state=JSON.parse(fs.readFileSync(current,'utf8'));
 for(const suffix of ['playwright','playwright/report.json']){
  try{if(fs.lstatSync(path.join(run,suffix)).isSymbolicLink())throw new Error('Linked managed report output');}
  catch(error){if(error.code!=='ENOENT')throw error;}
 }
 if(state.schema!==1||state.state!=='active'||!Number.isInteger(state.owner_pid)||state.owner_pid<=0)throw new Error('Managed report run is not active');
 try{process.kill(state.owner_pid,0);}catch(error){if(error.code!=='EPERM')throw new Error('Managed report run owner is not active');}
 if(!process.env.HIGHGRADE_RUN_TOKEN||process.env.HIGHGRADE_RUN_TOKEN!==state.run_token)throw new Error('Managed report run context does not match the wrapper');
 return path.join(run,'playwright');
}
const outputDir=reportOutputDir();
export default defineConfig({testDir:'./tests',timeout:30000,workers:1,reporter:[['list'],['json',{outputFile:process.env.HIGHGRADE_UI_REPORT||path.join(outputDir,'report.json')}]],use:{baseURL:process.env.HIGHGRADE_UI_BASE_URL||'http://127.0.0.1:6006',headless:true,launchOptions:process.env.HIGHGRADE_CHROME?{executablePath:process.env.HIGHGRADE_CHROME}:undefined,viewport:{width:1440,height:900}},outputDir});
