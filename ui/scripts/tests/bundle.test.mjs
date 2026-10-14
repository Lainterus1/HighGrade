import {test} from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,copyFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

test('manifest records both local fonts, delivered hashes and their own OFL notices',()=>{
 const root=mkdtempSync(path.join(tmpdir(),'hg-bundle-manifest-')),ui=path.join(root,'ui');
 function put(relative,bytes){const file=path.join(ui,relative);mkdirSync(path.dirname(file),{recursive:true});writeFileSync(file,bytes);}
 try{
  writeFileSync(path.join(root,'Cargo.toml'),'[package]\nversion = "0.4.0"\n');
  put('scripts/bundle.mjs',readFileSync(fileURLToPath(new URL('../bundle.mjs',import.meta.url))));
  put('package.json',JSON.stringify({dependencies:{react:'fixture'}}));
  for(const font of ['Manrope','Lora']){
   const bytes=Buffer.from(`${font} independent font fixture`);
   put(`src/assets/fonts/${font}.ttf`,bytes);put(`dist/fonts/${font}.ttf`,bytes);
   put(`dist/licenses/${font}-OFL.txt`,`${font} fixture license`);
  }
  put('src/components/icons.tsx','fixture adapted icons');
  put('src/assets/icons/arrow.svg','fixture icon');put('dist/icons/arrow.svg','fixture icon');
  put('src/assets/brand/mark.svg','fixture mark');put('dist/brand/mark.svg','fixture mark');
  put('dist/assets/main.js','fixture compiled icons');
  const result=spawnSync(process.execPath,['scripts/bundle.mjs','manifest'],{cwd:ui,encoding:'utf8',env:{...process.env,HIGHGRADE_SOURCE_SHA:'a'.repeat(40)}});
  assert.equal(result.status,0,result.stderr);
  const manifest=JSON.parse(readFileSync(path.join(ui,'dist/highgrade-ui.json'),'utf8'));
  assert.equal(manifest.source_sha,'a'.repeat(40));assert.equal(manifest.cli_version,'0.4.0');
  for(const font of ['Manrope','Lora']){
   const entry=manifest.provenance.find(p=>p.source===`src/assets/fonts/${font}.ttf`);
   assert.ok(entry,`missing ${font} provenance`);
   const expected=createHash('sha256').update(readFileSync(path.join(ui,entry.source))).digest('hex');
   assert.equal(entry.sha256,expected);assert.deepEqual(entry.delivered,[`fonts/${font}.ttf`]);
   assert.equal(manifest.files[entry.delivered[0]],expected);assert.equal(entry.license,`licenses/${font}-OFL.txt`);
   assert.equal(readFileSync(path.join(ui,'dist',entry.license),'utf8'),`${font} fixture license`);
  }
 }finally{rmSync(root,{recursive:true});}
});
