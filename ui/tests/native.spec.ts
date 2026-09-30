import {test,expect} from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import {fixture} from './native-fixture';
import {readFileSync,writeFileSync,unlinkSync} from 'node:fs';
import path from 'node:path';
test.setTimeout(90000);
let f:ReturnType<typeof fixture>;let url:string;
test.beforeEach(async()=>{f=fixture();url=await f.start()});test.afterEach(async()=>{await f?.close()});
test('HG55 S1 S2: real catalog filters counts and empty states preserve files',async({page})=>{
 const dated=path.join(f.root,'specs/changes/HG-0005/spec.json');const datedSpec=JSON.parse(readFileSync(dated,'utf8'));datedSpec.created_at+=3600;writeFileSync(dated,JSON.stringify(datedSpec));
 const before=f.hash();await page.goto(url);await expect(page.locator('.spec-row')).toHaveCount(3);
 await page.getByLabel('Тема',{exact:true}).selectOption('interface');await expect(page.locator('.spec-row')).toHaveCount(1);await expect(page.locator('.spec-row')).toContainText('HG-0002');
 await page.getByLabel('Поиск по спецификациям').fill('absent');await expect(page.getByRole('status')).toContainText('Ничего не найдено');await page.getByRole('button',{name:'Сбросить фильтры'}).click();
 await page.getByRole('button',{name:'Все спецификации'}).click();await expect(page.locator('.spec-row')).toHaveCount(5);await page.getByLabel('Показать отменённые').check();await expect(page.locator('.spec-row')).toHaveCount(6);await page.locator('.spec-row').filter({hasText:'HG-0007'}).click();await expect(page.locator('.document .eyebrow')).toContainText('Отменена');await page.getByLabel('Порядок').selectOption('date');await expect(page.locator('.spec-row').first()).toContainText('HG-0005');
 await page.getByRole('button',{name:'Завершены'}).click();await expect(page.locator('.spec-row')).toContainText('HG-0004');await expect(page.getByText('Прежний формат: 1')).toBeVisible();expect(f.hash()).toEqual(before);
 for(const id of ['HG-0001','HG-0002','HG-0003']){const r=f.row(id);f.attention(id,{action:'respond',request_id:r.primary_action.id,content_sha256:r.content_sha256,decision:r.primary_action.kind==='question'?'answer':'accepted',author:'Test user',comment:'Ответ теста',verified_revision:r.primary_action.change_sha256})}
 await page.evaluate(()=>window.dispatchEvent(new Event('focus')));await page.getByRole('button',{name:'Нужно решение'}).click();await expect(page.getByRole('status').filter({hasText:'Сейчас решений не требуется'})).toBeVisible();
});
test('HG59 S3 S4: integrated history stays visible without a false work selection',async({page})=>{
 for(const id of ['HG-0001','HG-0002'])f.cli('spec-abandon','--id',id,'--expected',f.sha(),'--reason','Fixture without current work');
 f.edit('HG-0005',{tags:['interface']});f.ready('HG-0005');f.handoff('HG-0005','result');
 const needsChanges=f.row('HG-0005');f.attention('HG-0005',{action:'respond',request_id:'result',content_sha256:needsChanges.content_sha256,decision:'needs_changes',author:'Fixture user',comment:'Изменить результат',verified_revision:'fixture-build-v1'});
 writeFileSync(path.join(f.root,'proof.txt'),'Evidence changed after the observed run');
 expect(f.cli('spec-new','--title','Недоступная история','--expected',f.sha()).change.id).toBe('HG-0008');
 f.edit('HG-0008',{goal:'История',rationale:'Fixture',scope:'Fixture',tasks:[{id:'HG-0008-T1',description:'Работа',done:true}],operations:[{action:'add',requirement:{id:'HG-0008-R1',title:'Условие',statement:'Условие',scenarios:[{id:'HG-0008-S1',given:'Дано',when:'Действие',then:'Результат',verification:'Наблюдение'}]}}]});
 const source=path.join(f.root,'history-proof.txt'),report=path.join(f.root,'history-report.txt'),input=path.join(f.root,'history-input.json');
 writeFileSync(source,'Observed fixture');writeFileSync(report,'Observed fixture');writeFileSync(input,JSON.stringify({command:'isolated fixture observation',captured_at:'2026-09-29T00:00:00Z',method:'manual',scenario:'HG-0008-S1',outcome:'passed',observation:'fixture satisfied',inputs:['history-proof.txt'],report:'history-report.txt'}));
 f.cli('spec-evidence','--id','HG-0008','--expected',f.sha(),'--input','history-input.json');
 f.cli('spec-review','--id','HG-0008','--expected',f.sha(),'--reviewer','fixture','--verdict','go','--conclusion','Fixture contract satisfied');
 f.cli('spec-integrate','--id','HG-0008','--expected',f.sha());unlinkSync(source);
 expect(f.row('HG-0004').category).toBe('historical');expect(f.row('HG-0005').category).toBe('historical');expect(f.row('HG-0008').category).toBe('unknown');
 const before=f.hash();await page.goto(url);
 await expect(page.getByRole('button',{name:'Все спецификации'})).toHaveAttribute('aria-pressed','true');
 await expect(page.locator('.spec-row[aria-current="true"]')).toHaveCount(1);
 await page.getByRole('button',{name:'В работе'}).click();await expect(page.locator('.spec-row')).toHaveCount(0);await expect(page.locator('.document')).toHaveCount(0);
 await expect(page.locator('.empty-state')).toContainText('В этом разделе пока нет спецификаций');
 await page.getByRole('button',{name:'Все спецификации'}).click();await page.getByLabel('Поиск по спецификациям').fill('HG-0004');
 await expect(page.locator('.spec-row')).toHaveCount(1);await page.locator('.spec-row').click();
 await expect(page.locator('.document .eyebrow .status-badge')).toHaveText('Интегрировано');
 await expect(page.locator('.document .status-context')).toContainText('текущая проверка устарела');
 await expect(page.locator('.document .acceptance-context')).toContainText('Прежнее решение: результат принят. Сейчас это решение неактуально.');
 await page.getByLabel('Поиск по спецификациям').fill('');await page.getByLabel('Тема',{exact:true}).selectOption('interface');
 await expect(page.locator('.spec-row')).toHaveCount(1);await expect(page.locator('.spec-row')).toContainText('HG-0005');
 await page.locator('.spec-row').click();await expect(page.locator('.document .acceptance-context')).toContainText('Прежнее решение: требуются изменения. Сейчас это решение неактуально.');
 await page.getByLabel('Тема',{exact:true}).selectOption('');await page.locator('.spec-row').filter({hasText:'HG-0008'}).click();
 await expect(page.locator('.document .eyebrow .status-badge')).toHaveText('Не удалось проверить');
 await expect(page.locator('.document .status-context')).toContainText('входы текущей проверки недоступны');
 await expect(page.locator('.document .acceptance-context')).toHaveText('Прежней приёмки результата нет.');
 expect(f.hash()).toEqual(before);
});
test('HG55 S3 S4: full source operations and hostile text remain inert',async({page})=>{
 const c=f.cli('spec-read','--id','HG-0005').change;const hostile='<script>window.specAttack=true</script><img src="https://tracker.invalid/pixel"> javascript:alert(1)';
 f.edit('HG-0005',{rationale:'',scope:hostile,questions:['Незаполненный вопрос'],depends_on:[{id:'HG-0003',reason:'Основание зависимости'}],operations:[...c.operations,{action:'remove',id:'HG-0003-R1',reason:'Убираем прежнее правило с исключением X'}]});const before=f.hash();const external:string[]=[];page.on('request',r=>{if(!r.url().startsWith(url))external.push(r.url())});
 await page.goto(url);await page.getByRole('button',{name:'В работе'}).click();await page.locator('.spec-row').filter({hasText:'HG-0005'}).click();await expect(page.locator('.document')).toContainText(hostile);await expect(page.locator('.document')).toContainText('Убираем прежнее правило с исключением X');await expect(page.getByRole('heading',{name:'Цель',exact:true})).toHaveCount(1);await expect(page.getByRole('button',{name:'Критерии готовности и сценарии'})).toHaveCount(0);await expect(page.getByRole('button',{name:'Зависимости и связи'})).toHaveCount(0);await expect(page.getByRole('button',{name:'История и диагностика'})).toHaveCount(0);await expect(page.locator('.document')).not.toContainText(/HG-\d+-[RST]\d+/);expect(await page.evaluate(()=>Reflect.get(window,'specAttack'))).toBeUndefined();expect(external).toEqual([]);expect(f.hash()).toEqual(before);
 expect((await new AxeBuilder({page}).include('.workspace').analyze()).violations).toEqual([]);
});
test('HG55 S5: one actual handoff action or quiet reading',async({page})=>{
 await page.route('**/api/specs',async route=>{const response=await route.fetch();const body=await response.json();const list=body.result??body;const row=list.changes.find((r:any)=>r.id==='HG-0003');row.primary_action.reason='Результат готов: проверьте интерфейс и примите либо верните на доработку. Проверки и версия — docs/evidence/specification-ui/handoff.md.';await route.fulfill({response,json:body})});
 await page.goto(url);for(const [id,name] of [['HG-0001','Ответить на вопрос'],['HG-0002','Согласовать требования'],['HG-0003','Принять результат']]){await page.locator('.spec-row').filter({hasText:id}).click();await expect(page.locator('.decision').getByRole('button',{name,exact:true})).toBeEnabled();await expect(page.locator('.decision button')).toHaveCount(1)}
 await expect(page.locator('.document .eyebrow .status-badge')).toHaveCount(0);await expect(page.locator('.spec-row[aria-current="true"] .status-badge')).toHaveText('Нужно решение');await expect(page.locator('.decision')).not.toContainText('handoff.md');await page.getByRole('button',{name:'Принять результат',exact:true}).click();await expect(page.getByRole('dialog')).not.toContainText('handoff.md');await expect(page.getByLabel('Версия проверенного результата',{exact:true})).toBeVisible();expect((await page.getByRole('dialog').boundingBox())!.height).toBeLessThan(520);await page.keyboard.press('Escape');await expect(page.getByRole('dialog')).toHaveCount(0);
 await expect(page.getByRole('button',{name:'Результаты проверки',exact:true})).not.toBeVisible();await expect(page.getByRole('button',{name:'Результаты проверки',exact:true})).toHaveCount(0);await expect(page.getByRole('button',{name:'Задачи',exact:true})).toHaveCount(0);await expect(page.getByText('Объявленные проверки',{exact:true})).toHaveCount(0);await expect(page.getByRole('heading',{name:'Запуски',exact:true})).toHaveCount(0);await page.getByRole('button',{name:'В работе'}).click();await page.locator('.spec-row').filter({hasText:'HG-0005'}).click();await expect(page.locator('.decision')).toHaveCount(0);await page.screenshot({path:test.info().outputPath('reading.png'),fullPage:true});
});
test('HG55 S6: context restored and external CLI edit requires explicit refresh',async({page})=>{
 f.edit('HG-0005',{scope:'Длинная область\n'.repeat(80)});await page.goto(url);await page.getByRole('button',{name:'Все спецификации'}).click();await page.locator('.spec-row').filter({hasText:'HG-0005'}).click();await expect(page.locator('.document h1')).toHaveText('Спецификация 5');await page.getByRole('button',{name:'Требование 5 Добавить',exact:true}).click();await page.evaluate(()=>window.scrollTo(0,600));await page.locator('.spec-row').filter({hasText:'HG-0002'}).click();await expect(page.locator('.document h1')).toHaveText('Спецификация 2');await page.locator('.spec-row').filter({hasText:'HG-0005'}).click();await expect(page.locator('.document h1')).toHaveText('Спецификация 5');await expect(page.getByRole('button',{name:'Требование 5 Добавить',exact:true})).toHaveAttribute('aria-expanded','true');await expect.poll(()=>page.evaluate(()=>window.scrollY)).toBeGreaterThan(400);
 f.edit('HG-0002',{goal:'Независимая правка'});const refreshed=page.waitForResponse(r=>r.url().endsWith('/api/specs/HG-0005'));await page.evaluate(()=>window.dispatchEvent(new Event('focus')));await refreshed;await expect(page.getByRole('status')).toHaveCount(0);await page.getByRole('button',{name:'Обновить',exact:true}).click();await expect(page.getByRole('button',{name:'Обновить',exact:true})).toBeEnabled();await expect(page.getByRole('button',{name:'Требование 5 Добавить',exact:true})).toHaveAttribute('aria-expanded','true');
 f.edit('HG-0005',{goal:'Внешняя редакция B'});await page.evaluate(()=>window.dispatchEvent(new Event('focus')));await expect(page.getByRole('status')).toContainText('новая редакция');await expect(page.locator('.document')).not.toContainText('Внешняя редакция B');await page.getByRole('button',{name:'Загрузить новую редакцию'}).click();await expect(page.locator('.document')).toContainText('Внешняя редакция B');
});
test('HG55 S7: unavailable API preserves visible document and retries',async({page})=>{
 await page.goto(url);await expect(page.locator('.document h1')).toBeVisible();await page.route('**/api/specs',route=>route.abort());await page.evaluate(()=>window.dispatchEvent(new Event('focus')));await expect(page.getByRole('alert')).toContainText('Связь с проектом потеряна');await expect(page.locator('.document h1')).toBeVisible();await page.unroute('**/api/specs');await page.getByRole('button',{name:'Повторить чтение'}).click();await expect(page.getByRole('alert')).toHaveCount(0);
});

