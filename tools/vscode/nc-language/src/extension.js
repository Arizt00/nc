/* eslint-disable no-console */
const vscode = require('vscode');
const cp = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');
const core = require('./nc_semantic');

const SEMANTIC_TOKEN_TYPES = [
  'mode',
  'modeRuntime',
  'modeCompiler',
  'modeSystem',
  'modeMachine',
  'modePrimitive',
  'modeKernel',
  'modeBasal',
  'basalEntity',
  'kernelEntity',
  'section',
  'type',
  'pointerType',
  'function',
  'printFunction',
  'pointerFunction',
  'colorFunction',
  'controlFlow',
  'loopControl',
  'kernelAddressKeyword',
  'kernelMmio8',
  'kernelMmio16',
  'kernelMmio32',
  'kernelMmio64',
  'kernelMmio128',
  'kernelMmio256',
  'kernelMmio512',
  'method',
  'variable',
  'parameter',
  'property',
  'operator',
  'materialWrite',
  'visibilityKeyword',
  'number',
  'string',
  'comment',
  'runtimeFamily',
  'compilerPhase',
  'primitiveEntity',
  'hardwareEntity',
  'machineRegister',
  'machineInstruction',
  'ffiSymbol',
  'capability',
  'policy',
  'diagnostic',
  'rawHazard',
  'importHint',
  'importKeyword',
  'importDomain',
  'importFamily',
  'importItem',
  'container',
  'stcSection',
  'typeSection',
  'localSection',
  'constSection',
  'fnSection',
  'faSection',
  'mainFamily',
  'atReference',
  'atGroup',
  'projectionOperator',
  'verificationOperator',
  'devCall',
  'returnKeyword',
  'basicType',
  'advancedType',
  'genericType',
  'arrayType',
  'memRegionType',
  'boolLiteral',
  'stringEscape',
  'copyClone',
  'destructure',
  'constructure'
];

const SEMANTIC_TOKEN_MODIFIERS = [
  'readonly',
  'unsafe',
  'strict',
  'primitiveOnly',
  'machineOnly',
  'runtimeOnly',
  'deprecated',
  'experimental',
  'error',
  'warning',
  'evidence',
  'sealed',
  'redacted'
];

const SEMANTIC_LEGEND = new vscode.SemanticTokensLegend(
  SEMANTIC_TOKEN_TYPES,
  SEMANTIC_TOKEN_MODIFIERS
);

const COMPLETION_TRIGGER_CHARS = Array.from(new Set([
  '.', '@', ':', '<', '>', '(', '[', ',', '\\', '#', '?', '&',
  ...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ'
]));

const PROJECT_MODULE_CACHE_MS = 5000;
const PROJECT_MODULE_CACHE = new Map();
const NCC_COMPATIBILITY_CACHE = new Map();
const VIRTUAL_DEFINITION_SCHEME = 'nc-def';
const VIRTUAL_DEFINITIONS = new Map();
let virtualDefinitionSeq = 0;

function execCapture(cmd, args, extraEnv) {
  const env = Object.assign({}, process.env, extraEnv || {});
  const out = cp.spawnSync(cmd, args, {
    encoding: 'utf8',
    env,
    windowsHide: true
  });
  if (out.error) {
    throw out.error;
  }
  return {
    code: out.status === null ? 1 : out.status,
    stdout: (out.stdout || '').trim(),
    stderr: (out.stderr || '').trim()
  };
}

function execCaptureAsync(cmd, args, extraEnv, signal) {
  return new Promise((resolve, reject) => {
    const env = Object.assign({}, process.env, extraEnv || {});
    const child = cp.spawn(cmd, args, {
      env,
      windowsHide: true,
      stdio: ['ignore', 'pipe', 'pipe']
    });
    let stdout = '';
    let stderr = '';
    let settled = false;

    const abort = () => {
      if (!child.killed) {
        child.kill();
      }
    };
    const finish = (fn, value) => {
      if (settled) {
        return;
      }
      settled = true;
      if (signal) {
        signal.removeEventListener('abort', abort);
      }
      fn(value);
    };

    if (signal) {
      if (signal.aborted) {
        abort();
      } else {
        signal.addEventListener('abort', abort, { once: true });
      }
    }
    child.stdout.setEncoding('utf8');
    child.stderr.setEncoding('utf8');
    child.stdout.on('data', (chunk) => { stdout += chunk; });
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    child.on('error', (error) => finish(reject, error));
    child.on('close', (code) => {
      if (signal && signal.aborted) {
        const error = new Error('diagnostic process superseded by a newer document version');
        error.code = 'NC_DIAGNOSTIC_ABORTED';
        finish(reject, error);
        return;
      }
      finish(resolve, {
        code: code === null ? 1 : code,
        stdout: stdout.trim(),
        stderr: stderr.trim()
      });
    });
  });
}

function tryExecCapture(cmd, args, extraEnv) {
  try {
    return execCapture(cmd, args, extraEnv);
  } catch (err) {
    return {
      code: 1,
      stdout: '',
      stderr: err && err.message ? err.message : String(err || ''),
      error: err
    };
  }
}

function isNativeWindowsHost() {
  return process.platform === 'win32' && !(vscode.env.remoteName || '').toLowerCase().includes('wsl');
}

function shellQuoteForBash(value) {
  return `'${String(value).replace(/'/g, `'\\''`)}'`;
}

function shellQuoteForPowerShell(value) {
  return `'${String(value).replace(/'/g, "''")}'`;
}

function shellQuoteForCmd(value) {
  return `"${String(value).replace(/"/g, '\\"')}"`;
}

function hostPlatformKind() {
  if ((vscode.env.remoteName || '').toLowerCase().includes('wsl')) {
    return 'linux';
  }
  if (process.platform === 'win32') {
    return 'windows';
  }
  if (process.platform === 'darwin') {
    return 'macos';
  }
  return 'linux';
}

function availableWslDistros() {
  if (process.platform !== 'win32') {
    return [];
  }
  const res = tryExecCapture('wsl.exe', ['-l', '-q']);
  if (res.code !== 0 || !res.stdout) {
    return [];
  }
  return res.stdout
    .replace(/\0/g, '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

function preferredWslDistro() {
  const requested = (config().get('runHost') || 'auto').toString().trim().toLowerCase();
  const distros = availableWslDistros();
  if (requested && requested !== 'auto' && requested !== 'wsl') {
    const exact = distros.find((row) => row.toLowerCase() === requested);
    if (exact) {
      return exact;
    }
  }
  return distros.find((row) => /kali/i.test(row))
    || distros.find((row) => /ubuntu/i.test(row))
    || distros[0]
    || '';
}

function forcedRunHost() {
  return (config().get('runHost') || 'auto').toString().trim().toLowerCase();
}

function wslCommandPrefix(distro) {
  const prefix = ['wsl.exe'];
  if (distro) {
    prefix.push('-d', distro);
  }
  return prefix;
}

function resolveWslNccPath(distro) {
  if (process.platform !== 'win32') {
    return null;
  }
  const args = [...wslCommandPrefix(distro), 'bash', '-lc', 'command -v ncc'];
  const res = tryExecCapture(args[0], args.slice(1));
  if (res.code !== 0 || !res.stdout) {
    return null;
  }
  return res.stdout.split(/\r?\n/)[0].trim() || 'ncc';
}

function resolveNccForExecution(fileName) {
  const forced = forcedRunHost();
  if (process.platform === 'win32' && ['wsl', 'kali', 'ubuntu', 'linux'].includes(forced)) {
    const distro = preferredWslDistro();
    const nccPath = resolveWslNccPath(distro);
    if (nccPath) {
      return {
        nccPath,
        toolchainId: distro || 'wsl',
        source: distro ? `wsl:${distro}` : 'wsl:ncc',
        transport: 'wsl',
        distro
      };
    }
  }
  const resolved = resolveNccPath({ allowWsl: true });
  if (resolved.transport === 'wsl' && !resolved.distro) {
    return Object.assign({}, resolved, { distro: preferredWslDistro() });
  }
  return resolved;
}

function toWslPath(winPath) {
  const res = cp.spawnSync('wsl.exe', ['wslpath', '-a', winPath], {
    encoding: 'utf8',
    windowsHide: true
  });
  if (res.status !== 0) {
    throw new Error((res.stderr || res.stdout || `wslpath failed for ${winPath}`).trim());
  }
  return (res.stdout || '').trim();
}

function wslWorkspaceForDocument(document) {
  const folder = vscode.workspace.getWorkspaceFolder(document.uri);
  if (folder) {
    return toWslPath(folder.uri.fsPath);
  }
  return toWslPath(path.dirname(document.fileName));
}

function config() {
  return vscode.workspace.getConfiguration('nc');
}

function resolveToolchainFlag() {
  const raw = (config().get('toolchain') || 'auto').toString().trim();
  if (!raw || raw.toLowerCase() === 'auto') {
    return null;
  }
  return raw;
}

function configuredPath(setting) {
  const raw = (config().get(setting) || 'auto').toString().trim();
  if (!raw || raw.toLowerCase() === 'auto') {
    return null;
  }
  return raw;
}

function commandOnPath(name) {
  const probe = process.platform === 'win32'
    ? tryExecCapture('where.exe', [name])
    : tryExecCapture('which', [name]);
  if (probe.code !== 0 || !probe.stdout) {
    return null;
  }
  return probe.stdout.split(/\r?\n/).map((line) => line.trim()).find(Boolean) || null;
}

function workspaceNccCandidates() {
  const suffixes = process.platform === 'win32'
    ? ['ncc.exe', 'ncc']
    : ['ncc', 'ncc.exe'];
  const roots = new Set((vscode.workspace.workspaceFolders || []).map((folder) => folder.uri.fsPath));
  const activeFile = vscode.window.activeTextEditor?.document?.fileName;
  if (activeFile) {
    roots.add(path.dirname(activeFile));
  }
  const candidates = [];
  for (const root of roots) {
    let dir = root;
    while (dir && dir !== path.dirname(dir)) {
      for (const exe of suffixes) {
        candidates.push(path.join(dir, 'target', 'debug', exe));
        candidates.push(path.join(dir, '.ncc', 'bin', exe));
        candidates.push(path.join(dir, '.ncc', 'run', exe));
      }
      dir = path.dirname(dir);
    }
  }
  return Array.from(new Set(candidates));
}

function installedNccCandidates() {
  const candidates = [];
  if (process.platform === 'win32') {
    const local = process.env.LOCALAPPDATA;
    if (local) {
      candidates.push(path.join(local, 'nC', 'bin', 'ncc.exe'));
    }
  } else {
    const home = os.homedir();
    if (home) {
      candidates.push(path.join(home, '.cargo', 'bin', 'ncc'));
      candidates.push(path.join(home, '.local', 'bin', 'ncc'));
    }
    candidates.push('/usr/local/bin/ncc');
  }
  return candidates;
}

function nccCompatibilityCacheKey(candidate) {
  try {
    const stat = fs.statSync(candidate);
    return `${candidate}|${stat.size}|${stat.mtimeMs}`;
  } catch (_err) {
    return candidate;
  }
}

function probeNativeNccCompatibility(candidate) {
  if (!candidate) {
    return { ok: false, reason: 'empty ncc candidate' };
  }
  const key = nccCompatibilityCacheKey(candidate);
  if (NCC_COMPATIBILITY_CACHE.has(key)) {
    return NCC_COMPATIBILITY_CACHE.get(key);
  }

  const imports = tryExecCapture(candidate, ['imports', 'manifest', '--json']);
  let hasMemoryMap = false;
  if (imports.code === 0 && imports.stdout) {
    try {
      const payload = JSON.parse(imports.stdout);
      hasMemoryMap = Array.isArray(payload.entries) && payload.entries.some((row) =>
        String(row.family || '').toLowerCase() === 'memory'
          && String(row.item || '').toLowerCase() === 'map'
      );
    } catch (_err) {
      hasMemoryMap = false;
    }
  }

  const debug = tryExecCapture(candidate, ['debug-adapter', '--probe', '--json']);
  let hasDebugAdapter = false;
  if (debug.code === 0 && debug.stdout) {
    try {
      const payload = JSON.parse(debug.stdout);
      hasDebugAdapter = payload.protocol === 'Debug Adapter Protocol'
        && String(payload.status || '').startsWith('closed');
    } catch (_err) {
      hasDebugAdapter = false;
    }
  }

  const result = {
    ok: hasMemoryMap && hasDebugAdapter,
    hasMemoryMap,
    hasDebugAdapter,
    reason: hasMemoryMap && hasDebugAdapter
      ? 'compatible'
      : `required surfaces missing: map=${hasMemoryMap} debug-adapter=${hasDebugAdapter}`
  };
  NCC_COMPATIBILITY_CACHE.set(key, result);
  return result;
}

function compatibleNativeResolution(candidate, source, toolchainId) {
  const probe = probeNativeNccCompatibility(candidate);
  return probe.ok ? resolvedNative(candidate, source, toolchainId) : null;
}

function resolvedNative(pathOrCommand, source, toolchainId) {
  return {
    nccPath: pathOrCommand,
    toolchainId: toolchainId || source,
    source,
    transport: 'native'
  };
}

function resolveNccPath(options = {}) {
  const configuredNcc = configuredPath('nccPath');
  if (configuredNcc) {
    const transport = (config().get('kitTransport') || 'native').toString().trim().toLowerCase();
    if (transport === 'wsl') {
      return {
        nccPath: configuredNcc,
        toolchainId: config().get('kit') || 'wsl',
        source: 'settings:nc.nccPath',
        transport: 'wsl'
      };
    }
    const configured = compatibleNativeResolution(
      configuredNcc,
      'settings:nc.nccPath',
      config().get('kit') || null
    );
    if (!configured) {
      const probe = probeNativeNccCompatibility(configuredNcc);
      throw new Error(`Configured nc.nccPath is incompatible: ${probe.reason}. Run ncup and select the updated ncc.`);
    }
    return configured;
  }

  const ncupPath = configuredPath('ncupPath') || commandOnPath('ncup') || commandOnPath('ncup.exe') || 'ncup';
  const args = ['which', 'ncc', '--json'];
  const toolchain = resolveToolchainFlag();
  if (toolchain) {
    args.push('--toolchain', toolchain);
  }
  const res = tryExecCapture(ncupPath, args);
  if (res.code === 0) {
    const parsed = JSON.parse(res.stdout);
    if (parsed.path) {
      const resolved = compatibleNativeResolution(parsed.path, 'ncup', parsed.toolchain_id || null);
      if (resolved) {
        return resolved;
      }
    }
  }

  for (const candidate of installedNccCandidates()) {
    if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
      const resolved = compatibleNativeResolution(candidate, 'installed:ncc-suite', 'installed');
      if (resolved) {
        return resolved;
      }
    }
  }

  const pathNcc = commandOnPath('ncc') || commandOnPath('ncc.exe');
  if (pathNcc) {
    const resolved = compatibleNativeResolution(pathNcc, 'PATH:ncc', 'path');
    if (resolved) {
      return resolved;
    }
  }

  for (const candidate of workspaceNccCandidates()) {
    if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
      const resolved = compatibleNativeResolution(candidate, 'workspace:ncc-kit', 'workspace');
      if (resolved) {
        return resolved;
      }
    }
  }

  if (options.allowWsl && process.platform === 'win32') {
    const wsl = tryExecCapture('wsl.exe', ['bash', '-lc', 'command -v ncc']);
    if (wsl.code === 0 && wsl.stdout) {
      return {
        nccPath: wsl.stdout.split(/\r?\n/)[0].trim() || 'ncc',
        toolchainId: 'wsl',
        source: 'wsl:ncc',
        transport: 'wsl'
      };
    }
  }

  const reason = res.stderr || res.stdout || 'ncup was not found and no compatible ncc kit was discovered';
  throw new Error(`${reason}. Run ncup, then configure nC: Create ncc Kit or set nc.nccPath to the updated ncc executable.`);
}

function routerScriptPath(context) {
  const suffix = process.platform === 'win32' ? '.exe' : '';
  const bundled = path.join(context.extensionPath, 'bin', `ncc-tools-${process.platform}-${process.arch}${suffix}`);
  return fs.existsSync(bundled) ? bundled : 'ncc-tools';
}

function forceRoute(document, resolvedNcc) {
  const mode = core.sanitizeLspMode(config().get('lspMode'));
  if (mode === 'auto') {
    return null;
  }
  if (mode === 'nide-broker') {
    return {
      backend: 'nide',
      transport: 'broker-nide',
      command: [resolvedNcc.nccPath, 'nide', 'diagnostics', document.fileName]
    };
  }
  if (mode === 'nclsp') {
    return {
      backend: 'nclsp',
      transport: 'lsp-direct',
      command: [path.join(path.dirname(resolvedNcc.nccPath), 'nclsp'), 'validate', document.fileName, '--profile', core.sanitizeProfile(config().get('profile')), '--migration-stage', 'error-by-profile', '--json']
    };
  }
  return {
    backend: 'nc-sys-lsp',
    transport: 'lsp-direct',
    command: [
      path.join(path.dirname(resolvedNcc.nccPath), 'nc-sys-lsp'),
      'analyze',
      document.fileName,
      '--profile',
      core.sanitizeProfile(config().get('profile')),
      '--migration-stage',
      'error-by-profile',
      '--json'
    ]
  };
}

