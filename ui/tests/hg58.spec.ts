import {test,expect} from '@playwright/test';
import {fixture} from './native-fixture';
import path from 'node:path';
const out=path.resolve('../target/highgrade/tmp/hg58');
test('HG58 typography operations and compact reading at desktop mobile and reduced motion',async({page})=>{
 const f=fixture();try{
  const c=f.cli('spec-read','--id','HG-0005').change;
  const existing=f.cli('spec-read','--id','HG-0003').change.operations[0].requirement;
  f.edit('HG-0005',{operations:[...c.operations,{action:'modify',requirement:{...existing,title:'Изменяемое требование',statement:'Ссылка HG-0003-R1 сохранена'}},{action:'remove',id:'HG-0004-R1',reason:'Удаляем прежнее правило'}]});
  const before=f.hash();await page.goto(await f.start());await page.getByRole('button',{name:'Все спецификации'}).click();await page.locator('.spec-row').filter({hasText:'HG-0005'}).click();await expect(page.locator('.document h1')).toHaveText('Спецификация 5');
  for(const name of ['Добавить','Изменить','Удалить'])await expect(page.locator('.document').getByRole('img',{name,exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Изменяемое требование Изменить',exact:true}).click();await expect(page.getByText('Ссылка HG-0003-R1 сохранена',{exact:true})).toBeVisible();
  for(const [width,height] of [[1440,1000],[390,844],[720,450]]){
   await page.setViewportSize({width,height});await page.evaluate(()=>document.fonts.ready);
   expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBeTruthy();
   expect(await page.evaluate(()=>[...document.fonts].some(f=>f.family==='Manrope'&&f.status==='loaded'))).toBeTruthy();
   await page.screenshot({path:path.join(out,`native-${width}.png`),fullPage:true});
  }
  await page.emulateMedia({reducedMotion:'reduce'});
  expect(await page.locator('.spec-changes [data-slot="accordion-trigger"]').first().evaluate(e=>getComputedStyle(e).transitionDuration)).toBe('0s');
  await page.getByRole('button',{name:'Редактировать',exact:true}).click();await expect(page.getByLabel('Цель',{exact:true})).toBeFocused();
  const focus=await page.getByLabel('Цель',{exact:true}).evaluate(e=>({outline:getComputedStyle(e).outlineStyle,width:getComputedStyle(e).outlineWidth,shadow:getComputedStyle(e).boxShadow}));expect(focus.outline).toBe('solid');expect(focus.width).toBe('2px');expect(focus.shadow).toBe('none');expect(f.hash()).toEqual(before);
 }finally{await f.close()}
});
