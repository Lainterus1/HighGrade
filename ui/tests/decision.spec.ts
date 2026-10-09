import {test,expect,type Page} from '@playwright/test';
import type {Handoff,ReadResult,Session} from '../src/api/client';
import {decisionDraftKey,decisionRevision,readDecisionDraft,writeDecisionDraft,clearDecisionDraft} from '../src/features/specs/decisionDrafts';
import {fixture} from './native-fixture';
import {writeFileSync} from 'node:fs';
import path from 'node:path';

const session:Session={api_version:'2',token:'fixture',project:{name:'Одинаковое имя',root:'/project-a'}};
const request:Handoff={id:'request-a',kind:'result',reason:'Fixture',content_sha256:'content-a',change_sha256:'change-a',inputs_sha256:'inputs-a',at:1,response:null};
const read={store_sha256:'store-a',local_sha256:'local-a',change_sha256:'change-a',inputs_sha256:'inputs-a',change:{id:'HG-TEST'}} as ReadResult;

test.describe('HG61 S1 S2: decision draft scope and revision',()=>{
 test('preserves exact draft through repeated remounts and clears only the confirmed target',()=>{
  const key=decisionDraftKey(session,read,request),revision=decisionRevision(read,request);
  const draft={revision,author:'Автор',comment:'Не потерять текст',status:'needs_changes' as const,uncertain:true,error:'Ответ потерян'};
  writeDecisionDraft(key,draft);
  expect(readDecisionDraft(key,revision)).toEqual(draft);
  expect(readDecisionDraft(key,revision)).toEqual(draft);
  const other=decisionDraftKey(session,read,{...request,id:'request-b'});
  writeDecisionDraft(other,{...draft,comment:'Другой ответ'});
  clearDecisionDraft(key);
  expect(readDecisionDraft(key,revision).author).toBe('');
  expect(readDecisionDraft(other,revision).comment).toBe('Другой ответ');
  clearDecisionDraft(other);
 });
 test('isolates the canonical project root, specification and request, even with equal display names',()=>{
  const key=decisionDraftKey(session,read,request),revision=decisionRevision(read,request);
  writeDecisionDraft(key,{revision,author:'Автор A',comment:'Приватный черновик A',status:'needs_changes' as const,uncertain:true,error:'Неоднозначный исход'});
  const keys=[
   decisionDraftKey({...session,project:{...session.project,root:'/project-b'}},read,request),
   decisionDraftKey(session,{...read,change:{...read.change,id:'HG-OTHER'}},request),
   decisionDraftKey(session,read,{...request,id:'request-b'}),
   decisionDraftKey(session,read,{...request,kind:'question'}),
  ];
  for(const other of keys){expect(other).not.toBe(key);expect(readDecisionDraft(other,revision)).toMatchObject({author:'',comment:'',status:'accepted',uncertain:false})}
  clearDecisionDraft(key);
 });
 test('keeps text, selected status and retry guard across target or input drift',()=>{
  const key=decisionDraftKey(session,read,request),revision=decisionRevision(read,request);
  const draft={revision,author:'Автор',comment:'Пояснение',status:'needs_changes' as const,uncertain:true,error:'Неоднозначный исход'};
  const revisions=[
   decisionRevision({...read,inputs_sha256:'inputs-b'},request),
   decisionRevision({...read,change_sha256:'change-b'},request),
   decisionRevision(read,{...request,content_sha256:'content-b'}),
   decisionRevision(read,{...request,change_sha256:'change-b'}),
   decisionRevision(read,{...request,inputs_sha256:'inputs-b'}),
   decisionRevision(read,{...request,at:2}),
   decisionRevision(read,{...request,kind:'requirements'}),
   decisionRevision(read,{...request,reason:'Другой вопрос'}),
   decisionRevision(read,{...request,verified_revision:'saved-build-b'}),
  ];
  for(const next of revisions){writeDecisionDraft(key,draft);expect(readDecisionDraft(key,next)).toMatchObject({revision:next,author:'Автор',comment:'Пояснение',status:'needs_changes',uncertain:true})}
  expect(decisionRevision({...read,store_sha256:'unrelated-store'},request)).toBe(revision);
  clearDecisionDraft(key);
 });
});

