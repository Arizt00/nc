import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const dist=path.join(root,'sites/nc-language/dist');
const errors=[];let pages=0,links=0;
function walk(dir){return fs.readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?walk(path.join(dir,e.name)):[path.join(dir,e.name)]);}
for(const file of walk(dist).filter(f=>f.endsWith('.html'))){pages++;const text=fs.readFileSync(file,'utf8');if(!text.includes('lang="en"'))errors.push(`${file}: missing language`);if((text.match(/<h1[ >]/g)||[]).length!==1)errors.push(`${file}: expected one h1`);for(const m of text.matchAll(/(?:href|src)="([^"#]+)(?:#[^"]*)?"/g)){const u=m[1].split('?')[0];if(/^(?:https?:|data:|mailto:)/.test(u))continue;links++;let target=u.startsWith('/')?path.join(dist,u):path.resolve(path.dirname(file),u);if(target===dist||target===dist+path.sep)target=path.join(dist,'index.html');else if(!path.extname(target))target+='.html';if(!target.startsWith(dist+path.sep)||!fs.existsSync(target))errors.push(`${path.relative(dist,file)}: missing ${u}`);}}
let animatedPages=0,readingPages=0;
for(const file of walk(dist).filter(file=>file.endsWith('.html'))){
  const html=fs.readFileSync(file,'utf8');if(html.includes('http-equiv="refresh"'))continue;
  const relative=path.relative(dist,file).replaceAll(path.sep,'/');
  const expected=relative==='index.html'?'hero':relative.startsWith('library/')||['nc-programming-language.html','ncc-compiler.html'].includes(relative)?'reading':'browse';
  if((html.match(/id="page-particles"/g)||[]).length!==1)errors.push(relative+': expected one atmospheric canvas');
  if((html.match(/id="motion-toggle"/g)||[]).length!==1)errors.push(relative+': expected one shared motion control');
  if(!html.includes('data-atmosphere="'+expected+'"'))errors.push(relative+': wrong particle profile');
  if(!/src="[^"]*scene\.js\?v=[a-f0-9]{12}"/.test(html))errors.push(relative+': missing versioned particle script');
  if((html.match(/id="continuity-canvas"/g)||[]).length!==(relative==='index.html'?1:0))errors.push(relative+': unexpected hero canvas');
  animatedPages++;if(expected==='reading')readingPages++;
}
const index=JSON.parse(fs.readFileSync(path.join(dist,'search.json'),'utf8'));
if(index.length!==18||index.filter(item=>item.url.startsWith('library/')).length!==16)errors.push('Expected 16 chapters and two overview search entries');
for(const item of index)if(!fs.existsSync(path.join(dist,item.url+'.html')))errors.push(`Search result missing ${item.url}`);
for(const name of ['origen-y-guia','arquitectura-y-bootstrap'])if(!fs.existsSync(path.join(root,`docs/public/${name}.pdf`)))errors.push(`Missing PDF ${name}`);
if(errors.length){console.error(errors.join('\n'));process.exitCode=1;}else console.log(JSON.stringify({ok:true,pages,localLinks:links,searchablePages:index.length,animatedPages,readingPages}));