async function routeDocument(context, document, signal) {
  try {
    const resolved = resolveNccPath();
    const forced = forceRoute(document, resolved);
    if (forced) {
      return forced;
    }

    const ncProfile = core.sanitizeProfile(config().get('profile'));
    const toolchain = resolveToolchainFlag();
    const args = [
      'route-vscode',
      '--file',
      document.fileName,
      '--profile',
      ncProfile,
      '--json'
    ];
    if (toolchain) {
      args.push('--toolchain', toolchain);
    }
    const res = await execCaptureAsync(routerScriptPath(context), args, null, signal);
    if (res.code !== 0) {
      throw new Error(res.stderr || res.stdout || 'nc router failed');
    }
    return JSON.parse(res.stdout);
  } catch (err) {
    if (err && err.code === 'NC_DIAGNOSTIC_ABORTED') {
      throw err;
    }
    if (!isNativeWindowsHost() || document.languageId !== 'nc') {
      throw err;
    }
    return routeDocumentViaWslNclsp(document, err);
  }
}

function routeDocumentViaWslNclsp(document, nativeError) {
  const ncProfile = core.sanitizeProfile(config().get('profile'));
  const file = toWslPath(document.fileName);
  const cwd = wslWorkspaceForDocument(document);
  const command = [
    'cd',
    shellQuoteForBash(cwd),
    '&&',
    'nclsp',
    'validate',
    shellQuoteForBash(file),
    '--ide',
    'vscode',
    '--profile',
    shellQuoteForBash(ncProfile),
    '--migration-stage',
    'error-by-profile',
    '--json'
  ].join(' ');
  return {
    backend: 'nclsp',
    transport: 'wsl-nclsp-fallback',
    native_error: nativeError && nativeError.message ? nativeError.message : String(nativeError || ''),
    command: ['wsl.exe', 'bash', '-lc', command]
  };
}

function findBridgeJson(startPath) {
  let dir = fs.existsSync(startPath) && fs.statSync(startPath).isDirectory()
    ? startPath
    : path.dirname(startPath);
  const rootHints = new Set((vscode.workspace.workspaceFolders || []).map((folder) => path.resolve(folder.uri.fsPath)));
  while (dir && dir !== path.dirname(dir)) {
    const candidate = path.join(dir, 'bridge.json');
    if (fs.existsSync(candidate)) {
      return { root: dir, bridge: candidate };
    }
    if (rootHints.has(path.resolve(dir))) {
      break;
    }
    dir = path.dirname(dir);
  }
  return null;
}

function diagRange(line, column, endLine, endColumn) {
  const lineIdx = Math.max(0, (line || 1) - 1);
  const colIdx = Math.max(0, (column || 1) - 1);
  const endLineIdx = Math.max(lineIdx, (endLine || line || 1) - 1);
  const endColIdx = Math.max(colIdx + 1, (endColumn || (column || 1) + 1) - 1);
  return new vscode.Range(lineIdx, colIdx, endLineIdx, endColIdx);
}

function diagRangeFromRow(row, fallbackMessage) {
  if (row && row.range && row.range.start && row.range.end) {
    return new vscode.Range(
      Math.max(0, row.range.start.line || 0),
      Math.max(0, row.range.start.character || 0),
      Math.max(0, row.range.end.line || 0),
      Math.max(1, row.range.end.character || 1)
    );
  }
  const location = row && (row.line || row.column)
    ? { line: row.line, column: row.column }
    : core.diagnosticLocationFromMessage(fallbackMessage || (row && row.message));
  return diagRange(location.line, location.column, row && row.end_line, row && row.end_column);
}

function diagnosticCodeTarget(code) {
  if (!code) {
    return undefined;
  }
  const root = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
  if (!root) {
    return code;
  }
  const candidates = [
    path.join(root, 'docs', 'spec', 'compiler', 'compiler_diagnostics.md'),
    path.join(root, 'docs', 'base', 'lsp_architecture_nclsp_nc_sys_lsp.md'),
    path.join(root, 'docs', 'spec', 'high_level', 'high_level_diagnostic_policy.md')
  ];
  const target = candidates.find((candidate) => fs.existsSync(candidate));
  return target ? { value: code, target: vscode.Uri.file(target) } : code;
}

function attachDiagnosticMetadata(diagnostic, row, source, document) {
  const code = (row && (row.code || row.Code)) || core.diagnosticCodeFromMessage(diagnostic.message);
  diagnostic.source = source;
  diagnostic.code = diagnosticCodeTarget(code);
  const bridge = document ? findBridgeJson(document.fileName) : null;
  if (bridge) {
    diagnostic.relatedInformation = [
      new vscode.DiagnosticRelatedInformation(
        new vscode.Location(vscode.Uri.file(bridge.bridge), new vscode.Position(0, 0)),
        `nC project bridge: ${path.basename(bridge.bridge)}`
      )
    ];
  }
  return diagnostic;
}

function severityFromString(raw) {
  const v = (raw || '').toString().toLowerCase();
  if (v === 'error') {
    return vscode.DiagnosticSeverity.Error;
  }
  if (v === 'warning') {
    return vscode.DiagnosticSeverity.Warning;
  }
  if (v === 'information' || v === 'info') {
    return vscode.DiagnosticSeverity.Information;
  }
  return vscode.DiagnosticSeverity.Hint;
}

function parseNideDiagnostics(payload, document) {
  const out = [];
  const warnings = Array.isArray(payload.warning_entries) ? payload.warning_entries : [];
  const errors = Array.isArray(payload.error_entries) ? payload.error_entries : [];

  for (const row of warnings) {
    const d = new vscode.Diagnostic(
      diagRangeFromRow(row, row.message),
      row.message || 'nide warning',
      vscode.DiagnosticSeverity.Warning
    );
    out.push(attachDiagnosticMetadata(d, row, 'nide', document));
  }
  for (const row of errors) {
    const d = new vscode.Diagnostic(
      diagRangeFromRow(row, row.message),
      row.message || 'nide error',
      vscode.DiagnosticSeverity.Error
    );
    out.push(attachDiagnosticMetadata(d, row, 'nide', document));
  }
  return out;
}

function parseNclspDiagnostics(payload, document) {
  const out = [];
  const warnings = Array.isArray(payload.warnings) ? payload.warnings : [];
  const errors = Array.isArray(payload.errors) ? payload.errors : [];
  for (const row of warnings) {
    const message = typeof row === 'string' ? row : (row.message || 'nclsp warning');
    const d = new vscode.Diagnostic(
      diagRangeFromRow(row, message),
      message,
      vscode.DiagnosticSeverity.Warning
    );
    out.push(attachDiagnosticMetadata(d, row, 'nclsp', document));
  }
  for (const row of errors) {
    const message = typeof row === 'string' ? row : (row.message || 'nclsp error');
    const d = new vscode.Diagnostic(
      diagRangeFromRow(row, message),
      message,
      vscode.DiagnosticSeverity.Error
    );
    out.push(attachDiagnosticMetadata(d, row, 'nclsp', document));
  }
  return out;
}

function parseLowLevelDiagnostics(payload, document) {
  const out = [];
  const rows = Array.isArray(payload.diagnostics) ? payload.diagnostics : [];
  for (const row of rows) {
    const d = new vscode.Diagnostic(
      diagRangeFromRow(row, row.message),
      row.message || 'nc-sys-lsp diagnostic',
      severityFromString(row.severity)
    );
    out.push(attachDiagnosticMetadata(d, row, 'nc-sys-lsp', document));
  }
  return out;
}

function parseEditorAssistDiagnostics(document, catalog) {
  const out = [];
  const model = document && !document.isUntitled && core.isNcLanguageId(document.languageId)
    ? buildProjectModel(document)
    : null;
  const projectModules = model
    ? Array.from(model.nodes.values())
      .filter((node) => (
        node.files.size > 0
        || node.codeFiles.size > 0
        || node.fromBridge
        || node.fromCode
        || node.containers.length > 0
      ))
      .map((node) => node.name)
    : core.projectModuleCompletionRowsFromPaths(cachedProjectSourceFiles(document), document.fileName)
      .map((row) => row.label);
  const rows = core.editorAssistDiagnostics(document.getText(), catalog, { projectModules });
  for (const row of rows) {
    const line = Math.max(0, row.line || 0);
    const character = Math.max(0, row.character || 0);
    const length = Math.max(1, row.length || 1);
    const d = new vscode.Diagnostic(
      new vscode.Range(line, character, line, character + length),
      row.message || 'nC editor assistance hint',
      severityFromString(row.severity)
    );
    d.code = row.code || undefined;
    d.source = 'nC editor assist';
    out.push(d);
  }
  out.push(...projectDuplicateContainerDiagnostics(document));
  return out;
}

function commandFromRoute(route) {
  if (!Array.isArray(route.command) || route.command.length === 0) {
    throw new Error('invalid route command');
  }
  return {
    cmd: route.command[0],
    args: route.command.slice(1)
  };
}

function nccArgsForFileVerb(verb, fileName) {
  const bridge = findBridgeJson(fileName);
  if (bridge && (verb === 'run' || verb === 'check')) {
    return ['project', verb, bridge.root];
  }
  return [verb, fileName];
}

function commandTargetFileName(target) {
  if (target && target.fsPath) {
    return target.fsPath;
  }
  if (target && target.path && target.scheme === 'file') {
    return target.fsPath || target.path;
  }
  if (typeof target === 'string' && target.trim()) {
    return target.trim();
  }
  return vscode.window.activeTextEditor?.document?.fileName || null;
}

function wslArg(resolved, arg) {
  const text = String(arg);
  if (process.platform === 'win32' && /^[A-Za-z]:[\\/]/.test(text)) {
    return toWslPath(text);
  }
  return text;
}

function createResolvedTerminalCommand(resolved, argv, cwdHint) {
  if (resolved.transport === 'wsl') {
    const cwd = cwdHint ? path.dirname(cwdHint) : (vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath || process.cwd());
    const wslCwd = process.platform === 'win32' ? toWslPath(cwd) : cwd;
    const command = [
      'cd',
      shellQuoteForBash(wslCwd),
      '&&',
      shellQuoteForBash(resolved.nccPath || 'ncc'),
      ...argv.map((arg) => shellQuoteForBash(wslArg(resolved, arg)))
    ].join(' ');
    const prefix = wslCommandPrefix(resolved.distro).join(' ');
    return `${prefix} bash -lc "${command.replace(/"/g, '\\"')}"`;
  }
  return createTerminalCommandLine(resolved.nccPath, argv);
}

function execResolvedNccCapture(resolved, argv, cwdHint) {
  if (resolved.transport === 'wsl') {
    const cwd = cwdHint ? path.dirname(cwdHint) : (vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath || process.cwd());
    const wslCwd = process.platform === 'win32' ? toWslPath(cwd) : cwd;
    const command = [
      'cd',
      shellQuoteForBash(wslCwd),
      '&&',
      shellQuoteForBash(resolved.nccPath || 'ncc'),
      ...argv.map((arg) => shellQuoteForBash(wslArg(resolved, arg)))
    ].join(' ');
    return execCapture('wsl.exe', [
      ...wslCommandPrefix(resolved.distro).slice(1),
      'bash',
      '-lc',
      command
    ]);
  }
  return execCapture(resolved.nccPath, argv);
}

function runFileCommand(resolved, verb, fileName) {
  const args = nccArgsForFileVerb(verb, fileName);
  return execResolvedNccCapture(resolved, args, fileName);
}

function runFormatter(resolved, fileName) {
  const fmtMode = (config().get('formatter') || 'nfmt').toString();
  if (resolved.transport === 'wsl') {
    return execResolvedNccCapture(resolved, ['nfmt', fileName, '--compact', '--in-place'], fileName);
  }
  const nccPath = resolved.nccPath;
  if (fmtMode === 'ncc-nfmt') {
    return execCapture(nccPath, ['nfmt', fileName, '--compact', '--in-place']);
  }
  return execCapture(resolveNfmtPath(nccPath), [fileName, '--compact', '--in-place']);
}

function resolveNfmtPath(nccPath) {
  const dir = path.dirname(nccPath || '');
  if (dir && dir !== '.') {
    const candidate = path.join(dir, process.platform === 'win32' ? 'nfmt.exe' : 'nfmt');
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  return commandOnPath('nfmt') || commandOnPath('nfmt.exe') || 'nfmt';
}

function runFormatterToText(resolved, fileName) {
  const fmtMode = (config().get('formatter') || 'nfmt').toString();
  if (resolved.transport === 'wsl') {
    return execResolvedNccCapture(resolved, ['nfmt', fileName, '--compact'], fileName);
  }
  const nccPath = resolved.nccPath;
  if (fmtMode === 'ncc-nfmt') {
    return execCapture(nccPath, ['nfmt', fileName, '--compact']);
  }
  return execCapture(resolveNfmtPath(nccPath), [fileName, '--compact']);
}

function fullDocumentRange(document) {
  const last = document.lineAt(Math.max(0, document.lineCount - 1));
  return new vscode.Range(0, 0, last.range.end.line, last.range.end.character);
}

function provideDocumentFormattingEdits(document) {
  if (document.languageId !== 'nc') {
    return [];
  }
  const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'nc-nfmt-'));
  const tmpName = `${path.basename(document.fileName || 'format.n').replace(/[^A-Za-z0-9_.-]/g, '_')}.n`;
  const tmpPath = path.join(tmpRoot, tmpName);
  try {
    fs.writeFileSync(tmpPath, document.getText(), 'utf8');
    const resolved = resolveNccPath({ allowWsl: true });
    const result = runFormatterToText(resolved, tmpPath);
    if (result.code !== 0) {
      throw new Error(result.stderr || result.stdout || 'nfmt failed');
    }
    const rendered = result.stdout || '';
    if (!rendered || rendered.startsWith('OK: nfmt no changes')) {
      return [];
    }
    if (rendered === document.getText()) {
      return [];
    }
    return [vscode.TextEdit.replace(fullDocumentRange(document), rendered)];
  } finally {
    try {
      fs.rmSync(tmpRoot, { recursive: true, force: true });
    } catch (_err) {
      // best-effort cleanup
    }
  }
}

function provideOnTypeFormattingEdits(document, position, ch) {
  if (!core.isNcLanguageId(document.languageId) || ch !== '}') {
    return [];
  }
  const line = document.lineAt(position.line);
  const expanded = core.expandCompactEmptyBlockLine(line.text);
  if (!expanded || expanded === line.text) {
    return [];
  }
  return [vscode.TextEdit.replace(line.range, expanded)];
}

function ncFileLane(fileName) {
  const ext = path.extname(fileName || '').toLowerCase();
  if (ext === '.nm') {
    return 'machine';
  }
  if (ext === '.ncp') {
    return 'primitive';
  }
  if (ext === '.nb' || ext === '.nbb') {
    return 'basal';
  }
  return 'source';
}

function isTerminalOnlyMachineSource(fileName) {
  return path.extname(fileName || '').toLowerCase() === '.nm';
}

function showMachineTerminalOnlyMessage() {
  vscode.window.showInformationMessage(
    '.nm is a Machine source and starts only from a terminal. Use ncc machine <command> <file.nm>.'
  );
}

function configuredTarget() {
  const target = (config().get('target') || 'auto').toString().trim();
  return !target || target.toLowerCase() === 'auto' ? '' : target;
}

function configuredRunProfile() {
  const profile = (config().get('runProfile') || 'debug').toString().trim().toLowerCase();
  const accepted = new Set([
    'debug', 'interpreted', 'real', 'native-fast', 'native-ffi',
    'release', 'release-fast', 'release-small', 'unsafe-fast', 'instrumented'
  ]);
  return accepted.has(profile) ? profile : 'debug';
}

function appendRunProfileArgs(argv, profile) {
  switch (profile) {
    case 'interpreted':
      argv.push('--interpreted');
      break;
    case 'real':
    case 'native-fast':
      argv.push('--native-fast');
      break;
    case 'native-ffi':
      argv.push('--native-ffi');
      break;
    case 'debug':
      break;
    default:
      argv.push('--profile', profile);
      break;
  }
  return argv;
}

function lowLevelRunArgs(fileName, mode) {
  const lane = ncFileLane(fileName);
  const target = configuredTarget();
  if (lane === 'machine') {
    return null;
  }
  if (lane === 'primitive') {
    return ['primitive', mode === 'debug' ? 'map' : 'run', fileName, '--json'];
  }
  if (lane === 'basal') {
    if (path.extname(fileName).toLowerCase() === '.nbb') {
      return ['basal', mode === 'debug' ? 'inspect-nbb' : 'read-nbb', fileName, '--json'];
    }
    return ['basal', mode === 'debug' ? 'map' : 'witness', fileName, '--json'];
  }
  return null;
}

function runArgsForFile(verb, fileName) {
  if (isTerminalOnlyMachineSource(fileName)) {
    if (verb === 'check') {
      return ['machine', 'check', fileName];
    }
    throw new Error('.nm Machine sources start only from a terminal');
  }
  const lowLevelArgs = lowLevelRunArgs(fileName, verb === 'debug' ? 'debug' : 'run');
  if (lowLevelArgs) {
    return lowLevelArgs;
  }
  const target = configuredTarget();
  const bridge = findBridgeJson(fileName);
  if (bridge && (verb === 'run' || verb === 'check')) {
    const argv = ['project', verb, bridge.root];
    if (target) {
      argv.push('--target', target);
    }
    return argv;
  }
  const argv = [verb, fileName];
  if (verb === 'run') {
    appendRunProfileArgs(argv, configuredRunProfile());
  }
  return argv;
}

