#[cfg(any(windows, test))]
fn utf8_bytes_to_utf16_strict(bytes: &[u8], name: &str) -> Result<Vec<u16>, String> {
    let value = std::str::from_utf8(bytes)
        .map_err(|_| format!("E_RUNTIME_WINDOW_UTF8_001: {name} is not valid UTF-8"))?;
    if value.chars().any(|ch| ch == '\0') {
        return Err(format!(
            "E_RUNTIME_WINDOW_TEXT_001: {name} contains an embedded NUL"
        ));
    }
    Ok(value.encode_utf16().collect())
}

#[cfg(windows)]
fn utf8_to_utf16_strict(value: &str, name: &str) -> Result<Vec<u16>, String> {
    utf8_bytes_to_utf16_strict(value.as_bytes(), name)
}

#[cfg(any(windows, test))]
fn validated_window_wait_timeout(timeout_ms: i64) -> Result<u32, String> {
    if !(0..i64::from(u32::MAX)).contains(&timeout_ms) {
        return Err("E_RUNTIME_WINDOW_WAIT_001: timeoutMs must be in 0..4294967295".into());
    }
    Ok(timeout_ms as u32)
}

// A bounded pump keeps a continuous WM_MOUSEMOVE/WM_PAINT stream from
// monopolising the application thread. The next loop iteration resumes the
// queue immediately, while rendering, resize handling and shutdown still get
// a scheduling point.
#[cfg(any(windows, test))]
const WINDOW_POLL_MESSAGE_BUDGET: usize = 256;

#[cfg(any(windows, test))]
#[derive(Debug, Default)]
struct PointerPairCache {
    pending_y: Option<i64>,
}

#[cfg(any(windows, test))]
impl PointerPairCache {
    fn store_y(&mut self, y: i64) {
        self.pending_y = Some(y);
    }

    fn take_y(&mut self) -> Option<i64> {
        self.pending_y.take()
    }
}

#[cfg(windows)]
mod imp {
    use super::{PointerPairCache, WINDOW_POLL_MESSAGE_BUDGET};
    use std::collections::HashMap;
    use std::ffi::{c_char, c_void};
    use std::mem::{size_of, zeroed};
    use std::ptr::{null, null_mut};
    use std::sync::{Mutex, OnceLock};
    use std::thread;
    use std::time::{Duration, Instant};

    type Handle = *mut c_void;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct MonitorInfoExW {
        size: u32,
        monitor: Rect,
        work: Rect,
        flags: u32,
        device: [u16; 32],
    }