test('HG55 S7: slow first snapshot is reused and temporary StoreBusy does not fail startup',async({page})=>{
 const before=f.hash();let busy=0,completedLists=0;const start=Date.now();
 await page.route('**/api/specs',async route=>{
  if(Date.now()-start<1800){busy++;await route.fulfill({status:422,contentType:'application/json',body:JSON.stringify({error:{code:'StoreBusy',message:'Controlled temporary catalog lock'}})});return}
  completedLists++;await new Promise(resolve=>setTimeout(resolve,11000));await route.fulfill({response:await route.fetch()});
 });
 await page.goto(url);await expect(page.locator('.document h1')).toHaveText('Спецификация 1',{timeout:25000});
 expect(busy).toBeGreaterThan(3);expect(completedLists).toBe(1);await expect(page.getByRole('alert')).toHaveCount(0);expect(f.hash()).toEqual(before);
});

test('HG55 S3 S6: compact source view and rapid selection show the last chosen spec',async({page})=>{
 const source=f.cli('spec-read','--id','HG-0001').change;const requirement=source.operations[0].requirement;f.edit('HG-0001',{scope:'Длинная область\n'.repeat(80),operations:[{action:'add',requirement:{...requirement,scenarios:[...requirement.scenarios,...[2,3,4].map(i=>({...requirement.scenarios[0],id:`HG-0001-S${i}`,then:`Дополнительное условие ${i}`}))]}}]});
 const before=f.hash();const errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto(url);await page.getByRole('button',{name:'Все спецификации'}).click();await page.locator('.spec-row').filter({hasText:'HG-0001'}).click();await expect(page.locator('.document h1')).toHaveText('Спецификация 1');
 await expect(page.getByRole('heading',{name:'Цель',exact:true})).toBeVisible();await expect(page.getByText('Нажмите на пункт, чтобы прочитать полное условие.')).toHaveCount(0);await expect(page.locator('.document .eyebrow .status-badge')).toBeVisible();
 await expect(page.getByRole('heading',{name:'Готово, когда',exact:true})).toBeVisible();
 await expect(page.getByText('Дополнительное условие 4',{exact:true})).toBeVisible();
 await expect(page.getByRole('button',{name:'Задачи',exact:true})).toHaveCount(0);await expect(page.getByRole('button',{name:'Подробности',exact:true})).toHaveCount(0);
 await expect(page.getByText('Условие 1',{exact:true})).not.toBeVisible();
 await expect(page.getByRole('button',{name:'Требование 1 Добавить',exact:true}).getByRole('img',{name:'Добавить',exact:true})).toBeVisible();await page.getByRole('button',{name:'Требование 1 Добавить',exact:true}).click();await expect(page.getByText('Условие 1',{exact:true})).toBeVisible();
 await page.evaluate(()=>window.scrollTo(0,500));await expect.poll(()=>page.evaluate(()=>window.scrollY)).toBe(500);
 let lists=0;page.on('request',r=>{if(r.url().endsWith('/api/specs'))lists++});
 let release!:()=>void;const blocked=new Promise<void>(resolve=>{release=resolve});
 await page.route('**/api/specs/HG-0005',async route=>{await blocked;await route.fulfill({response:await route.fetch()})});
 try{
  await page.locator('.spec-row').filter({hasText:'HG-0005'}).click();
  await expect(page.getByRole('status')).toHaveText('Открываем HG-0005…');
  await expect(page.locator('.spec-row[aria-current="true"]')).toContainText('HG-0005');
  await expect(page.locator('.decision')).toHaveCount(0);
  await page.locator('.spec-row').filter({hasText:'HG-0003'}).click();
  await expect(page.getByRole('status')).toHaveText('Открываем HG-0003…');
 }finally{release()}
 await expect(page.locator('.document h1')).toHaveText('Спецификация 3');
 await expect(page.locator('.spec-row[aria-current="true"]')).toContainText('HG-0003');
 expect(lists).toBe(0);
 await page.locator('.spec-row').filter({hasText:'HG-0001'}).click();
 await expect(page.locator('.document h1')).toHaveText('Спецификация 1');
 await expect(page.getByRole('button',{name:'Требование 1 Добавить',exact:true})).toHaveAttribute('aria-expanded','true');
 await expect.poll(()=>page.evaluate(()=>window.scrollY)).toBeGreaterThan(400);
 await expect(page.getByRole('status')).toHaveCount(0);expect(errors).toEqual([]);expect(f.hash()).toEqual(before);
});

test('Latest selection does not wait for an obsolete read',async({page})=>{
 await page.goto(url);await expect(page.getByRole('button',{name:'Обновить',exact:true})).toBeEnabled();
 await page.getByRole('button',{name:'Все спецификации'}).click();
 let reached!:()=>void,release!:()=>void;
 const started=new Promise<void>(resolve=>reached=resolve);
 const blocked=new Promise<void>(resolve=>release=resolve);
 await page.route('**/api/specs/HG-0005',async route=>{reached();await blocked;try{await route.abort()}catch{}});
 try{
  await page.locator('.spec-row').filter({hasText:'HG-0005'}).click();await started;
  await page.locator('.spec-row').filter({hasText:'HG-0002'}).click();
  // The previous response is deliberately never released before this assertion.
  await expect(page.locator('.document h1')).toHaveText('Спецификация 2',{timeout:5000});
  await expect(page.getByRole('alert')).toHaveCount(0);
 }finally{release()}
});
