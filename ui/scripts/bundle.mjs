// Build-time ownership of public copies, licenses and the embedded bundle manifest.
import {readFileSync,writeFileSync,mkdirSync,cpSync,readdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
import path from 'node:path';
const hash=p=>createHash('sha256').update(readFileSync(p)).digest('hex');
const json=p=>JSON.parse(readFileSync(p,'utf8'));
if(process.argv[2]==='prepare'){
 for(const dir of ['brand','fonts','icons']){mkdirSync(`public/${dir}`,{recursive:true});for(const name of readdirSync(`src/assets/${dir}`).filter(n=>!n.endsWith('.png')))cpSync(`src/assets/${dir}/${name}`,`public/${dir}/${name}`)}
 mkdirSync('public/licenses',{recursive:true});cpSync('licenses','public/licenses',{recursive:true});
 const packages=[];
 for(const [location,record] of Object.entries(json('package-lock.json').packages).filter(([location,record])=>location&&!record.dev&&!record.optional)){
  const files=readdirSync(location).filter(n=>/^(licen[cs]e|notice)(?:\.|$)/i.test(n));
  const delivered=[];
  if(!files.length){
   if(location!=='node_modules/react-remove-scroll-bar'||record.version!=='2.3.8')throw Error(`Missing license: ${location}`);
   delivered.push('licenses/react-remove-scroll-bar-LICENSE.txt');
  }
  for(const file of files){const destination=`licenses/${location.replaceAll('/','-')}-${file}`;cpSync(`${location}/${file}`,`public/${destination}`);delivered.push(destination)}
  packages.push({package:location,version:record.version,license:record.license,notices:delivered});
 }
 writeFileSync('public/licenses/packages.json',JSON.stringify(packages,null,2)+'\n');
}else if(process.argv[2]==='manifest'){
 const files={};function walk(dir){for(const item of readdirSync(dir,{withFileTypes:true})){const p=path.join(dir,item.name);if(item.isSymbolicLink())throw Error(`Linked bundle asset ${p}`);if(item.isDirectory())walk(p);else if(item.isFile())files[path.relative('dist',p).replaceAll('\\','/')]=hash(p)}}walk('dist');
 const cliVersion=readFileSync('../Cargo.toml','utf8').match(/^version = "([^"]+)"/m)[1];
 const sources=['src/assets/fonts/Manrope.ttf','src/assets/fonts/Lora.ttf','src/components/icons.tsx',...readdirSync('src/assets/icons').map(n=>`src/assets/icons/${n}`),...readdirSync('src/assets/brand').filter(n=>n.endsWith('.svg')).map(n=>`src/assets/brand/${n}`)];
 const provenance=sources.map(source=>({source,sha256:hash(source),delivered:Object.keys(files).filter(p=>files[p]===hash(source)),license:source.includes('fonts')?`licenses/${path.basename(source,'.ttf')}-OFL.txt`:source.includes('icons')?'licenses/Lucide-LICENSE.txt':'licenses/README.md'}));
 // Adapted React geometry is compiled into this exact JS, not shipped as a separate SVG.
 for(const item of provenance)if(item.source==='src/components/icons.tsx')item.delivered=Object.keys(files).filter(p=>p.endsWith('.js'));
 writeFileSync('dist/highgrade-ui.json',JSON.stringify({schema_version:1,api_version:'2',cli_version:cliVersion,source_sha:process.env.HIGHGRADE_SOURCE_SHA??null,files,provenance,dependencies:json('package.json').dependencies},null,2)+'\n');
}else throw Error('Expected prepare or manifest');
