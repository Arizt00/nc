import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const out=path.resolve(process.argv.slice(2).find(a=>!a.startsWith('--'))||path.join(root,'dist/public-launch/nc'));
if(out===root||!out.startsWith(path.join(root,'dist')+path.sep))throw Error('Output must be a new directory under dist');
const dry=process.argv.includes('--dry-run');
const trees=['crates','src','tools','runtime/nc','targets','link','diagnostics','examples','tests','fixtures','spec','bootstrap','docs/spec'];
const excluded=new Set(['target','dist','node_modules','.git','.ncc','.ghor','.agents','.idea','_archive','_tmp','reports','handoff','__pycache__','evidence','generated','cache','vendor']);
const extensions=new Set(['.rs','.n','.nm','.ncp','.ncc','.nb','.toml','.json','.lock','.md','.txt','.sh','.ps1','.cmd','.bat','.yaml','.yml','.c','.h','.cpp','.hpp','.s','.asm','.mjs','.js','.css','.html','.svg','.png','.ebnf','.grammar','.xml','.ini','.cmake','.csv','.java','.kt','.kts','.gradle','.properties','.in','.ts','.tsx']);
const manifest=JSON.parse(fs.readFileSync(path.join(root,'tools/release/public_beta_manifest.json'),'utf8'));
const files=new Set(['README.md','LICENSE','LICENSE-MIT','LICENSE-APACHE','CONTRIBUTING.md','SECURITY.md','AGENTS.md','Cargo.toml','Cargo.lock','rust-toolchain.toml','nc-toolchain.toml','nc.toml','bridge.json','ghor.toml','ghor.lock','primitive.hardware.allowlist.json',...manifest.public_doc_files]);
function walk(relative,all=false){const dir=path.join(root,relative);if(!fs.existsSync(dir))return;for(const entry of fs.readdirSync(dir,{withFileTypes:true})){if(excluded.has(entry.name)||entry.name.startsWith('.'))continue;const rel=relative+'/'+entry.name;if(entry.isSymbolicLink())continue;if(entry.isDirectory())walk(rel,all);else if(all||extensions.has(path.extname(entry.name).toLowerCase())||['Dockerfile','Makefile'].includes(entry.name))files.add(rel);}}
for(const tree of trees)walk(tree);
// Explicit compile-time fixtures; the rest of the historical archive stays private.
files.add('_archive/old_examples/performance/native_fast/benchmark_contracts.json');
files.add('_archive/old_examples/performance/native_fast/optimizer_pgo_hot_loop.n');
walk('docs/public',true);walk('.github/ISSUE_TEMPLATE',true);files.add('.github/pull_request_template.md');files.add('.github/workflows/public-docs.yml');
walk('sites/nc-language/dist',true); // static output is explicitly public
for(const e of fs.readdirSync(path.join(root,'docs'))){if(/^NCC_MODULES_V81_.*\.md$/.test(e)||/^NCC_MODULE1_V72_REPRO\.md$/.test(e))files.add('docs/'+e);}
const findings=[];const records=[];
for(const rel of [...files].sort()){const file=path.join(root,rel);if(!fs.existsSync(file)){findings.push({file:rel,reason:'missing allowlisted file'});continue;}const data=fs.readFileSync(file);if(data.length>8*1024*1024){findings.push({file:rel,reason:'source exceeds 8 MiB'});continue;}if(!rel.endsWith('.pdf')){const text=data.toString('utf8');for(const [name,re] of [['private-key',/-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/],['github-token',/\bgh[pousr]_[A-Za-z0-9]{30,}\b/],['openai-token',/\bsk-(?:proj-)?[A-Za-z0-9_-]{32,}\b/],['aws-key',/\bAKIA[A-Z0-9]{16}\b/]])if(re.test(text))findings.push({file:rel,reason:name});}records.push({path:rel,bytes:data.length,sha256:crypto.createHash('sha256').update(data).digest('hex')});}
console.log(JSON.stringify({dryRun:dry,files:records.length,bytes:records.reduce((s,r)=>s+r.bytes,0),findings}));
if(findings.length){process.exitCode=1;}
else if(!dry){
 if(fs.existsSync(out)&&fs.readdirSync(out).length&&!process.argv.includes('--refresh'))throw Error('Refusing nonempty export directory; use --refresh only for this export');
 fs.mkdirSync(out,{recursive:true});
 for(const r of records){const to=path.join(out,r.path);fs.mkdirSync(path.dirname(to),{recursive:true});fs.copyFileSync(path.join(root,r.path),to);}
 const extra=(relative,data)=>{const to=path.join(out,relative);fs.mkdirSync(path.dirname(to),{recursive:true});fs.writeFileSync(to,data);records.push({path:relative,bytes:Buffer.byteLength(data),sha256:crypto.createHash('sha256').update(data).digest('hex')});};
 extra('tests/fixtures/old_examples/performance/native_fast/benchmark_contracts.json',fs.readFileSync(path.join(root,'_archive/old_examples/performance/native_fast/benchmark_contracts.json')));
 extra('ROADMAP.md',fs.readFileSync(path.join(root,'docs/public/ROADMAP.github.md')));
 const agentText=fs.readFileSync(path.join(root,'docs/public/AGENTS.github.md'));
 fs.writeFileSync(path.join(out,'AGENTS.md'),agentText);
 const agentRecord=records.find(r=>r.path==='AGENTS.md');
 Object.assign(agentRecord,{bytes:agentText.length,sha256:crypto.createHash('sha256').update(agentText).digest('hex')});
 extra('.gitignore','/target/\n/dist/\n/_tmp/\n/reports/\n**/.ncc/\n**/.ghor/\nnode_modules/\n.env\n.env.*\n*.log\n*.exe\n*.dll\n*.so\n*.o\n*.obj\n*.zip\n*.tar.gz\n');
 // Preserve source bytes so bootstrap provenance and the publication digests survive cloning.
 extra('.gitattributes','* -text\n*.pdf binary\n*.png binary\n');
 fs.writeFileSync(path.join(out,'PUBLICATION.json'),JSON.stringify({schema:'nc.publication.v1',edition:'bootstrap-v81',date:'2026-10-24',source:'curated working tree snapshot; not a clean development commit',files:records},null,2)+'\n');
}