function debugArgsForFile(fileName) {
  if (isTerminalOnlyMachineSource(fileName)) {
    throw new Error('.nm Machine sources start only from a terminal');
  }
  const lowLevelArgs = lowLevelRunArgs(fileName, 'debug');
  if (lowLevelArgs) {
    return lowLevelArgs;
  }
  const argv = ['run', fileName];
  argv.push('--profile', 'debug', '--show-return');
  return argv;
}

function createTerminalRunCommand(resolved, verb, fileName) {
  const argv = runArgsForFile(verb, fileName);
  return createResolvedTerminalCommand(resolved, argv, fileName);
}

function createTerminalDebugCommand(resolved, fileName) {
  const argv = debugArgsForFile(fileName);
  return createResolvedTerminalCommand(resolved, argv, fileName);
}

function createTerminalCommandLine(nccPath, argv) {
  const escaped = argv.map((arg) => {
    if (arg === null || arg === undefined) {
      return '';
    }
    const text = arg.toString();
    if (/^[A-Za-z0-9._:\\/-]+$/.test(text)) {
      return text;
    }
    return `"${text.replace(/"/g, '\\"')}"`;
  });
  return `"${nccPath}" ${escaped.join(' ')}`.trim();
}

function createBestTerminal(name) {
  return vscode.window.createTerminal(name);
}

function lowLevelRunsOutsideIde(fileName) {
  return ncFileLane(fileName) !== 'source';
}

function launchExternalTerminal(title, commandLine, cwdHint) {
  const cwd = cwdHint ? path.dirname(cwdHint) : (vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath || process.cwd());
  const platform = hostPlatformKind();
  if (platform === 'windows') {
    const forced = forcedRunHost();
    if (forced === 'cmd') {
      const child = cp.spawn('cmd.exe', ['/c', 'start', title, 'cmd.exe', '/k', commandLine], {
        cwd,
        detached: true,
        windowsHide: false,
        stdio: 'ignore'
      });
      child.unref();
      return true;
    }
    const psCommand = `Set-Location ${shellQuoteForPowerShell(cwd)}; ${commandLine}`;
    const child = cp.spawn('cmd.exe', ['/c', 'start', title, 'powershell.exe', '-NoExit', '-Command', psCommand], {
      cwd,
      detached: true,
      windowsHide: false,
      stdio: 'ignore'
    });
    child.unref();
    return true;
  }

  if (platform === 'macos') {
    const script = [
      'tell application "Terminal"',
      `  do script "cd ${cwd.replace(/"/g, '\\"')} && ${commandLine.replace(/"/g, '\\"')}"`,
      '  activate',
      'end tell'
    ].join('\n');
    const child = cp.spawn('osascript', ['-e', script], { detached: true, stdio: 'ignore' });
    child.unref();
    return true;
  }

  const bashCommand = `cd ${shellQuoteForBash(cwd)} && ${commandLine}; echo; read -r -p 'nC finished. Press Enter to close...' _`;
  const candidates = [
    { cmd: 'x-terminal-emulator', args: ['-e', 'bash', '-lc', bashCommand] },
    { cmd: 'gnome-terminal', args: ['--', 'bash', '-lc', bashCommand] },
    { cmd: 'konsole', args: ['-e', 'bash', '-lc', bashCommand] },
    { cmd: 'xterm', args: ['-e', 'bash', '-lc', bashCommand] }
  ];
  for (const candidate of candidates) {
    if (!commandOnPath(candidate.cmd)) {
      continue;
    }
    const child = cp.spawn(candidate.cmd, candidate.args, { cwd, detached: true, stdio: 'ignore' });
    child.unref();
    return true;
  }
  return false;
}

function runCommandForFile(title, resolved, argv, fileName, outsideIde) {
  const commandLine = createResolvedTerminalCommand(resolved, argv, fileName);
  if (outsideIde) {
    if (launchExternalTerminal(title, commandLine, fileName)) {
      vscode.window.showInformationMessage(`${title}: launched outside VS Code`);
      return;
    }
    vscode.window.showErrorMessage(`${title}: low-level files must run outside VS Code, but no external terminal was found.`);
    return;
  }
  const terminal = createBestTerminal(title);
  terminal.show(true);
  terminal.sendText(commandLine);
}

function debugAdapterExecutable(resolved) {
  if (resolved.transport === 'wsl') {
    const command = `${shellQuoteForBash(resolved.nccPath || 'ncc')} debug-adapter`;
    return new vscode.DebugAdapterExecutable('wsl.exe', [
      ...wslCommandPrefix(resolved.distro).slice(1),
      'bash',
      '-lc',
      command
    ]);
  }
  return new vscode.DebugAdapterExecutable(resolved.nccPath || 'ncc', ['debug-adapter']);
}

function resolveNativeDebugAdapter(resolved) {
  if (resolved.transport === 'wsl') {
    const lookup = [
      'command -v lldb-dap',
      'command -v lldb-vscode',
      'find /usr/lib/llvm-* -maxdepth 2 -type f -name lldb-dap 2>/dev/null | sort -V | tail -1',
      'command -v gdb'
    ].join(' || ');
    const found = tryExecCapture('wsl.exe', [
      ...wslCommandPrefix(resolved.distro).slice(1),
      'bash',
      '-lc',
      lookup
    ]);
    const tool = (found.stdout || '').split(/\r?\n/).map((line) => line.trim()).find(Boolean);
    if (found.code !== 0 || !tool) {
      throw new Error('GDB DAP or lldb-dap is required for source debugging in the selected WSL toolchain');
    }
    const command = /(^|\/)gdb$/.test(tool)
      ? `exec ${shellQuoteForBash(tool)} -q -i dap -ex 'set debuginfod enabled off'`
      : `exec ${shellQuoteForBash(tool)}`;
    return new vscode.DebugAdapterExecutable('wsl.exe', [
      ...wslCommandPrefix(resolved.distro).slice(1),
      'bash',
      '-lc',
      command
    ]);
  }

  const configured = configuredPath('debugAdapterPath');
  const candidates = [
    configured,
    commandOnPath('lldb-dap'),
    commandOnPath('lldb-dap.exe'),
    commandOnPath('lldb-vscode'),
    process.platform === 'win32' && process.env.ProgramFiles
      ? path.join(process.env.ProgramFiles, 'LLVM', 'bin', 'lldb-dap.exe')
      : null,
    commandOnPath('gdb'),
    commandOnPath('gdb.exe'),
    process.platform === 'win32' ? 'C:\\msys64\\ucrt64\\bin\\gdb.exe' : null,
    process.platform === 'win32' ? 'C:\\msys64\\mingw64\\bin\\gdb.exe' : null
  ].filter(Boolean);
  const tool = candidates.find((candidate) => fs.existsSync(candidate) || commandOnPath(candidate));
  if (!tool) {
    throw new Error('GDB DAP or lldb-dap was not found; install GDB/LLVM or configure nc.debugAdapterPath');
  }
  const args = /(^|[\\/])gdb(?:\.exe)?$/i.test(tool)
    ? ['-q', '-i', 'dap', '-ex', 'set debuginfod enabled off']
    : [];
  return new vscode.DebugAdapterExecutable(tool, args);
}

function debugAdapterProbe(resolved) {
  if (resolved.transport === 'wsl') {
    const command = `${shellQuoteForBash(resolved.nccPath || 'ncc')} debug-adapter --probe --json`;
    const args = [
      ...wslCommandPrefix(resolved.distro).slice(1),
      'bash',
      '-lc',
      command
    ];
    return tryExecCapture('wsl.exe', args);
  }
  return tryExecCapture(resolved.nccPath || 'ncc', ['debug-adapter', '--probe', '--json']);
}

function debugProgramPathForResolved(resolved, fileName) {
  if (resolved.transport === 'wsl' && process.platform === 'win32') {
    return toWslPath(fileName);
  }
  return fileName;
}

function debugCwdForResolved(resolved, fileName) {
  const cwd = path.dirname(fileName);
  if (resolved.transport === 'wsl' && process.platform === 'win32') {
    return toWslPath(cwd);
  }
  return cwd;
}

function buildDebugConfiguration(fileName, resolved) {
  if (isTerminalOnlyMachineSource(fileName)) {
    return null;
  }
  const lane = ncFileLane(fileName);
  return {
    type: 'nc',
    request: 'launch',
    name: `nC Debug ${path.basename(fileName)}`,
    program: debugProgramPathForResolved(resolved, fileName),
    cwd: debugCwdForResolved(resolved, fileName),
    lane,
    profile: lane === 'source' ? 'debug' : lane,
    stopOnEntry: true
  };
}

function nativeDebugArtifactPaths(fileName) {
  const stem = path.basename(fileName, path.extname(fileName)).replace(/[^A-Za-z0-9_]/g, '_') || 'main';
  const root = path.join(path.dirname(fileName), '.ncc', 'debug');
  return {
    root,
    executable: path.join(root, 'bin', `${stem}.debug.exe`)
  };
}

function prepareNativeDebugBuild(fileName, resolved) {
  const artifacts = nativeDebugArtifactPaths(fileName);
  const result = execResolvedNccCapture(resolved, [
    'compiler',
    'build-native',
    fileName,
    '--profile',
    'debug',
    '--out-dir',
    artifacts.root,
    '--out',
    artifacts.executable,
    '--no-run',
    '--strict',
    '--json'
  ], fileName);
  if (result.code !== 0 || !fs.existsSync(artifacts.executable)) {
    throw new Error((result.stderr || result.stdout || 'native debug build failed').trim());
  }
  return resolved.transport === 'wsl' && process.platform === 'win32'
    ? toWslPath(artifacts.executable)
    : artifacts.executable;
}

async function startDebugSessionForFile(fileName, resolved) {
  if (isTerminalOnlyMachineSource(fileName)) {
    return false;
  }
  const probe = debugAdapterProbe(resolved);
  if (probe.code !== 0) {
    return false;
  }
  const folder = vscode.workspace.getWorkspaceFolder(vscode.Uri.file(fileName));
  const configRow = buildDebugConfiguration(fileName, resolved);
  return vscode.debug.startDebugging(folder, configRow);
}

function registerNcDebugAdapter(context) {
  context.subscriptions.push(vscode.debug.registerDebugConfigurationProvider('nc', {
    resolveDebugConfiguration(folder, debugConfig) {
      const activeFile = vscode.window.activeTextEditor?.document?.fileName;
      const requestedProgram = debugConfig.ncSourceProgram || debugConfig.program;
      const program = requestedProgram && requestedProgram !== '${file}'
        ? requestedProgram
        : activeFile;
      if (!program) {
        return undefined;
      }
      if (isTerminalOnlyMachineSource(program)) {
        showMachineTerminalOnlyMessage();
        return undefined;
      }
      const resolved = resolveNccForExecution(program);
      const configRow = Object.assign(
        {},
        buildDebugConfiguration(program, resolved),
        debugConfig,
        {
          type: 'nc',
          request: debugConfig.request || 'launch',
          program: debugProgramPathForResolved(resolved, program),
          cwd: debugCwdForResolved(resolved, program)
        }
      );
      if (ncFileLane(program) === 'source') {
        configRow.ncSourceProgram = program;
        configRow.ncNativeEngine = 'lldb-dap';
        configRow.program = prepareNativeDebugBuild(program, resolved);
        configRow.cwd = debugCwdForResolved(resolved, program);
        configRow.sourceLanguages = ['nc'];
      }
      return configRow;
    }
  }));

  context.subscriptions.push(vscode.debug.registerDebugAdapterDescriptorFactory('nc', {
    createDebugAdapterDescriptor(session) {
      const rawProgram = session.configuration.ncSourceProgram
        || session.configuration.program
        || vscode.window.activeTextEditor?.document?.fileName
        || '';
      const program = process.platform === 'win32' && rawProgram.startsWith('/mnt/')
        ? vscode.window.activeTextEditor?.document?.fileName || rawProgram
        : rawProgram;
      const resolved = resolveNccForExecution(program);
      if (session.configuration.ncNativeEngine === 'lldb-dap') {
        return resolveNativeDebugAdapter(resolved);
      }
      return debugAdapterExecutable(resolved);
    }
  }));
}

function runNccSubcommandTerminal(title, argv) {
  const resolved = resolveNccPath({ allowWsl: true });
  const terminal = createBestTerminal(title);
  terminal.show(true);
  terminal.sendText(createResolvedTerminalCommand(resolved, argv, vscode.window.activeTextEditor?.document?.fileName || null));
}

function runNccSubcommandCapture(argv) {
  const resolved = resolveNccPath();
  return execResolvedNccCapture(resolved, argv);
}

async function selectToolchain() {
  const ncupPath = configuredPath('ncupPath') || commandOnPath('ncup') || commandOnPath('ncup.exe') || 'ncup';
  const res = execCapture(ncupPath, ['toolchain', 'list', '--available']);
  if (res.code !== 0) {
    throw new Error(res.stderr || res.stdout || 'ncup toolchain list failed');
  }
  const lines = (res.stdout || '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .map((line) => line.replace(/^([*=])\s+/, '').replace(/\s+\[.*\]$/, ''));
  const picks = Array.from(new Set(lines));
  const chosen = await vscode.window.showQuickPick(picks, {
    placeHolder: 'Select nC toolchain'
  });
  if (!chosen) {
    return;
  }
  const setResult = execCapture(ncupPath, ['default', chosen]);
  if (setResult.code !== 0) {
    throw new Error(setResult.stderr || setResult.stdout || 'ncup default failed');
  }
  await config().update('toolchain', chosen, vscode.ConfigurationTarget.Workspace);
  vscode.window.showInformationMessage(`nC toolchain selected: ${chosen}`);
}

function discoverNccKits() {
  const kits = [];
  const add = (label, detail, nccPath, transport, kit) => {
    if (!nccPath || kits.some((row) => row.nccPath === nccPath && row.transport === transport)) {
      return;
    }
    kits.push({ label, detail, nccPath, transport, kit: kit || label });
  };

  const configuredNcc = configuredPath('nccPath');
  if (configuredNcc) {
    add('Configured ncc kit', 'From nc.nccPath', configuredNcc, (config().get('kitTransport') || 'native').toString(), config().get('kit') || 'configured');
  }

  const ncupPath = configuredPath('ncupPath') || commandOnPath('ncup') || commandOnPath('ncup.exe') || null;
  if (ncupPath) {
    const args = ['which', 'ncc', '--json'];
    const toolchain = resolveToolchainFlag();
    if (toolchain) {
      args.push('--toolchain', toolchain);
    }
    const res = tryExecCapture(ncupPath, args);
    if (res.code === 0 && res.stdout) {
      try {
        const parsed = JSON.parse(res.stdout);
        add(`ncup kit: ${parsed.toolchain_id || 'default'}`, parsed.path || ncupPath, parsed.path, 'native', parsed.toolchain_id || 'ncup');
      } catch (_err) {
        // Ignore malformed external output; other kit discovery paths remain available.
      }
    }
  }

  const pathNcc = commandOnPath('ncc') || commandOnPath('ncc.exe');
  if (pathNcc) {
    add('PATH ncc kit', pathNcc, pathNcc, 'native', 'path');
  }

  for (const candidate of workspaceNccCandidates()) {
    if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
      add('Workspace ncc kit', candidate, candidate, 'native', 'workspace');
    }
  }

  if (process.platform === 'win32') {
    const wsl = tryExecCapture('wsl.exe', ['bash', '-lc', 'command -v ncc']);
    if (wsl.code === 0 && wsl.stdout) {
      const wslNcc = wsl.stdout.split(/\r?\n/)[0].trim();
      add('WSL ncc kit', wslNcc, wslNcc, 'wsl', 'wsl');
    }
  }

  return kits;
}

async function writeNccKit(kit) {
  await config().update('kit', kit.kit || kit.label, vscode.ConfigurationTarget.Workspace);
  await config().update('kitTransport', kit.transport || 'native', vscode.ConfigurationTarget.Workspace);
  await config().update('nccPath', kit.nccPath, vscode.ConfigurationTarget.Workspace);
  vscode.window.showInformationMessage(`nC kit ready: ${kit.label}`);
}

async function selectNccKit() {
  const kits = discoverNccKits();
  kits.push({
    label: 'Manual ncc path',
    detail: 'Type the full path to ncc or ncc.exe',
    manual: true,
    transport: 'native',
    kit: 'manual'
  });
  const chosen = await vscode.window.showQuickPick(kits, {
    placeHolder: 'Select ncc kit'
  });
  if (!chosen) {
    return;
  }
  if (chosen.manual) {
    const nccPath = await vscode.window.showInputBox({
      prompt: 'Full path to ncc or ncc.exe',
      value: 'ncc'
    });
    if (!nccPath) {
      return;
    }
    const transport = await vscode.window.showQuickPick(['native', 'wsl'], {
      placeHolder: 'How should VS Code execute this ncc?'
    }) || 'native';
    await writeNccKit({
      label: `Manual ${transport} ncc kit`,
      nccPath,
      transport,
      kit: 'manual'
    });
    return;
  }
  await writeNccKit(chosen);
}

async function createNccKit() {
  const kits = discoverNccKits();
  if (kits.length === 1) {
    await writeNccKit(kits[0]);
    return;
  }
  await selectNccKit();
}

