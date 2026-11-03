import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const dist=path.join(root,'sites/nc-language/dist');
const origin='https://ncc-lenguage.org';
const errors=[];
const sitemap=[...fs.readFileSync(path.join(dist,'sitemap.xml'),'utf8').matchAll(/<loc>([^<]+)<\/loc>/g)].map(m=>m[1]);
if(sitemap.length!==21||new Set(sitemap).size!==21)errors.push('Expected 21 distinct canonical sitemap URLs');
const titles=new Set(),descriptions=new Set();
for(const url of sitemap){
  const pathname=new URL(url).pathname;
  if(!url.startsWith(origin+'/')||/\.html(?:$|[?#])/.test(url))errors.push(`Non-canonical sitemap URL: ${url}`);
  const relative=pathname==='/'?'index.html':pathname.slice(1)+'.html';
  const file=path.join(dist,relative);
  if(!fs.existsSync(file)){errors.push(`Missing sitemap page: ${url}`);continue;}
  const text=fs.readFileSync(file,'utf8');
  const fail=message=>errors.push(`${relative}: ${message}`);
  if(text.includes('noindex'))fail('indexable page contains noindex');
  const canonical=[...text.matchAll(/rel="canonical" href="([^"]+)"/g)];
  if(canonical.length!==1||canonical[0][1]!==url)fail('canonical does not match served route');
  if(!text.includes(`property="og:url" content="${url}"`))fail('social URL differs from canonical');
  const title=text.match(/<title>([^<]+)<\/title>/)?.[1];
  const description=text.match(/name="description" content="([^"]+)"/)?.[1];
  if(!title||titles.has(title))fail('missing or duplicate title');
  if(!description||descriptions.has(description))fail('missing or duplicate description');
  titles.add(title);descriptions.add(description);
  for(const link of text.matchAll(/href="([^"]+)"/g))if(!/^https?:/.test(link[1])&&/\.html(?:$|[?#])/.test(link[1]))fail(`link redirects instead of using canonical route: ${link[1]}`);
  try{
    const scripts=[...text.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)];
    if(scripts.length!==1)throw Error('expected one JSON-LD graph');
    const data=JSON.parse(scripts[0][1]),graph=data['@graph'];
    const page=graph.find(node=>node['@type']==='WebPage');
    if(data['@context']!=='https://schema.org'||page.url!==url||page.inLanguage!=='en')throw Error('page identity mismatch');
    const site=graph.find(node=>node['@type']==='WebSite');
    if(site.url!==origin+'/'||site.name!=='nC')throw Error('site identity mismatch');
    if(relative!=='index.html'){
      const crumbs=graph.find(node=>node['@type']==='BreadcrumbList')?.itemListElement;
      if(!text.includes('aria-label="Breadcrumb"')||!crumbs||crumbs.at(-1).item!==url||crumbs.some((item,i)=>item.position!==i+1))throw Error('missing or inconsistent breadcrumb');
    }
  }catch(error){fail(`invalid structured data: ${error.message}`);}
}
const missing=fs.readFileSync(path.join(dist,'404.html'),'utf8');
if(!missing.includes('name="robots" content="noindex"')||sitemap.includes(origin+'/404'))errors.push('404 must remain noindex and outside sitemap');
if(errors.length){console.error(errors.join('\n'));process.exitCode=1;}else console.log(JSON.stringify({ok:true,sitemapPages:sitemap.length,uniqueTitles:titles.size,uniqueDescriptions:descriptions.size,structuredDataPages:sitemap.length}));
