use super::*;

pub(crate) fn runtime_int(v: &RuntimeValue) -> Option<i64> {
    match v {
        RuntimeValue::Number(n) => runtime_parse_int_literal(n),
        _ => None,
    }
}

pub(crate) fn runtime_str(v: &RuntimeValue) -> Option<String> {
    match v {
        RuntimeValue::String(s) => Some(s.clone()),
        _ => None,
    }
}

pub(crate) fn runtime_non_empty_str(v: &RuntimeValue) -> Option<String> {
    let value = runtime_str(v)?;
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn runtime_rich_output_string_arg(
    args: &[RuntimeValue],
    index: usize,
    fn_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    args.get(index).and_then(runtime_str).ok_or_else(|| {
        format!(
            "E_RUNTIME_RICH_OUTPUT_TYPE_001: {fn_name} requires argument {} as str",
            index + 1
        )
        .into()
    })
}

fn runtime_rich_output_int_arg(
    args: &[RuntimeValue],
    index: usize,
    fn_name: &str,
) -> Result<i64, Box<dyn std::error::Error>> {
    args.get(index).and_then(runtime_int).ok_or_else(|| {
        format!(
            "E_RUNTIME_RICH_OUTPUT_TYPE_001: {fn_name} requires argument {} as int",
            index + 1
        )
        .into()
    })
}

fn runtime_rich_output_key(raw: &str) -> String {
    raw.chars()
        .filter(|ch| !matches!(ch, '-' | '_' | ' '))
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn runtime_rich_output_color_enabled() -> bool {
    let no_color = std::env::var("NC_NO_COLOR")
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false);
    !no_color
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeRgb {
    r: u8,
    g: u8,
    b: u8,
}

impl RuntimeRgb {
    const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    fn blend(self, target: RuntimeRgb, amount: f32) -> Self {
        fn mix(a: u8, b: u8, amount: f32) -> u8 {
            let a = a as f32;
            let b = b as f32;
            (a + (b - a) * amount).round().clamp(0.0, 255.0) as u8
        }
        Self::new(
            mix(self.r, target.r, amount),
            mix(self.g, target.g, amount),
            mix(self.b, target.b, amount),
        )
    }

    fn saturate(self, amount: f32) -> Self {
        let gray = (0.299 * self.r as f32 + 0.587 * self.g as f32 + 0.114 * self.b as f32)
            .round()
            .clamp(0.0, 255.0) as u8;
        let base = RuntimeRgb::new(gray, gray, gray);
        base.blend(self, amount)
    }
}

fn runtime_rich_output_truecolor_code(rgb: RuntimeRgb, background: bool) -> String {
    let prefix = if background { 48 } else { 38 };
    format!("{prefix};2;{};{};{}", rgb.r, rgb.g, rgb.b)
}

fn runtime_rich_output_ansi256_code(value: u8, background: bool) -> String {
    let prefix = if background { 48 } else { 38 };
    format!("{prefix};5;{value}")
}

fn runtime_rich_output_parse_u8(raw: &str) -> Option<u8> {
    let value = raw.trim().parse::<u16>().ok()?;
    u8::try_from(value).ok()
}

fn runtime_rich_output_parse_hex_color(raw: &str) -> Option<RuntimeRgb> {
    let text = raw.trim();
    let hex = text
        .strip_prefix('#')
        .or_else(|| text.strip_prefix("0x"))
        .or_else(|| text.strip_prefix("0X"))?;
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some(RuntimeRgb::new(r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(RuntimeRgb::new(r, g, b))
        }
        _ => None,
    }
}

fn runtime_rich_output_parse_rgb_color(raw: &str) -> Option<RuntimeRgb> {
    let text = raw.trim();
    let lower = text.to_ascii_lowercase();
    let lowered = lower.as_str();
    let inner = text
        .strip_prefix("rgb(")
        .and_then(|v| v.strip_suffix(')'))
        .or_else(|| {
            lowered
                .strip_prefix("rgb(")
                .and_then(|v| v.strip_suffix(')'))
                .map(|v| &text[text.len() - v.len() - 1..text.len() - 1])
        })
        .or_else(|| text.strip_prefix("rgb:"))
        .or_else(|| {
            lowered
                .strip_prefix("rgb:")
                .map(|v| &text[text.len() - v.len()..])
        })
        .unwrap_or(text);
    let sep = if inner.contains(',') {
        ','
    } else if inner.contains(':') {
        ':'
    } else {
        return None;
    };
    let parts = inner.split(sep).collect::<Vec<_>>();
    if parts.len() != 3 {
        return None;
    }
    Some(RuntimeRgb::new(
        runtime_rich_output_parse_u8(parts[0])?,
        runtime_rich_output_parse_u8(parts[1])?,
        runtime_rich_output_parse_u8(parts[2])?,
    ))
}

fn runtime_rich_output_parse_ansi256(raw: &str) -> Option<u8> {
    let text = raw.trim();
    let lower = text.to_ascii_lowercase();
    let value = lower
        .strip_prefix("ansi256:")
        .or_else(|| lower.strip_prefix("color256:"))
        .or_else(|| lower.strip_prefix("ansi:"))
        .or_else(|| lower.strip_prefix("256:"))?;
    runtime_rich_output_parse_u8(value)
}

fn runtime_rich_output_named_rgb(key: &str) -> Option<RuntimeRgb> {
    let rgb = match key {
        "black" | "negro" => (0, 0, 0),
        "white" | "blanco" => (255, 255, 255),
        "red" | "rojo" => (255, 0, 0),
        "green" | "verde" => (0, 128, 0),
        "blue" | "azul" => (0, 0, 255),
        "yellow" | "amarillo" => (255, 255, 0),
        "orange" | "naranja" => (255, 165, 0),
        "cyan" | "cian" => (0, 255, 255),
        "magenta" | "fuchsia" | "purple" | "morado" | "violet" | "violeta" | "lila" => {
            (255, 0, 255)
        }
        "gray" | "grey" | "gris" => (128, 128, 128),
        "aliceblue" => (240, 248, 255),
        "antiquewhite" => (250, 235, 215),
        "aqua" => (0, 255, 255),
        "aquamarine" => (127, 255, 212),
        "azure" => (240, 255, 255),
        "beige" => (245, 245, 220),
        "bisque" => (255, 228, 196),
        "blanchedalmond" => (255, 235, 205),
        "blueviolet" => (138, 43, 226),
        "brown" | "marron" | "cafe" => (165, 42, 42),
        "burlywood" => (222, 184, 135),
        "cadetblue" => (95, 158, 160),
        "chartreuse" => (127, 255, 0),
        "chocolate" => (210, 105, 30),
        "coral" => (255, 127, 80),
        "cornflowerblue" => (100, 149, 237),
        "cornsilk" => (255, 248, 220),
        "crimson" | "carmesi" => (220, 20, 60),
        "darkblue" | "azuloscuro" => (0, 0, 139),
        "darkcyan" | "cianoscuro" => (0, 139, 139),
        "darkgoldenrod" => (184, 134, 11),
        "darkgray" | "darkgrey" | "grisoscuro" => (169, 169, 169),
        "darkgreen" | "verdeoscuro" => (0, 100, 0),
        "darkkhaki" => (189, 183, 107),
        "darkmagenta" | "moradooscuro" => (139, 0, 139),
        "darkolivegreen" => (85, 107, 47),
        "darkorange" | "naranjaoscuro" => (255, 140, 0),
        "darkorchid" => (153, 50, 204),
        "darkred" | "rojooscuro" => (139, 0, 0),
        "darksalmon" => (233, 150, 122),
        "darkseagreen" => (143, 188, 143),
        "darkslateblue" => (72, 61, 139),
        "darkslategray" | "darkslategrey" => (47, 79, 79),
        "darkturquoise" => (0, 206, 209),
        "darkviolet" | "violetaoscuro" => (148, 0, 211),
        "deeppink" => (255, 20, 147),
        "deepskyblue" => (0, 191, 255),
        "dimgray" | "dimgrey" => (105, 105, 105),
        "dodgerblue" => (30, 144, 255),
        "firebrick" => (178, 34, 34),
        "floralwhite" => (255, 250, 240),
        "forestgreen" => (34, 139, 34),
        "gainsboro" => (220, 220, 220),
        "ghostwhite" => (248, 248, 255),
        "gold" | "oro" => (255, 215, 0),
        "goldenrod" => (218, 165, 32),
        "greenyellow" => (173, 255, 47),
        "honeydew" => (240, 255, 240),
        "hotpink" => (255, 105, 180),
        "indianred" => (205, 92, 92),
        "indigo" => (75, 0, 130),
        "ivory" => (255, 255, 240),
        "khaki" => (240, 230, 140),
        "lavender" => (230, 230, 250),
        "lavenderblush" => (255, 240, 245),
        "lawngreen" => (124, 252, 0),
        "lemonchiffon" => (255, 250, 205),
        "lightblue" | "azulclaro" => (173, 216, 230),
        "lightcoral" => (240, 128, 128),
        "lightcyan" | "cianclaro" => (224, 255, 255),
        "lightgoldenrodyellow" => (250, 250, 210),
        "lightgray" | "lightgrey" | "grisclaro" => (211, 211, 211),
        "lightgreen" | "verdeclaro" => (144, 238, 144),
        "lightpink" | "rosaclaro" => (255, 182, 193),
        "lightsalmon" => (255, 160, 122),
        "lightseagreen" => (32, 178, 170),
        "lightskyblue" => (135, 206, 250),
        "lightslategray" | "lightslategrey" => (119, 136, 153),
        "lightsteelblue" => (176, 196, 222),
        "lightyellow" | "amarilloclaro" => (255, 255, 224),
        "lime" => (0, 255, 0),
        "limegreen" => (50, 205, 50),
        "linen" => (250, 240, 230),
        "maroon" => (128, 0, 0),
        "mediumaquamarine" => (102, 205, 170),
        "mediumblue" => (0, 0, 205),
        "mediumorchid" => (186, 85, 211),
        "mediumpurple" => (147, 112, 219),
        "mediumseagreen" => (60, 179, 113),
        "mediumslateblue" => (123, 104, 238),
        "mediumspringgreen" => (0, 250, 154),
        "mediumturquoise" => (72, 209, 204),
        "mediumvioletred" => (199, 21, 133),
        "midnightblue" => (25, 25, 112),
        "mintcream" => (245, 255, 250),
        "mistyrose" => (255, 228, 225),
        "moccasin" => (255, 228, 181),
        "navajowhite" => (255, 222, 173),
        "navy" => (0, 0, 128),
        "oldlace" => (253, 245, 230),
        "olive" => (128, 128, 0),
        "olivedrab" => (107, 142, 35),
        "orangered" => (255, 69, 0),
        "orchid" => (218, 112, 214),
        "palegoldenrod" => (238, 232, 170),
        "palegreen" => (152, 251, 152),
        "paleturquoise" => (175, 238, 238),
        "palevioletred" => (219, 112, 147),
        "papayawhip" => (255, 239, 213),
        "peachpuff" => (255, 218, 185),
        "peru" => (205, 133, 63),
        "pink" | "rosa" => (255, 192, 203),
        "plum" => (221, 160, 221),
        "powderblue" => (176, 224, 230),
        "rebeccapurple" => (102, 51, 153),
        "rosybrown" => (188, 143, 143),
        "royalblue" => (65, 105, 225),
        "saddlebrown" => (139, 69, 19),
        "salmon" => (250, 128, 114),
        "sandybrown" => (244, 164, 96),
        "seagreen" => (46, 139, 87),
        "seashell" => (255, 245, 238),
        "sienna" => (160, 82, 45),
        "silver" | "plata" => (192, 192, 192),
        "skyblue" => (135, 206, 235),
        "slateblue" => (106, 90, 205),
        "slategray" | "slategrey" => (112, 128, 144),
        "snow" => (255, 250, 250),
        "springgreen" => (0, 255, 127),
        "steelblue" => (70, 130, 180),
        "tan" => (210, 180, 140),
        "teal" => (0, 128, 128),
        "thistle" => (216, 191, 216),
        "tomato" => (255, 99, 71),
        "turquoise" => (64, 224, 208),
        "wheat" => (245, 222, 179),
        "whitesmoke" => (245, 245, 245),
        "yellowgreen" => (154, 205, 50),
        _ => return None,
    };
    Some(RuntimeRgb::new(rgb.0, rgb.1, rgb.2))
}

fn runtime_rich_output_derived_rgb(key: &str) -> Option<RuntimeRgb> {
    let derivations = [
        ("claro", "light"),
        ("oscuro", "dark"),
        ("brillante", "bright"),
        ("apagado", "dim"),
        ("pastel", "pastel"),
        ("neon", "neon"),
        ("profundo", "deep"),
        ("light", "light"),
        ("dark", "dark"),
        ("bright", "bright"),
        ("dim", "dim"),
        ("soft", "pastel"),
        ("deep", "deep"),
        ("neon", "neon"),
    ];
    for (marker, mode) in derivations {
        let base_key = key
            .strip_suffix(marker)
            .or_else(|| key.strip_prefix(marker));
        if let Some(base_key) = base_key {
            if base_key.is_empty() {
                continue;
            }
            let base = runtime_rich_output_named_rgb(base_key)?;
            return Some(match mode {
                "light" => base.blend(RuntimeRgb::new(255, 255, 255), 0.35),
                "dark" => base.blend(RuntimeRgb::new(0, 0, 0), 0.35),
                "bright" => base
                    .saturate(1.35)
                    .blend(RuntimeRgb::new(255, 255, 255), 0.12),
                "dim" => base.blend(RuntimeRgb::new(96, 96, 96), 0.35),
                "pastel" => base
                    .saturate(0.55)
                    .blend(RuntimeRgb::new(255, 255, 255), 0.45),
                "neon" => base
                    .saturate(1.6)
                    .blend(RuntimeRgb::new(255, 255, 255), 0.05),
                "deep" => base.saturate(1.25).blend(RuntimeRgb::new(0, 0, 0), 0.22),
                _ => base,
            });
        }
    }
    None
}

fn runtime_rich_output_color_code(name: &str, background: bool) -> Option<String> {
    if let Some(rgb) = runtime_rich_output_parse_hex_color(name)
        .or_else(|| runtime_rich_output_parse_rgb_color(name))
    {
        return Some(runtime_rich_output_truecolor_code(rgb, background));
    }
    if let Some(value) = runtime_rich_output_parse_ansi256(name) {
        return Some(runtime_rich_output_ansi256_code(value, background));
    }
    let key = runtime_rich_output_key(name);
    let rgb =
        runtime_rich_output_named_rgb(&key).or_else(|| runtime_rich_output_derived_rgb(&key))?;
    Some(runtime_rich_output_truecolor_code(rgb, background))
}

fn runtime_rich_output_style_code(name: &str) -> Option<&'static str> {
    match runtime_rich_output_key(name).as_str() {
        "bold" => Some("1"),
        "dim" => Some("2"),
        "italic" => Some("3"),
        "underline" => Some("4"),
        "blink" => Some("5"),
        "inverse" | "invert" => Some("7"),
        "strike" | "strikethrough" => Some("9"),
        _ => None,
    }
}

fn runtime_rich_output_ansi(text: &str, code: &str) -> String {
    if runtime_rich_output_color_enabled() {
        format!("\u{1b}[{code}m{text}\u{1b}[0m")
    } else {
        text.to_string()
    }
}

pub(crate) fn runtime_rich_output_color_call(
    args: &[RuntimeValue],
    background: bool,
    bold: bool,
) -> Result<RuntimeValue, Box<dyn std::error::Error>> {
    let fn_name = if background {
        "bgColor"
    } else if bold {
        "colorBold"
    } else {
        "color"
    };
    let text = runtime_rich_output_string_arg(args, 0, fn_name)?;
    let color = runtime_rich_output_string_arg(args, 1, fn_name)?;
    let Some(color_code) = runtime_rich_output_color_code(&color, background) else {
        return Err(format!(
            "E_RUNTIME_RICH_OUTPUT_COLOR_001: {fn_name} received unknown color '{color}'"
        )
        .into());
    };
    let code = if bold {
        format!("1;{color_code}")
    } else {
        color_code
    };
    Ok(RuntimeValue::String(runtime_rich_output_ansi(&text, &code)))
}

pub(crate) fn runtime_rich_output_style_call(
    args: &[RuntimeValue],
) -> Result<RuntimeValue, Box<dyn std::error::Error>> {
    let text = runtime_rich_output_string_arg(args, 0, "style")?;
    let style = runtime_rich_output_string_arg(args, 1, "style")?;
    let Some(code) = runtime_rich_output_style_code(&style) else {
        return Err(format!(
            "E_RUNTIME_RICH_OUTPUT_STYLE_001: style received unknown style '{style}'"
        )
        .into());
    };
    Ok(RuntimeValue::String(runtime_rich_output_ansi(&text, code)))
}

fn runtime_rich_output_emoji(name: &str) -> Option<&'static str> {
    match runtime_rich_output_key(name).as_str() {
        "rocket" | "ship" => Some("\u{1f680}"),
        "ok" | "check" | "success" => Some("\u{2705}"),
        "warn" | "warning" => Some("\u{26a0}\u{fe0f}"),
        "error" | "fail" | "cross" => Some("\u{274c}"),
        "info" => Some("\u{2139}\u{fe0f}"),
        "spark" | "sparkles" => Some("\u{2728}"),
        "fire" => Some("\u{1f525}"),
        "hourglass" | "wait" => Some("\u{23f3}"),
        "gear" | "settings" => Some("\u{2699}\u{fe0f}"),
        "lock" => Some("\u{1f512}"),
        "unlock" => Some("\u{1f513}"),
        "package" | "box" => Some("\u{1f4e6}"),
        "table" => Some("\u{1f5c2}\u{fe0f}"),
        "pointer" => Some("\u{1f4cd}"),
        _ => None,
    }
}