    const _: [(); 104] = [(); size_of::<MonitorInfoExW>()];

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Msg {
        hwnd: Handle,
        message: u32,
        w_param: usize,
        l_param: isize,
        time: u32,
        point: Point,
        private: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct BitmapInfoHeader {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bit_count: u16,
        compression: u32,
        size_image: u32,
        x_pels_per_meter: i32,
        y_pels_per_meter: i32,
        colors_used: u32,
        colors_important: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct RgbQuad {
        blue: u8,
        green: u8,
        red: u8,
        reserved: u8,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct BitmapInfo {
        header: BitmapInfoHeader,
        colors: [RgbQuad; 1],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct BlendFunction {
        operation: u8,
        flags: u8,
        source_constant_alpha: u8,
        alpha_format: u8,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleA(module: *const c_char) -> Handle;
    }

    #[link(name = "user32")]
    extern "system" {
        fn CreateWindowExW(
            ex_style: u32,
            class_name: *const u16,
            title: *const u16,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: Handle,
            menu: Handle,
            instance: Handle,
            param: *mut c_void,
        ) -> Handle;
        fn ShowWindow(hwnd: Handle, command: i32) -> i32;
        fn UpdateWindow(hwnd: Handle) -> i32;
        fn PeekMessageA(msg: *mut Msg, hwnd: Handle, min: u32, max: u32, remove: u32) -> i32;
        fn PeekMessageW(msg: *mut Msg, hwnd: Handle, min: u32, max: u32, remove: u32) -> i32;
        fn TranslateMessage(msg: *const Msg) -> i32;
        fn DispatchMessageA(msg: *const Msg) -> isize;
        fn MsgWaitForMultipleObjectsEx(
            count: u32,
            handles: *const Handle,
            milliseconds: u32,
            wake_mask: u32,
            flags: u32,
        ) -> u32;
        fn DestroyWindow(hwnd: Handle) -> i32;
        fn IsWindow(hwnd: Handle) -> i32;
        fn GetForegroundWindow() -> Handle;
        fn SetWindowTextW(hwnd: Handle, title: *const u16) -> i32;
        fn SetWindowPos(
            hwnd: Handle,
            after: Handle,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
        fn SetForegroundWindow(hwnd: Handle) -> i32;
        fn GetClientRect(hwnd: Handle, rect: *mut Rect) -> i32;
        fn GetDC(hwnd: Handle) -> Handle;
        fn ReleaseDC(hwnd: Handle, dc: Handle) -> i32;
        fn FillRect(dc: Handle, rect: *const Rect, brush: Handle) -> i32;
        fn GetCursorPos(point: *mut Point) -> i32;
        fn ScreenToClient(hwnd: Handle, point: *mut Point) -> i32;
        fn GetAsyncKeyState(key: i32) -> i16;
        fn SetProcessDpiAwarenessContext(value: Handle) -> i32;
        fn SetProcessDPIAware() -> i32;
        fn GetDpiForWindow(hwnd: Handle) -> u32;
        fn IsIconic(hwnd: Handle) -> i32;
        fn IsZoomed(hwnd: Handle) -> i32;
        fn MonitorFromWindow(hwnd: Handle, flags: u32) -> Handle;
        fn GetMonitorInfoW(monitor: Handle, info: *mut MonitorInfoExW) -> i32;
        fn DrawTextW(dc: Handle, text: *const u16, count: i32, rect: *mut Rect, format: u32)
            -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateSolidBrush(color: u32) -> Handle;
        fn CreatePen(style: i32, width: i32, color: u32) -> Handle;
        fn CreateCompatibleDC(dc: Handle) -> Handle;
        fn CreateDCW(
            driver: *const u16,
            device: *const u16,
            output: *const u16,
            init_data: *const c_void,
        ) -> Handle;
        fn DeleteDC(dc: Handle) -> i32;
        fn CreateCompatibleBitmap(dc: Handle, width: i32, height: i32) -> Handle;
        fn CreateDIBSection(
            dc: Handle,
            info: *const BitmapInfo,
            usage: u32,
            bits: *mut *mut c_void,
            section: Handle,
            offset: u32,
        ) -> Handle;
        fn BitBlt(
            dst: Handle,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            src: Handle,
            src_x: i32,
            src_y: i32,
            operation: u32,
        ) -> i32;
        fn StretchBlt(
            dst: Handle,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            src: Handle,
            src_x: i32,
            src_y: i32,
            src_width: i32,
            src_height: i32,
            operation: u32,
        ) -> i32;
        fn SetStretchBltMode(dc: Handle, mode: i32) -> i32;
        fn DeleteObject(object: Handle) -> i32;
        fn SelectObject(dc: Handle, object: Handle) -> Handle;
        fn SetTextColor(dc: Handle, color: u32) -> u32;
        fn SetBkMode(dc: Handle, mode: i32) -> i32;
        fn TextOutW(dc: Handle, x: i32, y: i32, text: *const u16, len: i32) -> i32;
        fn MoveToEx(dc: Handle, x: i32, y: i32, previous: *mut Point) -> i32;
        fn LineTo(dc: Handle, x: i32, y: i32) -> i32;
        fn Ellipse(dc: Handle, left: i32, top: i32, right: i32, bottom: i32) -> i32;
        fn RoundRect(
            dc: Handle,
            left: i32,
            top: i32,
            right: i32,
            bottom: i32,
            ellipse_width: i32,
            ellipse_height: i32,
        ) -> i32;
        fn CreateFontW(
            height: i32,
            width: i32,
            escapement: i32,
            orientation: i32,
            weight: i32,
            italic: u32,
            underline: u32,
            strike_out: u32,
            char_set: u32,
            out_precision: u32,
            clip_precision: u32,
            quality: u32,
            pitch_and_family: u32,
            face: *const u16,
        ) -> Handle;
        fn GetDeviceCaps(dc: Handle, index: i32) -> i32;
        fn GdiFlush() -> i32;
    }

    #[link(name = "msimg32")]
    extern "system" {
        fn AlphaBlend(
            dst: Handle,
            dst_x: i32,
            dst_y: i32,
            dst_width: i32,
            dst_height: i32,
            src: Handle,
            src_x: i32,
            src_y: i32,
            src_width: i32,
            src_height: i32,
            blend: BlendFunction,
        ) -> i32;
    }

    const WS_OVERLAPPEDWINDOW: u32 = 0x00cf_0000;
    const WS_VISIBLE: u32 = 0x1000_0000;
    const SW_HIDE: i32 = 0;
    const SW_SHOW: i32 = 5;
    const SW_MAXIMIZE: i32 = 3;
    const SW_MINIMIZE: i32 = 6;
    const SW_RESTORE: i32 = 9;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const PM_REMOVE: u32 = 0x0001;
    const WM_KEYDOWN: u32 = 0x0100;
    const WM_KEYUP: u32 = 0x0101;
    const WM_CHAR: u32 = 0x0102;
    const WM_DESTROY: u32 = 0x0002;
    const WM_CLOSE: u32 = 0x0010;
    const WM_QUIT: u32 = 0x0012;
    const QS_ALLINPUT: u32 = 0x04ff;
    const MWMO_INPUTAVAILABLE: u32 = 0x0004;
    const WAIT_OBJECT_0: u32 = 0;
    const WAIT_TIMEOUT: u32 = 0x0000_0102;
    const WAIT_FAILED: u32 = u32::MAX;
    const TRANSPARENT: i32 = 1;
    const COLORONCOLOR: i32 = 3;
    const HALFTONE: i32 = 4;
    const SRCCOPY: u32 = 0x00cc_0020;
    const LOGPIXELSX: i32 = 88;
    const VREFRESH: i32 = 116;
    const MONITOR_DEFAULTTONEAREST: u32 = 2;
    const DT_CENTER: u32 = 0x0001;
    const DT_VCENTER: u32 = 0x0004;
    const DT_SINGLELINE: u32 = 0x0020;
    const DT_NOPREFIX: u32 = 0x0800;
    const DIB_RGB_COLORS: u32 = 0;
    const BI_RGB: u32 = 0;
    const AC_SRC_OVER: u8 = 0;
    const AC_SRC_ALPHA: u8 = 1;
    const FONT_CACHE_LIMIT: usize = 128;
    const SEGOE_UI_WIDE: [u16; 9] = [
        b'S' as u16,
        b'e' as u16,
        b'g' as u16,
        b'o' as u16,
        b'e' as u16,
        b' ' as u16,
        b'U' as u16,
        b'I' as u16,
        0,
    ];

    #[derive(Clone, Copy, Debug)]
    struct Viewport {
        logical_width: i32,
        logical_height: i32,
        mode: i32,
    }

    #[derive(Debug)]
    struct CanvasState {
        quality: i32,
        frame_mode: i32,
        target_fps: i32,
        last_present: Option<Instant>,
        frame_delta_ms: i64,
        last_client_size: Option<(i32, i32)>,
        last_dpi: i32,
        viewport: Option<Viewport>,
        pointer_pair: PointerPairCache,
    }

    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    struct FontKey {
        size: i32,
        weight: i32,
        quality: u32,
        dpi: i32,
    }

    impl Default for CanvasState {
        fn default() -> Self {
            Self {
                quality: 1,
                frame_mode: 0,
                target_fps: 0,
                last_present: None,
                frame_delta_ms: 0,
                last_client_size: None,
                last_dpi: 96,
                viewport: None,
                pointer_pair: PointerPairCache::default(),
            }
        }
    }

    #[derive(Debug)]
    struct Backbuffer {
        dc: isize,
        bitmap: isize,
        width: i32,
        height: i32,
    }

    impl Drop for Backbuffer {
        fn drop(&mut self) {
            unsafe {
                // Delete the DC first so the bitmap is no longer selected into it.
                DeleteDC(self.dc as Handle);
                DeleteObject(self.bitmap as Handle);
            }
        }
    }

    static BACKBUFFERS: OnceLock<Mutex<HashMap<isize, Backbuffer>>> = OnceLock::new();
    static CANVAS_STATES: OnceLock<Mutex<HashMap<isize, CanvasState>>> = OnceLock::new();
    static FONT_CACHE: OnceLock<Mutex<HashMap<FontKey, isize>>> = OnceLock::new();
    static DPI_AWARENESS: OnceLock<()> = OnceLock::new();

    fn backbuffers() -> &'static Mutex<HashMap<isize, Backbuffer>> {
        BACKBUFFERS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    fn canvas_states() -> &'static Mutex<HashMap<isize, CanvasState>> {
        CANVAS_STATES.get_or_init(|| Mutex::new(HashMap::new()))
    }

    fn font_cache() -> &'static Mutex<HashMap<FontKey, isize>> {
        FONT_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
    }

    fn clear_font_cache() {
        if let Ok(mut fonts) = font_cache().lock() {
            for font in fonts.drain().map(|(_, font)| font) {
                unsafe { DeleteObject(font as Handle) };
            }
        }
    }

    fn remove_backbuffer(handle: Handle) {
        if let Ok(mut buffers) = backbuffers().lock() {
            buffers.remove(&(handle as isize));
        }
        let mut no_windows_remain = false;
        if let Ok(mut states) = canvas_states().lock() {
            states.remove(&(handle as isize));
            no_windows_remain = states.is_empty();
        }
        if no_windows_remain {
            clear_font_cache();
        }
    }

    fn hwnd(value: i64) -> Handle {
        value as isize as Handle
    }

    fn int32(value: i64, name: &str) -> Result<i32, String> {
        i32::try_from(value)
            .map_err(|_| format!("E_RUNTIME_WINDOW_RANGE_001: {name} does not fit Win32 i32"))
    }

    fn color_ref(rgb: i64) -> u32 {
        let rgb = rgb as u32 & 0x00ff_ffff;
        ((rgb & 0x0000_00ff) << 16) | (rgb & 0x0000_ff00) | ((rgb & 0x00ff_0000) >> 16)
    }

    fn enable_process_dpi_awareness() {
        DPI_AWARENESS.get_or_init(|| unsafe {
            // PER_MONITOR_AWARE_V2 is the pointer-sized pseudo handle -4. If
            // another component fixed awareness earlier, retain it and use
            // the Vista-compatible process-aware fallback where possible.
            if SetProcessDpiAwarenessContext((-4isize) as Handle) == 0 {
                SetProcessDPIAware();
            }
        });
    }

    fn client_size(handle: Handle) -> Option<(i32, i32)> {
        let mut rect = Rect::default();
        if handle.is_null() || unsafe { GetClientRect(handle, &mut rect) } == 0 {
            return None;
        }
        let width = rect.right.saturating_sub(rect.left);
        let height = rect.bottom.saturating_sub(rect.top);
        (width > 0 && height > 0).then_some((width, height))
    }

    fn canvas_quality(handle: i64) -> i32 {
        canvas_states()
            .lock()
            .ok()
            .and_then(|states| {
                states
                    .get(&(hwnd(handle) as isize))
                    .map(|state| state.quality)
            })
            .unwrap_or(1)
    }

    fn font_quality(handle: i64) -> u32 {
        match canvas_quality(handle) {
            0 => 0, // DEFAULT_QUALITY
            2 => 6, // CLEARTYPE_NATURAL_QUALITY
            _ => 5, // CLEARTYPE_QUALITY
        }
    }

    fn with_cached_font<T>(
        handle: i64,
        size: i32,
        weight: i32,
        draw: impl FnOnce(Handle) -> T,
    ) -> Result<T, String> {
        let quality = font_quality(handle);
        let dpi = window_dpi(handle)?.clamp(1, i64::from(i32::MAX)) as i32;
        let key = FontKey {
            size,
            weight,
            quality,
            dpi,
        };
        let mut fonts = font_cache()
            .lock()
            .map_err(|_| "E_RUNTIME_CANVAS_FONT_001: font cache is poisoned".to_string())?;
        let font = if let Some(font) = fonts.get(&key).copied() {
            font as Handle
        } else {
            if fonts.len() >= FONT_CACHE_LIMIT {
                if let Some(evicted_key) = fonts.keys().next().copied() {
                    if let Some(evicted) = fonts.remove(&evicted_key) {
                        unsafe { DeleteObject(evicted as Handle) };
                    }
                }
            }
            let font = unsafe {
                CreateFontW(
                    -size,
                    0,
                    0,
                    0,
                    weight,
                    0,
                    0,
                    0,
                    1,
                    0,
                    0,
                    quality,
                    0,
                    SEGOE_UI_WIDE.as_ptr(),
                )
            };
            if font.is_null() {
                return Err("E_RUNTIME_CANVAS_FONT_001: CreateFontW returned NULL".into());
            }
            fonts.insert(key, font as isize);
            font
        };
        // Keep the cache mutex held while the object is selected. This makes
        // bounded eviction and last-window cleanup safe under concurrent draw.
        Ok(draw(font))
    }

    #[derive(Clone, Copy, Debug)]
    struct CanvasTransform {
        scale_x: f64,
        scale_y: f64,
        offset_x: f64,
        offset_y: f64,
    }

    impl CanvasTransform {
        fn identity() -> Self {
            Self {
                scale_x: 1.0,
                scale_y: 1.0,
                offset_x: 0.0,
                offset_y: 0.0,
            }
        }

        fn x(self, value: i64) -> i32 {
            scaled_i32(value as f64 * self.scale_x + self.offset_x)
        }

        fn y(self, value: i64) -> i32 {
            scaled_i32(value as f64 * self.scale_y + self.offset_y)
        }

        fn width(self, value: i64) -> i32 {
            scaled_extent(value, self.scale_x)
        }

        fn height(self, value: i64) -> i32 {
            scaled_extent(value, self.scale_y)
        }

        fn uniform(self, value: i64) -> i32 {
            scaled_extent(value, self.scale_x.abs().min(self.scale_y.abs()))
        }

        fn inverse_x(self, value: i64) -> i64 {
            ((value as f64 - self.offset_x) / self.scale_x).round() as i64
        }

        fn inverse_y(self, value: i64) -> i64 {
            ((value as f64 - self.offset_y) / self.scale_y).round() as i64
        }
    }

    fn scaled_i32(value: f64) -> i32 {
        value.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32
    }

    fn scaled_extent(value: i64, scale: f64) -> i32 {
        let scaled = scaled_i32(value as f64 * scale);
        if value > 0 {
            scaled.max(1)
        } else if value < 0 {
            scaled.min(-1)
        } else {
            0
        }
    }

    fn canvas_transform(handle: i64) -> CanvasTransform {
        let window = hwnd(handle);
        let viewport = canvas_states().lock().ok().and_then(|states| {
            states
                .get(&(window as isize))
                .and_then(|state| state.viewport)
        });
        let Some(viewport) = viewport else {
            return CanvasTransform::identity();
        };
        let Some((client_width, client_height)) = client_size(window) else {
            return CanvasTransform::identity();
        };
        let sx = f64::from(client_width) / f64::from(viewport.logical_width);
        let sy = f64::from(client_height) / f64::from(viewport.logical_height);
        match viewport.mode {
            0 => CanvasTransform {
                scale_x: sx,
                scale_y: sy,
                offset_x: 0.0,
                offset_y: 0.0,
            },
            1 | 2 => {
                let scale = if viewport.mode == 1 {
                    sx.min(sy)
                } else {
                    sx.max(sy)
                };
                CanvasTransform {
                    scale_x: scale,
                    scale_y: scale,
                    offset_x: (f64::from(client_width) - f64::from(viewport.logical_width) * scale)
                        / 2.0,
                    offset_y: (f64::from(client_height)
                        - f64::from(viewport.logical_height) * scale)
                        / 2.0,
                }
            }
            _ => CanvasTransform::identity(),
        }
    }

    pub(crate) fn window_open(title: &str, width: i64, height: i64) -> Result<i64, String> {
        enable_process_dpi_awareness();
        let mut title = super::utf8_to_utf16_strict(title, "window title")?;
        title.push(0);
        let width = int32(width, "window width")?;
        let height = int32(height, "window height")?;
        if width <= 0 || height <= 0 {
            return Err("E_RUNTIME_WINDOW_RANGE_001: window dimensions must be positive".into());
        }
        let class = [
            b'S' as u16,
            b'T' as u16,
            b'A' as u16,
            b'T' as u16,
            b'I' as u16,
            b'C' as u16,
            0,
        ];
        let instance = unsafe { GetModuleHandleA(null()) };
        let handle = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                i32::MIN,
                i32::MIN,
                width,
                height,
                null_mut(),
                null_mut(),
                instance,
                null_mut(),
            )
        };
        if handle.is_null() {
            return Err("E_RUNTIME_WINDOW_CREATE_001: CreateWindowExW returned NULL".into());
        }
        unsafe {
            ShowWindow(handle, SW_SHOW);
            UpdateWindow(handle);
        }
        let initial_size = client_size(handle);
        let initial_dpi = window_dpi(handle as isize as i64)
            .unwrap_or(96)
            .clamp(1, i64::from(i32::MAX)) as i32;
        if let Ok(mut states) = canvas_states().lock() {
            states.insert(
                handle as isize,
                CanvasState {
                    last_client_size: initial_size,
                    last_dpi: initial_dpi,
                    ..CanvasState::default()
                },
            );
        }
        Ok(handle as isize as i64)
    }

    pub(crate) fn window_show(handle: i64) -> Result<bool, String> {
        let handle = hwnd(handle);
        if handle.is_null() {
            return Ok(false);
        }
        unsafe {
            ShowWindow(handle, SW_SHOW);
            UpdateWindow(handle);
        }
        Ok(true)
    }

    pub(crate) fn window_hide(handle: i64) -> Result<bool, String> {
        let handle = hwnd(handle);
        if handle.is_null() {
            return Ok(false);
        }
        unsafe { ShowWindow(handle, SW_HIDE) };
        Ok(true)
    }

    pub(crate) fn window_poll(handle: i64) -> Result<bool, String> {
        let handle = hwnd(handle);
        if handle.is_null() || unsafe { IsWindow(handle) } == 0 {
            return Ok(false);
        }
        let mut msg: Msg = unsafe { zeroed() };
        let mut close_requested = false;
        let mut processed = 0usize;
        while processed < WINDOW_POLL_MESSAGE_BUDGET {
            // WM_QUIT is a thread message and therefore has no HWND. Probe it
            // separately so the normal pump can remain isolated to this
            // window instead of consuming events owned by sibling windows.
            if unsafe { PeekMessageA(&mut msg, null_mut(), WM_QUIT, WM_QUIT, PM_REMOVE) } != 0 {
                close_requested = true;
                break;
            }
            // WM_KEYDOWN and WM_CHAR are owned by window_read_key and
            // window_read_char. Pump the disjoint ranges around them so the
            // typed input APIs retain both the virtual key and translated
            // Unicode character.
            let mut found = unsafe { PeekMessageA(&mut msg, handle, 0, WM_KEYDOWN - 1, PM_REMOVE) };
            if found == 0 {
                found = unsafe { PeekMessageA(&mut msg, handle, WM_KEYUP, WM_KEYUP, PM_REMOVE) };
            }
            if found == 0 {
                found = unsafe { PeekMessageA(&mut msg, handle, WM_CHAR + 1, u32::MAX, PM_REMOVE) };
            }
            if found == 0 {
                break;
            }
            processed += 1;
            if matches!(msg.message, WM_CLOSE | WM_DESTROY | WM_QUIT) {
                if msg.message == WM_CLOSE && !msg.hwnd.is_null() {
                    unsafe { DestroyWindow(msg.hwnd) };
                }
                if msg.message == WM_QUIT || msg.hwnd.is_null() || msg.hwnd == handle {
                    close_requested = true;
                }
                // Keep draining the queue. A render loop must not inherit an
                // unbounded backlog after one event-heavy frame.
                continue;
            }
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageA(&msg);
            }
        }
        let open = !close_requested && unsafe { IsWindow(handle) } != 0;
        if !open {
            remove_backbuffer(handle);
        }
        Ok(open)
    }

