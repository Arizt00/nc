// Optional documentation tool. Install Playwright separately or pass its module directory.
import fs from 'node:fs';
import path from 'node:path';
import {createRequire} from 'node:module';
import {fileURLToPath} from 'node:url';
import {markdown,books} from './build-site.mjs';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.NC_DOC_NODE_MODULES?path.join(process.env.NC_DOC_NODE_MODULES,'playwright'):'playwright');
const browser=await chromium.launch({headless:true,...(process.env.NC_DOC_BROWSER?{channel:process.env.NC_DOC_BROWSER}:{})});
try{
 for(const b of books){
  const md=fs.readFileSync(path.join(root,`docs/public/${b.file}.md`),'utf8');
  const title=md.split('\n')[0].slice(2),chapters=md.split(/^## /m).slice(1);
  const html=`<!doctype html><html lang="es"><head><meta charset="utf-8"><title>${title}</title><style>
  @page{size:A4;margin:19mm 20mm 20mm}*{box-sizing:border-box}body{margin:0;color:#17251d;font:10.5pt/1.53 Arial,sans-serif}h1,h2,h3{font-weight:500;line-height:1.15;letter-spacing:-.02em;color:#000}h1{font-size:35pt;margin:18mm 0 10mm;max-width:145mm}h2{font-size:24pt;margin:0 0 8mm}h3{font-size:13pt;margin-top:8mm}p{margin:0 0 4mm}a{color:inherit}code{font:8.5pt Consolas,monospace}pre{white-space:pre-wrap;overflow-wrap:anywhere;line-height:1.45;margin:0;font-size:8.5pt}.code{background:#eef1e8;padding:4mm;margin:5mm 0;break-inside:avoid}.code-title{font:8pt Arial;color:#52634d;margin-bottom:2mm}.copy{display:none}table{border-collapse:collapse;width:100%;font-size:9pt;margin:5mm 0}td,th{border:1px solid #d9d9d9;padding:3mm;text-align:left;vertical-align:top}th{background:#e7ede1}td:first-child{width:31%}tr{break-inside:avoid}.cover{break-after:page;min-height:247mm;position:relative}.mark{font-size:54pt;font-weight:700;letter-spacing:-5pt;color:#214735}.eyebrow{font:9pt Arial;letter-spacing:.12em;color:#4b604c}.cover .summary{font-size:14pt;line-height:1.55;max-width:143mm;margin:12mm 0}.contents{margin-top:16mm;border-top:1px solid #d9d9d9;padding-top:7mm;display:grid;grid-template-columns:1fr 1fr;gap:3mm 8mm;font-size:9pt}.contents span{color:#52634d;margin-right:2mm}.chapter{break-before:page}.chapter>.eyebrow{margin-bottom:6mm}.chapter p,.chapter li{orphans:3;widows:3}li{margin:2mm 0}ul{padding-left:5mm}.edition{margin-top:8mm;font-size:9pt;color:#566451}
  </style></head><body><section class="cover"><div class="mark">nC</div><p class="eyebrow">DOCUMENTACIÓN DEL LENGUAJE</p><h1>${title}</h1><p class="summary">${b.file==='origen-y-guia'?'Por qué existe nC, cómo preparar las herramientas y cómo leer un programa desde su fuente hasta la capa de máquina.':'Contratos del compilador, memoria, ABI, materialización y alcance de la reconstrucción nativa V81.'}</p><p class="edition">Edición de bootstrap V81<br>24 de octubre de 2026<br>github.com/Arizt00/nc</p><div class="contents">${chapters.map((c,i)=>`<div><span>${String(i+1).padStart(2,'0')}</span> ${c.split('\n')[0]}</div>`).join('')}</div></section>${chapters.map((c,i)=>`<section class="chapter"><p class="eyebrow">${String(i+1).padStart(2,'0')} / ${b.label.toUpperCase()}</p>${markdown('## '+c)}</section>`).join('')}</body></html>`;
  const page=await browser.newPage();await page.setContent(html,{waitUntil:'load'});await page.evaluate(()=>document.fonts.ready);
  await page.pdf({path:path.join(root,`docs/public/${b.file}.pdf`),format:'A4',printBackground:true,preferCSSPageSize:true,displayHeaderFooter:true,headerTemplate:'<div></div>',footerTemplate:`<div style="font:8px Arial;color:#5d675b;width:100%;padding:0 20mm;display:flex;justify-content:space-between"><span>nC · ${b.label} · V81</span><span><span class="pageNumber"></span> / <span class="totalPages"></span></span></div>`});
  fs.mkdirSync(path.join(root,'_tmp/nc-publication'),{recursive:true});fs.writeFileSync(path.join(root,`_tmp/nc-publication/${b.file}.html`),html);await page.close();console.log(`PDF: docs/public/${b.file}.pdf`);
 }
}finally{await browser.close();}
