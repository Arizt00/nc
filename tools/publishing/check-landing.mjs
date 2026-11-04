import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const dist=path.join(root,'sites/nc-language/dist'),html=fs.readFileSync(path.join(dist,'index.html'),'utf8');
const errors=[];
const ids=[...html.matchAll(/\bid="([^"]+)"/g)].map(match=>match[1]);
if(ids.length!==new Set(ids).size)errors.push('Duplicate landing IDs');
for(const match of html.matchAll(/href="#([^"]+)"/g))if(!ids.includes(match[1]))errors.push('Missing anchor: '+match[1]);
for(const id of ['main','nav','hero-title','architecture','language-levels','continuity-canvas','scene-status','motion-toggle','search-dialog','phase-panel','terminal-panel'])if(!ids.includes(id))errors.push('Missing interactive surface: '+id);
if(!html.includes('One language.')||!html.includes('Every layer.'))errors.push('Missing approved headline');
if(/approved-landing|reference_[12]\.(?:jpg|png)/.test(html))errors.push('Reference capture used as a production asset');
let modules=0;
for(const file of ['hero-scene.js','hero-shaders.js','visual-state.js']){
  const source=fs.readFileSync(path.join(dist,file),'utf8');modules++;
  for(const match of source.matchAll(/from ['"]([^'"]+)['"]/g)){
    if(!match[1].startsWith('./')){errors.push('Non-local module: '+match[1]);continue;}
    const target=path.resolve(dist,match[1].split('?')[0]);
    if(!target.startsWith(dist+path.sep)||!fs.existsSync(target))errors.push('Missing module: '+match[1]);
  }
}
for(const file of ['assets/hand-hero-v1.webp','vendor/three/0.187.0/LICENSE'])if(!fs.existsSync(path.join(dist,file)))errors.push('Missing production asset: '+file);
const handBytes=fs.statSync(path.join(dist,'assets/hand-hero-v1.webp')).size;
if(handBytes>250000)errors.push('Hand asset exceeds the 250 KB budget');
if(errors.length){console.error(errors.join('\n'));process.exitCode=1;}
else console.log(JSON.stringify({ok:true,uniqueIds:ids.length,modules,handBytes,localGraphicsEngine:true}));