function compactWords(text, maxWords = 50) {
  const words = String(text || '')
    .replace(/\s+/g, ' ')
    .trim()
    .split(' ')
    .filter(Boolean);
  if (words.length <= maxWords) {
    return words.join(' ');
  }
  return `${words.slice(0, maxWords).join(' ')}...`;
}

function cleanExplanation(text, maxWords = 50) {
  const internalPatterns = [
    /\b(?:ncc|plugin|IDE)\b/i,
    /\bsemantic\s+authority\b/i,
    /\bsource\s+of\s+truth\b/i,
    /\broute\s+documentation\b/i,
    /\bcompiler\s+authority\b/i
  ];
  const raw = String(text || '')
    .replace(/\bnC\s+/g, '')
    .replace(/\s+/g, ' ')
    .trim();
  if (!raw) {
    return '';
  }
  const sentences = raw
    .split(/(?<=[.!?])\s+/)
    .map((row) => row.trim())
    .filter(Boolean)
    .filter((row) => !internalPatterns.some((pattern) => pattern.test(row)));
  const selected = sentences.length > 0 ? sentences.join(' ') : raw;
  return compactWords(selected, maxWords);
}

function appendExplanation(md, text, maxWords = 50) {
  const clean = cleanExplanation(text, maxWords);
  if (clean) {
    md.appendMarkdown(`${clean}\n\n`);
  }
}

function localKindName(kind) {
  if (kind === 'function') {
    return 'Function';
  }
  if (kind === 'container') {
    return 'Container';
  }
  return 'Variable';
}

function catalogMarkdown(entry) {
  const md = new vscode.MarkdownString();
  md.isTrusted = false;
  const title = entry.signature || entry.label;
  md.appendMarkdown(`**${title}**\n\n`);
  appendExplanation(md, entry.documentation || entry.detail);
  const preferred = core.preferredImportStatement(entry);
  if (preferred) {
    md.appendMarkdown(`Import: \`${preferred}\`\n\n`);
  }
  const legacy = core.legacyImportStatement(entry);
  if (legacy && legacy !== preferred) {
    md.appendMarkdown(`Legacy import: \`${legacy}\`\n\n`);
  }
  if (entry.risk) {
    md.appendMarkdown(`Risk: \`${entry.risk}\`\n\n`);
  }
  return md;
}

function buildHover(document, position, registry, catalog) {
  const lineText = document.lineAt(position.line).text;
  const positionedCatalogHit = core.lookupCatalogEntryAtLinePosition(catalog, lineText, position.character);
  if (positionedCatalogHit) {
    return new vscode.Hover(
      catalogMarkdown(positionedCatalogHit.entry),
      new vscode.Range(position.line, positionedCatalogHit.start, position.line, positionedCatalogHit.end)
    );
  }

  const callHit = core.callExpressionAtPosition(lineText, position.character);
  if (callHit && callHit.signature) {
    const sig = callHit.signature;
    const md = new vscode.MarkdownString();
    md.isTrusted = false;
    md.appendCodeblock(sig.signature, 'n');
    appendExplanation(md, sig.detail);
    if (sig.params.length > 0) {
      md.appendMarkdown(`Parameters: ${sig.params.map((param) => `\`${param}\``).join(', ')}\n\n`);
    }
    if (sig.importHint && !core.isBackendPreludeSymbol(sig.name)) {
      md.appendMarkdown(`Import: \`${sig.importHint}\`\n\n`);
    }
    return new vscode.Hover(
      md,
      new vscode.Range(position.line, callHit.nameStart, position.line, callHit.close + 1)
    );
  }

  const word = core.findWordRangeInLine(lineText, position.character);

  const plusChar = lineText[position.character];
  if (plusChar === '+') {
    const plus = registry.plus_operator || {};
    const md = new vscode.MarkdownString();
    md.appendMarkdown('**+**\n\n');
    appendExplanation(md, plus.strict_concat_message || 'Numeric addition. Text composition uses explicit render helpers in strict profiles.');
    const allowed = Array.isArray(plus.allowed_textual_forms) ? plus.allowed_textual_forms : [];
    if (allowed.length > 0) {
      md.appendMarkdown('Examples:\n');
      for (const row of allowed.slice(0, 4)) {
        md.appendMarkdown(`- \`${row}\`\n`);
      }
    }
    return new vscode.Hover(md, new vscode.Range(position.line, position.character, position.line, position.character + 1));
  }

  for (const marker of core.MODE_MARKERS) {
    const idx = lineText.indexOf(marker);
    if (idx >= 0 && position.character >= idx && position.character <= idx + marker.length) {
      const md = new vscode.MarkdownString();
      md.appendMarkdown(`**${marker}**\n\n`);
      appendExplanation(md, 'Mode marker. It selects the language surface used by the surrounding block.');
      return new vscode.Hover(md, new vscode.Range(position.line, idx, position.line, idx + marker.length));
    }
  }

  if (!word) {
    return null;
  }

  const token = word.text;
  const normalized = core.normalizeWord(token);
  const diagnosticHit = core.lookupDiagnosticHelp(normalized);
  if (diagnosticHit) {
    const md = new vscode.MarkdownString();
    md.appendMarkdown(`**${diagnosticHit.code}**\n\n`);
    appendExplanation(md, diagnosticHit.detail);
    return new vscode.Hover(md, new vscode.Range(position.line, word.start, position.line, word.end));
  }

  const catalogHit = core.lookupCatalogEntry(catalog, normalized);
  if (catalogHit) {
    return new vscode.Hover(catalogMarkdown(catalogHit), new vscode.Range(position.line, word.start, position.line, word.end));
  }

  const printHit = core.lookupPrintMember(registry, token.startsWith('print') ? token : normalized);
  if (printHit) {
    const md = new vscode.MarkdownString();
    md.appendMarkdown(`**${printHit.title}**\n\n`);
    appendExplanation(md, printHit.detail);
    return new vscode.Hover(md, new vscode.Range(position.line, word.start, position.line, word.end));
  }

  const runtimeHit = core.lookupRuntimeFamily(registry, token);
  if (runtimeHit) {
    const md = new vscode.MarkdownString();
    md.appendMarkdown(`**${runtimeHit.title}**\n\n`);
    appendExplanation(md, runtimeHit.detail);
    return new vscode.Hover(md, new vscode.Range(position.line, word.start, position.line, word.end));
  }

  const symbols = core.parseSymbols(document.getText());
  const all = [...symbols.containers, ...symbols.functions, ...symbols.variables];
  const hit = all.find((row) => row.name === normalized);
  if (hit) {
    const kind = symbols.functions.includes(hit) ? 'function' : (symbols.variables.includes(hit) ? 'variable' : 'container');
    const md = new vscode.MarkdownString();
    md.appendMarkdown(`**${normalized}**\n\n`);
    md.appendMarkdown(`${localKindName(kind)}.`);
    if (hit.typeName) {
      md.appendMarkdown(` Type: \`${hit.typeName}\`.`);
    }
    const usageCount = core.collectOccurrences(document.getText(), normalized).length;
    md.appendMarkdown(` Used ${usageCount} time${usageCount === 1 ? '' : 's'} in this file.`);
    return new vscode.Hover(md, new vscode.Range(position.line, word.start, position.line, word.end));
  }

  const pointerHit = core.lookupPointerBase(normalized);
  if (pointerHit) {
    const md = new vscode.MarkdownString();
    md.appendMarkdown(`**${pointerHit.title}**\n\n`);
    appendExplanation(md, pointerHit.detail);
    return new vscode.Hover(md, new vscode.Range(position.line, word.start, position.line, word.end));
  }

  const typeMap = core.typedSymbolMap(document.getText());
  if (typeMap.has(normalized)) {
    const md = new vscode.MarkdownString();
    md.appendMarkdown(`**@${normalized}**\n\n`);
    md.appendMarkdown(`Projected type: \`${typeMap.get(normalized)}\`.`);
    return new vscode.Hover(md, new vscode.Range(position.line, word.start, position.line, word.end));
  }

  if (core.TYPES.includes(normalized)) {
    const definition = core.lookupTypeDefinition(normalized);
    const md = new vscode.MarkdownString();
    md.appendMarkdown(`**${definition ? definition.title : normalized}**\n\n`);
    if (definition) {
      appendExplanation(md, definition.detail);
    }
    return new vscode.Hover(md, new vscode.Range(position.line, word.start, position.line, word.end));
  }

  return null;
}

function missingImportEditForCompletion(document, row) {
  const statement = core.preferredImportStatement(row);
  if (!document || !statement || core.hasImportForEntry(document.getText(), row)) {
    return [];
  }
  return importTextEdits(document, statement);
}

function completionKindForCatalog(row) {
  if (row.kind === 'type') {
    return vscode.CompletionItemKind.TypeParameter;
  }
  if (row.kind === 'mode') {
    return vscode.CompletionItemKind.Module;
  }
  if (row.kind === 'operator') {
    return vscode.CompletionItemKind.Operator;
  }
  if (row.kind === 'section') {
    return vscode.CompletionItemKind.Struct;
  }
  if (row.kind === 'projection') {
    return vscode.CompletionItemKind.Snippet;
  }
  if (row.kind === 'print-control') {
    return vscode.CompletionItemKind.Snippet;
  }
  if (row.kind === 'auth' || row.kind === 'dom') {
    return vscode.CompletionItemKind.Module;
  }
  if (row.kind === 'diagnostic') {
    return vscode.CompletionItemKind.Constant;
  }
  return vscode.CompletionItemKind.Function;
}

function catalogSignatureSuffix(row) {
  const label = row.label || '';
  const signature = row.signature || '';
  if (!signature || !label || !signature.startsWith(label)) {
    return '';
  }
  return signature.slice(label.length).trim();
}

function catalogScopeLabel(row) {
  const scope = core.facadeImportScope(row);
  if (!scope) {
    if (row.legacyImport || row.importPath) {
      return 'legacy use';
    }
    return row.kind || 'nC';
  }
  const item = scope.items && scope.items.length === 1 ? `:${scope.items[0]}` : '';
  return `${scope.domain}/${scope.family}${item}`;
}

function catalogCompletionLabel(row) {
  return {
    label: row.label,
    detail: catalogSignatureSuffix(row),
    description: catalogScopeLabel(row)
  };
}

function catalogSortBucket(row) {
  const scope = core.facadeImportScope(row);
  if (row.kind === 'function') {
    return '10';
  }
  if (row.kind === 'type') {
    return '20';
  }
  if (row.kind === 'section') {
    return '25';
  }
  if (row.kind === 'mode') {
    return '26';
  }
  if (scope && scope.domain === 'stoud') {
    return '30';
  }
  if (scope && scope.domain === 'dom') {
    return '40';
  }
  if (scope && scope.domain === 'auth') {
    return '80';
  }
  if (row.kind === 'diagnostic') {
    return '70';
  }
  if (row.kind === 'operator' || row.kind === 'projection') {
    return '75';
  }
  return '60';
}

function decorateCatalogCompletion(item, row, index) {
  item.filterText = row.label;
  item.sortText = `${catalogSortBucket(row)}_${String(index).padStart(4, '0')}_${row.label}`;
  item.detail = row.signature || cleanExplanation(row.detail, 18) || (row.kind || 'symbol');
  item.documentation = catalogMarkdown(row);
  if (row.kind === 'function') {
    item.commitCharacters = ['(', ';'];
  } else if (row.kind === 'type') {
    item.commitCharacters = ['<', ' ', ';'];
  }
  if (!core.autoImportAllowed(row) && core.preferredImportStatement(row)) {
    item.detail = `${item.detail} - policy required`;
  }
  return item;
}

function snippetForCatalog(row) {
  const text = row.insertText || row.label;
  if (text.includes('${')) {
    return new vscode.SnippetString(text);
  }
  if (row.kind === 'function' && !text.includes('(')) {
    return new vscode.SnippetString(`${text}($1)`);
  }
  return text;
}

function projectRootForDocument(document) {
  if (!document || !document.uri || document.uri.scheme !== 'file') {
    return null;
  }
  const folder = vscode.workspace.getWorkspaceFolder(document.uri);
  return folder ? folder.uri.fsPath : path.dirname(document.fileName);
}

function collectProjectSourceFiles(root, limit = 300, maxDepth = 5) {
  const out = [];
  const ignored = new Set([
    '.git', '.hg', '.svn', '.ncc', 'target', 'node_modules', 'dist', 'build',
    'out', 'cmake-build-debug', '__pycache__'
  ]);
  const walk = (dir, depth) => {
    if (!dir || depth > maxDepth || out.length >= limit) {
      return;
    }
    let entries = [];
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch (_err) {
      return;
    }
    for (const entry of entries) {
      if (out.length >= limit) {
        return;
      }
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        if (!ignored.has(entry.name)) {
          walk(full, depth + 1);
        }
        continue;
      }
      if (entry.isFile() && ['.n', '.nc'].includes(path.extname(entry.name).toLowerCase())) {
        out.push(full);
      }
    }
  };
  walk(root, 0);
  return out;
}

function cachedProjectSourceFiles(document) {
  const root = projectRootForDocument(document);
  if (!root) {
    return [];
  }
  const now = Date.now();
  const cached = PROJECT_MODULE_CACHE.get(root);
  if (cached && now - cached.time < PROJECT_MODULE_CACHE_MS) {
    return cached.files;
  }
  const files = collectProjectSourceFiles(root);
  PROJECT_MODULE_CACHE.set(root, { time: now, files });
  return files;
}

function sourceFilesForNavigation(document) {
  const files = new Set(cachedProjectSourceFiles(document));
  if (document && document.fileName) {
    files.add(document.fileName);
  }
  return Array.from(files)
    .filter((file) => file && fs.existsSync(file))
    .sort((a, b) => {
      const aCurrent = document && path.resolve(a).toLowerCase() === path.resolve(document.fileName).toLowerCase();
      const bCurrent = document && path.resolve(b).toLowerCase() === path.resolve(document.fileName).toLowerCase();
      if (aCurrent !== bCurrent) {
        return aCurrent ? -1 : 1;
      }
      return a.localeCompare(b);
    });
}

function resolveProjectModuleFile(document, moduleName) {
  if (!moduleName || !/^[A-Za-z_][A-Za-z0-9_]*$/.test(moduleName)) {
    return null;
  }
  const current = document?.fileName ? path.resolve(document.fileName).toLowerCase() : '';
  if (document && !document.isUntitled && core.isNcLanguageId(document.languageId)) {
    const model = buildProjectModel(document);
    const node = model.nodes.get(normalizeProjectModuleName(moduleName));
    if (node) {
      const modelFiles = Array.from(node.files)
        .filter((file) => file && fs.existsSync(file) && (!current || path.resolve(file).toLowerCase() !== current))
        .sort((a, b) => a.localeCompare(b));
      if (modelFiles.length > 0) {
        return modelFiles[0];
      }
    }
  }
  const matches = cachedProjectSourceFiles(document)
    .filter((file) => {
      const ext = path.extname(file).toLowerCase();
      return ['.n', '.nc'].includes(ext)
        && path.basename(file, ext) === moduleName
        && (!current || path.resolve(file).toLowerCase() !== current);
    })
    .sort((a, b) => {
      const extA = path.extname(a).toLowerCase() === '.n' ? 0 : 1;
      const extB = path.extname(b).toLowerCase() === '.n' ? 0 : 1;
      return extA - extB || a.localeCompare(b);
    });
  return matches[0] || null;
}

function readJsonFileSafe(fileName) {
  if (!fileName || !fs.existsSync(fileName)) {
    return null;
  }
  try {
    return JSON.parse(fs.readFileSync(fileName, 'utf8'));
  } catch (_err) {
    return null;
  }
}

function relativeProjectPath(root, fileName) {
  if (!fileName) {
    return '';
  }
  const rel = root ? path.relative(root, fileName) : fileName;
  return (rel || path.basename(fileName)).replace(/\\/g, '/');
}

function projectSourcePath(root, raw) {
  if (!raw || typeof raw !== 'string') {
    return null;
  }
  return path.isAbsolute(raw) ? raw : path.join(root, raw);
}

function normalizeProjectModuleName(raw) {
  const text = String(raw || '').trim();
  if (!text) {
    return '';
  }
  const ext = path.extname(text);
  const base = ext ? path.basename(text, ext) : text;
  return base.replace(/[^A-Za-z0-9_]/g, '_');
}

function stringArrayFromProjectValue(value) {
  if (!value) {
    return [];
  }
  if (Array.isArray(value)) {
    return value.flatMap((row) => stringArrayFromProjectValue(row));
  }
  if (typeof value === 'string') {
    return [value];
  }
  if (typeof value === 'object') {
    if (typeof value.name === 'string') {
      return [value.name];
    }
    if (typeof value.module === 'string') {
      return [value.module];
    }
    if (typeof value.unit === 'string') {
      return [value.unit];
    }
    if (typeof value.source === 'string') {
      return [value.source];
    }
  }
  return [];
}

function compactProjectImports(text) {
  const out = [];
  for (const re of [
    /^\s*use\s*<\s*project\s*:\s*([^>]*)>\s*;?/gm,
    /^\s*use\s+project\s*\{\s*([^}]*)\s*\}\s*;?/gm
  ]) {
    let match = re.exec(text || '');
    while (match) {
      for (const item of match[1].split(',')) {
        const name = item.trim();
        if (/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) {
          out.push(name);
        }
      }
      match = re.exec(text || '');
    }
  }
  return out;
}