pub(crate) fn runtime_rich_output_emoji_call(
    args: &[RuntimeValue],
) -> Result<RuntimeValue, Box<dyn std::error::Error>> {
    let name = runtime_rich_output_string_arg(args, 0, "emoji")?;
    let Some(symbol) = runtime_rich_output_emoji(&name) else {
        return Err(format!(
            "E_RUNTIME_RICH_OUTPUT_EMOJI_001: emoji received unknown name '{name}'"
        )
        .into());
    };
    Ok(RuntimeValue::String(symbol.to_string()))
}

pub(crate) fn runtime_rich_output_progress_bar_call(
    args: &[RuntimeValue],
) -> Result<RuntimeValue, Box<dyn std::error::Error>> {
    let label = runtime_rich_output_string_arg(args, 0, "progressBar")?;
    let current = runtime_rich_output_int_arg(args, 1, "progressBar")?;
    let total = runtime_rich_output_int_arg(args, 2, "progressBar")?;
    let width = runtime_rich_output_int_arg(args, 3, "progressBar")?;
    if total <= 0 {
        return Err("E_RUNTIME_RICH_OUTPUT_PROGRESS_001: progressBar requires total > 0".into());
    }
    if !(1..=120).contains(&width) {
        return Err(
            "E_RUNTIME_RICH_OUTPUT_PROGRESS_001: progressBar requires width in 1..=120".into(),
        );
    }
    let clamped_current = current.clamp(0, total);
    let filled = ((clamped_current as i128 * width as i128) / total as i128) as usize;
    let empty = width as usize - filled;
    Ok(RuntimeValue::String(format!(
        "{label} [{}{}] {current}/{total}",
        "#".repeat(filled),
        "-".repeat(empty)
    )))
}

