# Runtime Window/Canvas Win32

Status: closed for the `x86_64-pc-windows-msvc` PE64 lane.

## Surface

Normal code imports the complete UI group with:

```nC
use stoud.ui { canvas, input, window };
```

The compact facade expands to existing typed runtime families:

- `window` -> `nC.runtime.terminal`
- `canvas` -> `nC.runtime.render`
- `input` -> `nC.runtime.input`

This keeps the 90-family base stable while using the native `terminal` extension.
The operational runtime catalog has 93 families: 90 base families plus `math`,
`terminal` and `json`.

## Window Contract

`windowOpen` creates a real top-level Win32 window through `CreateWindowExW`.
Its nC `str` title is validated as UTF-8 and converted losslessly to UTF-16;
invalid input fails instead of being replacement-decoded. `windowSetTitle`
uses the same conversion and `SetWindowTextW`. The returned `int` is an opaque
pointer-width `HWND`; callers must not perform arithmetic on it. `windowPoll`
drains pending messages for its own `HWND` except `WM_KEYDOWN` and `WM_CHAR`,
which remain owned by the typed input functions. It probes the thread-level
`WM_QUIT` message separately because that message has no `HWND`, and returns
`false` after close, destroy or quit. A normal `WM_DESTROY` is delivered
synchronously by `DestroyWindow`; the final `IsWindow` check observes that
native invalidation even though it was never a queued message.
`windowWaitEvent(handle, timeoutMs)` waits without polling through
`MsgWaitForMultipleObjectsEx(QS_ALLINPUT, MWMO_INPUTAVAILABLE)` and leaves the
queue untouched. `windowClose` transfers the native handle to `DestroyWindow`.

Canvas v3 makes the process DPI-aware before the first native window is
created. The host requests per-monitor-v2 awareness and falls back to legacy
process awareness when Windows has already fixed the process mode.

```text
windowDpi(int handle) -> int
windowRefreshHz(int handle) -> int
windowState(int handle) -> int
windowSetState(int handle, int state) -> bool
```

`windowDpi` returns the effective monitor DPI (`96` is 100 percent) and zero
for an invalid window. Both the Rust host and PE64 `windowRefreshHz` paths
resolve the nearest monitor on every call with `MonitorFromWindow`, obtain its
device name through `GetMonitorInfoW`, create that display DC and read
`VREFRESH`. They are therefore not fixed to the monitor used at window creation
and can follow a window moved between displays. A missing or unusable driver
value falls back to 60 Hz.
`windowState` returns `0` for normal/restored, `1` for minimized, `2` for
maximized and `-1` for an invalid window. `windowSetState` accepts exactly the
same three non-negative values and maps them to restore, minimize and maximize.

## Canvas Contract

Canvas v2 adds the following typed frame surface without removing the original
immediate-mode calls:

```text
canvasBeginFrame(int handle, int color) -> bool
canvasRoundRect(int handle, int x, int y, int w, int h, int radius, int color) -> bool
canvasTextStyled(int handle, int x, int y, str text, int color, int size, int weight) -> bool
canvasBackdropBlur(int handle, int radius) -> bool
canvasPresent(int handle) -> bool
```

`canvasBeginFrame` creates or reuses a memory DC and compatible bitmap, clears
it, and makes it the target of the v2 drawing calls. `canvasPresent` copies that
surface to the client DC with `BitBlt`; without an active frame it retains the
v1 `GdiFlush` behavior. Coordinates and dimensions are device pixels. Colors
are `0xRRGGBB` converted to Win32 `COLORREF`. Styled text uses `CreateFontW`
with a Segoe UI face, positive pixel size and a weight clamped by the host to
the Win32 1..1000 range.

The blur is an honest GDI approximation rather than a Gaussian shader: it
downsamples the current backbuffer into a smaller compatible bitmap and scales
it back with `StretchBlt` in `HALFTONE` mode. `radius` controls the downsample
factor (clamped to keep a non-empty surface). It therefore gives the intended
modal/backdrop diffusion but not a mathematically exact radius kernel.

