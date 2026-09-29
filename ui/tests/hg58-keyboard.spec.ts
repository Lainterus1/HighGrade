import {test,expect} from '@playwright/test';
test('HG58 keyboard navigation icon states and Cyrillic weights at 100 and 200 percent',async({page})=>{
 const external:string[]=[];page.on('request',r=>{if(!r.url().startsWith('http://127.0.0.1:6006')&&!r.url().startsWith('data:'))external.push(r.url())});
 await page.goto('/iframe.html?id='+encodeURIComponent('спецификации-рабочее-пространство--approval')+'&viewMode=story');
 const nav=page.locator('.nav button');await expect(nav).toHaveCount(4);
 for(let i=0;i<4;i++){
  for(let tries=0;tries<30 && !(await nav.nth(i).evaluate(e=>e===document.activeElement));tries++)await page.keyboard.press('Tab');
  await expect(nav.nth(i)).toBeFocused();expect(await nav.nth(i).evaluate(e=>e.matches(':focus-visible'))).toBeTruthy();
  expect(await nav.nth(i).evaluate(e=>getComputedStyle(e).outlineStyle)).not.toBe('none');
  await page.keyboard.press('Enter');await expect(nav.nth(i)).toHaveAttribute('aria-pressed','true');
  await expect(nav.nth(i).locator('svg')).toHaveAttribute('aria-hidden','true');
  const other=nav.nth((i+1)%4);await expect(other).toHaveAttribute('aria-pressed','false');
  expect(await nav.nth(i).evaluate(e=>getComputedStyle(e).backgroundColor)).not.toBe(await other.evaluate(e=>getComputedStyle(e).backgroundColor));
  await page.screenshot({path:`../target/highgrade/tmp/hg58/keyboard-${i}.png`});
 }
 await page.goto('/iframe.html?id='+encodeURIComponent('основы-визуальный-язык--typography')+'&viewMode=story');
 // A specimen rendered with the actual loaded application font, not a replacement font.
 await page.locator('.token-card').evaluate(el=>{el.replaceChildren();for(const weight of [400,550,700]){const p=document.createElement('p');p.textContent='Ёж, щука, съёмка — HighGrade 0123456789';p.style.cssText=`font-weight:${weight};font-size:20px;margin:20px 0`;el.append(p)}});
 for(const scale of [1,2]){
  await page.setViewportSize({width:1440/scale,height:900/scale});await page.evaluate(()=>document.fonts.ready);
  for(const weight of [400,550,700])expect(await page.evaluate(w=>document.fonts.check(`${w} 20px Manrope`,'Ёж, щука, съёмка'),weight)).toBeTruthy();
  expect(await page.evaluate(()=>[...document.fonts].some(f=>f.family==='Manrope'&&f.status==='loaded'))).toBeTruthy();
  expect(await page.locator('.token-card p').evaluateAll(es=>es.map(e=>getComputedStyle(e).fontWeight))).toEqual(['400','550','700']);
  expect(await page.locator('.token-card p').first().evaluate(e=>getComputedStyle(e).fontFamily)).toContain('Manrope');
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBeTruthy();
  await page.screenshot({path:`../target/highgrade/tmp/hg58/cyrillic-${scale}.png`,fullPage:true});
 }
 expect(external).toEqual([]);
});