function addProjectContainer(containerMap, row) {
  if (!row || !row.name) {
    return;
  }
  const list = containerMap.get(row.name) || [];
  const rowFile = row.file ? path.resolve(row.file).toLowerCase() : '';
  if (row.origin !== 'source' && rowFile && list.some((item) => {
    const itemFile = item.file ? path.resolve(item.file).toLowerCase() : '';
    return itemFile && itemFile === rowFile && item.name === row.name;
  })) {
    return;
  }
  const key = `${path.resolve(row.file || '').toLowerCase()}:${row.line ?? -1}:${row.name}`;
  if (!list.some((item) => item.key === key)) {
    list.push(Object.assign({ key }, row));
    containerMap.set(row.name, list);
  }
}

function readNccCodeJsonUnits(root) {
  const dir = root ? path.join(root, '.ncc', 'code') : null;
  if (!dir || !fs.existsSync(dir)) {
    return [];
  }
  let entries = [];
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch (_err) {
    return [];
  }
  const units = [];
  for (const entry of entries) {
    if (!entry.isFile() || path.extname(entry.name).toLowerCase() !== '.json') {
      continue;
    }
    const file = path.join(dir, entry.name);
    const payload = readJsonFileSafe(file);
    if (!payload || typeof payload !== 'object') {
      continue;
    }
    const name = normalizeProjectModuleName(
      payload.name || payload.module || payload.unit || payload.id || path.basename(entry.name, '.json')
    );
    const source = projectSourcePath(root, payload.source || payload.source_path || payload.file || payload.path);
    const imports = [
      ...stringArrayFromProjectValue(payload.imports),
      ...stringArrayFromProjectValue(payload.deps),
      ...stringArrayFromProjectValue(payload.dependencies),
      ...stringArrayFromProjectValue(payload.uses)
    ].map(normalizeProjectModuleName).filter(Boolean);
    const containers = [];
    for (const symbol of Array.isArray(payload.symbols) ? payload.symbols : []) {
      const symbolName = symbol && (symbol.container || (symbol.kind === 'container' ? symbol.name : null));
      const containerName = normalizeProjectModuleName(symbolName);
      if (containerName) {
        containers.push(containerName);
      }
    }
    const directContainer = normalizeProjectModuleName(payload.container || payload.container_name);
    if (directContainer) {
      containers.push(directContainer);
    }
    units.push({
      name,
      source,
      code: file,
      imports: Array.from(new Set(imports)),
      containers: Array.from(new Set(containers))
    });
  }
  return units.sort((a, b) => a.name.localeCompare(b.name));
}

function ensureProjectNode(nodes, name) {
  const key = normalizeProjectModuleName(name);
  if (!key) {
    return null;
  }
  if (!nodes.has(key)) {
    nodes.set(key, {
      name: key,
      files: new Set(),
      codeFiles: new Set(),
      imports: new Set(),
      importedBy: new Set(),
      containers: [],
      hasEntrypoint: false,
      fromBridge: false,
      fromCode: false,
      role: 'unidad'
    });
  }
  return nodes.get(key);
}

function addProjectEdge(nodes, edges, from, to, origin) {
  const source = ensureProjectNode(nodes, from);
  const target = ensureProjectNode(nodes, to);
  if (!source || !target || source.name === target.name) {
    return;
  }
  const key = `${source.name}->${target.name}`;
  if (!edges.some((edge) => edge.key === key)) {
    edges.push({ key, from: source.name, to: target.name, origin: origin || 'source' });
  }
  source.imports.add(target.name);
  target.importedBy.add(source.name);
}

function detectProjectCycles(nodes, edges) {
  const adjacency = new Map();
  for (const name of nodes.keys()) {
    adjacency.set(name, []);
  }
  for (const edge of edges) {
    adjacency.get(edge.from)?.push(edge.to);
  }
  const state = new Map();
  const stack = [];
  const seen = new Set();
  const cycles = [];

  const visit = (node) => {
    state.set(node, 'visiting');
    stack.push(node);
    for (const next of adjacency.get(node) || []) {
      if (state.get(next) === 'visiting') {
        const start = stack.indexOf(next);
        const cycle = stack.slice(start).concat(next);
        const canonical = cycle.slice(0, -1).sort().join('|');
        if (!seen.has(canonical)) {
          seen.add(canonical);
          cycles.push(cycle);
        }
        continue;
      }
      if (!state.has(next)) {
        visit(next);
      }
    }
    stack.pop();
    state.set(node, 'visited');
  };

  for (const name of nodes.keys()) {
    if (!state.has(name)) {
      visit(name);
    }
  }
  return cycles;
}

function projectResolutionRank(model, row) {
  const file = row.file ? path.resolve(row.file).toLowerCase() : '';
  const moduleRank = model.linkOrder.indexOf(row.module);
  if (model.mainFile && file === path.resolve(model.mainFile).toLowerCase()) {
    return [0, moduleRank < 0 ? 9999 : moduleRank, relativeProjectPath(model.root, row.file)];
  }
  if (moduleRank >= 0) {
    return [1, moduleRank, relativeProjectPath(model.root, row.file)];
  }
  return [2, 9999, relativeProjectPath(model.root, row.file)];
}

function compareProjectContainerRows(model, a, b) {
  const ar = projectResolutionRank(model, a);
  const br = projectResolutionRank(model, b);
  for (let i = 0; i < ar.length; i += 1) {
    if (ar[i] < br[i]) {
      return -1;
    }
    if (ar[i] > br[i]) {
      return 1;
    }
  }
  return 0;
}

function projectDuplicateContainers(model) {
  const out = [];
  for (const [name, rows] of model.containersByName.entries()) {
    if (rows.length < 2) {
      continue;
    }
    const ordered = rows.slice().sort((a, b) => compareProjectContainerRows(model, a, b));
    out.push({ name, canonical: ordered[0], rows: ordered });
  }
  return out.sort((a, b) => a.name.localeCompare(b.name));
}

function buildProjectModel(document) {
  const workspaceRoot = document ? projectRootForDocument(document) : vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
  const startPath = document?.fileName || workspaceRoot || process.cwd();
  const bridgeInfo = findBridgeJson(startPath);
  const root = bridgeInfo?.root || workspaceRoot || path.dirname(startPath);
  const bridge = bridgeInfo ? readJsonFileSafe(bridgeInfo.bridge) : null;
  const nodes = new Map();
  const edges = [];
  const containersByName = new Map();
  const sourceFiles = new Set(collectProjectSourceFiles(root));
  const codeUnits = readNccCodeJsonUnits(root);
  const bridgeUnits = Array.isArray(bridge?.units) ? bridge.units : [];
  const linkOrder = Array.isArray(bridge?.link_order)
    ? bridge.link_order.map(normalizeProjectModuleName).filter(Boolean)
    : [];
  const mainFile = bridge?.main ? projectSourcePath(root, bridge.main) : null;

  if (document?.fileName) {
    sourceFiles.add(document.fileName);
  }
  for (const unit of bridgeUnits) {
    if (unit.source) {
      sourceFiles.add(projectSourcePath(root, unit.source));
    }
  }
  for (const unit of codeUnits) {
    if (unit.source) {
      sourceFiles.add(unit.source);
    }
  }

  for (const file of Array.from(sourceFiles).filter(Boolean).sort((a, b) => a.localeCompare(b))) {
    if (!fs.existsSync(file)) {
      continue;
    }
    const ext = path.extname(file).toLowerCase();
    const moduleName = normalizeProjectModuleName(path.basename(file, ext));
    const node = ensureProjectNode(nodes, moduleName);
    if (!node) {
      continue;
    }
    node.files.add(file);
    let text = '';
    try {
      text = document?.fileName && path.resolve(document.fileName).toLowerCase() === path.resolve(file).toLowerCase()
        ? document.getText()
        : fs.readFileSync(file, 'utf8');
    } catch (_err) {
      continue;
    }
    const symbols = core.parseSymbols(text);
    node.hasEntrypoint = node.hasEntrypoint || symbols.mains.length > 0 || (mainFile && path.resolve(file).toLowerCase() === path.resolve(mainFile).toLowerCase());
    for (const container of symbols.containers) {
      node.containers.push(container.name);
      addProjectContainer(containersByName, {
        name: container.name,
        file,
        line: container.line,
        character: container.character,
        module: node.name,
        origin: 'source'
      });
    }
    for (const imported of compactProjectImports(text)) {
      addProjectEdge(nodes, edges, node.name, imported, 'source');
    }
  }

  for (const unit of bridgeUnits) {
    const node = ensureProjectNode(nodes, unit.name || unit.source || unit.code);
    if (!node) {
      continue;
    }
    node.fromBridge = true;
    if (unit.source) {
      node.files.add(projectSourcePath(root, unit.source));
    }
    if (unit.code) {
      node.codeFiles.add(projectSourcePath(root, unit.code));
    }
    const imports = [
      ...stringArrayFromProjectValue(unit.imports),
      ...stringArrayFromProjectValue(unit.deps),
      ...stringArrayFromProjectValue(unit.dependencies)
    ].map(normalizeProjectModuleName).filter(Boolean);
    for (const imported of imports) {
      addProjectEdge(nodes, edges, node.name, imported, 'bridge');
    }
    for (const symbol of Array.isArray(unit.symbols) ? unit.symbols : []) {
      const symbolName = normalizeProjectModuleName(symbol?.container || (symbol?.kind === 'container' ? symbol.name : null));
      if (symbolName) {
        node.containers.push(symbolName);
        addProjectContainer(containersByName, {
          name: symbolName,
          file: unit.source ? projectSourcePath(root, unit.source) : null,
          line: 0,
          character: 0,
          module: node.name,
          origin: 'bridge'
        });
      }
    }
  }

  if (Array.isArray(bridge?.edges)) {
    for (const row of bridge.edges) {
      if (Array.isArray(row) && row.length >= 2) {
        addProjectEdge(nodes, edges, row[0], row[1], 'bridge');
      } else if (row && typeof row === 'object') {
        addProjectEdge(nodes, edges, row.from || row.source, row.to || row.target, 'bridge');
      }
    }
  }

  for (const unit of codeUnits) {
    const node = ensureProjectNode(nodes, unit.name || unit.source || unit.code);
    if (!node) {
      continue;
    }
    node.fromCode = true;
    if (unit.source) {
      node.files.add(unit.source);
    }
    if (unit.code) {
      node.codeFiles.add(unit.code);
    }
    for (const imported of unit.imports || []) {
      addProjectEdge(nodes, edges, node.name, imported, '.ncc/code');
    }
    for (const containerName of unit.containers || []) {
      node.containers.push(containerName);
      addProjectContainer(containersByName, {
        name: containerName,
        file: unit.source || null,
        line: 0,
        character: 0,
        module: node.name,
        origin: '.ncc/code'
      });
    }
  }

  for (const node of nodes.values()) {
    if (node.hasEntrypoint) {
      node.role = node.imports.size > 0 ? 'madre principal' : 'madre principal sin hijos';
    } else if (node.imports.size > 0 && node.importedBy.size > 0) {
      node.role = 'madre/hijo';
    } else if (node.imports.size > 0) {
      node.role = 'madre';
    } else if (node.importedBy.size > 0) {
      node.role = 'hijo';
    } else {
      node.role = 'unidad';
    }
    node.containers = Array.from(new Set(node.containers)).sort((a, b) => a.localeCompare(b));
  }

  const model = {
    root,
    bridgePath: bridgeInfo?.bridge || null,
    bridge,
    mainFile,
    linkOrder,
    nodes,
    edges,
    containersByName,
    codeUnits,
    cycles: [],
    duplicates: []
  };
  model.cycles = detectProjectCycles(nodes, edges);
  model.duplicates = projectDuplicateContainers(model);
  return model;
}

function projectGraphDocument(model) {
  const lines = [
    'Project graph',
    '',
    `Root: ${model.root}`,
    `bridge.json: ${model.bridgePath ? relativeProjectPath(model.root, model.bridgePath) : 'not found'}`,
    `.ncc/code/*.json: ${model.codeUnits.length} unit(s)`,
    `Nodes: ${model.nodes.size}`,
    `Edges: ${model.edges.length}`,
    '',
    'Mermaid:',
    '```mermaid',
    'graph TD'
  ];
  if (model.nodes.size === 0) {
    lines.push('  empty["empty project"]');
  } else {
    for (const node of Array.from(model.nodes.values()).sort((a, b) => a.name.localeCompare(b.name))) {
      const marker = node.hasEntrypoint ? 'main' : node.role;
      lines.push(`  ${node.name}["${node.name}\\n${marker}"]`);
    }
    for (const edge of model.edges) {
      lines.push(`  ${edge.from} --> ${edge.to}`);
    }
  }
  lines.push('```', '');

  lines.push('Madres e hijos:');
  for (const node of Array.from(model.nodes.values()).sort((a, b) => a.name.localeCompare(b.name))) {
    const source = Array.from(node.files).map((file) => relativeProjectPath(model.root, file)).join(', ') || 'sin fuente';
    const children = Array.from(node.imports).sort((a, b) => a.localeCompare(b)).join(', ') || '-';
    const parents = Array.from(node.importedBy).sort((a, b) => a.localeCompare(b)).join(', ') || '-';
    lines.push(`- ${node.name}: ${node.role}; hijos=[${children}]; madres=[${parents}]; fuente=${source}`);
  }
  lines.push('');

  if (model.cycles.length > 0) {
    lines.push('Ciclos detectados en el grafo:');
    for (const cycle of model.cycles) {
      lines.push(`- ${cycle.join(' -> ')}`);
    }
    lines.push('Los ciclos se muestran aqui para inspeccion; no generan diagnostico del editor.');
  } else {
    lines.push('Ciclos detectados en el grafo: ninguno.');
  }
  lines.push('');

  if (model.duplicates.length > 0) {
    lines.push('Contenedores duplicados:');
    for (const duplicate of model.duplicates) {
      const canonical = duplicate.canonical;
      const canonicalPath = relativeProjectPath(model.root, canonical.file);
      const shadows = duplicate.rows
        .filter((row) => row !== canonical)
        .map((row) => relativeProjectPath(model.root, row.file))
        .join(', ');
      lines.push(`- ${duplicate.name}: gana ${canonicalPath || canonical.module}; quedan sombreados ${shadows || '-'}.`);
    }
    lines.push('Resolucion: bridge main primero, luego link_order, despues ruta lexicografica.');
  } else {
    lines.push('Contenedores duplicados: ninguno.');
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function projectDuplicateContainerDiagnostics(document) {
  if (!document || document.isUntitled || !core.isNcLanguageId(document.languageId)) {
    return [];
  }
  const model = buildProjectModel(document);
  const current = path.resolve(document.fileName).toLowerCase();
  const out = [];
  for (const duplicate of model.duplicates) {
    for (const row of duplicate.rows) {
      if (!row.file || path.resolve(row.file).toLowerCase() !== current) {
        continue;
      }
      const canonicalPath = relativeProjectPath(model.root, duplicate.canonical.file);
      const rowPath = relativeProjectPath(model.root, row.file);
      const wins = row === duplicate.canonical;
      const msg = wins
        ? `Duplicate container '${duplicate.name}'. Project order keeps ${rowPath} first; later declarations are shadowed.`
        : `Duplicate container '${duplicate.name}'. Project order keeps ${canonicalPath} first; this declaration is shadowed.`;
      const diagnostic = new vscode.Diagnostic(
        new vscode.Range(row.line || 0, row.character || 0, row.line || 0, (row.character || 0) + duplicate.name.length),
        msg,
        vscode.DiagnosticSeverity.Warning
      );
      diagnostic.source = 'nC project';
      diagnostic.code = 'NC_PROJECT_CONTAINER_DUPLICATE_001';
      diagnostic.relatedInformation = duplicate.rows
        .filter((other) => other.file)
        .map((other) => new vscode.DiagnosticRelatedInformation(
          new vscode.Location(
            vscode.Uri.file(other.file),
            new vscode.Position(other.line || 0, other.character || 0)
          ),
          other === duplicate.canonical ? 'first declaration by project order' : 'shadowed duplicate declaration'
        ));
      out.push(diagnostic);
    }
  }
  return out;
}

async function showProjectGraph() {
  const active = vscode.window.activeTextEditor?.document || null;
  const document = active?.uri?.scheme === 'file' ? active : null;
  const model = buildProjectModel(document);
  const location = createVirtualDefinition('project-graph', projectGraphDocument(model));
  await vscode.commands.executeCommand('vscode.open', location.uri);
}

function createVirtualDefinition(title, body) {
  virtualDefinitionSeq += 1;
  const safe = title.replace(/[^A-Za-z0-9_.-]/g, '_').slice(0, 80) || 'definition';
  const uri = vscode.Uri.parse(`${VIRTUAL_DEFINITION_SCHEME}:/${safe}-${virtualDefinitionSeq}.ncdoc`);
  VIRTUAL_DEFINITIONS.set(uri.toString(), body);
  return new vscode.Location(uri, new vscode.Position(0, 0));
}

function registerVirtualDefinitionProvider(context) {
  context.subscriptions.push(vscode.workspace.registerTextDocumentContentProvider(
    VIRTUAL_DEFINITION_SCHEME,
    {
      provideTextDocumentContent(uri) {
        return VIRTUAL_DEFINITIONS.get(uri.toString()) || 'Definition unavailable.\n';
      }
    }
  ));
}

function importCatalogRowsFor(facadeImport, catalog) {
  const entries = core.catalogEntries(catalog);
  return entries.filter((row) => (row.facadeImport || '').replace(/\s+/g, ' ') === facadeImport.replace(/\s+/g, ' '));
}

function virtualFacadeImportDefinition(domain, family, item, catalog) {
  const module = domain === 'project' ? 'project' : domain === 'stoud' ? family : `${domain}.${family}`;
  const surface = item
    ? `use <${module}:${item}>;`
    : `use <${module}:...>;`;
  const rows = item ? importCatalogRowsFor(surface, catalog) : [];
  const title = item ? `${domain}.${family}.${item}` : `${domain}.${family}`;
  const lines = [
    `Import ${surface}`,
    '',
    item
      ? `Activa el item '${item}' de la familia '${family}' dentro del dominio '${domain}'.`
      : `Agrupa items de la familia '${family}' dentro del dominio '${domain}'.`,
    ''
  ];
  if (rows.length > 0) {
    lines.push('Funciones relacionadas:');
    for (const row of rows.slice(0, 12)) {
      lines.push(`- ${row.label || row.symbol || row.name}: ${row.detail || row.documentation || 'import item'}`);
    }
  } else {
    lines.push('Definicion de import compacto. El compilador valida si el item tiene respaldo real.');
  }
  lines.push('');
  return createVirtualDefinition(title, `${lines.join('\n')}\n`);
}

function virtualLegacyImportDefinition(runtimeFamily, catalog) {
  const legacy = `nC.runtime.${runtimeFamily}`;
  const entries = core.catalogEntries(catalog).filter((row) => row.legacyImport === legacy || row.importPath === legacy);
  const lines = [
    `Import use ${legacy};`,
    '',
    `Import legacy de runtime para la familia '${runtimeFamily}'. La forma oficial es use stoud.familia { elemento }; cuando existe equivalencia.`,
    ''
  ];
  for (const row of entries.slice(0, 12)) {
    lines.push(`- ${row.label || row.symbol || row.name}: ${row.detail || row.documentation || 'runtime symbol'}`);
  }
  if (entries.length === 0) {
    lines.push('Familia runtime reconocida por el lenguaje.');
  }
  lines.push('');
  return createVirtualDefinition(legacy, `${lines.join('\n')}\n`);
}

function itemAtListPosition(itemsRaw, listStart, character) {
  let cursor = listStart;
  for (const raw of itemsRaw.split(',')) {
    const leading = raw.match(/^\s*/)?.[0]?.length || 0;
    const item = raw.trim();
    const start = cursor + leading;
    const end = start + item.length;
    if (item && character >= start && character <= end) {
      return item;
    }
    cursor += raw.length + 1;
  }
  return itemsRaw.split(',').map((item) => item.trim()).filter(Boolean)[0] || '';
}

function projectImportDefinitionAtPosition(document, lineText, position) {
  const match = lineText.match(/^\s*use\s*<\s*project\s*:\s*([^>]*)>\s*;?/)
    || lineText.match(/^\s*use\s+project\s*\{\s*([^}]*)\s*\}\s*;?/);
  if (!match) {
    return null;
  }
  const listStart = lineText.includes('<') ? lineText.indexOf(':') + 1 : lineText.indexOf('{') + 1;
  const moduleName = itemAtListPosition(match[1], listStart, position.character);
  const target = resolveProjectModuleFile(document, moduleName);
  if (!target) {
    return null;
  }
  return new vscode.Location(vscode.Uri.file(target), new vscode.Position(0, 0));
}

function importDefinitionAtPosition(document, position, catalog) {
  const lineText = document.lineAt(position.line).text;
  const project = projectImportDefinitionAtPosition(document, lineText, position);
  if (project) {
    return project;
  }

  const compact = lineText.match(/^\s*use\s*<\s*([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)?)\s*:\s*([^>]*)>\s*;?/);
  if (compact && compact[1] !== 'project') {
    const path = compact[1].split('.');
    const domain = path.length === 1 ? 'stoud' : path[0];
    const family = path.length === 1 ? path[0] : path[1];
    const listStart = lineText.indexOf(':') + 1;
    const item = itemAtListPosition(compact[2], listStart, position.character);
    return virtualFacadeImportDefinition(domain, family, item, catalog);
  }

  const facade = lineText.match(/^\s*use\s+([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)+)\s*\{\s*([^}]*)\s*\}\s*;?/);
  if (facade) {
    const path = facade[1].split('.');
    const domain = path[0];
    const family = path.slice(1).join('.');
    const listStart = lineText.indexOf('{') + 1;
    const item = itemAtListPosition(facade[2], listStart, position.character);
    return virtualFacadeImportDefinition(domain, family, item, catalog);
  }

  return null;
}

