// Search metadata describes the published V81 edition, not unpublished compiler work.
export const siteOrigin = 'https://ncc-lenguage.org';
export const publicUrl = file => `${siteOrigin}/${file === 'index.html' ? '' : file.replace(/\.html$/, '')}`;
const metadata = {
  'index.html': ['nC Programming Language & ncc Compiler', 'Explore nC, an experimental programming language, and ncc, its native compiler. Read the documentation, download the toolchain and build your first program.'],
  'nc-programming-language.html': ['nC Programming Language: Design, Memory & Machine Code', 'Learn how the nC programming language connects functions, explicit memory and typed machine code. Explore its design, syntax and first program with ncc.'],
  'ncc-compiler.html': ['ncc Compiler for nC: Native Toolchain & Quick Start', 'Use ncc to analyze nC source, inspect IR and build native executables. Get the V81 bootstrap kit for Windows or Linux and follow the compiler quick start.'],
  'library.html': ['nC Documentation: Language Guide & ncc Compiler Internals', 'Read the nC language guide and ncc compiler reference: installation, syntax, manual memory, typed machine code, IR, ABI, linking and bootstrap evidence.'],
  'downloads.html': ['Download ncc: nC Compiler for Windows & Linux', 'Download the ncc V81 bootstrap compiler for Windows x86-64 or Linux and WSL. Find runtime requirements, SHA-256 checksums and the first-program guide.'],
  'why-nc-exists': ['Why nC Exists: Programming Language Design', 'Understand why nC connects functions, memory layout, typed machine instructions and linking within one language, and what toolchain sovereignty means.'],
  'reading-the-project': ['nC and ncc: Source Files, Compiler & Project Structure', 'Learn the difference between the nC language and ncc compiler, how .n source files fit the toolchain, and where to find examples and bootstrap contracts.'],
  'setting-up-the-toolchain': ['Install ncc: nC Toolchain Setup on Windows & Linux', 'Set up the ncc V81 native kit on Windows or Linux and WSL. Verify compiler and runtime identity, compile hello.n and check the output and exit code.'],
  'the-first-program': ['Your First nC Program: Compile and Run with ncc', 'Write and run a first nC program with ncc. Follow the hello.n example, understand its output import and entry point, then inspect semantics and IR.'],
  'places-and-functions': ['nC Syntax: Places, Functions & Control Flow', 'Read the nC guide to typed places, functions, parameters and control flow. Follow source examples and inspect the contracts checked by the compiler.'],
  'memory-and-services': ['nC Memory and Runtime Services', 'Understand manual memory in nC, explicit allocation and release, and the runtime services a program imports. Type checking does not add ownership.'],
  'the-machine-layer': ['nC Machine Layer: Typed Instructions with [@machine]', 'Explore the nC machine layer: typed instructions, labels, control transfers and terminators. Separate semantic validation from execution on a target.'],
  'working-during-bootstrap': ['Working with the ncc Compiler During Bootstrap', 'Use ncc during bootstrap with explicit compiler and provider identities. Learn how to report a reproducible failure and respect the native CLI scope.'],
  'contracts-between-levels': ['nC Architecture: Contracts Between Language Levels', 'Trace nC contracts across ordinary source, root, machine and primitive layers. Learn how memory, runtime and linking preserve program meaning.'],
  'parser-ast-semantics-and-ir': ['ncc Compiler Internals: Parser, AST, Semantics & IR', 'Follow the ncc compiler pipeline from parsing and AST construction to semantic analysis and IR. Review the contracts each phase must preserve.'],
  'machine-code-and-control-flow': ['nC Machine Code: Control Flow & Terminators', 'Read the nC architecture reference for typed machine instructions, control-flow edges and terminators, including their relationship to analysis and IR.'],
  'memory-and-representation': ['nC Memory Model: Layout, Pointers & Representation', 'Study nC memory representation, layout, alignment and raw pointers. Separate language-level contracts from the guarantees of a materialized target.'],
  'materialization-abi-and-linking': ['ncc Code Generation: Materialization, ABI & Linking', 'Follow ncc materialization from IR to objects and executable images. Understand target layouts, calling conventions, symbols, relocations and providers.'],
  'what-bootstrap-v81-demonstrates': ['ncc Bootstrap V81: Evidence, Scope & Limitations', 'Review what the ncc V81 bootstrap proves on Windows and Linux x86-64, how B/C generations were compared, and why global Stage3 promotion remains open.'],
  'reproducing-and-reviewing-evidence': ['Reproduce ncc Bootstrap Builds and Review Evidence', 'Reproduce ncc bootstrap checks using identified inputs, compiler hashes and providers. Review raw artifacts, receipts and the limits of each result.'],
  'releases-and-contributions': ['Contribute to nC: Releases, Tests & Compiler Changes', 'Contribute reproducible reports and coherent changes to nC and ncc. Review release evidence and keep parser, AST, semantics, IR and documentation aligned.'],
};
const escape = text => text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
export function pageMetadata(file) {
  const entry = metadata[file] || metadata[file.replace(/^library\//, '').replace(/\.html$/, '')];
  if (!entry) throw new Error(`Missing search metadata: ${file}`);
  return {title: entry[0], description: entry[1]};
}
export function cleanPageLinks(html, file) {
  return html.replace(/href="([^"]+)"/g, (match, href) => {
    if (/^(?:[a-z][a-z0-9+.-]*:|\/\/|#)/i.test(href)) return match;
    const target = new URL(href, `${siteOrigin}/${file}`);
    if (!target.pathname.endsWith('.html')) return match;
    target.pathname = target.pathname === '/index.html' ? '/' : target.pathname.replace(/\.html$/, '');
    return `href="${target.pathname}${target.search}${target.hash}"`;
  });
}
export function decoratePage(html, file, article) {
  const {title, description} = pageMetadata(file), url = publicUrl(file);
  const home = `${siteOrigin}/`, languageId = `${home}#language`, compilerId = `${home}#compiler`;
  const website = {'@type':'WebSite','@id':`${home}#website`,url:home,name:'nC',alternateName:'nC Programming Language',inLanguage:'en',description:'The nC programming language, ncc compiler and English documentation.'};
  const language = {'@type':'ComputerLanguage','@id':languageId,name:'nC',alternateName:'nC programming language',url:publicUrl('nc-programming-language.html'),description:'An experimental programming language connecting functions, manual memory and typed machine instructions.'};
  const compiler = {'@type':'SoftwareSourceCode','@id':compilerId,name:'ncc',alternateName:'ncc compiler for nC',url:publicUrl('ncc-compiler.html'),codeRepository:'https://github.com/Arizt00/nc',about:{'@id':languageId},description:'The nC compiler and its native bootstrap sources, with retained Stage0 compatibility.'};
  const page = {'@type':'WebPage','@id':`${url}#webpage`,url,name:title,description,inLanguage:'en',isPartOf:{'@id':website['@id']},about:[{'@id':languageId},{'@id':compilerId}]};
  const graph = [website, language, compiler, page];
  if (file !== 'index.html') {
    const label = article?.title || ({'library.html':'Documentation','downloads.html':'Download ncc','nc-programming-language.html':'nC programming language','ncc-compiler.html':'ncc compiler'})[file];
    const trail = [{name:'nC',url:home}, ...(article ? [{name:'Documentation',url:publicUrl('library.html')}] : []), {name:label,url}];
    const breadcrumbId = `${url}#breadcrumb`;
    graph.push({'@type':'BreadcrumbList','@id':breadcrumbId,itemListElement:trail.map((entry, index)=>({'@type':'ListItem',position:index+1,name:entry.name,item:entry.url}))});
    page.breadcrumb = {'@id':breadcrumbId};
    const breadcrumb = `<nav class="breadcrumbs" aria-label="Breadcrumb"><ol>${trail.map((entry,index)=>`<li>${index===trail.length-1?`<span aria-current="page">${escape(entry.name)}</span>`:`<a href="${entry.url}">${escape(entry.name)}</a>`}</li>`).join('')}</ol></nav>`;
    html = article ? html.replace(/<a class="mobile-breadcrumb"[^>]*>[\s\S]*?<\/a>/,breadcrumb) : html.replace(/(<main\b[^>]*>)/,`$1${breadcrumb}`);
  }
  html = html.replace(/<title>[\s\S]*?<\/title>/,`<title>${escape(title)}</title>`)
    .replace(/<meta name="description" content="[^"]*">/,`<meta name="description" content="${escape(description)}">`)
    .replace(/<meta property="og:title" content="[^"]*">/,`<meta property="og:title" content="${escape(title)}">`)
    .replace(/<meta property="og:description" content="[^"]*">/,`<meta property="og:description" content="${escape(description)}">`)
    .replace('</head>',`<meta property="og:site_name" content="nC"><meta property="og:locale" content="en_US"><script type="application/ld+json">${JSON.stringify({'@context':'https://schema.org','@graph':graph}).replaceAll('<','\\u003c')}</script></head>`);
  return cleanPageLinks(html, file);
}

export function overviewPages(repo, release) {
  return [
    {file:'nc-programming-language.html',title:'nC programming language',body:`<main id="main" class="wrap overview"><article class="article"><p class="eyebrow">LANGUAGE DESIGN</p><h1>The nC programming language<span>.</span></h1><p class="lead">nC is an experimental programming language for expressing functions, memory operations and machine instructions within one semantic model. Its compiler is called ncc.</p><div class="actions"><a class="button primary" href="library/the-first-program.html">Write your first nC program ↗</a><a class="text-link" href="ncc-compiler.html">Meet the ncc compiler ↗</a></div>
<h2>One program, several levels</h2><p>A program's behavior depends on more than its surface syntax. Data layout, alignment, calling conventions and linkage all affect the result. nC makes these decisions part of the language's design, with contracts that must remain consistent through analysis, IR, lowering and linking.</p><p>Ordinary <code>.n</code> source describes functions, data, imports and control flow. Root <code>[@]</code> establishes foundational contracts. <code>[@machine]</code> expresses typed instructions and control transfers; <code>primitive[@]</code> covers lower-level material operations. These levels share a semantic model rather than separate, interchangeable implementations.</p><p>Start with the <a href="library/why-nc-exists.html">language's design and motivation</a>, then read the <a href="library/contracts-between-levels.html">contracts between nC levels</a>.</p>
<h2>Explicit memory and representation</h2><p>Memory has manual/raw semantics. The program is responsible for allocation, address validity and release. Type checking does not introduce ownership, automatic deallocation or implicit lifetime checks for raw pointers.</p><p>The <a href="library/memory-and-services.html">memory and runtime guide</a> covers the program-facing contract. The <a href="library/memory-and-representation.html">representation reference</a> explains why target layout and ABI remain explicit.</p>
<h2>Learn the syntax with a complete program</h2><p>The first example imports <code>io:output</code>, declares a <code>to main</code> entry point and calls <code>println</code>. Its container also declares a final with <code>fa:</code>. Use the <a href="library/the-first-program.html">annotated hello.n example</a> to see how those pieces fit, then continue with <a href="library/places-and-functions.html">places, functions and control flow</a>.</p>
<h2>Current scope</h2><p>The public documentation and downloads describe bootstrap edition V81. The native kit was tested on Windows x86-64 and Ubuntu 24.04 under WSL x86-64. Support claims apply to the documented profiles and fixtures; global Stage3 promotion remains open.</p><p>nC is currently relevant to compiler development, data representation, ABI work and native code generation. The <a href="library/what-bootstrap-v81-demonstrates.html">V81 evidence</a> separates verified behavior from work still in progress.</p>
<h2>nC is the language. ncc is the compiler.</h2><p>Use <a href="ncc-compiler.html">ncc</a> to analyze source, inspect IR and emit native code. Read the <a href="library.html">nC documentation</a>, <a href="downloads.html">download the compiler</a> or inspect the <a href="${repo}">public nC and ncc source repository</a>.</p></article></main>`},
    {file:'ncc-compiler.html',title:'ncc compiler',body:`<main id="main" class="wrap overview"><article class="article"><p class="eyebrow">NATIVE TOOLCHAIN · V81</p><h1>ncc, the compiler for nC<span>.</span></h1><p class="lead">ncc analyzes nC source, produces intermediate representation and emits native code for documented target profiles. The V81 bootstrap kit includes the compiler, its runtime provider and a first program.</p><div class="actions"><a class="button primary" href="downloads.html">Download ncc ↗</a><a class="text-link" href="library/setting-up-the-toolchain.html">Read the installation guide ↗</a></div>
<h2>Compile a first program on Windows</h2><p>Download the Windows x86-64 kit, verify the SHA-256 checksums in the <a href="${release}">release notes</a>, and extract it. Keep <code>ncc.exe</code> and <code>ncruntime.dll</code> in the same directory. Run these commands in PowerShell from that directory:</p><div class="code"><div class="code-title"><span>PowerShell</span><button class="copy" type="button" aria-label="Copy code">Copy</button></div><pre><code>.\\ncc.exe self-test
.\\ncc.exe compiler emit-executable hello.n --target x86_64-pc-windows-msvc --out hello.exe
.\\hello.exe</code></pre></div><p>The expected output is <code>Hello, nC public beta</code>, with exit code <code>0</code>. The prebuilt kit does not require a Rust or LLVM installation.</p>
<h2>Linux and WSL</h2><p>The Linux x86-64 kit was tested on Ubuntu 24.04 under WSL and requires glibc 2.34 symbols or later. Keep its shared runtime provider beside the compiler. The generated example also needs the library search path shown in the <a href="library/setting-up-the-toolchain.html">Linux installation and execution recipe</a>.</p>
<h2>Inspect the program before execution</h2><p>The native compiler exposes its analysis and emission operations under <code>compiler</code>. From the extracted kit, these commands check the example and emit its IR:</p><div class="code"><div class="code-title"><span>PowerShell</span><button class="copy" type="button" aria-label="Copy code">Copy</button></div><pre><code>.\\ncc.exe compiler sema hello.n
.\\ncc.exe compiler emit-ir hello.n</code></pre></div><p>Follow the <a href="library/the-first-program.html">first-program guide</a> for the source and expected behavior. The <a href="library/parser-ast-semantics-and-ir.html">compiler pipeline reference</a> explains the responsibilities of parser, AST, semantic analysis and IR.</p>
<h2>Bootstrap scope and reproducibility</h2><p>V81 is a prerelease bootstrap edition with a partial native command set. Consult the kit README before using a Stage0 command. Building the retained Stage0 from source has separate seed and tool requirements.</p><p>The recorded B/C generations are byte-identical within each tested host. That result does not establish complete command parity or global Stage3 promotion. Keep compiler and provider hashes with each result, and read the <a href="library/what-bootstrap-v81-demonstrates.html">bootstrap evidence and limitations</a>.</p>
<h2>Source and bug reports</h2><p>The <a href="${repo}">ncc compiler source</a> is maintained with the nC language. A useful report includes the host, compiler and provider digests, complete command, minimal source, diagnostic and expected result. The <a href="${repo}/blob/main/CONTRIBUTING.md">contribution guide</a> describes the review contract.</p><p>New to the language? Start with the <a href="nc-programming-language.html">nC programming language overview</a> or browse the <a href="library.html">English documentation</a>.</p></article></main>`},
  ];
}
