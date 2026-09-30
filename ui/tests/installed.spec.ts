import {test,expect} from '@playwright/test';
import {spawnSync} from 'node:child_process';
import {readFileSync,writeFileSync,mkdirSync,rmSync,existsSync} from 'node:fs';
import {createHash} from 'node:crypto';
import path from 'node:path';
import {fixture} from './native-fixture';
const configured=process.env.HIGHGRADE_UI_CANDIDATE;
test.skip(!configured,'Requires exact-SHA candidate built in an isolated release fixture');
test.setTimeout(180000);
test('HG57 S1-S7: installed release, recovery, two roots, local assets and full human workflow',async({page,request})=>{
 const candidate=path.resolve(configured!);const candidateExe=path.join(candidate,'highgrade.exe');const manifest=JSON.parse(readFileSync(path.join(candidate,'kit/manifest.json'),'utf8'));
 const parent=path.resolve('../target/highgrade/tmp');const profile=path.join(parent,'ui-installed-profile');expect(existsSync(profile)).toBeFalsy();mkdirSync(profile);
 const f=fixture(candidateExe),other=fixture(candidateExe);const before=f.hash(),beforeOther=other.hash();
 const sha=(b:Buffer)=>createHash('sha256').update(b).digest('hex');
 function cli(op:string,args:string[]=[],success=true){const p=spawnSync(candidateExe,[op,'--profile',profile,...args],{encoding:'utf8',windowsHide:true});const r=JSON.parse(p.stdout);if(success)expect(p.status,p.stdout).toBe(0);else expect(p.status).not.toBe(0);return r}
 const oldSource=process.env.HIGHGRADE_OLD_SOURCE!;const oldExe=process.env.HIGHGRADE_OLD_EXE!;
 try{
 cli('global-install',['--source',oldSource,'--candidate-exe',oldExe]);
 const pointerPath=path.join(profile,'.highgrade/global/active.json');const oldPointer=JSON.parse(readFileSync(pointerPath,'utf8'));
 const skillRel='.agents/skills/highgrade-work/SKILL.md',skillPath=path.join(profile,skillRel);const skillBefore=readFileSync(skillPath),interrupted=Buffer.from('Interrupted test candidate');
 const transactionPath=path.join(profile,'.highgrade/global/skill-transaction.json');writeFileSync(transactionPath,JSON.stringify({old_active:oldPointer,candidate:manifest.release,before:{[skillRel]:[...skillBefore]},after:{[skillRel]:sha(interrupted)}}));writeFileSync(skillPath,interrupted);
 expect(JSON.stringify(cli('global-status',[],false))).toContain('GlobalSkillRecoveryRequired');cli('global-recover');expect(readFileSync(skillPath)).toEqual(skillBefore);expect(JSON.parse(readFileSync(pointerPath,'utf8'))).toEqual(oldPointer);
 const args=['--source',path.join(candidate,'kit'),'--candidate-exe',candidateExe];const preview=cli('global-update',args,false);expect(preview.result.validation).toBe('passed');const fingerprint=preview.measurements.find((m:any)=>m.candidate_sha256).candidate_sha256;
 cli('global-update',[...args,'--apply','true','--candidate-sha256',fingerprint]);cli('global-status');
 const active=JSON.parse(readFileSync(pointerPath,'utf8'));expect(active.release).toBe(manifest.release);const installed=path.join(profile,'.highgrade/global/releases',active.release,'highgrade.exe');expect(sha(readFileSync(installed))).toBe(sha(readFileSync(candidateExe)));
 expect(f.hash()).toEqual(before);expect(other.hash()).toEqual(beforeOther);
 const url=await f.start(installed),otherUrl=await other.start(installed);expect(url).not.toBe(otherUrl);
 const external:string[]=[],errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));await page.route('**/*',route=>{const u=new URL(route.request().url());if(u.hostname!=='127.0.0.1'){external.push(u.href);return route.abort()}return route.continue()});
 await page.goto(otherUrl);await expect(page.locator('.workspace')).toBeVisible();expect(other.hash()).toEqual(beforeOther);
 await page.goto(url);await page.getByRole('button',{name:'Все спецификации'}).click();await page.getByLabel('Поиск по спецификациям').fill('HG-0005');await page.locator('.spec-row').filter({hasText:'HG-0005'}).click();await expect(page.locator('.document')).toContainText('Цель 5');await page.getByLabel('Поиск по спецификациям').fill('');
 await page.evaluate(()=>document.fonts.ready);expect(await page.evaluate(()=>[...document.fonts].some(f=>f.family==='Manrope'&&f.status==='loaded'))).toBeTruthy();await expect(page.locator('svg[stroke-width="1.75"]').first()).toBeVisible();
 const bundle=await(await request.get(url+'/highgrade-ui.json')).json();expect(bundle.source_sha).toBe(process.env.HIGHGRADE_UI_SOURCE_SHA);expect(bundle.cli_version).toBe(manifest.cli_version);
 for(const [file,digest] of Object.entries(bundle.files)){const response=await request.get(`${url}/${file}`);expect(response.ok(),file).toBeTruthy();expect(sha(await response.body()),file).toBe(digest)}
 const font=bundle.provenance.find((p:any)=>p.source.endsWith('Manrope.ttf'));expect(font.delivered.length).toBeGreaterThan(0);expect(bundle.files[font.delivered[0]]).toBe(font.sha256);expect(await(await request.get(url+'/'+font.license)).text()).toContain('SIL OPEN FONT LICENSE');expect(await(await request.get(url+'/licenses/Lucide-LICENSE.txt')).text()).toContain('ISC');
 await page.getByRole('button',{name:'Редактировать',exact:true}).click();await page.getByLabel('Цель',{exact:true}).fill('Условие установленного UI');f.edit('HG-0005',{goal:'Одновременная правка CLI'});await page.getByRole('button',{name:'Сохранить изменения'}).click();await expect(page.getByRole('alert')).toContainText('LocalConflict');await page.getByRole('button',{name:'Перечитать и сравнить'}).click();await page.getByRole('button',{name:'Применить мои поля к новой базе'}).click();await page.getByRole('button',{name:'Сохранить изменения'}).click();await expect(page.getByRole('status')).toContainText('Состояние сохранено');expect(f.cli('spec-read','--id','HG-0005').change.goal).toBe('Условие установленного UI');
 f.handoff('HG-0005','requirements');await page.getByRole('button',{name:'Обновить',exact:true}).click();await page.getByRole('button',{name:'Согласовать требования',exact:true}).click();await page.getByLabel('Ваше имя',{exact:true}).fill('Тестовый пользователь');await page.getByRole('button',{name:'Подтвердить',exact:true}).click();await expect(page.getByRole('status')).toContainText('Состояние сохранено');expect(f.cli('spec-read','--id','HG-0005').change.tasks[0].done).toBe(false);
 f.ready('HG-0005');f.handoff('HG-0005','result');await page.getByRole('button',{name:'Обновить',exact:true}).click();await page.getByRole('button',{name:'Принять результат',exact:true}).click();await page.getByLabel('Ваше имя',{exact:true}).fill('Тестовый пользователь');await page.getByLabel('Версия проверенного результата',{exact:true}).fill(process.env.HIGHGRADE_UI_SOURCE_SHA!);await page.getByRole('button',{name:'Подтвердить',exact:true}).click();await expect(page.getByRole('status')).toContainText('Состояние сохранено');expect(f.cli('spec-read','--id','HG-0005').change.acceptance.at(-1).verified_revision).toBe(process.env.HIGHGRADE_UI_SOURCE_SHA);expect(f.row('HG-0005').category).toBe('completed');
 await page.screenshot({path:test.info().outputPath('installed.png'),fullPage:true});expect(other.hash()).toEqual(beforeOther);expect(external).toEqual([]);expect(errors).toEqual([]);
 const catalogNow=f.hash();const bytes=readFileSync(installed);const needle=Buffer.from(`"api_version": "${bundle.api_version}"`);const position=bytes.indexOf(needle);expect(position).toBeGreaterThan(0);expect(bytes.indexOf(needle,position+1)).toBe(-1);const damaged=Buffer.from(bytes);damaged[position+needle.length-2]='9'.charCodeAt(0);const invalidExe=path.join(profile,'incompatible-ui.exe');writeFileSync(invalidExe,damaged);const rejected=spawnSync(invalidExe,['ui','--root',f.root,'--no-open','true'],{encoding:'utf8',env:{...process.env,PATH:''},windowsHide:true});expect(rejected.status).not.toBe(0);expect(rejected.stdout).toContain('UiBundleInvalid');expect(rejected.stdout).not.toContain('ui_url');expect(f.hash()).toEqual(catalogNow);
 const stopped=await(await request.get(otherUrl+'/api/session')).json();expect((await request.post(otherUrl+'/api/shutdown',{headers:{Origin:otherUrl,'X-HighGrade-Token':stopped.token,'X-HighGrade-Api':stopped.api_version}})).ok()).toBeTruthy();
 }finally{await f.close();await other.close();if(path.dirname(profile)!==parent)throw Error('Unsafe cleanup');rmSync(profile,{recursive:true,force:true})}
});