function projectImportItemsInPrefix(lineText, character) {
  const prefix = (lineText || '').slice(0, Math.max(0, character));
  const match = prefix.match(/^\s*use\s*<\s*project\s*:([^>]*)$/)
    || prefix.match(/^\s*use\s+project\s*\{([^}]*)$/);
  if (!match) {
    return new Set();
  }
  return new Set(match[1].split(',').map((item) => item.trim()).filter(Boolean));
}

function snippetFromInsertText(text, fallbackLabel) {
  const insert = text || `${fallbackLabel}($1)`;
  if (insert.includes('${') || insert.includes('$1')) {
    return new vscode.SnippetString(insert);
  }
  if (!insert.includes('(')) {
    return new vscode.SnippetString(`${insert}($1)`);
  }
  return insert;
}

function lowLevelCompletionKind(kind) {
  if (kind === 'snippet') {
    return vscode.CompletionItemKind.Snippet;
  }
  if (kind === 'instruction') {
    return vscode.CompletionItemKind.Operator;
  }
  if (kind === 'register') {
    return vscode.CompletionItemKind.Variable;
  }
  if (kind === 'enum') {
    return vscode.CompletionItemKind.EnumMember;
  }
  return vscode.CompletionItemKind.Keyword;
}

function buildCompletions(registry, document, catalog, position) {
  const pool = core.completionPool(registry, catalog);
  const items = [];
  const seen = new Set();
  const lineText = position ? document.lineAt(position.line).text : '';
  const prefix = position ? lineText.slice(0, position.character) : '';
  const text = document.getText();
  const modeFamily = core.detectModeFamily(document.fileName, text);
  if (['machine-low-level', 'root-basal', 'primitive-low-level', 'hardware-low-level'].includes(modeFamily)) {
    return [];
  }
  const symbols = core.parseSymbols(text);
  const pushUnique = (item) => {
    const key = item.label && item.label.label ? item.label.label : item.label;
    if (key && seen.has(key)) {
      return;
    }
    if (key) {
      seen.add(key);
    }
    items.push(item);
  };
  const currentWord = (prefix.match(/[A-Za-z_][A-Za-z0-9_]*$/) || [''])[0];
  const importRows = core.facadeImportCompletionRows(text, lineText, position ? position.character : 0);

  const lowLevelRows = core.lowLevelCompletionRows(document.fileName, text, lineText);
  if (lowLevelRows.length > 0) {
    for (const row of lowLevelRows) {
      const item = new vscode.CompletionItem(row.label, lowLevelCompletionKind(row.kind));
      item.insertText = row.insertText && (row.insertText.includes('${') || row.insertText.includes('$1'))
        ? new vscode.SnippetString(row.insertText)
        : row.insertText;
      if (item.insertText instanceof vscode.SnippetString) {
        item.insertTextRules = vscode.CompletionItemInsertTextRule.InsertAsSnippet;
      }
      item.detail = row.detail;
      item.sortText = row.sortText;
      item.commitCharacters = row.kind === 'register' ? [',', ']', ';'] : [' ', ';', ':'];
      pushUnique(item);
    }
    return items;
  }

  if (importRows.length > 0 || core.isProjectImportCompletionContext(lineText, position ? position.character : 0)) {
    for (const row of importRows) {
      const item = new vscode.CompletionItem(row.label, row.kind === 'item' ? vscode.CompletionItemKind.Value : vscode.CompletionItemKind.Module);
      item.insertText = row.insertText && row.insertText.includes('${')
        ? new vscode.SnippetString(row.insertText)
        : row.insertText;
      if (item.insertText instanceof vscode.SnippetString) {
        item.insertTextRules = vscode.CompletionItemInsertTextRule.InsertAsSnippet;
      }
      item.detail = row.detail;
      item.sortText = row.sortText;
      pushUnique(item);
    }
    if (core.isProjectImportCompletionContext(lineText, position ? position.character : 0)) {
      const used = projectImportItemsInPrefix(lineText, position ? position.character : 0);
      const model = document && !document.isUntitled && core.isNcLanguageId(document.languageId)
        ? buildProjectModel(document)
        : null;
      const projectRows = model
        ? Array.from(model.nodes.values())
          .filter((node) => (
            node.files.size > 0
            || node.codeFiles.size > 0
            || node.fromBridge
            || node.fromCode
            || node.containers.length > 0
          ))
          .map((node) => ({
            label: node.name,
            insertText: node.name,
            detail: `project module ${node.name}`,
            sortText: `00_project_${node.name}`
          }))
        : core.projectModuleCompletionRowsFromPaths(cachedProjectSourceFiles(document), document.fileName);
      for (const row of projectRows.filter((row) => !used.has(row.label))) {
        const item = new vscode.CompletionItem(row.label, vscode.CompletionItemKind.Module);
        item.insertText = row.insertText;
        item.detail = row.detail;
        item.sortText = row.sortText;
        pushUnique(item);
      }
    }
    return items;
  }

  const fieldRows = core.structFieldCompletionRows(text, lineText, position ? position.character : 0);
  if (fieldRows.length > 0 && /\.\s*[A-Za-z0-9_]*$/.test(prefix)) {
    for (const row of fieldRows) {
      const item = new vscode.CompletionItem(row.label, vscode.CompletionItemKind.Field);
      item.insertText = row.insertText;
      item.detail = row.detail;
      item.sortText = row.sortText;
      item.commitCharacters = [';', ',', ')', ' '];
      pushUnique(item);
    }
    return items;
  }

  const entryRows = core.entrypointCompletionRows(text, lineText, position ? position.character : 0);
  if (entryRows.length > 0 && /^\s*ref(?:\s+main)?(?:\s*->>)?\s*$/.test(prefix)) {
    for (const row of entryRows) {
      const item = new vscode.CompletionItem(row.label, vscode.CompletionItemKind.Reference);
      item.insertText = new vscode.SnippetString(row.insertText);
      item.insertTextRules = vscode.CompletionItemInsertTextRule.InsertAsSnippet;
      item.detail = row.detail;
      item.sortText = row.sortText;
      pushUnique(item);
    }
    return items;
  }

  const focusedPrefix = currentWord.length > 0 || /[@.]$/.test(prefix.trim());
  const showLargeCatalog = currentWord.length >= 2 || /[@.]$/.test(prefix.trim());

  for (const kw of pool.keywords) {
    pushUnique(new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword));
  }

  for (const variable of symbols.variables) {
    const item = new vscode.CompletionItem(variable.name, vscode.CompletionItemKind.Variable);
    item.insertText = variable.name;
    item.detail = variable.typeName ? `local ${variable.typeName}` : 'local symbol';
    item.sortText = `05_local_${variable.name}`;
    pushUnique(item);
  }

  for (const fn of symbols.functions) {
    const item = new vscode.CompletionItem(fn.name, vscode.CompletionItemKind.Function);
    item.insertText = new vscode.SnippetString(`${fn.name}($1)`);
    item.insertTextRules = vscode.CompletionItemInsertTextRule.InsertAsSnippet;
    item.detail = 'local function prediction';
    item.sortText = `06_fn_${fn.name}`;
    pushUnique(item);

    if (/ref\s+main\s*->>\s*$/.test(prefix)) {
      const refItem = new vscode.CompletionItem(fn.name, vscode.CompletionItemKind.Reference);
      refItem.insertText = fn.name;
      refItem.detail = 'entrypoint prediction from local functions';
      refItem.sortText = `01_entry_${fn.name}`;
      pushUnique(refItem);
    }
  }

  for (const row of fieldRows) {
    const item = new vscode.CompletionItem(row.label, vscode.CompletionItemKind.Field);
    item.insertText = row.insertText;
    item.detail = row.detail;
    item.sortText = row.sortText;
    item.commitCharacters = [';', ',', ')', ' '];
    pushUnique(item);
  }

  for (const row of entryRows) {
    const item = new vscode.CompletionItem(row.label, vscode.CompletionItemKind.Reference);
    item.insertText = new vscode.SnippetString(row.insertText);
    item.insertTextRules = vscode.CompletionItemInsertTextRule.InsertAsSnippet;
    item.detail = row.detail;
    item.sortText = row.sortText;
    pushUnique(item);
  }

  for (const row of core.importedFunctionCompletionRows(text, catalog)) {
    const item = new vscode.CompletionItem(row.label, vscode.CompletionItemKind.Function);
    item.insertText = snippetFromInsertText(row.insertText, row.label);
    if (item.insertText instanceof vscode.SnippetString) {
      item.insertTextRules = vscode.CompletionItemInsertTextRule.InsertAsSnippet;
    }
    item.detail = row.signature || row.detail;
    item.documentation = new vscode.MarkdownString(`${row.detail}\n\nImported by \`${row.facadeImport || 'visible use statement'}\`.`);
    item.sortText = `01_imported_fn_${row.label}`;
    item.commitCharacters = ['(', ';'];
    pushUnique(item);
  }

  if (showLargeCatalog) {
    for (const row of core.missingImportSuggestions(text, catalog)) {
      const statement = core.preferredImportStatement(row);
      const item = new vscode.CompletionItem(statement, vscode.CompletionItemKind.Module);
      item.insertText = statement;
      item.detail = `predicted import for ${row.label || row.symbol}`;
      item.sortText = `00_import_${row.label || row.symbol}`;
      pushUnique(item);
    }
  }

  if (showLargeCatalog) {
    for (const prediction of core.nccCommandPredictions(document.fileName)) {
      const item = new vscode.CompletionItem(prediction.label, vscode.CompletionItemKind.Event);
      item.insertText = prediction.command;
      item.detail = prediction.detail;
      item.documentation = new vscode.MarkdownString(`Terminal prediction:\n\n\`${prediction.command}\``);
      item.sortText = `90_ncc_${prediction.label}`;
      pushUnique(item);
    }
  }

  if (focusedPrefix) {
    for (const t of pool.types) {
      pushUnique(new vscode.CompletionItem(t, vscode.CompletionItemKind.TypeParameter));
    }
  }
  let catalogIndex = 0;
  if (showLargeCatalog) {
    for (const row of pool.catalogSymbols) {
      const item = new vscode.CompletionItem(catalogCompletionLabel(row), completionKindForCatalog(row));
      item.insertText = snippetForCatalog(row);
      if (item.insertText instanceof vscode.SnippetString) {
        item.insertTextRules = vscode.CompletionItemInsertTextRule.InsertAsSnippet;
      }
      decorateCatalogCompletion(item, row, catalogIndex);
      catalogIndex += 1;
      item.additionalTextEdits = missingImportEditForCompletion(document, row);
      pushUnique(item);
    }
    for (const row of pool.catalogTypes) {
      const item = new vscode.CompletionItem(catalogCompletionLabel(row), vscode.CompletionItemKind.TypeParameter);
      item.insertText = snippetForCatalog(row);
      if (item.insertText instanceof vscode.SnippetString) {
        item.insertTextRules = vscode.CompletionItemInsertTextRule.InsertAsSnippet;
      }
      decorateCatalogCompletion(item, row, catalogIndex);
      catalogIndex += 1;
      pushUnique(item);
    }
  }
  if (focusedPrefix) {
    for (const printName of pool.printMembers) {
      const item = new vscode.CompletionItem(printName, vscode.CompletionItemKind.Function);
      item.insertText = printName === 'println' ? 'println($1);' : `${printName}($1);`;
      item.insertTextRules = vscode.CompletionItemInsertTextRule.InsertAsSnippet;
      item.detail = 'print family';
      pushUnique(item);
    }
  }
  if (showLargeCatalog) {
    for (const family of pool.runtimeFamilies) {
      const item = new vscode.CompletionItem(family, vscode.CompletionItemKind.Module);
      item.insertText = family;
      item.detail = 'runtime API family';
      pushUnique(item);
    }
  }
  if (showLargeCatalog) {
    for (const row of core.IMPORT_SUGGESTIONS) {
      if (seen.has(row.symbol)) {
        continue;
      }
      const importStatement = core.preferredImportStatement(row);
      const item = new vscode.CompletionItem(row.symbol, vscode.CompletionItemKind.Function);
      item.detail = `Auto import: ${importStatement}`;
      item.documentation = new vscode.MarkdownString(`${row.detail}\n\nSelecting this completion inserts \`${importStatement}\` when it is missing. The quick fix can also add it later.`);
      item.insertText = new vscode.SnippetString(`${row.symbol}($1)`);
      item.insertTextRules = vscode.CompletionItemInsertTextRule.InsertAsSnippet;
      item.additionalTextEdits = missingImportEditForCompletion(document, row);
      pushUnique(item);
    }
    for (const pointerType of pool.pointerTypes) {
      const item = new vscode.CompletionItem(pointerType, vscode.CompletionItemKind.TypeParameter);
      item.detail = 'pointer/provenance type';
      pushUnique(item);
    }
  }

  const fnSnippet = new vscode.CompletionItem('function declaration', vscode.CompletionItemKind.Snippet);
  fnSnippet.insertText = new vscode.SnippetString('${1:name} :: in(${2}) -> ${3:void} {\n    ${4}\n}');
  pushUnique(fnSnippet);

  const probeSnippet = new vscode.CompletionItem('@probe(...)', vscode.CompletionItemKind.Snippet);
  probeSnippet.insertText = new vscode.SnippetString('@probe(${1:name})');
  pushUnique(probeSnippet);

  const regionSnippet = new vscode.CompletionItem('pRegion raw memRegion descriptor', vscode.CompletionItemKind.Snippet);
  regionSnippet.insertText = new vscode.SnippetString('${1:region} >> memRegion<${2:byte}> <- pRegion(${3:pointer}, ${4:length});');
  regionSnippet.additionalTextEdits = missingImportEditForCompletion(document, {
    legacyImport: 'nC.runtime.region',
    facadeImport: 'use stoud.memory { region };'
  });
  regionSnippet.detail = 'bounded memRegion from pointer';
  pushUnique(regionSnippet);

  const rawBoundarySnippet = new vscode.CompletionItem('raw pointer hazard boundary', vscode.CompletionItemKind.Snippet);
  rawBoundarySnippet.insertText = new vscode.SnippetString([
    'risk:',
    '    allow rawPtr;',
    '    accept rawPtr "${1:reason}";',
    '    evidence boundary;',
    'raw {',
    '    ${2}',
    '}'
  ].join('\n'));
  rawBoundarySnippet.detail = 'raw pointer authority with auditable hazard boundary';
  pushUnique(rawBoundarySnippet);

  return items;
}