pub(crate) fn runtime_rich_output_spinner_frame_call(
    args: &[RuntimeValue],
) -> Result<RuntimeValue, Box<dyn std::error::Error>> {
    let label = runtime_rich_output_string_arg(args, 0, "spinnerFrame")?;
    let frame = runtime_rich_output_int_arg(args, 1, "spinnerFrame")?;
    let frames = ["-", "\\", "|", "/"];
    let idx = frame.rem_euclid(frames.len() as i64) as usize;
    Ok(RuntimeValue::String(format!("{label} {}", frames[idx])))
}

fn runtime_rich_output_cell_width(text: &str) -> usize {
    text.chars().count()
}

fn runtime_rich_output_pad_cell(text: &str, width: usize) -> String {
    let len = runtime_rich_output_cell_width(text);
    if len >= width {
        text.to_string()
    } else {
        format!("{text}{}", " ".repeat(width - len))
    }
}

fn runtime_rich_output_table_border(widths: &[usize]) -> String {
    let mut out = String::new();
    for width in widths {
        out.push('+');
        out.push_str(&"-".repeat(width + 2));
    }
    out.push('+');
    out
}

fn runtime_rich_output_table_row(cells: &[String], widths: &[usize]) -> String {
    let mut out = String::new();
    for (cell, width) in cells.iter().zip(widths) {
        out.push_str("| ");
        out.push_str(&runtime_rich_output_pad_cell(cell, *width));
        out.push(' ');
    }
    out.push('|');
    out
}

