const fs = require('fs');
const path = require('path');

const KEYWORDS = [
  'if', 'else', 'while', 'for', 'match', 'guard', 'unless', 'return', 'dev', 'skip', 'stop',
  'break', 'continue', 'use', 'to', 'ref', 'main', 'final', 'fn', 'in', 'type', 'const', 'local',
  'public', 'internal', 'private',
  'program', 'runtime', 'network', 'system', 'kernel', 'tooling', 'compiler', 'security',
  'inspect', 'ai', 'machine', 'primitive', 'basal', 'static', 'stack', 'abi',
  'using', 'load', 'store', 'copy', 'fill', 'bytes', 'addr', 'size', 'align', 'offset',
  'volatile', 'atomic', 'section', 'weak', 'common', 'comdat', 'tls', 'percpu', 'stc', 'fa'
];

const TYPES = [
  'int', 'u64', 'byte', 'bool', 'str', 'text', 'char', 'ft32', 'ft64', 'addr', 'word', 'dword', 'qword',
  'sbyte', 'sword', 'sdword', 'sqword', 'void', 'null', 'p', 'ptr', 'ref', 'site', 'memRegion',
  'slice', 'span', 'bytes', 'array', 'list', 'queue', 'vector', 'Result', 'Option', 'xy', 'xyz', 'matrix',
  'tensor', 'dinArray', 'dinArray.binary', 'dinArray.unbinary'
];

const POINTER_TYPES = ['addr', 'p', 'p mut', 'ptr', 'ptr mut', 'ref', 'ref mut', 'site', 'memRegion', 'slice', 'span'];

const OFFICIAL_SECTIONS = ['const', 'type', 'stc', 'local', 'fn', 'fa'];

const IMPORT_SUGGESTIONS = [
  { symbol: 'readInt', importPath: 'nC.runtime.input', detail: 'Read an integer from standard input.' },
  { symbol: 'readLine', importPath: 'nC.runtime.input', detail: 'Read a text line from standard input.' },
  { symbol: 'readBool', importPath: 'nC.runtime.input', detail: 'Read a boolean value from standard input.' },
  { symbol: 'print', importPath: 'nC.runtime.output', detail: 'Write human output.' },
  { symbol: 'println', importPath: 'nC.runtime.output', detail: 'Write human output with newline.' },
  { symbol: 'color', importPath: 'nC.runtime.output', detail: 'Render text with an ANSI foreground color.' },
  { symbol: 'colorBold', importPath: 'nC.runtime.output', detail: 'Render text with bold ANSI color.' },
  { symbol: 'renderInt', importPath: 'nC.runtime.render', detail: 'Render an integer explicitly for text output.' },
  { symbol: 'renderBool', importPath: 'nC.runtime.render', detail: 'Render a boolean explicitly for text output.' },
  { symbol: 'renderBinary', importPath: 'nC.runtime.render', facadeImport: 'use <io:render>;', detail: 'Render an integer as exact binary text.' },
  { symbol: 'renderBin', importPath: 'nC.runtime.render', facadeImport: 'use <io:render>;', detail: 'Compact alias for renderBinary.' },
  { symbol: 'renderHex', importPath: 'nC.runtime.render', detail: 'Render a number as hexadecimal text.' },
  { symbol: 'alloc', importPath: 'nC.runtime.heap', facadeImport: 'use <memory:heap>;', detail: 'Allocate typed heap storage.' },
  { symbol: 'free', importPath: 'nC.runtime.heap', facadeImport: 'use <memory:heap>;', detail: 'Release typed heap storage.' },
  { symbol: 'realloc', importPath: 'nC.runtime.heap', facadeImport: 'use <memory:heap>;', detail: 'Resize typed heap storage.' },
  { symbol: 'map', importPath: 'nC.runtime.memory', facadeImport: 'use <memory:map>;', detail: 'Map raw bytes into process memory.' },
  { symbol: 'unmap', importPath: 'nC.runtime.memory', facadeImport: 'use <memory:map>;', detail: 'Release a mapped memory range.' },
  { symbol: 'remap', importPath: 'nC.runtime.memory', facadeImport: 'use <memory:map>;', detail: 'Resize a mapped memory range.' },
  { symbol: 'Region', importPath: 'nC.runtime.region', detail: 'Region/memRegion support.' },
  { symbol: 'pRegion', importPath: 'nC.runtime.region', detail: 'Build a raw memRegion<T> descriptor from pointer and length.' },
  { symbol: 'regionCreate', importPath: 'nC.runtime.region', detail: 'Create a runtime memory region with explicit element type and bounds.' },
  { symbol: 'listAsRegion', importPath: 'nC.runtime.region', detail: 'Expose a list as a bounded memRegion<T> view.' },
  { symbol: 'pAt', importPath: 'nC.runtime.region', detail: 'Compute a raw pointer at a region index.' },
  { symbol: 'pNull', importPath: 'nC.runtime.ptr', detail: 'Compare a raw pointer with null.' },
  { symbol: 'pAddr', importPath: 'nC.runtime.ptr', detail: 'Extract the machine address from a pointer.' },
  { symbol: 'pFromAddr', importPath: 'nC.runtime.ptr', detail: 'Construct p<T> from a machine address.' },
  { symbol: 'pCast', importPath: 'nC.runtime.ptr', detail: 'Reinterpret a pointer as another pointee type.' },
  { symbol: 'pRead', importPath: 'nC.runtime.ptr', detail: 'Load through a raw pointer.' },
  { symbol: 'pWrite', importPath: 'nC.runtime.ptr', detail: 'Store through a raw pointer.' },
  { symbol: 'pOffset', importPath: 'nC.runtime.ptr', detail: 'Raw pointer arithmetic by sizeof(T).' },
  { symbol: 'pDiff', importPath: 'nC.runtime.ptr', detail: 'Element distance between two raw pointers.' },
  { symbol: 'addrOf', importPath: 'nC.runtime.addr', detail: 'Address/provenance helper.' },
  { symbol: 'spanOf', importPath: 'nC.runtime.span', detail: 'Create a bounded span.' },
  { symbol: 'sliceOf', importPath: 'nC.runtime.slice', detail: 'Create a bounded slice.' }
];

const FACADE_IMPORT_FAMILIES = [
  {
    label: 'use <io:output, render, input>;',
    insertText: 'use <io:output, render, input>;',
    domain: 'stoud',
    family: 'io',
    items: ['input', 'output', 'errorOutput', 'std', 'render', 'concat', 'flush', 'random', 'range', 'int', 'float', 'bool', 'seed', 'entropy', 'uuid'],
    detail: 'Input, output and explicit render helpers.'
  },
  {
    label: 'use <memory:ptr, heap, region, map>;',
    insertText: 'use <memory:ptr, heap, region, map>;',
    domain: 'stoud',
    family: 'memory',
    items: ['ptr', 'region', 'stack', 'heap', 'map', 'alloc', 'buffer', 'bytes', 'layout', 'align', 'endian', 'slice', 'span'],
    detail: 'Pointers, heap allocation, bounded regions and native memory mappings.'
  },
  {
    label: 'use <collections:list, queue, vector, span, slice>;',
    insertText: 'use <collections:list, queue, vector, span, slice>;',
    domain: 'stoud',
    family: 'collections',
    items: ['array', 'list', 'queue', 'vector', 'span', 'slice', 'option', 'result', 'error'],
    detail: 'Lists, queues, vectors and bounded collection views.'
  },
  {
    label: 'use <data:json>;',
    insertText: 'use <data:json>;',
    domain: 'stoud',
    family: 'data',
    items: ['json'],
    detail: 'Typed JSON parsing, construction, mutation, files and diagnostics.'
  },
  {
    label: 'use <math:core, trig, numeric>;',
    insertText: 'use <math:core, trig, numeric>;',
    domain: 'stoud',
    family: 'math',
    items: ['core', 'algebra', 'trig', 'geometry', 'vector', 'matrix', 'tensor', 'stats', 'prob', 'calc', 'numeric', 'complex', 'discrete', 'sign'],
    detail: 'Arithmetic, trigonometry, numeric, vector and complex math surfaces.'
  },
  {
    label: 'use <time:clock, timer>;',
    insertText: 'use <time:clock, timer>;',
    domain: 'stoud',
    family: 'time',
    items: ['time', 'clock', 'sleep', 'timer'],
    detail: 'Wall-clock, monotonic-clock, sleep and timer helpers.'
  },
  {
    label: 'use <fs:read, write, path>;',
    insertText: 'use <fs:read, write, path>;',
    domain: 'stoud',
    family: 'fs',
    items: ['fs', 'read', 'write', 'append', 'sync', 'exists', 'delete', 'copy', 'mkdir', 'rmdir', 'list', 'metadata', 'permissions', 'path', 'dir', 'file', 'stream'],
    detail: 'Filesystem, directory, path, file and stream helpers.'
  },
  {
    label: 'use <net:tcp, udp, dns, http, tls>;',
    insertText: 'use <net:tcp, udp, dns, http, tls>;',
    domain: 'stoud',
    family: 'net',
    items: ['net', 'socket', 'tcp', 'udp', 'http', 'dns', 'tls'],
    detail: 'Networking helpers for sockets, TCP, UDP, DNS, HTTP and TLS.'
  },
  {
    label: 'use <process:spawn, wait, kill>;',
    insertText: 'use <process:spawn, wait, kill>;',
    domain: 'stoud',
    family: 'process',
    items: ['process', 'args', 'env', 'exit', 'session', 'command', 'user', 'temp', 'service', 'signal', 'spawn', 'wait', 'kill', 'stdio', 'status'],
    detail: 'Process spawning, waiting, status and stdio helpers.'
  },
  {
    label: 'use <thread:spawn, join>;',
    insertText: 'use <thread:spawn, join>;',
    domain: 'stoud',
    family: 'thread',
    items: ['thread', 'spawn', 'join', 'sleep', 'local', 'sync', 'lock', 'atomic', 'channel'],
    detail: 'Thread spawning, joining, sleeping, local storage and synchronization helpers.'
  },
  {
    label: 'use <task:async, scheduler, cancel>;',
    insertText: 'use <task:async, scheduler, cancel>;',
    domain: 'stoud',
    family: 'task',
    items: ['task', 'thread', 'async', 'scheduler', 'cancel', 'event'],
    detail: 'Explicit task, async state-machine, scheduler and cancellation surfaces.'
  },
  {
    label: 'use <text:string, format, parse>;',
    insertText: 'use <text:string, format, parse>;',
    domain: 'stoud',
    family: 'text',
    items: ['text', 'string', 'unicode', 'format', 'parse', 'convert', 'escape', 'pad', 'width', 'wrap', 'truncate'],
    detail: 'Explicit text storage, conversion, parsing and Unicode helpers.'
  },
  {
    label: 'use <sync:atomic, lock, channel>;',
    insertText: 'use <sync:atomic, lock, channel>;',
    domain: 'stoud',
    family: 'sync',
    items: ['sync', 'lock', 'atomic', 'channel'],
    detail: 'Hosted synchronization, locks, atomics and channels.'
  },
  {
    label: 'use <audit:diagnostic, trace, debug>;',
    insertText: 'use <audit:diagnostic, trace, debug>;',
    domain: 'stoud',
    family: 'audit',
    items: ['debug', 'diagnostic', 'log', 'trace', 'status', 'assert', 'validate', 'profile', 'bridge'],
    detail: 'Diagnostics, traces, reports and explicit audit helpers.'
  },
  {
    label: 'use <terminal:terminal>;',
    insertText: 'use <terminal:terminal>;',
    domain: 'stoud',
    family: 'terminal',
    items: ['terminal', 'clear', 'moveCursor', 'hideCursor', 'showCursor', 'enterRawMode', 'exitRawMode', 'readKeyNonBlocking'],
    detail: 'Interactive terminal state, cursor and raw input operations.'
  },
  {
    label: 'use <mobile:telephony, tts_engine, permissions>;',
    insertText: 'use <mobile:telephony, tts_engine, permissions>;',
    domain: 'stoud',
    family: 'mobile',
    items: ['telephony', 'tts_engine', 'network', 'permissions'],
    detail: 'Mobile telephony, speech and permission providers.'
  },
  {
    label: 'use <network:server, client, send_bytes, receive_bytes>;',
    insertText: 'use <network:server, client, send_bytes, receive_bytes>;',
    domain: 'stoud',
    family: 'network',
    items: ['network', 'server', 'client', 'send_bytes', 'receive_bytes', 'server_open', 'isWiFiConnected'],
    detail: 'Mobile and embedded network service primitives.'
  },
  {
    label: 'use <sys:permissions, get_current_time, sleep>;',
    insertText: 'use <sys:permissions, get_current_time, sleep>;',
    domain: 'stoud',
    family: 'sys',
    items: ['permissions', 'get_current_time', 'cpu_load', 'registry_write', 'registry_read', 'sleep'],
    detail: 'Hosted system permissions, clocks, registry and load state.'
  },
  {
    label: 'use <os:os, platform, openUri>;',
    insertText: 'use <os:os, platform, openUri>;',
    domain: 'stoud',
    family: 'os',
    items: ['os', 'platform', 'openUri', 'media', 'mediaNext', 'mediaPlay', 'mediaPause', 'mediaPrevious'],
    detail: 'Hosted operating-system and media integration.'
  },
  {
    label: 'use <ui:window, canvas, input>;',
    insertText: 'use <ui:window, canvas, input>;',
    domain: 'stoud',
    family: 'ui',
    items: ['window', 'canvas', 'input'],
    detail: 'Window, canvas and interactive UI surfaces.'
  },
  {
    label: 'use <core.types:option, result>;',
    insertText: 'use <core.types:option, result>;',
    domain: 'core',
    family: 'types',
    items: ['types', 'option', 'result', 'error'],
    detail: 'Freestanding type contracts without hosted runtime dependencies.'
  },
  {
    label: 'use <core.memory:ptr, region, layout>;',
    insertText: 'use <core.memory:ptr, region, layout>;',
    domain: 'core',
    family: 'memory',
    items: ['ptr', 'region', 'bytes', 'layout', 'align', 'endian', 'slice', 'span', 'addr', 'volatile'],
    detail: 'Freestanding raw memory, layout and volatile operations.'
  },
  {
    label: 'use <core.sync:atomic>;',
    insertText: 'use <core.sync:atomic>;',
    domain: 'core',
    family: 'sync',
    items: ['atomic', 'fence', 'barrier', 'wait', 'notify'],
    detail: 'Freestanding atomic and memory-order primitives.'
  },
  {
    label: 'use <alloc.heap:alloc, heap, buffer>;',
    insertText: 'use <alloc.heap:alloc, heap, buffer>;',
    domain: 'alloc',
    family: 'heap',
    items: ['alloc', 'heap', 'buffer'],
    detail: 'Optional allocator-backed heap storage.'
  },
  {
    label: 'use <alloc.collections:list, queue, vector>;',
    insertText: 'use <alloc.collections:list, queue, vector>;',
    domain: 'alloc',
    family: 'collections',
    items: ['array', 'vector', 'list', 'queue'],
    detail: 'Allocator-backed collections with explicit storage contracts.'
  },
  {
    label: 'use <alloc.string:string>;',
    insertText: 'use <alloc.string:string>;',
    domain: 'alloc',
    family: 'string',
    items: ['string'],
    detail: 'Allocator-backed string storage.'
  },
  {
    label: 'use <auth.system:process, thread>;',
    insertText: 'use <auth.system:process, thread>;',
    domain: 'auth',
    family: 'system',
    items: ['file', 'dir', 'path', 'env', 'process', 'thread', 'permission', 'service', 'signal', 'session', 'media', 'os'],
    detail: 'Explicit authority-bearing system capabilities.'
  },
  {
    label: 'use <auth.security:capability, policy, sandbox>;',
    insertText: 'use <auth.security:capability, policy, sandbox>;',
    domain: 'auth',
    family: 'security',
    items: ['audit', 'capability', 'policy', 'sandbox'],
    detail: 'Explicit security authority and policy surfaces.'
  },
  {
    label: 'use <auth.resource:resource, handle, session>;',
    insertText: 'use <auth.resource:resource, handle, session>;',
    domain: 'auth',
    family: 'resource',
    items: ['resource', 'handle', 'session'],
    detail: 'Authority-bearing resource handles and sessions.'
  },
  {
    label: 'use <dom.compiler:parse, sema, diagnostics>;',
    insertText: 'use <dom.compiler:parse, sema, diagnostics>;',
    domain: 'dom',
    family: 'compiler',
    items: ['parse', 'sema', 'diagnostics'],
    detail: 'Compiler-domain parsing, semantic and diagnostic surfaces.'
  },
  {
    label: 'use <dom.high:typeinfo, consteval, macros>;',
    insertText: 'use <dom.high:typeinfo, consteval, macros>;',
    domain: 'dom',
    family: 'high',
    items: ['typeinfo', 'metadata', 'consteval', 'comptime', 'macros'],
    detail: 'High-level compile-time and metadata facilities.'
  },
  {
    label: 'use <dom.inspect:site, trace, report>;',
    insertText: 'use <dom.inspect:site, trace, report>;',
    domain: 'dom',
    family: 'inspect',
    items: ['site', 'metadata', 'trace', 'report', 'evidence'],
    detail: 'Inspection, trace, report and evidence facilities.'
  },
  {
    label: 'use <dom.ai:agent, model, policy>;',
    insertText: 'use <dom.ai:agent, model, policy>;',
    domain: 'dom',
    family: 'ai',
    items: ['agent', 'eval', 'memory', 'model', 'policy'],
    detail: 'Explicit AI-domain agents, models and evaluation facilities.'
  },
  {
    label: 'use <dom.tooling:build, fmt, lsp, package>;',
    insertText: 'use <dom.tooling:build, fmt, lsp, package>;',
    domain: 'dom',
    family: 'tooling',
    items: ['build', 'fmt', 'lint', 'lsp', 'package'],
    detail: 'Compiler tooling, formatting, language server and package facilities.'
  },
  {
    label: 'use project { module };',
    insertText: 'use project { ${1:module} };',
    domain: 'project',
    family: 'project',
    items: ['module'],
    detail: 'Import child modules from the current project.'
  }
];

const POINTER_BASE = [
  { symbol: 'addr', title: 'addr', detail: 'Raw machine address. Arithmetic, aliasing and invalid addresses are accepted; validity belongs to the program.' },
  { symbol: 'p', title: 'p<T>', detail: 'Typed raw address. It carries element layout only; null, aliasing, bounds, lifetime and validity remain unchecked.' },
  { symbol: 'p mut', title: 'p mut<T>', detail: 'Compact mutable raw pointer. It adds no implicit bounds, lifetime, ownership or alias checks.' },
  { symbol: 'ptr', title: 'ptr<T>', detail: 'Migrator spelling for p<T>. Normal Material V2 source uses p<T>; both lower to the same unchecked address bits during migration.' },
  { symbol: 'ref', title: 'ref<T>', detail: 'Explicit address-shaped contract. It performs no hidden lifetime, alias or validity enforcement in manual/raw execution.' },
  { symbol: 'ref mut', title: 'ref mut<T>', detail: 'Mutable address-shaped contract with no hidden borrow checking or cleanup.' },
  { symbol: 'site', title: 'site<T>', detail: 'Address plus optional inspection metadata. The metadata never changes generated code or blocks dangerous access.' },
  { symbol: 'memRegion', title: 'memRegion<T>', detail: 'Explicit data, length and capacity descriptor. Operations remain unchecked unless source calls an audit helper.' },
  { symbol: 'slice', title: 'slice<T>', detail: 'Explicit data and length view. The stored length does not add automatic bounds checks.' },
  { symbol: 'span', title: 'span<T>', detail: 'Explicit contiguous data and length view with no ownership or automatic validation.' }
];

const DIAGNOSTIC_HELP = [
  { code: 'E_PTR_NULL_DEREF_001', detail: 'Nullable pointer/ref used without explicit nullability or refinement.' },
  { code: 'E_PTR_OUT_OF_BOUNDS_001', detail: 'Pointer, span, slice or memRegion access is outside known bounds.' },
  { code: 'E_PTR_STALE_SITE_001', detail: 'Pointer/site provenance became stale after free, invalidation or scope exit.' },
  { code: 'E_PTR_ALIAS_MUT_001', detail: 'Mutable alias, downcast or unsafe projection violates pointer authority.' },
  { code: 'E_PTR_ALIGNMENT_001', detail: 'Load/store/cast requires stricter alignment than the address or pointer provides.' },
  { code: 'E_PTR_PROVENANCE_001', detail: 'Pointer provenance is missing, unknown or uninitialized.' },
  { code: 'E_PTR_FFI_CONTRACT_001', detail: 'FFI pointer crossing lacks required ABI/layout/ownership/nullability contract.' },
  { code: 'E_PTR_RAW_REQUIRED_001', detail: 'Operation requires explicit raw/risk authority or a safer memory surface.' },
  { code: 'E_PTR_TYPE_002', detail: 'Pointer read/write element type does not match the declared pointee type.' },
  { code: 'E_PTR_LOWERING_001', detail: 'Pointer operation cannot lower to the selected machine profile.' },
  { code: 'E_TEXT_PLUS_001', detail: 'Text concatenation must use explicit render helpers in strict profiles.' }
];

const MODE_MARKERS = [
  'program', 'runtime', 'network', 'system', 'kernel', 'tooling', 'compiler',
  'security', 'inspect', 'ai', 'machine', 'primitive', 'basal'
];