    pub(crate) fn window_wait_event(handle: i64, timeout_ms: i64) -> Result<bool, String> {
        let timeout = super::validated_window_wait_timeout(timeout_ms)?;
        let window = hwnd(handle);
        if window.is_null() || unsafe { IsWindow(window) } == 0 {
            return Ok(false);
        }
        let status = unsafe {
            MsgWaitForMultipleObjectsEx(0, null(), timeout, QS_ALLINPUT, MWMO_INPUTAVAILABLE)
        };
        match status {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            WAIT_FAILED => {
                Err("E_RUNTIME_WINDOW_WAIT_002: MsgWaitForMultipleObjectsEx failed".into())
            }
            _ => Ok(false),
        }
    }

    pub(crate) fn window_close(handle: i64) -> Result<bool, String> {
        let handle = hwnd(handle);
        remove_backbuffer(handle);
        Ok(!handle.is_null() && unsafe { DestroyWindow(handle) } != 0)
    }

    pub(crate) fn window_is_open(handle: i64) -> Result<bool, String> {
        let handle = hwnd(handle);
        Ok(!handle.is_null() && unsafe { IsWindow(handle) } != 0)
    }

    pub(crate) fn window_is_focused(handle: i64) -> Result<bool, String> {
        let handle = hwnd(handle);
        Ok(!handle.is_null() && unsafe { GetForegroundWindow() } == handle)
    }

    pub(crate) fn window_set_title(handle: i64, title: &str) -> Result<bool, String> {
        let mut title = super::utf8_to_utf16_strict(title, "window title")?;
        title.push(0);
        Ok(unsafe { SetWindowTextW(hwnd(handle), title.as_ptr()) } != 0)
    }

    pub(crate) fn window_set_size(handle: i64, width: i64, height: i64) -> Result<bool, String> {
        let width = int32(width, "window width")?;
        let height = int32(height, "window height")?;
        if width <= 0 || height <= 0 {
            return Err("E_RUNTIME_WINDOW_RANGE_001: window dimensions must be positive".into());
        }
        Ok(unsafe {
            SetWindowPos(
                hwnd(handle),
                null_mut(),
                0,
                0,
                width,
                height,
                SWP_NOMOVE | SWP_NOZORDER,
            )
        } != 0)
    }

    fn window_extent(handle: i64, width: bool) -> Result<i64, String> {
        let mut rect = Rect::default();
        if unsafe { GetClientRect(hwnd(handle), &mut rect) } == 0 {
            return Ok(0);
        }
        Ok(if width {
            i64::from(rect.right - rect.left)
        } else {
            i64::from(rect.bottom - rect.top)
        })
    }

    pub(crate) fn window_width(handle: i64) -> Result<i64, String> {
        window_extent(handle, true)
    }

    pub(crate) fn window_height(handle: i64) -> Result<i64, String> {
        window_extent(handle, false)
    }

    pub(crate) fn window_dpi(handle: i64) -> Result<i64, String> {
        let window = hwnd(handle);
        if window.is_null() || unsafe { IsWindow(window) } == 0 {
            return Ok(0);
        }
        let dpi = unsafe { GetDpiForWindow(window) };
        if dpi != 0 {
            return Ok(i64::from(dpi));
        }
        with_dc(handle, |dc| {
            i64::from(unsafe { GetDeviceCaps(dc, LOGPIXELSX) }.max(0))
        })
    }