function buildInlayHints(document, range) {
  const modeFamily = core.detectModeFamily(document.fileName, document.getText());
  if (['machine-low-level', 'root-basal', 'primitive-low-level', 'hardware-low-level'].includes(modeFamily)) {
    return [];
  }
  const hints = core.inlayHintRows(document.getText(), {
    parameters: config().get('inlayHints.parameters') !== false,
    types: config().get('inlayHints.types') !== false
  });
  const out = [];
  const startLine = range ? range.start.line : 0;
  const endLine = range ? range.end.line : Math.max(0, document.lineCount - 1);
  for (const row of hints) {
    if (row.line < startLine || row.line > endLine) {
      continue;
    }
    const pos = new vscode.Position(row.line, Math.max(0, row.character || 0));
    const kind = row.kind === 'type'
      ? vscode.InlayHintKind.Type
      : vscode.InlayHintKind.Parameter;
    const hint = new vscode.InlayHint(pos, row.label, kind);
    if (row.tooltip) {
      hint.tooltip = row.tooltip;
    }
    out.push(hint);
  }
  return out;
}

function parseCompilerTextDiagnostics(document, raw, source) {
  const rows = core.compilerTextDiagnostics(raw);
  return rows.map((row) => {
    const line = Math.max(0, row.line || 0);
    const character = Math.max(0, row.character || 0);
    const length = Math.max(1, row.length || 1);
    const d = new vscode.Diagnostic(
      new vscode.Range(line, character, line, character + length),
      row.message || raw || 'nC compiler diagnostic',
      severityFromString(row.severity)
    );
    d.code = row.code || undefined;
    d.source = source || 'ncc';
    return d;
  });
}

function buildCodeLenses(document) {
  if (!document || document.isUntitled || !core.isNcLanguageId(document.languageId)) {
    return [];
  }
  const range = new vscode.Range(0, 0, 0, 0);
  if (isTerminalOnlyMachineSource(document.fileName)) {
    return [
      new vscode.CodeLens(range, {
        title: 'Check',
        command: 'nc.check',
        arguments: [document.uri]
      })
    ];
  }
  return [
    new vscode.CodeLens(range, {
      title: 'Run',
      command: 'nc.run',
      arguments: [document.uri]
    }),
    new vscode.CodeLens(range, {
      title: 'Debug',
      command: 'nc.debug',
      arguments: [document.uri]
    }),
    new vscode.CodeLens(range, {
      title: 'Check',
      command: 'nc.check',
      arguments: [document.uri]
    })
  ];
}

function lineIsBlank(document, line) {
  return line >= 0 && line < document.lineCount && document.lineAt(line).text.trim() === '';
}

function importInsertionLine(document) {
  let lastImport = -1;
  for (let i = 0; i < document.lineCount; i += 1) {
    const line = document.lineAt(i).text.trim();
    if (line === '') {
      continue;
    }
    if (line.startsWith('use ')) {
      lastImport = i;
      continue;
    }
    break;
  }
  return lastImport >= 0 ? lastImport + 1 : 0;
}

function importInsertionPosition(document) {
  return new vscode.Position(importInsertionLine(document), 0);
}

function importInsertText(document, statement) {
  const line = importInsertionLine(document);
  const needsBlankAfter = !lineIsBlank(document, line);
  return `${statement}\n${needsBlankAfter ? '\n' : ''}`;
}

function mergedImportText(document, merge) {
  const nextLine = merge.line + 1;
  const nextIsImport = nextLine < document.lineCount && lineLooksLikeImport(document, nextLine);
  const needsBlankAfter = !nextIsImport && !lineIsBlank(document, nextLine);
  return `${merge.newLine}${needsBlankAfter ? '\n' : ''}`;
}

function importTextEdits(document, statement) {
  const merge = core.mergeFacadeImportStatement(document.getText(), statement);
  if (merge && merge.changed) {
    return [
      vscode.TextEdit.replace(
        new vscode.Range(merge.line, 0, merge.line, merge.lineText.length),
        mergedImportText(document, merge)
      )
    ];
  }
  return [
    vscode.TextEdit.insert(
      importInsertionPosition(document),
      importInsertText(document, statement)
    )
  ];
}

function autoImportEnabled() {
  return config().get('autoImportOnTokenComplete') !== false;
}

function autoImportAllowedForDocument(document, text) {
  if (!document || document.languageId !== 'nc' || document.isUntitled) {
    return false;
  }
  const family = core.detectModeFamily(document.fileName, text);
  return ![
    'root-basal',
    'machine-low-level',
    'primitive-low-level',
    'kernel-level'
  ].includes(family);
}

function lineLooksLikeImport(document, line) {
  if (!document || line < 0 || line >= document.lineCount) {
    return false;
  }
  const text = document.lineAt(line).text.trim();
  return text.startsWith('use ');
}

async function maybeAutoImportOnTokenComplete(document, change, catalog, inFlight) {
  if (!autoImportEnabled() || !document || !change || !core.isTokenBoundaryText(change.text)) {
    return;
  }
  const key = document.uri.toString();
  if (inFlight.has(key) || lineLooksLikeImport(document, change.range.start.line)) {
    return;
  }
  const text = document.getText();
  if (!autoImportAllowedForDocument(document, text)) {
    return;
  }
  const offset = document.offsetAt(change.range.start) + change.text.length;
  const row = core.autoImportCandidateAtOffset(text, offset, catalog);
  if (!row) {
    return;
  }
  const edits = importTextEdits(document, core.preferredImportStatement(row));
  if (edits.length === 0) {
    return;
  }
  const workspaceEdit = new vscode.WorkspaceEdit();
  for (const edit of edits) {
    workspaceEdit.replace(document.uri, edit.range, edit.newText);
  }
  inFlight.add(key);
  try {
    await vscode.workspace.applyEdit(workspaceEdit);
  } finally {
    setTimeout(() => inFlight.delete(key), 50);
  }
}

function createMissingImportAction(document, row) {
  const statement = core.preferredImportStatement(row);
  const action = new vscode.CodeAction(
    `Add import: ${statement}`,
    vscode.CodeActionKind.QuickFix
  );
  const edit = new vscode.WorkspaceEdit();
  for (const textEdit of importTextEdits(document, statement)) {
    if (textEdit.newText === `${statement}\n`) {
      edit.insert(document.uri, textEdit.range.start, textEdit.newText);
    } else {
      edit.replace(document.uri, textEdit.range, textEdit.newText);
    }
  }
  action.edit = edit;
  action.isPreferred = true;
  return action;
}

function allNavigableSymbols(symbols) {
  return [
    ...(symbols.functions || []),
    ...(symbols.variables || []),
    ...(symbols.parameters || []),
    ...(symbols.containers || [])
  ];
}

function codeOccurrenceAtPosition(document, name, position) {
  return core.collectCodeOccurrences(document.getText(), name).some((row) => {
    if (row.line !== position.line) {
      return false;
    }
    return position.character >= row.character && position.character <= row.character + row.length;
  });
}

function symbolDefinitionRow(document, name) {
  const symbols = core.parseSymbols(document.getText());
  return allNavigableSymbols(symbols).find((row) => row.name === name) || null;
}

function safeRenameOccurrences(document, name) {
  const text = document.getText();
  const lines = text.split(/\r?\n/);
  return core.collectCodeOccurrences(text, name).filter((row) => {
    const line = lines[row.line] || '';
    if (/^\s*use\b/.test(line)) {
      return false;
    }
    const before = row.character > 0 ? line[row.character - 1] : '';
    return before !== '.';
  });
}

function provideDefinition(document, position, catalog) {
  const importHit = importDefinitionAtPosition(document, position, catalog);
  if (importHit) {
    return importHit;
  }
  const lineText = document.lineAt(position.line).text;
  const word = core.findWordRangeInLine(lineText, position.character);
  if (!word) {
    return null;
  }
  const name = core.normalizeWord(word.text);
  if (!name || core.KEYWORDS.includes(name) || !codeOccurrenceAtPosition(document, name, position)) {
    return null;
  }
  const hit = symbolDefinitionRow(document, name);
  if (hit) {
    return new vscode.Location(document.uri, new vscode.Position(hit.line, hit.character));
  }
  return null;
}

function provideReferences(document, position) {
  const lineText = document.lineAt(position.line).text;
  const word = core.findWordRangeInLine(lineText, position.character);
  if (!word) {
    return [];
  }
  const name = core.normalizeWord(word.text);
  if (!name || core.KEYWORDS.includes(name) || !codeOccurrenceAtPosition(document, name, position)) {
    return [];
  }
  const out = [];
  const seen = new Set();
  for (const file of sourceFilesForNavigation(document)) {
    let text = '';
    try {
      text = path.resolve(file).toLowerCase() === path.resolve(document.fileName).toLowerCase()
        ? document.getText()
        : fs.readFileSync(file, 'utf8');
    } catch (_err) {
      continue;
    }
    for (const row of core.collectCodeOccurrences(text, name)) {
      const key = `${file}:${row.line}:${row.character}:${row.length}`;
      if (seen.has(key)) {
        continue;
      }
      seen.add(key);
      out.push(new vscode.Location(
        vscode.Uri.file(file),
        new vscode.Range(row.line, row.character, row.line, row.character + row.length)
      ));
    }
  }
  return out;
}

function provideDocumentSymbols(document) {
  const parsed = core.parseSymbols(document.getText());
  const out = [];
  const fullRange = new vscode.Range(0, 0, Math.max(0, document.lineCount - 1), 0);

  for (const container of parsed.containers) {
    const node = new vscode.DocumentSymbol(
      container.name,
      'container',
      vscode.SymbolKind.Namespace,
      fullRange,
      new vscode.Range(container.line, container.character, container.line, container.character + container.name.length)
    );

    const fns = parsed.functions.filter((row) => row.line >= container.line);
    for (const fn of fns) {
      node.children.push(new vscode.DocumentSymbol(
        fn.name,
        'fn',
        vscode.SymbolKind.Function,
        fullRange,
        new vscode.Range(fn.line, fn.character, fn.line, fn.character + fn.name.length)
      ));
    }

    const vars = parsed.variables.filter((row) => row.line >= container.line);
    for (const variable of vars) {
      node.children.push(new vscode.DocumentSymbol(
        variable.name,
        variable.typeName || 'stc',
        vscode.SymbolKind.Variable,
        fullRange,
        new vscode.Range(variable.line, variable.character, variable.line, variable.character + variable.name.length)
      ));
    }

    out.push(node);
  }

  return out;
}

function provideSemanticTokens(document) {
  const builder = new vscode.SemanticTokensBuilder(SEMANTIC_LEGEND);
  const spans = core.semanticTokenSpans(document.getText(), {
    fileName: document.fileName,
    languageId: document.languageId
  });
  for (const span of spans) {
    const idx = SEMANTIC_TOKEN_TYPES.indexOf(span.tokenType);
    if (idx >= 0) {
      let modifierMask = 0;
      if (span.tokenModifier) {
        const modifierIdx = SEMANTIC_TOKEN_MODIFIERS.indexOf(span.tokenModifier);
        if (modifierIdx >= 0) {
          modifierMask = 1 << modifierIdx;
        }
      }
      builder.push(span.line, span.startChar, span.length, idx, modifierMask);
    }
  }
  return builder.build();
}

function renameCurrentFileExtension(targetExt) {
  const editor = vscode.window.activeTextEditor;
  if (!editor || !targetExt) {
    return;
  }
  const uri = editor.document.uri;
  const oldPath = uri.fsPath;
  const nextPath = oldPath.replace(/\.[^.]+$/, targetExt);
  const fileEdit = vscode.WorkspaceEdit();
  fileEdit.renameFile(uri, vscode.Uri.file(nextPath), { overwrite: false, ignoreIfExists: true });
  vscode.workspace.applyEdit(fileEdit);
}

function provideCodeActions(document) {
  const actions = [];
  const text = document.getText();
  const targetExt = core.detectLowLevelRenameTarget(document.fileName, text);
  const currentExt = path.extname(document.fileName).toLowerCase();

  if (targetExt && targetExt !== currentExt) {
    const action = new vscode.CodeAction(
      `Rename file to ${targetExt} (lane migration policy)`,
      vscode.CodeActionKind.QuickFix
    );
    action.command = {
      command: 'nc.renameCurrentFileExtension',
      title: 'Rename extension',
      arguments: [targetExt]
    };
    actions.push(action);
  }

  if (/"[^"]*"\s*\+\s*@\w+/.test(text)) {
    const action = new vscode.CodeAction(
      'Use explicit render for text concatenation (+)',
      vscode.CodeActionKind.QuickFix
    );
    action.command = {
      command: 'nc.openBaseDoc',
      title: 'Open concat docs',
      arguments: ['plus_operator']
    };
    actions.push(action);
  }
  for (const row of core.missingImportSuggestions(text, provideCodeActions.catalog)) {
    actions.push(createMissingImportAction(document, row));
  }

  return actions;
}

function provideRenameEdits(document, position, newName) {
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(newName)) {
    throw new Error('invalid nC symbol name');
  }
  if (core.KEYWORDS.includes(newName)) {
    throw new Error('new name is a reserved keyword');
  }
  if (importDefinitionAtPosition(document, position, null)) {
    throw new Error('imports are not renamed; edit the import item explicitly');
  }
  const lineText = document.lineAt(position.line).text;
  const word = core.findWordRangeInLine(lineText, position.character);
  if (!word) {
    return null;
  }
  const currentName = core.normalizeWord(word.text);
  if (!currentName || core.KEYWORDS.includes(currentName)) {
    throw new Error('symbol cannot be renamed');
  }
  if (!codeOccurrenceAtPosition(document, currentName, position)) {
    throw new Error('rename is only available on code symbols');
  }
  if (!symbolDefinitionRow(document, currentName)) {
    throw new Error('rename requires a local symbol definition');
  }

  const edit = new vscode.WorkspaceEdit();
  const occ = safeRenameOccurrences(document, currentName);
  for (const row of occ) {
    edit.replace(
      document.uri,
      new vscode.Range(row.line, row.character, row.line, row.character + row.length),
      newName
    );
  }
  return edit;
}

function registerTaskProvider(context) {
  const provider = vscode.tasks.registerTaskProvider('nc', {
    provideTasks: () => {
      const tasks = [];
      const defs = [
        { label: 'nC: Run Active File', verb: 'run' },
        { label: 'nC: Check Active File', verb: 'check' },
        { label: 'nC: Debug Active File', verb: 'run', profile: 'debug' },
        { label: 'nC: Format Active File', verb: 'nfmt' }
      ];
      for (const row of defs) {
        const profile = row.profile ? ` --profile ${row.profile} --show-return` : '';
        const commandLine = row.verb === 'nfmt'
          ? `ncc nfmt "${'${file}'}" --compact --in-place`
          : `ncc ${row.verb} "${'${file}'}"${profile}`;
        const exec = new vscode.ShellExecution(commandLine);
        const task = new vscode.Task(
          { type: 'nc', verb: row.profile ? 'debug' : row.verb },
          vscode.TaskScope.Workspace,
          row.label,
          'nc',
          exec,
          []
        );
        tasks.push(task);
      }
      return tasks;
    },
    resolveTask: (task) => task
  });
  context.subscriptions.push(provider);
}

function updateStatusBar(statusBar) {
  const kit = (config().get('kit') || 'auto').toString();
  const toolchain = (config().get('toolchain') || 'auto').toString();
  const profile = core.sanitizeProfile(config().get('profile'));
  const runProfile = configuredRunProfile();
  const target = configuredTarget() || 'host';
  const lspMode = core.sanitizeLspMode(config().get('lspMode'));
  statusBar.text = `nC $(tools) ${kit} | ${toolchain} | edit:${profile} | run:${runProfile} | ${target} | ${lspMode}`;
  statusBar.tooltip = 'nC kit/toolchain/editorial profile/run profile/target/lsp mode';
  statusBar.show();
}

