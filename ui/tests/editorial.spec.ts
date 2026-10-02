import {test,expect} from '@playwright/test';
import {fixture} from './native-fixture';

test('Editorial local fonts render in Storybook and the real UI without external requests',async({page,request})=>{
 const f=fixture();
 try {
  const native=await f.start();
  const external:string[]=[];
  await page.route('**/*',route=>{
   const url=new URL(route.request().url());
   if(url.hostname!=='127.0.0.1'&&url.protocol!=='data:'){external.push(url.href);return route.abort()}
   return route.continue();
  });
  for(const url of ['/iframe.html?id='+encodeURIComponent('спецификации-рабочее-пространство--approval')+'&viewMode=story',native]){
   await page.goto(url);
   const heading=page.locator('.spec-document>h1');await expect(heading).toBeVisible();
   await page.evaluate(()=>document.fonts.ready);
   expect(await heading.evaluate(e=>getComputedStyle(e).fontFamily)).toContain('Lora');
   expect(await page.locator('.spec-goal .source-text').evaluate(e=>getComputedStyle(e).fontFamily)).toContain('Manrope');
   for(const family of ['Lora','Manrope'])expect(await page.evaluate(name=>[...document.fonts].some(f=>f.family===name&&f.status==='loaded'),family)).toBeTruthy();
  }
  expect(external).toEqual([]);
  const manifest=await (await request.get(native+'/highgrade-ui.json')).json();
  for(const family of ['Lora','Manrope']){
   const asset=manifest.provenance.find((p:{source:string})=>p.source===`src/assets/fonts/${family}.ttf`);
   expect(asset.license).toBe(`licenses/${family}-OFL.txt`);
   expect(asset.delivered.length).toBeGreaterThan(0);
   for(const file of [...asset.delivered,asset.license])expect((await request.get(native+'/'+file)).ok()).toBeTruthy();
  }
 }finally{await f.close()}
});
