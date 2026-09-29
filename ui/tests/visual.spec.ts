import {test,expect} from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import {readFileSync} from 'node:fs';
const screen=(name:string)=>`/iframe.html?id=${encodeURIComponent('спецификации-рабочее-пространство--'+name)}&viewMode=story`;
test('reading, font and real visual reference',async({page})=>{
 const errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));const external:string[]=[];page.on('request',r=>{if(!r.url().startsWith('http://127.0.0.1:6006')&&!r.url().startsWith('data:'))external.push(r.url())});
 await page.goto(screen('approval'));await expect(page.getByRole('heading',{name:'Интерфейс спецификаций',exact:true})).toBeVisible();await page.evaluate(()=>document.fonts.ready);expect(await page.locator('.document').evaluate(e=>!!(e.querySelector('.eyebrow')!.compareDocumentPosition(e.querySelector('h1')!) & Node.DOCUMENT_POSITION_FOLLOWING))).toBeTruthy();
 expect(await page.evaluate(()=>document.fonts.check('16px Manrope','Требования HighGrade'))).toBeTruthy();
 expect(await page.evaluate(()=>[...document.fonts].some(f=>f.family==='Manrope'&&f.status==='loaded'))).toBeTruthy();
 await page.screenshot({path:'../target/highgrade/tmp/hg58/desktop.png',fullPage:true});
 expect((await new AxeBuilder({page}).include('.workspace').analyze()).violations).toEqual([]);
 expect(external).toEqual([]);expect(errors).toEqual([]);
});
test('editing, validation, discard and decision dialog are keyboard usable',async({page})=>{
 const writes:string[]=[];page.on('request',r=>{if(r.method()!=='GET')writes.push(r.url())});
 await page.goto(screen('approval'));await page.getByRole('button',{name:'Редактировать',exact:true}).click();
 await expect(page.getByLabel('Цель',{exact:true})).toBeFocused();await page.getByLabel('Цель',{exact:true}).fill('');await page.getByRole('button',{name:'Сохранить изменения'}).click();await expect(page.getByRole('alert')).toContainText('не может быть пустым');
 await page.getByLabel('Цель',{exact:true}).fill('Мой несохранённый текст');await page.getByRole('button',{name:'Отмена',exact:true}).click();await expect(page.getByRole('dialog')).toBeVisible();await page.getByRole('button',{name:'Продолжить редактирование'}).click();await expect(page.getByLabel('Цель',{exact:true})).toHaveValue('Мой несохранённый текст');
 await page.getByRole('button',{name:'Отмена',exact:true}).click();await page.getByRole('button',{name:'Отменить правки',exact:true}).click();await expect(page.getByRole('button',{name:'Редактировать',exact:true})).toBeFocused();
 await page.getByRole('button',{name:'Согласовать требования',exact:true}).click();await expect(page.getByRole('dialog')).toContainText('не разрешение выполнять работу');
 expect((await new AxeBuilder({page}).include('[role="dialog"]').analyze()).violations).toEqual([]);await page.keyboard.press('Escape');await expect(page.getByRole('dialog')).not.toBeVisible();expect(writes).toEqual([]);
});
test('narrow and 200 percent layout preserve controls and focus',async({page})=>{
 await page.setViewportSize({width:720,height:900});await page.goto(screen('long'));await page.getByRole('button',{name:'Открыть список спецификаций'}).click();await page.getByLabel('Поиск по спецификациям').fill('zzzz');await expect(page.getByRole('status')).toContainText('Ничего не найдено');await page.getByRole('button',{name:'Открыть список спецификаций'}).click();
 await expect(page.locator('.mobile-context')).toContainText('Демонстрационные данные');await expect(page.locator('.mobile-context')).toContainText('D:\\my_projects\\MyCodex');
 expect(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth)).toBeTruthy();await page.screenshot({path:'../target/highgrade/tmp/hg58/narrow.png',fullPage:true});
 // Browser zoom 200% has half the CSS viewport at the same physical pixel size.
 const cdp=await page.context().newCDPSession(page);await cdp.send('Emulation.setDeviceMetricsOverride',{width:720,height:450,deviceScaleFactor:2,mobile:false});await page.goto(screen('approval'));
 expect(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth)).toBeTruthy();await page.getByRole('button',{name:'Редактировать',exact:true}).click();await expect(page.getByLabel('Цель',{exact:true})).toBeFocused();await page.screenshot({path:'../target/highgrade/tmp/hg58/zoom-200.png',fullPage:true});
});
test('selecting a specification changes its content and required action',async({page})=>{
 await page.goto(screen('approval'));
 await page.locator('.spec-row').filter({hasText:'HG-DEMO-02'}).click();
 await expect(page.getByRole('button',{name:'Принять результат',exact:true})).toBeVisible();
 await expect(page.getByRole('heading',{name:'Готово, когда',exact:true})).toBeVisible();
 const secondGoal=await page.locator('.section').first().textContent();
 await page.locator('.spec-row').filter({hasText:'HG-DEMO-03'}).click();
 await expect(page.getByRole('button',{name:'Ответить на вопрос',exact:true})).toBeVisible();
 expect(await page.locator('.section').first().textContent()).not.toEqual(secondGoal);
 await page.getByRole('button',{name:'Завершены'}).click();await page.locator('.spec-row').filter({hasText:'HG-DEMO-05'}).click();
 await expect(page.getByRole('button',{name:'Только чтение',exact:true})).toBeDisabled();
 await expect(page.getByRole('region',{name:'Ожидаемое решение'})).toHaveCount(0);
});
test('every documented state renders without accessibility violations or backend writes',async({page,request})=>{
 const index=await request.get('/index.json');const entries=Object.values((await index.json()).entries) as {id:string;type:string}[];
 expect(entries.filter(e=>e.type==='story').length).toBeGreaterThanOrEqual(25);
 const writes:string[]=[];page.on('request',r=>{if(r.method()!=='GET')writes.push(r.url())});
 for(const entry of entries.filter(e=>e.type==='story')) {
  await page.goto(`/iframe.html?id=${encodeURIComponent(entry.id)}&viewMode=story`);await page.locator('#storybook-root').waitFor();await expect(page.locator('#storybook-root > *').first(),entry.id).toBeVisible();
  const violations=(await new AxeBuilder({page}).include('#storybook-root').analyze()).violations;
  expect(violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>n.target)})),entry.id).toEqual([]);
  for(const name of ['editing','conflict','icons','brand'])if(entry.id.endsWith('--'+name))await page.screenshot({path:`../target/highgrade/tmp/hg58/${name}.png`,fullPage:true});
 }
 expect(writes).toEqual([]);
});
test('brand is vector-only and adapted icons render at their real sizes',async({page})=>{
 for(const file of ['mark','wordmark','mark-mono','wordmark-mono']){const svg=readFileSync(`src/assets/brand/${file}.svg`,'utf8');expect(svg).not.toMatch(/<(?:image|script|foreignObject)\b|data:image/);expect(svg).toContain('<path');}
 await page.goto('/iframe.html?id='+encodeURIComponent('основы-визуальный-язык--icons')+'&viewMode=story');
 for(const size of [16,20,24]){const icons=page.locator(`.icon-examples svg[width="${size}"]`);await expect(icons).toHaveCount(4);for(const icon of await icons.all()){await expect(icon).toHaveAttribute('stroke-width','1.75');await expect(icon).toHaveAttribute('aria-hidden','true')}}
});
test('catalog manager navigation and component controls work',async({page})=>{
 await page.goto('/?path=/story/'+encodeURIComponent('компоненты-кнопка--primary'));
 await expect(page.getByRole('button',{name:'Controls'})).toBeVisible();
 const control=page.getByRole('row').filter({has:page.getByText('children',{exact:true})}).getByRole('textbox');await expect(control).toBeVisible();await control.fill('Проверить требования');await expect(page.frameLocator('#storybook-preview-iframe').getByRole('button',{name:'Проверить требования'})).toBeVisible();
 await page.screenshot({path:'../target/highgrade/tmp/hg58/storybook.png',fullPage:true});
});
