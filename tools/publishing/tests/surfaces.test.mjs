import {test} from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {highlightNC,loadSurfaceExamples,renderSurfaceExplorer} from '../surfaces.mjs';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../../..');
const examples=loadSurfaceExamples(root);
const decode=text=>text.replace(/<span class="syntax-[a-z]+">([\s\S]*?)<\/span>/g,'$1').replaceAll('&quot;','"').replaceAll('&lt;','<').replaceAll('&gt;','>').replaceAll('&amp;','&');
test('Highlighting preserves exact source text, including hostile strings and whitespace',()=>{
 const source='// <script>bad</script>\nfn: public: run() -> bool {\n\tvalue >> int <- 42;\n print("<img src=x onerror=bad()> & \\\"text\\\"");\n return value < 50;\n}\n';
 const html=highlightNC(source);assert.equal(decode(html),source);assert.ok(!html.includes('<script>'));assert.ok(!html.includes('<img'));assert.match(html,/syntax-comment/);
});
test('Every advertised surface is tied to an unchanged public source',()=>{
 assert.deepEqual(examples.map(example=>example.id),['source','root','machine','primitive']);
 for(const example of examples){const original=fs.readFileSync(path.join(root,example.path));assert.equal(createHash('sha256').update(original).digest('hex'),example.sourceSha256);assert.ok(original.toString('utf8').replaceAll('\r\n','\n').includes(example.code));assert.equal(decode(highlightNC(example.code)),example.code);}
});
test('Tabs have unique controls, real references and a usable no-script route',()=>{
 const html=renderSurfaceExplorer(examples,'https://github.com/Arizt00/nc');
 for(const example of examples){assert.ok(html.includes(`aria-controls="surface-panel-${example.id}"`));assert.ok(html.includes(`aria-labelledby="surface-tab-${example.id}"`));assert.ok(html.includes(`/blob/main/${example.path}`));}
 assert.equal((html.match(/role="tab" /g)||[]).length,4);assert.equal((html.match(/role="tabpanel"/g)||[]).length,4);assert.ok(html.includes('<noscript>'));assert.ok(!html.includes('Run code'));
});