const PRINT_MEMBERS = [
  'print', 'println', 'color', 'colorBold', 'bgColor', 'style', 'emoji',
  'progressBar', 'spinnerFrame', 'table2', 'table3'
];

const POINTER_FUNCTION_RE = /\b(?:rd|wr|off|dist|nil|adr|from|asptr|pRead|pWrite|pOffset|pDiff|pNull|pRegion|pAt|pFromAddr|pCast|pAddr|ptrRead|ptrWrite|ptrOffset|ptrDiff|ptrIsNull|regionFromPtr|regionPtrAt|ptrFromAddr|ptrCast|ptrAddr)(?=\s*(?:<|\())/g;
const GENERIC_POINTER_FUNCTION_RE = /\b(?:Read|Write)(?=\s*<)/g;
const KERNEL_MMIO_TOKEN_ROWS = [
  { tokenType: 'kernelMmio8', pattern: /(?:\*\s*8|\b(?:load|store)8)(?=\s*\()/g },
  { tokenType: 'kernelMmio16', pattern: /(?:\*\s*16|\b(?:load|store)16)(?=\s*\()/g },
  { tokenType: 'kernelMmio32', pattern: /(?:\*\s*32|\b(?:load|store)32)(?=\s*\()/g },
  { tokenType: 'kernelMmio64', pattern: /(?:\*\s*64|\b(?:load|store)64)(?=\s*\()/g },
  { tokenType: 'kernelMmio128', pattern: /(?:\*\s*128|\b(?:load|store)128)(?=\s*\()/g },
  { tokenType: 'kernelMmio256', pattern: /(?:\*\s*256|\b(?:load|store)256)(?=\s*\()/g },
  { tokenType: 'kernelMmio512', pattern: /(?:\*\s*512|\b(?:load|store)512)(?=\s*\()/g }
];
const COLOR_FUNCTION_RE = /\b(?:color|colorBold|bgColor|style)(?=\s*\()/g;
const CONTROL_FLOW_RE = /\b(?:if|elseIf|else|while|for|loop|same|guard|unless|match)\b/g;
const LOOP_CONTROL_RE = /\b(?:break|continue)\b/g;

const BACKEND_PRELUDE_IMPORTS = new Set([
  'nC.runtime.string',
  'nC.runtime.terminal',
  'nC.runtime.time'
]);

const BACKEND_PRELUDE_SYMBOLS = new Set([
  'strEmpty', 'strRelease', 'strLen', 'len', 'strByteLen', 'strConcat', 'strSlice',
  'strSubstring', 'substring', 'repeatChar', 'charCode', 'charToStr', 'strCompare',
  'strEq', 'strContains', 'strIndexOf', 'strFromInt', 'strFromBool', 'strFromFt64',
  'strStartsWith', 'strEndsWith', 'strToUpper', 'strToLower',
  'now', 'nowMs', 'timestamp', 'date', 'sleep', 'monotonicMs', 'monotonicNs',
  'sleepUntilMs', 'formatTime', 'exists', 'listDir'
]);

const VALID_STRING_ESCAPES = new Set(['n', 't', '&', '?', '#', '"', "'", '\\']);

const CALL_SIGNATURES = [
  { name: 'print', signature: 'print(value: any) -> void', params: ['value'], detail: 'Write human output without adding a newline.', importHint: 'use <io:output>;' },
  { name: 'println', signature: 'println(value: any) -> void', params: ['value'], detail: 'Write human output with a newline.', importHint: 'use <io:output>;' },
  { name: 'color', signature: 'color(text: str, color: str) -> str', params: ['text', 'color'], detail: 'Render text with an ANSI foreground color.', importHint: 'use <io:output, render>;' },
  { name: 'colorBold', signature: 'colorBold(text: str, color: str) -> str', params: ['text', 'color'], detail: 'Render text with bold ANSI color.', importHint: 'use <io:output, render>;' },
  { name: 'readInt', signature: 'readInt() -> int', params: [], detail: 'Read an integer from standard input.', importHint: 'use <io:input>;' },
  { name: 'readLine', signature: 'readLine() -> str', params: [], detail: 'Read a text line from standard input.', importHint: 'use <io:input>;' },
  { name: 'readBool', signature: 'readBool() -> bool', params: [], detail: 'Read a boolean value from standard input.', importHint: 'use <io:input>;' },
  { name: 'dbgRegs', signature: 'dbgRegs(source: addr, output: addr, slots: int) -> int', params: ['source', 'output', 'slots'], detail: 'Copy a raw register snapshot into caller-owned memory.' },
  { name: 'dbgDump', signature: 'dbgDump(source: addr, output: addr, bytes: int) -> int', params: ['source', 'output', 'bytes'], detail: 'Copy raw memory into a caller-owned dump buffer.' },
  { name: 'dbgTrace', signature: 'dbgTrace(frame: addr, output: addr, maximum: int) -> int', params: ['frame', 'output', 'maximum'], detail: 'Walk frame pointers and write return addresses.' },
  { name: 'dbgSym', signature: 'dbgSym(table: addr, count: int, pc: addr, output: addr) -> bool', params: ['table', 'count', 'pc', 'output'], detail: 'Resolve an address through a caller-owned symbol table.' },
  { name: 'rspSum', signature: 'rspSum(data: addr, bytes: int) -> int', params: ['data', 'bytes'], detail: 'Compute the GDB RSP checksum for raw bytes.' },
  { name: 'rspFrame', signature: 'rspFrame(data: addr, bytes: int, output: addr) -> int', params: ['data', 'bytes', 'output'], detail: 'Frame raw bytes as a GDB RSP packet.' },
  { name: 'brkSet', signature: 'brkSet(code: addr, saved: addr) -> int', params: ['code', 'saved'], detail: 'Install a software breakpoint and retain the replaced opcode.' },
  { name: 'brkDel', signature: 'brkDel(code: addr, saved: addr, width: int) -> bool', params: ['code', 'saved', 'width'], detail: 'Restore the opcode replaced by a software breakpoint.' },
  { name: 'crashWrite', signature: 'crashWrite(report: addr, code: int, pc: addr, sp: addr, fault: addr, flags: int) -> bool', params: ['report', 'code', 'pc', 'sp', 'fault', 'flags'], detail: 'Write a fixed raw crash record into caller-owned memory.' },
  { name: 'kaslr', signature: 'kaslr(image: addr, slide: int) -> addr', params: ['image', 'slide'], detail: 'Apply an explicit raw relocation slide to an address.' },
  { name: 'canarySet', signature: 'canarySet(slot: addr, value: int) -> bool', params: ['slot', 'value'], detail: 'Write a stack canary into a caller-selected slot.' },
  { name: 'canaryOk', signature: 'canaryOk(slot: addr, expected: int) -> bool', params: ['slot', 'expected'], detail: 'Compare a caller-selected canary slot with an expected value.' },
  { name: 'sanPoison', signature: 'sanPoison(shadow: addr, index: int, tag: int) -> bool', params: ['shadow', 'index', 'tag'], detail: 'Write a poison tag into caller-owned shadow memory.' },
  { name: 'sanClear', signature: 'sanClear(shadow: addr, index: int) -> bool', params: ['shadow', 'index'], detail: 'Clear a tag in caller-owned shadow memory.' },
  { name: 'sanOk', signature: 'sanOk(shadow: addr, index: int) -> bool', params: ['shadow', 'index'], detail: 'Check whether a caller-owned shadow byte is clear.' },
  { name: 'renderInt', signature: 'renderInt(value: int) -> str', params: ['value'], detail: 'Render an integer explicitly for text output.', importHint: 'use <io:render>;' },
  { name: 'renderBool', signature: 'renderBool(value: bool) -> str', params: ['value'], detail: 'Render a boolean explicitly for text output.', importHint: 'use <io:render>;' },
  { name: 'renderBinary', signature: 'renderBinary(value: int) -> str', params: ['value'], detail: 'Render an integer as exact binary text.', importHint: 'use <io:render>;' },
  { name: 'renderBin', signature: 'renderBin(value: int) -> str', params: ['value'], detail: 'Compact alias for renderBinary.', importHint: 'use <io:render>;' },
  { name: 'renderHex', signature: 'renderHex(value: int) -> str', params: ['value'], detail: 'Render an integer as hexadecimal text.', importHint: 'use <io:render>;' },
  { name: 'strEmpty', signature: 'strEmpty() -> str', params: [], detail: 'Create an empty string value.', importHint: 'use <text:string>;' },
  { name: 'strLen', signature: 'strLen(text: str) -> int', params: ['text'], detail: 'Count logical characters in a string.', importHint: 'use <text:string>;' },
  { name: 'strByteLen', signature: 'strByteLen(text: str) -> int', params: ['text'], detail: 'Count UTF-8 bytes in a string.', importHint: 'use <text:string>;' },
  { name: 'strConcat', signature: 'strConcat(left: str, right: str) -> str', params: ['left', 'right'], detail: 'Concatenate two strings.', importHint: 'use <text:string>;' },
  { name: 'strSubstring', signature: 'strSubstring(text: str, start: int, len: int) -> str', params: ['text', 'start', 'len'], detail: 'Extract a substring by start and length.', importHint: 'use <text:string>;' },
  { name: 'strCompare', signature: 'strCompare(left: str, right: str) -> int', params: ['left', 'right'], detail: 'Compare two strings; returns -1, 0 or 1.', importHint: 'use <text:string>;' },
  { name: 'strContains', signature: 'strContains(text: str, needle: str) -> bool', params: ['text', 'needle'], detail: 'Check whether text contains a substring.', importHint: 'use <text:string>;' },
  { name: 'strIndexOf', signature: 'strIndexOf(text: str, needle: str) -> int', params: ['text', 'needle'], detail: 'Return substring index or -1.', importHint: 'use <text:string>;' },
  { name: 'strFromInt', signature: 'strFromInt(value: int) -> str', params: ['value'], detail: 'Convert an integer to text.', importHint: 'use <text:string>;' },
  { name: 'strFromBool', signature: 'strFromBool(value: bool) -> str', params: ['value'], detail: 'Convert a boolean to text.', importHint: 'use <text:string>;' },
  { name: 'strFromFt64', signature: 'strFromFt64(value: ft64) -> str', params: ['value'], detail: 'Convert a floating-point value to text.', importHint: 'use <text:string>;' },
  { name: 'abs', signature: 'abs(value: int|ft64) -> int|ft64', params: ['value'], detail: 'Return the absolute value.', importHint: 'use <math:core>;' },
  { name: 'min', signature: 'min(a: int|ft64, b: int|ft64) -> int|ft64', params: ['a', 'b'], detail: 'Return the smaller numeric value.', importHint: 'use <math:core>;' },
  { name: 'max', signature: 'max(a: int|ft64, b: int|ft64) -> int|ft64', params: ['a', 'b'], detail: 'Return the larger numeric value.', importHint: 'use <math:core>;' },
  { name: 'sqrt', signature: 'sqrt(value: ft64) -> ft64', params: ['value'], detail: 'Return the square root.', importHint: 'use <math:numeric>;' },
  { name: 'pow', signature: 'pow(base: ft64, exponent: ft64) -> ft64', params: ['base', 'exponent'], detail: 'Raise a number to a power.', importHint: 'use <math:numeric>;' },
  { name: 'sin', signature: 'sin(value: ft64) -> ft64', params: ['value'], detail: 'Return sine in radians.', importHint: 'use <math:trig>;' },
  { name: 'cos', signature: 'cos(value: ft64) -> ft64', params: ['value'], detail: 'Return cosine in radians.', importHint: 'use <math:trig>;' },
  { name: 'now', signature: 'now() -> int', params: [], detail: 'Read the current wall-clock timestamp.', importHint: 'use <time:time>;' },
  { name: 'monotonic', signature: 'monotonic() -> int', params: [], detail: 'Read a monotonic clock value.', importHint: 'use <time:clock>;' },
  { name: 'sleep', signature: 'sleep(ms: int) -> void', params: ['ms'], detail: 'Pause the current execution for milliseconds.', importHint: 'use <time:sleep>;' },
  { name: 'readFile', signature: 'readFile(path: str) -> Result<str, FsError>', params: ['path'], detail: 'Read a text file.', importHint: 'use <fs:read>;' },
  { name: 'writeFile', signature: 'writeFile(path: str, content: str) -> Result<bool, FsError>', params: ['path', 'content'], detail: 'Write a text file.', importHint: 'use <fs:write>;' },
  { name: 'exists', signature: 'exists(path: str) -> bool', params: ['path'], detail: 'Check whether a path exists.', importHint: 'use <fs:exists>;' },
  { name: 'listDir', signature: 'listDir(path: str) -> Result<str, FsError>', params: ['path'], detail: 'List a directory.', importHint: 'use <fs:list>;' },
  { name: 'tcpConnect', signature: 'tcpConnect(host: str, port: int) -> Result<TcpHandle, TcpError>', params: ['host', 'port'], detail: 'Open a TCP connection.', importHint: 'use <net:tcp>;' },
  { name: 'spawn', signature: 'spawn(command: str, args: list<str>) -> Result<ProcessHandle, ProcessError>', params: ['command', 'args'], detail: 'Spawn a process.', importHint: 'use <process:spawn>;' },
  { name: 'wait', signature: 'wait(process: ProcessHandle) -> Result<int, ProcessError>', params: ['process'], detail: 'Wait for a process to finish.', importHint: 'use <process:wait>;' },
  { name: 'threadSpawn', signature: 'threadSpawn(fn: fn) -> Result<ThreadHandle, ThreadError>', params: ['fn'], detail: 'Spawn a thread from a callable.', importHint: 'use <thread:spawn>;' },
  { name: 'threadJoin', signature: 'threadJoin(thread: ThreadHandle) -> Result<any, ThreadError>', params: ['thread'], detail: 'Join a thread and retrieve its result.', importHint: 'use <thread:join>;' },
  { name: 'alloc', signature: 'alloc<T>(count: int) -> p<T>', params: ['count'], detail: 'Allocate raw heap storage for count elements. No initialization, bounds or lifetime checks are implied.', importHint: 'use <memory:heap>;' },
  { name: 'free', signature: 'free<T>(pointer: p<T>) -> void', params: ['pointer'], detail: 'Release raw heap storage. Aliases remain raw values and may become dangling.', importHint: 'use <memory:heap>;' },
  { name: 'realloc', signature: 'realloc<T>(pointer: p<T>, count: int) -> p<T>', params: ['pointer', 'count'], detail: 'Resize raw heap storage and return its resulting pointer.', importHint: 'use <memory:heap>;' },
  { name: 'map', signature: 'map(bytes: int) -> Result<p<byte>, MemoryError>', params: ['bytes'], detail: 'Create a read-write anonymous memory mapping.', importHint: 'use <memory:map>;' },
  { name: 'mapAnonymous', signature: 'mapAnonymous(bytes: int, protection: int, flags: int) -> Result<p<byte>, MemoryError>', params: ['bytes', 'protection', 'flags'], detail: 'Create an anonymous mapping with explicit protection and flags.', importHint: 'use <memory:map>;' },
  { name: 'mapAt', signature: 'mapAt(address: addr, bytes: int, protection: int, flags: int) -> Result<p<byte>, MemoryError>', params: ['address', 'bytes', 'protection', 'flags'], detail: 'Request a mapping at an explicit address.', importHint: 'use <memory:map>;' },
  { name: 'mapShared', signature: 'mapShared(bytes: int, protection: int) -> Result<p<byte>, MemoryError>', params: ['bytes', 'protection'], detail: 'Create shared mapped memory.', importHint: 'use <memory:map>;' },
  { name: 'mapPrivate', signature: 'mapPrivate(bytes: int, protection: int) -> Result<p<byte>, MemoryError>', params: ['bytes', 'protection'], detail: 'Create private mapped memory.', importHint: 'use <memory:map>;' },
  { name: 'mapFile', signature: 'mapFile(path: str, bytes: int, offset: int, protection: int, flags: int) -> Result<p<byte>, MemoryError>', params: ['path', 'bytes', 'offset', 'protection', 'flags'], detail: 'Map a file range with explicit policy.', importHint: 'use <memory:map>;' },
  { name: 'mapFileShared', signature: 'mapFileShared(path: str, bytes: int, offset: int, protection: int) -> Result<p<byte>, MemoryError>', params: ['path', 'bytes', 'offset', 'protection'], detail: 'Create a shared file mapping.', importHint: 'use <memory:map>;' },
  { name: 'mapFilePrivate', signature: 'mapFilePrivate(path: str, bytes: int, offset: int, protection: int) -> Result<p<byte>, MemoryError>', params: ['path', 'bytes', 'offset', 'protection'], detail: 'Create a private file mapping.', importHint: 'use <memory:map>;' },
  { name: 'unmap', signature: 'unmap(pointer: p<byte>) -> Result<bool, MemoryError>', params: ['pointer'], detail: 'Release a mapped range.', importHint: 'use <memory:map>;' },
  { name: 'remap', signature: 'remap(pointer: p<byte>, bytes: int, mayMove: bool) -> Result<p<byte>, MemoryError>', params: ['pointer', 'bytes', 'mayMove'], detail: 'Resize a mapping and optionally move it.', importHint: 'use <memory:map>;' },
  { name: 'remapAt', signature: 'remapAt(pointer: p<byte>, bytes: int, address: addr) -> Result<p<byte>, MemoryError>', params: ['pointer', 'bytes', 'address'], detail: 'Resize and move a mapping to an explicit address.', importHint: 'use <memory:map>;' },
  { name: 'mapProtect', signature: 'mapProtect(pointer: p<byte>, protection: int) -> Result<bool, MemoryError>', params: ['pointer', 'protection'], detail: 'Change mapped-memory protection.', importHint: 'use <memory:map>;' },
  { name: 'mapSync', signature: 'mapSync(pointer: p<byte>, flags: int) -> Result<bool, MemoryError>', params: ['pointer', 'flags'], detail: 'Synchronize mapped bytes with their backing store.', importHint: 'use <memory:map>;' },
  { name: 'mapAdvise', signature: 'mapAdvise(pointer: p<byte>, advice: int) -> Result<bool, MemoryError>', params: ['pointer', 'advice'], detail: 'Provide an access-pattern hint for a mapping.', importHint: 'use <memory:map>;' },
  { name: 'mapLock', signature: 'mapLock(pointer: p<byte>) -> Result<bool, MemoryError>', params: ['pointer'], detail: 'Lock mapped pages in physical memory.', importHint: 'use <memory:map>;' },
  { name: 'mapUnlock', signature: 'mapUnlock(pointer: p<byte>) -> Result<bool, MemoryError>', params: ['pointer'], detail: 'Unlock mapped pages.', importHint: 'use <memory:map>;' },
  { name: 'mapLen', signature: 'mapLen(pointer: p<byte>) -> int', params: ['pointer'], detail: 'Return the current mapping length in bytes.', importHint: 'use <memory:map>;' },
  { name: 'mapAddress', signature: 'mapAddress(pointer: p<byte>) -> p<byte>', params: ['pointer'], detail: 'Return the base pointer of a mapping.', importHint: 'use <memory:map>;' },
  { name: 'mapIsMapped', signature: 'mapIsMapped(pointer: p<byte>) -> bool', params: ['pointer'], detail: 'Report whether a mapping is still live.', importHint: 'use <memory:map>;' },
  { name: 'listCreate', signature: 'listCreate<T>() -> list<T>', params: [], detail: 'Create a typed runtime list.', importHint: 'use <collections:list>;' },
  { name: 'listAdd', signature: 'listAdd<T>(list: list<T>, value: T) -> void', params: ['list', 'value'], detail: 'Append a value to a typed runtime list.', importHint: 'use <collections:list>;' },
  { name: 'listGet', signature: 'listGet<T>(list: list<T>, index: int) -> T', params: ['list', 'index'], detail: 'Read a typed runtime list element; out-of-bounds returns the typed default.', importHint: 'use <collections:list>;' },
  { name: 'listSet', signature: 'listSet<T>(list: list<T>, index: int, value: T) -> bool', params: ['list', 'index', 'value'], detail: 'Write a typed runtime list element; out-of-bounds returns false.', importHint: 'use <collections:list>;' },
  { name: 'vectorCreate', signature: 'vectorCreate<T>() -> vector<T>', params: [], detail: 'Create a typed runtime vector descriptor.', importHint: 'use <collections:vector>;' },
  { name: 'vectorPush', signature: 'vectorPush<T>(vector: vector<T>, value: T) -> void', params: ['vector', 'value'], detail: 'Append a value to a typed runtime vector.', importHint: 'use <collections:vector>;' },
  { name: 'vectorGet', signature: 'vectorGet<T>(vector: vector<T>, index: int) -> T', params: ['vector', 'index'], detail: 'Read a typed runtime vector element by stride.', importHint: 'use <collections:vector>;' },
  { name: 'vectorSet', signature: 'vectorSet<T>(vector: vector<T>, index: int, value: T) -> void', params: ['vector', 'index', 'value'], detail: 'Write a typed runtime vector element by stride.', importHint: 'use <collections:vector>;' },
  { name: 'vectorLen', signature: 'vectorLen<T>(vector: vector<T>) -> int', params: ['vector'], detail: 'Read the vector descriptor length.', importHint: 'use <collections:vector>;' },
  { name: 'queueCreate', signature: 'queueCreate<T>() -> queue<T>', params: [], detail: 'Create a typed FIFO queue.', importHint: 'use <collections:queue>;' },
  { name: 'queuePush', signature: 'queuePush<T>(queue: queue<T>, value: T) -> bool', params: ['queue', 'value'], detail: 'Push a value into a typed queue tail.', importHint: 'use <collections:queue>;' },
  { name: 'queuePop', signature: 'queuePop<T>(queue: queue<T>) -> Option<T>', params: ['queue'], detail: 'Pop a value from a typed queue head.', importHint: 'use <collections:queue>;' },
  { name: 'queueGet', signature: 'queueGet<T>(queue: queue<T>, index: int) -> T', params: ['queue', 'index'], detail: 'Read a queue element; out-of-bounds returns the typed default.', importHint: 'use <collections:queue>;' },
  { name: 'queueLen', signature: 'queueLen<T>(queue: queue<T>) -> int', params: ['queue'], detail: 'Read the queue descriptor length.', importHint: 'use <collections:queue>;' },
  { name: 'pRegion', signature: 'pRegion<T>(pointer: p<T>, len: int) -> memRegion<T>', params: ['pointer', 'len'], detail: 'Build a region descriptor from a raw pointer and explicit length.', importHint: 'use <memory:region>;' },
  { name: 'regionCreate', signature: 'regionCreate<T>(len: int) -> memRegion<T>', params: ['len'], detail: 'Create a runtime memory region with explicit element type and bounds.', importHint: 'use <memory:region>;' },
  { name: 'pAt', signature: 'pAt<T>(region: memRegion<T>, index: int) -> p<T>', params: ['region', 'index'], detail: 'Compute the raw pointer at a region index.', importHint: 'use <memory:region>;' },
  { name: 'pRead', signature: 'pRead<T>(pointer: p<T>) -> T', params: ['pointer'], detail: 'Load T from a raw pointer. Invalid, unaligned or dangling addresses remain program-defined hazards.', importHint: 'use <memory:ptr>;' },
  { name: 'pWrite', signature: 'pWrite<T>(pointer: p<T>, value: T) -> void', params: ['pointer', 'value'], detail: 'Store T through a raw pointer without implicit bounds or lifetime checks.', importHint: 'use <memory:ptr>;' },
  { name: 'pOffset', signature: 'pOffset<T>(pointer: p<T>, elements: int) -> p<T>', params: ['pointer', 'elements'], detail: 'Offset a pointer by elements of T.', importHint: 'use <memory:ptr>;' },
  { name: 'pDiff', signature: 'pDiff<T>(left: p<T>, right: p<T>) -> int', params: ['left', 'right'], detail: 'Return the raw element distance between two pointers.', importHint: 'use <memory:ptr>;' },
  { name: 'pNull', signature: 'pNull<T>(pointer: p<T>) -> bool', params: ['pointer'], detail: 'Compare a raw pointer with the null address.', importHint: 'use <memory:ptr>;' },
  { name: 'pFromAddr', signature: 'pFromAddr<T>(address: addr) -> p<T>', params: ['address'], detail: 'Construct a typed raw pointer from a machine address.', importHint: 'use <memory:ptr>;' },
  { name: 'pCast', signature: 'pCast<T>(pointer: p<any>) -> p<T>', params: ['pointer'], detail: 'Reinterpret a raw pointer with pointee type T.', importHint: 'use <memory:ptr>;' },
  { name: 'pAddr', signature: 'pAddr<T>(pointer: p<T>) -> addr', params: ['pointer'], detail: 'Extract the machine address carried by a raw pointer.', importHint: 'use <memory:ptr>;' },
  { name: 'rd', signature: 'rd<T>(pointer: p<T>) -> T', params: ['pointer'], detail: 'Compact unchecked raw load. Alias of ptrRead.', importHint: '' },
  { name: 'wr', signature: 'wr<T>(pointer: p<T>, value: T) -> void', params: ['pointer', 'value'], detail: 'Compact unchecked raw store. Alias of ptrWrite.', importHint: '' },
  { name: 'off', signature: 'off<T>(pointer: p<T>, elements: int) -> p<T>', params: ['pointer', 'elements'], detail: 'Compact unchecked pointer arithmetic. Alias of ptrOffset.', importHint: '' },
  { name: 'dist', signature: 'dist<T>(left: p<T>, right: p<T>) -> int', params: ['left', 'right'], detail: 'Compact raw pointer difference. Alias of ptrDiff.', importHint: '' },
  { name: 'nil', signature: 'nil<T>(pointer: p<T>) -> bool', params: ['pointer'], detail: 'Compare a raw pointer with null.', importHint: '' },
  { name: 'from', signature: 'from<T>(address: addr) -> p<T>', params: ['address'], detail: 'Construct a raw pointer from an address.', importHint: '' },
  { name: 'asptr', signature: 'asptr<T>(pointer: p<any>) -> p<T>', params: ['pointer'], detail: 'Reinterpret a pointer without runtime validation.', importHint: '' },
  { name: 'adr', signature: 'adr<T>(pointer: p<T>) -> addr', params: ['pointer'], detail: 'Extract the machine address from a raw pointer.', importHint: '' },
  { name: 'addrOf', signature: 'addrOf<T>(value: ref<T>) -> addr', params: ['value'], detail: 'Extract an address/provenance helper from a value.', importHint: 'use <memory:ptr>;' },
  { name: 'spanOf', signature: 'spanOf<T>(region: memRegion<T>) -> span<T>', params: ['region'], detail: 'Create a bounded span from a region.', importHint: 'use <memory:span>;' },
  { name: 'sliceOf', signature: 'sliceOf<T>(region: memRegion<T>, start: int, len: int) -> slice<T>', params: ['region', 'start', 'len'], detail: 'Create a bounded slice from a region.', importHint: 'use <memory:slice>;' }
];

const NCC_COMMAND_PREDICTIONS = [
  { label: 'ncc run current file', command: 'ncc run "${file}"', detail: 'Run the active nC file through ncc.' },
  { label: 'ncc check current file', command: 'ncc check "${file}"', detail: 'Check parser and semantic diagnostics for the active file.' },
  { label: 'ncc debug current file', command: 'ncc run "${file}" --profile debug --show-return', detail: 'Run the active nC file in debug profile.' },
  { label: 'ncc build-exe current file', command: 'ncc build-exe "${file}" "${fileBasenameNoExtension}.exe"', detail: 'Build an executable from the active nC file.' },
  { label: 'ncc emit-ir current file', command: 'ncc emit-ir "${file}"', detail: 'Emit nC IR for the active file.' },
  { label: 'ncc optimize current file', command: 'ncc optimize "${file}" --json', detail: 'Show optimizer evidence for the active file.' }
];

const TYPE_DEFINITIONS = {
  int: {
    title: 'int',
    detail: 'Signed integer value for whole-number arithmetic, counters, indexes and numeric returns.'
  },
  u64: {
    title: 'u64',
    detail: 'Unsigned 64-bit integer value for non-negative machine-sized quantities.'
  },
  bool: {
    title: 'bool',
    detail: 'Boolean value with only true or false, used by conditions, guards and success/failure returns.'
  },
  str: {
    title: 'str',
    detail: 'Runtime text string used for human-readable text, input lines and output rendering.'
  },
  text: {
    title: 'text',
    detail: 'Textual value surface for source that wants a descriptive text name instead of str.'
  },
  string: {
    title: 'string',
    detail: 'Text string alias/family member used where source prefers the longer textual name.'
  },
  char: {
    title: 'char',
    detail: 'Single character value. Use it for one textual element, not a full string.'
  },
  byte: {
    title: 'byte',
    detail: '8-bit unsigned storage unit for raw bytes, buffers and byte-addressable memory.'
  },
  sbyte: {
    title: 'sbyte',
    detail: '8-bit signed storage unit.'
  },
  word: {
    title: 'word',
    detail: 'Machine-oriented word storage surface.'
  },
  dword: {
    title: 'dword',
    detail: 'Double-word integer/storage width for low-level and machine-facing code.'
  },
  qword: {
    title: 'qword',
    detail: 'Quad-word integer/storage width, commonly used for addresses and 64-bit machine values.'
  },
  addr: {
    title: 'addr',
    detail: 'Raw address value. Provenance, alignment and dereference rules remain explicit.'
  },
  ft32: {
    title: 'ft32',
    detail: '32-bit floating-point numeric value for compact real-number storage and SIMD-friendly data.'
  },
  ft64: {
    title: 'ft64',
    detail: '64-bit floating-point numeric value.'
  },
  array: {
    title: 'array<T, N>',
    detail: 'Fixed-size typed aggregate. The element type and capacity are part of the declared type.'
  },
  list: {
    title: 'list<T>',
    detail: 'Typed dynamic list collection with indexed get/set and append operations.'
  },
  queue: {
    title: 'queue<T>',
    detail: 'Typed first-in-first-out collection for push/pop workflows.'
  },
  vector: {
    title: 'vector<T>',
    detail: 'Typed dynamic aggregate array lowered as descriptor plus backing storage.'
  },
  Result: {
    title: 'Result<T, E>',
    detail: 'Explicit success/error carrier. Use T for the success value and E for recoverable failure data.'
  },
  void: {
    title: 'void',
    detail: 'No returned value.'
  },
  null: {
    title: 'null',
    detail: 'Explicit absence/null value where the type surface permits it.'
  }
};

const PRIMITIVE_ENTITIES = [
  'id', 'module', 'type', 'version', 'opcode', 'encoding', 'relocation', 'abi', 'cycle', 'domain',
  'pin', 'wire', 'width', 'addr', 'register', 'map', 'volatile', 'ready', 'ack', 'trigger',
  'guard', 'irq', 'fault', 'error', 'retry', 'fallback', 'atomic', 'commit', 'reset', 'trace',
  'ptr', 'load', 'store', 'mmio', 'trap', 'fence', 'comb', 'seq', 'assign', 'always',
  'transition', 'drive', 'release', 'mux', 'decode', 'encode', 'pipeline',
  'anchor', 'slot', 'pattern', 'mask', 'window', 'signal', 'bus', 'state', 'timing', 'emit', 'sample', 'latch',
  'assert', 'rupture', 'bridge', 'seal', 'voltage', 'current', 'clock', 'edge', 'latency', 'metastability',
  'noise', 'temperature', 'power', 'thermalState', 'throttleState', 'cacheState', 'tlbState', 'pipelineState',
  'branchPredictorState', 'reorderState', 'outOfOrder', 'powerGate', 'dmaChannel', 'interruptLine', 'watchdog',
  'firmwareRegion', 'bootRom', 'bootRomRegion', 'biosRegion', 'uefiHandoff', 'resetVector', 'bootWindow',
  'bootAnchor', 'firmwareTable', 'deviceTree', 'acpiTable', 'microcode', 'microcodeState', 'isa', 'protocol',
  'controller', 'peripheral', 'phy', 'lane', 'channel', 'registerWindow', 'mmioWindow', 'portWindow',
  'physicalEvent', 'asyncHardwareEvent', 'sideEffect', 'physicalFault', 'physicalFailure', 'physicalViolation'
];

const MACHINE_REGISTERS_BY_ARCH = {
  x86_64: [
    'rax', 'rbx', 'rcx', 'rdx', 'rsi', 'rdi', 'rsp', 'rbp', 'rip', 'rflags',
    'r8', 'r9', 'r10', 'r11', 'r12', 'r13', 'r14', 'r15',
    'eax', 'ebx', 'ecx', 'edx', 'esi', 'edi', 'esp', 'ebp',
    'ax', 'bx', 'cx', 'dx', 'si', 'di', 'sp', 'bp',
    'al', 'bl', 'cl', 'dl', 'ah', 'bh', 'ch', 'dh',
    'xmm0', 'xmm1', 'xmm2', 'xmm3', 'ymm0', 'ymm1', 'ymm2', 'ymm3',
    'zmm0', 'zmm1', 'zmm2', 'zmm3', 'k0', 'k1', 'k2', 'k3',
    'cr0', 'cr2', 'cr3', 'cr4', 'cr8', 'dr0', 'dr1', 'dr2', 'dr3', 'dr6', 'dr7'
  ],
  i386: [
    'eax', 'ebx', 'ecx', 'edx', 'esi', 'edi', 'esp', 'ebp', 'eip', 'eflags',
    'ax', 'bx', 'cx', 'dx', 'si', 'di', 'sp', 'bp', 'al', 'bl', 'cl', 'dl',
    'ah', 'bh', 'ch', 'dh', 'st0', 'st1', 'mm0', 'mm1', 'xmm0', 'xmm1',
    'cr0', 'cr2', 'cr3', 'cr4', 'dr0', 'dr1', 'dr2', 'dr3', 'dr6', 'dr7'
  ],
  aarch64: [
    'x0', 'x1', 'x2', 'x3', 'x4', 'x5', 'x6', 'x7', 'x8', 'x9', 'x10', 'x11',
    'x12', 'x13', 'x14', 'x15', 'x16', 'x17', 'x18', 'x19', 'x20', 'x21', 'x22',
    'x23', 'x24', 'x25', 'x26', 'x27', 'x28', 'x29', 'x30', 'sp', 'lr', 'fp',
    'w0', 'w1', 'w2', 'w3', 'v0', 'v1', 'v2', 'v3', 'z0', 'z1', 'z2', 'z3',
    'p0', 'p1', 'p2', 'p3', 'ffr', 'nzcv', 'fpcr', 'fpsr'
  ],
  armv7: [
    'r0', 'r1', 'r2', 'r3', 'r4', 'r5', 'r6', 'r7', 'r8', 'r9', 'r10', 'r11',
    'r12', 'r13', 'r14', 'r15', 'sp', 'lr', 'pc', 'cpsr', 'spsr',
    's0', 's1', 's2', 's3', 'd0', 'd1', 'd2', 'd3', 'q0', 'q1', 'q2', 'q3'
  ],
  riscv32: [
    'x0', 'ra', 'sp', 'gp', 'tp', 't0', 't1', 't2', 's0', 's1', 'a0', 'a1',
    'a2', 'a3', 'a4', 'a5', 'a6', 'a7', 's2', 's3', 's4', 's5', 's6', 's7',
    's8', 's9', 's10', 's11', 't3', 't4', 't5', 't6', 'f0', 'f1', 'f2', 'f3'
  ],
  riscv64: [
    'x0', 'ra', 'sp', 'gp', 'tp', 't0', 't1', 't2', 's0', 's1', 'a0', 'a1',
    'a2', 'a3', 'a4', 'a5', 'a6', 'a7', 's2', 's3', 's4', 's5', 's6', 's7',
    's8', 's9', 's10', 's11', 't3', 't4', 't5', 't6', 'f0', 'f1', 'f2', 'f3'
  ],
  wasm32: ['$0', '$1', '$2', '$3']
};

const MACHINE_INSTRUCTIONS_BY_ARCH = {
  x86_64: [
    'mov', 'movzx', 'movsx', 'lea', 'add', 'adc', 'adcx', 'adox', 'sub', 'sbb', 'imul', 'mul',
    'div', 'idiv', 'and', 'or', 'xor', 'not', 'neg', 'shl', 'shr', 'sar', 'rol', 'ror',
    'cmp', 'test', 'cmov', 'setcc', 'jmp', 'je', 'jne', 'call', 'ret', 'push', 'pop',
    'in', 'out', 'hlt', 'nop', 'pause', 'mfence', 'sfence', 'lfence', 'serialize', 'cldemote',
    'clflush', 'clflushopt', 'clwb', 'prefetchnta', 'prefetcht0', 'prefetcht1', 'prefetcht2',
    'monitor', 'mwait', 'umonitor', 'umwait', 'tpause', 'syscall', 'sysret', 'rdtsc', 'rdtscp',
    'cpuid', 'getsec', 'rdpmc', 'rdpid', 'rdmsr', 'wrmsr', 'ud2', 'endbr64', 'endbr32', 'rdpkru', 'wrpkru',
    'rdrand', 'rdseed', 'aesenc', 'pclmulqdq', 'sha256rnds2', 'vmxon', 'vmptrld',
    'vmptrst', 'vmread', 'vmwrite', 'invept', 'invvpid', 'xsave', 'xrstor',
    'wbnoinvd', 'clts', 'clc', 'stc', 'cli', 'sti', 'cld', 'std'
  ],
  i386: [
    'mov', 'movzx', 'movsx', 'lea', 'add', 'adc', 'sub', 'sbb', 'imul', 'mul', 'div',
    'idiv', 'and', 'or', 'xor', 'not', 'neg', 'shl', 'shr', 'sar', 'cmp', 'test',
    'jmp', 'je', 'jne', 'call', 'ret', 'push', 'pop', 'pushad', 'popad', 'in', 'out',
    'aaa', 'aad', 'aam', 'aas', 'daa', 'das', 'salc', 'icebp', 'into', 'xlatb', 'ud2', 'hlt', 'nop',
    'endbr32', 'rdpkru', 'wrpkru', 'getsec', 'rdpmc', 'rdpid'
  ],
  aarch64: [
    'mov', 'movz', 'movk', 'adr', 'adrp', 'add', 'adds', 'sub', 'subs', 'mul', 'madd',
    'udiv', 'sdiv', 'and', 'orr', 'eor', 'lsl', 'lsr', 'asr', 'cmp', 'csel', 'b', 'bl',
    'br', 'blr', 'ret', 'ldr', 'str', 'ldp', 'stp', 'dmb', 'dsb', 'isb', 'sb', 'ssbb', 'pssbb', 'csdb', 'wfi', 'wfe',
    'sev', 'eret', 'mrs', 'msr', 'extr', 'ubfm', 'sbfm', 'bfm', 'paciasp', 'autiasp',
    'xpaclri', 'xpaci', 'xpacd', 'paciza', 'autiza', 'pacia', 'autda', 'ldxr', 'stxr', 'cas', 'swp',
    'whilelo', 'ptrue', 'pfalse', 'ld1b', 'ld1h', 'ld1w', 'ld1d', 'st1b', 'st1h', 'st1w', 'st1d',
    'cntb', 'cnth', 'cntw', 'cntd', 'incb', 'inch', 'incw', 'incd', 'cmpeq', 'cmpne', 'cmplt',
    'cmpgt', 'cmpge', 'cmple', 'sel', 'fmla', 'faddv', 'esb', 'tsb', 'dgh', 'xaflag', 'axflag', 'wfet', 'wfit'
  ],
  armv7: [
    'mov', 'movw', 'movt', 'add', 'adc', 'sub', 'sbc', 'mul', 'mla', 'udiv', 'sdiv',
    'and', 'orr', 'eor', 'lsl', 'lsr', 'asr', 'cmp', 'b', 'bl', 'bx', 'ldr', 'str',
    'ldm', 'stm', 'push', 'pop', 'dmb', 'dsb', 'isb', 'yield', 'wfi', 'setend', 'qadd', 'qsub',
    'ssat', 'usat', 'ldrex', 'strex', 'dbg', 'cpsid', 'cpsie', 'hvc', 'smc'
  ],
  riscv32: [
    'lui', 'auipc', 'jal', 'jalr', 'beq', 'bne', 'blt', 'bge', 'lb', 'lh', 'lw',
    'sb', 'sh', 'sw', 'addi', 'slti', 'xori', 'ori', 'andi', 'slli', 'srli', 'srai',
    'add', 'sub', 'mul', 'div', 'rem', 'fence', 'fence.tso', 'wrs.nto', 'wrs.sto', 'ecall', 'ebreak', 'wfi', 'mret',
    'amoadd.w', 'lr.w', 'sc.w', 'fence.i', 'pause', 'cbo.clean', 'cbo.flush', 'cbo.inval', 'cbo.zero',
    'sfence.vma', 'sfence.w.inval', 'sfence.inval.ir', 'sinval.vma', 'csrr', 'csrw', 'csrrw', 'csrrs', 'csrrc', 'csrrwi', 'csrrsi', 'csrrci',
    'clz', 'ctz', 'cpop', 'sext.b', 'sext.h', 'zext.h', 'orc.b', 'rev8',
    'bset', 'bclr', 'binv', 'bext', 'bseti', 'bclri', 'binvi', 'bexti', 'czero.eqz', 'czero.nez',
    'hfence.vvma', 'hfence.gvma', 'hinval.vvma', 'hinval.gvma',
    'hlv.b', 'hlv.bu', 'hlv.h', 'hlv.hu', 'hlv.w', 'hlvx.hu', 'hsv.b', 'hsv.h', 'hsv.w'
  ],
  riscv64: [
    'lui', 'auipc', 'jal', 'jalr', 'beq', 'bne', 'blt', 'bge', 'lb', 'lh', 'lw', 'ld',
    'sb', 'sh', 'sw', 'sd', 'addi', 'addiw', 'slli', 'srli', 'srai', 'add', 'sub', 'mul',
    'div', 'rem', 'fence', 'fence.tso', 'wrs.nto', 'wrs.sto', 'ecall', 'ebreak', 'wfi', 'mret', 'sret', 'hfence.gvma',
    'hfence.vvma', 'amoadd.d', 'lr.d', 'sc.d', 'bset', 'bclr', 'binv', 'bext',
    'fence.i', 'pause', 'cbo.clean', 'cbo.flush', 'cbo.inval', 'cbo.zero',
    'sfence.vma', 'sfence.w.inval', 'sfence.inval.ir', 'sinval.vma', 'csrr', 'csrw', 'csrrw', 'csrrs', 'csrrc', 'csrrwi', 'csrrsi', 'csrrci',
    'clz', 'ctz', 'cpop', 'clzw', 'ctzw', 'cpopw', 'sext.b', 'sext.h', 'zext.h', 'orc.b', 'rev8',
    'bseti', 'bclri', 'binvi', 'bexti', 'czero.eqz', 'czero.nez',
    'hfence.vvma', 'hinval.vvma', 'hinval.gvma',
    'hlv.b', 'hlv.bu', 'hlv.h', 'hlv.hu', 'hlv.w', 'hlv.wu', 'hlv.d',
    'hlvx.hu', 'hlvx.wu', 'hsv.b', 'hsv.h', 'hsv.w', 'hsv.d'
  ],
  wasm32: [
    'unreachable', 'nop', 'block', 'loop', 'if', 'else', 'end', 'br', 'br_if', 'return',
    'call', 'drop', 'select', 'local.get', 'local.set', 'local.tee', 'global.get', 'global.set',
    'i32.load', 'i32.store', 'i32.const', 'i32.eqz', 'i32.add', 'i32.sub', 'i32.mul',
    'i32.eq', 'i32.ne', 'i32.lt_s', 'i32.lt_u', 'i32.gt_s', 'i32.gt_u', 'i32.le_s', 'i32.le_u', 'i32.ge_s', 'i32.ge_u',
    'i32.div_s', 'i32.div_u', 'i32.rem_s', 'i32.rem_u', 'i32.and', 'i32.or', 'i32.xor', 'i32.shl', 'i32.shr_s', 'i32.shr_u', 'i32.rotl', 'i32.rotr',
    'i64.load', 'i64.store', 'i64.const', 'i64.eqz', 'i64.add', 'i64.sub', 'i64.mul', 'i64.div_s', 'i64.div_u',
    'i64.and', 'i64.or', 'i64.xor', 'i64.shl', 'i64.shr_s', 'i64.shr_u', 'i64.rotl', 'i64.rotr', 'memory.size', 'memory.grow',
    'memory.copy', 'memory.fill', 'memory.init', 'data.drop', 'table.get', 'table.set', 'table.size', 'table.grow',
    'table.fill', 'table.copy', 'table.init', 'elem.drop', 'ref.null', 'ref.is_null', 'ref.func'
  ]
};

const MACHINE_REGISTERS = Array.from(new Set(Object.values(MACHINE_REGISTERS_BY_ARCH).flat()));
const MACHINE_INSTRUCTIONS = Array.from(new Set([
  ...Object.values(MACHINE_INSTRUCTIONS_BY_ARCH).flat(),
  'volatile', 'opcode', 'ModRM', 'SIB', 'REX', 'VEX', 'EVEX'
]));

const BASAL_ENTITIES = [
  'root', 'binary', 'target', 'const', 'site', 'bind', 'routine', 'verify', 'report',
  'seal', 'identity', 'version', 'profile', 'authority', 'evidence', 'lowering',
  'kind', 'size', 'align', 'permissions', 'symbol', 'status', 'state', 'emit', 'require', 'hash'
];

const MAIN_FAMILY_WORDS = ['to main', 'ref main'];
const NC_EXTENSIONS = ['.n', '.nb', '.nbb', '.nm', '.ncp', '.nbo', '.init'];

const MODE_PATTERN_MAP = [
  { key: 'runtime', pattern: /::\s*runtime\s*\{/ },
  { key: 'compiler', pattern: /::\s*compiler\s*\{/ },
  { key: 'system', pattern: /::\s*system\s*\{/ },
  { key: 'tooling', pattern: /::\s*tooling\s*\{/ },
  { key: 'inspect', pattern: /::\s*inspect\s*\{/ },
  { key: 'security', pattern: /::\s*security\s*\{/ },
  { key: 'network', pattern: /::\s*network\s*\{/ },
  { key: 'kernel', pattern: /::\s*kernel\s*\{/ },
  { key: 'machine', pattern: /::\s*machine\s*\{/ },
  { key: 'ai', pattern: /::\s*ai\s*\{/ },
  { key: 'primitive', pattern: /::\s*primitive\s*\{/ },
  { key: 'root', pattern: /::\s*basal\s*\{/ },
  { key: 'program', pattern: /::\s*program\s*\{/ }
];

function compileWordRegex(words) {
  return new RegExp(`\\b(?:${words.map((w) => w.replace(/[.*+?^${}()|[\\]\\]/g, '\\\\$&')).join('|')})\\b`, 'g');
}

const PRIMITIVE_ENTITY_RE = compileWordRegex(PRIMITIVE_ENTITIES);
const MACHINE_INSTRUCTION_RE = compileWordRegex(MACHINE_INSTRUCTIONS);
const MACHINE_REGISTER_RE = compileWordRegex(MACHINE_REGISTERS);
const BASAL_ENTITY_RE = compileWordRegex(BASAL_ENTITIES);

const MODE_TOKEN_ROWS = [
  { pattern: /\bruntime(?=\s*\{)/g, tokenType: 'modeRuntime' },
  { pattern: /\bcompiler(?=\s*\{)/g, tokenType: 'modeCompiler' },
  { pattern: /\b(?:program|system|tooling|inspect|security|network|ai)(?=\s*\{)/g, tokenType: 'modeSystem' },
  { pattern: /\bkernel(?=\s*\{)/g, tokenType: 'modeKernel' },
  { pattern: /\bmachine(?=\s*\{)/g, tokenType: 'modeMachine' },
  { pattern: /\bprimitive(?=\s*\{)/g, tokenType: 'modePrimitive' },
  { pattern: /\bbasal(?=\s*\{)/g, tokenType: 'modeBasal' }
];

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

function loadBaseRegistry(extensionPath) {
  const registryPath = path.join(extensionPath, 'data', 'nc_base_registry.json');
  try {
    const raw = fs.readFileSync(registryPath, 'utf8');
    const parsed = JSON.parse(raw);
    return parsed;
  } catch (_) {
    return {
      schema: 'nc.vscode.base.registry.v1',
      version: '1.0.0.21v',
      state: 'fallback',
      source_docs: {},
      plus_operator: {
        strict_concat_message: 'Use explicit render in strict mode: "text" + renderInt(@x)',
        implicit_concat_error_codes: ['E_TEXT_PLUS_001', 'E_PLUS_002', 'E_SEC_RENDER_001', 'E_PRIMITIVE_TEXT_001'],
        allowed_textual_forms: ['str + str', 'str + renderInt(@x)']
      },
      print_family: PRINT_MEMBERS.map((name) => ({ name, tier: 'mixed', channel: 'stdout', status: 'active' })),
      runtime_families: [],
      runtime_capabilities: [],
      runtime_policies: [],
      diagnostics_catalog: []
    };
  }
}

function defaultIntellisenseCatalog() {
  return ensureRequiredCatalogSurfaces({
    schema: 'nc.ide.intellisense.catalog.v1',
    version: 'fallback',
    authority: 'editor-assist-only',
    symbols: IMPORT_SUGGESTIONS.map((row) => ({
      label: row.symbol,
      kind: 'function',
      legacyImport: row.importPath,
      detail: row.detail,
      documentation: row.detail,
      risk: 'low'
    })),
    types: POINTER_BASE.map((row) => ({
      label: row.symbol,
      kind: 'type',
      signature: row.title,
      detail: row.detail,
      documentation: row.detail,
      risk: 'medium'
    })),
    diagnostics: DIAGNOSTIC_HELP.map((row) => ({
      label: row.code,
      kind: 'diagnostic',
      detail: row.detail,
      documentation: row.detail
    }))
  });
}

function loadIntellisenseCatalog(extensionPath) {
  const catalogPath = path.join(extensionPath, 'data', 'nc_intellisense_catalog.json');
  try {
    const raw = fs.readFileSync(catalogPath, 'utf8');
    const parsed = JSON.parse(raw);
    return ensureRequiredCatalogSurfaces({
      schema: parsed.schema || 'nc.ide.intellisense.catalog.v1',
      version: parsed.version || 'unknown',
      authority: parsed.authority || 'editor-assist-only',
      symbols: Array.isArray(parsed.symbols) ? parsed.symbols : [],
      types: Array.isArray(parsed.types) ? parsed.types : [],
      diagnostics: Array.isArray(parsed.diagnostics) ? parsed.diagnostics : []
    });
  } catch (_) {
    return defaultIntellisenseCatalog();
  }
}

function loadStoudStdlibCatalog(extensionPath) {
  const catalogPath = path.join(extensionPath, 'data', 'stoud_stdlib_catalog.json');
  try {
    const raw = fs.readFileSync(catalogPath, 'utf8');
    const parsed = JSON.parse(raw);
    return {
      schema: parsed.schema || 'nc.stoud.stdlib.catalog.v1',
      version: parsed.version || 'unknown',
      canonicalFamilies: Array.isArray(parsed.canonicalFamilies) ? parsed.canonicalFamilies : [],
      families: Array.isArray(parsed.families) ? parsed.families : []
    };
  } catch (_) {
    return {
      schema: 'nc.stoud.stdlib.catalog.v1',
      version: 'fallback',
      canonicalFamilies: FACADE_IMPORT_FAMILIES
        .filter((row) => row.domain === 'stoud')
        .map((row) => row.family),
      families: []
    };
  }
}

function catalogEntries(catalog) {
  const source = catalog || defaultIntellisenseCatalog();
  return [
    ...(Array.isArray(source.symbols) ? source.symbols : []),
    ...(Array.isArray(source.types) ? source.types : []),
    ...(Array.isArray(source.diagnostics) ? source.diagnostics : [])
  ];
}

function cleanCatalogToken(raw) {
  return normalizeWord(raw)
    .replace(/\(.*$/, '')
    .replace(/[;:,]+$/, '')
    .trim();
}

function catalogEntryMatches(row, token) {
  if (!row || !token) {
    return false;
  }
  const aliases = Array.isArray(row.aliases) ? row.aliases : [];
  return row.label === token
    || row.signature === token
    || aliases.includes(token)
    || (row.signature || '').startsWith(`${token}<`)
    || (row.signature || '').startsWith(`${token}(`);
}

function lookupCatalogEntry(catalog, rawWord) {
  const direct = (rawWord || '').trim();
  if (direct) {
    const directHit = catalogEntries(catalog).find((row) => catalogEntryMatches(row, direct));
    if (directHit) {
      return directHit;
    }
  }
  const clean = cleanCatalogToken(rawWord);
  if (!clean) {
    return null;
  }
  return catalogEntries(catalog).find((row) => catalogEntryMatches(row, clean)) || null;
}

function catalogLookupTokens(catalog) {
  const tokens = [];
  for (const row of catalogEntries(catalog)) {
    const candidates = [row.label, ...(Array.isArray(row.aliases) ? row.aliases : [])]
      .filter(Boolean)
      .filter((token) => /[^A-Za-z0-9_]/.test(token) || token.includes(' '));
    for (const token of candidates) {
      tokens.push({ token, row });
    }
  }
  return tokens.sort((a, b) => b.token.length - a.token.length);
}

function lookupCatalogEntryAtLinePosition(catalog, lineText, character) {
  const word = findWordRangeInLine(lineText, character);
  if (word) {
    const hit = lookupCatalogEntry(catalog, word.text);
    if (hit) {
      return { entry: hit, start: word.start, end: word.end };
    }
  }
  const pos = Math.max(0, character || 0);
  for (const candidate of catalogLookupTokens(catalog)) {
    let idx = lineText.indexOf(candidate.token);
    while (idx >= 0) {
      const end = idx + candidate.token.length;
      if (pos >= idx && pos <= end) {
        return { entry: candidate.row, start: idx, end };
      }
      idx = lineText.indexOf(candidate.token, idx + 1);
    }
  }
  return null;
}

function preferredImportStatement(row) {
  if (!row) {
    return '';
  }
  if (isBackendPreludeEntry(row)) {
    return '';
  }
  if (row.facadeImport) {
    return row.facadeImport;
  }
  const legacy = row.legacyImport || row.importPath;
  return legacy ? `use ${legacy};` : '';
}

function facadeImportScope(row) {
  const parsed = parseFacadeImportStatement(preferredImportStatement(row));
  if (!parsed) {
    return null;
  }
  return {
    domain: parsed.domain,
    family: parsed.family,
    items: parsed.items
  };
}

function legacyImportStatement(row) {
  if (isBackendPreludeEntry(row)) {
    return '';
  }
  const legacy = row ? (row.legacyImport || row.importPath) : '';
  return legacy ? `use ${legacy};` : '';
}

function isBackendPreludeSymbol(symbol) {
  return BACKEND_PRELUDE_SYMBOLS.has(cleanCatalogToken(symbol));
}

function isBackendPreludeEntry(row) {
  if (!row) {
    return false;
  }
  const legacy = row.legacyImport || row.importPath || '';
  return BACKEND_PRELUDE_IMPORTS.has(legacy)
    || isBackendPreludeSymbol(row.symbol || row.label || row.name || '');
}

function hasFacadeImport(text, statement) {
  const expected = (statement || '').trim();
  if (!expected) {
    return false;
  }
  const expectedFacade = parseFacadeImportStatement(expected);
  return splitLines(text).some((line) => {
    const trimmed = line.trim();
    if (trimmed === expected) {
      return true;
    }
    const actualFacade = parseFacadeImportStatement(trimmed);
    if (!expectedFacade || !actualFacade) {
      return false;
    }
    if (actualFacade.domain !== expectedFacade.domain || actualFacade.family !== expectedFacade.family) {
      return false;
    }
    return expectedFacade.items.every((item) => actualFacade.items.includes(item));
  });
}

function parseFacadeImportStatement(line) {
  const raw = (line || '').trim();
  const compact = raw.match(/^use\s*<\s*([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)?)\s*:\s*([^>]*)>\s*;?$/);
  if (compact) {
    const path = compact[1].split('.');
    const items = compact[2].split(',').map((item) => item.trim()).filter(Boolean);
    if (path.length === 1 && path[0] === 'project') {
      return { domain: 'project', family: 'project', items };
    }
    return {
      domain: path.length === 1 ? 'stoud' : path[0],
      family: path.length === 1 ? path[0] : path[1],
      items
    };
  }
  const m = raw.match(/^use\s+([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\s*\{\s*([^}]*)\s*\}\s*;?$/);
  if (!m) return null;
  const path = m[1].split('.');
  const items = (m[2] || '')
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);
  if (path.length === 1 && path[0] === 'project') {
    return { domain: 'project', family: 'project', items };
  }
  return {
    domain: path[0],
    family: path.slice(1).join('.'),
    items
  };
}

function mergeFacadeImportStatement(text, statement) {
  const expected = parseFacadeImportStatement(statement);
  if (!expected || expected.items.length === 0) {
    return null;
  }
  const lines = splitLines(text);
  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    const actual = parseFacadeImportStatement(line);
    if (!actual || actual.domain !== expected.domain || actual.family !== expected.family) {
      continue;
    }
    const merged = actual.items.slice();
    for (const item of expected.items) {
      if (!merged.includes(item)) {
        merged.push(item);
      }
    }
    const indent = (line.match(/^\s*/) || [''])[0];
    const selector = actual.domain === 'project'
      ? 'project'
      : actual.domain === 'stoud' ? actual.family : `${actual.domain}.${actual.family}`;
    return {
      line: i,
      lineText: line,
      newLine: `${indent}use <${selector}:${merged.join(', ')}>;`,
      changed: merged.length !== actual.items.length
    };
  }
  return null;
}

function expandCompactEmptyBlockLine(line) {
  const source = line || '';
  const match = source.match(/^(\s*)((?:[A-Za-z_][A-Za-z0-9_]*)(?:\[[^\]]+\])?|to\s+main|raw|bridge|match\s*\([^)]*\)|if\s*\([^)]*\)|while\s*\([^)]*\)|for\s*\([^)]*\))\s*\{\s*\}\s*$/);
  if (!match) {
    return null;
  }
  const indent = match[1] || '';
  const header = match[2].trim();
  return `${indent}${header} {\n${indent}    \n${indent}}`;
}

function hasImportForEntry(text, row) {
  if (!row) {
    return false;
  }
  if (isBackendPreludeEntry(row)) {
    return true;
  }
  const preferred = preferredImportStatement(row);
  const legacy = row.legacyImport || row.importPath;
  return hasFacadeImport(text, preferred) || (legacy ? hasImport(text, legacy) : false);
}

function isNcLanguageId(id) {
  return id === 'nc' || id === 'nc-root' || id === 'nc-machine' || id === 'nc-primitive' || id === 'nc-hardware';
}

function sanitizeProfile(raw) {
  const value = (raw || 'standard').toString().trim().toLowerCase();
  if (value === 'novice' || value === 'standard' || value === 'expert') {
    return value;
  }
  return 'standard';
}

function sanitizeLspMode(raw) {
  const value = (raw || 'auto').toString().trim().toLowerCase();
  if (['auto', 'nclsp', 'nc-sys-lsp', 'nide-broker'].includes(value)) {
    return value;
  }
  return 'auto';
}

function supportedNcExtension(fileName) {
  const lowered = (fileName || '').toLowerCase();
  return NC_EXTENSIONS.some((ext) => lowered.endsWith(ext));
}

function diagnosticLocationFromMessage(message) {
  const text = (message || '').toString();
  let m = text.match(/\[(\d+):(\d+)\]/);
  if (m) {
    return { line: Number(m[1]), column: Number(m[2]) };
  }
  m = text.match(/(?:^|\s|:)(\d+):(\d+)(?::|\s|$)/);
  if (m) {
    return { line: Number(m[1]), column: Number(m[2]) };
  }
  return { line: 1, column: 1 };
}

function diagnosticCodeFromMessage(message) {
  const text = (message || '').toString();
  const m = text.match(/\b(?:E|W|NC)[A-Z0-9_-]*(?:_[A-Z0-9]+)*\b/);
  return m ? m[0] : '';
}

function detectLowLevelByContent(content) {
  const head = (content || '').slice(0, 8192);
  return /::\s*(?:machine|primitive|basal)\s*\{/.test(head);
}

function detectModeMarkers(content) {
  const head = (content || '').slice(0, 8192);
  const found = [];
  for (const row of MODE_PATTERN_MAP) {
    if (row.pattern.test(head)) {
      found.push(row.key);
    }
  }
  return found;
}

function detectCompactKernelSurface(content) {
  const code = maskNonCodeForNavigation(content || '');
  const physicalBinding = /(?:^|[;{}])\s*[A-Za-z_][A-Za-z0-9_]*\s+at\s+/m;
  const mmioWrite = /\*\s*(?:8|16|32|64|128|256|512)\s*\(/;
  const namedKernelContainer = /^\s*[A-Za-z_][A-Za-z0-9_]*\s*::\s*(?:kernel|system)\s*\{/im;
  const directMain = /\bref\s+main\s*->>/i;
  return physicalBinding.test(code) || mmioWrite.test(code) ||
    (namedKernelContainer.test(code) && directMain.test(code));
}

function detectModeFamily(fileName, content) {
  const lowered = (fileName || '').toLowerCase();
  if (lowered.endsWith('.nb')) {
    return 'root-basal';
  }
  if (lowered.endsWith('.nbb')) {
    return 'primitive-low-level';
  }
  if (lowered.endsWith('.nm')) {
    return 'machine-low-level';
  }
  if (lowered.endsWith('.ncp')) {
    return 'hardware-low-level';
  }

  const markers = detectModeMarkers(content || '');
  if (markers.includes('primitive')) {
    return 'primitive-low-level';
  }
  if (markers.includes('machine')) {
    return 'machine-low-level';
  }
  if (markers.includes('root')) {
    return 'root-basal';
  }
  if (markers.length > 0) {
    return `${markers[0]}-level`;
  }
  if (detectCompactKernelSurface(content || '')) {
    return 'kernel-level';
  }
  return 'high-level';
}

function splitLines(text) {
  return (text || '').split(/\r?\n/);
}

function parseSymbols(text) {
  const lines = splitLines(text);
  const containers = [];
  const functions = [];
  const variables = [];
  const parameters = [];
  const mains = [];

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];

    const containerMatch = line.match(/^([A-Za-z_][A-Za-z0-9_]*)(?:\s*::\s*(?:program|runtime|network|system|kernel|tooling|compiler|security|inspect|ai|machine|primitive|basal))?(?:\s*\[[^\]]+\])?\s*\{/);
    if (containerMatch && !KEYWORDS.includes(containerMatch[1])) {
      const name = containerMatch[1];
      containers.push({ name, line: i, character: line.indexOf(name) });
    }

    const fnMatch = line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*(?:::\s*(?:in|fn))?\s*\(([^\)]*)\)\s*(?:->\s*[^\s\{]+)?\s*\{/);
    if (fnMatch && !KEYWORDS.includes(fnMatch[1])) {
      const name = fnMatch[1];
      functions.push({ name, line: i, character: line.indexOf(name) });
      for (const param of fnMatch[2].split(',')) {
        const paramMatch = param.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*>>\s*([^,\)]+)/);
        if (paramMatch && !KEYWORDS.includes(paramMatch[1])) {
          const paramName = paramMatch[1];
          parameters.push({
            name: paramName,
            typeName: cleanDeclaredType(paramMatch[2]),
            functionName: name,
            line: i,
            character: line.indexOf(paramName, line.indexOf('('))
          });
        }
      }
    }

    const compactVarMatch = line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*>>\s*(.+?)\s*(?:<-|;)/);
    if (compactVarMatch && !KEYWORDS.includes(compactVarMatch[1])) {
      const name = compactVarMatch[1];
      if (!variables.some((row) => row.name === name && row.line === i)) {
        variables.push({
          name,
          typeName: cleanDeclaredType(compactVarMatch[2]),
          line: i,
          character: line.indexOf(name)
        });
      }
    }

    if (/\bto\s+main\b/.test(line)) {
      mains.push({ kind: 'to-main', line: i, character: line.indexOf('main') });
    }
    if (/\bref\s+main\b/.test(line)) {
      mains.push({ kind: 'ref-main', line: i, character: line.indexOf('main') });
    }
  }

  return { containers, functions, variables, parameters, mains };
}

function findWordRangeInLine(lineText, character) {
  if (!lineText || character < 0 || character > lineText.length) {
    return null;
  }
  const re = /[@A-Za-z_][A-Za-z0-9_\.\[\]@]*/g;
  let m = re.exec(lineText);
  while (m) {
    const start = m.index;
    const end = m.index + m[0].length;
    if (character >= start && character <= end) {
      return { start, end, text: m[0] };
    }
    m = re.exec(lineText);
  }
  return null;
}

function collectOccurrences(text, symbolName) {
  if (!symbolName) {
    return [];
  }
  const escaped = symbolName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const re = new RegExp(`(?:@)?\\b${escaped}\\b`, 'g');
  const lines = splitLines(text);
  const out = [];
  for (let lineIdx = 0; lineIdx < lines.length; lineIdx += 1) {
    const line = lines[lineIdx];
    let m = re.exec(line);
    while (m) {
      const raw = m[0];
      const start = m.index + (raw.startsWith('@') ? 1 : 0);
      out.push({ line: lineIdx, character: start, length: symbolName.length });
      m = re.exec(line);
    }
  }
  return out;
}

function completionPool(registry, catalog) {
  const runtimeFamilies = Array.isArray(registry.runtime_families) ? registry.runtime_families : [];
  const printMembers = (Array.isArray(registry.print_family) ? registry.print_family : [])
    .map((row) => row.name)
    .filter(Boolean);
  const source = catalog || defaultIntellisenseCatalog();
  const catalogTypes = (Array.isArray(source.types) ? source.types : []).map((row) => row.label).filter(Boolean);

  return {
    keywords: KEYWORDS.slice(),
    types: Array.from(new Set([...TYPES, ...catalogTypes])),
    pointerTypes: POINTER_TYPES.slice(),
    printMembers: Array.from(new Set([...PRINT_MEMBERS, ...printMembers])),
    runtimeFamilies: runtimeFamilies.slice(),
    catalogSymbols: Array.isArray(source.symbols) ? source.symbols.slice() : [],
    catalogTypes: Array.isArray(source.types) ? source.types.slice() : [],
    snippets: [
      'name {',
      'stc:',
      'const:',
      'type:',
      'local:',
      'fn:',
      'public:',
      'internal:',
      'private:',
      'abi native {',
      'ref main ->> run;',
      'fa:'
    ]
  };
}

function machineArchFromText(text) {
  const match = String(text || '').match(/\barch\s*:\s*(x86_64|i386|aarch64|armv7|riscv32|riscv64|wasm32)\b/i);
  return match ? match[1].toLowerCase() : '';
}

function lowLevelCompletionRow(label, insertText, detail, kind = 'keyword', rank = '20') {
  return {
    label,
    insertText: insertText || label,
    detail,
    kind,
    sortText: `${rank}_${label}`
  };
}

function machineCompletionRows(text, lineText = '') {
  const arch = machineArchFromText(text);
  const prefix = String(lineText || '');
  if (/\barch\s*:\s*[A-Za-z0-9_]*$/.test(prefix)) {
    return Object.keys(MACHINE_INSTRUCTIONS_BY_ARCH).map((name) => lowLevelCompletionRow(
      name, name, `Machine target architecture ${name}.`, 'enum', '00'
    ));
  }
  const targetValues = {
    abi: ['sysv64', 'win64', 'cdecl', 'aapcs64', 'aapcs32', 'ilp32', 'ilp32d', 'lp64', 'lp64d', 'wasm'],
    object: ['elf64', 'elf32', 'coff', 'macho64', 'wasm', 'raw'],
    endian: ['little', 'big'],
    pointerWidth: ['16', '32', '64', '128'],
    profile: ['userland', 'legacy-userland', 'kernel', 'firmware', 'nC-wasm32-core-v1'],
    binding: ['local', 'global', 'weak', 'extern', 'export'],
    type: ['routine', 'object', 'section'],
    section: ['.text', '.rodata', '.data', '.bss', '.tls', '.init', '.fini', '.pdata', '.xdata']
  };
  for (const [key, values] of Object.entries(targetValues)) {
    const pattern = new RegExp(`\\b${key}\\s*:\\s*[A-Za-z0-9_.-]*$`);
    if (pattern.test(prefix)) {
      return values.map((value) => lowLevelCompletionRow(value, value, `${key} value.`, 'enum', '00'));
    }
  }

  const rows = [
    lowLevelCompletionRow('machine container', '${1:name} :: machine {\n    target:\n        arch: ${2:x86_64};\n        abi: ${3:sysv64};\n        object: ${4:elf64};\n        endian: little;\n        pointerWidth: ${5:64};\n        profile: ${6:userland};\n\n    symbol:\n        ${7:main}:\n            section: .text;\n            binding: global;\n            type: routine;\n\n    entry:\n        ${7:main};\n\n    proc:\n        ${7:main}:\n            instr:\n                ${8:ret};\n\n    verify:\n        target;\n        operands;\n        encoding;\n        object;\n\n    seal:\n        artifact: ${4:elf64};\n        require: verified;\n}', 'Complete typed Machine unit.', 'snippet', '00'),
    lowLevelCompletionRow('target section', 'target:\n    arch: ${1:x86_64};\n    abi: ${2:sysv64};\n    object: ${3:elf64};\n    endian: little;\n    pointerWidth: ${4:64};\n    profile: ${5:userland};', 'Target, ABI and object contract.', 'snippet', '01'),
    lowLevelCompletionRow('symbol section', 'symbol:\n    ${1:main}:\n        section: .text;\n        binding: global;\n        type: routine;', 'Link-visible Machine symbol.', 'snippet', '01'),
    lowLevelCompletionRow('procedure section', 'proc:\n    ${1:main}:\n        instr:\n            ${2:ret};', 'Native instruction procedure.', 'snippet', '01'),
    lowLevelCompletionRow('verify section', 'verify:\n    target;\n    operands;\n    encoding;\n    object;', 'Material verification gates.', 'snippet', '01'),
    lowLevelCompletionRow('seal section', 'seal:\n    artifact: ${1:elf64};\n    require: verified;', 'Sealed output contract.', 'snippet', '01')
  ];
  for (const word of [
    'target', 'arch', 'abi', 'object', 'endian', 'pointerWidth', 'profile', 'features',
    'symbol', 'section', 'binding', 'type', 'entry', 'proc', 'instr', 'data', 'rodata',
    'bss', 'align', 'global', 'extern', 'public', 'reloc', 'verify', 'seal', 'artifact', 'require'
  ]) {
    rows.push(lowLevelCompletionRow(word, `${word}:`, 'Machine structural field.', 'keyword', '10'));
  }
  const arches = arch ? [arch] : Object.keys(MACHINE_INSTRUCTIONS_BY_ARCH);
  for (const name of arches.flatMap((name) => MACHINE_INSTRUCTIONS_BY_ARCH[name] || [])) {
    rows.push(lowLevelCompletionRow(name, `${name} `, `${arch || 'Multi-target'} native instruction.`, 'instruction', '20'));
  }
  for (const name of arches.flatMap((name) => MACHINE_REGISTERS_BY_ARCH[name] || [])) {
    rows.push(lowLevelCompletionRow(name, name, `${arch || 'Multi-target'} machine register.`, 'register', '15'));
  }
  return rows;
}

function basalCompletionRows() {
  const rows = [
    lowLevelCompletionRow('basal container', '${1:name} :: basal {\n    root:\n        identity: "${2:nC.root.name}";\n        version: 1;\n        profile: basal;\n        authority: sealed;\n\n    site:\n        ${3:rootSlot}:\n            kind: memory_anchor;\n            size: ${4:8};\n            align: ${5:8};\n            permissions: read | write;\n\n    verify:\n        ${6:true};\n\n    seal:\n        require: source_hash;\n}', 'Complete basal/root material contract.', 'snippet', '00'),
    lowLevelCompletionRow('root contract', 'root:\n    identity: "${1:nC.root.name}";\n    version: 1;\n    profile: basal;\n    authority: sealed;', 'Root identity and authority.', 'snippet', '01'),
    lowLevelCompletionRow('site contract', 'site:\n    ${1:slot}:\n        kind: memory_anchor;\n        size: ${2:8};\n        align: ${3:8};\n        permissions: read | write;', 'Address-backed root site.', 'snippet', '01'),
    lowLevelCompletionRow('binding contract', 'bind:\n    symbol ${1:name} -> root.site ${2:slot};', 'Bind a root path to a material symbol.', 'snippet', '01')
  ];
  for (const word of BASAL_ENTITIES) {
    rows.push(lowLevelCompletionRow(word, `${word}:`, 'Basal/root contract keyword.', 'keyword', '10'));
  }
  for (const value of ['memory_anchor', 'evidence_anchor', 'read', 'write', 'execute', 'source_hash', 'sealed', 'basal']) {
    rows.push(lowLevelCompletionRow(value, value, 'Basal material value.', 'enum', '20'));
  }
  return rows;
}

function primitiveCompletionRows(hardware) {
  const rows = [
    lowLevelCompletionRow(
      hardware ? 'hardware primitive container' : 'primitive container',
      'primitive[@] target ${1:x86_64}:\n    type material_target = "${2:x86_64-unknown-none}";\n    type material_environment = "freestanding";\n\n    isa:\n        ${3:base};\n\n    opcode:\n        ${4:native};\n\n    encoding:\n        machine_native_bytes;\n\n    relocation:\n        target_object_authority;\n\n    abi:\n        explicit_target_abi;\n\n    operation ${5:materialize}():\n        barrier compiler;\n\n    seal:\n        ${6:materialized};',
      hardware ? 'Physical hardware capability contract.' : 'Primitive material contract.',
      'snippet',
      '00'
    ),
    lowLevelCompletionRow('capability block', 'capability ${1:name}:\n    allow ${2:port.read};\n    deny ${3:host.console.write};', 'Explicit physical capability.', 'snippet', '01'),
    lowLevelCompletionRow('operation block', 'operation ${1:name}():\n    capability ${2:authority};\n    barrier ${3:io};', 'Primitive operation and barriers.', 'snippet', '01')
  ];
  const entities = hardware
    ? PRIMITIVE_ENTITIES
    : PRIMITIVE_ENTITIES.filter((name) => ![
      'voltage', 'current', 'temperature', 'phy', 'lane', 'peripheral', 'firmwareRegion'
    ].includes(name));
  for (const word of entities) {
    rows.push(lowLevelCompletionRow(word, `${word}:`, hardware ? 'Hardware/firmware directive.' : 'Primitive directive.', 'keyword', '10'));
  }
  for (const value of [
    'x86_64', 'i386', 'aarch64', 'armv7', 'riscv32', 'riscv64', 'wasm32', 'little', 'big',
    'read', 'write', 'rw', 'volatile', 'compiler', 'io', 'acquire', 'release', 'seq_cst',
    'freestanding', 'firmware', 'kernel', 'mmio', 'port_io', 'machine_native_bytes'
  ]) {
    rows.push(lowLevelCompletionRow(value, value, hardware ? 'Hardware material value.' : 'Primitive material value.', 'enum', '20'));
  }
  return rows;
}

function lowLevelCompletionRows(fileName, text, lineText = '') {
  const family = detectModeFamily(fileName, text);
  if (['machine-low-level', 'root-basal', 'primitive-low-level', 'hardware-low-level'].includes(family)) {
    return [];
  }
  return [];
}

function ensureRequiredCatalogSurfaces(catalog) {
  const result = catalog || { symbols: [], types: [], diagnostics: [] };
  const symbols = Array.isArray(result.symbols) ? result.symbols.slice() : [];
  const known = new Set(symbols.map((row) => row.label));
  for (const call of CALL_SIGNATURES.filter((row) => row.importHint === 'use <memory:map>;')) {
    if (known.has(call.name)) {
      continue;
    }
    known.add(call.name);
    symbols.push({
      label: call.name,
      kind: 'function',
      signature: call.signature,
      insertText: `${call.name}($1)`,
      legacyImport: 'nC.runtime.memory',
      facadeImport: call.importHint,
      detail: call.detail,
      documentation: call.detail,
      risk: 'high'
    });
  }
  return Object.assign({}, result, { symbols });
}

function maskNonCodeForNavigation(text) {
  let out = '';
  let inString = false;
  let inChar = false;
  let inBlockComment = false;
  let escaped = false;
  for (let i = 0; i < (text || '').length; i += 1) {
    const ch = text[i];
    const next = text[i + 1] || '';
    if (inBlockComment) {
      if (ch === '*' && next === '/') {
        out += '  ';
        i += 1;
        inBlockComment = false;
      } else {
        out += ch === '\n' ? '\n' : ' ';
      }
      continue;
    }
    if (inString || inChar) {
      out += ch === '\n' ? '\n' : ' ';
      if (escaped) {
        escaped = false;
        continue;
      }
      if (ch === '\\') {
        escaped = true;
        continue;
      }
      if (inString && ch === '"') {
        inString = false;
      } else if (inChar && ch === "'") {
        inChar = false;
      }
      continue;
    }
    if (ch === '/' && next === '/') {
      out += '  ';
      i += 1;
      while (i + 1 < text.length && text[i + 1] !== '\n') {
        out += ' ';
        i += 1;
      }
      continue;
    }
    if (ch === '/' && next === '*') {
      out += '  ';
      i += 1;
      inBlockComment = true;
      continue;
    }
    if (ch === '"') {
      out += ' ';
      inString = true;
      continue;
    }
    if (ch === "'") {
      out += ' ';
      inChar = true;
      continue;
    }
    out += ch;
  }
  return out;
}

function collectCodeOccurrences(text, symbolName) {
  if (!symbolName) {
    return [];
  }
  const masked = maskNonCodeForNavigation(text);
  const escaped = symbolName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const re = new RegExp(`(?:@)?\\b${escaped}\\b`, 'g');
  const lines = splitLines(masked);
  const out = [];
  for (let lineIdx = 0; lineIdx < lines.length; lineIdx += 1) {
    const line = lines[lineIdx];
    let m = re.exec(line);
    while (m) {
      const raw = m[0];
      const start = m.index + (raw.startsWith('@') ? 1 : 0);
      out.push({ line: lineIdx, character: start, length: symbolName.length });
      m = re.exec(line);
    }
  }
  return out;
}

function facadeImportCompletionRows(text, lineText = '', character = 0) {
  const prefix = (lineText || '').slice(0, Math.max(0, character));
  const trimmed = prefix.trim();
  const compactItem = prefix.match(/^\s*use\s*<\s*([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)?)\s*:\s*([^>]*)$/);
  const blockItem = prefix.match(/^\s*use\s+([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\s*\{\s*([^}]*)$/);
  const sourceMatch = compactItem || blockItem;
  const path = sourceMatch ? sourceMatch[1].split('.') : [];
  const itemMatch = sourceMatch
    ? path.length === 1 && path[0] === 'project'
      ? ['v2', 'project', 'project', sourceMatch[2]]
      : path.length === 1
        ? ['v2', 'stoud', path[0], sourceMatch[2]]
        : ['v2', path[0], path.slice(1).join('.'), sourceMatch[2]]
    : null;
  if (itemMatch) {
    const family = FACADE_IMPORT_FAMILIES.find((row) => row.domain === itemMatch[1] && row.family === itemMatch[2]);
    if (!family) {
      return [];
    }
    const used = new Set(itemMatch[3].split(',').map((item) => item.trim()).filter(Boolean));
    return family.items
      .filter((item) => !used.has(item))
      .map((item, index) => ({
        label: item,
        insertText: item,
        detail: `${family.domain}/${family.family} import item`,
        sortText: `00_import_item_${String(index).padStart(2, '0')}_${item}`,
        kind: 'item'
      }));
  }

  if (/^\s*use\s*<\s*project\s*:[^>]*$/.test(prefix) || /^\s*use\s+project\s*\{[^}]*$/.test(prefix)) {
    return [];
  }

  if (!/^use(?:(?:\s*<[^>]*)|(?:\s+[A-Za-z_][A-Za-z0-9_.]*)?(?:\s*\{[^}]*)?)?$/.test(trimmed)) {
    return [];
  }

  return FACADE_IMPORT_FAMILIES
    .filter((row) => !row.label.startsWith('use <project:') || !hasProjectImport(text, row.insertText))
    .map((row, index) => ({
      label: row.label,
      insertText: row.insertText,
      detail: row.detail,
      sortText: `00_import_family_${String(index).padStart(2, '0')}_${row.label}`,
      kind: 'family'
    }));
}

function hasProjectImport(text, statement) {
  const moduleMatch = (statement || '').match(/^use\s*<\s*project\s*:\s*([^>]+)>\s*;?$/)
    || (statement || '').match(/^use\s+project\s*\{\s*([^}]+)\s*\}\s*;?$/);
  if (!moduleMatch) {
    return false;
  }
  const moduleName = moduleMatch[1].replace(/\$\{\d+:([^}]+)\}/g, '$1').trim();
  return splitLines(text).some((line) => {
    const match = line.trim().match(/^use\s*<\s*project\s*:\s*([^>]+)>\s*;?$/)
      || line.trim().match(/^use\s+project\s*\{\s*([^}]+)\s*\}\s*;?$/);
    if (!match) {
      return false;
    }
    return match[1].split(',').map((item) => item.trim()).includes(moduleName);
  });
}

function isProjectImportCompletionContext(lineText = '', character = 0) {
  const prefix = (lineText || '').slice(0, Math.max(0, character));
  return /^\s*use\s*<\s*project\s*:[^>]*$/.test(prefix)
    || /^\s*use\s+project\s*\{[^}]*$/.test(prefix);
}

function importedFunctionCompletionRows(text, catalog) {
  const source = catalog || defaultIntellisenseCatalog();
  const rows = Array.isArray(source.symbols) && source.symbols.length > 0
    ? source.symbols
    : IMPORT_SUGGESTIONS.map((row) => Object.assign({ label: row.symbol, kind: 'function' }, row));
  const seen = new Set();
  const out = [];
  const push = (row) => {
    const label = row.label || row.symbol || row.name;
    if (!label || seen.has(label)) {
      return;
    }
    seen.add(label);
    out.push({
      label,
      insertText: row.insertText || `${label}($1)`,
      signature: row.signature || lookupCallSignature(label)?.signature || '',
      detail: row.detail || lookupCallSignature(label)?.detail || 'Imported function.',
      facadeImport: row.facadeImport || lookupCallSignature(label)?.importHint || '',
      row
    });
  };

  for (const row of rows) {
    const label = row.label || row.symbol;
    const isFunction = row.kind === 'function' || CALL_SIGNATURES.some((call) => call.name === label);
    if (isFunction && hasImportForEntry(text, row)) {
      push(row);
    }
  }

  for (const call of CALL_SIGNATURES) {
    const row = {
      label: call.name,
      kind: 'function',
      signature: call.signature,
      insertText: `${call.name}($1)`,
      detail: call.detail,
      facadeImport: call.importHint
    };
    if (hasImportForEntry(text, row)) {
      push(row);
    }
  }

  return out;
}

function cleanDeclaredType(raw) {
  const text = (raw || '')
    .replace(/\s+/g, ' ')
    .trim();
  let angleDepth = 0;
  let parenDepth = 0;
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (ch === '<') {
      angleDepth += 1;
    } else if (ch === '>') {
      angleDepth = Math.max(0, angleDepth - 1);
    } else if (ch === '(') {
      parenDepth += 1;
    } else if (ch === ')') {
      parenDepth = Math.max(0, parenDepth - 1);
    } else if ((ch === ';' || ch === '=' || ch === ':') && angleDepth === 0 && parenDepth === 0) {
      return text.slice(0, i).trim();
    }
  }
  return text;
}

function parseStructFields(text) {
  const lines = splitLines(text);
  const fieldsByType = new Map();
  let inTypeSection = false;
  let typeSectionIndent = -1;
  let current = null;
  let braceDepth = 0;

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    const trimmed = line.trimStart();
    const indent = line.length - trimmed.length;
    const section = trimmed.match(/^(const|type|stc|local|fn|fa|public|internal|private|layout)\s*:/);
    if (!current && section) {
      if (section[1] === 'type') {
        inTypeSection = true;
        typeSectionIndent = indent;
      } else if (indent <= typeSectionIndent && !['public', 'internal', 'private', 'layout'].includes(section[1])) {
        inTypeSection = false;
        typeSectionIndent = -1;
      }
    }

    if (!current) {
      const directType = line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*::\s*type\s*(?:\[[^\]]+\])?\s*\{/);
      const sectionType = inTypeSection
        ? line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*(?:\[[^\]]+\])?\s*\{/)
        : null;
      const typeMatch = directType || sectionType;
      if (typeMatch && !KEYWORDS.includes(typeMatch[1])) {
        current = { typeName: typeMatch[1], fields: [], line: i };
        braceDepth = (line.match(/\{/g) || []).length - (line.match(/\}/g) || []).length;
        fieldsByType.set(current.typeName, current.fields);
      }
    }

    if (current) {
      const fieldSource = i === current.line && line.includes('{')
        ? line.slice(line.indexOf('{') + 1)
        : line;
      const normalizedFieldSource = fieldSource.replace(/\b(?:public|private|internal)\s*:/g, ' ');
      const fieldRe = /(?:^|[;\s])([A-Za-z_][A-Za-z0-9_]*)\s*(?:>>|:)\s*([^;:=\{\}]+)/g;
      let fieldMatch = fieldRe.exec(normalizedFieldSource);
      while (fieldMatch) {
        current.fields.push({
          name: fieldMatch[1],
          typeName: cleanDeclaredType(fieldMatch[2]),
          ownerType: current.typeName,
          line: i,
          character: line.indexOf(fieldMatch[1])
        });
        fieldMatch = fieldRe.exec(normalizedFieldSource);
      }
      if (i !== current.line) {
        braceDepth += (line.match(/\{/g) || []).length - (line.match(/\}/g) || []).length;
      }
      if (braceDepth <= 0) {
        current = null;
      }
    }
  }

  return fieldsByType;
}

function baseAggregateType(typeName) {
  const clean = cleanDeclaredType(typeName).replace(/^@/, '');
  const pointerMatch = clean.match(/^(?:ptr|ref|site|ptr\s+mut|ref\s+mut)\s*<\s*([A-Za-z_][A-Za-z0-9_]*)/);
  if (pointerMatch) {
    return pointerMatch[1];
  }
  const direct = clean.match(/^([A-Za-z_][A-Za-z0-9_]*)/);
  return direct ? direct[1] : clean;
}

function elementAggregateType(typeName) {
  const clean = cleanDeclaredType(typeName);
  const match = clean.match(/^(?:vector|list|queue|dinArray|array|slice|span)\s*<\s*([A-Za-z_][A-Za-z0-9_]*)/);
  return match ? match[1] : '';
}

function structFieldType(fieldsByType, ownerType, fieldName) {
  const fields = fieldsByType.get(baseAggregateType(ownerType)) || [];
  const hit = fields.find((row) => row.name === fieldName);
  return hit ? hit.typeName : '';
}

function resolveFieldChainType(fieldsByType, rootType, chain) {
  let current = baseAggregateType(rootType);
  for (const fieldName of chain) {
    const next = structFieldType(fieldsByType, current, fieldName);
    if (!next) {
      return '';
    }
    current = baseAggregateType(next);
  }
  return current;
}

function aggregateReceiverAtPosition(text, lineText = '', character = 0) {
  const prefix = (lineText || '').slice(0, Math.max(0, character));
  const types = typedSymbolMap(text);
  const fieldsByType = parseStructFields(text);
  const match = prefix.match(/@?([A-Za-z_][A-Za-z0-9_]*)(\s*(?:\.\s*get\s*\([^)]*\)|\[[^\]]*\]))?((?:\s*\.\s*[A-Za-z_][A-Za-z0-9_]*)*)\s*\.\s*[A-Za-z0-9_]*$/);
  if (match && types.has(match[1])) {
    const rootType = match[2] ? elementAggregateType(types.get(match[1])) : baseAggregateType(types.get(match[1]));
    const chain = (match[3] || '')
      .split('.')
      .map((row) => row.trim())
      .filter(Boolean);
    return {
      receiver: match[1],
      typeName: resolveFieldChainType(fieldsByType, rootType, chain) || rootType
    };
  }
  return null;
}

function structFieldCompletionRows(text, lineText = '', character = 0) {
  const receiver = aggregateReceiverAtPosition(text, lineText, character);
  if (!receiver || !receiver.typeName) {
    return [];
  }
  const fields = parseStructFields(text).get(receiver.typeName) || [];
  return fields.map((field, index) => ({
    label: field.name,
    insertText: field.name,
    detail: `${field.ownerType}.${field.name}: ${field.typeName || 'value'}`,
    sortText: `00_field_${String(index).padStart(2, '0')}_${field.name}`,
    field
  }));
}

function entrypointCompletionRows(text, lineText = '', character = 0) {
  const symbols = parseSymbols(text);
  if (symbols.mains.length > 0) {
    return [];
  }
  const prefix = (lineText || '').slice(0, Math.max(0, character));
  if (prefix.trim() && !/^\s*ref(?:\s+main)?(?:\s*->>)?\s*$/.test(prefix)) {
    return [];
  }
  const functions = symbols.functions.length > 0
    ? symbols.functions
    : [{ name: 'run', line: 0, character: 0 }];
  return functions.map((fn, index) => ({
    label: `ref main ->> ${fn.name};`,
    insertText: `ref main ->> ${fn.name};`,
    detail: 'Entrypoint reference from a visible function.',
    sortText: `00_entry_${String(index).padStart(2, '0')}_${fn.name}`,
    functionName: fn.name
  }));
}

function projectModuleCompletionRowsFromPaths(paths, currentPath = '') {
  const current = currentPath ? path.resolve(currentPath).toLowerCase() : '';
  const seen = new Set();
  const rows = [];
  for (const candidate of paths || []) {
    if (!candidate) {
      continue;
    }
    const resolved = path.resolve(candidate).toLowerCase();
    if (current && resolved === current) {
      continue;
    }
    const ext = path.extname(candidate).toLowerCase();
    if (ext !== '.n' && ext !== '.nc') {
      continue;
    }
    const name = path.basename(candidate, ext);
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name) || seen.has(name)) {
      continue;
    }
    seen.add(name);
    rows.push({
      label: name,
      insertText: name,
      detail: `project module ${name}`,
      sortText: `00_project_${name}`
    });
  }
  return rows.sort((a, b) => a.label.localeCompare(b.label));
}

function hasImport(text, importPath) {
  const escaped = importPath.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return new RegExp(`^\\s*use\\s+${escaped}\\s*;`, 'm').test(text || '');
}

function symbolIsUsed(text, symbol) {
  const escaped = symbol.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return new RegExp(`\\b${escaped}\\s*(?:\\(|<|\\[|\\.|;)`, 'm').test(text || '');
}

function missingImportSuggestions(text, catalog) {
  const source = catalog || defaultIntellisenseCatalog();
  const catalogRows = Array.isArray(source.symbols) && source.symbols.length > 0
    ? source.symbols.map((row) => Object.assign({}, row, {
      symbol: row.label,
      importPath: row.legacyImport || row.importPath
    }))
    : IMPORT_SUGGESTIONS;
  const signatureRows = CALL_SIGNATURES
    .filter((row) => row.importHint)
    .map((row) => ({
      symbol: row.name,
      label: row.name,
      kind: 'function',
      facadeImport: row.importHint,
      detail: row.detail,
      documentation: row.detail
    }));
  const rows = [...catalogRows, ...signatureRows];
  const seen = new Set();
  const out = [];
  for (const row of rows) {
    const symbol = row.symbol || row.label;
    if (isBackendPreludeEntry(row)) {
      continue;
    }
    if (!symbol || (!row.facadeImport && !row.legacyImport && !row.importPath)) {
      continue;
    }
    const key = `${symbol}:${preferredImportStatement(row)}:${legacyImportStatement(row)}`;
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    if (symbolIsUsed(text, symbol) && !hasImportForEntry(text, row)) {
      out.push(row);
    }
  }
  return out;
}

function findSymbolUseLocation(text, symbol) {
  const escaped = symbol.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const re = new RegExp(`\\b${escaped}\\s*(?:\\(|<|\\[|\\.|;)`);
  const lines = splitLines(text);
  for (let lineIdx = 0; lineIdx < lines.length; lineIdx += 1) {
    const match = re.exec(lines[lineIdx]);
    if (match) {
      return { line: lineIdx, character: match.index, length: symbol.length };
    }
  }
  return { line: 0, character: 0, length: Math.max(1, symbol.length) };
}

function isTokenBoundaryText(inserted) {
  return /[\s();,:\[\]{}<>.]/.test(inserted || '');
}

function completedTokenBeforeBoundary(text, offset) {
  if (!text || offset < 0) {
    return '';
  }
  const left = text
    .slice(0, Math.min(offset, text.length))
    .replace(/[\s();,:\[\]{}<>.]+$/g, '');
  const match = left.match(/@?[A-Za-z_][A-Za-z0-9_.]*$/);
  if (!match) {
    return '';
  }
  return cleanCatalogToken(match[0].replace(/^@/, ''));
}

function autoImportAllowed(row) {
  if (isBackendPreludeEntry(row)) {
    return false;
  }
  const scope = facadeImportScope(row);
  if (!scope) {
    return Boolean(row && (row.legacyImport || row.importPath));
  }
  if (scope.domain === 'auth') {
    return false;
  }
  return scope.domain === 'stoud' || scope.domain === 'dom';
}

function autoImportCandidateAtOffset(text, offset, catalog) {
  const token = completedTokenBeforeBoundary(text, offset);
  if (!token) {
    return null;
  }
  const row = lookupCatalogEntry(catalog, token) || (() => {
    const signature = CALL_SIGNATURES.find((candidate) => candidate.name === token && candidate.importHint);
    return signature ? {
      label: signature.name,
      symbol: signature.name,
      kind: 'function',
      signature: signature.signature,
      facadeImport: signature.importHint,
      detail: signature.detail,
      documentation: signature.detail
    } : null;
  })();
  if (!row || !preferredImportStatement(row) || !autoImportAllowed(row) || hasImportForEntry(text, row)) {
    return null;
  }
  return row;
}

function editorDiagnostic(code, severity, line, character, length, message) {
  const safeLine = Math.max(0, line || 0);
  const safeCharacter = Math.max(0, character || 0);
  const safeLength = Math.max(1, length || 1);
  const body = message || code || 'nC diagnostic';
  const rendered = /^\[\d+:\d+\]/.test(body)
    ? body
    : `[${safeLine + 1}:${safeCharacter + 1}] ${body}`;
  return {
    code,
    severity,
    line: safeLine,
    character: safeCharacter,
    length: safeLength,
    message: rendered
  };
}

function compilerTextDiagnostics(raw) {
  const lines = splitLines(raw || '');
  const out = [];
  for (let i = 0; i < lines.length; i += 1) {
    const header = lines[i].match(/^\s*\[(\d+):(\d+)\]\s*(.+)$/);
    if (!header) {
      continue;
    }
    const sourceLine = lines[i + 1] || '';
    const caretLine = lines[i + 2] || '';
    let character = Math.max(0, Number(header[2]) - 1);
    let length = 1;
    const pipe = sourceLine.indexOf('|');
    const caret = caretLine.indexOf('^');
    if (pipe >= 0 && caret >= 0) {
      character = Math.max(0, caret - pipe - 2);
      const caretRun = (caretLine.slice(caret).match(/^\^+/) || ['^'])[0];
      length = Math.max(1, caretRun.length);
    }
    const block = [lines[i]];
    if (/^\s*\d+\s*\|/.test(sourceLine)) {
      block.push(sourceLine);
      if (caret >= 0) {
        block.push(caretLine);
      }
    }
    out.push({
      code: diagnosticCodeFromMessage(header[3]) || diagnosticCodeFromMessage(lines[i]),
      severity: /\bwarning\b/i.test(header[3]) ? 'warning' : 'error',
      line: Math.max(0, Number(header[1]) - 1),
      character,
      length,
      message: block.join('\n')
    });
  }
  return out;
}

function importFamilyCatalog() {
  const map = new Map();
  for (const row of FACADE_IMPORT_FAMILIES) {
    if (row.domain === 'project') {
      continue;
    }
    const domain = map.get(row.domain) || new Map();
    domain.set(row.family, new Set(row.items));
    map.set(row.domain, domain);
  }
  return map;
}

function importDiagnostics(text, options = {}) {
  const rows = [];
  const familyCatalog = importFamilyCatalog();
  const projectModules = new Set(options.projectModules || []);
  const lines = splitLines(text);
  for (let lineIdx = 0; lineIdx < lines.length; lineIdx += 1) {
    const line = lines[lineIdx];
    const trimmed = line.trim();
    if (!/^use\s+/.test(trimmed)) {
      continue;
    }

    const project = trimmed.match(/^use\s+project\s*\{\s*([^}]*)\s*\}\s*;?$/);
    if (project) {
      const items = project[1].split(',').map((item) => item.trim()).filter(Boolean);
      for (const item of items) {
        const at = line.indexOf(item);
        if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(item)) {
          rows.push(editorDiagnostic(
            'NC_EDITOR_IMPORT_PROJECT_FORM_001',
            'error',
            lineIdx,
            at,
            item.length,
            `Invalid project import '${item}'. Project imports require module identifiers.`
          ));
        } else if (projectModules.size > 0 && !projectModules.has(item)) {
          rows.push(editorDiagnostic(
            'NC_EDITOR_IMPORT_PROJECT_UNRESOLVED_001',
            'error',
            lineIdx,
            at,
            item.length,
            `Project import '${item}' was not found near this workspace.`
          ));
        }
      }
      continue;
    }

    const parsed = parseFacadeImportStatement(trimmed);
    if (!parsed) {
      rows.push(editorDiagnostic(
        'NC_EDITOR_IMPORT_FORM_001',
        'error',
        lineIdx,
        Math.max(0, line.indexOf('use')),
        Math.max(1, line.trim().length),
        'Invalid import. Use use <family:item, item>;'
      ));
      continue;
    }

    if (parsed.domain === 'project') {
      for (const item of parsed.items) {
        const at = line.indexOf(item);
        if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(item)) {
          rows.push(editorDiagnostic(
            'NC_EDITOR_IMPORT_PROJECT_FORM_001',
            'error',
            lineIdx,
            at,
            item.length,
            `Invalid project import '${item}'. Project imports require module identifiers.`
          ));
        } else if (projectModules.size > 0 && !projectModules.has(item)) {
          rows.push(editorDiagnostic(
            'NC_EDITOR_IMPORT_PROJECT_UNRESOLVED_001',
            'error',
            lineIdx,
            at,
            item.length,
            `Project import '${item}' was not found near this workspace.`
          ));
        }
      }
      continue;
    }

    const domain = familyCatalog.get(parsed.domain);
    if (!domain) {
      rows.push(editorDiagnostic(
        'E_USE_DOMAIN_UNKNOWN_001',
        'error',
        lineIdx,
        Math.max(0, line.indexOf(parsed.domain)),
        parsed.domain.length,
        `Unknown import domain '${parsed.domain}'.`
      ));
      continue;
    }
    const items = domain.get(parsed.family);
    if (!items) {
      rows.push(editorDiagnostic(
        'E_USE_FAMILY_UNKNOWN_001',
        'error',
        lineIdx,
        Math.max(0, line.indexOf(parsed.family)),
        parsed.family.length,
        `Unknown import family '${parsed.domain}.${parsed.family}'.`
      ));
      continue;
    }
    for (const item of parsed.items) {
      if (!items.has(item)) {
        rows.push(editorDiagnostic(
          'E_USE_ITEM_UNKNOWN_001',
          'error',
          lineIdx,
          Math.max(0, line.indexOf(item)),
          item.length,
          `Unknown import item '${item}' in ${parsed.domain}.${parsed.family}.`
        ));
      }
    }
  }
  return rows;
}

function stringEscapeDiagnostics(text) {
  const rows = [];
  const lines = splitLines(text);
  let inBlock = false;
  for (let lineIdx = 0; lineIdx < lines.length; lineIdx += 1) {
    const line = lines[lineIdx];
    let cursor = 0;
    while (cursor < line.length) {
      if (inBlock) {
        const end = line.indexOf('*/', cursor);
        if (end < 0) {
          break;
        }
        cursor = end + 2;
        inBlock = false;
        continue;
      }
      if (line.startsWith('//', cursor)) {
        break;
      }
      if (line.startsWith('/*', cursor)) {
        inBlock = true;
        cursor += 2;
        continue;
      }
      const ch = line[cursor];
      if (ch !== '"' && ch !== "'") {
        cursor += 1;
        continue;
      }
      const quote = ch;
      cursor += 1;
      while (cursor < line.length) {
        const current = line[cursor];
        if (current === '\\') {
          const next = line[cursor + 1] || '';
          if (!VALID_STRING_ESCAPES.has(next)) {
            const raw = next ? `\\${next}` : '\\';
            rows.push(editorDiagnostic(
              'NC_EDITOR_STRING_ESCAPE_001',
              'error',
              lineIdx,
              cursor,
              next ? 2 : 1,
              `Invalid string escape '${raw}'`
            ));
          }
          cursor += next ? 2 : 1;
          continue;
        }
        cursor += 1;
        if (current === quote) {
          break;
        }
      }
    }
  }
  return rows;
}

function knownImportedLabels(text, catalog) {
  const source = catalog || defaultIntellisenseCatalog();
  const labels = new Set();
  for (const row of Array.isArray(source.symbols) ? source.symbols : []) {
    if (hasImportForEntry(text, row)) {
      labels.add(row.label);
    }
  }
  return labels;
}

function knownSymbolSets(text, catalog) {
  const parsed = parseSymbols(text);
  const typed = typedSymbolMap(text);
  const variables = new Set([...typed.keys(), ...parsed.variables.map((row) => row.name)]);
  const functions = new Set(parsed.functions.map((row) => row.name));
  const containers = new Set(parsed.containers.map((row) => row.name));
  const catalogLabels = new Set(catalogEntries(catalog).map((row) => row.label).filter(Boolean));
  for (const call of CALL_SIGNATURES) {
    catalogLabels.add(call.name);
  }
  for (const member of PRINT_MEMBERS) {
    catalogLabels.add(member);
  }
  return {
    variables,
    functions,
    containers,
    catalogLabels,
    importedLabels: knownImportedLabels(text, catalog),
    types: new Set([...TYPES, ...Object.keys(TYPE_DEFINITIONS), ...POINTER_TYPES])
  };
}

function unresolvedSymbolDiagnostics(text, catalog) {
  const rows = [];
  const known = knownSymbolSets(text, catalog);
  const lines = splitLines(text);
  const seen = new Set();
  for (let lineIdx = 0; lineIdx < lines.length; lineIdx += 1) {
    const line = lines[lineIdx];
    if (/^\s*(?:use\b|(?:const|type|stc|local|fn|fa|public|internal|private|layout)\s*:|static\s*\{|state\s*\{|stack\s*\{|abi\s*\{|[A-Za-z_][A-Za-z0-9_]*\s*::\s*(?:type|in)\b)/.test(line)) {
      continue;
    }

    const refRe = /@([A-Za-z_][A-Za-z0-9_]*)/g;
    let ref = refRe.exec(line);
    while (ref) {
      const name = ref[1];
      const start = ref.index;
      const after = line.slice(start + ref[0].length);
      const before = start > 0 ? line[start - 1] : '';
      if (
        name !== 'fn' &&
        before !== '[' &&
        !/^\s*>>/.test(after) &&
        !known.variables.has(name) &&
        !known.functions.has(name) &&
        !known.containers.has(name) &&
        !known.catalogLabels.has(name)
      ) {
        const key = `${lineIdx}:${start}:${name}`;
        if (!seen.has(key)) {
          seen.add(key);
          rows.push(editorDiagnostic(
            'NC_EDITOR_SYMBOL_UNRESOLVED_001',
            'error',
            lineIdx,
            start,
            ref[0].length,
            `Unresolved symbol '@${name}'.`
          ));
        }
      }
      ref = refRe.exec(line);
    }

    const callRe = /\b([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>]*>)?\s*\(/g;
    let call = callRe.exec(line);
    while (call) {
      const name = call[1];
      const start = call.index;
      const previous = start > 0 ? line[start - 1] : '';
      const declarationLike = /->/.test(line.slice(call.index)) && /^\s*[A-Za-z_][A-Za-z0-9_]*\s*\(/.test(line);
      if (
        previous !== '.' &&
        previous !== '@' &&
        !declarationLike &&
        !KEYWORDS.includes(name) &&
        !known.functions.has(name) &&
        !known.catalogLabels.has(name)
      ) {
        const key = `${lineIdx}:${start}:${name}`;
        if (!seen.has(key)) {
          seen.add(key);
          rows.push(editorDiagnostic(
            'NC_EDITOR_SYMBOL_UNRESOLVED_001',
            'error',
            lineIdx,
            start,
            name.length,
            `Unresolved function '${name}'.`
          ));
        }
      }
      call = callRe.exec(line);
    }
  }
  return rows;
}

function normalizeTypeName(typeName) {
  return (typeName || '')
    .replace(/\s+/g, '')
    .replace(/\bprintable\b/g, 'any')
    .replace(/\bstring\b/g, 'str')
    .replace(/\bboolean\b/g, 'bool')
    .replace(/\bfloat32\b/g, 'ft32')
    .replace(/\bf32\b/g, 'ft32')
    .replace(/\bfloat64\b/g, 'ft64')
    .replace(/\bf64\b/g, 'ft64')
    .trim();
}

function typeCompatible(expected, actual) {
  const want = normalizeTypeName(expected);
  const got = normalizeTypeName(actual);
  if (!want || !got || want === got || want === 'any' || want === 'T') {
    return true;
  }
  if (want === 'str' && (got === 'text' || got === 'string')) {
    return true;
  }
  if ((want === 'text' || want === 'string') && got === 'str') {
    return true;
  }
  const numeric = new Set(['int', 'u64', 'byte', 'sbyte', 'word', 'sword', 'dword', 'sdword', 'qword', 'sqword', 'addr', 'ft32', 'ft64']);
  if (numeric.has(want) && numeric.has(got)) {
    return true;
  }
  if (/^[A-Z]$/.test(want)) {
    return true;
  }
  return false;
}

function isNumericLikeType(typeName) {
  const clean = normalizeTypeName(typeName).replace(/\|null$/g, '');
  return new Set(['int', 'u64', 'byte', 'sbyte', 'word', 'sword', 'dword', 'sdword', 'qword', 'sqword', 'addr', 'ft32', 'ft64']).has(clean);
}

function isPointerLikeType(typeName) {
  const clean = normalizeTypeName(typeName);
  return /^(?:ptr|ptrmut|ref|refmut|site)<.+>/.test(clean) || clean === 'addr';
}

function incrementCompatible(typeName) {
  return isNumericLikeType(typeName) || isPointerLikeType(typeName);
}

function returnTypeFromSignature(signature) {
  const match = (signature || '').match(/->\s*([^;{]+)/);
  return match ? match[1].trim() : '';
}

function signatureParamTypes(signature) {
  const match = (signature || '').match(/\((.*)\)\s*->/);
  if (!match) {
    return [];
  }
  return splitTopLevelArguments(match[1]).map((arg) => {
    const pair = arg.text.match(/:\s*(.+)$/);
    return pair ? pair[1].trim() : 'any';
  });
}

function functionTypeFromSignature(signature) {
  const params = signatureParamTypes(signature).map((row) => cleanDeclaredType(row) || 'any');
  const ret = cleanDeclaredType(returnTypeFromSignature(signature) || 'void');
  return `fn(${params.join(', ')}) -> ${ret}`;
}

function inferDiagnosticExpressionType(expr, typeMap, localCalls = new Map()) {
  const direct = inferExpressionType(expr, typeMap);
  if (direct) {
    return direct;
  }
  const explicitFnRef = (expr || '').trim().match(/^@fn\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)$/);
  if (explicitFnRef && localCalls.has(explicitFnRef[1])) {
    return functionTypeFromSignature(localCalls.get(explicitFnRef[1]).signature);
  }
  const fnRef = (expr || '').trim().match(/^@?([A-Za-z_][A-Za-z0-9_]*)$/);
  if (fnRef && localCalls.has(fnRef[1])) {
    return functionTypeFromSignature(localCalls.get(fnRef[1]).signature);
  }
  if (fnRef) {
    const known = lookupCallSignature(fnRef[1]);
    if (known) {
      return functionTypeFromSignature(known.signature);
    }
  }
  const call = (expr || '').trim().match(/^([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>]*>)?\s*\(/);
  if (call) {
    const local = localCalls.get(call[1]);
    if (local) {
      return returnTypeFromSignature(local.signature);
    }
    const known = lookupCallSignature(call[1]);
    if (known) {
      return returnTypeFromSignature(known.signature);
    }
  }
  return '';
}

function typeDiagnostics(text) {
  const rows = [];
  const lines = splitLines(text);
  const typeMap = typedSymbolMap(text);
  const localCalls = localCallSignatureMap(text);

  for (let lineIdx = 0; lineIdx < lines.length; lineIdx += 1) {
    const line = lines[lineIdx];
    const decl = /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*>>\s*(.+?)\s*<-\s*(.+?);?\s*$/.exec(line);
    if (decl) {
      const expected = cleanDeclaredType(decl[2]);
      const actual = inferDiagnosticExpressionType(decl[3], typeMap, localCalls);
      if (actual && !typeCompatible(expected, actual)) {
        const start = Math.max(0, line.indexOf(decl[3].trim()));
        rows.push(editorDiagnostic(
          'NC_EDITOR_TYPE_MISMATCH_001',
          'error',
          lineIdx,
          start,
          Math.max(1, decl[3].trim().length),
          `Type mismatch: expected '${expected}', found '${actual}'.`
        ));
      }
    }

    const compound = /^\s*@?([A-Za-z_][A-Za-z0-9_]*)\s*(\+=|-=|\*=|\/=|%=|&=|\|=|\^=|<<=|>>=)\s*(.+?);?\s*$/.exec(line);
    if (compound && typeMap.has(compound[1])) {
      const expected = typeMap.get(compound[1]);
      const actual = inferDiagnosticExpressionType(compound[3], typeMap, localCalls);
      if (actual && !typeCompatible(expected, actual)) {
        const start = Math.max(0, line.indexOf(compound[3].trim()));
        rows.push(editorDiagnostic(
          'NC_EDITOR_TYPE_MISMATCH_001',
          'error',
          lineIdx,
          start,
          Math.max(1, compound[3].trim().length),
          `Type mismatch for '${compound[1]} ${compound[2]}': expected '${expected}', found '${actual}'.`
        ));
      }
    }

    const inc = /^\s*@?([A-Za-z_][A-Za-z0-9_]*)\s*(\+\+|--)\s*;?\s*$/.exec(line);
    if (inc && typeMap.has(inc[1]) && !incrementCompatible(typeMap.get(inc[1]))) {
      rows.push(editorDiagnostic(
        'NC_EDITOR_TYPE_OPERATOR_001',
        'error',
        lineIdx,
        Math.max(0, line.indexOf(inc[1])),
        inc[1].length,
        `Operator '${inc[2]}' expects numeric, addr or pointer type; '${inc[1]}' is '${typeMap.get(inc[1])}'.`
      ));
    }

    const callRe = /\b([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>]*>)?\s*\(/g;
    let call = callRe.exec(line);
    while (call) {
      const name = call[1];
      const open = line.indexOf('(', call.index);
      const close = findCallClose(line, open);
      const signature = localCalls.get(name) || lookupCallSignature(name);
      if (signature && close > open) {
        const args = splitTopLevelArguments(line.slice(open + 1, close));
        const paramTypes = signature.paramTypes || signatureParamTypes(signature.signature);
        args.forEach((arg, idx) => {
          const expected = paramTypes[idx] || 'any';
          const actual = inferDiagnosticExpressionType(arg.text, typeMap, localCalls);
          if (actual && !typeCompatible(expected, actual)) {
            rows.push(editorDiagnostic(
              'NC_EDITOR_TYPE_MISMATCH_001',
              'error',
              lineIdx,
              open + 1 + arg.start,
              Math.max(1, arg.end - arg.start),
              `Type mismatch for argument ${idx + 1} of '${name}': expected '${expected}', found '${actual}'.`
            ));
          }
        });
      }
      callRe.lastIndex = Math.max(callRe.lastIndex, close + 1);
      call = callRe.exec(line);
    }
  }
  return rows;
}

function removedInitializerDiagnostics(text) {
  const rows = [];
  const lines = splitLines(text);
  let inBlockComment = false;
  for (let lineIdx = 0; lineIdx < lines.length; lineIdx += 1) {
    const line = lines[lineIdx];
    let quote = '';
    let escaped = false;
    for (let index = 0; index < line.length; index += 1) {
      const ch = line[index];
      const next = line[index + 1] || '';
      if (inBlockComment) {
        if (ch === '*' && next === '/') {
          inBlockComment = false;
          index += 1;
        }
        continue;
      }
      if (quote) {
        if (escaped) escaped = false;
        else if (ch === '\\') escaped = true;
        else if (ch === quote) quote = '';
        continue;
      }
      if (ch === '/' && next === '/') break;
      if (ch === '/' && next === '*') {
        inBlockComment = true;
        index += 1;
        continue;
      }
      if (ch === '"' || ch === "'") {
        quote = ch;
        continue;
      }
      if (ch === ':' && next === '=') {
        rows.push(editorDiagnostic(
          'E_ASSIGN_INIT_REMOVED_001',
          'error',
          lineIdx,
          index,
          2,
          "':=' was removed. Use '<-' for initialization and writes."
        ));
        index += 1;
      }
    }
  }
  return rows;
}

function fnReferenceDiagnostics(text, catalog) {
  const rows = [];
  const symbols = parseSymbols(text);
  const localFunctions = new Set(symbols.functions.map((row) => row.name));
  const catalogLabels = new Set(catalogEntries(catalog).map((row) => row.label).filter(Boolean));
  const lines = splitLines(text);
  for (let lineIdx = 0; lineIdx < lines.length; lineIdx += 1) {
    const line = lines[lineIdx];
    const entryStart = line.search(/\bref\s+main\s*->>/);
    const direct = line.match(/\bref\s+main\s*->>\s*([A-Za-z_][A-Za-z0-9_\.]*)\s*;/);
    if (entryStart >= 0 && !direct) {
      rows.push(editorDiagnostic(
        'NC_EDITOR_FN_REFERENCE_001',
        'error',
        lineIdx,
        entryStart,
        Math.max(1, line.length - entryStart),
        'Broken entrypoint reference. Expected `ref main ->> function;`.'
      ));
      continue;
    }
    if (direct) {
      const target = direct[1];
      if (!target.includes('.') && !localFunctions.has(target) && !catalogLabels.has(target)) {
        rows.push(editorDiagnostic(
          'NC_EDITOR_FN_UNRESOLVED_001',
          'error',
          lineIdx,
          line.indexOf(target),
          target.length,
          `Unresolved entrypoint '${target}'.`
        ));
      }
    }
  }
  return rows;
}

function editorAssistDiagnostics(text, catalog, options = {}) {
  const out = [];
  out.push(...stringEscapeDiagnostics(text));
  out.push(...removedInitializerDiagnostics(text));
  out.push(...importDiagnostics(text, options));
  out.push(...fnReferenceDiagnostics(text, catalog));
  out.push(...typeDiagnostics(text));
  out.push(...unresolvedSymbolDiagnostics(text, catalog));
  for (const row of missingImportSuggestions(text, catalog)) {
    const symbol = row.symbol || row.label;
    const loc = findSymbolUseLocation(text, symbol);
    const preferred = preferredImportStatement(row);
    out.push({
      code: 'NC_EDITOR_IMPORT_HINT_001',
      severity: 'hint',
      line: loc.line,
      character: loc.character,
      length: loc.length,
      message: `Import candidate: ${preferred}`
    });
  }
  return out;
}

function lookupPointerBase(word) {
  const clean = (word || '').trim();
  return POINTER_BASE.find((row) => row.symbol === clean || row.title === clean || row.title.startsWith(`${clean}<`)) || null;
}

function lookupDiagnosticHelp(code) {
  const clean = (code || '').trim();
  return DIAGNOSTIC_HELP.find((row) => row.code === clean) || null;
}

function inRanges(ranges, start, end) {
  return ranges.some((range) => start < range.end && end > range.start);
}

function semanticStringRanges(line, lineIdx, start, quote, addSpan, protect) {
  let cursor = start + 1;
  let segmentStart = start;
  while (cursor < line.length) {
    const ch = line[cursor];
    if (ch === '\\') {
      if (segmentStart < cursor) {
        addSpan(lineIdx, segmentStart, cursor - segmentStart, 'string');
      }
      const length = cursor + 1 < line.length ? 2 : 1;
      addSpan(lineIdx, cursor, length, 'stringEscape');
      cursor += length;
      segmentStart = cursor;
      continue;
    }
    cursor += 1;
    if (ch === quote) {
      break;
    }
  }
  if (segmentStart < cursor) {
    addSpan(lineIdx, segmentStart, cursor - segmentStart, 'string');
  }
  protect(lineIdx, start, cursor);
  return cursor;
}

function collectLexicalSemanticSpans(lines, addSpan, protect) {
  let inBlock = false;
  for (let lineIdx = 0; lineIdx < lines.length; lineIdx += 1) {
    const line = lines[lineIdx];
    let cursor = 0;
    while (cursor < line.length) {
      if (inBlock) {
        const end = line.indexOf('*/', cursor);
        const stop = end >= 0 ? end + 2 : line.length;
        addSpan(lineIdx, cursor, stop - cursor, 'comment');
        protect(lineIdx, cursor, stop);
        cursor = stop;
        if (end < 0) {
          break;
        }
        inBlock = false;
        continue;
      }
      if (line.startsWith('//', cursor)) {
        addSpan(lineIdx, cursor, line.length - cursor, 'comment');
        protect(lineIdx, cursor, line.length);
        break;
      }
      if (line.startsWith('/*', cursor)) {
        const end = line.indexOf('*/', cursor + 2);
        const stop = end >= 0 ? end + 2 : line.length;
        addSpan(lineIdx, cursor, stop - cursor, 'comment');
        protect(lineIdx, cursor, stop);
        cursor = stop;
        inBlock = end < 0;
        continue;
      }
      if (line[cursor] === '"' || line[cursor] === "'") {
        cursor = semanticStringRanges(line, lineIdx, cursor, line[cursor], addSpan, protect);
        continue;
      }
      cursor += 1;
    }
  }
}

function pushFacadeImportSpans(line, lineIdx, addSpan, isBlocked) {
  const compact = /\b(use)\s*<\s*([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)?)\s*:\s*([^>]*)>/g;
  let compactMatch = compact.exec(line);
  while (compactMatch) {
    const start = compactMatch.index;
    if (!isBlocked(lineIdx, start, start + compactMatch[0].length)) {
      addSpan(lineIdx, start, 3, 'importKeyword');
      const pathStart = line.indexOf(compactMatch[2], start + 3);
      const pathParts = compactMatch[2].split('.');
      addSpan(lineIdx, pathStart, pathParts[0].length, pathParts.length > 1 ? 'importDomain' : 'importFamily');
      if (pathParts.length > 1) {
        addSpan(lineIdx, pathStart + pathParts[0].length + 1, pathParts[1].length, 'importFamily');
      }
      const rawItems = compactMatch[3];
      const itemStart = line.indexOf(rawItems, pathStart + compactMatch[2].length);
      let searchAt = 0;
      for (const item of rawItems.split(',').map((row) => row.trim()).filter(Boolean)) {
        const relative = rawItems.indexOf(item, searchAt);
        if (relative >= 0) {
          addSpan(lineIdx, itemStart + relative, item.length, 'importItem');
          searchAt = relative + item.length;
        }
      }
    }
    compactMatch = compact.exec(line);
  }
  const re = /\b(use)\s+([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\s*\{([^}]*)\}/g;
  let m = re.exec(line);
  while (m) {
    const start = m.index;
    if (!isBlocked(lineIdx, start, start + m[0].length)) {
      addSpan(lineIdx, start, 3, 'importKeyword');
      const pathStart = line.indexOf(m[2], start + 3);
      const pathParts = m[2].split('.');
      addSpan(lineIdx, pathStart, pathParts[0].length, 'importDomain');
      if (pathParts.length > 1) {
        addSpan(lineIdx, pathStart + pathParts[0].length + 1, m[2].length - pathParts[0].length - 1, 'importFamily');
      }
      const rawItems = m[3];
      const itemStart = line.indexOf(rawItems, pathStart + m[2].length);
      let searchAt = 0;
      for (const item of rawItems.split(',').map((row) => row.trim()).filter(Boolean)) {
        const relative = rawItems.indexOf(item, searchAt);
        if (relative >= 0) {
          addSpan(lineIdx, itemStart + relative, item.length, 'importItem');
          searchAt = relative + item.length;
        }
      }
    }
    m = re.exec(line);
  }
}

function pushLegacyImportSpans(line, lineIdx, addSpan, isBlocked) {
  const re = /^\s*(use|import)\s+(nC\.(?:runtime|high|compiler|system|tooling|inspect|ai)\.[A-Za-z_][A-Za-z0-9_.]*)/g;
  const m = re.exec(line);
  if (!m) {
    return;
  }
  const useStart = line.indexOf(m[1]);
  const pathStart = line.indexOf(m[2], useStart + m[1].length);
  if (!isBlocked(lineIdx, useStart, pathStart + m[2].length)) {
    addSpan(lineIdx, useStart, m[1].length, 'importKeyword');
    addSpan(lineIdx, pathStart, m[2].length, 'importHint');
  }
}

function normalizeSemanticSpans(spans) {
  const out = [];
  const priority = {
    comment: 0,
    string: 1,
    stringEscape: 0,
    modeRuntime: 0,
    modeCompiler: 0,
    modeSystem: 0,
    modeMachine: 0,
    modePrimitive: 0,
    modeKernel: 0,
    modeBasal: 0,
    printFunction: 2,
    pointerFunction: 2,
    colorFunction: 2,
    controlFlow: 2,
    loopControl: 2,
    kernelAddressKeyword: 2,
    kernelMmio8: 2,
    kernelMmio16: 2,
    kernelMmio32: 2,
    kernelMmio64: 2,
    kernelMmio128: 2,
    kernelMmio256: 2,
    kernelMmio512: 2
  };
  for (const span of spans
    .filter((row) => row.length > 0)
    .sort((a, b) => (
      a.line - b.line
      || a.startChar - b.startChar
      || (priority[a.tokenType] ?? 10) - (priority[b.tokenType] ?? 10)
      || b.length - a.length
    ))) {
    const last = out[out.length - 1];
    if (last && last.line === span.line && span.startChar < last.startChar + last.length) {
      continue;
    }
    out.push(span);
  }
  return out;
}

function semanticTokenSpans(text, context = {}) {
  const lines = splitLines(text);
  const spans = [];
  const protectedRanges = lines.map(() => []);
  const languageId = (context.languageId || '').toLowerCase();
  const modeFamily = context.modeFamily || detectModeFamily(context.fileName || '', text);
  const machineMode = languageId === 'nc-machine' || modeFamily === 'machine-low-level';
  const primitiveMode = languageId === 'nc-primitive' || modeFamily === 'primitive-low-level';
  const hardwareMode = languageId === 'nc-hardware' || modeFamily === 'hardware-low-level';
  const basalMode = languageId === 'nc-root' || modeFamily === 'root-basal';
  const kernelSystemMode = modeFamily === 'kernel-level' || modeFamily === 'system-level';

  const addSpan = (line, startChar, length, tokenType, tokenModifier) => {
    if (length > 0) {
      spans.push({ line, startChar, length, tokenType, tokenModifier });
    }
  };

  const protect = (line, start, end) => {
    if (end > start) {
      protectedRanges[line].push({ start, end });
    }
  };

  const isBlocked = (line, start, end) => inRanges(protectedRanges[line] || [], start, end);

  const pushMatches = (line, lineIdx, re, tokenType, tokenModifier) => {
    let m = re.exec(line);
    while (m) {
      const start = m.index;
      const end = start + m[0].length;
      if (!isBlocked(lineIdx, start, end)) {
        addSpan(lineIdx, start, m[0].length, tokenType, tokenModifier);
      }
      m = re.exec(line);
    }
  };

  collectLexicalSemanticSpans(lines, addSpan, protect);

  const symbols = parseSymbols(text);
  for (const row of symbols.containers) {
    if (!isBlocked(row.line, row.character, row.character + row.name.length)) {
      addSpan(row.line, row.character, row.name.length, 'container');
    }
  }
  for (const row of symbols.functions) {
    if (!isBlocked(row.line, row.character, row.character + row.name.length)) {
      addSpan(row.line, row.character, row.name.length, 'function');
    }
  }
  for (const row of symbols.variables) {
    if (!isBlocked(row.line, row.character, row.character + row.name.length)) {
      addSpan(row.line, row.character, row.name.length, 'variable');
    }
  }

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    pushFacadeImportSpans(line, i, addSpan, isBlocked);
    pushLegacyImportSpans(line, i, addSpan, isBlocked);
    pushMatches(line, i, /\b(?:use|import)\b/g, 'importKeyword');
    pushMatches(line, i, /\b(?:to\s+main|ref\s+main)\b/g, 'mainFamily');
    pushMatches(line, i, /\b(?:public|internal|private)\b/g, 'visibilityKeyword');
    pushMatches(line, i, /\bstc\s*:/g, 'stcSection');
    pushMatches(line, i, /\btype\s*:/g, 'typeSection');
    pushMatches(line, i, /\blocal\s*:/g, 'localSection');
    pushMatches(line, i, /\bconst\s*:/g, 'constSection');
    pushMatches(line, i, /\bfn\s*:/g, 'fnSection');
    pushMatches(line, i, /\bfa\s*:/g, 'faSection');
    pushMatches(line, i, /\b(?:static|stack|abi)(?=\s*\{)/g, 'section');
    pushMatches(line, i, /\btype(?=\s*(?:\[|\{))/g, 'typeSection');
    pushMatches(line, i, /\bin(?=\s*\()/g, 'fnSection');
    pushMatches(line, i, /\b(?:load|store|copy|fill|addr|size|align|offset|bytes|using)\b/g, 'section');
    pushMatches(line, i, /\bdev(?=\s*\()/g, 'devCall');
    pushMatches(line, i, /\breturn\b/g, 'returnKeyword');
    pushMatches(line, i, /@\(\s*[A-Za-z_][A-Za-z0-9_.]*\s*\)/g, 'atGroup');
    pushMatches(line, i, /@[A-Za-z_][A-Za-z0-9_.]*/g, 'atReference');
    pushMatches(line, i, />>/g, 'projectionOperator');
    pushMatches(line, i, /::/g, 'operator');
    pushMatches(line, i, /<-/g, 'materialWrite');
    pushMatches(line, i, /:=/g, 'diagnostic', 'error');
    pushMatches(line, i, /!>/g, 'verificationOperator');
    pushMatches(line, i, /\b(?:true|false)\b/g, 'boolLiteral');
    pushMatches(line, i, /\bmemRegion\b/g, 'memRegionType');
    pushMatches(line, i, /\b(?:p|ptr)\??(?:\s+mut)?(?=\s*<)/g, 'pointerType');
    if (!/\buse\s*(?:<|[A-Za-z_])/.test(line)) {
      pushMatches(line, i, /\b(?:array|list|queue|vector|slice|span|bytes|matrix|tensor|dinArray(?:\.binary|\.unbinary)?)\b/g, 'arrayType');
      pushMatches(line, i, /\bfn\s*\([^)]*\)\s*->\s*[A-Za-z_][A-Za-z0-9_<>,\s\?\.]*/g, 'advancedType');
      pushMatches(line, i, /<\s*(?:[A-Za-z_][A-Za-z0-9_]*|[0-9]+)(?:\s*,\s*(?:[A-Za-z_][A-Za-z0-9_]*|[0-9]+))*\s*>/g, 'genericType');
      pushMatches(line, i, /\b(?:addr|site|Result|Option|xy|xyz|ref\??(?:\s+mut)?(?!\s+main))\b/g, 'advancedType');
      pushMatches(line, i, /\b(?:int|u64|byte|bool|str|text|char|ft32|ft64|word|dword|qword|sbyte|sword|sdword|sqword|void|null)\b/g, 'basicType');
    }
    pushMatches(line, i, /(?:\+\+|--|\+=|-=|\*=|\/=|%=|&=|\|=|\^=|<<=|>>=|=)/g, 'operator');
    pushMatches(line, i, /\b(?:print|println)\b(?!\.)/g, 'printFunction');
    pushMatches(line, i, COLOR_FUNCTION_RE, 'colorFunction');
    pushMatches(line, i, POINTER_FUNCTION_RE, 'pointerFunction', 'unsafe');
    pushMatches(line, i, GENERIC_POINTER_FUNCTION_RE, 'pointerFunction', 'unsafe');
    if (kernelSystemMode) {
      pushMatches(line, i, /\bat\b/g, 'kernelAddressKeyword', 'unsafe');
      for (const row of KERNEL_MMIO_TOKEN_ROWS) {
        pushMatches(line, i, row.pattern, row.tokenType, 'unsafe');
      }
    }
    pushMatches(line, i, CONTROL_FLOW_RE, 'controlFlow');
    pushMatches(line, i, LOOP_CONTROL_RE, 'loopControl');
    pushMatches(line, i, /\bnC\.runtime\.[A-Za-z_][A-Za-z0-9_]*\b/g, 'runtimeFamily', 'runtimeOnly');
    pushMatches(line, i, /\b(?:lex|parse|ast|sema|nir|lower|backend|diagnostic)\b/g, 'compilerPhase');
    if (primitiveMode) {
      pushMatches(line, i, PRIMITIVE_ENTITY_RE, 'primitiveEntity', 'primitiveOnly');
    }
    if (hardwareMode) {
      pushMatches(line, i, PRIMITIVE_ENTITY_RE, 'hardwareEntity', 'primitiveOnly');
    }
    if (machineMode) {
      pushMatches(line, i, MACHINE_REGISTER_RE, 'machineRegister', 'machineOnly');
      pushMatches(line, i, MACHINE_INSTRUCTION_RE, 'machineInstruction', 'machineOnly');
    }
    if (basalMode) {
      pushMatches(line, i, BASAL_ENTITY_RE, 'basalEntity', 'sealed');
    }
    pushMatches(line, i, /\b(?:foreign\.c|foreign\.cpp|foreign\.rust|extern|abi|layout|ownership|nullability|symbol|repr)\b/g, 'ffiSymbol');
    pushMatches(line, i, /\b(?:capability\.[A-Za-z0-9_.]+|runtime\.[A-Za-z0-9_.]*capability[A-Za-z0-9_.]*)\b/g, 'capability');
    pushMatches(line, i, /\b(?:policy\.[A-Za-z0-9_.]+|runtime\.[A-Za-z0-9_.]*policy[A-Za-z0-9_.]*)\b/g, 'policy');
    for (const row of MODE_TOKEN_ROWS) {
      pushMatches(line, i, row.pattern, row.tokenType);
    }
    pushMatches(line, i, /\[@\]/g, 'basalEntity', 'sealed');
    pushMatches(line, i, /\[@kernel\]/g, 'kernelEntity', 'unsafe');
    pushMatches(line, i, /\b(?:raw|risk|hazard|rawPtr|rawAddr|rawSite|unchecked)\b/g, 'rawHazard', 'unsafe');
    pushMatches(line, i, /\b(?:copy(?:\.[A-Za-z_]+)?|clone(?:\.[A-Za-z_]+)?)\b/g, 'copyClone');
    pushMatches(line, i, /\bdestructure(?:\.[A-Za-z_]+)?\b/g, 'destructure');
    pushMatches(line, i, /\bconstructure(?:\.[A-Za-z_]+)?\b/g, 'constructure');
    pushMatches(line, i, /\b(?:0x[0-9A-Fa-f]+|0b[01]+|[0-9]+(?:\.[0-9]+)?)\b/g, 'number');
    pushMatches(line, i, /\bE_[A-Z0-9_]+\b/g, 'diagnostic');
  }

  return normalizeSemanticSpans(spans);
}

function lookupPrintMember(registry, word) {
  const rows = Array.isArray(registry.print_family) ? registry.print_family : [];
  const clean = (word || '').trim();
  const hit = rows.find((row) => row.name === clean);
  if (!hit) {
    return null;
  }
  return {
    title: clean,
    detail: `tier=${hit.tier} channel=${hit.channel} status=${hit.status}`
  };
}

function lookupRuntimeFamily(registry, word) {
  const clean = (word || '').trim();
  const rows = Array.isArray(registry.runtime_families) ? registry.runtime_families : [];
  if (!rows.includes(clean)) {
    return null;
  }
  return {
    title: clean,
    detail: 'Runtime API family from the base catalog.'
  };
}

function lookupCallSignature(name) {
  const clean = cleanCatalogToken(name);
  if (!clean) {
    return null;
  }
  return CALL_SIGNATURES.find((row) => row.name === clean) || null;
}

function splitTopLevelArguments(raw) {
  const text = raw || '';
  const args = [];
  let start = 0;
  let depth = 0;
  let quote = null;
  let escaped = false;

  const push = (end) => {
    const original = text.slice(start, end);
    const leading = original.match(/^\s*/)?.[0].length || 0;
    const trailing = original.match(/\s*$/)?.[0].length || 0;
    const trimmed = original.trim();
    if (trimmed) {
      args.push({
        text: trimmed,
        start: start + leading,
        end: Math.max(start + leading, end - trailing)
      });
    }
  };

  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (quote) {
      if (escaped) {
        escaped = false;
      } else if (ch === '\\') {
        escaped = true;
      } else if (ch === quote) {
        quote = null;
      }
      continue;
    }
    if (ch === '"' || ch === "'") {
      quote = ch;
      continue;
    }
    if (ch === '(' || ch === '[' || ch === '{' || ch === '<') {
      depth += 1;
      continue;
    }
    if (ch === ')' || ch === ']' || ch === '}' || ch === '>') {
      depth = Math.max(0, depth - 1);
      continue;
    }
    if (ch === ',' && depth === 0) {
      push(i);
      start = i + 1;
    }
  }
  push(text.length);
  return args;
}

function findCallClose(lineText, openIndex) {
  let depth = 0;
  let quote = null;
  let escaped = false;
  for (let i = openIndex; i < lineText.length; i += 1) {
    const ch = lineText[i];
    if (quote) {
      if (escaped) {
        escaped = false;
      } else if (ch === '\\') {
        escaped = true;
      } else if (ch === quote) {
        quote = null;
      }
      continue;
    }
    if (ch === '"' || ch === "'") {
      quote = ch;
      continue;
    }
    if (ch === '(') {
      depth += 1;
    } else if (ch === ')') {
      depth -= 1;
      if (depth === 0) {
        return i;
      }
    }
  }
  return lineText.length;
}

function callExpressionAtPosition(lineText, character) {
  const re = /\b([A-Za-z_][A-Za-z0-9_.]*)\s*\(/g;
  let match = re.exec(lineText || '');
  while (match) {
    const name = match[1];
    const open = match.index + match[0].lastIndexOf('(');
    const close = findCallClose(lineText, open);
    if (character >= match.index && character <= close) {
      const argsStart = open + 1;
      const argsText = lineText.slice(argsStart, close);
      return {
        name,
        nameStart: match.index,
        open,
        close,
        argsStart,
        args: splitTopLevelArguments(argsText),
        signature: lookupCallSignature(name)
      };
    }
    match = re.exec(lineText);
  }
  return null;
}

function typedSymbolMap(text) {
  const symbols = parseSymbols(text);
  const map = new Map();
  for (const variable of symbols.variables) {
    if (variable.name && variable.typeName) {
      map.set(variable.name, variable.typeName);
    }
  }
  for (const line of splitLines(text)) {
    const match = /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*>>\s*(.+?)\s*(?:<-|;)\s*/.exec(line);
    if (match) {
      map.set(match[1], match[2].trim());
    }
  }
  return map;
}

function lookupTypeDefinition(typeName) {
  const clean = cleanCatalogToken(typeName || '').replace(/<.*$/, '').trim();
  if (!clean) {
    return null;
  }
  if (TYPE_DEFINITIONS[clean]) {
    return TYPE_DEFINITIONS[clean];
  }
  const pointer = lookupPointerBase(clean);
  if (pointer) {
    return {
      title: pointer.title,
      detail: pointer.detail
    };
  }
  if (clean === 'list') {
    return {
      title: 'list<T>',
      detail: 'Typed dynamic list collection with indexed get/set and append operations.'
    };
  }
  if (clean === 'queue') {
    return {
      title: 'queue<T>',
      detail: 'Typed first-in-first-out collection for push/pop workflows.'
    };
  }
  return null;
}

function inferExpressionType(expr, types) {
  const text = (expr || '').trim();
  const ref = text.match(/^@?([A-Za-z_][A-Za-z0-9_]*)$/);
  if (ref && types.has(ref[1])) {
    return types.get(ref[1]);
  }
  if (/^".*"$/.test(text)) {
    return 'str';
  }
  if (/^'(?:\\.|[^'\\])'$/.test(text)) {
    return 'char';
  }
  if (/^(true|false)$/.test(text)) {
    return 'bool';
  }
  if (/^(?:0x[0-9A-Fa-f]+|0b[01]+|[0-9]+)$/.test(text)) {
    return 'int';
  }
  if (/^[0-9]+\.[0-9]+$/.test(text)) {
    return 'ft64';
  }
  const aggregate = text.match(/\b(listCreate|queueCreate|vectorCreate)\s*<\s*([^>]+)\s*>/);
  if (aggregate) {
    const family = aggregate[1].replace(/Create$/, '');
    return `${family}<${aggregate[2].trim()}>`;
  }
  const region = text.match(/\b(?:pRegion|regionFromPtr)\s*<\s*([^>]+)\s*>/);
  if (region) {
    return `memRegion<${region[1].trim()}>`;
  }
  return '';
}

function localCallSignatureMap(text) {
  const map = new Map();
  for (const line of splitLines(text)) {
    const match = /\b([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*->\s*([A-Za-z_][A-Za-z0-9_<>,\s\|\?\.]*)/.exec(line);
    if (!match) {
      continue;
    }
    const name = match[1];
    const params = [];
    const paramTypes = [];
    for (const arg of splitTopLevelArguments(match[2])) {
      const param = /^@?([A-Za-z_][A-Za-z0-9_]*)\s*(?:>>|:)\s*(.+)$/.exec(arg.text);
      if (param) {
        params.push(param[1]);
        paramTypes.push(param[2].trim());
      }
    }
    map.set(name, {
      name,
      signature: `${name}(${params.map((param, idx) => `${param}: ${paramTypes[idx] || 'any'}`).join(', ')}) -> ${match[3].trim()}`,
      params,
      paramTypes,
      detail: 'Local function signature from the visible file.'
    });
  }
  return map;
}

function argumentDirectObjectInfo(argText, types) {
  const text = (argText || '').trim();
  const inferred = inferExpressionType(text, types);
  const ref = text.match(/^@?([A-Za-z_][A-Za-z0-9_]*)$/);
  if (ref) {
    return {
      directObject: true,
      kind: text.startsWith('@') ? 'local-reference' : 'symbol-reference',
      subject: text,
      typeName: inferred || types.get(ref[1]) || '',
      detail: `${text} is passed directly as the object/value for this parameter.`
    };
  }
  if (/^".*"$/.test(text)) {
    return {
      directObject: true,
      kind: 'string-literal',
      subject: text,
      typeName: inferred || 'str',
      detail: 'String literal passed directly as this parameter value.'
    };
  }
  if (/^'(?:\\.|[^'\\])'$/.test(text)) {
    return {
      directObject: true,
      kind: 'char-literal',
      subject: text,
      typeName: inferred || 'char',
      detail: 'Character literal passed directly as this parameter value.'
    };
  }
  if (/^(true|false)$/.test(text)) {
    return {
      directObject: true,
      kind: 'bool-literal',
      subject: text,
      typeName: inferred || 'bool',
      detail: 'Boolean literal passed directly as this parameter value.'
    };
  }
  if (/^(?:0x[0-9A-Fa-f]+|0b[01]+|[0-9]+)$/.test(text)) {
    return {
      directObject: true,
      kind: 'int-literal',
      subject: text,
      typeName: inferred || 'int',
      detail: 'Integer literal passed directly as this parameter value.'
    };
  }
  if (/^\{.*\}$/.test(text)) {
    return {
      directObject: true,
      kind: 'object-literal',
      subject: text,
      typeName: inferred || 'object',
      detail: 'Structured object literal passed directly as this parameter value.'
    };
  }
  if (/^\[.*\]$/.test(text)) {
    return {
      directObject: true,
      kind: 'array-literal',
      subject: text,
      typeName: inferred || 'array',
      detail: 'Array/list literal passed directly as this parameter value.'
    };
  }
  return {
    directObject: false,
    kind: 'expression',
    subject: text,
    typeName: inferred,
    detail: inferred ? `Expression passed to this parameter. Inferred visible type: ${inferred}.` : 'Expression passed to this parameter.'
  };
}

function parameterHintTooltip(signature, parameter, argText, types, paramType) {
  const direct = argumentDirectObjectInfo(argText, types);
  const parts = [`Parameter ${parameter} in ${signature.signature}.`];
  if (paramType) {
    const definition = lookupTypeDefinition(paramType);
    parts.push(`Declared type: ${paramType}.`);
    if (definition) {
      parts.push(`${definition.title}: ${definition.detail}`);
    }
  }
  if (direct.directObject) {
    parts.push(`Direct object: ${direct.subject}.`);
    if (direct.typeName) {
      parts.push(`Visible object type: ${direct.typeName}.`);
    }
    parts.push(direct.detail);
  } else if (direct.detail) {
    parts.push(direct.detail);
  }
  if (signature.detail) {
    parts.push(signature.detail);
  }
  return compactWords(parts.join(' '), 50);
}

function inlayHintRows(text, options = {}) {
  const settings = {
    parameters: options.parameters !== false,
    types: options.types !== false
  };
  const lines = splitLines(text);
  const rows = [];
  const types = typedSymbolMap(text);
  const localSignatures = localCallSignatureMap(text);

  for (let lineIdx = 0; lineIdx < lines.length; lineIdx += 1) {
    const line = lines[lineIdx];
    if (settings.parameters) {
      const callRe = /\b([A-Za-z_][A-Za-z0-9_.]*)\s*\(/g;
      let match = callRe.exec(line);
      while (match) {
        const name = match[1];
        const open = match.index + match[0].lastIndexOf('(');
        const close = findCallClose(line, open);
        if (/^\s*->/.test(line.slice(close + 1))) {
          callRe.lastIndex = Math.max(callRe.lastIndex, close + 1);
          match = callRe.exec(line);
          continue;
        }
        const signature = lookupCallSignature(name) || localSignatures.get(name);
        if (signature && signature.params.length > 0) {
          const args = splitTopLevelArguments(line.slice(open + 1, close));
          for (let i = 0; i < args.length && i < signature.params.length; i += 1) {
            if (!/^[A-Za-z_][A-Za-z0-9_]*\s*:/.test(args[i].text)) {
              rows.push({
                line: lineIdx,
                character: open + 1 + args[i].start,
                label: `${signature.params[i]}:`,
                kind: 'parameter',
                tooltip: parameterHintTooltip(signature, signature.params[i], args[i].text, types, signature.paramTypes ? signature.paramTypes[i] : ''),
                signature: signature.signature,
                call: name,
                parameter: signature.params[i],
                argument: args[i].text,
                directObject: argumentDirectObjectInfo(args[i].text, types).directObject
              });
            }
          }
        }
        callRe.lastIndex = Math.max(callRe.lastIndex, close + 1);
        match = callRe.exec(line);
      }
    }

    if (settings.types) {
      const projection = /@?([A-Za-z_][A-Za-z0-9_]*)\s*>>\s*([^:;]+)\s*:\s*(.+?);?\s*$/.exec(line);
      if (projection) {
        const inferred = inferExpressionType(projection[3], types);
        if (inferred && inferred !== projection[2].trim()) {
          rows.push({
            line: lineIdx,
            character: line.length,
            label: `expr: ${inferred}`,
            kind: 'type',
            tooltip: lookupTypeDefinition(inferred)
              ? `${lookupTypeDefinition(inferred).title}: ${lookupTypeDefinition(inferred).detail}`
              : `Expression type: ${inferred}`,
            inferred
          });
        }
      }
    }
  }
  return rows;
}

function nccCommandPredictions(fileName = '${file}') {
  const base = path.basename(fileName || '${file}');
  const stem = base.includes('.') ? base.replace(/\.[^.]+$/, '') : '${fileBasenameNoExtension}';
  return NCC_COMMAND_PREDICTIONS.map((row) => ({
    label: row.label,
    detail: row.detail,
    command: row.command
      .replace(/\$\{file\}/g, fileName || '${file}')
      .replace(/\$\{fileBasenameNoExtension\}/g, stem)
  }));
}

function normalizeWord(raw) {
  return (raw || '').replace(/^@/, '').trim();
}

function detectLowLevelRenameTarget(fileName, text) {
  const family = detectModeFamily(fileName, text);
  if (family === 'root-basal') {
    return '.nb';
  }
  if (family === 'machine-low-level') {
    return '.nm';
  }
  if (family === 'primitive-low-level') {
    return '.nbb';
  }
  if (family === 'hardware-low-level') {
    return '.ncp';
  }
  return null;
}

module.exports = {
  KEYWORDS,
  TYPES,
  POINTER_TYPES,
  OFFICIAL_SECTIONS,
  IMPORT_SUGGESTIONS,
  FACADE_IMPORT_FAMILIES,
  MODE_MARKERS,
  PRINT_MEMBERS,
  VALID_STRING_ESCAPES,
  CALL_SIGNATURES,
  NCC_COMMAND_PREDICTIONS,
  TYPE_DEFINITIONS,
  MAIN_FAMILY_WORDS,
  NC_EXTENSIONS,
  PRIMITIVE_ENTITIES,
  BASAL_ENTITIES,
  MACHINE_REGISTERS_BY_ARCH,
  MACHINE_INSTRUCTIONS_BY_ARCH,
  MACHINE_INSTRUCTIONS,
  loadBaseRegistry,
  loadIntellisenseCatalog,
  loadStoudStdlibCatalog,
  defaultIntellisenseCatalog,
  catalogEntries,
  lookupCatalogEntry,
  lookupCatalogEntryAtLinePosition,
  preferredImportStatement,
  facadeImportScope,
  legacyImportStatement,
  hasFacadeImport,
  parseFacadeImportStatement,
  mergeFacadeImportStatement,
  hasImportForEntry,
  isNcLanguageId,
  sanitizeProfile,
  sanitizeLspMode,
  supportedNcExtension,
  diagnosticLocationFromMessage,
  diagnosticCodeFromMessage,
  detectLowLevelByContent,
  detectModeMarkers,
  detectCompactKernelSurface,
  detectModeFamily,
  parseSymbols,
  findWordRangeInLine,
  collectOccurrences,
  collectCodeOccurrences,
  completionPool,
  lowLevelCompletionRows,
  facadeImportCompletionRows,
  isProjectImportCompletionContext,
  importedFunctionCompletionRows,
  parseStructFields,
  structFieldCompletionRows,
  entrypointCompletionRows,
  projectModuleCompletionRowsFromPaths,
  hasImport,
  missingImportSuggestions,
  isBackendPreludeSymbol,
  isBackendPreludeEntry,
  compilerTextDiagnostics,
  isTokenBoundaryText,
  completedTokenBeforeBoundary,
  autoImportAllowed,
  autoImportCandidateAtOffset,
  expandCompactEmptyBlockLine,
  editorAssistDiagnostics,
  lookupPointerBase,
  lookupDiagnosticHelp,
  semanticTokenSpans,
  lookupPrintMember,
  lookupRuntimeFamily,
  lookupCallSignature,
  splitTopLevelArguments,
  callExpressionAtPosition,
  typedSymbolMap,
  lookupTypeDefinition,
  inferExpressionType,
  localCallSignatureMap,
  argumentDirectObjectInfo,
  inlayHintRows,
  nccCommandPredictions,
  normalizeWord,
  detectLowLevelRenameTarget
};