test.describe('HG61 S1 S2: decision dialog interrupted flows',()=>{
 test.setTimeout(90000);
 let f:ReturnType<typeof fixture>,url:string;
 test.beforeEach(async()=>{f=fixture();url=await f.start()});
 test.afterEach(async()=>{await f?.close()});
 async function select(page:Page,id:string){
  await page.goto(url);await page.getByRole('button',{name:'Все спецификации'}).click();
  await page.locator('.spec-row').filter({hasText:id}).click();
  await expect(page.locator('.document h1')).toHaveText(`Спецификация ${Number(id.slice(-1))}`);
 }
 async function open(page:Page){await page.locator('.decision').getByRole('button').click();await expect(page.getByRole('dialog')).toBeVisible()}
 async function close(page:Page){await page.getByRole('dialog').locator('[data-slot="dialog-footer"]').getByRole('button',{name:'Закрыть',exact:true}).click();await expect(page.getByRole('dialog')).toHaveCount(0)}
 async function fill(page:Page,comment='Не потерять пояснение'){
  if(await page.getByLabel('Ваше имя',{exact:true}).count())await page.getByLabel('Ваше имя',{exact:true}).fill('Автор черновика');
  if(await page.getByLabel('Статус результата',{exact:true}).count())await page.getByLabel('Статус результата',{exact:true}).selectOption('needs_changes');
  await page.getByLabel(/^(Комментарий|Ответ)$/).fill(comment);
 }
 async function warnsBeforeUnload(page:Page){return page.evaluate(()=>!window.dispatchEvent(new Event('beforeunload',{cancelable:true})))}

 test('Close, close icon, Escape and navigation preserve the same result draft without writes',async({page})=>{
  await select(page,'HG-0003');const before=f.hash();let writes=0;page.on('request',r=>{if(r.method()==='POST')writes++});
  await open(page);await fill(page);await expect(page.getByLabel('Версия проверенного результата',{exact:true})).toHaveCount(0);
  for(const dismiss of ['footer','escape','icon'] as const){
   if(dismiss==='footer')await close(page);
   else if(dismiss==='escape')await page.keyboard.press('Escape');
   else await page.getByRole('dialog').locator('[data-slot="dialog-close"]').click();
   await expect(page.getByRole('dialog')).toHaveCount(0);expect(await warnsBeforeUnload(page)).toBe(true);
   await open(page);await expect(page.getByLabel('Ваше имя',{exact:true})).toHaveCount(0);
   await expect(page.getByLabel('Комментарий',{exact:true})).toHaveValue('Не потерять пояснение');
   await expect(page.getByLabel('Статус результата',{exact:true})).toHaveValue('needs_changes');
  }
  await close(page);await page.locator('.spec-row').filter({hasText:'HG-0002'}).click();await open(page);
  await expect(page.getByLabel('Ваше имя',{exact:true})).toHaveValue('');await expect(page.getByLabel('Комментарий',{exact:true})).toHaveValue('');
  await close(page);await page.locator('.spec-row').filter({hasText:'HG-0003'}).click();await open(page);
  await expect(page.getByLabel('Комментарий',{exact:true})).toHaveValue('Не потерять пояснение');expect(writes).toBe(0);expect(f.hash()).toEqual(before);
 });

 test('a lost unwritten response remains blocked after dismissal until GET inspection and explicit retry',async({page})=>{
  await select(page,'HG-0001');await open(page);await fill(page,'Ответ после проверки');let writes=0;
  await page.route('**/api/specs/HG-0001/attention',async route=>{writes++;if(writes===1)await route.abort();else await route.fulfill({response:await route.fetch()})});
  await page.getByRole('button',{name:'Сохранить ответ',exact:true}).click();await expect(page.getByRole('alert')).toBeVisible();
  await close(page);await open(page);await expect(page.getByRole('button',{name:'Сохранить ответ',exact:true})).toBeDisabled();
  await expect(page.getByLabel('Ответ',{exact:true})).toHaveValue('Ответ после проверки');expect(writes).toBe(1);
  await page.getByRole('button',{name:'Проверить состояние решения'}).click();await expect(page.getByRole('alert')).toContainText('Ответ не найден');
  await expect(page.getByRole('button',{name:'Сохранить ответ',exact:true})).toBeEnabled();expect(writes).toBe(1);expect(f.row('HG-0001').history[0].response).toBeNull();
  await page.getByRole('button',{name:'Сохранить ответ',exact:true}).click();await expect(page.getByRole('dialog')).toHaveCount(0);
  expect(writes).toBe(2);expect(f.row('HG-0001').history[0].response.comment).toBe('Ответ после проверки');expect(await warnsBeforeUnload(page)).toBe(false);
 });

 test('a lost committed response and a failed inspection cannot reset the retry guard',async({page})=>{
  await select(page,'HG-0001');await open(page);await fill(page,'Ответ записан один раз');let writes=0;
  await page.route('**/api/specs/HG-0001/attention',async route=>{writes++;await route.fetch();await route.abort()});
  await page.getByRole('button',{name:'Сохранить ответ',exact:true}).click();await expect(page.getByRole('alert')).toBeVisible();
  await page.route('**/api/specs/HG-0001',route=>route.abort());
  await page.getByRole('button',{name:'Проверить состояние решения'}).click();await expect(page.getByRole('alert')).toContainText('Связь с проектом потеряна');
  await close(page);await open(page);await expect(page.getByRole('button',{name:'Сохранить ответ',exact:true})).toBeDisabled();
  await expect(page.getByLabel('Ответ',{exact:true})).toHaveValue('Ответ записан один раз');await page.unroute('**/api/specs/HG-0001');
  await page.getByRole('button',{name:'Проверить состояние решения'}).click();await expect(page.getByRole('alert')).toContainText('В проекте записано решение');
  await close(page);await open(page);await expect(page.getByRole('button',{name:'Сохранить ответ',exact:true})).toBeDisabled();
  expect(writes).toBe(1);expect(f.row('HG-0001').history.filter((r:Handoff)=>r.response)).toHaveLength(1);
 });

 test('HG70 S3: a stale result preserves status and text without posting again',async({page})=>{
  await select(page,'HG-0003');const before=f.hash();await open(page);await fill(page,'Комментарий после изменения входов');
  await expect(page.getByLabel('Версия проверенного результата',{exact:true})).toHaveCount(0);let writes=0;
  page.on('request',r=>{if(r.method()==='POST')writes++});
  writeFileSync(path.join(f.root,'proof.txt'),'Inputs changed after opening the result');
  await page.getByRole('button',{name:'Сохранить статус',exact:true}).click();await expect(page.getByRole('alert')).toContainText('DecisionStale');
  await close(page);await open(page);await expect(page.getByRole('button',{name:'Сохранить статус',exact:true})).toBeDisabled();
  await expect(page.getByLabel('Комментарий',{exact:true})).toHaveValue('Комментарий после изменения входов');
  await page.getByRole('button',{name:'Проверить состояние решения'}).click();await expect(page.getByRole('alert')).toContainText('Редакция обращения изменилась');
  await expect(page.getByLabel('Версия проверенного результата',{exact:true})).toHaveCount(0);
  await close(page);await open(page);await expect(page.getByLabel('Версия проверенного результата',{exact:true})).toHaveCount(0);
  await expect(page.getByLabel('Ваше имя',{exact:true})).toHaveCount(0);await expect(page.getByRole('button',{name:'Сохранить статус',exact:true})).toBeDisabled();
  expect(writes).toBe(1);expect(f.hash()).toEqual(before);expect(f.cli('spec-read','--id','HG-0003').change.acceptance).toHaveLength(0);
 });

 test('HG70 S3: lost result response preserves selected status and records acceptance once',async({page})=>{
  await select(page,'HG-0003');await open(page);await page.getByLabel('Статус результата',{exact:true}).selectOption('accepted');let writes=0;
  await page.route('**/api/specs/HG-0003/attention',async route=>{writes++;await route.fetch();await route.abort()});
  await page.getByRole('button',{name:'Сохранить статус',exact:true}).click();await expect(page.getByRole('alert')).toBeVisible();
  await close(page);await open(page);await expect(page.getByLabel('Статус результата',{exact:true})).toHaveValue('accepted');await expect(page.getByRole('button',{name:'Сохранить статус',exact:true})).toBeDisabled();
  await page.getByRole('button',{name:'Проверить состояние решения'}).click();await expect(page.getByRole('alert')).toContainText('В проекте записано решение');
  expect(writes).toBe(1);expect(f.cli('spec-read','--id','HG-0003').change.acceptance).toHaveLength(1);await expect(page.getByRole('button',{name:'Сохранить статус',exact:true})).toBeDisabled();
 });

 test('a pending write blocks repeated actions and clears only its own draft after confirmation',async({page})=>{
  f.handoff('HG-0001','question','question-2');await select(page,'HG-0001');await open(page);await fill(page,'Первый ответ');
  let release!:()=>void,reached!:()=>void,writes=0;
  const waiting=new Promise<void>(resolve=>release=resolve),received=new Promise<void>(resolve=>reached=resolve);
  await page.route('**/api/specs/HG-0001/attention',async route=>{writes++;reached();await waiting;await route.fulfill({response:await route.fetch()})});
  await page.getByRole('button',{name:'Сохранить ответ',exact:true}).click();await received;
  try{
   await expect(page.getByRole('button',{name:'Сохранить ответ',exact:true})).toBeDisabled();await expect(page.getByLabel('Ответ',{exact:true})).toBeDisabled();
   await page.keyboard.press('Escape');await expect(page.getByRole('dialog')).toBeVisible();
   await page.getByRole('dialog').locator('[data-slot="dialog-close"]').click();await expect(page.getByRole('dialog')).toBeVisible();expect(writes).toBe(1);
  }finally{release()}
  await expect(page.getByRole('dialog')).toHaveCount(0);await open(page);
  await expect(page.getByLabel('Ваше имя',{exact:true})).toHaveValue('');await expect(page.getByLabel('Ответ',{exact:true})).toHaveValue('');
  expect(f.row('HG-0001').requests).toHaveLength(1);expect(writes).toBe(1);
 });
});
