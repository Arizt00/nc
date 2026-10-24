/* eslint-disable no-console */
const fs = require('fs');
const path = require('path');
const assert = require('assert');
const core = require('./nc_semantic');

function readJson(p) {
  return JSON.parse(fs.readFileSync(p, 'utf8'));
}

function readText(p) {
  return fs.readFileSync(p, 'utf8');
}

function normalizedText(p) {
  return readText(p).replace(/\r\n/g, '\n');
}

function readFixture(name) {
  const p = path.join(__dirname, '..', 'test-fixtures', name);
  return {
    path: p,
    text: fs.readFileSync(p, 'utf8')
  };
}

function hexFromRgb(rgb) {
  return `#${rgb.map((n) => n.toString(16).padStart(2, '0')).join('')}`.toUpperCase();
}

function wordCount(text) {
  return String(text || '').trim().split(/\s+/).filter(Boolean).length;
}

function main() {
  const root = path.join(__dirname, '..');
  const packageJsonPath = path.join(root, 'package.json');
  const pkg = readJson(packageJsonPath);
  const registry = core.loadBaseRegistry(root);
  const intellisenseCatalog = core.loadIntellisenseCatalog(root);
  const stoudStdlibCatalog = core.loadStoudStdlibCatalog(root);
  const colorPalettePath = path.join(root, '..', '..', 'theme', 'nc_colors.json');
  const semanticTokensPath = path.join(root, '..', '..', 'theme', 'nc_semantic_tokens.json');
  const visualIdentityPath = path.join(root, '..', '..', 'theme', 'nc_visual_identity.json');
  const baseLogoPath = path.join(root, '..', '..', '..', 'site', 'public', 'assets', 'nc-logo.svg');
  const themePath = path.join(root, 'themes', 'nc-sovereign-dark-color-theme.json');
  const systemsThemePath = path.join(root, 'themes', 'nc-systems-spectrum-color-theme.json');
  const polyglotThemePath = path.join(root, 'themes', 'nc-polyglot-systems-color-theme.json');
  const iconThemePath = path.join(root, 'icons', 'nc-icon-theme.json');
  const grammarPath = path.join(root, 'syntaxes', 'nc.tmLanguage.json');
  const basalGrammarPath = path.join(root, 'syntaxes', 'nc-basal.tmLanguage.json');
  const machineGrammarPath = path.join(root, 'syntaxes', 'nc-machine.tmLanguage.json');
  const backendObjectGrammarPath = path.join(root, 'syntaxes', 'nc-backend-object.tmLanguage.json');
  const targetInitGrammarPath = path.join(root, 'syntaxes', 'nc-target-init.tmLanguage.json');
  const primitiveGrammarPath = path.join(root, 'syntaxes', 'nc-primitive.tmLanguage.json');
  const hardwareGrammarPath = path.join(root, 'syntaxes', 'nc-hardware.tmLanguage.json');
  const snippetsPath = path.join(root, 'snippets', 'nc.code-snippets');
  const extensionLogoPath = path.join(root, 'assets', 'nc-logo.svg');
  const extensionSourcePath = path.join(root, 'src', 'extension.js');
  const semanticSourcePath = path.join(root, 'src', 'nc_semantic.js');
  const extensionSource = readText(extensionSourcePath);
  const semanticSource = readText(semanticSourcePath);

  assert.strictEqual(registry.schema, 'nc.vscode.base.registry.v1');
  assert(Array.isArray(registry.runtime_families));
  assert(registry.runtime_families.length >= 90);
  assert(Array.isArray(registry.runtime_capabilities) && registry.runtime_capabilities.length >= 90);
  assert(Array.isArray(registry.runtime_policies) && registry.runtime_policies.length >= 30);
  assert(Array.isArray(registry.diagnostics_catalog) && registry.diagnostics_catalog.length >= 30);
  assert.strictEqual(intellisenseCatalog.schema, 'nc.ide.intellisense.catalog.v1');
  assert(intellisenseCatalog.symbols.some((row) => row.label === 'readInt' && row.facadeImport === 'use <io:input>;'));
  assert(intellisenseCatalog.symbols.some((row) => row.label === 'pFromAddr' && row.facadeImport === 'use <memory:ptr>;'));
  assert(intellisenseCatalog.symbols.some((row) => row.label === 'pRead' && row.facadeImport === 'use <memory:ptr>;'));
  assert(intellisenseCatalog.symbols.some((row) => row.label === 'pWrite' && row.facadeImport === 'use <memory:ptr>;'));
  assert(intellisenseCatalog.symbols.some((row) => row.label === 'pAt' && row.facadeImport === 'use <memory:region>;'));
  assert(intellisenseCatalog.symbols.some((row) => row.label === 'fs' && row.facadeImport === 'use <auth.system:file>;'));
  assert(core.lookupCatalogEntry(intellisenseCatalog, 'memRegion').signature === 'memRegion<T>');
  assert.strictEqual(intellisenseCatalog.version, '1.0.0.21v');

  const requiredSemanticDictionaryEntries = [
    'int', 'bool', 'str', 'string', 'byte', 'word', 'dword', 'qword', 'signed', 'unsigned',
    'ft32', 'array', 'list', 'queue', 'vector', 'dinarray', 'Result', 'addr', 'p<T>', 'p mut<T>', 'ref<T>', 'ref mut<T>',
    'site<T>', 'memRegion<T>', 'slice<T>', 'span<T>', 'program', 'runtime', 'compiler', 'basal',
    'machine', 'primitive', 'kernel', '+', '-', '*', '/', '%', '&', '|', '^', '~',
    '>>', '>>>', '->', '->>', ':', '::', '<-', '<-', '@', '!>', 'x >> int <- value', 'mem[site]',
    'load T [address]', 'type', 'stc', 'local', 'fn', 'fa', 'to main', 'public', 'internal', 'private', 'static', 'stack', 'abi', 'in', 'const',
    'store', 'copy', 'fill', 'size', 'align', 'offset'
  ];
  for (const label of requiredSemanticDictionaryEntries) {
    const row = core.lookupCatalogEntry(intellisenseCatalog, label);
    assert(row, `semantic dictionary missing ${label}`);
    assert(!row.docs, `semantic dictionary entry ${label} must not expose repository paths`);
    assert(wordCount(row.documentation) > 0, `semantic dictionary entry ${label} must explain itself`);
    assert(wordCount(row.documentation) <= 50, `semantic dictionary entry ${label} exceeds 50 words`);
  }
  const operatorHit = core.lookupCatalogEntryAtLinePosition(intellisenseCatalog, 'value >>> 1;', 7);
  assert(operatorHit && operatorHit.entry.label === '>>>', 'semantic dictionary must resolve >>> at cursor');
  const modeHit = core.lookupCatalogEntryAtLinePosition(intellisenseCatalog, 'root :: machine {', 9);
  assert(modeHit && modeHit.entry.label === 'machine', 'semantic dictionary must resolve machine role');
  const projectionHit = core.lookupCatalogEntryAtLinePosition(intellisenseCatalog, 'value <- load T [address];', 12);
  assert(projectionHit && projectionHit.entry.label === 'load T [address]', 'semantic dictionary must resolve material load');

  assert.strictEqual(pkg.license, 'MIT OR Apache-2.0');
  assert.strictEqual(pkg.version, '1.0.0-21v');
  assert.strictEqual(pkg.ncProductVersion, '1.0.0.21v');
  assert.strictEqual(pkg.ncProfessionalIdeParity?.tier, 'c_cpp_rust_java_grade');
  for (const lang of ['c', 'cpp', 'rust', 'java']) {
    assert(pkg.ncProfessionalIdeParity.referenceLanguages.includes(lang), `professional parity missing ${lang}`);
  }
  for (const surface of [
    'liveCompilerDiagnostics',
    'semanticTokens',
    'completion',
    'hoverDocumentation',
    'inlayHints',
    'goToDefinition',
    'realNavigation',
    'projectImportNavigation',
    'closedProjectImportResolution',
    'findReferences',
    'safeRename',
    'predictiveCompletions',
    'codeLensRunDebug',
    'runCheckFormat',
    'documentFormatting',
    'nfmtCompactFormatter',
    'officialCompactSyntax',
    'legacyVerboseCompatibility',
    'stableTypeSurface',
    'externalStoudStdlibCatalog',
    'stoudIoMemoryCollectionsMathTimeFsNetProcessThread',
    'genericArityTyping',
    'vectorQueueTypeSurface',
    'nestedAggregateFieldCompletion',
    'functionValueTyping',
    'compoundOperatorTyping',
    'universalRunDebug',
    'nccDebugAdapter',
    'nccKitResolution',
    'bridgeJsonAwareness',
    'nccCodeJsonAwareness',
    'projectGraphView',
    'projectMotherChildDetection',
    'projectDuplicateContainerWarnings',
    'perExtensionFileIcons',
    'fileIconIsolation',
    'externalThemeCompatibility'
  ]) {
    assert(pkg.ncProfessionalIdeParity.surfaces.includes(surface), `professional parity surface missing ${surface}`);
  }
  assert(pkg.repository && pkg.repository.url);
  assert(pkg.scripts && pkg.scripts.test && pkg.scripts['package:vsix']);
  assert(pkg.scripts['package:vsix'].includes('--out dist/'), 'package:vsix must output to dist/');
  assert(pkg.scripts['package:vsix'].includes('nc-language-1.0.0-21v.vsix'), 'package:vsix must target the 1.0.0.21v artifact');
  assert.strictEqual(pkg.icon, 'assets/nc-logo-128.png', 'VS Code package icon must use the nC logo asset');
  assert.strictEqual(pkg.contributes.configuration.properties['nc.editorAssistHints'].default, true, 'editor assist hints must default on');
  assert.strictEqual(pkg.contributes.configuration.properties['nc.inlayHints.parameters'].default, true, 'parameter inlay hints must default on');
  assert.strictEqual(pkg.contributes.configuration.properties['nc.inlayHints.types'].default, true, 'type inlay hints must default on');
  assert(!pkg.contributes.configuration.properties['nc.inlayHints.parameters'].markdownDescription.includes('semantic authority'), 'inlay setting copy must not expose internal authority wording');
  assert.strictEqual(pkg.contributes.configuration.properties['nc.nccPath'].default, 'auto', 'ncc path must default to auto discovery');
  assert.strictEqual(pkg.contributes.configuration.properties['nc.ncupPath'].default, 'auto', 'ncup path must default to auto discovery');
  assert.strictEqual(pkg.contributes.configuration.properties['nc.kit'].default, 'auto', 'ncc kit must default to auto discovery');
  assert.strictEqual(pkg.contributes.configuration.properties['nc.kitTransport'].default, 'native', 'ncc kit transport must default to native');
  assert(pkg.contributes.configuration.properties['nc.formatter'].markdownDescription.includes('nfmt --compact'), 'formatter setting must document compact normal form');
  assert.strictEqual(pkg.contributes.configuration.properties['nc.runHost'].default, 'auto', 'run host must default to auto');
  assert(pkg.contributes.configuration.properties['nc.runHost'].enum.includes('kali'), 'run host must expose Kali routing');
  assert(pkg.contributes.configuration.properties['nc.runHost'].enum.includes('ubuntu'), 'run host must expose Ubuntu routing');
  assert(!Object.prototype.hasOwnProperty.call(pkg.contributes.configuration.properties, 'nc.lowLevelRunOutsideIde'), 'low-level files must not expose an in-IDE run override');
  assert(!pkg.contributes.configuration.properties['nc.machineSystemLoader'], '.nm IDE loader setting must not exist');
  assert(!Object.prototype.hasOwnProperty.call(pkg.contributes.configuration.properties, 'nc.inlayHints.entrypoints'), 'entrypoint inlay hints must not be exposed');

  assert.strictEqual(stoudStdlibCatalog.schema, 'nc.stoud.stdlib.catalog.v1');
  assert.deepStrictEqual(
    stoudStdlibCatalog.canonicalFamilies,
    ['io', 'data', 'memory', 'collections', 'math', 'time', 'fs', 'net', 'process', 'thread'],
    'stoud stdlib families must keep canonical order'
  );
  for (const family of stoudStdlibCatalog.families) {
    assert(stoudStdlibCatalog.canonicalFamilies.includes(family.family), `unexpected stoud family ${family.family}`);
    assert(Array.isArray(family.items) && family.items.length > 0, `stoud family ${family.family} must expose items`);
    assert(Array.isArray(family.functions) && family.functions.length > 0, `stoud family ${family.family} must document functions`);
    for (const fn of family.functions) {
      assert(fn.name && fn.signature, `stoud ${family.family} function missing name/signature`);
      assert(Array.isArray(fn.params), `stoud ${fn.name} must document params`);
      assert(Object.prototype.hasOwnProperty.call(fn, 'returns'), `stoud ${fn.name} must document return type`);
      assert(Array.isArray(fn.errors), `stoud ${fn.name} must document errors`);
      assert(fn.backend && /backend|runtime|native|hosted|descriptor|filesystem|network|thread|process|intrinsic/i.test(fn.backend), `stoud ${fn.name} must document backend lowering`);
    }
  }

  const languageRows = pkg.contributes.languages || [];
  const allExtensions = languageRows.flatMap((row) => row.extensions || []);
  for (const ext of ['.n', '.nb', '.nbb', '.nm', '.ncp', '.nbo', '.init']) {
    assert(allExtensions.includes(ext), `missing language extension ${ext}`);
  }
  assert(!allExtensions.includes('.nc'), 'legacy .nc extension must not be claimed by the nC plugin');
  for (const ext of ['main.n', 'root.nb', 'basal.nbb', 'machine.nm', 'primitive.ncp', 'backend.nbo', 'startup.init']) {
    assert(core.supportedNcExtension(ext), `supported extension not recognized: ${ext}`);
  }
  assert(!core.supportedNcExtension('legacy.nc'), 'legacy .nc extension should not be recognized');
  assert(!core.supportedNcExtension('notes.txt'), 'non-nC extension should not be recognized');
  assert.deepStrictEqual(core.diagnosticLocationFromMessage('[33:21] Expected Semicolon'), { line: 33, column: 21 });
  assert.strictEqual(core.diagnosticCodeFromMessage('NC-SYS-LSP-MIG-0002: rename file'), 'NC-SYS-LSP-MIG-0002');

  const commandTitles = (pkg.contributes.commands || []).map((row) => row.title);
  for (const title of [
    'nC: Check Current File',
    'nC: Run Current File',
    'nC: Debug Current File',
    'nC: Format File',
    'nC: Create ncc Kit',
    'nC: Select ncc Kit',
    'nC: Show Compiler Diagnostics',
    'nC: Show Compiler Pipeline',
    'nC: Show Compiler AST',
    'nC: Show Compiler IR',
    'nC: Show Project Graph',
    'nC: Show Runtime Families',
    'nC: Show Runtime Exports',
    'nC: Show Runtime Capabilities',
    'nC: Show Runtime Policies',
    'nC: Primitive Doctor',
    'nC: Primitive Check',
    'nC: Primitive Witness',
    'nC: Primitive Map',
    'nC: Primitive Lower',
    'nC: Primitive QEMU',
    'nC: Select Toolchain',
    'nC: Open Base Docs'
  ]) {
    assert(commandTitles.includes(title), `missing command ${title}`);
  }
  assert(!commandTitles.includes('nC: Machine Doctor'), 'forbidden command advertised: nC: Machine Doctor');
  assert(pkg.activationEvents.includes('onCommand:nc.debug'), 'debug command must activate the extension');
  assert(pkg.activationEvents.includes('onCommand:nc.showProjectGraph'), 'project graph command must activate the extension');
  assert(pkg.activationEvents.includes('onDebugResolve:nc'), 'nC debug resolver must activate the extension');
  assert(pkg.contributes.menus?.['editor/title']?.some((row) => row.command === 'nc.debug'), 'editor title must expose nC debug');
  assert(pkg.contributes.menus?.['editor/context']?.some((row) => row.command === 'nc.run'), 'editor context must expose nC run');
  assert(pkg.contributes.menus?.['editor/context']?.some((row) => row.command === 'nc.showProjectGraph'), 'editor context must expose nC project graph');
  const breakpointLangs = (pkg.contributes.breakpoints || []).map((row) => row.language);
  for (const lang of ['nc', 'nc-root', 'nc-primitive', 'nc-hardware']) {
    assert(breakpointLangs.includes(lang), `breakpoints missing ${lang}`);
  }
  assert(!breakpointLangs.includes('nc-machine'), '.nm must not expose IDE breakpoints');
  const debuggerRows = pkg.contributes.debuggers || [];
  assert(!debuggerRows.some((row) => row.type === 'nc' && row.languages.includes('nc-machine')), '.nm must not expose an IDE debugger lane');
  assert(debuggerRows.some((row) => row.type === 'nc' && row.languages.includes('nc-hardware')), 'nC debugger contribution missing hardware lane');

  for (const forbidden of [
    'Docs: `',
    'Source: `',
    'Compiler authority remains',
    'Prediction only',
    'Predicted import',
    'Local editor outline',
    'parser/sema truth',
    'nC predictive inlay hint',
    'nC type:',
    'nC `+` operator',
    'catalogDocsPath'
  ]) {
    assert(!extensionSource.includes(forbidden), `extension hover/definition copy leaked: ${forbidden}`);
  }
  for (const forbidden of [
    'ncc owns',
    'Local nC function signature',
    'ncc remains semantic authority'
  ]) {
    assert(!semanticSource.includes(forbidden), `semantic helper copy leaked: ${forbidden}`);
  }
  const inlayRows = core.inlayHintRows([
    'demo {',
    '  fn:',
    '    run() -> bool {',
    '      println("Hola");',
    '      return true;',
    '    }',
    '}'
  ].join('\n'));
  const valueHint = inlayRows.find((row) => row.call === 'println' && row.parameter === 'value');
  assert(valueHint, 'println(value:) inlay hint missing');
  assert(wordCount(valueHint.tooltip) <= 50, 'inlay hint tooltip must stay within 50 words');
  assert(!/ncc|semantic authority|source of truth|plugin/i.test(valueHint.tooltip), 'inlay hint tooltip must not expose internal authority/source wording');

  const requiredLicenseFiles = ['LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE'];
  for (const rel of requiredLicenseFiles) {
    assert(fs.existsSync(path.join(root, rel)), `${rel} missing`);
  }

  assert(fs.existsSync(themePath), 'nc-sovereign-dark-color-theme.json missing');
  assert(fs.existsSync(systemsThemePath), 'nc-systems-spectrum-color-theme.json missing');
  assert(fs.existsSync(polyglotThemePath), 'nc-polyglot-systems-color-theme.json missing');
  assert(fs.existsSync(iconThemePath), 'icons/nc-icon-theme.json missing');
  assert(fs.existsSync(grammarPath), 'nc.tmLanguage.json missing');
  assert(fs.existsSync(basalGrammarPath), 'nc-basal.tmLanguage.json missing');
  assert(fs.existsSync(machineGrammarPath), 'nc-machine.tmLanguage.json missing');
  assert(fs.existsSync(backendObjectGrammarPath), 'nc-backend-object.tmLanguage.json missing');
  assert(fs.existsSync(targetInitGrammarPath), 'nc-target-init.tmLanguage.json missing');
  assert(fs.existsSync(primitiveGrammarPath), 'nc-primitive.tmLanguage.json missing');
  assert(fs.existsSync(hardwareGrammarPath), 'nc-hardware.tmLanguage.json missing');
  assert(fs.existsSync(snippetsPath), 'nc.code-snippets missing');
  assert(fs.existsSync(colorPalettePath), 'tools/theme/nc_colors.json missing');
  assert(fs.existsSync(semanticTokensPath), 'tools/theme/nc_semantic_tokens.json missing');
  assert(fs.existsSync(visualIdentityPath), 'tools/theme/nc_visual_identity.json missing');
  assert(fs.existsSync(baseLogoPath), 'site/public/assets/nc-logo.svg missing');
  assert(fs.existsSync(extensionLogoPath), 'assets/nc-logo.svg missing');
  assert(fs.existsSync(path.join(root, 'assets', 'nc-logo-128.png')), 'assets/nc-logo-128.png missing');
  assert(fs.existsSync(path.join(root, 'assets', 'nc-logo-256.png')), 'assets/nc-logo-256.png missing');
  assert(fs.existsSync(path.join(root, 'assets', 'marketplace', 'nc-logo-128.png')), 'assets/marketplace/nc-logo-128.png missing');
  assert(fs.existsSync(path.join(root, 'assets', 'marketplace', 'nc-logo-256.png')), 'assets/marketplace/nc-logo-256.png missing');
  assert.strictEqual(normalizedText(extensionLogoPath), normalizedText(baseLogoPath), 'VS Code SVG logo must match site/public/assets/nc-logo.svg');
  assert(fs.statSync(path.join(root, 'assets', 'nc-logo-128.png')).size > 1024, 'assets/nc-logo-128.png looks empty');
  assert(fs.statSync(path.join(root, 'assets', 'nc-logo-256.png')).size > 2048, 'assets/nc-logo-256.png looks empty');

  const colorPalette = readJson(colorPalettePath);
  const visualIdentity = readJson(visualIdentityPath);
  assert(visualIdentity.palettes.includes('nC Polyglot Systems'), 'visual identity missing nC Polyglot Systems');
  assert.strictEqual(visualIdentity.logo.official_svg, 'site/public/assets/nc-logo.svg');
  assert.strictEqual(visualIdentity.polyglot_systems.rust_hazard, '#CE412B');

  const colors = colorPalette.colors || {};
  for (const [name, value] of Object.entries(colors)) {
    assert(value.category, `missing category for ${name}`);
    assert(Array.isArray(value.rgb) && value.rgb.length === 3, `missing rgb for ${name}`);
    assert(value.hex, `missing hex for ${name}`);
    assert.strictEqual(value.hex.toUpperCase(), hexFromRgb(value.rgb), `hex/rgb mismatch for ${name}`);
  }

  const requiredCriticalColors = {
    'polyglot.java.primary': '#F89820',
    'polyglot.java.secondary': '#5382A1',
    'polyglot.python.primary': '#3776AB',
    'polyglot.python.secondary': '#FFD43B',
    'polyglot.c.primary': '#A8B9CC',
    'polyglot.cpp.primary': '#00599C',
    'polyglot.cpp.secondary': '#659AD2',
    'polyglot.rust.primary': '#DEA584',
    'polyglot.rust.hazard': '#CE412B',
    'primitive.signal': '#CE412B',
    'machine.instruction': '#00599C',
    'type.int': '#A8B9CC',
    'function.name': '#659AD2',
    'literal.int': '#F89820',
    'runtime.input': '#5382A1',
    'compiler.sema': '#F89820',
    'raw.hazard': '#CE412B'
  };
  for (const [key, expected] of Object.entries(requiredCriticalColors)) {
    assert(colors[key], `critical color ${key} missing`);
    assert.strictEqual(colors[key].hex.toUpperCase(), expected);
  }

  const theme = readJson(themePath);
  assert(theme.name === 'nC Sovereign Dark');
  assert(pkg.contributes && Array.isArray(pkg.contributes.themes));
  assert(pkg.contributes.themes.some((t) => t.path === './themes/nc-sovereign-dark-color-theme.json'));
  assert(pkg.contributes.themes.some((t) => t.path === './themes/nc-aqua-runtime-color-theme.json'));
  assert(pkg.contributes.themes.some((t) => t.path === './themes/nc-crimson-diagnostics-color-theme.json'));
  assert(pkg.contributes.themes.some((t) => t.path === './themes/nc-systems-spectrum-color-theme.json'));
  assert(pkg.contributes.themes.some((t) => t.path === './themes/nc-polyglot-systems-color-theme.json'));
  assert(pkg.contributes.iconThemes.some((t) => t.id === 'nc-icons' && t.path === './icons/nc-icon-theme.json'));

  const iconTheme = readJson(iconThemePath);
  const legacyIconThemePath = path.join(root, 'themes', 'nc-file-icons.json');
  const legacyIconTheme = readJson(legacyIconThemePath);
  assert(!Object.prototype.hasOwnProperty.call(iconTheme, 'file'), 'icon theme must not override every file icon');
  assert(!Object.prototype.hasOwnProperty.call(iconTheme.iconDefinitions || {}, '_file'), 'icon theme must not define a global file icon');
  assert(!Object.prototype.hasOwnProperty.call(iconTheme.iconDefinitions || {}, '_folder'), 'icon theme must not define a global folder icon');
  assert(!Object.prototype.hasOwnProperty.call(iconTheme, 'fileNames'), 'icon theme must not claim specific filenames');
  assert(!Object.prototype.hasOwnProperty.call(iconTheme, 'languageIds'), 'icon theme must not claim language ids');
  assert(!Object.prototype.hasOwnProperty.call(iconTheme, 'folderNames'), 'icon theme must not claim folder names');
  assert(!Object.prototype.hasOwnProperty.call(iconTheme.fileExtensions || {}, 'nc'), 'icon theme must not claim .nc');
  const allowedIconExtensions = ['n', 'nm', 'nb', 'nbb', 'ncp', 'nbo', 'init'];
  assert.deepStrictEqual(Object.keys(iconTheme.fileExtensions || {}).sort(), allowedIconExtensions.slice().sort(), 'icon theme must claim only nC extensions');
  const expectedIconMappings = {
    n: 'ncHighLevel',
    nm: 'ncMachine',
    nb: 'ncBasal',
    nbb: 'ncPrimitiveBasal',
    ncp: 'ncPrimitiveCapability',
    nbo: 'ncBackendObject',
    init: 'ncTargetInit'
  };
  for (const [ext, definition] of Object.entries(expectedIconMappings)) {
    assert.strictEqual(iconTheme.fileExtensions[ext], definition, `icon theme missing ${ext}`);
    assert(iconTheme.iconDefinitions[definition], `icon definition missing ${definition}`);
    assert(fs.existsSync(path.join(root, iconTheme.iconDefinitions[definition].iconPath.replace(/^\.\.\//, ''))), `icon asset missing for ${ext}`);
  }
  for (const foreign of ['c', 'cc', 'cpp', 'cxx', 'h', 'hpp', 'rs', 'java', 'md', 'json', 'toml', 'yaml', 'yml', 'js', 'ts']) {
    assert(!Object.prototype.hasOwnProperty.call(iconTheme.fileExtensions || {}, foreign), `icon theme must not claim .${foreign}`);
  }
  assert.deepStrictEqual(legacyIconTheme.fileExtensions, iconTheme.fileExtensions, 'legacy icon metadata must match active icon theme');
  assert.deepStrictEqual(Object.keys(legacyIconTheme.iconDefinitions || {}).sort(), Object.keys(iconTheme.iconDefinitions || {}).sort(), 'legacy icon definitions must match active icon theme');
  assert(!fs.existsSync(path.join(root, 'assets', 'file-icons', 'nc-file-nc.svg')), 'legacy .nc icon asset must not be packaged');
  assert(!fs.existsSync(path.join(root, 'icons', 'nc-file.svg')), 'legacy .nc icon copy must not remain');

  const systemsTheme = readJson(systemsThemePath);
  const polyglotTheme = readJson(polyglotThemePath);
  assert.strictEqual(systemsTheme.semanticTokenColors.pointerType, '#DEA584');
  assert.strictEqual(systemsTheme.semanticTokenColors.machineInstruction, '#00599C');
  assert.strictEqual(systemsTheme.semanticTokenColors.rawHazard, '#CE412B');
  assert.strictEqual(systemsTheme.semanticTokenColors.modeRuntime, '#5382A1');
  assert.strictEqual(systemsTheme.semanticTokenColors.modeMachine, '#00599C');
  assert.strictEqual(systemsTheme.semanticTokenColors.modePrimitive, '#CE412B');
  assert.strictEqual(systemsTheme.semanticTokenColors.importItem, '#FFD43B');
  assert.strictEqual(systemsTheme.semanticTokenColors.stringEscape, '#FFD43B');
  assert.strictEqual(systemsTheme.semanticTokenColors.printFunction, '#5382A1');
  assert.strictEqual(systemsTheme.semanticTokenColors.pointerFunction, '#DEA584');
  assert.strictEqual(systemsTheme.semanticTokenColors.colorFunction, '#3A7D44');
  assert.strictEqual(systemsTheme.semanticTokenColors.controlFlow, '#F8961E');
  assert.strictEqual(systemsTheme.semanticTokenColors.loopControl, '#F9C74F');
  assert.strictEqual(systemsTheme.semanticTokenColors.kernelAddressKeyword, '#43AA8B');
  assert.strictEqual(systemsTheme.semanticTokenColors.kernelMmio32, '#F94144');
  assert.strictEqual(systemsTheme.semanticTokenColors.kernelMmio64, '#A4568D');
  assert.notStrictEqual(
    systemsTheme.semanticTokenColors.kernelMmio128,
    systemsTheme.semanticTokenColors.kernelMmio256,
    'each compact MMIO width must retain an independent color'
  );
  assert.strictEqual(polyglotTheme.name, 'nC Polyglot Systems');
  assert.strictEqual(polyglotTheme.colors['editor.foreground'], '#D9E6F2');
  assert.strictEqual(polyglotTheme.semanticTokenColors.section, '#3776AB');
  assert.strictEqual(polyglotTheme.semanticTokenColors.type, '#FFD43B');
  assert.strictEqual(polyglotTheme.semanticTokenColors.function, '#3776AB');
  assert.strictEqual(polyglotTheme.semanticTokenColors.number, '#FFD43B');

  const semanticTypes = (pkg.contributes.semanticTokenTypes || []).map((row) => row.id);
  for (const token of [
    'mode', 'modeRuntime', 'modeCompiler', 'modeSystem', 'modeMachine', 'modePrimitive', 'modeKernel', 'modeBasal',
    'basalEntity', 'kernelEntity', 'section', 'type', 'pointerType', 'function', 'printFunction',
    'pointerFunction', 'colorFunction', 'controlFlow', 'loopControl', 'kernelAddressKeyword',
    'kernelMmio8', 'kernelMmio16', 'kernelMmio32', 'kernelMmio64', 'kernelMmio128',
    'kernelMmio256', 'kernelMmio512', 'operator', 'runtimeFamily', 'compilerPhase',
    'primitiveEntity', 'hardwareEntity', 'machineRegister', 'machineInstruction', 'ffiSymbol', 'capability',
    'policy', 'diagnostic', 'rawHazard', 'importHint', 'importKeyword', 'importDomain', 'importFamily', 'importItem',
    'stcSection', 'typeSection', 'localSection', 'constSection', 'fnSection', 'faSection',
    'stringEscape', 'atReference', 'atGroup', 'copyClone', 'destructure', 'constructure'
  ]) {
    assert(semanticTypes.includes(token), `semantic token type missing: ${token}`);
  }

  const semanticModifiers = (pkg.contributes.semanticTokenModifiers || []).map((row) => row.id);
  for (const modifier of [
    'readonly', 'unsafe', 'strict', 'primitiveOnly', 'machineOnly', 'runtimeOnly',
    'deprecated', 'experimental', 'error', 'warning', 'evidence', 'sealed', 'redacted'
  ]) {
    assert(semanticModifiers.includes(modifier), `semantic token modifier missing: ${modifier}`);
  }

  const grammar = readJson(grammarPath);
  const modePattern = JSON.stringify(grammar.repository?.modes || {});
  const importPattern = JSON.stringify(grammar.repository?.imports || {});
  const sectionPattern = JSON.stringify(grammar.repository?.sections || {});
  const primitivePattern = grammar.repository?.primitiveentities?.patterns?.[0]?.match || '';
  const hardwarePattern = grammar.repository?.hardwareentities?.patterns?.[0]?.match || '';
  const ffiPattern = grammar.repository?.ffientities?.patterns?.[0]?.match || '';
  const nominalPattern = grammar.repository?.destructureconstructure?.patterns?.[0]?.match || '';
  const copyClonePattern = grammar.repository?.copyclone?.patterns?.[0]?.match || '';
  const diagnosticPattern = grammar.repository?.diagnostics?.patterns?.[0]?.match || '';
  const rawHazardPattern = grammar.repository?.rawhazards?.patterns?.[0]?.match || '';
  const atGroupPattern = grammar.repository?.atgroups?.patterns?.[0]?.match || '';
  const kernelCompactPattern = JSON.stringify(grammar.repository?.kernelcompact || {});

  for (const fragment of ['role.program', 'role.runtime', 'role.lowlevel']) {
    assert(modePattern.includes(fragment), `grammar missing role scope ${fragment}`);
  }
  for (const fragment of ['program', 'runtime', 'machine', 'primitive', 'kernel']) {
    assert(modePattern.includes(fragment), `grammar missing role ${fragment}`);
  }
  for (const fragment of ['meta.import.facade.nc', 'support.other.import.domain.nc', 'support.other.import.family.nc', 'support.other.import.item.nc']) {
    assert(importPattern.includes(fragment), `grammar missing facade import scope ${fragment}`);
  }
  for (const fragment of ['section.const.nc', 'section.type.nc', 'section.stc.nc', 'section.local.nc', 'section.function.nc', 'section.fa.nc', 'modifier.visibility.nc', 'section.static.nc', 'section.stack.nc', 'storage.type.abi.nc']) {
    assert(sectionPattern.includes(fragment), `grammar missing hybrid section scope ${fragment}`);
  }
  assert(atGroupPattern.includes('(@)'), 'grammar must color grouped @ references like @(name)');
  assert(atGroupPattern.includes('[A-Za-z_][A-Za-z0-9_'), 'grammar must capture grouped @ reference names');
  for (const fragment of ['kernel.address', 'mmio.width32', 'mmio.width64', 'mmio.width128', 'mmio.width256']) {
    assert(kernelCompactPattern.includes(fragment), `grammar missing compact kernel color scope ${fragment}`);
  }
  for (const fragment of [
    'thermalState',
    'throttleState',
    'cacheState',
    'tlbState',
    'pipelineState',
    'branchPredictorState',
    'reorderState',
    'outOfOrder',
    'powerGate',
    'firmwareRegion',
    'bootRom',
    'microcodeState',
    'physicalFault',
    'physicalViolation'
  ]) {
    assert(primitivePattern.includes(fragment), `grammar missing primitive entity: ${fragment}`);
    assert(hardwarePattern.includes(fragment), `grammar missing hardware entity: ${fragment}`);
  }
  for (const fragment of ['foreign\\.c', 'foreign\\.cpp', 'foreign\\.rust']) {
    assert(ffiPattern.includes(fragment), `grammar missing ffi symbol: ${fragment}`);
  }
  for (const fragment of ['ownership', 'lifetime', 'nullability', 'nullable', 'nonnullable']) {
    assert(ffiPattern.includes(fragment), `grammar missing ffi contract token: ${fragment}`);
  }
  for (const fragment of ['destructure', 'constructure']) {
    assert(nominalPattern.includes(fragment), `grammar missing nominal token: ${fragment}`);
  }
  for (const fragment of ['copy\\.value', 'clone\\.deep']) {
    assert(copyClonePattern.includes(fragment), `grammar missing copy/clone token: ${fragment}`);
  }
  assert(diagnosticPattern.includes('PTR'), 'grammar must highlight pointer diagnostics');
  assert(rawHazardPattern.includes('rawPtr'), 'grammar must highlight raw pointer hazards');
  assert(rawHazardPattern.includes('evidence\\s+boundary'), 'grammar must highlight evidence boundary hazards');
  assert(JSON.stringify(grammar.repository?.strings || {}).includes('print-control'), 'grammar must highlight print escape controls');
  assert(JSON.stringify(grammar.repository?.strings || {}).includes('invalid.illegal.escape.nc'), 'grammar must mark unknown string escapes');
  assert(JSON.stringify(grammar.repository?.printfamily || {}).includes('colorBold'), 'grammar must highlight color helpers');
  assert(JSON.stringify(grammar.repository?.operators || {}).includes('\\\\+\\\\+'), 'grammar must highlight compact increment');
  assert(JSON.stringify(grammar.repository?.operators || {}).includes('\\\\+='), 'grammar must highlight compact add assignment');
  assert(JSON.stringify(grammar.repository?.types || {}).includes('true|false'), 'grammar must keep bool literals separate from bool type');
  assert(JSON.stringify(grammar.repository?.types || {}).includes('byte|bool|str'), 'grammar must classify bool as a basic type');
  assert(JSON.stringify(grammar.repository?.types || {}).includes('queue|vector'), 'grammar must classify queue/vector aggregate types');
  assert(JSON.stringify(grammar.repository?.types || {}).includes('ft32|ft64'), 'grammar must classify ft32/ft64 numeric types');
  assert(JSON.stringify(grammar.repository?.types || {}).includes('support.type.function.nc'), 'grammar must classify function value types');
  assert(JSON.stringify(grammar.repository?.types || {}).includes('[0-9]+'), 'grammar generics must accept array<T, N> arity');

  assert(intellisenseCatalog.symbols.some((row) => row.label === 'color' && row.facadeImport === 'use <io:output, render>;'));
  assert(intellisenseCatalog.symbols.some((row) => row.label === 'colorBold' && row.facadeImport === 'use <io:output, render>;'));
  assert(intellisenseCatalog.symbols.some((row) => row.label === '\\n' && row.kind === 'print-control'));
  assert(intellisenseCatalog.symbols.some((row) => row.label === '##' && row.kind === 'print-control'));

  const snippets = readJson(snippetsPath);
  for (const snippet of [
    'helloWorld',
    'runtimeInputOutput',
    'ifElse',
    'matchGuardUnless',
    'memRegion',
    'siteAddr',
    'primitiveHardwareIdentityUart',
    'compilerReportSkeleton',
    'ffiCSkeleton',
    'ffiPointerContract',
    'pointerFfiContractFull',
    'pointerRegionFromPtr',
    'ptrNullable',
    'refMutable',
    'riskRawPointer',
    'importRuntimeInput',
    'importRuntimeOutput',
    'importStoudRichOutput',
    'printColorBold',
    'printSeparator',
    'printNewlineEscape',
    'printLiteralEscapes',
    'importRuntimeRegion',
    'importRuntimePtr',
    'compactIntDeclaration',
    'compactBoolDeclaration',
    'compactAddAssign',
    'compactIncrement',
    'basalRoutineRawReport',
    'destructureSkeleton',
    'constructureSkeleton',
    'copySkeleton',
    'cloneSkeleton'
  ]) {
    assert(Object.prototype.hasOwnProperty.call(snippets, snippet), `snippet missing: ${snippet}`);
  }
  assert(snippets.runtimeInputOutput.body.join('\n').includes('value >> int <- 0;'), 'runtime snippet must use official compact declaration');
  assert(snippets.runtimeInputOutput.body.join('\n').includes('value <- readInt();'), 'runtime snippet must use material assignment');
  assert(snippets.compactIntDeclaration.body.join('\n') === '${1:contador} >> int <- ${2:0};', 'compact int snippet must match official syntax');
  assert(snippets.compactBoolDeclaration.body.join('\n') === '${1:activo} >> bool <- ${2:true};', 'compact bool snippet must match official syntax');
  assert(snippets.compactAddAssign.body.join('\n') === '${1:x} += ${2:1};', 'compact add-assign snippet must match official syntax');
  assert(snippets.compactIncrement.body.join('\n') === '${1:i}++;', 'compact increment snippet must match official syntax');

  const high = readFixture('high_level.n');
  const rootMode = readFixture('root_mode.nb');
  const machine = readFixture('machine_mode.nm');
  const primitive = readFixture('primitive_mode.nbb');
  const hardware = readFixture('primitive_mode.ncp');

  assert.strictEqual(core.detectModeFamily(high.path, high.text), 'high-level');
  assert.strictEqual(core.detectModeFamily(rootMode.path, rootMode.text), 'root-basal');
  assert.strictEqual(core.detectModeFamily(machine.path, machine.text), 'machine-low-level');
  assert.strictEqual(core.detectModeFamily(primitive.path, primitive.text), 'primitive-low-level');
  assert.strictEqual(core.detectModeFamily(hardware.path, hardware.text), 'hardware-low-level');

  assert.strictEqual(core.detectModeFamily('runtime_file.n', 'demo :: runtime {}'), 'runtime-level');
  assert.strictEqual(core.detectModeFamily('compiler_file.n', 'demo :: compiler {}'), 'compiler-level');
  assert.strictEqual(core.detectModeFamily('system_file.n', 'demo :: system {}'), 'system-level');
  assert.strictEqual(core.detectModeFamily('tooling_file.n', 'demo :: tooling {}'), 'tooling-level');
  assert.strictEqual(core.detectModeFamily('inspect_file.n', 'demo :: inspect {}'), 'inspect-level');
  assert.strictEqual(core.detectModeFamily('security_file.n', 'demo :: security {}'), 'security-level');
  assert.strictEqual(core.detectModeFamily('network_file.n', 'demo :: network {}'), 'network-level');
  assert.strictEqual(core.detectModeFamily('kernel_file.n', 'demo :: kernel {}'), 'kernel-level');
  assert.strictEqual(
    core.detectModeFamily('guest.n', 'guest :: kernel { run :: in() -> bool { lapic at 0xFEE00000; return true; } ref main ->> run; }'),
    'kernel-level'
  );
  assert.strictEqual(
    core.detectModeFamily('guest.n', 'guest :: kernel { run :: in() -> bool { *256(0x1000, 0); return true; } ref main ->> run; }'),
    'kernel-level'
  );
  assert.strictEqual(
    core.detectModeFamily('boot.n', 'kernelBoot :: kernel { run :: in() -> bool { return true; } ref main ->> run; }'),
    'kernel-level'
  );
  assert.strictEqual(
    core.detectModeFamily('library.n', 'kernelHelpers :: kernel { probe :: in() -> bool { return true; } }'),
    'kernel-level'
  );
  assert.strictEqual(
    core.detectModeFamily('ordinary.n', 'ordinary :: program { run :: in() -> bool { text <- "*32(p, 1)"; /* lapic at 1; */ return true; } ref main ->> run; }'),
    'program-level'
  );

  const parsed = core.parseSymbols(high.text);
  assert(parsed.containers.some((row) => row.name === 'demoMainBasico'));
  assert(parsed.variables.some((row) => row.name === 'r'));
  assert(parsed.mains.some((row) => row.kind === 'ref-main'));
  const navigationParsed = core.parseSymbols(`
navDemo {
    stc:
        total >> int <- 0;

    fn:
        public:
            run(nombre >> str) -> bool {
                total <- 1;
                return true;
            }
    ref main ->> run;

    fa:
        return true;
}
`);
  assert(navigationParsed.functions.some((row) => row.name === 'run'), 'navigation must parse functions');
  assert(navigationParsed.variables.some((row) => row.name === 'total'), 'navigation must parse compact variables');
  assert(navigationParsed.parameters.some((row) => row.name === 'nombre'), 'navigation must parse parameters');

  const occ = core.collectOccurrences(high.text, 'a');
  assert(occ.length >= 2);
  const codeOcc = core.collectCodeOccurrences(`
demo {
    stc:
        valor >> int <- 0;
    fn:
        public:
            run() -> bool {
                valor <- valor + 1;
                println("valor");
                // valor in comment
                /* valor in block comment */
                return true;
            }
}
`, 'valor');
  assert.strictEqual(codeOcc.length, 3, 'code occurrences must ignore strings and comments');

  const spans = core.semanticTokenSpans(`
// print and type inside this comment stay gray
/* state {
   println("not code");
*/
use <io:input, output, render>;

runtimeMode :: runtime {
    Packet :: type { tag >> int; }
    run :: in() -> bool {
                stack {
                    localValue >> int;
                }
                destructure.static(readInt());
                constructure.static(readInt());
                copy.value(@x);
                clone.deep(@x);
                inspect(@(x));
                foreign.c;
                p mut<byte>;
                medida >> ft32 <- 1.5;
                okResult >> Result<int, str>;
                packets >> vector<Packet>;
                tabla >> array<Packet, 8>;
                contador >> int <- 0;
                activo >> bool <- true;
                contador += 1;
                contador++;
                site<byte>;
                E_PTR_ALIGNMENT_001;
                raw;
                use <io:input, output>;
                pWrite<byte>(pFromAddr<byte>(0x1000), 7);
                if (activo) {
                    continue;
                } else {
                    break;
                }
                print("plain");
                colorBold("strong", "cyan");
                println("line\\n amp\\& question\\? hash\\#");
                return true;
    }
    ref main ->> run;
}
`, { fileName: 'runtime_mode.n', languageId: 'nc' });

  assert(spans.some((row) => row.tokenType === 'comment' && row.length > 20));
  assert(!spans.some((row) => row.tokenType === 'section' && row.line === 2), 'block comments must protect section-looking text');
  assert(spans.some((row) => row.tokenType === 'modeRuntime'));
  assert(spans.some((row) => row.tokenType === 'importFamily'));
  assert(spans.some((row) => row.tokenType === 'importItem'));
  assert(spans.some((row) => row.tokenType === 'function'));
  assert(spans.some((row) => row.tokenType === 'typeSection'));
  assert(spans.some((row) => row.tokenType === 'section'));
  assert(spans.some((row) => row.tokenType === 'stringEscape'));
  assert(spans.some((row) => row.tokenType === 'destructure'));
  assert(spans.some((row) => row.tokenType === 'constructure'));
  assert(spans.some((row) => row.tokenType === 'copyClone'));
  assert(spans.some((row) => row.tokenType === 'ffiSymbol'));
  assert(spans.some((row) => row.tokenType === 'pointerType'));
  assert(spans.some((row) => row.tokenType === 'pointerFunction'));
  assert(spans.some((row) => row.tokenType === 'printFunction'));
  assert(spans.some((row) => row.tokenType === 'colorFunction'));
  assert(spans.some((row) => row.tokenType === 'controlFlow'));
  assert(spans.some((row) => row.tokenType === 'loopControl'));
  assert(spans.some((row) => row.tokenType === 'basicType'));
  assert(spans.some((row) => row.tokenType === 'arrayType'));
  assert(spans.some((row) => row.tokenType === 'genericType'));
  assert(spans.some((row) => row.tokenType === 'advancedType'));
  assert(spans.some((row) => row.tokenType === 'boolLiteral'));
  assert(spans.some((row) => row.tokenType === 'operator'));
  assert(spans.some((row) => row.tokenType === 'rawHazard'));
  assert(spans.some((row) => row.tokenType === 'diagnostic'));
  assert(spans.some((row) => row.tokenType === 'atGroup'));
  const orderedSpans = spans.slice().sort((a, b) => a.line - b.line || a.startChar - b.startChar);
  for (let i = 1; i < orderedSpans.length; i += 1) {
    const prev = orderedSpans[i - 1];
    const next = orderedSpans[i];
    assert(prev.line !== next.line || next.startChar >= prev.startChar + prev.length, 'semantic token spans must not overlap');
  }

  const mixedModeText = 'mov(); physicalFault(); print("ok"); pWrite<int>(p, 1);';
  const sourceModeSpans = core.semanticTokenSpans(mixedModeText, { fileName: 'demo.n', languageId: 'nc' });
  assert(!sourceModeSpans.some((row) => row.tokenType === 'machineInstruction'), 'machine colors must not leak into .n');
  assert(!sourceModeSpans.some((row) => row.tokenType === 'primitiveEntity'), 'primitive colors must not leak into .n');
  assert(!sourceModeSpans.some((row) => row.tokenType === 'hardwareEntity'), 'hardware colors must not leak into .n');
  assert(sourceModeSpans.some((row) => row.tokenType === 'printFunction'));
  assert(sourceModeSpans.some((row) => row.tokenType === 'pointerFunction'));

  const kernelSpans = core.semanticTokenSpans(`
  kernelColors {
    touch() -> bool {
        lapic at 0xFEE00000;
        value = load32(lapic);
        *8(lapic, value);
        *16(lapic, value);
        *32(lapic, value);
        *64(lapic, value);
        *128(lapic, value);
        *256(lapic, value);
        *512(lapic, value);
        return true;
    }
    main: touch;
}
`, { fileName: 'kernelColors.n', languageId: 'nc' });
  assert(kernelSpans.some((row) => row.tokenType === 'kernelAddressKeyword'));
  for (const width of [8, 16, 32, 64, 128, 256, 512]) {
    assert(
      kernelSpans.some((row) => row.tokenType === `kernelMmio${width}`),
      `kernel MMIO semantic token missing for width ${width}`
    );
  }
  const ordinaryAtSpans = core.semanticTokenSpans('value = at; product = value * 32;', { fileName: 'ordinary.n', languageId: 'nc' });
  assert(!ordinaryAtSpans.some((row) => row.tokenType === 'kernelAddressKeyword'), 'kernel at color must not leak into ordinary .n');
  assert(!ordinaryAtSpans.some((row) => row.tokenType === 'kernelMmio32'), 'kernel MMIO color must not leak into ordinary .n');

  const machineModeSpans = core.semanticTokenSpans(mixedModeText, { fileName: 'demo.nm', languageId: 'nc-machine' });
  assert(machineModeSpans.some((row) => row.tokenType === 'machineInstruction'), '.nm must activate machine colors');
  assert(!machineModeSpans.some((row) => row.tokenType === 'primitiveEntity'), '.nm must not activate primitive colors');
  assert(!machineModeSpans.some((row) => row.tokenType === 'hardwareEntity'), '.nm must not activate hardware colors');
  const expandedIsaInstructions = [
    'endbr64', 'wbnoinvd', 'getsec', 'cld', 'rdpmc', 'rdpid',
    'sb', 'esb', 'tsb', 'dgh', 'xaflag', 'axflag', 'wfet', 'wfit',
    'yield', 'cpsid', 'cpsie', 'hvc', 'smc', 'dbg',
    'fence.tso', 'hfence.vvma', 'hfence.gvma', 'hinval.vvma', 'hinval.gvma',
    'hlv.w', 'hlvx.hu', 'hsv.w', 'sfence.w.inval', 'sfence.inval.ir', 'sinval.vma',
    'table.get', 'table.set', 'table.copy', 'table.init', 'elem.drop',
    'memory.init', 'data.drop', 'ref.null', 'ref.func',
  ];
  const expandedIsaSpans = core.semanticTokenSpans(
    `${expandedIsaInstructions.join('; ')};`,
    { fileName: 'expanded.nm', languageId: 'nc-machine' }
  );
  assert.strictEqual(
    expandedIsaSpans.filter((row) => row.tokenType === 'machineInstruction').length,
    expandedIsaInstructions.length,
    'new cross-architecture instructions must receive machine coloring'
  );

  const primitiveModeSpans = core.semanticTokenSpans(mixedModeText, { fileName: 'demo.nbb', languageId: 'nc-primitive' });
  assert(primitiveModeSpans.some((row) => row.tokenType === 'primitiveEntity'), 'primitive files must activate primitive colors');
  assert(!primitiveModeSpans.some((row) => row.tokenType === 'hardwareEntity'), 'primitive files must not activate hardware colors');
  assert(!primitiveModeSpans.some((row) => row.tokenType === 'machineInstruction'), 'primitive files must not activate machine colors');

  const hardwareModeSpans = core.semanticTokenSpans(mixedModeText, { fileName: 'demo.ncp', languageId: 'nc-hardware' });
  assert(hardwareModeSpans.some((row) => row.tokenType === 'hardwareEntity'), 'hardware files must activate physical hardware colors');
  assert(!hardwareModeSpans.some((row) => row.tokenType === 'primitiveEntity'), 'hardware files must not reuse primitive semantic colors');
  assert(!hardwareModeSpans.some((row) => row.tokenType === 'machineInstruction'), 'hardware files must not activate machine colors');

  const x86MachineCompletions = core.lowLevelCompletionRows('oracle.nm', 'oracle :: machine {\n    target:\n        arch: x86_64;\n}', '                cld');
  assert.strictEqual(x86MachineCompletions.length, 0, '.nm must remain fully manual');
  const aarch64MachineCompletions = core.lowLevelCompletionRows('oracle.nm', 'oracle :: machine {\n    target:\n        arch: aarch64;\n}', '                pac');
  assert.strictEqual(aarch64MachineCompletions.length, 0, '.nm must not suggest PAUTH');
  const machineArchCompletions = core.lowLevelCompletionRows('oracle.nm', 'oracle :: machine {', '        arch: ');
  assert.strictEqual(machineArchCompletions.length, 0, '.nm target architecture is manual');
  const basalCompletions = core.lowLevelCompletionRows('root.nb', 'root :: basal {', '    si');
  assert.strictEqual(basalCompletions.length, 0, '.nb must remain fully manual');
  const primitiveCompletions = core.lowLevelCompletionRows('material.nbb', 'primitive[@] target x86_64:', '    enc');
  assert.strictEqual(primitiveCompletions.length, 0, '.nbb must remain fully manual');
  const hardwareCompletions = core.lowLevelCompletionRows('board.ncp', 'primitive[@] target aarch64:', '    firm');
  assert.strictEqual(hardwareCompletions.length, 0, '.ncp must remain fully manual');
  const lowLevelLanguageIds = ['nc-root', 'nc-machine', 'nc-primitive', 'nc-hardware'];
  const snippetLanguages = new Set((pkg.contributes.snippets || []).map((row) => row.language));
  for (const languageId of lowLevelLanguageIds) {
    assert(!snippetLanguages.has(languageId), `${languageId} must not contribute snippets`);
    const defaults = pkg.contributes.configurationDefaults[`[${languageId}]`];
    assert(defaults, `${languageId} manual editor defaults missing`);
    assert.strictEqual(defaults['editor.quickSuggestions'], false, `${languageId} quick suggestions must be disabled`);
    assert.strictEqual(defaults['editor.suggestOnTriggerCharacters'], false, `${languageId} trigger suggestions must be disabled`);
    assert.strictEqual(defaults['editor.wordBasedSuggestions'], 'off', `${languageId} word suggestions must be disabled`);
    assert.strictEqual(defaults['editor.inlineSuggest.enabled'], false, `${languageId} inline suggestions must be disabled`);
    assert.strictEqual(defaults['editor.snippetSuggestions'], 'none', `${languageId} snippet suggestions must be disabled`);
    assert.strictEqual(defaults['editor.inlayHints.enabled'], 'off', `${languageId} predictive inlays must be disabled`);
  }
  const manualExtensionSource = readText(extensionSourcePath);
  assert(
    manualExtensionSource.includes("const assistedSourceSelector = [{ language: 'nc' }];"),
    'completion and inlay providers must be registered only for .n',
  );
  assert(
    manualExtensionSource.includes('registerCompletionItemProvider(assistedSourceSelector'),
    'low-level languages must not receive a VS Code completion provider',
  );
  assert(
    manualExtensionSource.includes('registerInlayHintsProvider(assistedSourceSelector'),
    'low-level languages must not receive predictive inlay providers',
  );

  const missingImports = core.missingImportSuggestions(`
demo {
    fn:
        run() -> bool {
            public:
                @x >> int : readInt();
                println(@x);
                return true;
        }
}
`, intellisenseCatalog);
  assert(missingImports.some((row) => row.legacyImport === 'nC.runtime.input' && row.facadeImport === 'use <io:input>;'));
  assert(missingImports.some((row) => row.legacyImport === 'nC.runtime.output' && row.facadeImport === 'use <io:output>;'));

  const backendPreludeImports = core.missingImportSuggestions(`
demo {
    run() -> bool {
        text >> str = strConcat("a", charToStr('b'));
        stamp = nowMs();
        present = exists(".");
        files = listDir(".");
        return strLen(text) > 0;
    }
}
`, intellisenseCatalog);
  for (const label of ['strConcat', 'charToStr', 'nowMs', 'exists', 'listDir', 'strLen']) {
    assert(!backendPreludeImports.some((row) => row.label === label || row.symbol === label), `${label} must be backend-prelude and import-free`);
    assert(core.isBackendPreludeSymbol(label), `${label} missing backend-prelude identity`);
  }
  assert.strictEqual(core.preferredImportStatement(core.lookupCatalogEntry(intellisenseCatalog, 'strLen')), '');

  const pointerMissingImports = core.missingImportSuggestions(`
demo {
    fn:
        run() -> bool {
            public:
                @region >> memRegion<byte> : pRegion(@p, 4);
                @span >> span<byte> : spanOf(@region);
                return true;
        }
}
`, intellisenseCatalog);
  assert(pointerMissingImports.some((row) => row.legacyImport === 'nC.runtime.region'));
  assert(pointerMissingImports.some((row) => row.legacyImport === 'nC.runtime.span'));

  const assistDiagnostics = core.editorAssistDiagnostics(`
demo {
    fn:
        run() -> bool {
            public:
                @x >> int : readInt();
                @region >> memRegion<byte> : pRegion(@p, 4);
                return true;
        }
}
`, intellisenseCatalog);
  assert(assistDiagnostics.some((row) => row.code === 'NC_EDITOR_IMPORT_HINT_001' && row.message.includes('use <io:input>;')));
  assert(assistDiagnostics.some((row) => row.code === 'NC_EDITOR_IMPORT_HINT_001' && row.message.includes('use <memory:region>;')));

  const compilerTextRows = core.compilerTextDiagnostics(`[7:39] Invalid string escape '\\&'
7 | println("...")
              ^
`);
  assert.strictEqual(compilerTextRows.length, 1, 'ncc text diagnostic must be parsed');
  assert.strictEqual(compilerTextRows[0].line, 6);
  assert.strictEqual(compilerTextRows[0].character, 10);
  assert(compilerTextRows[0].message.includes("[7:39] Invalid string escape '\\&'"));

  const strongDiagnostics = core.editorAssistDiagnostics(`
use <io:ghost>;
use <project:missing>;
use <fs:read, write>;
use <math:core, trig>;
use <process:spawn, wait>;
use <thread:spawn, join>;

demo {
    fn:
        take(value: int) -> bool {
            public:
                return true;
        }

        run() -> bool {
            public:
                @x >> int : "bad";
                println(@ghost);
                take("bad");
                return true;
        }

    ref main ->> ;
    ref main ->> missingFn;
}
`, intellisenseCatalog, { projectModules: ['child'] });
  assert(strongDiagnostics.some((row) => row.code === 'E_USE_ITEM_UNKNOWN_001'), 'unknown import item diagnostic missing');
  assert(strongDiagnostics.some((row) => row.code === 'NC_EDITOR_IMPORT_PROJECT_UNRESOLVED_001'), 'missing project import diagnostic missing');
  assert(!strongDiagnostics.some((row) => row.code === 'E_USE_FAMILY_UNKNOWN_001' && row.message.includes('fs')), 'stoud fs family should be accepted');
  assert(!strongDiagnostics.some((row) => row.code === 'E_USE_FAMILY_UNKNOWN_001' && row.message.includes('math')), 'stoud math family should be accepted');
  assert(!strongDiagnostics.some((row) => row.code === 'E_USE_FAMILY_UNKNOWN_001' && row.message.includes('process')), 'stoud process family should be accepted');
  assert(!strongDiagnostics.some((row) => row.code === 'E_USE_FAMILY_UNKNOWN_001' && row.message.includes('thread')), 'stoud thread family should be accepted');
  assert(strongDiagnostics.some((row) => row.code === 'NC_EDITOR_SYMBOL_UNRESOLVED_001' && row.message.includes('@ghost')), 'unresolved @symbol diagnostic missing');
  assert(strongDiagnostics.some((row) => row.code === 'NC_EDITOR_TYPE_MISMATCH_001' && row.message.includes("expected 'int', found 'str'")), 'type mismatch diagnostic missing');
  assert(strongDiagnostics.some((row) => row.code === 'NC_EDITOR_FN_REFERENCE_001'), 'broken @fn diagnostic missing');
  assert(strongDiagnostics.some((row) => row.code === 'NC_EDITOR_FN_UNRESOLVED_001'), 'unresolved @fn diagnostic missing');

  const compactTypeDiagnostics = core.editorAssistDiagnostics(`
compactTypes :: program {
    stc:
        contador >> int <- 0;
        activo >> bool <- true;
        texto >> str <- "ok";

    run :: in() -> bool {
        contador += 1;
        activo++;
        texto += 1;
        return activo;
    }
}
`, intellisenseCatalog);
  assert(!compactTypeDiagnostics.some((row) => row.message.includes('contador +=')), 'int += int should be accepted');
  assert(compactTypeDiagnostics.some((row) => row.code === 'NC_EDITOR_TYPE_OPERATOR_001' && row.message.includes("activo")), 'bool ++ diagnostic missing');
  assert(compactTypeDiagnostics.some((row) => row.code === 'NC_EDITOR_TYPE_MISMATCH_001' && row.message.includes("texto +=")), 'str += int diagnostic missing');

  const functionValueDiagnostics = core.editorAssistDiagnostics(`
fnValues :: program {
    acepta :: in(value >> int) -> bool {
        return true;
    }

    run :: in() -> bool {
        handler >> fn(int) -> bool <- acepta;
        alsoHandler >> fn(int) -> bool <- acepta;
        badHandler >> fn(int) -> bool <- readLine;
        return true;
    }
}
`, intellisenseCatalog);
  const functionValueMismatches = functionValueDiagnostics.filter((row) => (
    row.code === 'NC_EDITOR_TYPE_MISMATCH_001'
    && row.message.includes("expected 'fn(int) -> bool'")
  ));
  assert.strictEqual(functionValueMismatches.length, 1, 'only the wrong function value assignment should fail');
  assert(functionValueMismatches[0].message.includes("found 'fn() -> str'"), 'wrong function value assignment diagnostic missing');

  const invalidEscapeDiagnostics = core.editorAssistDiagnostics(`
demo {
    fn:
        run() -> bool {
            public:
                println("bad\\q");
                return true;
        }
}
`, intellisenseCatalog);
  assert(invalidEscapeDiagnostics.some((row) => row.code === 'NC_EDITOR_STRING_ESCAPE_001' && row.message.includes("Invalid string escape '\\q'")), 'invalid string escape diagnostic missing');

  const pointerBase = core.lookupPointerBase('memRegion');
  assert(pointerBase && pointerBase.title === 'memRegion<T>');
  const pointerDiagnostic = core.lookupDiagnosticHelp('E_PTR_ALIGNMENT_001');
  assert(pointerDiagnostic && pointerDiagnostic.detail.includes('alignment'));

  const noMissingImports = core.missingImportSuggestions(`
use nC.runtime.input;
inputReady {
    fn:
        run() -> bool {
            public:
                @x >> int : readInt();
                return true;
        }
}
`, intellisenseCatalog);
  assert(!noMissingImports.some((row) => row.legacyImport === 'nC.runtime.input'));

  const noMissingFacadeImports = core.missingImportSuggestions(`
use <io:input>;
inputReady {
    fn:
        run() -> bool {
            public:
                @x >> int : readInt();
                return true;
        }
}
`, intellisenseCatalog);
  assert(!noMissingFacadeImports.some((row) => row.legacyImport === 'nC.runtime.input'));

  const groupedFacadeImports = core.missingImportSuggestions(`
use <io:input, output, render>;
inputReady {
    fn:
        run() -> bool {
            public:
                @x >> int : readInt();
                println(@x);
                renderInt(@x);
                return true;
        }
}
`, intellisenseCatalog);
  assert(!groupedFacadeImports.some((row) => row.legacyImport === 'nC.runtime.input'), 'grouped stoud input import must satisfy readInt');
  assert(!groupedFacadeImports.some((row) => row.legacyImport === 'nC.runtime.output'), 'grouped stoud output import must satisfy println');
  assert(!groupedFacadeImports.some((row) => row.legacyImport === 'nC.runtime.render'), 'grouped stoud render import must satisfy renderInt');

  const callHit = core.callExpressionAtPosition('listSet(@eventos, 2, @x);', 9);
  assert(callHit && callHit.signature && callHit.signature.name === 'listSet', 'call hover must resolve listSet signature');
  assert.deepStrictEqual(callHit.args.map((row) => row.text), ['@eventos', '2', '@x']);

  const inlayHints = core.inlayHintRows(`
demo {
    fn:
        public:
            run() -> bool {
                @eventos >> list<int> : listCreate<int>();
                @x >> int : listGet(@eventos, 2);
                listSet(@eventos, 2, @x);
                ref main ->> run;
                return true;
        }
}
`);
  for (const label of ['list:', 'index:', 'value:']) {
    assert(inlayHints.some((row) => row.label === label), `missing inlay hint ${label}`);
  }
  assert(!inlayHints.some((row) => String(row.label).startsWith('entry:')), 'ref main entrypoint hint must stay hidden');
  assert(inlayHints.some((row) => row.label === 'value:' && row.tooltip && row.tooltip.includes('Parameter value')), 'parameter inlay hint tooltip missing');
  const charDefinition = core.lookupTypeDefinition('char');
  assert(charDefinition && charDefinition.detail.includes('Single character'), 'char type definition missing');
  assert(core.lookupTypeDefinition('ft32')?.detail.includes('32-bit'), 'ft32 type definition missing');
  assert(core.lookupTypeDefinition('vector<Packet>')?.title === 'vector<T>', 'vector type definition missing');
  assert(core.lookupTypeDefinition('Result<int, str>')?.title === 'Result<T, E>', 'Result type definition missing');

  const directObjectHints = core.inlayHintRows(`
demo {
    stc:
        alumno >> Alumno <- {};

    fn:
        internal:
            procesar(alumno >> Alumno) -> bool {
                return true;
            }

        public:
            run() -> bool {
                println("Hola");
                procesar(@alumno);
                return true;
            }
}
`);
  const literalHint = directObjectHints.find((row) => row.call === 'println' && row.parameter === 'value');
  assert(literalHint && literalHint.directObject, 'literal direct object parameter hint missing');
  assert(literalHint.tooltip.includes('Direct object') && literalHint.tooltip.includes('str'), 'literal direct object tooltip must include type');
  const localObjectHint = directObjectHints.find((row) => row.call === 'procesar' && row.parameter === 'alumno');
  assert(localObjectHint && localObjectHint.directObject, 'local function direct object parameter hint missing');
  assert(localObjectHint.tooltip.includes('@alumno') && localObjectHint.tooltip.includes('Declared type: Alumno'), 'local direct object tooltip must include object and declared type');
  assert(wordCount(localObjectHint.tooltip) <= 50, 'local direct object tooltip must stay within 50 words');

  const nccPredictions = core.nccCommandPredictions('Clase01.n');
  assert(nccPredictions.some((row) => row.command.includes('ncc run') && row.command.includes('Clase01.n')), 'ncc run prediction missing');
  assert(nccPredictions.some((row) => row.command.includes('--profile debug')), 'ncc debug prediction missing');

  const extensionSrc = readText(path.join(root, 'src', 'extension.js'));
  for (const marker of [
    'registerInlayHintsProvider',
    'registerCodeLensProvider',
    "registerCommand('nc.debug'",
    "registerCommand('nc.createKit'",
    "registerCommand('nc.selectKit'",
    'buildInlayHints',
    'buildCodeLenses',
    'discoverNccKits',
    'resolveNccPath',
    'resolveNccForExecution',
    'registerNcDebugAdapter',
    'registerDebugAdapterDescriptorFactory',
    'debugAdapterProbe',
    'resolveNativeDebugAdapter',
    'prepareNativeDebugBuild',
    'ncNativeEngine',
    'lowLevelRunsOutsideIde',
    'runCommandForFile',
    'lowLevelRunArgs',
    'registerDocumentFormattingEditProvider',
    'provideDocumentFormattingEdits',
    'registerVirtualDefinitionProvider',
    'resolveProjectModuleFile',
    'importDefinitionAtPosition',
    'readNccCodeJsonUnits',
    'buildProjectModel',
    'detectProjectCycles',
    'projectDuplicateContainerDiagnostics',
    'showProjectGraph',
    'bridge.json',
    '.ncc/code/*.json',
    'safeRenameOccurrences',
    'collectCodeOccurrences',
    '--compact',
    'nccCommandPredictions'
  ]) {
    assert(extensionSrc.includes(marker), `extension source missing ${marker}`);
  }
  assert(extensionSrc.includes('isTerminalOnlyMachineSource'), '.nm terminal-only guard missing');
  assert(extensionSrc.includes('.nm is a Machine source and starts only from a terminal'), '.nm terminal-only guidance missing');
  assert(extensionSrc.includes("return ['machine', 'check', fileName]"), '.nm Check must route to ncc machine check');
  const runTitleRows = pkg.contributes.menus['editor/title'].filter((row) => row.command === 'nc.run' || row.command === 'nc.debug');
  assert(runTitleRows.every((row) => !row.when.includes('nc-machine')), '.nm must not expose editor title Run/Debug');
  const runContextRows = pkg.contributes.menus['editor/context'].filter((row) => row.command === 'nc.run' || row.command === 'nc.debug');
  assert(runContextRows.every((row) => !row.when.includes('nc-machine')), '.nm must not expose context Run/Debug');
  assert(!pkg.contributes.debuggers[0].languages.includes('nc-machine'), '.nm must not be offered by the IDE debugger');
  assert(extensionSrc.includes("'primitive', mode === 'debug' ? 'map' : 'run'"), 'primitive run/debug routing missing');
  assert(extensionSrc.includes("'basal', mode === 'debug' ? 'map' : 'witness'"), 'basal run/debug routing missing');
  assert(extensionSrc.includes('probeNativeNccCompatibility'), 'ncc compatibility probe missing');
  assert(extensionSrc.includes('configuredRunProfile'), 'separate execution profile missing');
  assert(
    extensionSrc.indexOf("'command -v lldb-dap'") < extensionSrc.indexOf("'command -v gdb'"),
    'source debugging must prefer the verified LLDB-DAP route before GDB-DAP'
  );
  const sourceRunBuilder = extensionSrc.slice(
    extensionSrc.indexOf('function runArgsForFile'),
    extensionSrc.indexOf('function debugArgsForFile')
  );
  assert(!sourceRunBuilder.includes('core.sanitizeProfile'), 'editorial profile must not enter ncc run');
  const sourceDebugBuilder = extensionSrc.slice(
    extensionSrc.indexOf('function debugArgsForFile'),
    extensionSrc.indexOf('function createTerminalRunCommand')
  );
  assert(!sourceDebugBuilder.includes("argv.push('--target'"), 'source debug must not pass unsupported --target');
  assert(pkg.contributes.configuration.properties['nc.runProfile']?.default === 'debug', 'run profile setting missing');
  assert(pkg.contributes.configuration.properties['nc.target']?.default === 'auto', 'target must default to host auto');

  const mergedFacade = core.mergeFacadeImportStatement(
    'use <io:input>;\n',
    'use <io:output>;'
  );
  assert(mergedFacade && mergedFacade.changed);
  assert.strictEqual(mergedFacade.newLine, 'use <io:input, output>;');

  const facadeSpans = core.semanticTokenSpans('use <io:input, output, render>;\n');
  assert(facadeSpans.some((row) => row.tokenType === 'importKeyword'));
  assert(facadeSpans.some((row) => row.tokenType === 'importFamily'));
  assert(facadeSpans.filter((row) => row.tokenType === 'importItem').length === 3);

  const familyRows = core.facadeImportCompletionRows('', 'use <', 'use <'.length);
  assert(familyRows.some((row) => row.label === 'use <io:output, render, input>;'), 'compact io import family completion missing');
  assert(familyRows.some((row) => row.label === 'use <memory:ptr, heap, region, map>;'), 'compact memory import family completion missing');
  for (const family of ['math', 'time', 'fs', 'net', 'process', 'thread', 'text', 'sync', 'audit', 'terminal', 'mobile', 'network', 'sys', 'os', 'ui']) {
    assert(familyRows.some((row) => row.label.startsWith(`use <${family}:`)), `compact ${family} import family completion missing`);
  }
  const memoryItems = core.facadeImportCompletionRows('', 'use <memory:', 'use <memory:'.length);
  assert.deepStrictEqual(
    memoryItems.map((row) => row.label),
    ['ptr', 'region', 'stack', 'heap', 'map', 'alloc', 'buffer', 'bytes', 'layout', 'align', 'endian', 'slice', 'span']
  );
  const importedMapFunctions = core.importedFunctionCompletionRows('use <memory:map>;\n', intellisenseCatalog);
  for (const functionName of ['map', 'mapAnonymous', 'mapFile', 'unmap', 'remap', 'mapProtect', 'mapSync', 'mapLen', 'mapIsMapped']) {
    assert(importedMapFunctions.some((row) => row.label === functionName), `memory map completion missing ${functionName}`);
  }
  for (const functionName of ['dbgRegs', 'dbgDump', 'dbgTrace', 'dbgSym', 'rspSum', 'rspFrame', 'brkSet', 'brkDel', 'crashWrite', 'kaslr', 'canarySet', 'canaryOk', 'sanPoison', 'sanClear', 'sanOk']) {
    const signature = core.CALL_SIGNATURES.find((row) => row.name === functionName);
    assert(signature, `compact kernel signature missing ${functionName}`);
    assert(!signature.importHint, `compact kernel intrinsic must not request an import: ${functionName}`);
  }
  const fsItems = core.facadeImportCompletionRows('', 'use <fs:', 'use <fs:'.length);
  assert(fsItems.some((row) => row.label === 'read') && fsItems.some((row) => row.label === 'path'), 'stoud fs import items missing');

  const importedIo = core.importedFunctionCompletionRows('use <io:output, render, input>;\n', intellisenseCatalog);
  assert(importedIo.some((row) => row.label === 'println'), 'imported println completion missing');
  assert(importedIo.some((row) => row.label === 'renderInt'), 'imported renderInt completion missing');
  assert(importedIo.some((row) => row.label === 'renderBinary'), 'imported renderBinary completion missing');
  assert(importedIo.some((row) => row.label === 'renderBin'), 'imported renderBin completion missing');
  assert(importedIo.some((row) => row.label === 'readInt'), 'imported readInt completion missing');
  const importedMemory = core.importedFunctionCompletionRows('use <memory:ptr, heap, region>;\n', intellisenseCatalog);
  for (const label of ['alloc', 'free', 'realloc', 'pRead', 'pRegion']) {
    assert(importedMemory.some((row) => row.label === label), `imported ${label} completion missing`);
  }
  const importedCollections = core.importedFunctionCompletionRows('use <collections:list, queue, vector>;\n', intellisenseCatalog);
  for (const label of ['listGet', 'queuePush', 'vectorPush', 'vectorGet', 'vectorSet']) {
    assert(importedCollections.some((row) => row.label === label), `imported ${label} completion missing`);
  }
  const importedString = core.importedFunctionCompletionRows('', intellisenseCatalog);
  for (const label of ['strLen', 'strByteLen', 'strConcat', 'strSubstring', 'strCompare', 'strContains', 'strIndexOf', 'strFromInt', 'strFromBool', 'strFromFt64']) {
    assert(importedString.some((row) => row.label === label), `imported ${label} completion missing`);
  }
  const importedStoudStd = core.importedFunctionCompletionRows(`
use <math:core, trig, numeric>;
use <fs:read, write>;
use <process:spawn, wait>;
use <thread:spawn, join>;
`, intellisenseCatalog);
  for (const label of ['sqrt', 'sin', 'readFile', 'writeFile', 'spawn', 'wait', 'threadSpawn', 'threadJoin']) {
    assert(importedStoudStd.some((row) => row.label === label), `imported ${label} completion missing`);
  }

  const structCompletionSource = `
demo {
    type:
        Address { cp >> int; city >> str; }
        Packet { tag >> int; size >> int; address >> Address; }

    stc:
        packet >> Packet;
        packetArray >> array<Packet, 8>;
        packetList >> list<Packet>;
        packets >> vector<Packet>;

    fn:
        public:
            run() -> bool {
                packet.
                packet.address.
                packetArray[0].
                packetList.get(0).address.
                packets.get(0).
                packets[0].
                return true;
            }
}
`;
  const directFieldRows = core.structFieldCompletionRows(structCompletionSource, '        packet.', '        packet.'.length);
  assert(directFieldRows.some((row) => row.label === 'tag' && row.detail.includes('Packet.tag')), 'direct struct field completion missing');
  const getFieldRows = core.structFieldCompletionRows(structCompletionSource, '        packets.get(0).', '        packets.get(0).'.length);
  assert(getFieldRows.some((row) => row.label === 'size'), 'aggregate get field completion missing');
  const indexFieldRows = core.structFieldCompletionRows(structCompletionSource, '        packets[0].', '        packets[0].'.length);
  assert(indexFieldRows.some((row) => row.label === 'tag'), 'aggregate index field completion missing');
  const nestedFieldRows = core.structFieldCompletionRows(structCompletionSource, '        packet.address.', '        packet.address.'.length);
  assert(nestedFieldRows.some((row) => row.label === 'cp'), 'nested struct field completion missing');
  const arrayStructRows = core.structFieldCompletionRows(structCompletionSource, '        packetArray[0].', '        packetArray[0].'.length);
  assert(arrayStructRows.some((row) => row.label === 'address'), 'array-of-struct field completion missing');
  const listNestedRows = core.structFieldCompletionRows(structCompletionSource, '        packetList.get(0).address.', '        packetList.get(0).address.'.length);
  assert(listNestedRows.some((row) => row.label === 'city'), 'list-of-struct nested field completion missing');

  const entryRows = core.entrypointCompletionRows('demo {\n    fn:\n        public:\n            run() -> bool { return true; }\n}\n', '    ref ', '    ref '.length);
  assert(entryRows.some((row) => row.label === 'ref main ->> run;'), 'ref main entrypoint completion missing');
  assert.strictEqual(core.entrypointCompletionRows('demo { ref main ->> run; }', '    ref ', '    ref '.length).length, 0);

  const projectRows = core.projectModuleCompletionRowsFromPaths(
    ['src/calculadora.n', 'src/worker.nc', 'src/main.n', 'src/readme.md'],
    'src/main.n'
  );
  assert.deepStrictEqual(projectRows.map((row) => row.label), ['calculadora', 'worker']);
  assert(core.isProjectImportCompletionContext('use <project:', 'use <project:'.length), 'project import context missing');

  const autoImportText = `
demo {
    run :: in() -> bool {
        x >> int <- readInt(
    }
}
`;
  const autoImportCandidate = core.autoImportCandidateAtOffset(
    autoImportText,
    autoImportText.indexOf('readInt(') + 'readInt('.length,
    intellisenseCatalog
  );
  assert(autoImportCandidate && autoImportCandidate.facadeImport === 'use <io:input>;');
  assert.strictEqual(core.completedTokenBeforeBoundary('println(@x);', 'println('.length), 'println');
  assert.strictEqual(core.isTokenBoundaryText('('), true);
  assert.strictEqual(core.autoImportAllowed(autoImportCandidate), true);
  const renderBinaryImportCandidate = core.autoImportCandidateAtOffset('renderBinary(42);', 'renderBinary('.length, intellisenseCatalog);
  assert(renderBinaryImportCandidate && renderBinaryImportCandidate.facadeImport === 'use <io:render>;');
  const authCandidate = core.autoImportCandidateAtOffset('fs.', 'fs.'.length, intellisenseCatalog);
  assert.strictEqual(authCandidate, null, 'auth imports must not be inserted silently by token completion');
  const ptrImportCandidate = core.autoImportCandidateAtOffset('pRead(@p);', 'pRead('.length, intellisenseCatalog);
  assert(ptrImportCandidate && ptrImportCandidate.facadeImport === 'use <memory:ptr>;');
  assert.strictEqual(core.autoImportAllowed(ptrImportCandidate), true);
  const fsImportCandidate = core.autoImportCandidateAtOffset('readFile("a.txt");', 'readFile('.length, intellisenseCatalog);
  assert(fsImportCandidate && fsImportCandidate.facadeImport === 'use <fs:read>;');
  const processImportCandidate = core.autoImportCandidateAtOffset('spawn("cmd", []);', 'spawn('.length, intellisenseCatalog);
  assert(processImportCandidate && processImportCandidate.facadeImport === 'use <process:spawn>;');
  assert.strictEqual(core.expandCompactEmptyBlockLine('contador {}'), 'contador {\n    \n}');
  assert.strictEqual(core.expandCompactEmptyBlockLine('    if (ok) {}'), '    if (ok) {\n        \n    }');

  const printHit = core.lookupPrintMember(registry, 'println');
  assert(printHit && printHit.title === 'println');
  assert.strictEqual(core.lookupPrintMember(registry, 'print.inspect'), null);

  const runtimeInputHit = core.lookupRuntimeFamily(registry, 'nC.runtime.input');
  const runtimeOutputHit = core.lookupRuntimeFamily(registry, 'nC.runtime.output');
  assert(runtimeInputHit && runtimeInputHit.title === 'nC.runtime.input');
  assert(runtimeOutputHit && runtimeOutputHit.title === 'nC.runtime.output');

  const lowLevelTarget = core.detectLowLevelRenameTarget(high.path.replace(/\.n$/, '.n'), rootMode.text);
  assert.strictEqual(lowLevelTarget, '.nb');
  assert.strictEqual(core.detectModeFamily('primitive_probe.nbb', primitive.text), 'primitive-low-level');
  assert.strictEqual(core.detectModeFamily('hardware_probe.ncp', hardware.text), 'hardware-low-level');

  console.log('OK: nc-language smoke tests passed');
}

main();