The Windows host runtime owns one resize-aware backbuffer per `HWND`, so a
resized or multi-window host session remains coherent. Canvas v3 also makes the
self-contained PE64 lane resize-aware: `canvasBeginFrame` compares the current
client extent with the property-backed bitmap and recreates the DC/bitmap when
needed. The window remains a normal `WS_OVERLAPPEDWINDOW`, so native move,
resize, minimize, maximize and restore behavior stays under the window manager.
There is deliberately no artificial minimum client size: the physical
backbuffer follows every positive client extent, while a minimized or other
zero-area client makes `canvasBeginFrame` return `false` without discarding the
last valid surface.

## Canvas V3 Contract

Canvas v3 extends, rather than replaces, every v1/v2 call:

```text
canvasSetQuality(int handle, int quality) -> bool
canvasSetFrameMode(int handle, int mode, int targetFps) -> bool
canvasFrameDeltaMs(int handle) -> int
canvasPollResize(int handle) -> bool
canvasSetViewport(int handle, int logicalW, int logicalH, int mode) -> bool
canvasPointerX(int handle) -> int
canvasPointerY(int handle) -> int
canvasTextCentered(int handle, int x, int y, int w, int h,
                   str text, int color, int size, int weight) -> bool
canvasShadowRoundRect(int handle, int x, int y, int w, int h,
                      int radius, int color, int spread) -> bool
canvasRoundRectAlpha(int handle, int x, int y, int w, int h,
                     int radius, int color, int alpha) -> bool
canvasRoundRectGradient(int handle, int x, int y, int w, int h,
                        int radius, int colorA, int colorB, int vertical) -> bool
canvasBackdropBlurV3(int handle, int radius, int passes, int tintColor) -> bool
```

Quality is `0` balanced, `1` high (the default) or `2` ultra. On the host this
selects font rasterization, the blur resampling path, a maximum of 2/4/8 blur
passes and 32/64/128 physical pixels of shadow spread respectively. Invalid
quality values are diagnosed rather than silently accepted.

Frame mode is `0` uncapped, `1` fixed-rate or `2` refresh-paced. Uncapped mode
never sleeps. Fixed mode requires `targetFps` in `1..1000`. Refresh-paced mode
uses that target when positive and otherwise uses `windowRefreshHz`. Pacing is
a monotonic-clock deadline plus `Sleep`; PE64 uses
`QueryPerformanceCounter/Frequency`, a coarse sleep and a short deadline spin,
so 60/120/144/240 Hz do not collapse to integer milliseconds. It is not a DWM
swap-chain or hardware vsync promise. `canvasFrameDeltaMs` is zero until two successful presentations
and after a mode reset, then reports the elapsed whole milliseconds between
successful `canvasPresent` calls.

`canvasPollResize` returns `true` exactly once for each observed client-size
change and updates its baseline. The next `canvasBeginFrame` owns physical
backbuffer reallocation. A minimized zero-area client cannot begin a frame and
returns `false` without corrupting the previous surface.

The viewport keeps application geometry independent of display resolution:

- mode `0` stretches independently on x/y;
- mode `1` uses `min(client/logical)` and centers a letterboxed image;
- mode `2` uses `max(client/logical)` and centers a cropped fill image.

An active viewport transforms coordinates, extents, line widths, radii, font
sizes and shadow spread for all v2/v3 primitives. The backbuffer itself remains
at physical client resolution, including 3840x2160; presentation does not scale
it a second time. `canvasPointerX/Y` apply the exact inverse transform and do
not clamp, so pointer values over letterbox bars may be outside logical bounds.
Portable code must use the normal `canvasBeginFrame`/draw/`canvasPresent` cycle;
the compact PE thunk applies its GDI mapping mode when the frame begins.

Physical backbuffer operations temporarily select `MM_TEXT` and explicitly
reset the viewport origin to `(0,0)`. Win32 does not reset that origin when the
mapping mode changes. The explicit reset keeps clears, presentation and both
blur paths aligned after a fit/fill viewport has introduced letterbox offsets;
the saved logical mapping is restored before further logical drawing.

`canvasTextCentered` uses a single line with horizontal and vertical centering,
no mnemonic-prefix interpretation, positive pixel size after viewport scaling,
and weight clamped to `1..1000`.