    pub(crate) fn window_refresh_hz(handle: i64) -> Result<i64, String> {
        let window = hwnd(handle);
        if window.is_null() || unsafe { IsWindow(window) } == 0 {
            return Ok(0);
        }
        let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
        if monitor.is_null() {
            return Ok(60);
        }
        let mut info = MonitorInfoExW {
            size: size_of::<MonitorInfoExW>() as u32,
            ..MonitorInfoExW::default()
        };
        if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
            return Ok(60);
        }
        let dc = unsafe { CreateDCW(info.device.as_ptr(), null(), null(), null()) };
        if dc.is_null() {
            return Ok(60);
        }
        let hz = unsafe { GetDeviceCaps(dc, VREFRESH) };
        unsafe { DeleteDC(dc) };
        Ok(i64::from(if hz <= 1 { 60 } else { hz }))
    }

    pub(crate) fn window_state(handle: i64) -> Result<i64, String> {
        let window = hwnd(handle);
        if window.is_null() || unsafe { IsWindow(window) } == 0 {
            return Ok(-1);
        }
        Ok(if unsafe { IsIconic(window) } != 0 {
            1
        } else if unsafe { IsZoomed(window) } != 0 {
            2
        } else {
            0
        })
    }

    pub(crate) fn window_set_state(handle: i64, state: i64) -> Result<bool, String> {
        let window = hwnd(handle);
        if window.is_null() || unsafe { IsWindow(window) } == 0 {
            return Ok(false);
        }
        let command = match state {
            0 => SW_RESTORE,
            1 => SW_MINIMIZE,
            2 => SW_MAXIMIZE,
            _ => {
                return Err(
                    "E_RUNTIME_WINDOW_STATE_001: state must be 0 (normal), 1 (minimized), or 2 (maximized)"
                        .into(),
                )
            }
        };
        unsafe { ShowWindow(window, command) };
        Ok(true)
    }

    pub(crate) fn window_focus(handle: i64) -> Result<bool, String> {
        Ok(unsafe { SetForegroundWindow(hwnd(handle)) } != 0)
    }

    fn with_dc<T>(handle: i64, draw: impl FnOnce(Handle) -> T) -> Result<T, String> {
        let hwnd = hwnd(handle);
        let dc = unsafe { GetDC(hwnd) };
        if dc.is_null() {
            return Err("E_RUNTIME_CANVAS_DC_001: GetDC returned NULL".into());
        }
        let result = draw(dc);
        unsafe { ReleaseDC(hwnd, dc) };
        Ok(result)
    }

    fn with_canvas_dc<T>(handle: i64, draw: impl FnOnce(Handle) -> T) -> Result<T, String> {
        let key = hwnd(handle) as isize;
        let buffers = backbuffers()
            .lock()
            .map_err(|_| "E_RUNTIME_CANVAS_STATE_001: backbuffer state is poisoned".to_string())?;
        if let Some(buffer) = buffers.get(&key) {
            return Ok(draw(buffer.dc as Handle));
        }
        drop(buffers);
        with_dc(handle, draw)
    }

    pub(crate) fn canvas_set_quality(handle: i64, quality: i64) -> Result<bool, String> {
        let window = hwnd(handle);
        if window.is_null() || unsafe { IsWindow(window) } == 0 {
            return Ok(false);
        }
        let quality = match quality {
            0..=2 => quality as i32,
            _ => {
                return Err(
                    "E_RUNTIME_CANVAS_QUALITY_001: quality must be 0 (balanced), 1 (high), or 2 (ultra)"
                        .into(),
                )
            }
        };
        let changed = {
            let mut states = canvas_states()
                .lock()
                .map_err(|_| "E_RUNTIME_CANVAS_STATE_001: canvas state is poisoned".to_string())?;
            let state = states.entry(window as isize).or_default();
            let changed = state.quality != quality;
            state.quality = quality;
            changed
        };
        if changed {
            clear_font_cache();
        }
        Ok(true)
    }

    pub(crate) fn canvas_set_frame_mode(
        handle: i64,
        mode: i64,
        target_fps: i64,
    ) -> Result<bool, String> {
        let window = hwnd(handle);
        if window.is_null() || unsafe { IsWindow(window) } == 0 {
            return Ok(false);
        }
        if !(0..=2).contains(&mode) {
            return Err(
                "E_RUNTIME_CANVAS_FRAME_001: frame mode must be 0 (uncapped), 1 (fixed), or 2 (refresh-paced)"
                    .into(),
            );
        }
        if mode == 1 && !(1..=1000).contains(&target_fps) {
            return Err(
                "E_RUNTIME_CANVAS_FRAME_001: fixed frame mode requires targetFps in 1..1000".into(),
            );
        }
        if mode == 2 && target_fps > 1000 {
            return Err(
                "E_RUNTIME_CANVAS_FRAME_001: refresh-paced targetFps must be <= 1000".into(),
            );
        }
        let mut states = canvas_states()
            .lock()
            .map_err(|_| "E_RUNTIME_CANVAS_STATE_001: canvas state is poisoned".to_string())?;
        let state = states.entry(window as isize).or_default();
        state.frame_mode = mode as i32;
        state.target_fps = target_fps.clamp(0, 1000) as i32;
        state.last_present = None;
        state.frame_delta_ms = 0;
        Ok(true)
    }

    pub(crate) fn canvas_frame_delta_ms(handle: i64) -> Result<i64, String> {
        let window = hwnd(handle);
        if window.is_null() || unsafe { IsWindow(window) } == 0 {
            return Ok(0);
        }
        let states = canvas_states()
            .lock()
            .map_err(|_| "E_RUNTIME_CANVAS_STATE_001: canvas state is poisoned".to_string())?;
        Ok(states
            .get(&(window as isize))
            .map_or(0, |state| state.frame_delta_ms))
    }

    pub(crate) fn canvas_poll_resize(handle: i64) -> Result<bool, String> {
        let window = hwnd(handle);
        let Some(current) = client_size(window) else {
            return Ok(false);
        };
        let current_dpi = window_dpi(handle)?.clamp(1, i64::from(i32::MAX)) as i32;
        let (changed, dpi_changed) = {
            let mut states = canvas_states()
                .lock()
                .map_err(|_| "E_RUNTIME_CANVAS_STATE_001: canvas state is poisoned".to_string())?;
            let state = states.entry(window as isize).or_default();
            let changed = state
                .last_client_size
                .is_some_and(|previous| previous != current);
            let dpi_changed = state.last_dpi != current_dpi;
            state.last_client_size = Some(current);
            state.last_dpi = current_dpi;
            (changed, dpi_changed)
        };
        if dpi_changed {
            clear_font_cache();
        }
        Ok(changed)
    }

    pub(crate) fn canvas_set_viewport(
        handle: i64,
        logical_width: i64,
        logical_height: i64,
        mode: i64,
    ) -> Result<bool, String> {
        let window = hwnd(handle);
        if window.is_null() || unsafe { IsWindow(window) } == 0 {
            return Ok(false);
        }
        let logical_width = int32(logical_width, "logical viewport width")?;
        let logical_height = int32(logical_height, "logical viewport height")?;
        if logical_width <= 0 || logical_height <= 0 {
            return Err(
                "E_RUNTIME_CANVAS_VIEWPORT_001: logical viewport dimensions must be positive"
                    .into(),
            );
        }
        if !(0..=2).contains(&mode) {
            return Err(
                "E_RUNTIME_CANVAS_VIEWPORT_001: viewport mode must be 0 (stretch), 1 (fit), or 2 (fill)"
                    .into(),
            );
        }
        let mut states = canvas_states()
            .lock()
            .map_err(|_| "E_RUNTIME_CANVAS_STATE_001: canvas state is poisoned".to_string())?;
        states.entry(window as isize).or_default().viewport = Some(Viewport {
            logical_width,
            logical_height,
            mode: mode as i32,
        });
        Ok(true)
    }

    pub(crate) fn canvas_pointer_x(handle: i64) -> Result<i64, String> {
        let Some(point) = sample_client_pointer(handle) else {
            return Ok(-1);
        };
        let transform = canvas_transform(handle);
        let x = transform.inverse_x(i64::from(point.x));
        let y = transform.inverse_y(i64::from(point.y));
        let mut states = canvas_states()
            .lock()
            .map_err(|_| "E_RUNTIME_CANVAS_STATE_001: canvas state is poisoned".to_string())?;
        states
            .entry(hwnd(handle) as isize)
            .or_default()
            .pointer_pair
            .store_y(y);
        Ok(x)
    }

    pub(crate) fn canvas_pointer_y(handle: i64) -> Result<i64, String> {
        if let Some(y) = canvas_states()
            .lock()
            .map_err(|_| "E_RUNTIME_CANVAS_STATE_001: canvas state is poisoned".to_string())?
            .get_mut(&(hwnd(handle) as isize))
            .and_then(|state| state.pointer_pair.take_y())
        {
            return Ok(y);
        }
        let Some(point) = sample_client_pointer(handle) else {
            return Ok(-1);
        };
        Ok(canvas_transform(handle).inverse_y(i64::from(point.y)))
    }

    pub(crate) fn canvas_begin_frame(handle: i64, color: i64) -> Result<bool, String> {
        let window = hwnd(handle);
        if window.is_null() || unsafe { IsWindow(window) } == 0 {
            return Ok(false);
        }
        let mut rect = Rect::default();
        if unsafe { GetClientRect(window, &mut rect) } == 0 {
            return Ok(false);
        }
        let width = rect.right.saturating_sub(rect.left);
        let height = rect.bottom.saturating_sub(rect.top);
        if width <= 0 || height <= 0 {
            return Ok(false);
        }

        let key = window as isize;
        let mut buffers = backbuffers()
            .lock()
            .map_err(|_| "E_RUNTIME_CANVAS_STATE_001: backbuffer state is poisoned".to_string())?;
        let resize = buffers
            .get(&key)
            .is_none_or(|buffer| buffer.width != width || buffer.height != height);
        if resize {
            buffers.remove(&key);
            let window_dc = unsafe { GetDC(window) };
            if window_dc.is_null() {
                return Err("E_RUNTIME_CANVAS_DC_001: GetDC returned NULL".into());
            }
            let memory_dc = unsafe { CreateCompatibleDC(window_dc) };
            let bitmap = if memory_dc.is_null() {
                null_mut()
            } else {
                unsafe { CreateCompatibleBitmap(window_dc, width, height) }
            };
            unsafe { ReleaseDC(window, window_dc) };
            if memory_dc.is_null() || bitmap.is_null() {
                if !memory_dc.is_null() {
                    unsafe { DeleteDC(memory_dc) };
                }
                if !bitmap.is_null() {
                    unsafe { DeleteObject(bitmap) };
                }
                return Err(
                    "E_RUNTIME_CANVAS_BACKBUFFER_001: compatible DC/bitmap creation failed".into(),
                );
            }
            if unsafe { SelectObject(memory_dc, bitmap) }.is_null() {
                unsafe {
                    DeleteDC(memory_dc);
                    DeleteObject(bitmap);
                }
                return Err("E_RUNTIME_CANVAS_BACKBUFFER_001: bitmap selection failed".into());
            }
            buffers.insert(
                key,
                Backbuffer {
                    dc: memory_dc as isize,
                    bitmap: bitmap as isize,
                    width,
                    height,
                },
            );
        }

        let Some(buffer) = buffers.get(&key) else {
            return Ok(false);
        };
        let brush = unsafe { CreateSolidBrush(color_ref(color)) };
        if brush.is_null() {
            return Ok(false);
        }
        let ok = unsafe { FillRect(buffer.dc as Handle, &rect, brush) } != 0;
        unsafe { DeleteObject(brush) };
        Ok(ok)
    }

    pub(crate) fn canvas_clear(handle: i64, color: i64) -> Result<bool, String> {
        let mut rect = Rect::default();
        if unsafe { GetClientRect(hwnd(handle), &mut rect) } == 0 {
            return Ok(false);
        }
        with_canvas_dc(handle, |dc| {
            let brush = unsafe { CreateSolidBrush(color_ref(color)) };
            if brush.is_null() {
                return false;
            }
            let ok = unsafe { FillRect(dc, &rect, brush) } != 0;
            unsafe { DeleteObject(brush) };
            ok
        })
    }

    pub(crate) fn canvas_rect(
        handle: i64,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        color: i64,
    ) -> Result<bool, String> {
        let transform = canvas_transform(handle);
        let x = transform.x(x);
        let y = transform.y(y);
        let width = transform.width(width);
        let height = transform.height(height);
        let rect = Rect {
            left: x,
            top: y,
            right: x.saturating_add(width),
            bottom: y.saturating_add(height),
        };
        with_canvas_dc(handle, |dc| {
            let brush = unsafe { CreateSolidBrush(color_ref(color)) };
            if brush.is_null() {
                return false;
            }
            let ok = unsafe { FillRect(dc, &rect, brush) } != 0;
            unsafe { DeleteObject(brush) };
            ok
        })
    }

    pub(crate) fn canvas_text(
        handle: i64,
        x: i64,
        y: i64,
        text: &str,
        color: i64,
    ) -> Result<bool, String> {
        let transform = canvas_transform(handle);
        let x = transform.x(x);
        let y = transform.y(y);
        let text = super::utf8_to_utf16_strict(text, "canvas text")?;
        let len = i32::try_from(text.len())
            .map_err(|_| "E_RUNTIME_WINDOW_TEXT_001: canvas text is too long".to_string())?;
        let scaled_default_size = transform.uniform(16).max(1);
        with_canvas_dc(handle, |dc| {
            with_cached_font(handle, scaled_default_size, 400, |font| {
                let previous = unsafe { SelectObject(dc, font) };
                unsafe {
                    SetTextColor(dc, color_ref(color));
                    SetBkMode(dc, TRANSPARENT);
                }
                let ok = unsafe { TextOutW(dc, x, y, text.as_ptr(), len) } != 0;
                if !previous.is_null() {
                    unsafe { SelectObject(dc, previous) };
                }
                ok
            })
        })?
    }

    pub(crate) fn canvas_line(
        handle: i64,
        x1: i64,
        y1: i64,
        x2: i64,
        y2: i64,
        color: i64,
        width: i64,
    ) -> Result<bool, String> {
        let transform = canvas_transform(handle);
        let x1 = transform.x(x1);
        let y1 = transform.y(y1);
        let x2 = transform.x(x2);
        let y2 = transform.y(y2);
        let width = transform.uniform(width.max(1)).max(1);
        with_canvas_dc(handle, |dc| {
            let pen = unsafe { CreatePen(0, width, color_ref(color)) };
            if pen.is_null() {
                return false;
            }
            let previous = unsafe { SelectObject(dc, pen) };
            let moved = unsafe { MoveToEx(dc, x1, y1, null_mut()) } != 0;
            let drawn = moved && unsafe { LineTo(dc, x2, y2) } != 0;
            if !previous.is_null() {
                unsafe { SelectObject(dc, previous) };
            }
            unsafe { DeleteObject(pen) };
            drawn
        })
    }

    pub(crate) fn canvas_ellipse(
        handle: i64,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        color: i64,
    ) -> Result<bool, String> {
        let transform = canvas_transform(handle);
        let left = transform.x(x);
        let top = transform.y(y);
        let right = left.saturating_add(transform.width(width));
        let bottom = top.saturating_add(transform.height(height));
        with_canvas_dc(handle, |dc| {
            let brush = unsafe { CreateSolidBrush(color_ref(color)) };
            if brush.is_null() {
                return false;
            }
            let previous = unsafe { SelectObject(dc, brush) };
            let ok = unsafe { Ellipse(dc, left, top, right, bottom) } != 0;
            if !previous.is_null() {
                unsafe { SelectObject(dc, previous) };
            }
            unsafe { DeleteObject(brush) };
            ok
        })
    }

    pub(crate) fn canvas_round_rect(
        handle: i64,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        radius: i64,
        color: i64,
    ) -> Result<bool, String> {
        let transform = canvas_transform(handle);
        let left = transform.x(x);
        let top = transform.y(y);
        let width = transform.width(width);
        let height = transform.height(height);
        if width <= 0 || height <= 0 {
            return Ok(false);
        }
        let diameter = transform
            .uniform(radius.max(0).saturating_mul(2))
            .min(width)
            .min(height);
        with_canvas_dc(handle, |dc| {
            let color = color_ref(color);
            let brush = unsafe { CreateSolidBrush(color) };
            let pen = unsafe { CreatePen(0, 1, color) };
            if brush.is_null() || pen.is_null() {
                if !brush.is_null() {
                    unsafe { DeleteObject(brush) };
                }
                if !pen.is_null() {
                    unsafe { DeleteObject(pen) };
                }
                return false;
            }
            let previous_brush = unsafe { SelectObject(dc, brush) };
            let previous_pen = unsafe { SelectObject(dc, pen) };
            let ok = unsafe {
                RoundRect(
                    dc,
                    left,
                    top,
                    left.saturating_add(width),
                    top.saturating_add(height),
                    diameter,
                    diameter,
                )
            } != 0;
            if !previous_pen.is_null() {
                unsafe { SelectObject(dc, previous_pen) };
            }
            if !previous_brush.is_null() {
                unsafe { SelectObject(dc, previous_brush) };
            }
            unsafe {
                DeleteObject(pen);
                DeleteObject(brush);
            }
            ok
        })
    }

    pub(crate) fn canvas_text_styled(
        handle: i64,
        x: i64,
        y: i64,
        text: &str,
        color: i64,
        size: i64,
        weight: i64,
    ) -> Result<bool, String> {
        let transform = canvas_transform(handle);
        let x = transform.x(x);
        let y = transform.y(y);
        let size = transform.uniform(size.max(1)).max(1);
        let weight = int32(weight.clamp(1, 1000), "styled text weight")?;
        let text = super::utf8_to_utf16_strict(text, "styled canvas text")?;
        let len = i32::try_from(text.len())
            .map_err(|_| "E_RUNTIME_WINDOW_TEXT_001: canvas text is too long".to_string())?;
        with_canvas_dc(handle, |dc| {
            with_cached_font(handle, size, weight, |font| {
                let previous = unsafe { SelectObject(dc, font) };
                unsafe {
                    SetTextColor(dc, color_ref(color));
                    SetBkMode(dc, TRANSPARENT);
                }
                let ok = unsafe { TextOutW(dc, x, y, text.as_ptr(), len) } != 0;
                if !previous.is_null() {
                    unsafe { SelectObject(dc, previous) };
                }
                ok
            })
        })?
    }

    pub(crate) fn canvas_text_centered(
        handle: i64,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        text: &str,
        color: i64,
        size: i64,
        weight: i64,
    ) -> Result<bool, String> {
        let transform = canvas_transform(handle);
        let left = transform.x(x);
        let top = transform.y(y);
        let width = transform.width(width);
        let height = transform.height(height);
        if width <= 0 || height <= 0 {
            return Ok(false);
        }
        let size = transform.uniform(size.max(1)).max(1);
        let weight = int32(weight.clamp(1, 1000), "centered text weight")?;
        let text = super::utf8_to_utf16_strict(text, "centered canvas text")?;
        let len = i32::try_from(text.len())
            .map_err(|_| "E_RUNTIME_WINDOW_TEXT_001: canvas text is too long".to_string())?;
        let mut rect = Rect {
            left,
            top,
            right: left.saturating_add(width),
            bottom: top.saturating_add(height),
        };
        with_canvas_dc(handle, |dc| {
            with_cached_font(handle, size, weight, |font| {
                let previous = unsafe { SelectObject(dc, font) };
                unsafe {
                    SetTextColor(dc, color_ref(color));
                    SetBkMode(dc, TRANSPARENT);
                }
                let ok = unsafe {
                    DrawTextW(
                        dc,
                        text.as_ptr(),
                        len,
                        &mut rect,
                        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                    )
                } != 0;
                if !previous.is_null() {
                    unsafe { SelectObject(dc, previous) };
                }
                ok
            })
        })?
    }

    fn rgb_components(color: i64) -> (u8, u8, u8) {
        let value = color as u32;
        (
            ((value >> 16) & 0xff) as u8,
            ((value >> 8) & 0xff) as u8,
            (value & 0xff) as u8,
        )
    }

    fn premultiplied_bgra(red: u8, green: u8, blue: u8, alpha: u8) -> u32 {
        let alpha32 = u32::from(alpha);
        let red = u32::from(red) * alpha32 / 255;
        let green = u32::from(green) * alpha32 / 255;
        let blue = u32::from(blue) * alpha32 / 255;
        blue | (green << 8) | (red << 16) | (alpha32 << 24)
    }

    fn alpha_blend_pixels_to_dc(
        dc: Handle,
        dst_x: i32,
        dst_y: i32,
        dst_width: i32,
        dst_height: i32,
        src_width: i32,
        src_height: i32,
        pixels: &[u32],
    ) -> bool {
        if dst_width <= 0
            || dst_height <= 0
            || src_width <= 0
            || src_height <= 0
            || pixels.len() != src_width as usize * src_height as usize
        {
            return false;
        }
        let info = BitmapInfo {
            header: BitmapInfoHeader {
                size: std::mem::size_of::<BitmapInfoHeader>() as u32,
                width: src_width,
                height: -src_height,
                planes: 1,
                bit_count: 32,
                compression: BI_RGB,
                size_image: (src_width as u32)
                    .saturating_mul(src_height as u32)
                    .saturating_mul(4),
                ..BitmapInfoHeader::default()
            },
            ..BitmapInfo::default()
        };
        let source_dc = unsafe { CreateCompatibleDC(dc) };
        if source_dc.is_null() {
            return false;
        }
        let mut bits: *mut c_void = null_mut();
        let bitmap =
            unsafe { CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0) };
        if bitmap.is_null() || bits.is_null() {
            unsafe { DeleteDC(source_dc) };
            if !bitmap.is_null() {
                unsafe { DeleteObject(bitmap) };
            }
            return false;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u32>(), pixels.len());
        }
        let previous = unsafe { SelectObject(source_dc, bitmap) };
        if previous.is_null() {
            unsafe {
                DeleteDC(source_dc);
                DeleteObject(bitmap);
            }
            return false;
        }
        let ok = unsafe {
            AlphaBlend(
                dc,
                dst_x,
                dst_y,
                dst_width,
                dst_height,
                source_dc,
                0,
                0,
                src_width,
                src_height,
                BlendFunction {
                    operation: AC_SRC_OVER,
                    flags: 0,
                    source_constant_alpha: 255,
                    alpha_format: AC_SRC_ALPHA,
                },
            )
        } != 0;
        unsafe {
            SelectObject(source_dc, previous);
            DeleteDC(source_dc);
            DeleteObject(bitmap);
        }
        ok
    }

    fn rounded_signed_distance(px: f64, py: f64, width: f64, height: f64, radius: f64) -> f64 {
        let radius = radius.max(0.0).min(width.min(height) / 2.0);
        let qx = (px - width / 2.0).abs() - (width / 2.0 - radius);
        let qy = (py - height / 2.0).abs() - (height / 2.0 - radius);
        qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius
    }

    fn rounded_gradient_pixels(
        width: i32,
        height: i32,
        radius: i32,
        color_a: i64,
        color_b: i64,
        vertical: bool,
        alpha: u8,
        quality: i32,
    ) -> Result<Vec<u32>, String> {
        let area = usize::try_from(width)
            .ok()
            .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
            .filter(|area| *area <= 67_108_864)
            .ok_or_else(|| {
                "E_RUNTIME_CANVAS_BITMAP_001: composited rounded rectangle is too large".to_string()
            })?;
        let (ar, ag, ab) = rgb_components(color_a);
        let (br, bg, bb) = rgb_components(color_b);
        let mut pixels = Vec::with_capacity(area);
        let edge_width = match quality {
            0 => 0.0,
            2 => 1.5,
            _ => 1.0,
        };
        for py in 0..height {
            for px in 0..width {
                let distance = rounded_signed_distance(
                    f64::from(px) + 0.5,
                    f64::from(py) + 0.5,
                    f64::from(width),
                    f64::from(height),
                    f64::from(radius),
                );
                let coverage = if edge_width == 0.0 {
                    if distance <= 0.0 {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    (0.5 - distance / edge_width).clamp(0.0, 1.0)
                };
                let denominator = if vertical { height - 1 } else { width - 1 }.max(1);
                let numerator = if vertical { py } else { px };
                let t = f64::from(numerator) / f64::from(denominator);
                let lerp =
                    |a: u8, b: u8| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as u8;
                let pixel_alpha = (f64::from(alpha) * coverage).round() as u8;
                pixels.push(premultiplied_bgra(
                    lerp(ar, br),
                    lerp(ag, bg),
                    lerp(ab, bb),
                    pixel_alpha,
                ));
            }
        }
        Ok(pixels)
    }

    pub(crate) fn canvas_round_rect_alpha(
        handle: i64,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        radius: i64,
        color: i64,
        alpha: i64,
    ) -> Result<bool, String> {
        let transform = canvas_transform(handle);
        let left = transform.x(x);
        let top = transform.y(y);
        let width = transform.width(width);
        let height = transform.height(height);
        let radius = transform.uniform(radius.max(0));
        if width <= 0 || height <= 0 {
            return Ok(false);
        }
        if !(0..=255).contains(&alpha) {
            return Err("E_RUNTIME_CANVAS_ALPHA_001: alpha must be in 0..255".into());
        }
        let pixels = rounded_gradient_pixels(
            width,
            height,
            radius,
            color,
            color,
            true,
            alpha as u8,
            canvas_quality(handle),
        )?;
        with_canvas_dc(handle, |dc| {
            alpha_blend_pixels_to_dc(dc, left, top, width, height, width, height, &pixels)
        })
    }

    pub(crate) fn canvas_round_rect_gradient(
        handle: i64,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        radius: i64,
        color_a: i64,
        color_b: i64,
        vertical: i64,
    ) -> Result<bool, String> {
        let transform = canvas_transform(handle);
        let left = transform.x(x);
        let top = transform.y(y);
        let width = transform.width(width);
        let height = transform.height(height);
        let radius = transform.uniform(radius.max(0));
        if width <= 0 || height <= 0 {
            return Ok(false);
        }
        if !(0..=1).contains(&vertical) {
            return Err("E_RUNTIME_CANVAS_GRADIENT_001: vertical must be 0 or 1".into());
        }
        let pixels = rounded_gradient_pixels(
            width,
            height,
            radius,
            color_a,
            color_b,
            vertical != 0,
            255,
            canvas_quality(handle),
        )?;
        with_canvas_dc(handle, |dc| {
            alpha_blend_pixels_to_dc(dc, left, top, width, height, width, height, &pixels)
        })
    }

    pub(crate) fn canvas_shadow_round_rect(
        handle: i64,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        radius: i64,
        color: i64,
        spread: i64,
    ) -> Result<bool, String> {
        let transform = canvas_transform(handle);
        let x = transform.x(x);
        let y = transform.y(y);
        let width = transform.width(width);
        let height = transform.height(height);
        let radius = transform.uniform(radius.max(0));
        let quality = canvas_quality(handle);
        let spread_cap = match quality {
            0 => 32,
            2 => 128,
            _ => 64,
        };
        let spread = transform.uniform(spread.max(0)).min(spread_cap);
        if width <= 0 || height <= 0 {
            return Ok(false);
        }
        let bitmap_width = width.saturating_add(spread.saturating_mul(2));
        let bitmap_height = height.saturating_add(spread.saturating_mul(2));
        let area = usize::try_from(bitmap_width)
            .ok()
            .and_then(|w| {
                usize::try_from(bitmap_height)
                    .ok()
                    .and_then(|h| w.checked_mul(h))
            })
            .filter(|area| *area <= 67_108_864)
            .ok_or_else(|| {
                "E_RUNTIME_CANVAS_BITMAP_001: shadow composition is too large".to_string()
            })?;
        let (red, green, blue) = rgb_components(color);
        let encoded_alpha = ((color as u64 >> 24) & 0xff) as u8;
        let base_alpha = if encoded_alpha == 0 {
            96
        } else {
            encoded_alpha
        };
        let mut pixels = Vec::with_capacity(area);
        let softness = f64::from(spread.max(1));
        for py in 0..bitmap_height {
            for px in 0..bitmap_width {
                let distance = rounded_signed_distance(
                    f64::from(px - spread) + 0.5,
                    f64::from(py - spread) + 0.5,
                    f64::from(width),
                    f64::from(height),
                    f64::from(radius),
                );
                let coverage = if distance <= 0.0 {
                    1.0
                } else {
                    (1.0 - distance / softness).clamp(0.0, 1.0).powi(2)
                };
                let alpha = (f64::from(base_alpha) * coverage).round() as u8;
                pixels.push(premultiplied_bgra(red, green, blue, alpha));
            }
        }
        with_canvas_dc(handle, |dc| {
            alpha_blend_pixels_to_dc(
                dc,
                x.saturating_sub(spread),
                y.saturating_sub(spread),
                bitmap_width,
                bitmap_height,
                bitmap_width,
                bitmap_height,
                &pixels,
            )
        })
    }

    pub(crate) fn canvas_backdrop_blur(handle: i64, radius: i64) -> Result<bool, String> {
        let window = hwnd(handle);
        if window.is_null() || unsafe { IsWindow(window) } == 0 {
            return Ok(false);
        }
        let radius = int32(radius.max(0), "backdrop blur radius")?;
        let buffers = backbuffers()
            .lock()
            .map_err(|_| "E_RUNTIME_CANVAS_STATE_001: backbuffer state is poisoned".to_string())?;
        let Some(buffer) = buffers.get(&(window as isize)) else {
            // Blur is intentionally a frame/backbuffer operation; it never
            // captures arbitrary desktop pixels from a direct window DC.
            return Ok(false);
        };
        if radius == 0 {
            return Ok(true);
        }
        let factor = radius.clamp(1, 32).saturating_add(1);
        let small_width = (buffer.width / factor).max(1);
        let small_height = (buffer.height / factor).max(1);
        let source_dc = buffer.dc as Handle;
        let temporary_dc = unsafe { CreateCompatibleDC(source_dc) };
        let temporary_bitmap = if temporary_dc.is_null() {
            null_mut()
        } else {
            unsafe { CreateCompatibleBitmap(source_dc, small_width, small_height) }
        };
        if temporary_dc.is_null() || temporary_bitmap.is_null() {
            if !temporary_dc.is_null() {
                unsafe { DeleteDC(temporary_dc) };
            }
            if !temporary_bitmap.is_null() {
                unsafe { DeleteObject(temporary_bitmap) };
            }
            return Ok(false);
        }
        if unsafe { SelectObject(temporary_dc, temporary_bitmap) }.is_null() {
            unsafe {
                DeleteDC(temporary_dc);
                DeleteObject(temporary_bitmap);
            }
            return Ok(false);
        }
        unsafe {
            SetStretchBltMode(temporary_dc, HALFTONE);
            SetStretchBltMode(source_dc, HALFTONE);
        }
        let downsampled = unsafe {
            StretchBlt(
                temporary_dc,
                0,
                0,
                small_width,
                small_height,
                source_dc,
                0,
                0,
                buffer.width,
                buffer.height,
                SRCCOPY,
            )
        } != 0;
        let upsampled = downsampled
            && unsafe {
                StretchBlt(
                    source_dc,
                    0,
                    0,
                    buffer.width,
                    buffer.height,
                    temporary_dc,
                    0,
                    0,
                    small_width,
                    small_height,
                    SRCCOPY,
                )
            } != 0;
        unsafe {
            DeleteDC(temporary_dc);
            DeleteObject(temporary_bitmap);
        }
        Ok(upsampled)
    }

    pub(crate) fn canvas_backdrop_blur_v3(
        handle: i64,
        radius: i64,
        passes: i64,
        tint_color: i64,
    ) -> Result<bool, String> {
        let window = hwnd(handle);
        if window.is_null() || unsafe { IsWindow(window) } == 0 {
            return Ok(false);
        }
        if radius < 0 {
            return Err("E_RUNTIME_CANVAS_BLUR_001: blur radius must be non-negative".into());
        }
        if passes <= 0 {
            return Err("E_RUNTIME_CANVAS_BLUR_001: blur passes must be positive".into());
        }
        let quality = canvas_quality(handle);
        let max_passes = match quality {
            0 => 2,
            2 => 8,
            _ => 4,
        };
        let effective_passes = i32::try_from(passes.min(max_passes))
            .map_err(|_| "E_RUNTIME_CANVAS_BLUR_001: blur pass count is too large".to_string())?;
        let radius = int32(radius, "backdrop blur V3 radius")?.min(256);
        let buffers = backbuffers()
            .lock()
            .map_err(|_| "E_RUNTIME_CANVAS_STATE_001: backbuffer state is poisoned".to_string())?;
        let Some(buffer) = buffers.get(&(window as isize)) else {
            return Ok(false);
        };
        let source_dc = buffer.dc as Handle;
        let mut blurred = true;
        if radius > 0 {
            let divisor = effective_passes.saturating_mul(2).max(1);
            let mut factor = 1 + (radius + divisor - 1) / divisor;
            factor = factor.min(match quality {
                0 => 12,
                2 => 5,
                _ => 8,
            });
            let small_width = (buffer.width / factor.max(1)).max(1);
            let small_height = (buffer.height / factor.max(1)).max(1);
            let temporary_dc = unsafe { CreateCompatibleDC(source_dc) };
            let temporary_bitmap = if temporary_dc.is_null() {
                null_mut()
            } else {
                unsafe { CreateCompatibleBitmap(source_dc, small_width, small_height) }
            };
            if temporary_dc.is_null() || temporary_bitmap.is_null() {
                if !temporary_dc.is_null() {
                    unsafe { DeleteDC(temporary_dc) };
                }
                if !temporary_bitmap.is_null() {
                    unsafe { DeleteObject(temporary_bitmap) };
                }
                return Ok(false);
            }
            let previous = unsafe { SelectObject(temporary_dc, temporary_bitmap) };
            if previous.is_null() {
                unsafe {
                    DeleteDC(temporary_dc);
                    DeleteObject(temporary_bitmap);
                }
                return Ok(false);
            }
            let stretch_mode = if quality == 0 { COLORONCOLOR } else { HALFTONE };
            unsafe {
                SetStretchBltMode(temporary_dc, stretch_mode);
                SetStretchBltMode(source_dc, stretch_mode);
            }
            for _ in 0..effective_passes {
                let downsampled = unsafe {
                    StretchBlt(
                        temporary_dc,
                        0,
                        0,
                        small_width,
                        small_height,
                        source_dc,
                        0,
                        0,
                        buffer.width,
                        buffer.height,
                        SRCCOPY,
                    )
                } != 0;
                blurred = downsampled
                    && unsafe {
                        StretchBlt(
                            source_dc,
                            0,
                            0,
                            buffer.width,
                            buffer.height,
                            temporary_dc,
                            0,
                            0,
                            small_width,
                            small_height,
                            SRCCOPY,
                        )
                    } != 0;
                if !blurred {
                    break;
                }
            }
            unsafe {
                SelectObject(temporary_dc, previous);
                DeleteDC(temporary_dc);
                DeleteObject(temporary_bitmap);
            }
        }
        let tint_alpha = ((tint_color as u64 >> 24) & 0xff) as u8;
        if blurred && tint_alpha != 0 {
            let (red, green, blue) = rgb_components(tint_color);
            let pixel = premultiplied_bgra(red, green, blue, tint_alpha);
            blurred = alpha_blend_pixels_to_dc(
                source_dc,
                0,
                0,
                buffer.width,
                buffer.height,
                1,
                1,
                &[pixel],
            );
        }
        Ok(blurred)
    }

    fn frame_target_hz(handle: i64) -> Result<i32, String> {
        let window = hwnd(handle);
        let (mode, target) = {
            let states = canvas_states()
                .lock()
                .map_err(|_| "E_RUNTIME_CANVAS_STATE_001: canvas state is poisoned".to_string())?;
            states
                .get(&(window as isize))
                .map_or((0, 0), |state| (state.frame_mode, state.target_fps))
        };
        Ok(match mode {
            1 => target,
            2 if target > 0 => target,
            2 => window_refresh_hz(handle)?.clamp(1, 1000) as i32,
            _ => 0,
        })
    }

    fn pace_frame(handle: i64) -> Result<(), String> {
        let target_hz = frame_target_hz(handle)?;
        if target_hz <= 0 {
            return Ok(());
        }
        let last_present = canvas_states()
            .lock()
            .map_err(|_| "E_RUNTIME_CANVAS_STATE_001: canvas state is poisoned".to_string())?
            .get(&(hwnd(handle) as isize))
            .and_then(|state| state.last_present);
        let Some(last_present) = last_present else {
            return Ok(());
        };
        let interval = Duration::from_secs_f64(1.0 / f64::from(target_hz));
        let elapsed = last_present.elapsed();
        if elapsed < interval {
            thread::sleep(interval - elapsed);
        }
        Ok(())
    }

    fn record_present(handle: i64, now: Instant) -> Result<(), String> {
        let mut states = canvas_states()
            .lock()
            .map_err(|_| "E_RUNTIME_CANVAS_STATE_001: canvas state is poisoned".to_string())?;
        let state = states.entry(hwnd(handle) as isize).or_default();
        state.frame_delta_ms = state.last_present.map_or(0, |previous| {
            now.duration_since(previous).as_millis() as i64
        });
        state.last_present = Some(now);
        Ok(())
    }

    pub(crate) fn canvas_present(handle: i64) -> Result<bool, String> {
        let window = hwnd(handle);
        if unsafe { IsWindow(window) } == 0 {
            return Ok(false);
        }
        pace_frame(handle)?;
        let buffers = backbuffers()
            .lock()
            .map_err(|_| "E_RUNTIME_CANVAS_STATE_001: backbuffer state is poisoned".to_string())?;
        if let Some(buffer) = buffers.get(&(window as isize)) {
            let window_dc = unsafe { GetDC(window) };
            if window_dc.is_null() {
                return Ok(false);
            }
            let copied = unsafe {
                BitBlt(
                    window_dc,
                    0,
                    0,
                    buffer.width,
                    buffer.height,
                    buffer.dc as Handle,
                    0,
                    0,
                    SRCCOPY,
                )
            } != 0;
            unsafe { ReleaseDC(window, window_dc) };
            let ok = copied && unsafe { GdiFlush() } != 0;
            if ok {
                record_present(handle, Instant::now())?;
            }
            return Ok(ok);
        }
        let ok = unsafe { GdiFlush() } != 0;
        if ok {
            record_present(handle, Instant::now())?;
        }
        Ok(ok)
    }

    fn sample_client_pointer(handle: i64) -> Option<Point> {
        let mut point = Point::default();
        if unsafe { GetCursorPos(&mut point) } == 0
            || unsafe { ScreenToClient(hwnd(handle), &mut point) } == 0
        {
            return None;
        }
        Some(point)
    }

    fn mouse_axis(handle: i64, x_axis: bool) -> Result<i64, String> {
        let Some(point) = sample_client_pointer(handle) else {
            return Ok(-1);
        };
        Ok(i64::from(if x_axis { point.x } else { point.y }))
    }

    pub(crate) fn mouse_x(handle: i64) -> Result<i64, String> {
        mouse_axis(handle, true)
    }

    pub(crate) fn mouse_y(handle: i64) -> Result<i64, String> {
        mouse_axis(handle, false)
    }

    pub(crate) fn input_down(key: i64) -> Result<bool, String> {
        let key = int32(key, "virtual key")?;
        Ok((unsafe { GetAsyncKeyState(key) } as u16 & 0x8000) != 0)
    }

    pub(crate) fn window_read_key(handle: i64) -> Result<i64, String> {
        let mut msg: Msg = unsafe { zeroed() };
        let found =
            unsafe { PeekMessageA(&mut msg, hwnd(handle), WM_KEYDOWN, WM_KEYDOWN, PM_REMOVE) };
        if found == 0 {
            return Ok(0);
        }
        unsafe { TranslateMessage(&msg) };
        Ok(msg.w_param as i64)
    }

    pub(crate) fn window_read_char(handle: i64) -> Result<i64, String> {
        let mut msg: Msg = unsafe { zeroed() };
        let found = unsafe { PeekMessageW(&mut msg, hwnd(handle), WM_CHAR, WM_CHAR, PM_REMOVE) };
        Ok(if found == 0 { 0 } else { msg.w_param as i64 })
    }

    #[allow(dead_code)]
    fn _keep_win32_constants_checked() -> u32 {
        SWP_NOSIZE
    }
}