pub(crate) fn runtime_rich_output_table_call(
    args: &[RuntimeValue],
    columns: usize,
) -> Result<RuntimeValue, Box<dyn std::error::Error>> {
    let fn_name = if columns == 2 { "table2" } else { "table3" };
    let mut cells = Vec::with_capacity(columns * 2);
    for index in 0..(columns * 2) {
        cells.push(runtime_rich_output_string_arg(args, index, fn_name)?);
    }
    let mut widths = vec![0usize; columns];
    for col in 0..columns {
        let header_width = runtime_rich_output_cell_width(&cells[col]);
        let value_width = runtime_rich_output_cell_width(&cells[col + columns]);
        widths[col] = header_width.max(value_width);
    }
    let border = runtime_rich_output_table_border(&widths);
    let header = runtime_rich_output_table_row(&cells[..columns], &widths);
    let values = runtime_rich_output_table_row(&cells[columns..], &widths);
    Ok(RuntimeValue::String(format!(
        "{border}\n{header}\n{border}\n{values}\n{border}"
    )))
}

pub(crate) fn runtime_addr(v: &RuntimeValue) -> Option<String> {
    match v {
        RuntimeValue::Pointer(RuntimePointer::Addr(s)) => Some(s.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::runtime_rich_output_color_code;

    fn color_code(name: &str, background: bool) -> Option<String> {
        runtime_rich_output_color_code(name, background)
    }

    #[test]
    fn color_code_accepts_normalized_english_and_spanish_aliases() {
        assert_eq!(color_code("green", false).as_deref(), Some("38;2;0;128;0"));
        assert_eq!(color_code("verde", false).as_deref(), Some("38;2;0;128;0"));
        assert_eq!(
            color_code("azul claro", false).as_deref(),
            Some("38;2;173;216;230")
        );
        assert_eq!(color_code("cian", true).as_deref(), Some("48;2;0;255;255"));
        assert_eq!(
            color_code("morado", false).as_deref(),
            Some("38;2;255;0;255")
        );
    }

    #[test]
    fn color_code_accepts_truecolor_css_and_ansi256() {
        assert_eq!(
            color_code("#1E90FF", false).as_deref(),
            Some("38;2;30;144;255")
        );
        assert_eq!(color_code("#0f8", true).as_deref(), Some("48;2;0;255;136"));
        assert_eq!(
            color_code("rgb(12, 34, 56)", false).as_deref(),
            Some("38;2;12;34;56")
        );
        assert_eq!(
            color_code("ansi256:202", false).as_deref(),
            Some("38;5;202")
        );
        assert_eq!(
            color_code("ANSI256:202", false).as_deref(),
            Some("38;5;202")
        );
        assert_eq!(
            color_code("rebecca purple", false).as_deref(),
            Some("38;2;102;51;153")
        );
        assert!(color_code("rojo neon", false).is_some());
        assert!(color_code("azul profundo", false).is_some());
    }
}