Host shadows and rounded alpha/gradients are premultiplied 32-bit DIB
compositions through `AlphaBlend`. PE64 uses real `AlphaBlend` for rounded
alpha/shadow and `GradientFill` under a rounded GDI clipping region; it does not
substitute opaque layered rectangles. Its solid alpha source is a 1x1
compatible bitmap expanded over the destination, so it does not allocate a
full-client temporary surface per panel. Shadow colors use `0xAARRGGBB`; an omitted
high alpha byte selects the visible default alpha 96. `spread` is capped by the
quality profile. `canvasRoundRectAlpha` instead takes explicit alpha `0..255`
and a normal `0xRRGGBB` color. Gradients are fully opaque and `vertical` must be
`0` (horizontal) or `1` (vertical).

`canvasBackdropBlurV3` performs iterative downsample/upsample diffusion on the
current frame. Radius is non-negative and capped at 256 physical pixels;
passes must be positive and are capped by quality. Both Windows paths derive
the physical downsample factor as
`1 + ceil(radius / (effectivePasses * 2))`, capped to 12/8/5 for
balanced/high/ultra. `tintColor` is `0xAARRGGBB`: alpha zero means no tint.
Both paths alpha-composite the tint. This is materially smoother than the
single-pass v2 blur but remains a deterministic GDI approximation, not a GPU
Gaussian kernel.

Host DIB compositions reject surfaces above 67,108,864 pixels to bound one
temporary allocation. Canvas text is strictly validated UTF-8 and converted
losslessly to UTF-16 code units for `TextOutW`/`DrawTextW`; invalid UTF-8 and
embedded NUL fail explicitly and are never replacement-decoded. Fonts are
cached by physical size, weight, quality and effective DPI, bounded to 128
entries, invalidated on quality/DPI changes and deleted after the last window
closes. This supports names and units such as `Configuración`, `Dónde`, `Jöns`
and `°C`, but remains GDI text rather than a complex-script shaping engine.
Window titles follow the same strict conversion; `nC – Tabla Periódica V3` is
a retained non-ASCII regression case.
DirectWrite, HDR color and GPU effects belong to a future renderer backend
rather than being claimed by Canvas v3.

## Input Contract

`mouseX` and `mouseY` convert screen coordinates to client coordinates.
`mouseDown` and `keyDown` read the high bit returned by `GetAsyncKeyState`.
Mouse buttons use Win32 virtual-key values (`1` left, `2` right, `4` middle).
`windowReadKey(handle)` removes one `WM_KEYDOWN` message for that window and
passes it to `TranslateMessage` before returning its `wParam` virtual-key code.
`windowReadChar(handle)` then removes one generated `WM_CHAR` and returns its
UTF-16 code unit; both functions return zero when no matching event is queued.
This separates navigation keys from editable text. `keyDown` remains the
continuous state path for movement and held controls.

## Backend Contract

The internal PE64 linker removes runtime leaf names from the loader import set,
emits Win64 ABI thunks as real x86_64 bytes, and builds grouped import descriptors:

- `USER32.dll`: wide windows/titles, monitor selection, drained messages,
  input, DC access, `FillRect` and per-window canvas properties.
- `GDI32.dll`: compatible DC/bitmap allocation, blit/stretch, brushes, fonts,
  rounded rectangles, text, lines, ellipses, monitor-specific display DCs and
  flush.
- `MSIMG32.dll`: `AlphaBlend` and `GradientFill`.
- `KERNEL32.dll`: module/process support, QPC frame pacing and `GetTickCount64`
  backing `runtime.time.monotonicMs() -> int` in PE64.

The thunks preserve nonvolatile registers, reserve Win64 shadow space, align the
stack before calls and place arguments five through twelve in ABI stack slots.
The generated image retains `.reloc`, ASLR/NX flags and normal PE IAT metadata.
If `windowOpen` is present without console-input calls, the final image uses
`IMAGE_SUBSYSTEM_WINDOWS_GUI`; terminal programs and mixed programs that call
`readInt`/`readLine` retain `IMAGE_SUBSYSTEM_WINDOWS_CUI`.

## Host Runtime

`ncc run` invokes raw Win32 host bindings on Windows. On non-Windows hosts the
same calls fail with `E_RUNTIME_WINDOW_PLATFORM_001`; no terminal or in-memory
window is reported as a successful graphical operation.

## Reproducible Smoke

```text
ncc check examples/12_apps/win32_window_canvas_smoke.n
ncc build examples/12_apps/win32_window_canvas_smoke.n --out-dir .ncc --cc nC.win64 --target x86_64-pc-windows-msvc --json
.ncc/bin/win32_window_canvas_smoke.exe
```