async function openBaseDoc(context, key) {
  const registry = core.loadBaseRegistry(context.extensionPath);
  let rel = null;
  if (key === 'plus_operator') {
    rel = registry.source_docs?.plus_operator;
  } else if (key === 'print_family') {
    rel = registry.source_docs?.print_family;
  } else if (key === 'runtime_api') {
    rel = registry.source_docs?.runtime_api;
  }
  if (!rel) {
    rel = 'docs/base/README.md';
  }
  const root = vscode.workspace.workspaceFolders?.[0]?.uri?.fsPath;
  if (!root) {
    vscode.window.showWarningMessage('Open a workspace folder to navigate base docs.');
    return;
  }
  const target = vscode.Uri.file(path.join(root, rel));
  await vscode.commands.executeCommand('vscode.open', target);
}

async function activate(context) {
  const diagnostics = vscode.languages.createDiagnosticCollection('nc');
  context.subscriptions.push(diagnostics);
  registerNcDebugAdapter(context);
  registerVirtualDefinitionProvider(context);

  const registry = core.loadBaseRegistry(context.extensionPath);
  const catalog = core.loadIntellisenseCatalog(context.extensionPath);
  provideCodeActions.catalog = catalog;
  const debounce = new Map();
  const diagnosticEpoch = new Map();
  const diagnosticsInFlight = new Map();
  const autoImportInFlight = new Set();

  const statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
  context.subscriptions.push(statusBar);
  updateStatusBar(statusBar);

  const refreshDiagnostics = async (document, epoch, signal) => {
    if (!document || !core.isNcLanguageId(document.languageId) || document.isUntitled) {
      return;
    }
    const key = document.uri.toString();
    const isCurrent = () => !signal.aborted && diagnosticEpoch.get(key) === epoch;
    try {
      const route = await routeDocument(context, document, signal);
      if (!isCurrent()) {
        return;
      }
      const routed = commandFromRoute(route);
      const res = await execCaptureAsync(routed.cmd, routed.args, null, signal);
      if (!isCurrent()) {
        return;
      }
      if (res.code !== 0) {
        const rawDiagnostics = res.stderr || res.stdout || '';
        const parsedTextDiagnostics = parseCompilerTextDiagnostics(document, rawDiagnostics, route.backend || 'ncc');
        const routedErrors = parsedTextDiagnostics.length > 0
          ? parsedTextDiagnostics
          : [
            new vscode.Diagnostic(
              new vscode.Range(0, 0, 0, 1),
              rawDiagnostics || `nC diagnostics failed with code ${res.code}`,
              vscode.DiagnosticSeverity.Error
            )
          ];
        if (config().get('editorAssistHints')) {
          routedErrors.push(...parseEditorAssistDiagnostics(document, catalog));
        }
        if (isCurrent()) {
          diagnostics.set(document.uri, routedErrors);
        }
        return;
      }

      const payload = JSON.parse(res.stdout || '{}');
      let parsed = [];
      if (route.transport === 'broker-nide') {
        parsed = parseNideDiagnostics(payload, document);
      } else if (route.backend === 'nclsp') {
        parsed = parseNclspDiagnostics(payload, document);
      } else {
        parsed = parseLowLevelDiagnostics(payload, document);
      }
      if (config().get('editorAssistHints')) {
        parsed.push(...parseEditorAssistDiagnostics(document, catalog));
      }
      if (isCurrent()) {
        diagnostics.set(document.uri, parsed);
      }
    } catch (err) {
      if ((err && err.code === 'NC_DIAGNOSTIC_ABORTED') || !isCurrent()) {
        return;
      }
      const fallback = [
        new vscode.Diagnostic(
          new vscode.Range(0, 0, 0, 1),
          `nC diagnostics routing failure: ${err.message || String(err)}`,
          vscode.DiagnosticSeverity.Error
        )
      ];
      if (config().get('editorAssistHints')) {
        fallback.push(...parseEditorAssistDiagnostics(document, catalog));
      }
      diagnostics.set(document.uri, fallback);
    }
  };

  const scheduleDiagnostics = (document) => {
    if (!document || !core.isNcLanguageId(document.languageId)) {
      return;
    }
    const key = document.uri.toString();
    const epoch = (diagnosticEpoch.get(key) || 0) + 1;
    diagnosticEpoch.set(key, epoch);
    const prev = debounce.get(key);
    if (prev) {
      clearTimeout(prev);
    }
    const active = diagnosticsInFlight.get(key);
    if (active) {
      active.abort();
      diagnosticsInFlight.delete(key);
    }
    const timer = setTimeout(() => {
      debounce.delete(key);
      const controller = new AbortController();
      diagnosticsInFlight.set(key, controller);
      refreshDiagnostics(document, epoch, controller.signal).finally(() => {
        if (diagnosticsInFlight.get(key) === controller) {
          diagnosticsInFlight.delete(key);
        }
      });
    }, 250);
    debounce.set(key, timer);
  };

  context.subscriptions.push(vscode.workspace.onDidSaveTextDocument((doc) => scheduleDiagnostics(doc)));
  context.subscriptions.push(vscode.workspace.onDidOpenTextDocument((doc) => scheduleDiagnostics(doc)));
  context.subscriptions.push(vscode.workspace.onDidChangeTextDocument((ev) => {
    for (const change of ev.contentChanges || []) {
      maybeAutoImportOnTokenComplete(ev.document, change, catalog, autoImportInFlight);
    }
    scheduleDiagnostics(ev.document);
  }));
  context.subscriptions.push(vscode.workspace.onDidCloseTextDocument((doc) => {
    const key = doc.uri.toString();
    const timer = debounce.get(key);
    if (timer) {
      clearTimeout(timer);
      debounce.delete(key);
    }
    const active = diagnosticsInFlight.get(key);
    if (active) {
      active.abort();
      diagnosticsInFlight.delete(key);
    }
    diagnosticEpoch.delete(key);
    diagnostics.delete(doc.uri);
  }));
  context.subscriptions.push({
    dispose() {
      for (const timer of debounce.values()) {
        clearTimeout(timer);
      }
      for (const active of diagnosticsInFlight.values()) {
        active.abort();
      }
      debounce.clear();
      diagnosticsInFlight.clear();
      diagnosticEpoch.clear();
    }
  });

  context.subscriptions.push(vscode.workspace.onDidChangeConfiguration((ev) => {
    if (ev.affectsConfiguration('nc')) {
      updateStatusBar(statusBar);
      const active = vscode.window.activeTextEditor?.document;
      if (active && core.isNcLanguageId(active.languageId)) {
        scheduleDiagnostics(active);
      }
    }
  }));

  context.subscriptions.push(vscode.commands.registerCommand('nc.openBaseDoc', (key) => openBaseDoc(context, key)));
  context.subscriptions.push(vscode.commands.registerCommand('nc.renameCurrentFileExtension', (targetExt) => renameCurrentFileExtension(targetExt)));

  context.subscriptions.push(
    vscode.commands.registerCommand('nc.run', async (target) => {
      const fileName = commandTargetFileName(target);
      if (!fileName) {
        return;
      }
      if (isTerminalOnlyMachineSource(fileName)) {
        showMachineTerminalOnlyMessage();
        return;
      }
      const resolved = resolveNccForExecution(fileName);
      const argv = runArgsForFile('run', fileName);
      runCommandForFile('nC Run', resolved, argv, fileName, lowLevelRunsOutsideIde(fileName));
    })
  );
  context.subscriptions.push(
    vscode.commands.registerCommand('nc.runLegacy', async () => vscode.commands.executeCommand('nc.run'))
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('nc.debug', async (target) => {
      const fileName = commandTargetFileName(target);
      if (!fileName) {
        return;
      }
      if (isTerminalOnlyMachineSource(fileName)) {
        showMachineTerminalOnlyMessage();
        return;
      }
      const resolved = resolveNccForExecution(fileName);
      if (await startDebugSessionForFile(fileName, resolved)) {
        return;
      }
      const argv = debugArgsForFile(fileName);
      runCommandForFile('nC Debug', resolved, argv, fileName, lowLevelRunsOutsideIde(fileName));
    })
  );
  context.subscriptions.push(
    vscode.commands.registerCommand('nc.debugLegacy', async () => vscode.commands.executeCommand('nc.debug'))
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('nc.check', async (target) => {
      const fileName = commandTargetFileName(target);
      if (!fileName) {
        return;
      }
      const resolved = resolveNccPath({ allowWsl: true });
      const result = runFileCommand(resolved, 'check', fileName);
      if (result.code === 0) {
        vscode.window.showInformationMessage('nC check: OK');
      } else {
        vscode.window.showErrorMessage(result.stderr || result.stdout || 'nC check failed');
      }
    })
  );
  context.subscriptions.push(
    vscode.commands.registerCommand('nc.checkLegacy', async () => vscode.commands.executeCommand('nc.check'))
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('nc.format', async (target) => {
      const editor = vscode.window.activeTextEditor;
      const fileName = commandTargetFileName(target);
      if (!fileName) {
        return;
      }
      if (editor && editor.document.fileName === fileName && editor.document.isDirty) {
        await editor.document.save();
      }
      const resolved = resolveNccPath({ allowWsl: true });
      const result = runFormatter(resolved, fileName);
      if (result.code === 0) {
        vscode.window.showInformationMessage('nC format: OK');
      } else {
        vscode.window.showErrorMessage(result.stderr || result.stdout || 'nC format failed');
      }
    })
  );
  context.subscriptions.push(
    vscode.commands.registerCommand('nc.formatLegacy', async () => vscode.commands.executeCommand('nc.format'))
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('nc.selectToolchain', async () => {
      try {
        await selectToolchain();
        updateStatusBar(statusBar);
      } catch (err) {
        vscode.window.showErrorMessage(`nC Select Toolchain failed: ${err.message || String(err)}`);
      }
    })
  );
  context.subscriptions.push(
    vscode.commands.registerCommand('nc.selectKit', async () => {
      try {
        await selectNccKit();
        updateStatusBar(statusBar);
      } catch (err) {
        vscode.window.showErrorMessage(`nC Select Kit failed: ${err.message || String(err)}`);
      }
    })
  );
  context.subscriptions.push(
    vscode.commands.registerCommand('nc.createKit', async () => {
      try {
        await createNccKit();
        updateStatusBar(statusBar);
      } catch (err) {
        vscode.window.showErrorMessage(`nC Create Kit failed: ${err.message || String(err)}`);
      }
    })
  );

  const activeFile = () => vscode.window.activeTextEditor?.document?.fileName;
  const output = vscode.window.createOutputChannel('nC Commands');
  context.subscriptions.push(output);

  const runWithActiveFileInTerminal = (title, subcommandPrefix) => {
    const file = activeFile();
    if (!file) {
      return;
    }
    runNccSubcommandTerminal(title, [...subcommandPrefix, file]);
  };

  const runSimpleCapture = (title, args) => {
    const result = runNccSubcommandCapture(args);
    output.clear();
    output.appendLine(`$ ncc ${args.join(' ')}`);
    if (result.stdout) {
      output.appendLine(result.stdout);
    }
    if (result.stderr) {
      output.appendLine(result.stderr);
    }
    output.show(true);
    if (result.code !== 0) {
      vscode.window.showErrorMessage(`${title}: failed`);
    }
  };

  context.subscriptions.push(vscode.commands.registerCommand('nc.showCompilerDiagnostics', () => runSimpleCapture('Compiler Diagnostics', ['compiler', 'diagnostics', '--json'])));
  context.subscriptions.push(vscode.commands.registerCommand('nc.showCompilerPipeline', () => runWithActiveFileInTerminal('nC Compiler Pipeline', ['compiler', 'pipeline'])));
  context.subscriptions.push(vscode.commands.registerCommand('nc.showCompilerAst', () => runWithActiveFileInTerminal('nC Compiler AST', ['compiler', 'ast'])));
  context.subscriptions.push(vscode.commands.registerCommand('nc.showCompilerIr', () => runWithActiveFileInTerminal('nC Compiler IR', ['compiler', 'nir'])));
  context.subscriptions.push(vscode.commands.registerCommand('nc.showProjectGraph', () => showProjectGraph()));
  context.subscriptions.push(vscode.commands.registerCommand('nc.showRuntimeFamilies', () => runSimpleCapture('Runtime Families', ['runtime', 'families', '--json'])));
  context.subscriptions.push(vscode.commands.registerCommand('nc.showRuntimeExports', async () => {
    const family = await vscode.window.showInputBox({
      prompt: 'Runtime family (e.g. nC.runtime.input)',
      value: 'nC.runtime.input'
    });
    if (!family) {
      return;
    }
    runSimpleCapture('Runtime Exports', ['runtime', 'exports', family, '--json']);
  }));
  context.subscriptions.push(vscode.commands.registerCommand('nc.showRuntimeCapabilities', () => runSimpleCapture('Runtime Capabilities', ['runtime', 'capabilities', '--json'])));
  context.subscriptions.push(vscode.commands.registerCommand('nc.showRuntimePolicies', () => runSimpleCapture('Runtime Policies', ['runtime', 'policies', '--json'])));

  context.subscriptions.push(vscode.commands.registerCommand('nc.primitiveDoctor', () => runSimpleCapture('Primitive Doctor', ['primitive', 'doctor', '--json'])));
  context.subscriptions.push(vscode.commands.registerCommand('nc.primitiveCheck', () => runWithActiveFileInTerminal('nC Primitive Check', ['primitive', 'check'])));
  context.subscriptions.push(vscode.commands.registerCommand('nc.primitiveWitness', () => runWithActiveFileInTerminal('nC Primitive Witness', ['primitive', 'witness', '--json'])));
  context.subscriptions.push(vscode.commands.registerCommand('nc.primitiveMap', () => runWithActiveFileInTerminal('nC Primitive Map', ['primitive', 'map', '--json'])));
  context.subscriptions.push(vscode.commands.registerCommand('nc.primitiveLower', () => runWithActiveFileInTerminal('nC Primitive Lower', ['primitive', 'lower', '--json'])));
  context.subscriptions.push(vscode.commands.registerCommand('nc.primitiveQemu', () => runWithActiveFileInTerminal('nC Primitive QEMU', ['primitive', 'qemu'])));

  const selector = [
    { language: 'nc' },
    { language: 'nc-root' },
    { language: 'nc-machine' },
    { language: 'nc-primitive' },
    { language: 'nc-hardware' }
  ];
  const assistedSourceSelector = [{ language: 'nc' }];

  context.subscriptions.push(vscode.languages.registerHoverProvider(selector, {
    provideHover(document, position) {
      return buildHover(document, position, registry, catalog);
    }
  }));

  context.subscriptions.push(vscode.languages.registerCompletionItemProvider(assistedSourceSelector, {
    provideCompletionItems(document, position) {
      return buildCompletions(registry, document, catalog, position);
    }
  }, ...COMPLETION_TRIGGER_CHARS));

  context.subscriptions.push(vscode.languages.registerInlayHintsProvider(assistedSourceSelector, {
    provideInlayHints(document, range) {
      return buildInlayHints(document, range);
    }
  }));

  context.subscriptions.push(vscode.languages.registerCodeLensProvider(selector, {
    provideCodeLenses(document) {
      return buildCodeLenses(document);
    }
  }));

  context.subscriptions.push(vscode.languages.registerDefinitionProvider(selector, {
    provideDefinition(document, position) {
      return provideDefinition(document, position, catalog);
    }
  }));

  context.subscriptions.push(vscode.languages.registerReferenceProvider(selector, {
    provideReferences(document, position) {
      return provideReferences(document, position);
    }
  }));

  context.subscriptions.push(vscode.languages.registerDocumentSymbolProvider(selector, {
    provideDocumentSymbols(document) {
      return provideDocumentSymbols(document);
    }
  }));

  context.subscriptions.push(vscode.languages.registerCodeActionsProvider(selector, {
    provideCodeActions(document) {
      return provideCodeActions(document);
    }
  }, {
    providedCodeActionKinds: [vscode.CodeActionKind.QuickFix]
  }));

  context.subscriptions.push(vscode.languages.registerDocumentFormattingEditProvider({ language: 'nc' }, {
    provideDocumentFormattingEdits(document) {
      return provideDocumentFormattingEdits(document);
    }
  }));

  context.subscriptions.push(vscode.languages.registerOnTypeFormattingEditProvider(selector, {
    provideOnTypeFormattingEdits(document, position, ch) {
      return provideOnTypeFormattingEdits(document, position, ch);
    }
  }, '}'));

  context.subscriptions.push(vscode.languages.registerRenameProvider(selector, {
    provideRenameEdits(document, position, newName) {
      return provideRenameEdits(document, position, newName);
    }
  }));

  context.subscriptions.push(vscode.languages.registerDocumentSemanticTokensProvider(selector, {
    provideDocumentSemanticTokens(document) {
      return provideSemanticTokens(document);
    }
  }, SEMANTIC_LEGEND));

  registerTaskProvider(context);

  const active = vscode.window.activeTextEditor?.document;
  if (active && core.isNcLanguageId(active.languageId)) {
    scheduleDiagnostics(active);
  }
}

function deactivate() {
  // no-op
}

module.exports = {
  activate,
  deactivate
};