#[cfg(not(windows))]
mod imp {
    fn unavailable<T>() -> Result<T, String> {
        Err("E_RUNTIME_WINDOW_PLATFORM_001: Win32 window/canvas runtime requires a Windows host or a PE64 build target".into())
    }

    pub(crate) fn window_open(_: &str, _: i64, _: i64) -> Result<i64, String> {
        unavailable()
    }
    pub(crate) fn window_show(_: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn window_hide(_: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn window_poll(_: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn window_wait_event(_: i64, _: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn window_close(_: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn window_is_open(_: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn window_is_focused(_: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn window_set_title(_: i64, _: &str) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn window_set_size(_: i64, _: i64, _: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn window_width(_: i64) -> Result<i64, String> {
        unavailable()
    }
    pub(crate) fn window_height(_: i64) -> Result<i64, String> {
        unavailable()
    }
    pub(crate) fn window_dpi(_: i64) -> Result<i64, String> {
        unavailable()
    }
    pub(crate) fn window_refresh_hz(_: i64) -> Result<i64, String> {
        unavailable()
    }
    pub(crate) fn window_state(_: i64) -> Result<i64, String> {
        unavailable()
    }
    pub(crate) fn window_set_state(_: i64, _: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn window_focus(_: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_clear(_: i64, _: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_begin_frame(_: i64, _: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_set_quality(_: i64, _: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_set_frame_mode(_: i64, _: i64, _: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_frame_delta_ms(_: i64) -> Result<i64, String> {
        unavailable()
    }
    pub(crate) fn canvas_poll_resize(_: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_set_viewport(_: i64, _: i64, _: i64, _: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_pointer_x(_: i64) -> Result<i64, String> {
        unavailable()
    }
    pub(crate) fn canvas_pointer_y(_: i64) -> Result<i64, String> {
        unavailable()
    }
    pub(crate) fn canvas_rect(
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
    ) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_text(_: i64, _: i64, _: i64, _: &str, _: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_round_rect(
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
    ) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_text_styled(
        _: i64,
        _: i64,
        _: i64,
        _: &str,
        _: i64,
        _: i64,
        _: i64,
    ) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_text_centered(
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: &str,
        _: i64,
        _: i64,
        _: i64,
    ) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_shadow_round_rect(
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
    ) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_round_rect_alpha(
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
    ) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_round_rect_gradient(
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
    ) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_line(
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
    ) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_ellipse(
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
        _: i64,
    ) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_present(_: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_backdrop_blur(_: i64, _: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn canvas_backdrop_blur_v3(_: i64, _: i64, _: i64, _: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn mouse_x(_: i64) -> Result<i64, String> {
        unavailable()
    }
    pub(crate) fn mouse_y(_: i64) -> Result<i64, String> {
        unavailable()
    }
    pub(crate) fn input_down(_: i64) -> Result<bool, String> {
        unavailable()
    }
    pub(crate) fn window_read_key(_: i64) -> Result<i64, String> {
        unavailable()
    }
    pub(crate) fn window_read_char(_: i64) -> Result<i64, String> {
        unavailable()
    }
}

pub(crate) use imp::*;

#[cfg(test)]
mod unicode_text_tests {
    use super::*;

    #[test]
    fn canvas_and_window_text_utf8_convert_losslessly_to_utf16() {
        for value in [
            "Configuración",
            "Dónde",
            "Jöns",
            "°C",
            "nC – Tabla Periódica Material V3",
        ] {
            let wide = utf8_bytes_to_utf16_strict(value.as_bytes(), "fixture")
                .expect("valid UTF-8 fixture must convert");
            assert_eq!(
                String::from_utf16(&wide).expect("UTF-16 output must remain valid"),
                value
            );
        }
    }

    #[test]
    fn canvas_text_utf8_rejects_invalid_or_embedded_nul_input() {
        let invalid = utf8_bytes_to_utf16_strict(&[b'J', 0x80], "fixture")
            .expect_err("invalid UTF-8 must never be replaced silently");
        assert!(invalid.contains("E_RUNTIME_WINDOW_UTF8_001"));

        let nul = utf8_bytes_to_utf16_strict(b"Donde\0", "fixture")
            .expect_err("embedded NUL must remain an explicit text error");
        assert!(nul.contains("E_RUNTIME_WINDOW_TEXT_001"));
    }

    #[test]
    fn window_wait_timeout_rejects_negative_and_reserved_infinite_values() {
        assert_eq!(validated_window_wait_timeout(0).unwrap(), 0);
        assert_eq!(validated_window_wait_timeout(250).unwrap(), 250);
        assert_eq!(
            validated_window_wait_timeout(i64::from(u32::MAX) - 1).unwrap(),
            u32::MAX - 1
        );
        for invalid in [-1, i64::from(u32::MAX), i64::MAX] {
            let error = validated_window_wait_timeout(invalid)
                .expect_err("invalid timeout must not reach Win32");
            assert!(error.contains("E_RUNTIME_WINDOW_WAIT_001"), "{error}");
        }
    }

    #[test]
    fn event_pump_budget_is_finite_but_large_enough_for_input_bursts() {
        assert!(WINDOW_POLL_MESSAGE_BUDGET >= 64);
        assert!(WINDOW_POLL_MESSAGE_BUDGET <= 1024);
    }

    #[test]
    fn pointer_pair_cache_returns_one_atomic_y_sample_then_expires() {
        let mut cache = PointerPairCache::default();
        assert_eq!(cache.take_y(), None);
        cache.store_y(481);
        assert_eq!(cache.take_y(), Some(481));
        assert_eq!(cache.take_y(), None);

        cache.store_y(10);
        cache.store_y(11);
        assert_eq!(cache.take_y(), Some(11));
    }
}
