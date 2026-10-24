# Runtime Terminal

`nC.runtime.terminal` owns terminal control and the native window handle
surface. Normal graphical code may use the compact facade:

```nC
use stoud.ui { canvas, input, window };
```

Direct terminal operations belong to the hosted backend prelude and need no
source import:

```nC
clear();
moveCursor(1, 1);
hideCursor();
showCursor();
```

The direct module and the compact `window` facade export the same complete
native-window surface, including `windowWaitEvent`.

## Terminal Operations

| Function | Return | Effect |
| --- | --- | --- |
| `clear()` | `bool` | Clears the active terminal. |
| `moveCursor(row, column)` | `bool` | Moves the cursor to a one-based cell. |
| `hideCursor()` / `showCursor()` | `bool` | Changes cursor visibility. |
| `enterRawMode()` / `exitRawMode()` | `bool` | Enters or leaves unbuffered terminal input. |
| `readKeyNonBlocking()` | `int` | Returns a key code without blocking. |

## Native Window Operations

| Function | Return | Effect |
| --- | --- | --- |
| `windowOpen(title, width, height)` | `int` | Creates and shows a top-level window. |
| `windowShow(handle)` / `windowHide(handle)` | `bool` | Changes native visibility. |
| `windowPoll(handle)` | `bool` | Drains non-text messages; false means the window closed. |
| `windowWaitEvent(handle, timeoutMs)` | `bool` | Sleeps until input is queued or the finite timeout expires. |
| `windowClose(handle)` | `bool` | Destroys the native window. |
| `windowIsOpen(handle)` | `bool` | Tests whether the handle is still valid. |
| `windowIsFocused(handle)` | `bool` | Tests whether the handle is the current foreground window. |
| `windowSetTitle(handle, title)` | `bool` | Replaces the native title. |
| `windowSetSize(handle, width, height)` | `bool` | Resizes the outer window. |
| `windowWidth(handle)` / `windowHeight(handle)` | `int` | Reads client-area dimensions. |
| `windowFocus(handle)` | `bool` | Requests foreground focus. |

The compact `input` facade also exposes `windowReadKey(handle) -> int` and
`windowReadChar(handle) -> int`. The first consumes one discrete native
`WM_KEYDOWN` event, calls `TranslateMessage`, and returns its virtual-key code.
The second consumes the resulting UTF-16 `WM_CHAR` code unit. Both return zero
when no matching event is queued. Use `keyDown(code)` for held/not-held state.

`windowWaitEvent` uses `MsgWaitForMultipleObjectsEx` with `QS_ALLINPUT` and
`MWMO_INPUTAVAILABLE`; it does not consume the event. `timeoutMs` must be in
`0..4294967295`, excluding Win32's reserved `INFINITE` value.

The `int` returned by `windowOpen` is an opaque pointer-width handle. It must
not be used in arithmetic. Dimensions are pixels and must be positive.

## Backend And Errors

The seven direct terminal operations are materialized by LLVM/MCT without a C
provider. Win32 uses `Get/SetConsoleMode`, `WriteFile` and
`ReadConsoleInputW`; Linux/Android and Darwin use their `termios`, `poll`,
`read` and `write` ABIs. WASI supports ANSI output; because WASI defines no raw
TTY contract, `enterRawMode` returns `false`, `exitRawMode` returns `true` and
`readKeyNonBlocking` returns `-1`.

For `x86_64-pc-windows-msvc`, the internal PE64 linker emits Win64 ABI thunks
and real `USER32.dll`, `GDI32.dll` and `KERNEL32.dll` imports. `ncc run` uses
the same Win32 APIs on Windows. A hosted run on another OS reports
`E_RUNTIME_WINDOW_PLATFORM_001`; invalid dimensions report
`E_RUNTIME_WINDOW_RANGE_001`; invalid native text reports
`E_RUNTIME_WINDOW_TEXT_001`.

Native-window-only executables use the PE GUI subsystem and open without an
auxiliary console. A program that requires console input keeps the console
subsystem even if it also creates a native window.

The complete drawing and input contracts are specified in
`docs/spec/runtime/runtime_win32_window_canvas.md`.

## Evidence

- Terminal fixture: `examples/runtime/families/terminal_basic.n`.
- Window/canvas fixture: `examples/12_apps/win32_window_canvas_smoke.n`.
- Strict check: `ncc runtime check examples/runtime/families/terminal_basic.n --strict --json`.
- Native build: `ncc build examples/12_apps/win32_window_canvas_smoke.n --cc nC.win64 --target x86_64-pc-windows-msvc`.
