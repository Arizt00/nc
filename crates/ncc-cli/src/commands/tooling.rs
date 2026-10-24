use super::super::*;

pub(crate) fn run_nfmt_command(args: &[String]) -> Result<(), CliFailure> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!("{}", command_help("nfmt").expect("nfmt help must exist"));
        return Ok(());
    }
    if args.is_empty() {
        return Err(actionable_command_error(
            "nfmt",
            "nfmt syntax: nfmt <file.n> [--check|--compact|--in-place|--out <path>]",
        )
        .into());
    }
    let path = args[0].clone();
    let mut check_only = false;
    let mut in_place = false;
    let mut quiet = false;
    let mut allow_invalid = false;
    // Compact is the canonical source form. `--compact` remains accepted so
    // existing editor tasks and scripts do not need a flag migration.
    let mut compact = true;
    let mut out_path: Option<String> = None;

    let mut idx = 1usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--check" => {
                check_only = true;
                idx += 1;
            }
            "-h" | "--help" | "help" => {
                println!("{}", command_help("nfmt").expect("nfmt help must exist"));
                return Ok(());
            }
            "--in-place" => {
                in_place = true;
                idx += 1;
            }
            "--compact" => {
                compact = true;
                idx += 1;
            }
            "--quiet" => {
                quiet = true;
                idx += 1;
            }
            "--allow-invalid" => {
                allow_invalid = true;
                idx += 1;
            }
            "--out" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("nfmt", "nfmt syntax: --out requires a file path")
                })?;
                out_path = Some(value.clone());
                idx += 2;
            }
            other => {
                return Err(actionable_command_error(
                    "nfmt",
                    format!(
                        "nfmt: unknown option '{}'; expected --check/--compact/--in-place/--out/--quiet/--allow-invalid",
                        other
                    ),
                )
                .into());
            }
        }
    }

    if in_place && out_path.is_some() {
        return Err(actionable_command_error(
            "nfmt",
            "nfmt: use either --in-place or --out <path>, not both",
        )
        .into());
    }

    let src = fs::read_to_string(&path).map_err(|error| {
        CliFailure::filesystem(
            "tooling.nfmt",
            "read formatter input",
            Path::new(&path),
            error,
        )
    })?;
    let fmt = format_nfmt_source_with_policy(&src, compact, allow_invalid)
        .map_err(|error| CliFailure::from_boxed_at(error, "tooling.nfmt"))?;
    if check_only {
        if fmt.changed {
            return Err(CliFailure::error(
                "E_NFMT_CHECK_001",
                "tooling.nfmt",
                "formatting changes are required",
                "run `ncc nfmt <file> --in-place` to apply canonical formatting",
            ));
        }
        if !quiet {
            println!("OK: nfmt check {}", path);
        }
        return Ok(());
    }

    if !fmt.changed {
        if !quiet {
            println!("OK: nfmt no changes {}", path);
        }
        return Ok(());
    }

    if in_place {
        fs::write(&path, fmt.source.as_bytes()).map_err(|error| {
            CliFailure::filesystem(
                "tooling.nfmt",
                "write formatted source",
                Path::new(&path),
                error,
            )
        })?;
        if !quiet {
            println!("OK: nfmt wrote {}", path);
        }
    } else if let Some(out) = out_path {
        fs::write(&out, fmt.source.as_bytes()).map_err(|error| {
            CliFailure::filesystem(
                "tooling.nfmt",
                "write formatted output",
                Path::new(&out),
                error,
            )
        })?;
        if !quiet {
            println!("OK: nfmt wrote {}", out);
        }
    } else if !quiet {
        print!("{}", fmt.source);
    }

    Ok(())
}

#[cfg(test)]
fn format_nfmt_source(
    src: &str,
    compact: bool,
) -> Result<PrimitiveBaseResult, Box<dyn std::error::Error>> {
    format_nfmt_source_with_policy(src, compact, false)
}

fn format_nfmt_source_with_policy(
    src: &str,
    compact: bool,
    allow_invalid: bool,
) -> Result<PrimitiveBaseResult, Box<dyn std::error::Error>> {
    reject_removed_nfmt_surface(src)?;
    if compact {
        let canonical = material_v2_from_compact_source(src);
        if let Ok(tokens) = ncc_lexer::lex(&canonical) {
            if ncc_parser::validate_material_v2_surface(&tokens).is_ok()
                && ncc_parser::parse_material_v2(tokens).is_ok()
            {
                return Ok(PrimitiveBaseResult {
                    changed: canonical != src,
                    source: canonical,
                });
            }
        }
        if [
            " :: program {",
            " :: runtime {",
            " :: system {",
            " :: kernel {",
            " :: machine {",
            " :: primitive {",
            " :: basal {",
        ]
        .iter()
        .any(|marker| src.contains(marker))
        {
            let material = material_v2_from_compact_source(src);
            if let Ok(tokens) = ncc_lexer::lex(&material) {
                if ncc_parser::parse_material_v2(tokens).is_ok() {
                    return Ok(PrimitiveBaseResult {
                        changed: material != src,
                        source: material,
                    });
                }
            }
        }
        if allow_invalid {
            return Ok(PrimitiveBaseResult {
                changed: canonical != src,
                source: canonical,
            });
        }
    }

    let base = baseize_primitive_source(src)?;
    if !compact {
        return Ok(base);
    }

    let compacted = compact_n_source(&base.source)?;
    let material = material_v2_from_compact_source(&compacted);
    let changed = base.changed || material != base.source;
    if changed {
        let tokens = ncc_lexer::lex(&material)?;
        if let Err(error) = ncc_parser::parse_material_v2(tokens) {
            let source_line = material
                .lines()
                .nth(error.line.saturating_sub(1))
                .unwrap_or("<line unavailable>");
            return Err(format!(
                "nfmt generated invalid canonical source: {}\n{} | {}",
                error, error.line, source_line
            )
            .into());
        }
    }
    Ok(PrimitiveBaseResult {
        source: material,
        changed,
    })
}

fn reject_removed_nfmt_surface(src: &str) -> Result<(), Box<dyn std::error::Error>> {
    let Ok(tokens) = ncc_lexer::lex(src) else {
        return Ok(());
    };
    if let Err(error) = ncc_parser::validate_material_v2_surface(&tokens) {
        let rendered = error.to_string();
        if rendered.contains("E_STC_CANONICAL_001")
            || rendered.contains("E_USE_PROJECT_CANONICAL_001")
        {
            return Err(Box::new(error));
        }
    }
    Ok(())
}

fn material_v2_from_compact_source(src: &str) -> String {
    let completed_projections = material_v2_complete_projection_chains(src);
    let normalized_imports = compact_imports(&completed_projections);
    let merged_state = material_v2_merge_adjacent_state_blocks(&normalized_imports);
    let separated_constants = material_v2_extract_state_constants(&merged_state);
    let compact_source = material_v2_collapse_generated_final(&separated_constants);
    let lines = compact_source.lines().collect::<Vec<_>>();
    let mut out = Vec::<String>::new();
    let mut compact_state_indents = Vec::<usize>::new();
    let mut source_depth = 0usize;
    let mut in_block_comment = false;

    for line in lines.iter().copied() {
        let trimmed = line.trim_start();
        let indent = line.len().saturating_sub(trimmed.len());

        if compact_state_indents.last().is_some_and(|state_indent| {
            indent == *state_indent && (trimmed == "}" || trimmed.starts_with("} //"))
        }) {
            if let Some(comment) = trimmed.strip_prefix('}').map(str::trim) {
                if !comment.is_empty() {
                    out.push(format!("{}{}", " ".repeat(indent), comment));
                }
            }
            compact_state_indents.pop();
            source_depth = update_brace_depth(line, source_depth);
            in_block_comment = update_block_comment_state(line, in_block_comment);
            continue;
        }

        if !in_block_comment && trimmed == "state {" {
            out.push(format!("{}stc:", " ".repeat(indent)));
            compact_state_indents.push(indent);
            source_depth = update_brace_depth(line, source_depth);
            in_block_comment = update_block_comment_state(line, in_block_comment);
            continue;
        }
        if source_depth == 0 {
            if let Some(entry) = plain_import_line(trimmed).and_then(parse_compact_import_line) {
                out.push(render_compact_import_entry(&entry));
                in_block_comment = update_block_comment_state(line, in_block_comment);
                continue;
            }
        }

        if source_depth == 0 {
            if let Some(definition) = material_v2_global_const_definition(trimmed) {
                out.push(format!("{}{}", " ".repeat(indent), definition));
                in_block_comment = update_block_comment_state(line, in_block_comment);
                continue;
            }
            if let Some(definition) = material_v2_global_type_definition(trimmed) {
                out.push(format!("{}{}", " ".repeat(indent), definition));
                source_depth = update_brace_depth(line, source_depth);
                in_block_comment = update_block_comment_state(line, in_block_comment);
                continue;
            }
        }

        if source_depth == 0 {
            if let Some(header) = material_v2_container_header(trimmed) {
                out.push(format!("{}{}", " ".repeat(indent), header));
                source_depth = update_brace_depth(line, source_depth);
                in_block_comment = update_block_comment_state(line, in_block_comment);
                continue;
            }
        }

        let rendered = if in_block_comment || trimmed.starts_with("/*") || trimmed.starts_with("//")
        {
            line.to_string()
        } else {
            material_v2_rewrite_code_line(line)
        };
        out.push(rendered);

        if !in_block_comment && !trimmed.starts_with("/*") {
            source_depth = update_brace_depth(line, source_depth);
        }
        in_block_comment = update_block_comment_state(line, in_block_comment);
    }

    let mut rendered = out.join("\n");
    if compact_source.ends_with('\n') {
        rendered.push('\n');
    }
    let grouped =
        material_v2_group_direct_functions(&material_v2_compact_type_contract_functions(&rendered));
    normalize_nfmt_trailing_whitespace(&grouped)
}

fn normalize_nfmt_trailing_whitespace(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut in_string = false;
    let mut in_char = false;
    let mut in_block_comment = false;
    let mut escaped = false;

    for raw_line in src.split_inclusive('\n') {
        let has_newline = raw_line.ends_with('\n');
        let without_newline = raw_line.strip_suffix('\n').unwrap_or(raw_line);
        let line = without_newline
            .strip_suffix('\r')
            .unwrap_or(without_newline);
        let chars = line.as_bytes();
        let mut index = 0usize;
        let mut line_comment = false;

        while index < chars.len() {
            let byte = chars[index];
            let next = chars.get(index + 1).copied();

            if line_comment {
                break;
            }
            if in_string {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    in_string = false;
                }
                index += 1;
                continue;
            }
            if in_char {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'\'' {
                    in_char = false;
                }
                index += 1;
                continue;
            }
            if in_block_comment {
                if byte == b'*' && next == Some(b'/') {
                    in_block_comment = false;
                    index += 2;
                } else {
                    index += 1;
                }
                continue;
            }

            match (byte, next) {
                (b'/', Some(b'/')) => line_comment = true,
                (b'/', Some(b'*')) => {
                    in_block_comment = true;
                    index += 1;
                }
                (b'"', _) => in_string = true,
                (b'\'', _) => in_char = true,
                _ => {}
            }
            index += 1;
        }

        if in_string {
            out.push_str(line);
        } else {
            out.push_str(line.trim_end_matches([' ', '\t']));
        }
        if has_newline {
            out.push('\n');
        }
    }

    out
}

fn material_v2_merge_adjacent_state_blocks(src: &str) -> String {
    let lines = src.lines().collect::<Vec<_>>();
    let mut out = Vec::<String>::with_capacity(lines.len());
    let mut index = 0usize;
    let mut state_indent = None::<usize>;
    let mut state_depth = 0usize;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim_start();
        let indent = line.len().saturating_sub(trimmed.len());

        if state_indent.is_none() && matches!(trimmed, "state {" | "static {") {
            state_indent = Some(indent);
            state_depth = 1;
            out.push(format!("{}state {{", " ".repeat(indent)));
            index += 1;
            continue;
        }

        if state_indent.is_some() {
            let next_depth = update_brace_depth(line, state_depth);
            if next_depth == 0 && trimmed == "}" && state_indent == Some(indent) {
                let mut next = index + 1;
                while next < lines.len() && lines[next].trim().is_empty() {
                    next += 1;
                }
                if next < lines.len() {
                    let next_trimmed = lines[next].trim_start();
                    let next_indent = lines[next].len().saturating_sub(next_trimmed.len());
                    if matches!(next_trimmed, "state {" | "static {") && next_indent == indent {
                        state_depth = 1;
                        index = next + 1;
                        continue;
                    }
                }
                state_indent = None;
            }
            state_depth = next_depth;
        }

        out.push(line.to_string());
        index += 1;
    }
    let mut rendered = out.join("\n");
    if src.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

fn material_v2_extract_state_constants(src: &str) -> String {
    let lines = src.lines().collect::<Vec<_>>();
    let mut out = Vec::<String>::with_capacity(lines.len());
    let mut index = 0usize;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim_start();
        if trimmed != "state {" {
            out.push(line.to_string());
            index += 1;
            continue;
        }

        let state_indent = line.len().saturating_sub(trimmed.len());
        let mut depth = 1usize;
        let mut cursor = index + 1;
        let mut constants = Vec::<String>::new();
        let mut body = Vec::<String>::new();
        let mut closing = None::<String>;
        while cursor < lines.len() {
            let current = lines[cursor];
            let current_trimmed = current.trim_start();
            let next_depth = update_brace_depth(current, depth);
            if depth == 1
                && next_depth == 0
                && current_trimmed.starts_with('}')
                && current.len().saturating_sub(current_trimmed.len()) == state_indent
            {
                closing = Some(current.to_string());
                break;
            }
            if depth == 1 {
                if let Some(constant) = material_v2_state_const_decl(current) {
                    constants.push(constant);
                    depth = next_depth;
                    cursor += 1;
                    continue;
                }
            }
            body.push(current.to_string());
            depth = next_depth;
            cursor += 1;
        }

        if constants.is_empty() || closing.is_none() {
            out.push(line.to_string());
            index += 1;
            continue;
        }
        out.push(format!("{}const:", " ".repeat(state_indent)));
        out.extend(constants);
        out.push(String::new());
        out.push(line.to_string());
        out.extend(body);
        out.push(closing.expect("state closing checked"));
        index = cursor + 1;
    }
    let mut rendered = out.join("\n");
    if src.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

fn material_v2_state_const_decl(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let indent = line.len().saturating_sub(trimmed.len());
    let (code, comment) = trimmed
        .split_once("//")
        .map_or((trimmed, ""), |(code, comment)| (code.trim_end(), comment));
    let body = code.strip_prefix("const ")?.strip_suffix(';')?.trim();
    let (left, value) = body
        .split_once("<-")
        .or_else(|| body.split_once(":="))
        .or_else(|| body.split_once('='))?;
    let (name, ty) = left.split_once(">>").or_else(|| left.split_once(':'))?;
    let name = name.trim();
    let ty = ty.trim();
    if !is_simple_ident(name) || ty.is_empty() {
        return None;
    }
    let suffix = if comment.is_empty() {
        String::new()
    } else {
        format!(" //{comment}")
    };
    Some(format!(
        "{}{} >> {} <- {};{}",
        " ".repeat(indent),
        name,
        ty,
        value.trim(),
        suffix
    ))
}

fn material_v2_direct_function_header(trimmed: &str) -> Option<String> {
    for marker in [" :: in(", " :: fn("] {
        if let Some(index) = trimmed.find(marker) {
            let prefix = trimmed[..index].trim();
            let (modifier, name) = prefix
                .strip_prefix("async ")
                .map_or(("", prefix), |name| ("async ", name.trim()));
            if is_simple_ident(name) {
                return Some(format!(
                    "{modifier}{name}({}",
                    &trimmed[index + marker.len()..]
                ));
            }
        }
    }
    None
}

fn material_v2_compact_type_contract_functions(src: &str) -> String {
    let mut rendered = src
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            let indent = &line[..line.len().saturating_sub(trimmed.len())];
            for marker in [" :: in(", " :: fn("] {
                if let Some(index) = trimmed.find(marker) {
                    let name = trimmed[..index].trim();
                    if matches!(name, "static" | "default" | "move" | "copy" | "region") {
                        return format!("{indent}{name}({}", &trimmed[index + marker.len()..]);
                    }
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    if src.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

fn material_v2_group_direct_functions(src: &str) -> String {
    let mut out = Vec::<String>::new();
    let mut depth = 0usize;
    let mut in_group = false;
    let mut in_block_comment = false;

    for line in src.lines() {
        let trimmed = line.trim_start();
        let indent = line.len().saturating_sub(trimmed.len());
        let direct_header = (!in_block_comment && depth == 1)
            .then(|| material_v2_direct_function_header(trimmed))
            .flatten();

        if let Some(header) = direct_header {
            if !in_group {
                out.push(format!("{}fn:", " ".repeat(indent)));
                in_group = true;
            }
            out.push(format!("{}{}", " ".repeat(indent + 4), header));
        } else if in_group && depth > 1 {
            out.push(format!("    {line}"));
        } else if in_group
            && (trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*"))
        {
            out.push(if trimmed.is_empty() {
                String::new()
            } else {
                format!("    {line}")
            });
        } else {
            in_group = false;
            out.push(line.to_string());
        }

        if !in_block_comment && !trimmed.starts_with("/*") {
            depth = update_brace_depth(line, depth);
        }
        in_block_comment = update_block_comment_state(line, in_block_comment);
    }

    let mut rendered = out.join("\n");
    if src.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

fn material_v2_collapse_generated_final(src: &str) -> String {
    if src.contains("ref main ->> shutdown;") || src.contains("shutdown();") {
        return src.to_string();
    }
    let lines = src.lines().collect::<Vec<_>>();
    let mut out = Vec::<String>::with_capacity(lines.len());
    let mut index = 0usize;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim_start();
        let indent = line.len().saturating_sub(trimmed.len());
        let generated_final = trimmed == "shutdown :: in() -> bool {"
            && lines.get(index + 1).is_some_and(|next| {
                let body = next.trim();
                body.starts_with("return ") && body.ends_with(';')
            })
            && lines.get(index + 2).is_some_and(|next| {
                next.trim() == "}" && next.len().saturating_sub(next.trim_start().len()) == indent
            });
        if generated_final {
            out.push(format!("{}fa:", " ".repeat(indent)));
            out.push(format!(
                "{}{}",
                " ".repeat(indent + 4),
                lines[index + 1].trim()
            ));
            index += 3;
            continue;
        }
        out.push(line.to_string());
        index += 1;
    }
    let mut rendered = out.join("\n");
    if src.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

fn material_v2_rewrite_code_line(line: &str) -> String {
    let comment_start = material_v2_comment_start(line).unwrap_or(line.len());
    let (code, comment) = line.split_at(comment_start);
    let trimmed = code.trim();
    if let Some(target) = compact_main_entry(trimmed) {
        let indent = &code[..code.len().saturating_sub(code.trim_start().len())];
        return format!("{indent}ref main ->> {target};{comment}");
    }
    let mut rendered = if looks_like_function_header(code.trim_start())
        && code
            .find('(')
            .is_some_and(|open| !code[..open].contains(">>"))
    {
        let open = code
            .find('(')
            .expect("function header has an opening parenthesis");
        format!(
            "{}{}",
            &code[..open],
            compact_function_params_surface(&code[open..])
        )
    } else {
        code.to_string()
    };
    rendered = rendered.replace(" :: fn(", " :: in(");
    rendered = material_v2_replace_removed_initializer(&rendered);
    rendered = material_v2_replace_colon_place_decl(&rendered);
    rendered = material_v2_replace_projection_colon_init(&rendered);
    rendered = material_v2_rewrite_mem_projection(&rendered);
    rendered = material_v2_replace_plain_assignment(&rendered);
    rendered = material_v2_rewrite_context_suffix(&rendered);
    rendered = material_v2_rewrite_fn_compat_calls(&rendered);
    rendered = material_v2_rewrite_probe_call(&rendered);
    rendered = material_v2_strip_context_sigil(&rendered);
    rendered = compact_surface_aliases(&rendered);
    rendered.push_str(comment);
    rendered
}

fn material_v2_complete_projection_chains(src: &str) -> String {
    let lines = src.lines().collect::<Vec<_>>();
    let mut out = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().copied().enumerate() {
        let code = line
            .split_once("//")
            .map_or(line, |(code, _)| code)
            .trim_end();
        let next = lines
            .get(index + 1)
            .map(|next| next.split_once("//").map_or(*next, |(code, _)| code).trim())
            .unwrap_or_default();
        let projection_name = code.find(">>").and_then(|shift| {
            let left = code[..shift].trim().trim_start_matches('@');
            let tail = &code[shift + 2..];
            (is_simple_ident(left)
                && !code.ends_with(';')
                && (tail.contains(':') || tail.contains("<-")))
            .then_some(left)
        });
        let continues_with_failure = projection_name.is_some_and(|name| {
            let next = next.trim_start_matches('@');
            next.strip_prefix(name)
                .is_some_and(|tail| tail.trim_start().starts_with("!>"))
        });
        if continues_with_failure {
            let insert = line.find("//").unwrap_or(line.len());
            let mut completed = String::with_capacity(line.len() + 1);
            completed.push_str(line[..insert].trim_end());
            completed.push(';');
            if insert < line.len() {
                completed.push(' ');
                completed.push_str(&line[insert..]);
            }
            out.push(completed);
        } else {
            out.push(line.to_string());
        }
    }
    let mut rendered = out.join("\n");
    if src.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

fn material_v2_replace_removed_initializer(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut quote = None::<char>;
    let mut escaped = false;
    while index < chars.len() {
        let ch = chars[index];
        if let Some(active_quote) = quote {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }
        if matches!(ch, '"' | '\'') {
            quote = Some(ch);
            out.push(ch);
            index += 1;
            continue;
        }
        if ch == ':' && chars.get(index + 1) == Some(&'=') {
            out.push_str("<-");
            index += 2;
            continue;
        }
        out.push(ch);
        index += 1;
    }
    out
}

fn material_v2_comment_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut index = 0usize;
    let mut quote = None::<u8>;
    let mut escaped = false;
    while index + 1 < bytes.len() {
        let byte = bytes[index];
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
            index += 1;
            continue;
        }
        if byte == b'/' && matches!(bytes[index + 1], b'/' | b'*') {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn material_v2_container_header(trimmed: &str) -> Option<String> {
    let (code, comment) = trimmed
        .split_once("//")
        .map_or((trimmed, ""), |(code, comment)| (code.trim_end(), comment));
    if !code.ends_with('{') || code.contains(" :: in(") || code.contains(" :: type") {
        return None;
    }
    if ["struct ", "structure ", "package ", "module "]
        .iter()
        .any(|prefix| code.starts_with(prefix))
    {
        return None;
    }
    let body = code.trim_end_matches('{').trim();
    let (name, role) = if let Some(name) = body.strip_suffix("[primitive[@]]") {
        (name.trim(), "primitive")
    } else if let Some((name, role)) = body.split_once(" :: ") {
        (name.trim(), material_v2_role_for_context(role.trim()))
    } else if let Some(rest) = body.strip_prefix("primitive[@]") {
        (rest.trim(), "primitive")
    } else if let Some(rest) = body.strip_prefix("[@") {
        let (context, name) = rest.split_once("]")?;
        (name.trim(), material_v2_role_for_context(context))
    } else if let Some(start) = body.find("[@") {
        let name = body[..start].trim();
        let context = body[start + 2..].strip_suffix(']')?;
        (name, material_v2_role_for_context(context))
    } else {
        (body, "program")
    };
    let name = name
        .strip_prefix("public ")
        .or_else(|| name.strip_prefix("private "))
        .or_else(|| name.strip_prefix("internal "))
        .or_else(|| name.strip_prefix("strictly "))
        .unwrap_or(name)
        .trim()
        .split('[')
        .next()
        .unwrap_or(name)
        .trim();
    if name.is_empty() || name.contains(char::is_whitespace) {
        return None;
    }
    let suffix = if comment.is_empty() {
        String::new()
    } else {
        format!(" //{comment}")
    };
    let inferred_role = if role == "program" && name.to_ascii_lowercase().starts_with("kernel") {
        "kernel"
    } else {
        role
    };
    if inferred_role == "program" {
        Some(format!("{name} {{{suffix}"))
    } else {
        Some(format!("{name} :: {inferred_role} {{{suffix}"))
    }
}

fn material_v2_global_const_definition(trimmed: &str) -> Option<String> {
    let (code, comment) = trimmed
        .split_once("//")
        .map_or((trimmed, ""), |(code, comment)| (code.trim_end(), comment));
    let body = code.strip_prefix("const ")?.strip_suffix(';')?.trim();
    let (left, value) = body
        .split_once("<-")
        .or_else(|| body.split_once(":="))
        .or_else(|| body.split_once('='))?;
    let left = left.trim();
    let (name, ty) = if let Some((name, ty)) = left.split_once(">>") {
        (name.trim(), Some(ty.trim()))
    } else if let Some((name, ty)) = left.split_once(':') {
        (name.trim(), Some(ty.trim()))
    } else {
        (left, None)
    };
    if !is_simple_ident(name) || ty.is_some_and(str::is_empty) {
        return None;
    }
    let type_part = ty.map_or(String::new(), |ty| format!(" {ty}"));
    let suffix = if comment.is_empty() {
        String::new()
    } else {
        format!(" //{comment}")
    };
    Some(format!(
        "{name} :: const{type_part} <- {};{suffix}",
        value.trim()
    ))
}

fn material_v2_global_type_definition(trimmed: &str) -> Option<String> {
    let (code, comment) = trimmed
        .split_once("//")
        .map_or((trimmed, ""), |(code, comment)| (code.trim_end(), comment));
    let body = code
        .strip_prefix("struct ")
        .or_else(|| code.strip_prefix("structure "))?
        .strip_suffix('{')?
        .trim();
    if !is_simple_ident(body) {
        return None;
    }
    let suffix = if comment.is_empty() {
        String::new()
    } else {
        format!(" //{comment}")
    };
    Some(format!("{body} :: type {{{suffix}"))
}

fn material_v2_role_for_context(context: &str) -> &'static str {
    match context.trim().to_ascii_lowercase().as_str() {
        "" | "root" => "root",
        "runtime" => "runtime",
        "network" => "network",
        "system" => "system",
        "kernel" => "kernel",
        "tooling" => "tooling",
        "compiler" => "compiler",
        "security" => "security",
        "inspect" => "inspect",
        "ai" => "ai",
        "machine" => "machine",
        "primitive" => "primitive",
        "basal" => "basal",
        _ => "program",
    }
}

fn material_v2_replace_plain_assignment(line: &str) -> String {
    let trimmed = line.trim_start();
    if trimmed.starts_with("generate ")
        || trimmed.starts_with("type ")
        || trimmed.starts_with("associated type ")
    {
        return line.to_string();
    }
    let mut out = String::with_capacity(line.len());
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;
    while index < chars.len() {
        let ch = chars[index];
        if in_string || in_char {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if (in_string && ch == '"') || (in_char && ch == '\'') {
                in_string = false;
                in_char = false;
            }
            index += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            in_string = ch == '"';
            in_char = ch == '\'';
            out.push(ch);
            index += 1;
            continue;
        }
        if ch == '/' && chars.get(index + 1).is_some_and(|next| *next == '/') {
            out.extend(chars[index..].iter());
            break;
        }
        if ch == '=' {
            let previous = index.checked_sub(1).and_then(|i| chars.get(i)).copied();
            let next = chars.get(index + 1).copied();
            if !matches!(
                previous,
                Some('=' | '!' | '<' | '>' | '+' | '-' | '*' | '/' | '.' | ':')
            ) && !matches!(next, Some('=' | '>'))
            {
                out.push_str("<-");
                index += 1;
                continue;
            }
        }
        out.push(ch);
        index += 1;
    }
    out
}

fn material_v2_replace_projection_colon_init(line: &str) -> String {
    let Some(shift) = line.find(">>") else {
        return line.to_string();
    };
    let Some(relative_colon) = line[shift + 2..].find(':') else {
        return line.to_string();
    };
    let colon = shift + 2 + relative_colon;
    if line[shift + 2..colon].contains("<-")
        || line[shift + 2..colon].contains('{')
        || line[shift + 2..colon].contains(';')
    {
        return line.to_string();
    }
    if matches!(line.as_bytes().get(colon + 1), Some(&b':') | Some(&b'='))
        || line.as_bytes().get(colon.wrapping_sub(1)) == Some(&b':')
        || !line[colon + 1..].contains(';')
    {
        return line.to_string();
    }
    let mut rendered = String::with_capacity(line.len() + 1);
    rendered.push_str(&line[..colon]);
    rendered.push_str("<-");
    rendered.push_str(&line[colon + 1..]);
    rendered
}

fn material_v2_replace_colon_place_decl(line: &str) -> String {
    let comment_start = material_v2_comment_start(line).unwrap_or(line.len());
    let (code, comment) = line.split_at(comment_start);
    let segments = material_v2_top_level_semicolon_segments(code);
    if segments.len() <= 1 {
        return material_v2_replace_single_colon_place_decl(line);
    }
    let mut rendered = String::with_capacity(line.len() + segments.len() * 3);
    for segment in segments {
        rendered.push_str(&material_v2_replace_single_colon_place_decl(segment));
    }
    rendered.push_str(comment);
    rendered
}

fn material_v2_top_level_semicolon_segments(line: &str) -> Vec<&str> {
    let bytes = line.as_bytes();
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut round = 0usize;
    let mut square = 0usize;
    let mut curly = 0usize;
    let mut quote = None::<u8>;
    let mut escaped = false;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == active_quote {
                quote = None;
            }
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = Some(byte),
            b'(' => round += 1,
            b')' => round = round.saturating_sub(1),
            b'[' => square += 1,
            b']' => square = square.saturating_sub(1),
            b'{' => curly += 1,
            b'}' => curly = curly.saturating_sub(1),
            b';' if round == 0 && square == 0 && curly == 0 => {
                segments.push(&line[start..=index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < line.len() {
        segments.push(&line[start..]);
    }
    segments
}

fn material_v2_replace_single_colon_place_decl(line: &str) -> String {
    let trimmed = line.trim_start();
    if ["const ", "type ", "metadata "]
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
    {
        return line.to_string();
    }
    let code = line.split_once("//").map_or(line, |(code, _)| code);
    if !code.trim_end().ends_with(';') {
        return line.to_string();
    }
    let Some(colon) = line.find(':') else {
        return line.to_string();
    };
    if matches!(line.as_bytes().get(colon + 1), Some(&b':') | Some(&b'=')) {
        return line.to_string();
    }
    let name = line[..colon].trim();
    if !is_simple_ident(name) {
        return line.to_string();
    }
    if matches!(
        name,
        "abi"
            | "header"
            | "library"
            | "linkage"
            | "layout"
            | "ownership"
            | "nullability"
            | "sideEffects"
            | "signature"
            | "path"
            | "cppAbi"
            | "repr"
            | "exportedAs"
            | "exceptionBoundary"
    ) {
        return line.to_string();
    }
    let type_head = line[colon + 1..]
        .trim_start()
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .next()
        .unwrap_or("");
    if type_head.is_empty()
        || matches!(
            type_head,
            "for" | "while" | "loop" | "do" | "switch" | "match" | "case" | "default"
        )
        || (type_head
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_lowercase())
            && !matches!(
                type_head,
                "int"
                    | "uint"
                    | "byte"
                    | "char"
                    | "str"
                    | "string"
                    | "bool"
                    | "skk"
                    | "addr"
                    | "usize"
                    | "isize"
                    | "ft32"
                    | "ft64"
                    | "qword"
                    | "word"
                    | "dword"
                    | "p"
                    | "ptr"
                    | "array"
                    | "list"
                    | "queue"
                    | "vector"
                    | "memRegion"
                    | "fn"
            ))
    {
        return line.to_string();
    }
    let indent_len = line.len().saturating_sub(line.trim_start().len());
    let mut rendered = String::with_capacity(line.len() + 3);
    rendered.push_str(&line[..indent_len]);
    rendered.push_str(name);
    rendered.push_str(" >> ");
    rendered.push_str(line[colon + 1..].trim_start());
    rendered
}

fn material_v2_rewrite_fn_compat_calls(line: &str) -> String {
    let mut rendered = line.to_string();
    let mut search_from = 0usize;
    while let Some(relative_start) = rendered[search_from..].find("@fn(") {
        let start = search_from + relative_start;
        if material_v2_comment_start(&rendered).is_some_and(|comment| start >= comment) {
            break;
        }
        let bytes = rendered.as_bytes();
        let mut depth = 1usize;
        let mut index = start + 4;
        let mut quote = None::<u8>;
        let mut escaped = false;
        let mut comma = None::<usize>;
        let mut close = None::<usize>;
        while index < bytes.len() {
            let byte = bytes[index];
            if let Some(active_quote) = quote {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == active_quote {
                    quote = None;
                }
                index += 1;
                continue;
            }
            match byte {
                b'\'' | b'"' => quote = Some(byte),
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        close = Some(index);
                        break;
                    }
                }
                b',' if depth == 1 && comma.is_none() => comma = Some(index),
                _ => {}
            }
            index += 1;
        }
        let (Some(comma), Some(close)) = (comma, close) else {
            search_from = start + 4;
            continue;
        };
        let target = rendered[start + 4..comma].trim();
        if !target.split('.').all(|part| is_simple_ident(part.trim())) {
            search_from = close + 1;
            continue;
        }
        let arguments = rendered[comma + 1..close].trim();
        let replacement = format!("{target}({arguments})");
        rendered.replace_range(start..=close, &replacement);
        search_from = start + replacement.len();
    }
    rendered
}

fn material_v2_rewrite_probe_call(line: &str) -> String {
    let trimmed = line.trim_start();
    let indent = &line[..line.len().saturating_sub(trimmed.len())];
    let body = trimmed
        .strip_prefix("@probe(")
        .or_else(|| trimmed.strip_prefix("probe("));
    let Some(body) = body else {
        return line.to_string();
    };
    let Some(inner) = body.strip_suffix(");") else {
        return line.to_string();
    };
    format!("{indent}{};", inner.trim())
}

fn material_v2_rewrite_mem_projection(line: &str) -> String {
    let trimmed = line.trim_start();
    let indent = &line[..line.len().saturating_sub(trimmed.len())];
    let Some(after_mem) = trimmed.strip_prefix("mem[") else {
        return line.to_string();
    };
    let Some(close) = after_mem.find(']') else {
        return line.to_string();
    };
    let address = &after_mem[..close];
    let after_close = after_mem[close + 1..].trim_start();
    let Some(after_shift) = after_close.strip_prefix(">>") else {
        return line.to_string();
    };
    let Some((ty, value)) = after_shift.split_once("<-") else {
        return line.to_string();
    };
    format!(
        "{indent}store {} [{address}] <- {}",
        ty.trim(),
        value.trim_start()
    )
}

fn material_v2_strip_context_sigil(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;
    while index < chars.len() {
        let ch = chars[index];
        if in_string || in_char {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if (in_string && ch == '"') || (in_char && ch == '\'') {
                in_string = false;
                in_char = false;
            }
            index += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            in_string = ch == '"';
            in_char = ch == '\'';
            out.push(ch);
            index += 1;
            continue;
        }
        if ch == '/' && chars.get(index + 1).is_some_and(|next| *next == '/') {
            out.extend(chars[index..].iter());
            break;
        }
        if ch == '@'
            && chars
                .get(index + 1)
                .is_some_and(|next| is_ident_start(*next))
        {
            if chars.get(index + 1) == Some(&'f')
                && chars.get(index + 2) == Some(&'n')
                && chars.get(index + 3) == Some(&'(')
            {
                out.push(ch);
                index += 1;
                continue;
            }
            index += 1;
            continue;
        }
        out.push(ch);
        index += 1;
    }
    out
}

fn material_v2_rewrite_context_suffix(line: &str) -> String {
    let mut rendered = line.to_string();
    for context in [
        "runtime",
        "security",
        "system",
        "kernel",
        "machine",
        "primitive",
        "compiler",
        "tooling",
        "network",
        "inspect",
        "ai",
    ] {
        let marker = format!(" @{context}");
        while let Some(marker_start) = rendered.find(&marker) {
            let mut name_start = marker_start;
            while name_start > 0 && rendered.as_bytes()[name_start - 1].is_ascii_whitespace() {
                name_start -= 1;
            }
            let name_end = name_start;
            while name_start > 0 {
                let byte = rendered.as_bytes()[name_start - 1];
                if byte.is_ascii_alphanumeric() || byte == b'_' {
                    name_start -= 1;
                } else {
                    break;
                }
            }
            if name_start == name_end {
                break;
            }
            let name = rendered[name_start..name_end].to_string();
            rendered.replace_range(
                name_start..marker_start + marker.len(),
                &format!("{context}.{name}"),
            );
        }
    }
    rendered
}

fn compact_n_source(src: &str) -> Result<String, Box<dyn std::error::Error>> {
    let imports = compact_imports(src);
    let visible = collect_compact_visible_symbols(&imports);
    let storage = collect_compact_storage_symbols(&imports);
    // Section groups are the canonical visual structure. The formatter may
    // compact declarations inside them, but it must never erase `type:`,
    // `local:`, `fn:` or their visibility groups.
    let lifted = imports;
    let mut out = String::with_capacity(lifted.len());
    let mut section = NfmtSection::Other;
    let mut skip_compact_until_depth: Option<usize> = None;
    let mut in_block_comment = false;
    let mut brace_depth = 0usize;

    for raw_line in lifted.split_inclusive('\n') {
        let has_nl = raw_line.ends_with('\n');
        let line = raw_line.trim_end_matches('\n').trim_end_matches('\r');
        let trimmed = line.trim_start();
        let indent = line.len().saturating_sub(trimmed.len());
        let was_in_block_comment = in_block_comment;
        let starts_comment_block = trimmed.starts_with("/*");

        if compact_container_starts_low_level(trimmed) {
            skip_compact_until_depth = Some(brace_depth);
        }

        if skip_compact_until_depth.is_none() && !was_in_block_comment && !starts_comment_block {
            section = update_nfmt_section(section, indent, trimmed);
        }

        let rendered =
            if skip_compact_until_depth.is_some() || was_in_block_comment || starts_comment_block {
                line.to_string()
            } else {
                compact_n_line(line, section, &visible, &storage)
            };
        let rendered = if was_in_block_comment || starts_comment_block {
            rendered
        } else {
            compact_surface_aliases(&rendered)
        };

        out.push_str(&rendered);
        if has_nl {
            out.push('\n');
        }

        if !was_in_block_comment && !starts_comment_block {
            brace_depth = update_brace_depth(line, brace_depth);
        }
        in_block_comment = update_block_comment_state(line, in_block_comment);
        if let Some(base) = skip_compact_until_depth {
            if brace_depth <= base {
                skip_compact_until_depth = None;
            }
        }
    }

    Ok(align_compact_section_runs(&out))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NfmtSection {
    Stc,
    Type,
    Local,
    Const,
    Fn,
    Other,
}

fn update_nfmt_section(current: NfmtSection, indent: usize, trimmed: &str) -> NfmtSection {
    match trimmed {
        "stc:" | "state {" => NfmtSection::Stc,
        "type:" => NfmtSection::Type,
        "local:" => NfmtSection::Local,
        "const:" => NfmtSection::Const,
        "fn:" => NfmtSection::Fn,
        _ if trimmed.contains(":: type")
            || trimmed.contains(":: struct")
            || trimmed.contains(":: union")
            || trimmed.contains(":: enum") =>
        {
            NfmtSection::Type
        }
        _ if looks_like_function_header(trimmed) => NfmtSection::Fn,
        _ if indent <= 4 && trimmed.ends_with(':') => NfmtSection::Other,
        _ => current,
    }
}

fn compact_container_starts_low_level(trimmed: &str) -> bool {
    trimmed.starts_with("primitive[@]")
        || trimmed.starts_with("primitive [@]")
        || trimmed.contains("[@machine]")
}

fn compact_n_line(
    line: &str,
    section: NfmtSection,
    visible: &HashSet<String>,
    storage: &HashSet<String>,
) -> String {
    if line.trim().is_empty() {
        return line.to_string();
    }
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];

    if let Some(target) = compact_main_entry(trimmed) {
        return format!("{indent}ref main ->> {target};");
    }

    if matches!(section, NfmtSection::Fn) {
        if let Some(header) = compact_function_header(trimmed) {
            return format!("{indent}{header}");
        }
    }

    if matches!(section, NfmtSection::Stc | NfmtSection::Local) {
        if let Some(compact) = compact_uninitialized_projection(trimmed) {
            return format!("{indent}{compact}");
        }
        if let Some(compact) = compact_projection_declaration(trimmed, visible, storage) {
            return format!("{indent}{compact}");
        }
        if let Some(compact) = compact_colon_declaration(trimmed, visible) {
            return format!("{indent}{compact}");
        }
    }

    if matches!(section, NfmtSection::Const) {
        if let Some(compact) = compact_const_declaration(trimmed, visible) {
            return format!("{indent}{compact}");
        }
    }

    if matches!(section, NfmtSection::Type) {
        if let Some(compact) = compact_type_field(trimmed) {
            return format!("{indent}{compact}");
        }
    }

    if matches!(section, NfmtSection::Fn) {
        if let Some(compact) = compact_uninitialized_projection(trimmed) {
            return format!("{indent}{compact}");
        }
        if let Some(compact) = compact_projection_declaration(trimmed, visible, storage) {
            return format!("{indent}{compact}");
        }
    }

    if matches!(section, NfmtSection::Other) {
        if let Some(compact) = compact_projection_assignment(trimmed, visible) {
            return format!("{indent}{compact}");
        }
    }

    format!("{indent}{}", compact_context_refs(trimmed, visible))
}

fn compact_main_entry(trimmed: &str) -> Option<String> {
    if let Some(rest) = trimmed.strip_prefix("main:") {
        let target = rest.trim().strip_suffix(';')?.trim();
        return is_simple_ident(target).then(|| target.to_string());
    }
    let rest = trimmed.strip_prefix("ref main")?.trim_start();
    let rest = rest
        .strip_prefix("->>")
        .or_else(|| rest.strip_prefix("->"))?
        .trim_start();
    let target = if let Some(call) = rest.strip_prefix("@fn(") {
        call.strip_suffix(");")?.trim()
    } else {
        rest.strip_suffix(';')?.trim()
    };
    if target.is_empty()
        || !target
            .split('.')
            .all(|segment| is_simple_ident(segment.trim_start_matches('@')))
    {
        return None;
    }
    Some(target.trim_start_matches('@').to_string())
}

fn compact_function_header(trimmed: &str) -> Option<String> {
    if trimmed.contains(":: fn") || !looks_like_function_header(trimmed) {
        return None;
    }
    let open = trimmed.find('(')?;
    let prefix = trimmed[..open].trim();
    let (visibility, name) = match prefix.split_once(' ') {
        Some((visibility, name))
            if matches!(visibility, "public" | "private" | "internal" | "strictly") =>
        {
            (Some(visibility), name.trim())
        }
        _ => (None, prefix),
    };
    if !is_simple_ident(name) {
        return None;
    }
    let suffix = compact_function_params_surface(&trimmed[open..]);
    Some(match visibility {
        Some(visibility) => format!("{visibility} {name} :: fn{suffix}"),
        None => format!("{name} :: fn{suffix}"),
    })
}

fn compact_function_params_surface(suffix: &str) -> String {
    let Some(close) = suffix.rfind(')') else {
        return suffix.to_string();
    };
    let params = &suffix[1..close];
    let mut rendered = Vec::new();
    for raw in split_compact_param_items(params) {
        let param = raw.trim();
        if let Some((name, ty)) = param.split_once(">>") {
            if is_simple_ident(name.trim()) {
                rendered.push(format!(
                    "{} >> {}",
                    name.trim(),
                    compact_type_surface(ty.trim())
                ));
                continue;
            }
        }
        if let Some((name, ty)) = param.split_once(':') {
            if is_simple_ident(name.trim()) {
                rendered.push(format!(
                    "{} >> {}",
                    name.trim(),
                    compact_type_surface(ty.trim())
                ));
                continue;
            }
        }
        if let Some((name, ty)) = param.split_once(char::is_whitespace) {
            if is_simple_ident(name.trim()) && !ty.trim().is_empty() {
                rendered.push(format!(
                    "{} >> {}",
                    name.trim(),
                    compact_type_surface(ty.trim())
                ));
                continue;
            }
        }
        if is_simple_ident(param) {
            rendered.push(format!("{param} >> int"));
            continue;
        }
        rendered.push(param.to_string());
    }
    format!("({}){}", rendered.join(", "), &suffix[close + 1..])
}

#[cfg(test)]
mod compact_source_surface_tests {
    use super::*;

    #[test]
    fn nfmt_removes_non_material_trailing_whitespace_without_mutating_multiline_strings() {
        let source =
            "value >> int <- 1;   \n\t  \ntext <- \"first   \nsecond\";   \n/* note */   \n";
        let expected = "value >> int <- 1;\n\ntext <- \"first   \nsecond\";\n/* note */\n";
        assert_eq!(normalize_nfmt_trailing_whitespace(source), expected);
    }

    #[test]
    fn nfmt_preserves_hybrid_sections_visibility_and_main_alias_without_losing_comments() {
        let source = r#"
compactFormatting {
    local:
        state: int;

    fn:
        public:
            run() -> bool {
                // material state projection
                state <- 7;
                return state == 7;
            }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#;
        let formatted = format_nfmt_source(source, true).expect("compact format must succeed");
        assert!(formatted.source.contains("compactFormatting {"));
        assert!(formatted.source.contains("    local:"));
        assert!(formatted.source.contains("    fn:"));
        assert!(formatted.source.contains("        public:"));
        assert!(formatted.source.contains("            run() -> bool {"));
        assert!(formatted.source.contains("                state <- 7;"));
        assert!(formatted.source.contains("    ref main ->> run;"));
        assert!(formatted.source.contains("    fa:"));
        assert!(formatted.source.contains("// material state projection"));
        assert!(!formatted.source.contains("@fn("));
    }

    #[test]
    fn nfmt_preserves_stc_and_restores_fa_as_the_official_short_sections() {
        let source = r#"
compactSections :: program {
    stc:
        value >> int <- 1;

    run :: in() -> bool {
        return value == 1;
    }

    ref main ->> run;

    shutdown :: in() -> bool {
        return true;
    }
}
"#;
        let formatted = format_nfmt_source(source, true).expect("compact sections must format");
        assert!(formatted
            .source
            .contains("    stc:\n        value >> int <- 1;"));
        assert!(formatted.source.contains("    fa:\n        return true;"));
        assert!(formatted.source.contains("compactSections {"));
        assert!(!formatted.source.contains(":: program"));
        assert!(!formatted.source.contains("state {"));
        assert!(!formatted.source.contains("shutdown :: in"));
        assert!(formatted
            .source
            .contains("    fn:\n        run() -> bool {"));
        assert!(!formatted.source.contains("run :: in"));
        let tokens = ncc_lexer::lex(&formatted.source).expect("short sections must lex");
        ncc_parser::parse_material_v2(tokens).expect("short sections must parse");
    }

    #[test]
    fn nfmt_rejects_high_level_state_blocks() {
        let source = "bad { state { value >> int <- 1; } }";
        let error = format_nfmt_source(source, true).expect_err("state belongs to primitive/basal");
        assert!(error.to_string().contains("E_STC_CANONICAL_001"));
    }

    #[test]
    fn nfmt_rejects_removed_project_facade() {
        let source = "use::project::<alpha, beta>; app { fa: return true; }";
        let error = format_nfmt_source(source, true).expect_err("use::project is removed");
        assert!(error.to_string().contains("E_USE_PROJECT_CANONICAL_001"));
    }

    #[test]
    fn nfmt_maps_legacy_heap_and_list_modules_to_compact_facades() {
        let compact = compact_imports("use nC.heap { core };\nuse nC.list { * };\n");
        assert!(compact.contains("use <memory:heap>;"), "{compact}");
        assert!(compact.contains("use <collections:list>;"), "{compact}");
        assert!(!compact.contains("use <nC."), "{compact}");
    }

    #[test]
    fn nfmt_makes_pointer_aliases_official_without_touching_text_or_comments() {
        let line = r#"value >> ptr<int> = ptrRead<int>(raw); ptrWrite<int>(value, 7); print.dbg(value); println(\"ptr<int> ptrRead print.err\"); // ptrWrite<int> print.ok"#;
        let compact = compact_surface_aliases(line);
        assert!(compact.contains("value >> p<int> = rd(raw);"), "{compact}");
        assert!(compact.contains("wr(value, 7);"), "{compact}");
        assert!(compact.contains("println(value);"), "{compact}");
        assert!(
            compact.contains(r#"println(\"ptr<int> ptrRead print.err\")"#),
            "{compact}"
        );
        assert!(compact.ends_with("// ptrWrite<int> print.ok"), "{compact}");
        assert_eq!(compact_surface_aliases("ptr mutation;"), "ptr mutation;");
    }

    #[test]
    fn nfmt_preserves_grouped_kernel_parameters_as_visible_symbols() {
        let source = r#"
kernelDebugCompact {
    debug(
        trap, regs, memory: addr;
        slots, bytes: int
    ) -> bool {
        copied = dbgRegs(trap, regs, slots);
        dumped = dbgDump(trap, memory, bytes);
        return copied == slots && dumped == bytes;
    }

    idle() -> bool { return true; }
    main: idle;
}
"#;
        let formatted = format_nfmt_source(source, true).expect("compact format must succeed");
        assert!(
            formatted.source.contains("trap, regs, memory: addr;"),
            "{}",
            formatted.source
        );
        for parameter in ["trap", "regs", "memory", "slots", "bytes"] {
            assert!(
                !formatted.source.contains(&format!("@{parameter}")),
                "{}",
                formatted.source
            );
        }
    }

    #[test]
    fn nfmt_removes_only_runtime_families_materialized_by_the_backend_prelude() {
        let source = r#"use nC.runtime.string;
use nC.runtime.terminal;
use nC.runtime.time;
use::stoud::io::<output, exists, listDir>;
use::stoud::time::<time, sleep, timer>;
use::stoud::terminal::<clear, moveCursor>;
use::stoud::memory::<ptr, heap>;
use::stoud::mobile::<telephony, tts_engine, permissions>;
use::stoud::network::<server, client, send_bytes, receive_bytes>;
use::stoud::sys::<permissions, get_current_time, cpu_load>;
use::stoud::io::<fswrite, fread_lines, delete_file, rename_file>;
"#;
        let compact = compact_imports(source);
        assert!(!compact.contains("runtime.string"), "{compact}");
        assert!(!compact.contains("runtime.terminal"), "{compact}");
        assert!(!compact.contains("runtime.time"), "{compact}");
        assert!(compact.contains("use <io:output>;"), "{compact}");
        assert!(compact.contains("use <time:timer>;"), "{compact}");
        assert!(compact.contains("use <memory:ptr, heap>;"), "{compact}");
        assert!(!compact.contains("mobile:"), "{compact}");
        assert!(!compact.contains("network:"), "{compact}");
        assert!(!compact.contains("sys:"), "{compact}");
        assert!(!compact.contains("fswrite"), "{compact}");
    }

    #[test]
    fn nfmt_roundtrips_official_canonical_raw_fixture() {
        let source = include_str!("../../../../examples/06_memory/manual_raw_api_abi_layout.n");
        let formatted = format_nfmt_source(source, true).expect("canonical format must succeed");

        for required in [
            "use <io:output, render>;",
            "type:",
            "Packet {",
            "fn:",
            "internal:",
            "public:",
            "fillHeap(heap >> p<int>) -> int",
            "source >> Packet <- Packet { tag: 42, flags: 3 };",
            "cp(text, adr(b\"manual \"), 7);",
            "wr(off(text, 9), 0);",
            "answer >> int <- call(twice, rd(off(cells, 1)));",
            "ref main ->> run;",
            "fa:",
        ] {
            assert!(
                formatted.source.contains(required),
                "missing `{required}` in:\n{}",
                formatted.source
            );
        }

        for obsolete in [
            "use nC.runtime",
            "@fn(",
            "ptr<",
            "ptrRead",
            "ptrWrite",
            ":=",
        ] {
            assert!(
                !formatted.source.contains(obsolete),
                "obsolete surface `{obsolete}` survived in:\n{}",
                formatted.source
            );
        }

        assert_eq!(formatted.source, source);
        assert!(!formatted.changed);
    }

    #[test]
    fn nfmt_distinguishes_initializers_from_qualified_calls_and_projection_writes() {
        let visible = HashSet::from(["sum".to_string(), "value".to_string()]);
        let storage = HashSet::from(["sum".to_string()]);
        assert_eq!(
            compact_projection_declaration(
                "wifi >> bool = network::isWiFiConnected();",
                &visible,
                &storage
            )
            .as_deref(),
            Some("wifi >> bool <- network::isWiFiConnected();")
        );
        assert_eq!(
            compact_projection_declaration("@sum >> int : sum + @value;", &visible, &storage)
                .as_deref(),
            Some("sum <- sum + value;")
        );
        assert_eq!(
            compact_projection_declaration("cells[i] >> int : value;", &visible, &storage)
                .as_deref(),
            Some("cells[i] <- value;")
        );
    }

    #[test]
    fn nfmt_migrates_removed_colon_equal_to_the_single_write_operator() {
        let source = r#"
legacyInit {
    stc:
        value >> int := 1;
    fn:
        public:
            run() -> bool {
                value := 2;
                return value == 2;
            }
    ref main ->> run;
    fa:
        return true;
}
"#;
        let formatted = format_nfmt_source(source, true).expect("removed initializer migration");
        assert!(formatted.source.contains("value >> int <- 1;"));
        assert!(formatted.source.contains("value <- 2;"));
        assert!(!formatted.source.contains(":="));
        let tokens = ncc_lexer::lex(&formatted.source).expect("migrated source must lex");
        ncc_parser::parse_material_v2(tokens).expect("migrated source must parse");
    }

    #[test]
    fn nfmt_migrates_visible_multi_container_functions() {
        let source = include_str!("../../../../examples/01_basics/alpha_beta_doc.n");
        let compacted = format_nfmt_source(source, true)
            .expect("migration must render")
            .source;
        let tokens = ncc_lexer::lex(&compacted).expect("migration must lex");
        ncc_parser::parse(tokens)
            .unwrap_or_else(|error| panic!("migration must parse: {error}\n{compacted}"));
        assert!(compacted.contains("fn:"));
        assert!(compacted.contains("ping() -> int"));
        assert!(!compacted.contains("ping :: in"));
    }

    #[test]
    fn nfmt_keeps_material_write_as_the_single_assignment_surface() {
        let visible = HashSet::from(["slot".to_string(), "value".to_string()]);
        assert_eq!(
            compact_n_line(
                "        slot <- @value;",
                NfmtSection::Fn,
                &visible,
                &HashSet::new()
            ),
            "        slot <- value;"
        );
    }

    #[test]
    fn nfmt_preserves_function_locals_and_loop_bindings() {
        let source = include_str!("../../../../examples/02_flow/advanced_loops_backend.n");
        let compacted = format_nfmt_source(source, true)
            .expect("loop migration must render")
            .source;
        let tokens = ncc_lexer::lex(&compacted).expect("loop migration must lex");
        ncc_parser::parse(tokens)
            .unwrap_or_else(|error| panic!("loop migration must parse: {error}\n{compacted}"));
        assert!(compacted.contains("sum <- sum + value;"));
        assert!(compacted.contains("sum <- sum + n;"));

        let json = include_str!("../../../../examples/runtime/json/json_native_complete_ok.n");
        let compacted = format_nfmt_source(json, true)
            .expect("JSON migration must render")
            .source;
        let tokens = ncc_lexer::lex(&compacted).expect("JSON migration must lex");
        ncc_parser::parse(tokens)
            .unwrap_or_else(|error| panic!("JSON migration must parse: {error}\n{compacted}"));
        assert!(compacted.contains("root >> int <- jsonParse("));
        assert!(compacted.contains("same >> bool <- jsonEquals("));
    }

    #[test]
    fn nfmt_normalizes_legacy_type_block_indentation() {
        let source = include_str!("../../../../examples/01_basics/particle_mru_decay_advanced.n");
        let compacted = format_nfmt_source(source, true)
            .expect("type migration must render")
            .source;
        let tokens = ncc_lexer::lex(&compacted).expect("type migration must lex");
        ncc_parser::parse(tokens)
            .unwrap_or_else(|error| panic!("type migration must parse: {error}\n{compacted}"));
        assert!(compacted.contains("ParticleRun :: type"));
        assert!(compacted.contains("fn:"));
        assert!(compacted.contains("loadAdvancedStorage() -> bool"));
    }

    #[test]
    fn nfmt_compacts_function_contracts_inside_types() {
        let source = r#"
contracts {
    Packet :: type {
        tag >> int;
        constructure:
            static :: fn(tag >> int) -> Packet;
    }
    fa:
        return true;
}
"#;
        let compacted = format_nfmt_source(source, true)
            .expect("type contract formatting must render")
            .source;
        assert!(
            compacted.contains("static(tag >> int) -> Packet;"),
            "{compacted}"
        );
        assert!(!compacted.contains("static >> : fn"), "{compacted}");
        let tokens = ncc_lexer::lex(&compacted).expect("contract must lex");
        ncc_parser::parse(tokens).expect("contract must parse");
    }

    #[test]
    fn nfmt_groups_async_direct_functions() {
        let source = r#"
flow {
    async relay :: in(value >> int) -> int {
        return value;
    }
    fa:
        return true;
}
"#;
        let compacted = format_nfmt_source(source, true)
            .expect("async migration must render")
            .source;
        assert!(
            compacted.contains("fn:\n        async relay(value >> int) -> int {"),
            "{compacted}"
        );
        assert!(!compacted.contains(":: in"), "{compacted}");
        let tokens = ncc_lexer::lex(&compacted).expect("async migration must lex");
        ncc_parser::parse(tokens).expect("async migration must parse");
    }
}

fn compact_type_field(trimmed: &str) -> Option<String> {
    if trimmed.contains("::") || trimmed.ends_with(':') || trimmed.ends_with('{') || trimmed == "}"
    {
        return None;
    }
    let (name, ty) = trimmed.split_once(':')?;
    let name = name.trim();
    if !is_simple_ident(name) {
        return None;
    }
    let ty = ty.trim().strip_suffix(';')?.trim();
    if ty.is_empty() || ty.contains('=') {
        return None;
    }
    Some(format!("{name} >> {};", compact_type_surface(ty)))
}

fn compact_const_declaration(trimmed: &str, visible: &HashSet<String>) -> Option<String> {
    let (name, rest) = trimmed.split_once(':')?;
    let name = name.trim();
    if !is_simple_ident(name) || !rest.contains('=') {
        return None;
    }
    let (ty, expr) = rest.split_once('=')?;
    let expr = expr.trim().strip_suffix(';')?.trim();
    Some(format!(
        "{name} >> {} <- {};",
        compact_type_surface(ty.trim()),
        compact_context_refs(expr, visible)
    ))
}

fn compact_projection_declaration(
    trimmed: &str,
    visible: &HashSet<String>,
    storage: &HashSet<String>,
) -> Option<String> {
    let (lhs, rest) = trimmed.split_once(">>")?;
    let raw_name = lhs.trim();
    let projected = raw_name.starts_with('@');
    let name = raw_name.trim_start_matches('@').trim();
    let (ty, expr) = split_compact_initializer(rest)?;
    if ty.contains(';') {
        return None;
    }
    let raw_expr = expr.trim();
    let (expr, terminator) = match raw_expr.strip_suffix(';') {
        Some(expr) => (expr.trim(), ";"),
        None if !raw_expr.is_empty() => (raw_expr, ""),
        None => return None,
    };
    if projected && is_simple_ident(name) && storage.contains(name) {
        return Some(format!(
            "{name} <- {}{terminator}",
            compact_context_refs(expr, visible)
        ));
    }
    if !is_simple_ident(name) {
        let target = compact_context_refs(name, visible);
        if target.is_empty() || target.chars().any(char::is_whitespace) {
            return None;
        }
        return Some(format!(
            "{target} <- {}{terminator}",
            compact_context_refs(expr, visible),
        ));
    }
    Some(format!(
        "{name} >> {} <- {}{terminator}",
        compact_type_surface(ty.trim()),
        compact_context_refs(expr, visible)
    ))
}

fn compact_uninitialized_projection(trimmed: &str) -> Option<String> {
    let (lhs, ty) = trimmed.split_once(">>")?;
    let name = lhs.trim().trim_start_matches('@').trim();
    let ty = ty.trim().strip_suffix(';')?.trim();
    if !is_simple_ident(name) || ty.is_empty() || ty.contains([':', '=']) {
        return None;
    }
    Some(format!("{name} >> {};", compact_type_surface(ty)))
}

fn compact_colon_declaration(trimmed: &str, visible: &HashSet<String>) -> Option<String> {
    let (name, rest) = trimmed.split_once(':')?;
    let name = name.trim();
    if !is_simple_ident(name) {
        return None;
    }
    if let Some((ty, expr)) = rest.split_once('=') {
        let expr = expr.trim().strip_suffix(';')?.trim();
        return Some(format!(
            "{name} >> {} <- {};",
            compact_type_surface(ty.trim()),
            compact_context_refs(expr, visible)
        ));
    }
    let ty = rest.trim().strip_suffix(';')?.trim();
    (!ty.is_empty()).then(|| format!("{name} >> {};", compact_type_surface(ty)))
}

fn compact_type_surface(ty: &str) -> String {
    let mut rendered = ty.trim().replace("ptr<", "p<");
    let mut pointer_depth = 0usize;
    while rendered.ends_with('*') {
        rendered.pop();
        rendered = rendered.trim_end().to_string();
        pointer_depth += 1;
    }
    for _ in 0..pointer_depth {
        rendered = format!("p<{rendered}>");
    }
    rendered
}

fn compact_projection_assignment(trimmed: &str, visible: &HashSet<String>) -> Option<String> {
    let (lhs, rest) = trimmed.split_once(">>")?;
    let name = lhs.trim().trim_start_matches('@').trim();
    if !visible.contains(name) || !is_simple_ident(name) {
        return None;
    }
    let (_ty, expr) = split_compact_initializer(rest)?;
    let expr = expr.trim().strip_suffix(';')?.trim();
    Some(format!(
        "{name} <- {};",
        compact_context_refs(expr, visible)
    ))
}

fn split_compact_initializer(rest: &str) -> Option<(&str, &str)> {
    for (index, ch) in rest.char_indices() {
        let tail = &rest[index..];
        if tail.starts_with("<-") {
            return Some((&rest[..index], &rest[index + 2..]));
        }
        if tail.starts_with(":=") {
            return Some((&rest[..index], &rest[index + 2..]));
        }
        if ch == ':' {
            let previous = rest[..index].chars().next_back();
            let next = tail[1..].chars().next();
            if previous != Some(':') && next != Some(':') {
                return Some((&rest[..index], &rest[index + 1..]));
            }
        }
        if ch == '=' {
            let previous = rest[..index].chars().next_back();
            let next = tail[1..].chars().next();
            if !matches!(previous, Some('=' | '!' | '<' | '>')) && next != Some('=') {
                return Some((&rest[..index], &rest[index + 1..]));
            }
        }
    }
    None
}

fn compact_context_refs(input: &str, visible: &HashSet<String>) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;

    while let Some((idx, ch)) = chars.next() {
        if in_string || in_char {
            out.push(ch);
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if in_string && ch == '"' {
                in_string = false;
            } else if in_char && ch == '\'' {
                in_char = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            continue;
        }
        if ch == '\'' {
            in_char = true;
            out.push(ch);
            continue;
        }
        if ch == '/' && input[idx..].starts_with("//") {
            out.push_str(&input[idx..]);
            break;
        }
        if ch == '/' && input[idx..].starts_with("/*") {
            out.push_str(&input[idx..]);
            break;
        }
        if ch != '@' {
            out.push(ch);
            continue;
        }

        let Some((_, next)) = chars.peek().copied() else {
            out.push(ch);
            continue;
        };
        if !is_ident_start(next) {
            out.push(ch);
            continue;
        }

        let mut ident = String::new();
        while let Some((_, ident_ch)) = chars.peek().copied() {
            if is_ident_continue(ident_ch) {
                ident.push(ident_ch);
                chars.next();
            } else {
                break;
            }
        }

        if visible.contains(&ident) {
            out.push_str(&ident);
        } else {
            out.push('@');
            out.push_str(&ident);
        }
    }

    out
}

fn compact_surface_aliases(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;

    while let Some((idx, ch)) = chars.next() {
        if in_string || in_char {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if in_string && ch == '"' {
                in_string = false;
            } else if in_char && ch == '\'' {
                in_char = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            out.push(ch);
            continue;
        }
        if ch == '\'' {
            in_char = true;
            out.push(ch);
            continue;
        }
        if ch == '/' && (input[idx..].starts_with("//") || input[idx..].starts_with("/*")) {
            out.push_str(&input[idx..]);
            break;
        }
        if !is_ident_start(ch) {
            out.push(ch);
            continue;
        }

        let mut ident = String::from(ch);
        while let Some((_, next)) = chars.peek().copied() {
            if !is_ident_continue(next) {
                break;
            }
            ident.push(next);
            chars.next();
        }
        let rest_start = chars
            .peek()
            .map(|(offset, _)| *offset)
            .unwrap_or(input.len());
        let rest = &input[rest_start..];
        let next = rest.trim_start();
        if ident == "print" && rest.starts_with('.') {
            let member_len = rest[1..]
                .chars()
                .take_while(|candidate| is_ident_continue(*candidate))
                .map(char::len_utf8)
                .sum::<usize>();
            let suffix = &rest[1 + member_len..];
            if member_len > 0 && suffix.trim_start().starts_with('(') {
                for _ in 0..(1 + member_len) {
                    chars.next();
                }
                out.push_str("println");
                continue;
            }
        }
        let pointer_type_follows = next.starts_with('<')
            || next.strip_prefix("mut").is_some_and(|tail| {
                tail.starts_with('<')
                    || (tail.starts_with(char::is_whitespace) && tail.trim_start().starts_with('<'))
            });
        let c_style_star = rest.starts_with('*');
        let replacement = match ident.as_str() {
            "ptr" if pointer_type_follows => Some("p"),
            "ptrRead" | "pRead" | "Read"
                if next.starts_with('(') || next.starts_with('<') || c_style_star =>
            {
                Some("rd")
            }
            "ptrWrite" | "pWrite" | "Write"
                if next.starts_with('(') || next.starts_with('<') || c_style_star =>
            {
                Some("wr")
            }
            "ptrOffset" | "pOffset" | "Offset"
                if next.starts_with('(') || next.starts_with('<') || c_style_star =>
            {
                Some("off")
            }
            "ptrDiff" if next.starts_with('(') || next.starts_with('<') => Some("dist"),
            "ptrIsNull" if next.starts_with('(') || next.starts_with('<') => Some("nil"),
            "ptrFromAddr" | "pFromAddr" | "fromAddr"
                if next.starts_with('(') || next.starts_with('<') =>
            {
                Some("from")
            }
            "ptrCast" if next.starts_with('(') || next.starts_with('<') => Some("cast"),
            "ptrAddr" | "pAddr" if next.starts_with('(') || next.starts_with('<') => Some("adr"),
            "canonicalPath" if next.starts_with('(') => Some("officialPath"),
            "regionFromPtr" | "Region"
                if next.starts_with('(') || next.starts_with('<') || c_style_star =>
            {
                Some("pRegion")
            }
            "regionPtrAt" if next.starts_with('(') || next.starts_with('<') => Some("pAt"),
            _ => None,
        };
        if replacement.is_some() && c_style_star {
            chars.next();
        }
        out.push_str(replacement.unwrap_or(&ident));
    }

    elide_compact_call_type_args(&out)
}

fn elide_compact_call_type_args(input: &str) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(input.len());
    let mut index = 0usize;
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;

    while index < chars.len() {
        let ch = chars[index];
        if in_string || in_char {
            out.push(ch);
            index += 1;
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if (in_string && ch == '"') || (in_char && ch == '\'') {
                in_string = false;
                in_char = false;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            in_string = ch == '"';
            in_char = ch == '\'';
            out.push(ch);
            index += 1;
            continue;
        }
        if chars
            .get(index + 1)
            .is_some_and(|next| ch == '/' && matches!(next, '/' | '*'))
        {
            out.extend(chars[index..].iter());
            break;
        }
        if !is_ident_start(ch) {
            out.push(ch);
            index += 1;
            continue;
        }

        let start = index;
        index += 1;
        while index < chars.len() && is_ident_continue(chars[index]) {
            index += 1;
        }
        let name = chars[start..index].iter().collect::<String>();
        out.push_str(&name);
        let approved = matches!(
            name.as_str(),
            "rd" | "wr"
                | "off"
                | "dist"
                | "from"
                | "cast"
                | "pRegion"
                | "pAt"
                | "regionRead"
                | "regionWrite"
                | "listAt"
                | "queueAt"
                | "vectorAt"
                | "dinAt"
                | "sizeOf"
                | "alignOf"
        );
        if !approved || index >= chars.len() || chars[index] != '<' {
            continue;
        }

        let generic_start = index + 1;
        let mut cursor = generic_start;
        let mut depth = 1usize;
        while cursor < chars.len() && depth > 0 {
            match chars[cursor] {
                '<' => depth += 1,
                '>' => depth -= 1,
                _ => {}
            }
            cursor += 1;
        }
        if depth != 0 || cursor >= chars.len() || chars[cursor] != '(' {
            continue;
        }
        let generic_end = cursor - 1;
        if matches!(name.as_str(), "sizeOf" | "alignOf")
            && cursor + 1 < chars.len()
            && chars[cursor + 1] == ')'
        {
            out.push('(');
            out.push_str(
                chars[generic_start..generic_end]
                    .iter()
                    .collect::<String>()
                    .trim(),
            );
            out.push(')');
            index = cursor + 2;
        } else {
            index = cursor;
        }
    }

    out
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct CompactImportEntry {
    domain: String,
    family: String,
    items: Vec<String>,
}

fn compact_imports(src: &str) -> String {
    let mut out = Vec::<String>::new();
    let mut run = Vec::<String>::new();
    let mut depth = 0usize;
    let mut in_block_comment = false;

    for raw_line in src.split_inclusive('\n') {
        let has_nl = raw_line.ends_with('\n');
        let line = raw_line.trim_end_matches('\n').trim_end_matches('\r');
        let starts_comment_block = !in_block_comment && line.trim_start().starts_with("/*");
        let top_level_code = depth == 0 && !in_block_comment && !starts_comment_block;
        if top_level_code && implicit_legacy_import_line(line) {
            depth = update_brace_depth(line, depth);
            in_block_comment = update_block_comment_state(line, in_block_comment);
            continue;
        }
        if top_level_code && plain_import_line(line).is_some() {
            run.push(if has_nl {
                format!("{line}\n")
            } else {
                line.to_string()
            });
            depth = update_brace_depth(line, depth);
            in_block_comment = update_block_comment_state(line, in_block_comment);
            continue;
        }
        flush_compact_import_run(&mut out, &mut run);
        out.push(if has_nl {
            format!("{line}\n")
        } else {
            line.to_string()
        });
        if !in_block_comment && !starts_comment_block {
            depth = update_brace_depth(line, depth);
        }
        in_block_comment = update_block_comment_state(line, in_block_comment);
    }
    flush_compact_import_run(&mut out, &mut run);
    out.concat()
}

fn implicit_legacy_import_line(line: &str) -> bool {
    matches!(
        line.trim(),
        "use nC.runtime.string;"
            | "use nC.runtime.terminal;"
            | "use nC.runtime.time;"
            | "use::alloc::string::<string>;"
    )
}

fn flush_compact_import_run(out: &mut Vec<String>, run: &mut Vec<String>) {
    if run.is_empty() {
        return;
    }

    let mut known: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let mut raw_unknown = Vec::<String>::new();
    let trailing_newline = run.last().map(|line| line.ends_with('\n')).unwrap_or(false);

    for raw in run.drain(..) {
        let line = raw.trim_end_matches('\n').trim_end_matches('\r');
        match plain_import_line(line).and_then(parse_compact_import_line) {
            Some(entry) => {
                let domain = entry.domain;
                let family = entry.family;
                let items = known.entry((domain.clone(), family.clone())).or_default();
                for item in entry.items {
                    let item = if domain == "stoud"
                        && family == "io"
                        && item.eq_ignore_ascii_case("canonicalPath")
                    {
                        "officialPath".to_string()
                    } else {
                        item
                    };
                    if !compact_import_item_is_implicit(&domain, &family, &item) {
                        items.insert(item);
                    }
                }
            }
            None => raw_unknown.push(line.trim().to_string()),
        }
    }

    let mut rendered = Vec::<String>::new();
    for ((domain, family), items) in known {
        if items.is_empty() {
            continue;
        }
        let mut items = items.into_iter().collect::<Vec<_>>();
        items.sort_by(|a, b| {
            compact_import_item_rank(&domain, &family, a)
                .cmp(&compact_import_item_rank(&domain, &family, b))
                .then(a.cmp(b))
        });
        rendered.push(render_compact_import_entry(&CompactImportEntry {
            domain,
            family,
            items,
        }));
    }
    raw_unknown.sort();
    rendered.extend(raw_unknown);

    let rendered_len = rendered.len();
    for (idx, line) in rendered.into_iter().enumerate() {
        let needs_nl = trailing_newline || idx + 1 < rendered_len;
        if needs_nl {
            out.push(format!("{line}\n"));
        } else {
            out.push(line);
        }
    }
}

fn render_compact_import_entry(entry: &CompactImportEntry) -> String {
    let items = entry.items.join(", ");
    match entry.domain.as_str() {
        "stoud" => format!("use <{}:{items}>;", entry.family),
        "project" => format!("use <project:{items}>;"),
        "__direct__" => format!("use {} {{ {items} }};", entry.family),
        domain => format!("use <{domain}.{}:{items}>;", entry.family),
    }
}

fn compact_import_item_is_implicit(domain: &str, family: &str, item: &str) -> bool {
    matches!(
        (domain, family),
        ("stoud", "terminal")
            | ("stoud", "mobile")
            | ("stoud", "network")
            | ("stoud", "sys")
            | ("alloc", "string")
    ) || (domain == "stoud" && family == "time" && matches!(item, "time" | "sleep"))
        || (domain == "stoud"
            && family == "io"
            && matches!(
                item.to_ascii_lowercase().as_str(),
                "exists" | "listdir" | "fswrite" | "fread_lines" | "delete_file" | "rename_file"
            ))
        || (domain == "stoud"
            && family == "fs"
            && matches!(
                item.to_ascii_lowercase().as_str(),
                "exists" | "list" | "listdir"
            ))
}

fn plain_import_line(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.contains("//")
        || trimmed.contains("/*")
        || trimmed.contains("*/")
    {
        return None;
    }
    if trimmed.starts_with("use ") || trimmed.starts_with("use::") || trimmed.starts_with("import ")
    {
        Some(trimmed)
    } else {
        None
    }
}

fn parse_compact_import_line(line: &str) -> Option<CompactImportEntry> {
    if let Some(entry) = parse_angular_import_line(line) {
        return Some(entry);
    }
    if let Some(entry) = parse_material_block_import_line(line) {
        return Some(entry);
    }
    if let Some(entry) = parse_facade_import_line(line) {
        return Some(entry);
    }
    if let Some(entry) = parse_legacy_runtime_import_line(line) {
        return Some(entry);
    }
    parse_direct_dotted_import_line(line)
}

fn parse_material_block_import_line(line: &str) -> Option<CompactImportEntry> {
    let body = line.strip_prefix("use ")?.strip_suffix(';')?.trim();
    let (module, items_raw) = body.split_once('{')?;
    let items_raw = items_raw.strip_suffix('}')?.trim();
    let module = module.trim();
    let mut items = Vec::new();
    for item in items_raw.split(',').map(str::trim) {
        if item != "*" && !is_simple_ident(item) {
            return None;
        }
        items.push(item.to_string());
    }
    if items.is_empty() {
        return None;
    }
    let parts = module.split('.').collect::<Vec<_>>();
    let (domain, family) = match parts.as_slice() {
        ["nC", "heap"] => {
            items.clear();
            items.push("heap".to_string());
            ("stoud".to_string(), "memory".to_string())
        }
        ["nC", "list"] => {
            items.clear();
            items.push("list".to_string());
            ("stoud".to_string(), "collections".to_string())
        }
        ["project"] => ("project".to_string(), "module".to_string()),
        ["nC", _] => ("__direct__".to_string(), module.to_string()),
        [domain, family] if is_simple_ident(domain) && is_simple_ident(family) => {
            ((*domain).to_string(), (*family).to_string())
        }
        _ if parts.iter().all(|part| is_simple_ident(part)) => {
            ("__direct__".to_string(), module.to_string())
        }
        _ => return None,
    };
    Some(CompactImportEntry {
        domain,
        family,
        items,
    })
}

fn parse_angular_import_line(line: &str) -> Option<CompactImportEntry> {
    let body = line
        .strip_prefix("use <")?
        .strip_suffix(";")?
        .strip_suffix('>')?;
    if body.contains('|') {
        return None;
    }
    let (family, items_raw) = body.split_once(':')?;
    let family = family.trim();
    let items = items_raw
        .split(',')
        .map(str::trim)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if items.is_empty()
        || items
            .iter()
            .any(|item| item != "*" && !is_simple_ident(item))
    {
        return None;
    }
    let (domain, family) = if family == "project" {
        ("project".to_string(), "module".to_string())
    } else if let Some((domain, family)) = family.split_once('.') {
        if !is_simple_ident(domain) || !is_simple_ident(family) {
            return None;
        }
        (domain.to_string(), family.to_string())
    } else {
        if !is_simple_ident(family) {
            return None;
        }
        ("stoud".to_string(), family.to_string())
    };
    Some(CompactImportEntry {
        domain,
        family,
        items,
    })
}

fn parse_facade_import_line(line: &str) -> Option<CompactImportEntry> {
    let rest = line.strip_prefix("use::")?.strip_suffix(';')?;
    let (domain, rest) = rest.split_once("::")?;
    let (family, items_raw) = rest.split_once("::<")?;
    let items_raw = items_raw.strip_suffix('>')?;
    if !is_simple_ident(domain) || !is_simple_ident(family) {
        return None;
    }
    let mut items = Vec::<String>::new();
    for item in items_raw.split(',') {
        let item = item.trim();
        if !is_simple_ident(item) {
            return None;
        }
        items.push(item.to_string());
    }
    if items.is_empty() {
        return None;
    }
    Some(CompactImportEntry {
        domain: domain.to_string(),
        family: family.to_string(),
        items,
    })
}

fn parse_legacy_runtime_import_line(line: &str) -> Option<CompactImportEntry> {
    let family = line.strip_prefix("use nC.runtime.")?.strip_suffix(';')?;
    let (target_family, item) = match family {
        "input" => ("io", "input"),
        "output" => ("io", "output"),
        "render" => ("io", "render"),
        "random" => ("io", "random"),
        "fs" => ("io", "fs"),
        "heap" => ("memory", "heap"),
        "ptr" => ("memory", "ptr"),
        "addr" => ("memory", "ptr"),
        "region" => ("memory", "region"),
        "list" => ("collections", "list"),
        "queue" => ("collections", "queue"),
        "array" => ("collections", "array"),
        "result" => ("collections", "result"),
        "span" => ("collections", "span"),
        "slice" => ("collections", "slice"),
        _ => return None,
    };
    Some(CompactImportEntry {
        domain: "stoud".to_string(),
        family: target_family.to_string(),
        items: vec![item.to_string()],
    })
}

fn parse_direct_dotted_import_line(line: &str) -> Option<CompactImportEntry> {
    let body = line
        .strip_prefix("use ")
        .or_else(|| line.strip_prefix("import "))?
        .strip_suffix(';')?
        .trim();
    if body.contains(char::is_whitespace) || body.contains('{') || body.contains('<') {
        return None;
    }
    let (module, item) = if let Some(module) = body.strip_suffix(".*") {
        (module, "*")
    } else {
        body.rsplit_once('.')?
    };
    if module.is_empty()
        || module.split('.').any(|segment| !is_simple_ident(segment))
        || (item != "*" && !is_simple_ident(item))
    {
        return None;
    }
    Some(CompactImportEntry {
        domain: "__direct__".to_string(),
        family: module.to_string(),
        items: vec![item.to_string()],
    })
}

fn compact_import_item_rank(domain: &str, family: &str, item: &str) -> usize {
    let ordered: &[&str] = match (domain, family) {
        ("stoud", "io") => &[
            "output", "render", "input", "random", "fs", "fsread", "fswrite", "fsappend", "fsync",
        ],
        ("stoud", "memory") => &["ptr", "heap", "region", "buffer"],
        ("stoud", "collections") => &["array", "list", "queue", "result", "span", "slice"],
        _ => &[],
    };
    ordered
        .iter()
        .position(|candidate| candidate == &item)
        .unwrap_or(usize::MAX)
}

#[derive(Debug, Clone)]
struct ProjectionDeclParts {
    indent: String,
    name: String,
    ty: String,
    expr: String,
}

#[derive(Debug, Clone)]
struct TypeFieldParts {
    indent: String,
    name: String,
    ty: String,
}

#[derive(Debug, Clone)]
struct ConstDeclParts {
    indent: String,
    name: String,
    ty: String,
    expr: String,
}

fn align_compact_section_runs(src: &str) -> String {
    let mut lines = src.lines().map(str::to_string).collect::<Vec<_>>();
    let mut section = NfmtSection::Other;
    let mut idx = 0usize;
    while idx < lines.len() {
        let trimmed = lines[idx].trim_start();
        let indent = lines[idx].len() - trimmed.len();
        section = update_nfmt_section(section, indent, trimmed);

        if matches!(section, NfmtSection::Stc | NfmtSection::Local) {
            if let Some(parts) = parse_projection_decl_parts(&lines[idx]) {
                let start = idx;
                let mut run = vec![parts];
                idx += 1;
                while idx < lines.len() {
                    if let Some(next) = parse_projection_decl_parts(&lines[idx]) {
                        if next.indent == run[0].indent {
                            run.push(next);
                            idx += 1;
                            continue;
                        }
                    }
                    break;
                }
                render_projection_decl_run(&mut lines[start..start + run.len()], &run);
                continue;
            }
        }

        if matches!(section, NfmtSection::Const) {
            if let Some(parts) = parse_const_decl_parts(&lines[idx]) {
                let start = idx;
                let mut run = vec![parts];
                idx += 1;
                while idx < lines.len() {
                    if let Some(next) = parse_const_decl_parts(&lines[idx]) {
                        if next.indent == run[0].indent {
                            run.push(next);
                            idx += 1;
                            continue;
                        }
                    }
                    break;
                }
                render_const_decl_run(&mut lines[start..start + run.len()], &run);
                continue;
            }
        }

        if matches!(section, NfmtSection::Type) {
            if let Some(parts) = parse_type_field_parts(&lines[idx]) {
                let start = idx;
                let mut run = vec![parts];
                idx += 1;
                while idx < lines.len() {
                    if let Some(next) = parse_type_field_parts(&lines[idx]) {
                        if next.indent == run[0].indent {
                            run.push(next);
                            idx += 1;
                            continue;
                        }
                    }
                    break;
                }
                render_type_field_run(&mut lines[start..start + run.len()], &run);
                continue;
            }
        }

        idx += 1;
    }

    let mut rendered = lines.join("\n");
    if src.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

fn parse_projection_decl_parts(line: &str) -> Option<ProjectionDeclParts> {
    if line.contains("//") || line.contains("/*") {
        return None;
    }
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];
    let (name, rest) = trimmed.split_once(">>")?;
    let (ty, expr) = rest
        .split_once("<-")
        .or_else(|| rest.split_once(":="))
        .or_else(|| rest.split_once('='))?;
    let expr = expr.trim().strip_suffix(';')?.trim();
    let name = name.trim();
    if !is_simple_ident(name) || expr.is_empty() {
        return None;
    }
    Some(ProjectionDeclParts {
        indent: indent.to_string(),
        name: name.to_string(),
        ty: ty.trim().to_string(),
        expr: expr.to_string(),
    })
}

fn parse_type_field_parts(line: &str) -> Option<TypeFieldParts> {
    if line.contains("//") || line.contains("/*") || line.contains("::") {
        return None;
    }
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];
    let (name, ty) = trimmed.split_once(':')?;
    let ty = ty.trim().strip_suffix(';')?.trim();
    let name = name.trim();
    if !is_simple_ident(name) || ty.is_empty() || ty.contains('=') {
        return None;
    }
    Some(TypeFieldParts {
        indent: indent.to_string(),
        name: name.to_string(),
        ty: ty.to_string(),
    })
}

fn parse_const_decl_parts(line: &str) -> Option<ConstDeclParts> {
    if line.contains("//") || line.contains("/*") {
        return None;
    }
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];
    let (name, rest) = trimmed.split_once(':')?;
    let (ty, expr) = rest.split_once('=')?;
    let expr = expr.trim().strip_suffix(';')?.trim();
    let name = name.trim();
    if !is_simple_ident(name) || expr.is_empty() {
        return None;
    }
    Some(ConstDeclParts {
        indent: indent.to_string(),
        name: name.to_string(),
        ty: ty.trim().to_string(),
        expr: expr.to_string(),
    })
}

fn render_projection_decl_run(lines: &mut [String], run: &[ProjectionDeclParts]) {
    let name_width = run.iter().map(|row| row.name.len()).max().unwrap_or(0);
    let ty_width = run.iter().map(|row| row.ty.len()).max().unwrap_or(0);
    for (line, row) in lines.iter_mut().zip(run.iter()) {
        *line = format!(
            "{}{:<name_width$} >> {:<ty_width$} <- {};",
            row.indent, row.name, row.ty, row.expr
        );
    }
}

fn render_type_field_run(lines: &mut [String], run: &[TypeFieldParts]) {
    let name_width = run.iter().map(|row| row.name.len()).max().unwrap_or(0);
    for (line, row) in lines.iter_mut().zip(run.iter()) {
        *line = format!("{}{:<name_width$}: {};", row.indent, row.name, row.ty);
    }
}

fn render_const_decl_run(lines: &mut [String], run: &[ConstDeclParts]) {
    let name_width = run.iter().map(|row| row.name.len()).max().unwrap_or(0);
    let ty_width = run.iter().map(|row| row.ty.len()).max().unwrap_or(0);
    for (line, row) in lines.iter_mut().zip(run.iter()) {
        *line = format!(
            "{}{:<name_width$}: {:<ty_width$} = {};",
            row.indent, row.name, row.ty, row.expr
        );
    }
}

fn collect_compact_visible_symbols(src: &str) -> HashSet<String> {
    let mut visible = HashSet::new();
    let mut multiline_signature = None::<String>;
    for line in src.lines() {
        let trimmed = line.trim();
        collect_compact_line_symbol(trimmed, &mut visible);
        collect_compact_loop_symbols(trimmed, &mut visible);

        if let Some(signature) = multiline_signature.as_mut() {
            signature.push(' ');
            signature.push_str(trimmed);
            if trimmed.contains(')') {
                collect_compact_params(signature, &mut visible);
                multiline_signature = None;
            }
            continue;
        }

        if compact_function_signature_starts(trimmed) && !trimmed.contains(')') {
            multiline_signature = Some(trimmed.to_string());
        } else {
            collect_compact_params(trimmed, &mut visible);
        }
    }
    visible
}

fn collect_compact_storage_symbols(src: &str) -> HashSet<String> {
    let mut storage = HashSet::new();
    let mut section = NfmtSection::Other;
    for line in src.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        section = update_nfmt_section(section, indent, trimmed);
        if matches!(section, NfmtSection::Stc | NfmtSection::Local) {
            collect_compact_line_symbol(trimmed, &mut storage);
        }
    }
    storage
}

fn collect_compact_loop_symbols(trimmed: &str, visible: &mut HashSet<String>) {
    let for_tail = trimmed
        .strip_prefix("for ")
        .or_else(|| trimmed.split_once(" for ").map(|(_, tail)| tail));
    if let Some(tail) = for_tail {
        if !tail.starts_with('(') {
            if let Some(name) = tail.split_whitespace().next() {
                if is_simple_ident(name) {
                    visible.insert(name.to_string());
                }
            }
        }
    }

    for prefix in ["same in ", "each "] {
        if let Some(tail) = trimmed.strip_prefix(prefix) {
            let name = tail
                .split(|ch: char| ch == ':' || ch.is_whitespace())
                .next()
                .unwrap_or_default();
            if is_simple_ident(name) {
                visible.insert(name.to_string());
            }
        }
    }

    if let Some(tail) = trimmed.strip_prefix("while let ") {
        if let Some(open) = tail.find('(') {
            if let Some(close) = tail[open + 1..].find(')') {
                let binding = tail[open + 1..open + 1 + close]
                    .split(':')
                    .next()
                    .unwrap_or_default()
                    .trim();
                if is_simple_ident(binding) {
                    visible.insert(binding.to_string());
                }
            }
        }
    }
}

fn compact_function_signature_starts(trimmed: &str) -> bool {
    let Some(paren) = trimmed.find('(') else {
        return false;
    };
    is_simple_ident(trimmed[..paren].trim())
}

fn collect_compact_line_symbol(trimmed: &str, visible: &mut HashSet<String>) {
    if let Some((lhs, _)) = trimmed.split_once(">>") {
        let name = lhs.trim().trim_start_matches('@').trim();
        if is_simple_ident(name) {
            visible.insert(name.to_string());
        }
        return;
    }
    if let Some((name, rest)) = trimmed.split_once(':') {
        if rest.trim_end().ends_with(';') && is_simple_ident(name.trim()) {
            visible.insert(name.trim().to_string());
        }
    }
}

fn collect_compact_params(trimmed: &str, visible: &mut HashSet<String>) {
    let Some(paren) = trimmed.find('(') else {
        return;
    };
    let Some(close) = trimmed[paren + 1..].find(')') else {
        return;
    };
    if !trimmed.contains("->")
        && !trimmed[..paren]
            .chars()
            .all(|ch| ch.is_alphanumeric() || ch == '_')
    {
        return;
    }
    let params = &trimmed[paren + 1..paren + 1 + close];
    let mut pending = Vec::<&str>::new();
    for param in split_compact_param_items(params) {
        let param = param.trim();
        let Some((name, _ty)) = param.split_once(">>").or_else(|| param.split_once(':')) else {
            if is_simple_ident(param) {
                pending.push(param);
            }
            continue;
        };
        for grouped in pending.drain(..).chain(std::iter::once(name.trim())) {
            if is_simple_ident(grouped) {
                visible.insert(grouped.to_string());
            }
        }
    }
}

fn split_compact_param_items(params: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut start = 0usize;
    let mut angle = 0usize;
    let mut square = 0usize;
    let mut paren = 0usize;

    for (index, ch) in params.char_indices() {
        match ch {
            '<' => angle += 1,
            '>' => angle = angle.saturating_sub(1),
            '[' => square += 1,
            ']' => square = square.saturating_sub(1),
            '(' => paren += 1,
            ')' => paren = paren.saturating_sub(1),
            ',' | ';' if angle == 0 && square == 0 && paren == 0 => {
                items.push(&params[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    items.push(&params[start..]);
    items
}

fn looks_like_function_header(trimmed: &str) -> bool {
    if !trimmed.ends_with('{') || !trimmed.contains(')') || !trimmed.contains("->") {
        return false;
    }
    let Some(open) = trimmed.find('(') else {
        return false;
    };
    let mut head = trimmed[..open].trim();
    if head.contains('|') || head.contains('=') || head.contains(">>") {
        return false;
    }
    for prefix in ["public ", "private ", "internal ", "strictly ", "async "] {
        if let Some(rest) = head.strip_prefix(prefix) {
            head = rest.trim();
        }
    }
    if let Some(name) = head
        .strip_suffix(" :: in")
        .or_else(|| head.strip_suffix(" :: fn"))
    {
        head = name.trim();
    }
    is_simple_ident(head)
}

fn update_brace_depth(line: &str, depth: usize) -> usize {
    scan_code_braces(line, depth).0
}

fn scan_code_braces(line: &str, mut depth: usize) -> (usize, bool) {
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;
    let mut saw_open = false;
    for (idx, ch) in line.char_indices() {
        if in_string || in_char {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if in_string && ch == '"' {
                in_string = false;
            } else if in_char && ch == '\'' {
                in_char = false;
            }
            continue;
        }
        if ch == '/' && line[idx..].starts_with("//") {
            break;
        }
        if ch == '/' && line[idx..].starts_with("/*") {
            break;
        }
        match ch {
            '"' => in_string = true,
            '\'' => in_char = true,
            '{' => {
                saw_open = true;
                depth += 1;
            }
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    (depth, saw_open)
}

fn update_block_comment_state(line: &str, mut in_block: bool) -> bool {
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;
    let bytes = line.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() {
        let ch = bytes[idx] as char;
        if in_block {
            if idx + 1 < bytes.len() && bytes[idx] == b'*' && bytes[idx + 1] == b'/' {
                in_block = false;
                idx += 2;
                continue;
            }
            idx += 1;
            continue;
        }
        if in_string || in_char {
            if escaped {
                escaped = false;
                idx += 1;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                idx += 1;
                continue;
            }
            if in_string && ch == '"' {
                in_string = false;
            } else if in_char && ch == '\'' {
                in_char = false;
            }
            idx += 1;
            continue;
        }
        if idx + 1 < bytes.len() && bytes[idx] == b'/' && bytes[idx + 1] == b'/' {
            break;
        }
        if idx + 1 < bytes.len() && bytes[idx] == b'/' && bytes[idx + 1] == b'*' {
            in_block = true;
            idx += 2;
            continue;
        }
        if ch == '"' {
            in_string = true;
        } else if ch == '\'' {
            in_char = true;
        }
        idx += 1;
    }
    in_block
}

fn is_simple_ident(raw: &str) -> bool {
    let mut chars = raw.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    is_ident_start(first) && chars.all(is_ident_continue)
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

pub(crate) fn run_nlint_command(args: &[String]) -> Result<(), CliFailure> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!("{}", command_help("nlint").expect("nlint help must exist"));
        return Ok(());
    }
    if args.is_empty() {
        return Err(actionable_command_error("nlint", "nlint syntax: nlint <file.n>").into());
    }
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        println!("{}", command_help("nlint").expect("nlint help must exist"));
        return Ok(());
    }
    let path = args
        .first()
        .ok_or_else(|| actionable_command_error("nlint", "nlint syntax: nlint <file.n>"))?
        .to_string();
    let quiet = args.iter().skip(1).any(|arg| arg == "--quiet");
    if args.iter().skip(1).any(|arg| arg != "--quiet") {
        return Err(actionable_command_error("nlint", "nlint syntax: nlint <file.n>").into());
    }
    let src = fs::read_to_string(&path).map_err(|error| {
        CliFailure::filesystem("tooling.nlint", "read lint input", Path::new(&path), error)
    })?;
    let source_path = Path::new(&path);
    let ast = if let Some(ast) = content_cache::probe_ast(source_path, &src) {
        ast
    } else {
        let tokens = ncc_lexer::lex(&src).map_err(|diagnostic| {
            render_release_source_diagnostic(&diagnostic, "lexer", &path, &src)
        })?;
        ncc_parser::validate_material_v2_surface(&tokens).map_err(|diagnostic| {
            render_release_source_diagnostic(&diagnostic, "parser", &path, &src)
        })?;
        let ast = ncc_parser::parse_material_v2(tokens).map_err(|diagnostic| {
            render_release_source_diagnostic(&diagnostic, "parser", &path, &src)
        })?;
        content_cache::store_ast(source_path, &src, &ast);
        ast
    };
    let result = sema_analyze_with_content_cache(&ast)
        .map_err(|error| render_release_legacy_diagnostic(&error, "sema", &path, &src))?;
    for warning in result.warnings {
        eprintln!("warning: {warning}");
    }
    if !quiet {
        println!("OK: nlint {}", path);
    }
    Ok(())
}

pub(crate) fn nbuild_source_stem(source_path: &str) -> String {
    let stem = Path::new(source_path)
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("program");
    stem.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
}

#[derive(Debug, Clone)]
struct NbuildSourceExecutionArtifact {
    path: PathBuf,
    payload: serde_json::Value,
}

fn nbuild_write_source_execution_artifact(
    source_path: &str,
    artifact_root: &Path,
    program: &ncc_ast::Program,
    kernel_profile: bool,
) -> Result<NbuildSourceExecutionArtifact, Box<dyn std::error::Error>> {
    let contract = ncc_ast::resolve_source_execution_contract(program)
        .map_err(|error| actionable_command_error("nbuild", error))?;
    let contract = if kernel_profile {
        contract.constrained_for_kernel()
    } else {
        contract
    };
    let expansion = ncc_ast::source_expansion_report(program, &contract);
    let lowering_dir = artifact_root.join("lowering");
    fs::create_dir_all(&lowering_dir)?;
    super::compiler::compiler_native_remove_legacy_runtime_sources(&lowering_dir)?;
    let path = lowering_dir.join(format!(
        "{}.source-execution.json",
        nbuild_source_stem(source_path)
    ));
    let payload = serde_json::json!({
        "schema": "nc.compiler.source_execution_artifact.v2",
        "status": "materialized",
        "source": source_path,
        "contract": contract,
        "material_signature": contract.material_signature(),
        "expansion": expansion,
        "surface": {
            "official": "compact",
            "verbose": "compiler_internal_only"
        },
        "manual_equivalence": {
            "deterministic": true,
            "inspectable": true,
            "convertible_to_manual_ir": true
        },
        "manual_authority": {
            "compiler_role": "mechanical_translation_and_optimization_only",
            "runtime_effects_require_source_operation": true,
            "prelude_resolution_emits_no_call_without_source_call": true,
            "hidden_runtime_work": false
        }
    });
    fs::write(&path, serde_json::to_string_pretty(&payload)?.as_bytes())?;
    require_non_empty_output_file(
        "nbuild source execution artifact",
        path.to_string_lossy().as_ref(),
    )?;
    Ok(NbuildSourceExecutionArtifact { path, payload })
}

fn nbuild_attach_source_execution_payload(
    manifest: &mut serde_json::Value,
    artifact: &NbuildSourceExecutionArtifact,
) -> Result<(), Box<dyn std::error::Error>> {
    let object = manifest.as_object_mut().ok_or_else(|| {
        actionable_command_error("nbuild", "native build manifest must be a JSON object")
    })?;
    object.insert("source_execution".to_string(), artifact.payload.clone());
    if let Some(paths) = object
        .get_mut("paths")
        .and_then(serde_json::Value::as_object_mut)
    {
        paths.insert(
            "source_execution".to_string(),
            serde_json::Value::String(artifact.path.display().to_string()),
        );
    }
    if let Some(artifacts) = object
        .get_mut("artifacts")
        .and_then(serde_json::Value::as_array_mut)
    {
        let path = artifact.path.display().to_string();
        if !artifacts
            .iter()
            .any(|row| row.get("path").and_then(serde_json::Value::as_str) == Some(path.as_str()))
        {
            artifacts.push(nbuild_artifact_record(
                &artifact.path,
                "source_execution_contract",
            )?);
        }
    }
    Ok(())
}

fn default_nbuild_output_path(source_path: &str) -> String {
    let stem = nbuild_source_stem(source_path);
    let mut out = PathBuf::from("target").join("nc").join("bin").join(stem);
    if cfg!(windows) && out.extension().is_none() {
        out.set_extension("exe");
    }
    out.to_string_lossy().to_string()
}

fn default_nbuild_standard_output_path(
    source_path: &str,
    artifact_root: &Path,
    target_override: Option<&str>,
) -> String {
    let stem = nbuild_source_stem(source_path);
    let mut output = artifact_root.join("bin").join(stem);
    let target = target_override.unwrap_or_default().to_ascii_lowercase();
    if target.starts_with("wasm32") {
        output.set_extension("wasm");
    } else if target.contains("windows") || (target.is_empty() && cfg!(windows)) {
        output.set_extension("exe");
    }
    output.to_string_lossy().to_string()
}

fn nbuild_machine_ident(raw: &str) -> String {
    let mut out = raw
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();
    if out.is_empty() {
        out.push_str("build");
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

fn nbuild_machine_arch(spec: &ncc_ir::BackendTargetSpecIR) -> &'static str {
    match spec.isa {
        ncc_ir::BackendIsaIR::X86 => "i386",
        ncc_ir::BackendIsaIR::X86_64 => "x86_64",
        ncc_ir::BackendIsaIR::ArmV7 => "armv7",
        ncc_ir::BackendIsaIR::AArch64 => "aarch64",
        ncc_ir::BackendIsaIR::Wasm32 => "wasm32",
        ncc_ir::BackendIsaIR::RiscV32 => "riscv32",
        ncc_ir::BackendIsaIR::RiscV64 => "riscv64",
    }
}

fn nbuild_machine_abi(spec: &ncc_ir::BackendTargetSpecIR) -> &'static str {
    match spec.abi {
        ncc_ir::BackendAbiModelIR::I386SysV => "i386-sysv",
        ncc_ir::BackendAbiModelIR::I386Windows => "i386-msvc",
        ncc_ir::BackendAbiModelIR::SysV64 => "sysv64",
        ncc_ir::BackendAbiModelIR::Win64 => "win64",
        ncc_ir::BackendAbiModelIR::Aapcs32 => "aapcs32",
        ncc_ir::BackendAbiModelIR::Aapcs64 => "aapcs64",
        ncc_ir::BackendAbiModelIR::RiscV64 => "riscv64",
        ncc_ir::BackendAbiModelIR::Ilp32 => "ilp32",
        ncc_ir::BackendAbiModelIR::Wasi => "wasm",
        ncc_ir::BackendAbiModelIR::Freestanding => "freestanding",
        ncc_ir::BackendAbiModelIR::NcSovereign => "sovereign",
    }
}

fn nbuild_machine_object(spec: &ncc_ir::BackendTargetSpecIR) -> &'static str {
    match spec.object_format {
        ncc_ir::BackendObjectFormatIR::Elf32 => "elf32",
        ncc_ir::BackendObjectFormatIR::Elf64 => "elf64",
        ncc_ir::BackendObjectFormatIR::Coff => "coff",
        ncc_ir::BackendObjectFormatIR::MachO => "macho",
        ncc_ir::BackendObjectFormatIR::WasmModule => "wasm",
        ncc_ir::BackendObjectFormatIR::FreestandingImage => "freestanding",
    }
}

fn nbuild_apply_standard_target_override(
    profile: &mut ncc_ir::BackendLoweringProfileIR,
    target: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let destructive_policy = profile.destructive_policy.clone();
    match target {
        "x86_64-unknown-linux-gnu" => {
            *profile = ncc_ir::BackendLoweringProfileIR::nc_sysv64();
        }
        "x86_64-pc-windows-msvc" => {
            *profile = ncc_ir::BackendLoweringProfileIR::nc_win64();
        }
        "x86_64-apple-darwin" => {
            *profile = ncc_ir::BackendLoweringProfileIR::nc_sysv64();
            profile.target_spec.target_triple = "x86_64-apple-darwin".to_string();
            profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::MachO;
            profile.target_spec.debug_info_model = ncc_ir::BackendDebugInfoModelIR::Dwarf;
        }
        "aarch64-apple-darwin" => {
            *profile = ncc_ir::BackendLoweringProfileIR::nc_aapcs64();
        }
        "aarch64-unknown-linux-gnu" => {
            *profile = ncc_ir::BackendLoweringProfileIR::nc_aapcs64();
            profile.target_spec.target_triple = "aarch64-unknown-linux-gnu".to_string();
            profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::Elf64;
            profile.target_spec.debug_info_model = ncc_ir::BackendDebugInfoModelIR::Dwarf;
        }
        "i386-unknown-linux-gnu" => {
            *profile = ncc_ir::BackendLoweringProfileIR::nc_sysv64();
            profile.target = ncc_ir::BackendTargetIR::X86_64;
            profile.target_spec.target_triple = "i386-unknown-linux-gnu".to_string();
            profile.target_spec.target_family = ncc_ir::BackendTargetFamilyIR::Hosted;
            profile.target_spec.isa = ncc_ir::BackendIsaIR::X86;
            profile.target_spec.abi = ncc_ir::BackendAbiModelIR::I386SysV;
            profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::Elf32;
            profile.target_spec.relocation_model = ncc_ir::BackendRelocationModelIR::Static;
            profile.target_spec.code_model = ncc_ir::BackendCodeModelIR::Small;
            profile.target_spec.data_layout = ncc_ir::BackendDataLayoutIR::i386_sysv();
            profile.target_spec.pointer_width = 32;
            profile.target_spec.runtime_model = ncc_ir::BackendRuntimeModelIR::Hosted;
            profile.target_spec.host_services = ncc_ir::BackendHostServicesIR::Os;
            profile.target_spec.debug_info_model = ncc_ir::BackendDebugInfoModelIR::Dwarf;
            profile.target_spec.linker_model = ncc_ir::BackendLinkerModelIR::HostNative;
            profile.target_spec.support_tier = ncc_ir::BackendSupportTierIR::Tier3;
            profile.target_spec.backend_kind = ncc_ir::BackendKindIR::HostedProduction;
            profile.target_spec.profile_constraints =
                vec![ncc_ir::BackendProfileConstraintIR::RequiresHostedRuntime];
        }
        "riscv64-unknown-linux-gnu" => {
            *profile = ncc_ir::BackendLoweringProfileIR::nc_sysv64();
            profile.target = ncc_ir::BackendTargetIR::X86_64;
            profile.target_spec.target_triple = "riscv64-unknown-linux-gnu".to_string();
            profile.target_spec.target_family = ncc_ir::BackendTargetFamilyIR::Hosted;
            profile.target_spec.isa = ncc_ir::BackendIsaIR::RiscV64;
            profile.target_spec.abi = ncc_ir::BackendAbiModelIR::RiscV64;
            profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::Elf64;
            profile.target_spec.relocation_model = ncc_ir::BackendRelocationModelIR::Static;
            profile.target_spec.code_model = ncc_ir::BackendCodeModelIR::Small;
            profile.target_spec.data_layout = ncc_ir::BackendDataLayoutIR::lp64();
            profile.target_spec.pointer_width = 64;
            profile.target_spec.runtime_model = ncc_ir::BackendRuntimeModelIR::Hosted;
            profile.target_spec.host_services = ncc_ir::BackendHostServicesIR::Os;
            profile.target_spec.debug_info_model = ncc_ir::BackendDebugInfoModelIR::Dwarf;
            profile.target_spec.linker_model = ncc_ir::BackendLinkerModelIR::HostNative;
            profile.target_spec.support_tier = ncc_ir::BackendSupportTierIR::Tier3;
            profile.target_spec.backend_kind = ncc_ir::BackendKindIR::HostedProduction;
            profile.target_spec.profile_constraints =
                vec![ncc_ir::BackendProfileConstraintIR::RequiresHostedRuntime];
        }
        "riscv64-unknown-elf" => {
            *profile = ncc_ir::BackendLoweringProfileIR::nc_sysv64();
            profile.target = ncc_ir::BackendTargetIR::X86_64;
            profile.target_spec.target_triple = "riscv64-unknown-elf".to_string();
            profile.target_spec.target_family = ncc_ir::BackendTargetFamilyIR::Freestanding;
            profile.target_spec.isa = ncc_ir::BackendIsaIR::RiscV64;
            profile.target_spec.abi = ncc_ir::BackendAbiModelIR::RiscV64;
            profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::Elf64;
            profile.target_spec.relocation_model = ncc_ir::BackendRelocationModelIR::Static;
            profile.target_spec.code_model = ncc_ir::BackendCodeModelIR::Kernel;
            profile.target_spec.data_layout = ncc_ir::BackendDataLayoutIR::lp64();
            profile.target_spec.pointer_width = 64;
            profile.target_spec.runtime_model = ncc_ir::BackendRuntimeModelIR::Freestanding;
            profile.target_spec.host_services = ncc_ir::BackendHostServicesIR::BareMetal;
            profile.target_spec.debug_info_model = ncc_ir::BackendDebugInfoModelIR::None;
            profile.target_spec.linker_model = ncc_ir::BackendLinkerModelIR::FreestandingInternal;
            profile.target_spec.support_tier = ncc_ir::BackendSupportTierIR::Tier3;
            profile.target_spec.backend_kind = ncc_ir::BackendKindIR::Freestanding;
            profile.target_spec.profile_constraints =
                vec![ncc_ir::BackendProfileConstraintIR::RequiresFreestandingRuntime];
        }
        "riscv32imac-unknown-elf" | "riscv32-unknown-elf" => {
            *profile = ncc_ir::BackendLoweringProfileIR::nc_sysv64();
            profile.target = ncc_ir::BackendTargetIR::X86_64;
            profile.target_spec.target_triple = "riscv32-unknown-elf".to_string();
            profile.target_spec.target_family = ncc_ir::BackendTargetFamilyIR::Freestanding;
            profile.target_spec.isa = ncc_ir::BackendIsaIR::RiscV32;
            profile.target_spec.abi = ncc_ir::BackendAbiModelIR::Ilp32;
            profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::Elf32;
            profile.target_spec.relocation_model = ncc_ir::BackendRelocationModelIR::Static;
            profile.target_spec.code_model = ncc_ir::BackendCodeModelIR::Kernel;
            profile.target_spec.data_layout = ncc_ir::BackendDataLayoutIR::ilp32();
            profile.target_spec.pointer_width = 32;
            profile.target_spec.runtime_model = ncc_ir::BackendRuntimeModelIR::Freestanding;
            profile.target_spec.host_services = ncc_ir::BackendHostServicesIR::BareMetal;
            profile.target_spec.debug_info_model = ncc_ir::BackendDebugInfoModelIR::None;
            profile.target_spec.linker_model = ncc_ir::BackendLinkerModelIR::FreestandingInternal;
            profile.target_spec.support_tier = ncc_ir::BackendSupportTierIR::Tier3;
            profile.target_spec.backend_kind = ncc_ir::BackendKindIR::Freestanding;
            profile.target_spec.profile_constraints =
                vec![ncc_ir::BackendProfileConstraintIR::RequiresFreestandingRuntime];
        }
        "wasm32-wasi" | "wasm32-unknown-unknown" => {
            *profile = ncc_ir::BackendLoweringProfileIR::nc_portable();
            profile.target_spec.target_triple = target.to_string();
        }
        other => {
            return Err(actionable_command_error(
                "nbuild",
                format!(
                    "normal .n target '{}' is not admitted; expected x86_64-unknown-linux-gnu, x86_64-pc-windows-msvc, x86_64-apple-darwin, aarch64-apple-darwin, aarch64-unknown-linux-gnu, i386-unknown-linux-gnu, riscv64-unknown-linux-gnu, riscv64-unknown-elf, riscv32imac-unknown-elf or wasm32-wasi",
                    other
                ),
            )
            .into());
        }
    }
    profile.destructive_policy = destructive_policy;
    Ok(())
}

#[cfg(test)]
mod nbuild_destructive_profile_tests {
    use super::*;

    #[test]
    fn standard_target_override_preserves_destructive_backend_policy() {
        let mut profile = ncc_ir::BackendLoweringProfileIR::nc_destructive();
        nbuild_apply_standard_target_override(&mut profile, "x86_64-unknown-linux-gnu")
            .expect("compatible target override must apply");
        assert_eq!(
            profile.target_spec.target_triple,
            "x86_64-unknown-linux-gnu"
        );
        assert!(profile.destructive_policy.enabled);
        assert!(profile.destructive_policy.statement_ambiguity);
        assert!(profile.destructive_policy.diagnostics_silent);
    }
}

pub(crate) fn nbuild_internal_os_link_format(
    object: &ncc_ir::BackendObjectIR,
    linker: Option<&str>,
) -> Option<&'static str> {
    if linker.is_some() {
        return None;
    }
    match object.target_spec.object_format {
        ncc_ir::BackendObjectFormatIR::Coff
            if object.target_spec.isa == ncc_ir::BackendIsaIR::X86_64
                && object.target_spec.abi == ncc_ir::BackendAbiModelIR::Win64 =>
        {
            Some("coff")
        }
        ncc_ir::BackendObjectFormatIR::Elf64 => Some("elf64"),
        ncc_ir::BackendObjectFormatIR::Elf32 => Some("elf32"),
        ncc_ir::BackendObjectFormatIR::MachO
            if matches!(
                object.target_spec.isa,
                ncc_ir::BackendIsaIR::X86_64 | ncc_ir::BackendIsaIR::AArch64
            ) =>
        {
            Some("macho")
        }
        _ => None,
    }
}

fn nbuild_internal_os_link_route(format: &str) -> &'static str {
    match format {
        "coff" => "native-pe64-internal",
        "elf64" => "native-elf64-internal",
        "elf32" => "native-elf32-internal",
        "macho" => "native-macho-internal",
        _ => "native-internal",
    }
}

fn nbuild_internal_os_link_map_suffix(format: &str) -> &'static str {
    match format {
        "coff" => "pe64",
        "elf64" => "elf64",
        "elf32" => "elf32",
        "macho" => "macho",
        _ => "native",
    }
}

static NBUILD_RUNTIME_IMPORT_SYMBOLS: std::sync::OnceLock<std::collections::BTreeSet<String>> =
    std::sync::OnceLock::new();

fn nbuild_is_runtime_import_symbol(symbol: &str) -> bool {
    let normalized = nbuild_normalize_runtime_import_symbol(symbol);
    NBUILD_RUNTIME_IMPORT_SYMBOLS
        .get_or_init(nbuild_runtime_import_symbol_set)
        .contains(&normalized)
}

fn nbuild_runtime_import_symbol_set() -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::<String>::new();
    for symbol in [
        "alloc",
        "heapAlloc",
        "realloc",
        "free",
        "calloc",
        "malloc",
        "printf",
        "puts",
        "putchar",
        "scanf",
        "fprintf",
        "sprintf",
        "snprintf",
        "_snprintf",
        "fflush",
        "write",
        "read",
        "strlen",
        "strcmp",
        "strncmp",
        "strstr",
        "memcpy",
        "memmove",
        "memset",
        "getchar",
        "ShellExecuteA",
        "keybd_event",
        "Sleep",
        "usleep",
        "GetStdHandle",
        "ReadFile",
        "WriteFile",
        "CreateFileA",
        "CloseHandle",
        "ExitProcess",
        "isOk",
    ] {
        nbuild_insert_runtime_symbol_aliases(&mut out, symbol);
    }

    for family in ncc_ast::runtime_api_family_specs() {
        for function in &family.functions {
            nbuild_insert_runtime_symbol_aliases(&mut out, &function.name);
            nbuild_insert_runtime_symbol_aliases(
                &mut out,
                &format!("{}.{}", family.import_path, function.name),
            );
            nbuild_insert_runtime_symbol_aliases(
                &mut out,
                &format!("{}.{}", family.module_path, function.name),
            );
        }
    }

    let manifest = ncc_ast::use_backend_lowering_manifest_all();
    for entry in manifest.entries {
        if entry.closed_real_backing_bytes {
            nbuild_insert_runtime_symbol_aliases(&mut out, &entry.backend_symbol);
        }
        for symbol in entry.runtime_symbols {
            nbuild_insert_runtime_symbol_aliases(&mut out, &symbol);
        }
    }

    out
}

fn nbuild_insert_runtime_symbol_aliases(
    out: &mut std::collections::BTreeSet<String>,
    symbol: &str,
) {
    let normalized = nbuild_normalize_runtime_import_symbol(symbol);
    if normalized.is_empty() {
        return;
    }
    out.insert(normalized.clone());
    if let Some(leaf) = normalized.rsplit('.').next() {
        if !leaf.is_empty() {
            out.insert(leaf.to_string());
        }
    }
}

fn nbuild_normalize_runtime_import_symbol(symbol: &str) -> String {
    let mut normalized = symbol
        .trim()
        .trim_start_matches("__imp_")
        .trim_start_matches('_')
        .to_ascii_lowercase();
    if let Some(stripped) = normalized.strip_prefix("ncc_runtime_") {
        normalized = stripped.replace('_', ".");
    } else if let Some(stripped) = normalized.strip_prefix("nc_runtime_") {
        normalized = stripped.replace('_', ".");
    }
    normalized
}

fn nbuild_runtime_import_materialization_payload(
    object: &ncc_ir::BackendObjectIR,
    internal_os_link_format: Option<&str>,
) -> serde_json::Value {
    let unresolved = object.unresolved_symbols();
    let runtime_symbols = unresolved
        .iter()
        .filter(|symbol| nbuild_is_runtime_import_symbol(symbol))
        .cloned()
        .collect::<Vec<_>>();
    let materializers = match internal_os_link_format {
        Some("coff") => vec![
            "pe64.import_table",
            "pe64.iat",
            "pe64.runtime_thunks.printf/readInt/fswrite/isOk/fflush",
            "pe64.relocations",
        ],
        Some("elf64") => vec![
            "elf64.program_headers",
            "elf64.dynamic_metadata_when_needed",
            "elf64.plt_got_for_supported_arches",
            "elf64.final_relocations",
        ],
        Some("elf32") => vec![
            "elf32.program_headers",
            "elf32.final_relocations",
            "elf32.static_runtime_contract",
        ],
        Some("macho") => vec![
            "macho.lc_main",
            "macho.symtab_strtab",
            "macho.bind_lazy_bind_when_supported",
            "macho.ad_hoc_code_signature",
        ],
        _ => vec!["configured_external_linker"],
    };
    serde_json::json!({
        "schema": "nc.compiler.runtime_import_materialization.v1",
        "status": if runtime_symbols.is_empty() {
            "closed_no_runtime_imports_requested"
        } else if internal_os_link_format.is_some() {
            "closed_by_internal_machine_linker"
        } else {
            "delegated_to_configured_linker"
        },
        "runtime_symbols": runtime_symbols,
        "all_unresolved_symbols_at_program_object_stage": unresolved,
        "native_call_admission": nbuild_runtime_native_call_admission_payload(),
        "direct_machine_materialized": internal_os_link_format.is_some(),
        "internal_os_link_format": internal_os_link_format,
        "materializers": materializers,
    })
}

fn nbuild_runtime_native_call_admission_payload() -> serde_json::Value {
    let official_runtime_function_count = ncc_ast::runtime_api_family_specs()
        .iter()
        .map(|family| family.functions.len())
        .sum::<usize>();
    let manifest = ncc_ast::use_backend_lowering_manifest_all();
    let backed_import_entries = manifest
        .entries
        .iter()
        .filter(|entry| entry.closed_real_backing_bytes)
        .count();
    serde_json::json!({
        "schema": "nc.compiler.runtime_native_call_admission.v1",
        "status": "closed_catalog_derived",
        "source_catalogs": [
            "ncc_ast.runtime_api_family_specs",
            "ncc_ast.use_backend_lowering_manifest_all",
            "backend.host_c_os_bridge_symbols"
        ],
        "official_runtime_function_count": official_runtime_function_count,
        "backend_import_entries": manifest.total_entries,
        "backed_backend_import_entries": backed_import_entries,
        "admitted_symbol_alias_count": NBUILD_RUNTIME_IMPORT_SYMBOLS
            .get_or_init(nbuild_runtime_import_symbol_set)
            .len(),
        "catalog_only_activation_symbols_excluded": true,
        "manual_runtime_allowlist": false,
    })
}

pub(crate) fn nbuild_retained_machine_bridge_source(
    source_path: &str,
    stem: &str,
    object: &ncc_ir::BackendObjectIR,
    entry_symbol: &str,
    entry_returns_bool: bool,
    object_path: &Path,
    exe_path: &Path,
) -> String {
    let container = nbuild_machine_ident(stem);
    let bridge_symbol = nbuild_machine_ident(&format!("ncc_{stem}_standard_build_bridge"));
    let machine_entry_symbol = entry_symbol.replace('"', "'");
    let init_symbol = nbuild_entry_stc_init_symbol(object, &machine_entry_symbol);
    let arch = nbuild_machine_arch(&object.target_spec);
    let abi = nbuild_machine_abi(&object.target_spec);
    let object_format = nbuild_machine_object(&object.target_spec);
    let pointer_width = object.target_spec.pointer_width;
    let inferred_backend_material = object_path.parent().and_then(Path::parent).map(|root| {
        root.join("lowering")
            .join(format!("{stem}.backend-object.nbo"))
    });
    let identity = nbuild_machine_source_identity(
        source_path,
        object,
        inferred_backend_material
            .as_deref()
            .filter(|path| path.is_file()),
    );
    let identity_symbol = nbuild_machine_ident(&format!("__ncc_source_{stem}"));
    let identity_data = nbuild_machine_source_identity_data(&identity_symbol, &identity);
    let mut externs = format!("        {machine_entry_symbol}:");
    if let Some(init) = init_symbol.as_deref() {
        externs.push_str(&format!("\n        {init}:"));
    }
    if matches!(object.target_spec.abi, ncc_ir::BackendAbiModelIR::Win64) {
        externs.push_str("\n        fflush:\n        ExitProcess:");
    }
    let init_x86 = init_symbol
        .as_deref()
        .map(|symbol| format!("        call {symbol};\n"))
        .unwrap_or_default();
    let init_x86_sysv = init_symbol
        .as_deref()
        .map(|symbol| {
            format!(
                "        push rdi;\n        push rsi;\n        push rdx;\n        call {symbol};\n        pop rdx;\n        pop rsi;\n        pop rdi;\n"
            )
        })
        .unwrap_or_default();
    let init_aarch64 = init_symbol
        .as_deref()
        .map(|symbol| format!("        bl {symbol};\n"))
        .unwrap_or_default();
    let init_riscv = init_symbol
        .as_deref()
        .map(|symbol| format!("        call {symbol};\n"))
        .unwrap_or_default();
    let normalize_x86 = entry_returns_bool
        .then_some("        xor eax, 1;\n")
        .unwrap_or_default();
    let normalize_armv7 = entry_returns_bool
        .then_some("        mov r1, 1;\n        eor r0, r1;\n")
        .unwrap_or_default();
    let normalize_aarch64 = entry_returns_bool
        .then_some("        mov w1, 1;\n        sub w0, w1, w0;\n")
        .unwrap_or_default();
    let normalize_riscv = entry_returns_bool
        .then_some("        xori a0, a0, 1;\n")
        .unwrap_or_default();
    let instr = match object.target_spec.isa {
        ncc_ir::BackendIsaIR::X86 => {
            format!("{init_x86}        call {machine_entry_symbol};\n{normalize_x86}        ret;")
        }
        ncc_ir::BackendIsaIR::X86_64 => {
            if matches!(object.target_spec.abi, ncc_ir::BackendAbiModelIR::Win64) {
                format!(
                    "        sub rsp, 40;\n{init_x86}        call {machine_entry_symbol};\n{normalize_x86}        mov rbx, rax;\n        mov rcx, 0;\n        call fflush;\n        mov rcx, rbx;\n        call ExitProcess;\n        add rsp, 40;\n        ret;"
                )
            } else {
                format!(
                    "{init_x86_sysv}        sub rsp, 8;\n        call {machine_entry_symbol};\n{normalize_x86}        add rsp, 8;\n        ret;"
                )
            }
        }
        ncc_ir::BackendIsaIR::ArmV7 => {
            format!(
                "{init_aarch64}        bl {machine_entry_symbol};\n{normalize_armv7}        bx lr;"
            )
        }
        ncc_ir::BackendIsaIR::AArch64 => {
            format!(
                "{init_aarch64}        bl {machine_entry_symbol};\n{normalize_aarch64}        ret;"
            )
        }
        ncc_ir::BackendIsaIR::RiscV64 => {
            format!(
                "{init_riscv}        call {machine_entry_symbol};\n{normalize_riscv}        ret;"
            )
        }
        ncc_ir::BackendIsaIR::RiscV32 => {
            format!(
                "{init_riscv}        call {machine_entry_symbol};\n{normalize_riscv}        ret;"
            )
        }
        ncc_ir::BackendIsaIR::Wasm32 => "        i32.const 0;\n        return;".to_string(),
    };

    format!(
        "{container} :: machine {{\n    target:\n        arch: {arch};\n        abi: {abi};\n        object: {object_format};\n        endian: little;\n        pointerWidth: {pointer_width};\n        profile: userland;\n\n    debug:\n        file: \"{}\";\n        source: \"{}\";\n        entry: \"{}\";\n        macho_unwind: compact;\n        producer: \"ncc build standard .n -> root -> .nm -> .nbo -> .obj -> .exe\";\n\n    extern:\n{externs}\n\n    symbol:\n        {bridge_symbol}:\n            section: .text;\n            binding: global;\n            type: routine;\n\n    entry:\n        {bridge_symbol};\n\n    proc:\n        {bridge_symbol}:\n\n    instr:\n{instr}\n{identity_data}\n    verify:\n        target;\n        encoding;\n        object;\n\n    seal:\n        artifact: executable;\n        require: verified;\n\n    evidence:\n        sourceIdentitySha256: \"{}\";\n        sourceContentSha256: \"{}\";\n        backendObjectSha256: \"{}\";\n        typeLayoutUnits: \"{}\";\n        containerStorageSymbols: \"{}\";\n        object: \"{}\";\n        executable: \"{}\";\n}}\n",
        nbuild_machine_literal_path(source_path),
        nbuild_machine_literal_path(source_path),
        entry_symbol.replace('"', "'"),
        identity.source_identity_sha256,
        identity.source_content_sha256,
        identity.backend_object_sha256,
        identity.type_layout_units,
        identity.container_storage_symbols,
        nbuild_machine_literal_path(&object_path.display().to_string()),
        nbuild_machine_literal_path(&exe_path.display().to_string()),
    )
}

fn nbuild_entry_stc_init_symbol(
    object: &ncc_ir::BackendObjectIR,
    entry_symbol: &str,
) -> Option<String> {
    let container = entry_symbol.split('.').next()?.trim();
    if container.is_empty() || container == entry_symbol {
        return None;
    }
    let init = format!("__ncc_stc_init.{container}");
    object
        .symbols
        .iter()
        .any(|symbol| symbol.name == init)
        .then_some(init)
}

fn nbuild_machine_system_main_symbol(spec: &ncc_ir::BackendTargetSpecIR) -> &'static str {
    if matches!(spec.object_format, ncc_ir::BackendObjectFormatIR::MachO) {
        "_main"
    } else {
        "main"
    }
}

fn nbuild_machine_system_printf_symbol(spec: &ncc_ir::BackendTargetSpecIR) -> &'static str {
    if matches!(spec.object_format, ncc_ir::BackendObjectFormatIR::MachO) {
        "_printf"
    } else {
        "printf"
    }
}

fn nbuild_machine_db_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| byte.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Debug, Clone)]
struct NbuildMachineSourceIdentity {
    source_identity_sha256: String,
    source_content_sha256: String,
    backend_object_sha256: String,
    type_layout_units: usize,
    container_storage_symbols: usize,
}

fn nbuild_machine_source_identity(
    source_path: &str,
    object: &ncc_ir::BackendObjectIR,
    backend_material_path: Option<&Path>,
) -> NbuildMachineSourceIdentity {
    let source_bytes = fs::read(source_path).unwrap_or_else(|_| source_path.as_bytes().to_vec());
    let canonical_source = Path::new(source_path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(source_path));
    let mut identity_bytes = canonical_source.to_string_lossy().as_bytes().to_vec();
    identity_bytes.push(0);
    identity_bytes.extend_from_slice(&source_bytes);
    let backend_text = backend_material_path
        .and_then(|path| fs::read_to_string(path).ok())
        .unwrap_or_else(|| ncc_ir::render_object_text(object));
    let retained_type_layout_units = backend_text
        .lines()
        .filter(|line| line.trim_start().starts_with(".section .meta.type."))
        .count();
    let retained_container_storage_symbols = backend_text
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            line.starts_with("__ncc_stc_")
                && !line.starts_with("__ncc_stc_init.")
                && line.contains(" section=")
        })
        .count();
    NbuildMachineSourceIdentity {
        source_identity_sha256: crate::content_cache::sha256_bytes(&identity_bytes),
        source_content_sha256: crate::content_cache::sha256_bytes(&source_bytes),
        backend_object_sha256: crate::content_cache::sha256_bytes(backend_text.as_bytes()),
        type_layout_units: retained_type_layout_units,
        container_storage_symbols: retained_container_storage_symbols,
    }
}

fn nbuild_machine_source_identity_data(
    symbol: &str,
    identity: &NbuildMachineSourceIdentity,
) -> String {
    let payload = format!(
        "nC.source.machine.v2\0{}\0{}\0{}\0types={}\0storage={}\0",
        identity.source_identity_sha256,
        identity.source_content_sha256,
        identity.backend_object_sha256,
        identity.type_layout_units,
        identity.container_storage_symbols,
    );
    format!(
        "\n    data:\n        {symbol}:\n            db {};\n",
        nbuild_machine_db_bytes(payload.as_bytes())
    )
}

#[cfg(test)]
mod nbuild_machine_source_identity_tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn empty_sysv_object() -> ncc_ir::BackendObjectIR {
        empty_object(ncc_ir::BackendLoweringProfileIR::nc_sysv64())
    }

    fn empty_object(profile: ncc_ir::BackendLoweringProfileIR) -> ncc_ir::BackendObjectIR {
        ncc_ir::BackendObjectIR {
            target: profile.target,
            target_spec: profile.target_spec,
            sections: Vec::new(),
            symbols: Vec::new(),
            relocations: Vec::new(),
        }
    }

    #[test]
    fn every_source_materializes_a_distinct_valid_machine_identity() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be after the Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "ncc_machine_source_identity_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(&root).expect("create machine identity scratch directory");
        let first_source = root.join("first.n");
        let second_source = root.join("second.n");
        fs::write(&first_source, b"first { fa: return true; }").expect("write first source");
        fs::write(&second_source, b"second { fa: return false; }").expect("write second source");

        let object = empty_sysv_object();
        let backend_material = root.join("program.backend-object.nbo");
        fs::write(
            &backend_material,
            ".section .meta.type.program ; kind=type-meta\n\
.symbols\n\
    __ncc_stc_program_left section=.data.program.stc offset=0 visibility=global\n\
    __ncc_stc_program_right section=.data.program.stc offset=8 visibility=global\n",
        )
        .expect("write retained backend material");
        let material_identity = nbuild_machine_source_identity(
            first_source.to_string_lossy().as_ref(),
            &object,
            Some(&backend_material),
        );
        assert_eq!(material_identity.type_layout_units, 1);
        assert_eq!(material_identity.container_storage_symbols, 2);
        let first = nbuild_retained_machine_bridge_source(
            first_source.to_string_lossy().as_ref(),
            "shared_name",
            &object,
            "first.run",
            false,
            &root.join("first.obj"),
            &root.join("first.exe"),
        );
        let second = nbuild_retained_machine_bridge_source(
            second_source.to_string_lossy().as_ref(),
            "shared_name",
            &object,
            "first.run",
            false,
            &root.join("second.obj"),
            &root.join("second.exe"),
        );

        assert_ne!(first, second, "source identity must change machine bytes");
        assert!(first.contains("sourceIdentitySha256:"));
        assert!(second.contains("sourceIdentitySha256:"));
        assert!(first.contains("__ncc_source_shared_name:"));
        assert!(second.contains("__ncc_source_shared_name:"));
        let first_nm = root.join("first.nm");
        let second_nm = root.join("second.nm");
        fs::write(&first_nm, first).expect("write first machine bridge");
        fs::write(&second_nm, second).expect("write second machine bridge");
        crate::commands::machine::check_machine_source_path(first_nm.to_string_lossy().as_ref())
            .expect("first generated machine bridge must validate");
        crate::commands::machine::check_machine_source_path(second_nm.to_string_lossy().as_ref())
            .expect("second generated machine bridge must validate");

        fs::remove_dir_all(&root).expect("remove machine identity scratch directory");
    }

    #[test]
    fn bool_entries_normalize_nc_success_to_host_exit_zero_per_isa() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock must be after the Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "ncc_bool_entry_bridge_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(&root).expect("create bool entry bridge directory");
        let source = root.join("bool_entry.n");
        fs::write(&source, b"boolEntry { run :: fn() -> bool { return true; } ref main ->> run; fa: return true; }")
            .expect("write bool entry source");

        let sysv = nbuild_hosted_machine_entry_source(
            source.to_string_lossy().as_ref(),
            "bool_sysv",
            &empty_object(ncc_ir::BackendLoweringProfileIR::nc_sysv64()),
            "boolEntry.run",
            true,
            &root.join("bool_sysv.o"),
            &root.join("bool_sysv"),
            None,
            None,
            true,
        );
        assert!(sysv.contains("xor eax, 1;\n        mov rdi, rax;"));

        let mut sysv_init_object = empty_object(ncc_ir::BackendLoweringProfileIR::nc_sysv64());
        sysv_init_object
            .symbols
            .push(ncc_ir::BackendObjectSymbolIR {
                name: "__ncc_stc_init.boolEntry".into(),
                section: ".text.__ncc_stc_init.boolEntry".into(),
                offset: 0,
                visibility: ncc_ir::BackendSymbolVisibilityIR::Global,
            });
        let sysv_init = nbuild_hosted_machine_entry_source(
            source.to_string_lossy().as_ref(),
            "bool_sysv_init",
            &sysv_init_object,
            "boolEntry.run",
            true,
            &root.join("bool_sysv_init.o"),
            &root.join("bool_sysv_init"),
            None,
            None,
            true,
        );
        assert!(sysv_init.contains(
            "push rdi;\n        push rsi;\n        push rdx;\n        call __ncc_stc_init.boolEntry;\n        pop rdx;\n        pop rsi;\n        pop rdi;"
        ));

        let sysv_capture = nbuild_hosted_machine_entry_source(
            source.to_string_lossy().as_ref(),
            "bool_sysv_capture",
            &empty_object(ncc_ir::BackendLoweringProfileIR::nc_sysv64()),
            "boolEntry.run",
            true,
            &root.join("bool_sysv_capture.o"),
            &root.join("bool_sysv_capture"),
            None,
            Some(NCC_NATIVE_RETURN_MARKER),
            true,
        );
        assert!(sysv_capture.contains("ncc_return_fmt:"));
        assert!(sysv_capture.contains("mov rdi, 0;\n        call exit;"));

        let mut sysv_stdio_object = empty_object(ncc_ir::BackendLoweringProfileIR::nc_sysv64());
        sysv_stdio_object
            .relocations
            .push(ncc_ir::BackendObjectRelocationIR {
                section: ".text.boolEntry.run".into(),
                offset: 0,
                kind: ncc_ir::BackendObjectRelocationKindIR::CallNear32,
                symbol: "printf".into(),
                addend: 0,
            });
        let sysv_stdio = nbuild_hosted_machine_entry_source(
            source.to_string_lossy().as_ref(),
            "bool_sysv_stdio",
            &sysv_stdio_object,
            "boolEntry.run",
            true,
            &root.join("bool_sysv_stdio.o"),
            &root.join("bool_sysv_stdio"),
            None,
            None,
            true,
        );
        assert!(sysv_stdio.contains("exit:"));
        assert!(sysv_stdio.contains("xor eax, 1;\n        mov rdi, rax;\n        call exit;"));
        assert!(!sysv_stdio.contains("mov rax, 60;"));

        let win64 = nbuild_retained_machine_bridge_source(
            source.to_string_lossy().as_ref(),
            "bool_win64",
            &empty_object(ncc_ir::BackendLoweringProfileIR::nc_win64()),
            "boolEntry.run",
            true,
            &root.join("bool_win64.obj"),
            &root.join("bool_win64.exe"),
        );
        assert!(win64.contains("xor eax, 1;\n        mov rbx, rax;"));

        let aarch64 = nbuild_hosted_machine_entry_source(
            source.to_string_lossy().as_ref(),
            "bool_aarch64",
            &empty_object(ncc_ir::BackendLoweringProfileIR::nc_aapcs64()),
            "boolEntry.run",
            true,
            &root.join("bool_aarch64.o"),
            &root.join("bool_aarch64"),
            None,
            None,
            true,
        );
        assert!(aarch64.contains("mov w1, 1;\n        sub w0, w1, w0;"));

        for (name, machine_source) in [
            ("bool_sysv.nm", sysv),
            ("bool_sysv_init.nm", sysv_init),
            ("bool_sysv_capture.nm", sysv_capture),
            ("bool_sysv_stdio.nm", sysv_stdio),
            ("bool_win64.nm", win64),
            ("bool_aarch64.nm", aarch64),
        ] {
            let path = root.join(name);
            fs::write(&path, machine_source).expect("write generated bool entry bridge");
            crate::commands::machine::check_machine_source_path(path.to_string_lossy().as_ref())
                .expect("generated bool entry bridge must validate as machine source");
        }

        fs::remove_dir_all(&root).expect("remove bool entry bridge directory");
    }
}

fn nbuild_machine_literal_path(raw: &str) -> String {
    raw.strip_prefix(r"\\?\")
        .unwrap_or(raw)
        .replace('\\', "/")
        .replace('"', "'")
}

pub(crate) fn nbuild_hosted_machine_entry_source(
    source_path: &str,
    stem: &str,
    object: &ncc_ir::BackendObjectIR,
    entry_symbol: &str,
    entry_returns_bool: bool,
    object_path: &Path,
    exe_path: &Path,
    backend_material_path: Option<&Path>,
    return_marker: Option<&str>,
    direct_os_entry: bool,
) -> String {
    let container = nbuild_machine_ident(stem);
    let system_main = nbuild_machine_system_main_symbol(&object.target_spec);
    let printf_symbol = nbuild_machine_system_printf_symbol(&object.target_spec);
    let machine_entry_symbol = entry_symbol.replace('"', "'");
    let init_symbol = nbuild_entry_stc_init_symbol(object, &machine_entry_symbol);
    let arch = nbuild_machine_arch(&object.target_spec);
    let abi = nbuild_machine_abi(&object.target_spec);
    let object_format = nbuild_machine_object(&object.target_spec);
    let pointer_width = object.target_spec.pointer_width;
    let capture_return = return_marker.is_some();
    let identity = nbuild_machine_source_identity(source_path, object, backend_material_path);
    let identity_symbol = nbuild_machine_ident(&format!("__ncc_source_{stem}"));
    let hosted_stdio_referenced = object.unresolved_symbols().iter().any(|symbol| {
        matches!(
            symbol.trim_start_matches('_'),
            "printf"
                | "fprintf"
                | "vprintf"
                | "vfprintf"
                | "puts"
                | "putchar"
                | "fputs"
                | "fwrite"
                | "fflush"
        )
    });
    let hosted_os_entry = direct_os_entry
        && (object.target_spec.target_triple.contains("linux")
            || object.target_spec.target_triple.contains("windows")
            || object.target_spec.target_triple.contains("darwin"));
    let uses_raw_linux_exit = hosted_os_entry
        && object.target_spec.target_triple.contains("linux")
        && !capture_return
        && !hosted_stdio_referenced
        && matches!(
            object.target_spec.isa,
            ncc_ir::BackendIsaIR::X86
                | ncc_ir::BackendIsaIR::X86_64
                | ncc_ir::BackendIsaIR::AArch64
                | ncc_ir::BackendIsaIR::RiscV32
                | ncc_ir::BackendIsaIR::RiscV64
        );
    let mut externs = format!("        {machine_entry_symbol}:");
    if let Some(init) = init_symbol.as_deref() {
        externs.push_str(&format!("\n        {init}:"));
    }
    if capture_return {
        externs.push_str(&format!("\n        {printf_symbol}:"));
    }
    let hosted_exit_symbol = if matches!(
        object.target_spec.object_format,
        ncc_ir::BackendObjectFormatIR::MachO
    ) {
        "_exit"
    } else {
        "exit"
    };
    if hosted_os_entry && !uses_raw_linux_exit {
        if matches!(object.target_spec.abi, ncc_ir::BackendAbiModelIR::Win64) {
            externs.push_str("\n        fflush:\n        ExitProcess:");
        } else {
            externs.push_str(&format!("\n        {hosted_exit_symbol}:"));
        }
    }
    let init_x86 = init_symbol
        .as_deref()
        .map(|symbol| format!("        call {symbol};\n"))
        .unwrap_or_default();
    let init_x86_sysv = init_symbol
        .as_deref()
        .map(|symbol| {
            format!(
                "        push rdi;\n        push rsi;\n        push rdx;\n        call {symbol};\n        pop rdx;\n        pop rsi;\n        pop rdi;\n"
            )
        })
        .unwrap_or_default();
    let init_aarch64 = init_symbol
        .as_deref()
        .map(|symbol| format!("        bl {symbol};\n"))
        .unwrap_or_default();
    let init_riscv = init_symbol
        .as_deref()
        .map(|symbol| format!("        call {symbol};\n"))
        .unwrap_or_default();
    let normalize_x86 = entry_returns_bool
        .then_some("        xor eax, 1;\n")
        .unwrap_or_default();
    let normalize_armv7 = entry_returns_bool
        .then_some("        mov r1, 1;\n        eor r0, r1;\n")
        .unwrap_or_default();
    let normalize_aarch64 = entry_returns_bool
        .then_some("        mov w1, 1;\n        sub w0, w1, w0;\n")
        .unwrap_or_default();
    let normalize_riscv = entry_returns_bool
        .then_some("        xori a0, a0, 1;\n")
        .unwrap_or_default();
    let mut data = nbuild_machine_source_identity_data(&identity_symbol, &identity);
    if let Some(marker) = return_marker {
        let mut fmt = marker.as_bytes().to_vec();
        fmt.extend_from_slice(b"%lld\n\0");
        data.push_str(&format!(
            "        ncc_return_fmt:\n            db {};\n",
            nbuild_machine_db_bytes(&fmt)
        ));
    }
    let direct_exit = |return_register: &str| -> Option<String> {
        if !hosted_os_entry {
            return None;
        }
        match (
            object.target_spec.object_format.clone(),
            object.target_spec.isa.clone(),
        ) {
            (ncc_ir::BackendObjectFormatIR::Elf32, ncc_ir::BackendIsaIR::X86)
                if uses_raw_linux_exit =>
            {
                Some(
                    "        mov ebx, eax;\n        mov eax, 1;\n        int 128;\n        ret;"
                        .to_string(),
                )
            }
            (ncc_ir::BackendObjectFormatIR::Elf64, ncc_ir::BackendIsaIR::X86_64)
                if uses_raw_linux_exit =>
            {
                Some(
                    "        mov rdi, rax;\n        mov rax, 60;\n        syscall;\n        ret;"
                        .to_string(),
                )
            }
            (ncc_ir::BackendObjectFormatIR::Elf64, ncc_ir::BackendIsaIR::AArch64)
                if uses_raw_linux_exit =>
            {
                Some("        mov x8, 93;\n        svc 0;\n        ret;".to_string())
            }
            (ncc_ir::BackendObjectFormatIR::Elf32, ncc_ir::BackendIsaIR::X86) => Some(format!(
                "        push {return_register};\n        call {hosted_exit_symbol};\n        add esp, 4;\n        ret;"
            )),
            (ncc_ir::BackendObjectFormatIR::Elf64, ncc_ir::BackendIsaIR::X86_64) => Some(format!(
                "        mov rdi, {return_register};\n        call {hosted_exit_symbol};\n        add rsp, 8;\n        ret;"
            )),
            (ncc_ir::BackendObjectFormatIR::Elf64, ncc_ir::BackendIsaIR::AArch64) => {
                Some(format!("        bl {hosted_exit_symbol};\n        ret;"))
            }
            (
                ncc_ir::BackendObjectFormatIR::Elf32 | ncc_ir::BackendObjectFormatIR::Elf64,
                ncc_ir::BackendIsaIR::RiscV64 | ncc_ir::BackendIsaIR::RiscV32,
            ) if uses_raw_linux_exit => {
                Some("        addi a7, zero, 93;\n        ecall;\n        ret;".to_string())
            }
            (ncc_ir::BackendObjectFormatIR::MachO, ncc_ir::BackendIsaIR::X86_64) => Some(format!(
                "        mov rdi, {return_register};\n        call {hosted_exit_symbol};\n        add rsp, 8;\n        ret;"
            )),
            (ncc_ir::BackendObjectFormatIR::MachO, ncc_ir::BackendIsaIR::AArch64) => {
                Some(format!("        bl {hosted_exit_symbol};\n        ret;"))
            }
            _ => None,
        }
    };
    let instr = match object.target_spec.isa {
        ncc_ir::BackendIsaIR::X86 => {
            let tail = direct_exit("eax")
                .unwrap_or_else(|| "        mov eax, 0;\n        ret;".to_string());
            format!("{init_x86}        call {machine_entry_symbol};\n{normalize_x86}{tail}")
        }
        ncc_ir::BackendIsaIR::X86_64 => {
            if matches!(object.target_spec.abi, ncc_ir::BackendAbiModelIR::Win64) {
                if hosted_os_entry {
                    format!(
                        "        sub rsp, 40;\n{init_x86}        call {machine_entry_symbol};\n{normalize_x86}        mov rbx, rax;\n        mov rcx, 0;\n        call fflush;\n        mov rcx, rbx;\n        call ExitProcess;\n        add rsp, 40;\n        ret;"
                    )
                } else if capture_return {
                    format!(
                        "        sub rsp, 40;\n{init_x86}        call {machine_entry_symbol};\n        mov rdx, rax;\n        lea rcx, [rel ncc_return_fmt];\n        call {printf_symbol};\n        add rsp, 40;\n        mov rax, 0;\n        ret;"
                    )
                } else {
                    format!(
                        "        sub rsp, 40;\n{init_x86}        call {machine_entry_symbol};\n        add rsp, 40;\n        mov rax, 0;\n        ret;"
                    )
                }
            } else if capture_return {
                if hosted_os_entry {
                    format!(
                        "{init_x86_sysv}        sub rsp, 8;\n        call {machine_entry_symbol};\n        mov rsi, rax;\n        lea rdi, [rel ncc_return_fmt];\n        mov rax, 0;\n        call {printf_symbol};\n        mov rdi, 0;\n        call {hosted_exit_symbol};\n        add rsp, 8;\n        mov rax, 0;\n        ret;"
                    )
                } else {
                    format!(
                        "{init_x86_sysv}        sub rsp, 8;\n        call {machine_entry_symbol};\n        mov rsi, rax;\n        lea rdi, [rel ncc_return_fmt];\n        mov rax, 0;\n        call {printf_symbol};\n        add rsp, 8;\n        mov rax, 0;\n        ret;"
                    )
                }
            } else {
                let tail = direct_exit("rax")
                    .unwrap_or_else(|| "        mov rax, 0;\n        ret;".to_string());
                if hosted_os_entry {
                    format!("{init_x86_sysv}        sub rsp, 8;\n        call {machine_entry_symbol};\n{normalize_x86}{tail}")
                } else {
                    format!(
                        "{init_x86_sysv}        sub rsp, 8;\n        call {machine_entry_symbol};\n{normalize_x86}        add rsp, 8;\n{tail}"
                    )
                }
            }
        }
        ncc_ir::BackendIsaIR::ArmV7 => {
            format!(
                "{init_aarch64}        bl {machine_entry_symbol};\n{normalize_armv7}        bx lr;"
            )
        }
        ncc_ir::BackendIsaIR::AArch64 => {
            let tail = direct_exit("x0").unwrap_or_else(|| "        ret;".to_string());
            format!("{init_aarch64}        bl {machine_entry_symbol};\n{normalize_aarch64}{tail}")
        }
        ncc_ir::BackendIsaIR::RiscV64 => {
            let tail = direct_exit("a0").unwrap_or_else(|| "        ret;".to_string());
            format!("{init_riscv}        call {machine_entry_symbol};\n{normalize_riscv}{tail}")
        }
        ncc_ir::BackendIsaIR::RiscV32 => {
            let tail = direct_exit("a0").unwrap_or_else(|| "        ret;".to_string());
            format!("{init_riscv}        call {machine_entry_symbol};\n{normalize_riscv}{tail}")
        }
        ncc_ir::BackendIsaIR::Wasm32 => "        unreachable;".to_string(),
    };

    format!(
        "{container} :: machine {{\n    target:\n        arch: {arch};\n        abi: {abi};\n        object: {object_format};\n        endian: little;\n        pointerWidth: {pointer_width};\n        profile: userland;\n\n    debug:\n        file: \"{}\";\n        source: \"{}\";\n        entry: \"{}\";\n        producer: \"ncc hosted .n -> root -> .nm -> .nbo -> .obj -> .exe entry bridge\";\n        macho_unwind: compact;\n\n    extern:\n{externs}\n\n    symbol:\n        {system_main}:\n            section: .text;\n            binding: global;\n            type: routine;\n\n    entry:\n        {system_main};\n\n    proc:\n        {system_main}:\n\n    instr:\n{instr}\n{data}\n    verify:\n        target;\n        encoding;\n        object;\n\n    seal:\n        artifact: executable;\n        require: verified;\n\n    evidence:\n        sourceIdentitySha256: \"{}\";\n        sourceContentSha256: \"{}\";\n        backendObjectSha256: \"{}\";\n        typeLayoutUnits: \"{}\";\n        containerStorageSymbols: \"{}\";\n        object: \"{}\";\n        executable: \"{}\";\n}}\n",
        nbuild_machine_literal_path(source_path),
        nbuild_machine_literal_path(source_path),
        entry_symbol.replace('"', "'"),
        identity.source_identity_sha256,
        identity.source_content_sha256,
        identity.backend_object_sha256,
        identity.type_layout_units,
        identity.container_storage_symbols,
        nbuild_machine_literal_path(&object_path.display().to_string()),
        nbuild_machine_literal_path(&exe_path.display().to_string()),
        data = data,
    )
}

fn nbuild_artifact_record(
    path: &Path,
    kind: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let metadata = fs::metadata(path)?;
    Ok(serde_json::json!({
        "kind": kind,
        "path": path.display().to_string(),
        "size": metadata.len(),
        "sha256": sha256_hex_file(path)?,
    }))
}

#[derive(Debug, Clone)]
struct NbuildTargetInitArtifact {
    path: PathBuf,
    report: serde_json::Value,
}

fn nbuild_target_file_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn nbuild_target_executable_kind(target: &str) -> &'static str {
    if target.contains("windows") {
        "pe"
    } else if target.contains("darwin") || target.contains("apple") {
        "macho"
    } else if target.starts_with("wasm32-") {
        "wasm"
    } else {
        "elf"
    }
}

fn nbuild_target_loader_kind(target: &str) -> &'static str {
    match nbuild_target_executable_kind(target) {
        "pe" => "windows-pe-loader",
        "macho" => "darwin-macho-loader",
        "wasm" => "wasi-module-loader",
        _ if target.contains("none") || target.contains("unknown-elf") => "firmware-elf-loader",
        _ => "host-elf-loader",
    }
}

#[allow(clippy::too_many_arguments)]
fn nbuild_materialize_target_init(
    stem: &str,
    artifact_root: &Path,
    target: &str,
    arch: &str,
    abi: &str,
    object_format: &str,
    pointer_width: usize,
    entry: &str,
    machine_source_path: &Path,
    startup_payload_path: &Path,
    payload_kind: &str,
) -> Result<NbuildTargetInitArtifact, Box<dyn std::error::Error>> {
    let machine_bytes = fs::read(machine_source_path)?;
    let startup_bytes = fs::read(startup_payload_path)?;
    if machine_bytes.is_empty() || startup_bytes.is_empty() {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "target init requires non-empty Machine and startup payloads: machine='{}' payload='{}'",
                machine_source_path.display(),
                startup_payload_path.display()
            ),
        )
        .into());
    }

    let machine_sha256 = crate::content_cache::sha256_bytes(&machine_bytes);
    let payload_sha256 = crate::content_cache::sha256_bytes(&startup_bytes);
    let executable_kind = nbuild_target_executable_kind(target);
    let loader_kind = nbuild_target_loader_kind(target);
    let header = format!(
        "NCI1\nschema=nc.compiler.target-init.v1\ntarget={target}\narch={arch}\nabi={abi}\nobject={object_format}\nexecutable-kind={executable_kind}\nloader={loader_kind}\npointer-width={pointer_width}\nendian=little\nentry={entry}\nmachine-sha256={machine_sha256}\npayload-kind={payload_kind}\npayload-sha256={payload_sha256}\npayload-bytes={}\n\n",
        startup_bytes.len()
    );
    let mut bytes = Vec::with_capacity(header.len() + startup_bytes.len());
    bytes.extend_from_slice(header.as_bytes());
    bytes.extend_from_slice(&startup_bytes);

    let init_dir = artifact_root.join("init");
    fs::create_dir_all(&init_dir)?;
    let target_component = nbuild_target_file_component(target);
    let path = init_dir.join(format!("{stem}.{target_component}.init"));
    fs::write(&path, &bytes)?;
    require_non_empty_output_file("nbuild target init", path.to_string_lossy().as_ref())?;

    let retained = fs::read(&path)?;
    let header_end = retained
        .windows(2)
        .position(|window| window == b"\n\n")
        .map(|offset| offset + 2)
        .ok_or_else(|| {
            actionable_command_error("nbuild", "target init has no header terminator")
        })?;
    let retained_header = std::str::from_utf8(&retained[..header_end])
        .map_err(|_| actionable_command_error("nbuild", "target init header is not valid UTF-8"))?;
    let retained_payload = &retained[header_end..];
    if !retained.starts_with(b"NCI1\n")
        || !retained_header.contains("schema=nc.compiler.target-init.v1\n")
        || !retained_header.contains(&format!("machine-sha256={machine_sha256}\n"))
        || !retained_header.contains(&format!("payload-sha256={payload_sha256}\n"))
        || retained_payload != startup_bytes
    {
        return Err(actionable_command_error(
            "nbuild",
            "target init read-back verification rejected its header, hashes or startup bytes",
        )
        .into());
    }

    let report = serde_json::json!({
        "schema": "nc.compiler.target-init.v1",
        "status": "materialized_and_read_back_verified",
        "path": path.display().to_string(),
        "target": target,
        "arch": arch,
        "abi": abi,
        "object_format": object_format,
        "executable_kind": executable_kind,
        "loader": loader_kind,
        "pointer_width": pointer_width,
        "endian": "little",
        "entry": entry,
        "machine_source": machine_source_path.display().to_string(),
        "machine_sha256": machine_sha256,
        "payload": startup_payload_path.display().to_string(),
        "payload_kind": payload_kind,
        "payload_sha256": payload_sha256,
        "payload_bytes": startup_bytes.len(),
        "container_bytes": retained.len(),
        "relocations_retained_in_payload": payload_kind != "wasm-module",
    });
    Ok(NbuildTargetInitArtifact { path, report })
}

fn nbuild_attach_target_init_payload(
    manifest: &mut serde_json::Value,
    init: &NbuildTargetInitArtifact,
) -> Result<(), Box<dyn std::error::Error>> {
    let artifact = nbuild_artifact_record(&init.path, "target_init")?;
    let object = manifest.as_object_mut().ok_or_else(|| {
        actionable_command_error("nbuild", "native build manifest must be a JSON object")
    })?;
    object.insert("target_init".to_string(), init.report.clone());

    if let Some(paths) = object
        .get_mut("paths")
        .and_then(serde_json::Value::as_object_mut)
    {
        paths.insert(
            "target_init".to_string(),
            serde_json::Value::String(init.path.display().to_string()),
        );
    }
    if let Some(artifacts) = object
        .get_mut("artifacts")
        .and_then(serde_json::Value::as_array_mut)
    {
        artifacts.push(artifact);
    }
    if let Some(order) = object
        .get_mut("lowering_lineage")
        .and_then(|value| value.get_mut("order"))
        .and_then(serde_json::Value::as_array_mut)
    {
        if !order
            .iter()
            .any(|value| value.as_str() == Some("target.init"))
        {
            let index = order
                .iter()
                .position(|value| {
                    value
                        .as_str()
                        .is_some_and(|value| value.contains("machine"))
                })
                .unwrap_or(order.len());
            order.insert(index, serde_json::Value::String("target.init".to_string()));
        }
    }
    for field in ["c_like_artifact_set", "complete_artifact_set"] {
        if let Some(values) = object
            .get_mut("speed_contract")
            .and_then(|value| value.get_mut(field))
            .and_then(serde_json::Value::as_array_mut)
        {
            if !values.iter().any(|value| value.as_str() == Some(".init")) {
                let index = values
                    .iter()
                    .position(|value| value.as_str() == Some(".nm"))
                    .unwrap_or(values.len());
                values.insert(index, serde_json::Value::String(".init".to_string()));
            }
        }
    }
    if let Some(pipeline) = object
        .get_mut("pipeline")
        .and_then(serde_json::Value::as_array_mut)
    {
        let step = "materialize and read back architecture target .init";
        if !pipeline.iter().any(|value| value.as_str() == Some(step)) {
            let index = pipeline
                .iter()
                .position(|value| value.as_str().is_some_and(|value| value.contains("link")))
                .unwrap_or(pipeline.len());
            pipeline.insert(index, serde_json::Value::String(step.to_string()));
        }
    }
    Ok(())
}

fn nbuild_should_use_llvm_fallback(message: &str) -> bool {
    message.contains("has no native bytes")
        || message.contains("refusing to materialize backend text as machine code")
}

fn run_nbuild_llvm_fallback(
    path: &str,
    out_path: &str,
    artifact_root: &Path,
    profile_args: &[String],
    profile_label: &str,
    opt_level: &str,
    json: bool,
    linked_program: Option<ncc_ast::Program>,
    project_linkage: Option<serde_json::Value>,
    source_execution: &NbuildSourceExecutionArtifact,
    basal_root: &NbuildBasalRootArtifacts,
    content_cache_root: Option<&Path>,
    profile: &ncc_ir::BackendLoweringProfileIR,
    resolved_entry: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let current_exe = std::env::current_exe()?;
    let mut args = vec![
        "compiler".to_string(),
        "build-native".to_string(),
        path.to_string(),
        "--out-dir".to_string(),
        artifact_root.display().to_string(),
        "--profile".to_string(),
        profile_label.to_string(),
        "--out".to_string(),
        out_path.to_string(),
    ];
    args.extend(profile_args.iter().cloned());
    if let Some(cache_root) = content_cache_root {
        args.push("--content-cache-dir".to_string());
        args.push(cache_root.display().to_string());
    } else {
        args.push("--no-incremental".to_string());
    }
    if opt_level != "backend-default" {
        args.push("--opt-level".to_string());
        args.push(opt_level.to_string());
    }
    if json {
        args.push("--json".to_string());
    }

    if let Some(program) = linked_program {
        let mut payload =
            super::compiler::compiler_build_native_payload_for_linked_program(&args[2..], program)?;
        if let (Some(linkage), Some(object)) = (project_linkage, payload.as_object_mut()) {
            object.insert("project_linkage".to_string(), linkage);
        }
        nbuild_attach_source_execution_payload(&mut payload, source_execution)?;
        payload["paths"]["basal_root_source"] =
            serde_json::json!(basal_root.source_path.display().to_string());
        payload["paths"]["basal_root_binary"] =
            serde_json::json!(basal_root.binary_path.display().to_string());
        payload["lowering_lineage"] = serde_json::json!({
            "schema": "nc.lowering.source-root-machine-nbo.v2",
            "status": "materialized",
            "basal": nbuild_basal_root_lineage_payload(basal_root),
            "order": ["source.n", "root.nb", "root.nbb", "machine.nm", "backend-object.nbo", "native-object", "executable"]
        });
        nbuild_attach_fallback_target_init(
            &mut payload,
            path,
            artifact_root,
            profile,
            resolved_entry,
        )?;
        if let Some(manifest) = payload
            .pointer("/paths/manifest")
            .and_then(|value| value.as_str())
        {
            fs::write(manifest, serde_json::to_string_pretty(&payload)?.as_bytes())?;
        }
        let success =
            payload.get("status").and_then(|value| value.as_str()) == Some("native_build_passed");
        if !crate::ncc_child_output_quiet() {
            if json {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else if success {
                println!(
                    "OK: nbuild project LLVM fallback exe={} route=linked-project-llvm-fallback",
                    out_path
                );
            }
        }
        if success {
            return Ok(());
        }
        return Err(
            actionable_command_error("nbuild", "linked project LLVM/MCT fallback failed").into(),
        );
    }

    let output = std::process::Command::new(&current_exe)
        .args(&args)
        .output()?;
    if output.status.success() {
        let manifest_path = artifact_root
            .join("run")
            .join(format!("{}.native-build.json", nbuild_source_stem(path)));
        if manifest_path.is_file() {
            let mut payload: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
            nbuild_attach_source_execution_payload(&mut payload, source_execution)?;
            payload["paths"]["basal_root_source"] =
                serde_json::json!(basal_root.source_path.display().to_string());
            payload["paths"]["basal_root_binary"] =
                serde_json::json!(basal_root.binary_path.display().to_string());
            payload["lowering_lineage"] = serde_json::json!({
                "schema": "nc.lowering.source-root-machine-nbo.v2",
                "status": "materialized",
                "basal": nbuild_basal_root_lineage_payload(basal_root),
                "order": ["source.n", "root.nb", "root.nbb", "machine.nm", "backend-object.nbo", "native-object", "executable"]
            });
            nbuild_attach_fallback_target_init(
                &mut payload,
                path,
                artifact_root,
                profile,
                resolved_entry,
            )?;
            fs::write(
                &manifest_path,
                serde_json::to_string_pretty(&payload)?.as_bytes(),
            )?;
        }
    }
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    if output.status.success() {
        Ok(())
    } else {
        Err(actionable_command_error(
            "nbuild",
            format!(
                "native-fast direct build was incomplete, and LLVM/MCT fallback failed with status {:?}",
                output.status.code()
            ),
        )
        .into())
    }
}

fn nbuild_attach_fallback_target_init(
    payload: &mut serde_json::Value,
    source_path: &str,
    artifact_root: &Path,
    profile: &ncc_ir::BackendLoweringProfileIR,
    resolved_entry: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let machine_source = payload
        .pointer("/paths/machine_source")
        .and_then(serde_json::Value::as_str)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            actionable_command_error(
                "nbuild",
                "LLVM/MCT fallback did not retain the Machine source required by target .init",
            )
        })?;
    let entry_object = payload
        .pointer("/paths/machine_entry_object")
        .and_then(serde_json::Value::as_str)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_file());
    let startup_payload = entry_object.or_else(|| {
        payload
            .pointer("/paths/object")
            .and_then(serde_json::Value::as_str)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .filter(|path| path.is_file())
    });
    let startup_payload = startup_payload.ok_or_else(|| {
        actionable_command_error(
            "nbuild",
            "LLVM/MCT fallback did not retain an entry or native object for target .init",
        )
    })?;
    let payload_kind = if payload
        .pointer("/paths/machine_entry_object")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|path| !path.is_empty() && Path::new(path) == startup_payload.as_path())
    {
        "machine-entry-object"
    } else {
        "native-object"
    };
    let target_init = nbuild_materialize_target_init(
        &nbuild_source_stem(source_path),
        artifact_root,
        profile.target_spec.target_triple.as_str(),
        nbuild_machine_arch(&profile.target_spec),
        nbuild_machine_abi(&profile.target_spec),
        nbuild_machine_object(&profile.target_spec),
        profile.target_spec.pointer_width,
        resolved_entry,
        &machine_source,
        &startup_payload,
        payload_kind,
    )?;
    nbuild_attach_target_init_payload(payload, &target_init)
}

fn nbuild_standard_target_requires_mct_body(profile: &ncc_ir::BackendLoweringProfileIR) -> bool {
    matches!(
        profile.target_spec.isa,
        ncc_ir::BackendIsaIR::X86
            | ncc_ir::BackendIsaIR::X86_64
            | ncc_ir::BackendIsaIR::AArch64
            | ncc_ir::BackendIsaIR::RiscV32
            | ncc_ir::BackendIsaIR::RiscV64
    )
}

fn nbuild_object_requires_json_runtime_object(object: &ncc_ir::BackendObjectIR) -> bool {
    object
        .unresolved_symbols()
        .iter()
        .any(|symbol| super::compiler::compiler_native_is_json_runtime_symbol(symbol))
}

fn nbuild_object_requires_mct_memory_map_runtime(object: &ncc_ir::BackendObjectIR) -> bool {
    object.unresolved_symbols().iter().any(|symbol| {
        matches!(
            symbol.rsplit('.').next().unwrap_or(symbol).trim(),
            "map"
                | "mapAnonymous"
                | "mapAt"
                | "mapShared"
                | "mapPrivate"
                | "mapFile"
                | "mapFileShared"
                | "mapFilePrivate"
                | "unmap"
                | "remap"
                | "remapAt"
                | "mapProtect"
                | "mapSync"
                | "mapAdvise"
                | "mapLock"
                | "mapUnlock"
                | "mapLen"
                | "mapAddress"
                | "mapIsMapped"
        )
    })
}

fn nbuild_wasm_output_path(
    source_path: &str,
    requested_out_path: &str,
    artifact_root: &Path,
    explicit_output: bool,
) -> PathBuf {
    if explicit_output {
        return PathBuf::from(requested_out_path);
    }
    artifact_root
        .join("bin")
        .join(format!("{}.wasm", nbuild_source_stem(source_path)))
}

fn nbuild_wasm_launcher_name(stem: &str, suffix: &str) -> String {
    format!("{stem}.wasm.{suffix}")
}

fn write_nbuild_wasm_launchers(
    wasm_path: &Path,
    bin_dir: &Path,
    code_dir: &Path,
    stem: &str,
) -> Result<(PathBuf, PathBuf, PathBuf, PathBuf), Box<dyn std::error::Error>> {
    fs::create_dir_all(bin_dir)?;
    fs::create_dir_all(code_dir)?;
    let wasm_abs = wasm_path
        .canonicalize()
        .unwrap_or_else(|_| wasm_path.to_path_buf())
        .to_string_lossy()
        .replace('\\', "\\\\");
    let launcher_n = code_dir.join(format!("{stem}_wasm_launcher.n"));
    let ps1 = bin_dir.join(nbuild_wasm_launcher_name(stem, "ps1"));
    let bat = bin_dir.join(nbuild_wasm_launcher_name(stem, "bat"));
    let sh = bin_dir.join(nbuild_wasm_launcher_name(stem, "sh"));

    fs::write(
        &launcher_n,
        format!(
            "use::stoud::io::<output>;\n\n{stem}WasmLauncher {{\n    to main {{\n        println(\"WASM module: {wasm_abs}\");\n        println(\"Run with wasmtime/wasmer/wasm3 through the generated host launcher.\");\n    }}\n\n    fa:\n        return true;\n}}\n"
        ),
    )?;
    fs::write(
        &ps1,
        format!(
            "$module = \"{wasm_abs}\"\nif (Get-Command wasmtime -ErrorAction SilentlyContinue) {{ & wasmtime $module @args; exit $LASTEXITCODE }}\nif (Get-Command wasmer -ErrorAction SilentlyContinue) {{ & wasmer run $module -- @args; exit $LASTEXITCODE }}\nif (Get-Command wasm3 -ErrorAction SilentlyContinue) {{ & wasm3 $module @args; exit $LASTEXITCODE }}\nWrite-Error \"No WASI runtime found. Install wasmtime, wasmer or wasm3.\"\nexit 127\n"
        ),
    )?;
    fs::write(
        &bat,
        format!(
            "@echo off\r\nset MODULE={wasm_abs}\r\nwhere wasmtime >nul 2>nul\r\nif %ERRORLEVEL%==0 (\r\n  wasmtime \"%MODULE%\" %*\r\n  exit /b %ERRORLEVEL%\r\n)\r\nwhere wasmer >nul 2>nul\r\nif %ERRORLEVEL%==0 (\r\n  wasmer run \"%MODULE%\" -- %*\r\n  exit /b %ERRORLEVEL%\r\n)\r\nwhere wasm3 >nul 2>nul\r\nif %ERRORLEVEL%==0 (\r\n  wasm3 \"%MODULE%\" %*\r\n  exit /b %ERRORLEVEL%\r\n)\r\necho No WASI runtime found. Install wasmtime, wasmer or wasm3.\r\nexit /b 127\r\n"
        ),
    )?;
    fs::write(
        &sh,
        format!(
            "#!/usr/bin/env sh\nset -eu\nmodule='{wasm_abs}'\nif command -v wasmtime >/dev/null 2>&1; then exec wasmtime \"$module\" \"$@\"; fi\nif command -v wasmer >/dev/null 2>&1; then exec wasmer run \"$module\" -- \"$@\"; fi\nif command -v wasm3 >/dev/null 2>&1; then exec wasm3 \"$module\" \"$@\"; fi\necho 'No WASI runtime found. Install wasmtime, wasmer or wasm3.' >&2\nexit 127\n"
        ),
    )?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&sh)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&sh, perms)?;
    }

    Ok((launcher_n, ps1, bat, sh))
}

fn run_nbuild_wasm_os_lane_command(
    source_path: &str,
    requested_out_path: &str,
    explicit_output: bool,
    artifact_root: &Path,
    ast: ncc_ast::Program,
    profile: ncc_ir::BackendLoweringProfileIR,
    execution_profile: NccExecutionProfile,
    opt_level: &str,
    json: bool,
    source_execution: &NbuildSourceExecutionArtifact,
    basal_root: &NbuildBasalRootArtifacts,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_backend_artifact_profile(BackendArtifactKind::BuildWasm, &profile)?;
    ncc_sema::validate_main_entries(&ast, ncc_sema::MainEntryRequirement::Required)
        .map_err(|err| -> Box<dyn std::error::Error> { err.into() })?;

    let route_started = Instant::now();
    let stem = nbuild_source_stem(source_path);
    let code_dir = artifact_root.join("code");
    let lowering_dir = artifact_root.join("lowering");
    let obj_dir = artifact_root.join("obj");
    let run_dir = artifact_root.join("run");
    let bin_dir = artifact_root.join("bin");
    for dir in [&code_dir, &lowering_dir, &obj_dir, &run_dir, &bin_dir] {
        fs::create_dir_all(dir)?;
    }
    super::compiler::compiler_native_remove_legacy_runtime_sources(&lowering_dir)?;

    let wasm_path = nbuild_wasm_output_path(
        source_path,
        requested_out_path,
        artifact_root,
        explicit_output,
    );
    if let Some(parent) = wasm_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let wasm = emit_wasm_from_ast_with_profile(&ast, profile.clone())?;
    fs::write(&wasm_path, &wasm)?;
    require_non_empty_output_file("nbuild wasm module", wasm_path.to_string_lossy().as_ref())?;

    let wasm_obj_path = obj_dir.join(format!("{stem}.wasm"));
    fs::write(&wasm_obj_path, &wasm)?;
    require_non_empty_output_file(
        "nbuild wasm object mirror",
        wasm_obj_path.to_string_lossy().as_ref(),
    )?;

    let source = fs::read_to_string(source_path)?;
    let mut retained_ir = ncc_ir::lower_with_backend_profile_and_source_exact(
        ast.clone(),
        profile.clone(),
        source_path,
        &source,
    );
    retained_ir.target = profile.target.clone();
    retained_ir.target_spec = profile.target_spec.clone();
    let pass_pipeline = ncc_ir::mct_llvm_pass_pipeline(
        nbuild_mct_optimization_level_for_standard_build(opt_level, execution_profile)?,
    );
    let retained_module = ncc_ir::mct_emit_llvm_module_with_debug_config_and_target_attributes(
        &retained_ir,
        pass_pipeline,
        super::mct_json::mct_llvm_debug_config_for_path(source_path),
        nbuild_standard_mct_target_attributes(&profile, execution_profile),
    );
    if retained_module.emitted_function_count == 0 || retained_module.rejected_function_count > 0 {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "WASM route could not retain complete LLVM for target '{}': emitted={} rejected={}; {}",
                profile.target_spec.target_triple,
                retained_module.emitted_function_count,
                retained_module.rejected_function_count,
                nbuild_mct_rejection_summary(&retained_module),
            ),
        )
        .into());
    }
    let llvm_ir_path = lowering_dir.join(format!("{stem}.wasm.mct.ll"));
    fs::write(&llvm_ir_path, retained_module.llvm_ir.as_bytes())?;
    require_non_empty_output_file(
        "nbuild wasm retained LLVM",
        llvm_ir_path.to_string_lossy().as_ref(),
    )?;

    let optimized = emit_optimized_backend_object_ir_from_ast_with_profile(&ast, profile.clone())?;
    let resolved_entry = resolve_native_entry_symbol(&ast, None);
    let machine_source_path = lowering_dir.join(format!("{stem}.nm"));
    let machine_source = nbuild_retained_machine_bridge_source(
        source_path,
        &stem,
        &optimized.object,
        &resolved_entry,
        native_entry_returns_bool(&ast, None),
        &wasm_obj_path,
        &wasm_path,
    );
    fs::write(&machine_source_path, machine_source.as_bytes())?;
    require_non_empty_output_file(
        "nbuild wasm machine source",
        machine_source_path.to_string_lossy().as_ref(),
    )?;
    super::machine::check_machine_source_path(machine_source_path.to_string_lossy().as_ref())?;

    let target_init = nbuild_materialize_target_init(
        &stem,
        artifact_root,
        profile.target_spec.target_triple.as_str(),
        "wasm32",
        "wasi",
        "wasm-module",
        profile.target_spec.pointer_width,
        &resolved_entry,
        &machine_source_path,
        &wasm_obj_path,
        "wasm-module",
    )?;

    let lowering_report_path = lowering_dir.join(format!("{stem}.wasm-lowering.json"));
    let lowering_payload = serde_json::json!({
        "schema": "nc.compiler.wasm_lowering.v1",
        "status": "wasm_lowering_passed",
        "source": source_path,
        "target": profile.target_spec.target_triple,
        "object_format": "wasm-module",
        "bytes": wasm.len(),
        "magic": [wasm.get(0).copied(), wasm.get(1).copied(), wasm.get(2).copied(), wasm.get(3).copied()],
    });
    fs::write(
        &lowering_report_path,
        serde_json::to_string_pretty(&lowering_payload)?.as_bytes(),
    )?;

    let (launcher_n, ps1, bat, sh) =
        write_nbuild_wasm_launchers(&wasm_path, &bin_dir, &code_dir, &stem)?;
    let manifest_path = run_dir.join(format!("{stem}.wasm-build.json"));
    let artifacts = vec![
        nbuild_artifact_record(&basal_root.source_path, "basal_root_source")?,
        nbuild_artifact_record(&basal_root.binary_path, "basal_root_binary")?,
        nbuild_artifact_record(&wasm_path, "wasm_module_executable")?,
        nbuild_artifact_record(&wasm_obj_path, "wasm_object_mirror")?,
        nbuild_artifact_record(&llvm_ir_path, "wasm_mct_llvm_ir")?,
        nbuild_artifact_record(&machine_source_path, "wasm_machine_source")?,
        nbuild_artifact_record(&lowering_report_path, "wasm_lowering_report")?,
        nbuild_artifact_record(&launcher_n, "launcher_n")?,
        nbuild_artifact_record(&ps1, "powershell_launcher")?,
        nbuild_artifact_record(&bat, "batch_launcher")?,
        nbuild_artifact_record(&sh, "shell_launcher")?,
    ];
    let mut payload = serde_json::json!({
        "schema": "nc.compiler.standard_wasm_build.v2",
        "status": "wasm_os_lane_passed",
        "source": source_path,
        "source_sha256": sha256_hex_file(Path::new(source_path))?,
        "artifact_root": artifact_root.display().to_string(),
        "target": {
            "triple": profile.target_spec.target_triple,
            "isa": "wasm32",
            "abi": "wasi",
            "object_format": "wasm-module",
            "pointer_width": profile.target_spec.pointer_width,
        },
        "paths": {
            "basal_root_source": basal_root.source_path.display().to_string(),
            "basal_root_binary": basal_root.binary_path.display().to_string(),
            "wasm_module": wasm_path.display().to_string(),
            "wasm_object_mirror": wasm_obj_path.display().to_string(),
            "llvm_ir": llvm_ir_path.display().to_string(),
            "machine_source": machine_source_path.display().to_string(),
            "lowering_report": lowering_report_path.display().to_string(),
            "launcher_n": launcher_n.display().to_string(),
            "powershell_launcher": ps1.display().to_string(),
            "batch_launcher": bat.display().to_string(),
            "shell_launcher": sh.display().to_string(),
            "manifest": manifest_path.display().to_string(),
        },
        "lowering_lineage": {
            "schema": "nc.lowering.source-root-machine-nbo.v2",
            "status": "materialized",
            "basal": nbuild_basal_root_lineage_payload(basal_root),
            "machine": {
                "path": machine_source_path.display().to_string(),
                "sha256": sha256_hex_file(&machine_source_path)?,
                "container_matches_file_stem": true
            },
            "order": ["source.n", "root.nb", "root.nbb", "machine.nm", "wasm-module"]
        },
        "pipeline": [
            "parse .n",
            "sema",
            "lower backend IR with portable profile",
            "retain target LLVM IR",
            "retain and validate per-source machine bridge",
            "emit wasm32 module bytes",
            "write OS launchers for WASI runtimes"
        ],
        "artifacts": artifacts,
        "truth_boundary": {
            "wasm_module_is_real_backend_output": true,
            "llvm_ir_is_target_specific": true,
            "machine_source_is_per_input_and_validated": true,
            "wasm_magic_and_version_emitted": wasm.len() >= 8 && &wasm[0..4] == b"\0asm",
            "native_os_exe_generated": false,
            "os_executable_lane": "wasi-runtime-launcher",
            "requires_external_wasi_runtime": true,
            "missing_wasi_runtime_is_not_faked": true
        },
        "timing_ms": {
            "total_backend_route": nbuild_elapsed_ms(route_started)
        }
    });
    nbuild_attach_target_init_payload(&mut payload, &target_init)?;
    nbuild_attach_source_execution_payload(&mut payload, source_execution)?;
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&payload)?.as_bytes(),
    )?;

    if !crate::ncc_child_output_quiet() {
        if json {
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!(
                "OK: nbuild wasm={} manifest={} launcher={} route=wasm32-wasi-os-lane",
                wasm_path.display(),
                manifest_path.display(),
                sh.display()
            );
        }
    }
    Ok(())
}

fn nbuild_mct_optimization_level_for_standard_build(
    opt_level: &str,
    execution_profile: NccExecutionProfile,
) -> Result<ncc_ir::MctLlvmOptimizationLevelIR, Box<dyn std::error::Error>> {
    if opt_level == "backend-default" {
        return Ok(match execution_profile {
            NccExecutionProfile::Debug => ncc_ir::MctLlvmOptimizationLevelIR::O0,
            NccExecutionProfile::Release => ncc_ir::MctLlvmOptimizationLevelIR::O2,
            NccExecutionProfile::ReleaseFast => ncc_ir::MctLlvmOptimizationLevelIR::O3,
            NccExecutionProfile::ReleaseSmall => ncc_ir::MctLlvmOptimizationLevelIR::Os,
            NccExecutionProfile::UnsafeFast => ncc_ir::MctLlvmOptimizationLevelIR::O3,
            NccExecutionProfile::Instrumented => ncc_ir::MctLlvmOptimizationLevelIR::O1,
        });
    }
    super::mct_support::parse_mct_llvm_optimization_level_value(opt_level)
}

fn nbuild_effective_optimization_level_label(
    opt_level: &str,
    execution_profile: NccExecutionProfile,
) -> Result<&'static str, Box<dyn std::error::Error>> {
    Ok(ncc_ir::render_mct_llvm_optimization_level(
        nbuild_mct_optimization_level_for_standard_build(opt_level, execution_profile)?,
    ))
}

fn nbuild_requested_optimization_label(opt_level: &str) -> &str {
    if opt_level == "backend-default" {
        "auto"
    } else {
        opt_level
    }
}

fn nbuild_elapsed_ms(start: Instant) -> u64 {
    start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

fn nbuild_release_fast_auto_payload(
    route: &str,
    execution_profile: NccExecutionProfile,
    opt_level: &str,
    effective_opt_level: &str,
    timings: serde_json::Value,
    artifact_metrics: serde_json::Value,
    optimizer_payload: &serde_json::Value,
) -> serde_json::Value {
    let optimizer_totals = optimizer_payload
        .pointer("/internal_optimizer/totals")
        .or_else(|| optimizer_payload.pointer("/totals"));
    let optimizer_metric = |name: &str| {
        optimizer_totals
            .and_then(|totals| totals.get(name))
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    };
    serde_json::json!({
        "schema": "nc.compiler.release_fast_auto.v1",
        "status": if execution_profile == NccExecutionProfile::ReleaseFast {
            "automatic_release_fast_applied"
        } else {
            "explicit_profile_applied"
        },
        "normal_build_surface": true,
        "special_command_required": false,
        "profile": execution_profile.label(),
        "default_profile": "release-fast",
        "route": route,
        "optimization": {
            "requested": nbuild_requested_optimization_label(opt_level),
            "effective": effective_opt_level,
            "source": if opt_level == "backend-default" {
                "normal build profile default"
            } else {
                "explicit user option"
            }
        },
        "measurable": true,
        "timings_ms": timings,
        "artifact_metrics": artifact_metrics,
        "optimizer_report": {
            "schema": optimizer_payload.get("schema").cloned().unwrap_or(serde_json::Value::Null),
            "status": optimizer_payload.get("status").cloned().unwrap_or(serde_json::Value::Null),
            "release_fast_pipeline_status": optimizer_payload
                .pointer("/release_fast_pipeline/status")
                .or_else(|| optimizer_payload.pointer("/internal_optimizer/release_fast_pipeline/status"))
                .cloned()
                .unwrap_or_else(|| optimizer_payload
                    .pointer("/module/pass_pipeline/status")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null)),
            "ssa_transforms": optimizer_metric("ssa_transforms"),
            "inline_expansions": optimizer_metric("inline_expansions"),
            "loop_transforms": optimizer_metric("loop_transforms"),
            "register_allocation_live_intervals": optimizer_metric("register_allocation_live_intervals"),
            "instruction_selection_transforms": optimizer_metric("instruction_selection_transforms")
        },
        "pipeline": [
            "parse .n",
            "sema",
            "release-fast optimizer",
            "machine backend object",
            "native object",
            "native executable"
        ],
        "truth_boundary": {
            "integrated_in_ncc_build": true,
            "ncc_optimize_command_required": false,
            "interpreter_fallback": false,
            "object_and_executable_metrics_retained": true,
            "timings_are_current_build_measurements": true
        }
    })
}

fn nbuild_rename_mct_main_wrapper(
    llvm_ir: &str,
    entry_symbol: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let needle = "define i32 @main";
    if !llvm_ir.contains(needle) {
        return Err(actionable_command_error(
            "nbuild",
            "standard MCT object route expected an LLVM main wrapper, but none was emitted",
        )
        .into());
    }
    Ok(llvm_ir.replacen(needle, &format!("define i32 @{entry_symbol}"), 1))
}

fn nbuild_mct_rejection_summary(module: &ncc_ir::MctLlvmModuleIR) -> String {
    let rejected = module
        .functions
        .iter()
        .filter(|function| matches!(function.status, ncc_ir::MctLlvmFunctionStatusIR::Rejected))
        .map(|function| {
            let diagnostics = function
                .diagnostics
                .iter()
                .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
                .collect::<Vec<_>>()
                .join(" | ");
            if diagnostics.is_empty() {
                format!("{}: rejected without a diagnostic", function.source_name)
            } else {
                format!("{}: {diagnostics}", function.source_name)
            }
        })
        .collect::<Vec<_>>();
    if rejected.is_empty() {
        "no rejected function detail was emitted".to_string()
    } else {
        rejected.join("; ")
    }
}

#[derive(Debug, Clone)]
struct NbuildStandardMctBody {
    llvm_ir_path: PathBuf,
    object_path: PathBuf,
    profile_label: String,
    target_triple: String,
    entry_symbol: String,
    clang_tool: String,
    clang_args: Vec<String>,
    module: serde_json::Value,
    stage: serde_json::Value,
    hosted_runtime_referenced: bool,
    json_runtime_referenced: bool,
    unresolved_symbols: Vec<String>,
    incremental: serde_json::Value,
}

impl NbuildStandardMctBody {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "schema": "nc.compiler.standard_mct_machine_body.v1",
            "status": "materialized",
            "profile": self.profile_label,
            "target_triple": self.target_triple,
            "entry_symbol": self.entry_symbol,
            "llvm_ir": self.llvm_ir_path.display().to_string(),
            "object": self.object_path.display().to_string(),
            "llvm_sha256": sha256_hex_file(&self.llvm_ir_path).ok(),
            "object_sha256": sha256_hex_file(&self.object_path).ok(),
            "clang": {
                "tool": self.clang_tool,
                "args": self.clang_args,
                "stage": self.stage,
            },
            "module": self.module,
            "runtime": {
                "hosted_referenced": self.hosted_runtime_referenced,
                "json_referenced": self.json_runtime_referenced,
            },
            "unresolved_symbols": self.unresolved_symbols,
            "incremental": self.incremental,
            "truth_boundary": {
                "high_level_body_lowered_by_mct": true,
                "object_is_real_target_object": true,
                "machine_bridge_is_real_nm_object": true,
                "direct_backend_ir_is_retained_for_audit": true,
                "missing_tools_are_not_faked": true
            }
        })
    }
}

fn nbuild_standard_mct_target_attributes(
    profile: &ncc_ir::BackendLoweringProfileIR,
    execution_profile: NccExecutionProfile,
) -> ncc_ir::MctLlvmTargetAttributesIR {
    let mut attributes = ncc_ir::mct_llvm_target_attributes_for_cpu_tuning(
        &profile.target_spec,
        &profile.cpu_tuning.microarchitecture_id,
        &profile.cpu_tuning.fine_tuning_profile_id,
    )
    .unwrap_or_else(|_| ncc_ir::mct_llvm_target_attributes_for_spec(&profile.target_spec));

    if matches!(
        execution_profile,
        NccExecutionProfile::Release
            | NccExecutionProfile::ReleaseFast
            | NccExecutionProfile::ReleaseSmall
            | NccExecutionProfile::UnsafeFast
    ) {
        attributes
            .function_attributes
            .retain(|attribute| attribute.name != "uwtable");
        if let Some(frame_pointer) = attributes
            .function_attributes
            .iter_mut()
            .find(|attribute| attribute.name == "frame-pointer")
        {
            frame_pointer.value = Some("none".to_string());
            frame_pointer.llvm_ir = "\"frame-pointer\"=\"none\"".to_string();
        }
        attributes.attribute_group_ir = format!(
            "attributes {} = {{ {} }}\n",
            attributes.attribute_group,
            attributes
                .function_attributes
                .iter()
                .map(|attribute| attribute.llvm_ir.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        );
        attributes.status = "mct_llvm_target_attributes_cpu_tuned_release";
        attributes.diagnostics.push(ncc_ir::MctDiagnosticIR::new(
            "I_MCT_LLVM_RELEASE_ATTR_001",
            ncc_ir::MctDiagnosticSeverityIR::Info,
            format!(
                "Release MCT lowering uses CPU '{}' / tune '{}' without frame or unwind tables.",
                attributes.target_cpu,
                attributes.tune_cpu.as_deref().unwrap_or("target-default")
            ),
        ));
    }
    attributes
}

#[allow(clippy::too_many_arguments)]
fn nbuild_compile_standard_mct_body_object(
    ast: ncc_ast::Program,
    source_path: &str,
    source: &str,
    profile: ncc_ir::BackendLoweringProfileIR,
    profile_label: &str,
    opt_level: &str,
    execution_profile: NccExecutionProfile,
    stem: &str,
    lowering_dir: &Path,
    obj_path: &Path,
    backend_object_ir_path: &Path,
    basal_root: &NbuildBasalRootArtifacts,
    content_cache_root: Option<&Path>,
) -> Result<NbuildStandardMctBody, Box<dyn std::error::Error>> {
    let pass_pipeline = ncc_ir::mct_llvm_pass_pipeline(
        nbuild_mct_optimization_level_for_standard_build(opt_level, execution_profile)?,
    );
    let ll_path = lowering_dir.join(format!("{stem}.standard.mct.ll"));
    let clang_tool = super::mct_support::mct_find_tool("clang").ok_or_else(|| {
        actionable_command_error(
            "nbuild",
            "standard MCT object route requires clang to materialize LLVM IR into the requested target object",
        )
    })?;
    let ast_sha256 = crate::content_cache::sha256_bytes(&serde_json::to_vec(&ast)?);
    let cache_material = serde_json::json!({
        "schema": "ncc.nbuild.standard_mct_content_key.v1",
        "source": {
            "canonical_path": fs::canonicalize(source_path).unwrap_or_else(|_| PathBuf::from(source_path)).display().to_string(),
            "sha256": crate::content_cache::sha256_bytes(source.as_bytes()),
            "linked_ast_sha256": ast_sha256,
        },
        "compiler": crate::content_cache::compiler_image_identity(),
        "clang": crate::content_cache::tool_identity(Some(&clang_tool)),
        "backend_profile": format!("{profile:?}"),
        "profile_label": profile_label,
        "target": profile.target_spec.target_triple,
        "basal_root": {
            "source_sha256": basal_root.source_sha256,
            "binary_sha256": basal_root.binary_sha256,
        },
        "optimization": {
            "requested": opt_level,
            "execution_profile": execution_profile.label(),
            "llvm_level": ncc_ir::render_mct_llvm_optimization_level(pass_pipeline.level.clone()),
            "clang_arg": pass_pipeline.clang_opt_arg,
            "passes": pass_pipeline.opt_passes_arg,
        },
    });
    let cache_key =
        crate::content_cache::content_cache_key("nbuild-standard-mct-o3", &cache_material);
    if let Some(cache_root) = content_cache_root {
        let destinations = BTreeMap::from([
            ("mct_llvm_ir".to_string(), ll_path.clone()),
            ("object".to_string(), obj_path.to_path_buf()),
            (
                "machine_backend_object_ir".to_string(),
                backend_object_ir_path.to_path_buf(),
            ),
        ]);
        if let Ok(Some(hit)) = crate::content_cache::restore_content_cache_entry(
            cache_root,
            "nbuild-standard-mct-o3",
            &cache_key,
            &destinations,
        ) {
            let metadata = hit.metadata;
            let profile_label_cached = metadata
                .get("profile")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(profile_label)
                .to_string();
            let target_triple = metadata
                .get("target_triple")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(profile.target_spec.target_triple.as_str())
                .to_string();
            let entry_symbol = metadata
                .get("entry_symbol")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("ncc_mct_main")
                .to_string();
            let module = metadata
                .get("module")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let hosted_runtime_referenced = metadata
                .pointer("/runtime/hosted_referenced")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let json_runtime_referenced = metadata
                .pointer("/runtime/json_referenced")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let unresolved_symbols = metadata
                .get("unresolved_symbols")
                .and_then(serde_json::Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let mut clang_args = metadata
                .pointer("/clang/args")
                .and_then(serde_json::Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            for arg in &mut clang_args {
                if arg.ends_with(".standard.mct.ll") {
                    *arg = ll_path.display().to_string();
                } else if arg.ends_with(".obj") {
                    *arg = obj_path.display().to_string();
                }
            }
            return Ok(NbuildStandardMctBody {
                llvm_ir_path: ll_path,
                object_path: obj_path.to_path_buf(),
                profile_label: profile_label_cached,
                target_triple,
                entry_symbol,
                clang_tool,
                clang_args,
                module,
                stage: serde_json::json!({
                    "stage": "content-cache-restore-mct-o3",
                    "status": "passed",
                    "cache_hit": true,
                    "key": hit.key,
                    "artifacts": hit.artifact_count,
                    "bytes": hit.bytes,
                }),
                hosted_runtime_referenced,
                json_runtime_referenced,
                unresolved_symbols,
                incremental: serde_json::json!({
                    "schema": "ncc.content_cache.result.v1",
                    "mode": "content-addressed",
                    "cache_hit": true,
                    "namespace": "nbuild-standard-mct-o3",
                    "key": cache_key,
                    "restored_artifacts": hit.artifact_count,
                    "restored_bytes": hit.bytes,
                    "reused": ["optimized LLVM IR", "MCT O3 object", "direct backend object IR audit"],
                }),
            });
        }
    }
    let (direct_backend_object_ir, unresolved_symbols) =
        match emit_optimized_backend_object_ir_from_ast_with_profile(&ast, profile.clone()) {
            Ok(artifact) => (
                ncc_ir::render_object_text(&artifact.object),
                artifact.object.unresolved_symbols(),
            ),
            Err(err) => (
                format!(
                    "direct-backend-audit-unavailable\nsource={source_path}\ntarget={}\nerror={}\n",
                    profile.target_spec.target_triple,
                    err.to_string().replace('\n', " ")
                ),
                Vec::new(),
            ),
        };
    let direct_backend_object_ir = nbuild_backend_object_with_material_lineage(
        stem,
        source_path,
        basal_root,
        &direct_backend_object_ir,
    );
    fs::write(backend_object_ir_path, direct_backend_object_ir.as_bytes())?;
    require_non_empty_output_file(
        "nbuild retained direct backend object IR",
        backend_object_ir_path.to_string_lossy().as_ref(),
    )?;
    let mut ir = ncc_ir::lower_with_backend_profile_and_source_exact(
        ast,
        profile.clone(),
        source_path,
        source,
    );
    ir.target = profile.target.clone();
    ir.target_spec = profile.target_spec.clone();
    let target_attributes = nbuild_standard_mct_target_attributes(&profile, execution_profile);
    let mut module = ncc_ir::mct_emit_llvm_module_with_debug_config_and_target_attributes(
        &ir,
        pass_pipeline.clone(),
        super::mct_json::mct_llvm_debug_config_for_path(source_path),
        target_attributes,
    );
    if module.emitted_function_count == 0 || module.rejected_function_count > 0 {
        let rejection_summary = nbuild_mct_rejection_summary(&module);
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "standard MCT object route could not emit a complete high-level body for target '{}': emitted={} rejected={}; {}",
                profile.target_spec.target_triple,
                module.emitted_function_count,
                module.rejected_function_count,
                rejection_summary
            ),
        )
        .into());
    }
    let entry_symbol = "ncc_mct_main".to_string();
    module.llvm_ir = nbuild_rename_mct_main_wrapper(&module.llvm_ir, &entry_symbol)?;

    fs::write(&ll_path, module.llvm_ir.as_bytes())?;
    require_non_empty_output_file(
        "nbuild standard MCT LLVM",
        ll_path.to_string_lossy().as_ref(),
    )?;

    let mut clang_args = vec![
        format!("--target={}", profile.target_spec.target_triple),
        pass_pipeline.clang_opt_arg.to_string(),
        "-ffreestanding".to_string(),
        "-fno-stack-protector".to_string(),
        "-fno-exceptions".to_string(),
        "-fno-unwind-tables".to_string(),
        "-fno-asynchronous-unwind-tables".to_string(),
        "-fomit-frame-pointer".to_string(),
        "-ffunction-sections".to_string(),
        "-fdata-sections".to_string(),
        "-fno-pic".to_string(),
        "-fno-pie".to_string(),
        "-x".to_string(),
        "ir".to_string(),
        "-c".to_string(),
        ll_path.display().to_string(),
        "-o".to_string(),
        obj_path.display().to_string(),
    ];
    match profile.target_spec.isa {
        ncc_ir::BackendIsaIR::RiscV32 => {
            clang_args.insert(1, "-march=rv32imac".to_string());
            clang_args.insert(2, "-mcmodel=medlow".to_string());
        }
        ncc_ir::BackendIsaIR::RiscV64 => {
            clang_args.insert(1, "-march=rv64imac".to_string());
            clang_args.insert(2, "-mcmodel=medlow".to_string());
        }
        _ => {}
    }
    let output = std::process::Command::new(&clang_tool)
        .args(&clang_args)
        .output();
    let stage = match output {
        Ok(output) => serde_json::json!({
            "stage": "clang-standard-mct-object",
            "tool": clang_tool,
            "args": clang_args,
            "status": if output.status.success() { "passed" } else { "failed" },
            "status_code": output.status.code(),
            "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
            "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
        }),
        Err(err) => serde_json::json!({
            "stage": "clang-standard-mct-object",
            "tool": clang_tool,
            "args": clang_args,
            "status": "missing_tool",
            "error": err.to_string(),
        }),
    };
    if stage.get("status").and_then(serde_json::Value::as_str) != Some("passed") {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "standard MCT object route failed while compiling LLVM IR to object: {}",
                serde_json::to_string_pretty(&stage)?
            ),
        )
        .into());
    }
    require_non_empty_output_file(
        "nbuild standard MCT object",
        obj_path.to_string_lossy().as_ref(),
    )?;

    let hosted_runtime_referenced =
        super::compiler::compiler_native_uses_hosted_runtime(&module.llvm_ir);
    let json_runtime_referenced =
        super::compiler::compiler_native_uses_json_runtime(&module.llvm_ir);
    let module_json = super::mct_json::mct_llvm_module_to_json(&module, false, Some(&ll_path));
    let mut body = NbuildStandardMctBody {
        llvm_ir_path: ll_path,
        object_path: obj_path.to_path_buf(),
        profile_label: profile_label.to_string(),
        target_triple: profile.target_spec.target_triple,
        entry_symbol,
        clang_tool,
        clang_args,
        module: module_json,
        stage,
        hosted_runtime_referenced,
        json_runtime_referenced,
        unresolved_symbols,
        incremental: serde_json::json!({
            "schema": "ncc.content_cache.result.v1",
            "mode": "content-addressed",
            "cache_hit": false,
            "namespace": "nbuild-standard-mct-o3",
            "key": cache_key,
            "stored": false,
        }),
    };
    if let Some(cache_root) = content_cache_root {
        let cache_artifacts = [
            crate::content_cache::ContentCacheArtifact::new("mct_llvm_ir", &body.llvm_ir_path),
            crate::content_cache::ContentCacheArtifact::new("object", &body.object_path),
            crate::content_cache::ContentCacheArtifact::new(
                "machine_backend_object_ir",
                backend_object_ir_path,
            ),
        ];
        match crate::content_cache::store_content_cache_entry(
            cache_root,
            "nbuild-standard-mct-o3",
            &cache_key,
            &cache_material,
            body.to_json(),
            &cache_artifacts,
        ) {
            Ok(()) => body.incremental["stored"] = serde_json::json!(true),
            Err(err) => body.incremental["store_error"] = serde_json::json!(err.to_string()),
        }
    }
    Ok(body)
}

pub(crate) fn nbuild_backend_object_with_machine_lineage(
    stem: &str,
    source_path: &str,
    body: &str,
) -> String {
    format!(
        "; nc.lowering.root-machine-nbo.v1\n; basal-root={stem}\n; source={}\n; machine-source={stem}.nm\n; address-origin=basal.root\n; address-backing=machine\n\n{body}",
        source_path.replace('\\', "/")
    )
}

#[derive(Debug, Clone)]
struct NbuildBasalRootArtifacts {
    source_path: PathBuf,
    binary_path: PathBuf,
    source_sha256: String,
    binary_sha256: String,
    source_input_sha256: String,
    report: serde_json::Value,
}

fn nbuild_materialize_basal_root(
    source_path: &str,
    artifact_root: &Path,
    pointer_width: usize,
) -> Result<NbuildBasalRootArtifacts, Box<dyn std::error::Error>> {
    let stem = nbuild_source_stem(source_path);
    let container = nbuild_machine_ident(&stem);
    let source_input_sha256 = sha256_hex_file(Path::new(source_path))?;
    let pointer_bytes = usize::from(pointer_width.max(8) / 8);
    let root_dir = artifact_root.join("root");
    fs::create_dir_all(&root_dir)?;
    let source = root_dir.join(format!("{container}.nb"));
    let binary = root_dir.join(format!("{container}.nbb"));
    let material = format!(
        r#"{container} :: root {{
    root:
        identity: "nC.root.{container}.{source_input_sha256}";
        version: 2;
        profile: basal;
        authority: sealed;
        binary: optional;
        evidence: optional;
        lowering: compiler;

    site:
        sourceEntry:
            kind: memory_anchor;
            size: {pointer_bytes};
            align: {pointer_bytes};
            permissions: read | write;

    bind:
        symbol source_binding -> root.site sourceEntry;

    routine:
        materialize() -> bool {{
            return true;
        }}

    verify:
        materialize();

    report:
        status "{container}::source-root-materialized";
        emit sourceEntry;

    seal:
        require source_hash;
}}
"#
    );
    fs::write(&source, material.as_bytes())?;
    let report = super::basal::emit_verified_nbb_artifact(&source, &binary)?;
    require_non_empty_output_file(
        "nbuild Basal root source",
        source.to_string_lossy().as_ref(),
    )?;
    require_non_empty_output_file(
        "nbuild Basal root binary",
        binary.to_string_lossy().as_ref(),
    )?;

    Ok(NbuildBasalRootArtifacts {
        source_sha256: sha256_hex_file(&source)?,
        binary_sha256: sha256_hex_file(&binary)?,
        source_input_sha256,
        source_path: source,
        binary_path: binary,
        report,
    })
}

fn nbuild_basal_root_lineage_payload(root: &NbuildBasalRootArtifacts) -> serde_json::Value {
    serde_json::json!({
        "source": {
            "path": root.source_path.display().to_string(),
            "sha256": root.source_sha256,
            "format": "nb",
            "container_matches_file_stem": true,
        },
        "binary": {
            "path": root.binary_path.display().to_string(),
            "sha256": root.binary_sha256,
            "format": "NBB0",
            "verification": root.report,
        },
        "input_source_sha256": root.source_input_sha256,
        "address_origin": "basal.root",
        "address_backing": "machine",
        "mutation_contract": "edit .n and rebuild .nb/.nbb/.nm/.nbo/object/executable as one content-addressed unit",
    })
}

fn nbuild_backend_object_with_material_lineage(
    stem: &str,
    source_path: &str,
    root: &NbuildBasalRootArtifacts,
    body: &str,
) -> String {
    format!(
        "; nc.lowering.source-root-machine-nbo.v2\n; basal-root={stem}\n; source={}\n; source-sha256={}\n; basal-source={}\n; basal-source-sha256={}\n; basal-binary={}\n; basal-binary-sha256={}\n; machine-source={stem}.nm\n; address-origin=basal.root\n; address-backing=machine\n\n{body}",
        source_path.replace('\\', "/"),
        root.source_input_sha256,
        root.source_path.to_string_lossy().replace('\\', "/"),
        root.source_sha256,
        root.binary_path.to_string_lossy().replace('\\', "/"),
        root.binary_sha256,
    )
}

#[allow(clippy::too_many_arguments)]
fn nbuild_compile_standard_runtime_object(
    referenced: bool,
    runtime_label: &str,
    stage_label: &str,
    source_text: &str,
    object_path: Option<&PathBuf>,
    clang_tool: &str,
    target_triple: &str,
    disable_builtins: bool,
    content_cache_root: Option<&Path>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if !referenced {
        return Ok(serde_json::json!({
            "stage": stage_label,
            "status": "skipped",
            "reason": format!("optimized LLVM module does not reference the {runtime_label} ABI")
        }));
    }
    let runtime_object = object_path.ok_or_else(|| {
        actionable_command_error(
            "nbuild",
            format!("{runtime_label} object path was not reserved"),
        )
    })?;
    let cache_material = serde_json::json!({
        "schema": "ncc.nbuild.runtime_object_content_key.v1",
        "runtime": runtime_label,
        "source_sha256": crate::content_cache::sha256_bytes(source_text.as_bytes()),
        "target": target_triple,
        "disable_builtins": disable_builtins,
        "compiler": crate::content_cache::compiler_image_identity(),
        "clang": crate::content_cache::tool_identity(Some(clang_tool)),
    });
    let cache_key =
        crate::content_cache::content_cache_key("nbuild-runtime-object", &cache_material);
    if let Some(cache_root) = content_cache_root {
        let destinations =
            BTreeMap::from([("runtime_object".to_string(), runtime_object.to_path_buf())]);
        if let Ok(Some(hit)) = crate::content_cache::restore_content_cache_entry(
            cache_root,
            "nbuild-runtime-object",
            &cache_key,
            &destinations,
        ) {
            return Ok(serde_json::json!({
                "stage": stage_label,
                "status": "passed",
                "route": "content-cache-restore",
                "cache_hit": true,
                "key": hit.key,
                "artifacts": hit.artifact_count,
                "bytes": hit.bytes,
                "tool": clang_tool,
                "target": target_triple,
            }));
        }
    }
    let runtime_args = super::compiler::compiler_native_embedded_c_object_command_args(
        target_triple,
        runtime_object,
        disable_builtins,
    );
    let output = super::compiler::compiler_native_compile_embedded_c_object(
        clang_tool,
        &runtime_args,
        source_text,
    );
    let stage = match output {
        Ok(output) => serde_json::json!({
            "stage": stage_label,
            "tool": clang_tool,
            "args": runtime_args,
            "status": if output.status.success() { "passed" } else { "failed" },
            "status_code": output.status.code(),
            "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
            "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
        }),
        Err(err) => serde_json::json!({
            "stage": stage_label,
            "tool": clang_tool,
            "args": runtime_args,
            "status": "missing_tool",
            "error": err.to_string(),
        }),
    };
    if stage.get("status").and_then(serde_json::Value::as_str) != Some("passed") {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "native {runtime_label} object materialization failed: {}",
                serde_json::to_string_pretty(&stage)?
            ),
        )
        .into());
    }
    require_non_empty_output_file(
        &format!("nbuild {runtime_label} object"),
        runtime_object.to_string_lossy().as_ref(),
    )?;
    if let Some(cache_root) = content_cache_root {
        let artifacts = [crate::content_cache::ContentCacheArtifact::new(
            "runtime_object",
            runtime_object,
        )];
        let _ = crate::content_cache::store_content_cache_entry(
            cache_root,
            "nbuild-runtime-object",
            &cache_key,
            &cache_material,
            stage.clone(),
            &artifacts,
        );
    }
    Ok(stage)
}

#[allow(clippy::too_many_arguments)]
fn run_nbuild_standard_mct_machine_command(
    path: &str,
    out_path: &str,
    artifact_root: &Path,
    ast: ncc_ast::Program,
    profile: ncc_ir::BackendLoweringProfileIR,
    execution_profile: NccExecutionProfile,
    opt_level: &str,
    linker: Option<&str>,
    entry: Option<&str>,
    json: bool,
    project_linkage: Option<serde_json::Value>,
    source_execution: &NbuildSourceExecutionArtifact,
    basal_root: &NbuildBasalRootArtifacts,
    content_cache_root: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    if entry.is_some() {
        return Err(actionable_command_error(
            "nbuild",
            "standard MCT-machine route uses the source ref main/to main wrapper as executable entry; explicit --entry is not admitted for this target yet",
        )
        .into());
    }

    let mut ast = ast;
    let _text_runtime_embedded = super::compiler::compiler_native_attach_nc_text_runtime(
        &mut ast,
        &profile.target_spec.target_triple,
    )?;
    super::compiler::compiler_native_attach_nc_parse_runtime(&mut ast)?;
    super::compiler::compiler_native_attach_nc_unicode_runtime(
        &mut ast,
        &profile.target_spec.target_triple,
    )?;
    sema_check_with_warnings(&ast)?;

    let source = fs::read_to_string(path)?;
    let stem = nbuild_source_stem(path);
    let code_dir = artifact_root.join("code");
    let lowering_dir = artifact_root.join("lowering");
    let obj_dir = artifact_root.join("obj");
    let run_dir = artifact_root.join("run");
    let bin_dir = artifact_root.join("bin");
    for dir in [&code_dir, &lowering_dir, &obj_dir, &run_dir, &bin_dir] {
        fs::create_dir_all(dir)?;
    }
    super::compiler::compiler_native_remove_legacy_runtime_sources(&lowering_dir)?;

    let nm_path = lowering_dir.join(format!("{stem}.nm"));
    let optimizer_report_path = lowering_dir.join(format!("{stem}.optimizer.json"));
    let backend_object_ir_path = lowering_dir.join(format!("{stem}.backend-object.nbo"));
    let obj_path = obj_dir.join(format!("{stem}.obj"));
    let manifest_path = run_dir.join(format!("{stem}.native-build.json"));
    let route_started = Instant::now();
    let effective_opt_level =
        nbuild_effective_optimization_level_label(opt_level, execution_profile)?;

    let profile_label = format!(
        "nC.standard.mct-machine.{}.{}",
        nbuild_machine_arch(&profile.target_spec),
        nbuild_machine_object(&profile.target_spec)
    );
    let optimize_started = Instant::now();
    let mct_body = nbuild_compile_standard_mct_body_object(
        ast,
        path,
        &source,
        profile.clone(),
        &profile_label,
        opt_level,
        execution_profile,
        &stem,
        &lowering_dir,
        &obj_path,
        &backend_object_ir_path,
        basal_root,
        content_cache_root,
    )?;
    let optimize_and_object_ms = nbuild_elapsed_ms(optimize_started);
    let mct_payload = mct_body.to_json();
    let report_started = Instant::now();
    fs::write(
        &optimizer_report_path,
        serde_json::to_string_pretty(&mct_payload)?.as_bytes(),
    )?;
    require_non_empty_output_file(
        "nbuild standard MCT report",
        optimizer_report_path.to_string_lossy().as_ref(),
    )?;
    require_non_empty_output_file(
        "nbuild retained direct backend object IR",
        backend_object_ir_path.to_string_lossy().as_ref(),
    )?;
    let report_ms = nbuild_elapsed_ms(report_started);

    let json_runtime_obj_path = mct_body
        .json_runtime_referenced
        .then(|| obj_dir.join(format!("{stem}.runtime-json.obj")));
    let hosted_runtime_obj_path = mct_body
        .hosted_runtime_referenced
        .then(|| obj_dir.join(format!("{stem}.runtime-hosted.obj")));
    let json_runtime_stage = nbuild_compile_standard_runtime_object(
        mct_body.json_runtime_referenced,
        "JSON runtime",
        "clang-standard-json-runtime-object",
        super::compiler::COMPILER_NATIVE_JSON_RUNTIME_SOURCE,
        json_runtime_obj_path.as_ref(),
        &mct_body.clang_tool,
        profile.target_spec.target_triple.as_str(),
        false,
        content_cache_root,
    )?;
    let hosted_runtime_stage = nbuild_compile_standard_runtime_object(
        mct_body.hosted_runtime_referenced,
        "hosted runtime",
        "clang-standard-hosted-runtime-object",
        super::compiler::COMPILER_NATIVE_HOSTED_RUNTIME_SOURCE,
        hosted_runtime_obj_path.as_ref(),
        &mct_body.clang_tool,
        profile.target_spec.target_triple.as_str(),
        true,
        content_cache_root,
    )?;

    let bridge_started = Instant::now();
    let bridge_object = ncc_ir::BackendObjectIR {
        target: profile.target.clone(),
        target_spec: profile.target_spec.clone(),
        sections: Vec::new(),
        symbols: Vec::new(),
        relocations: Vec::new(),
    };
    let nm_source = nbuild_hosted_machine_entry_source(
        path,
        &stem,
        &bridge_object,
        &mct_body.entry_symbol,
        false,
        &obj_path,
        Path::new(out_path),
        Some(&backend_object_ir_path),
        None,
        true,
    );
    fs::write(&nm_path, nm_source.as_bytes())?;
    require_non_empty_output_file("nbuild machine source", nm_path.to_string_lossy().as_ref())?;
    super::machine::check_machine_source_path(nm_path.to_string_lossy().as_ref())?;

    ensure_output_parent_dir(out_path)?;
    let bridge_obj_path = obj_dir.join(format!("{stem}.entry.obj"));
    super::machine::emit_machine_source_object_to_path(
        nm_path.to_string_lossy().as_ref(),
        profile.target_spec.target_triple.as_str(),
        bridge_obj_path.to_string_lossy().as_ref(),
    )?;
    require_non_empty_output_file(
        "nbuild machine entry object",
        bridge_obj_path.to_string_lossy().as_ref(),
    )?;
    let target_init = nbuild_materialize_target_init(
        &stem,
        artifact_root,
        profile.target_spec.target_triple.as_str(),
        nbuild_machine_arch(&profile.target_spec),
        nbuild_machine_abi(&profile.target_spec),
        nbuild_machine_object(&profile.target_spec),
        profile.target_spec.pointer_width,
        &mct_body.entry_symbol,
        &nm_path,
        &bridge_obj_path,
        "machine-entry-object",
    )?;
    let bridge_ms = nbuild_elapsed_ms(bridge_started);

    let link_started = Instant::now();
    let internal_os_link_format =
        nbuild_internal_os_link_format(&bridge_object, None).ok_or_else(|| {
            actionable_command_error(
                "nbuild",
                format!(
                    "target '{}' is not admitted by the internal OS executable linker",
                    profile.target_spec.target_triple
                ),
            )
        })?;
    let destructive_null_link = matches!(
        profile.destructive_policy.missing_symbol_policy,
        ncc_ir::BackendMissingSymbolPolicyIR::MaterializeNullAddress
    );
    let configured_linker = linker.is_some() && !destructive_null_link;
    let map_path = (!configured_linker).then(|| {
        run_dir.join(format!(
            "{stem}.{}.map",
            nbuild_internal_os_link_map_suffix(internal_os_link_format)
        ))
    });
    let mut inputs = vec![
        bridge_obj_path.to_string_lossy().to_string(),
        obj_path.to_string_lossy().to_string(),
    ];
    if let Some(runtime_object) = json_runtime_obj_path.as_ref() {
        inputs.push(runtime_object.to_string_lossy().to_string());
    }
    if let Some(runtime_object) = hosted_runtime_obj_path.as_ref() {
        inputs.push(runtime_object.to_string_lossy().to_string());
    }
    let native_link_report = if let Some(linker_bin) = linker.filter(|_| configured_linker) {
        let input_paths = inputs.iter().map(Path::new).collect::<Vec<_>>();
        crate::link_hosted_machine_object_paths_with_linker(
            &bridge_object,
            out_path,
            Some(linker_bin),
            &input_paths,
        )?;
        serde_json::json!({
            "schema": "nc.compiler.configured_linker.v1",
            "status": "passed",
            "linker": linker_bin,
            "target": profile.target_spec.target_triple,
            "inputs": inputs,
            "output": out_path,
            "unresolved_symbols": [],
            "dynamic_imports": null,
        })
    } else {
        super::machine::link_machine_object_paths_to_os_executable_with_missing_symbol_policy_silent(
            &inputs,
            profile.target_spec.target_triple.as_str(),
            out_path,
            map_path
                .as_ref()
                .map(|path| path.to_string_lossy())
                .as_deref(),
            profile.destructive_policy.missing_symbol_policy,
            &mct_body.unresolved_symbols,
        )?
    };
    let runtime_symbols = native_link_report
        .pointer("/dynamic_imports/imports")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            entry
                .get("symbol")
                .and_then(serde_json::Value::as_str)
                .or_else(|| entry.as_str())
        })
        .map(str::to_string)
        .collect::<Vec<_>>();
    require_non_empty_output_file("nbuild executable", out_path)?;
    release_native_executable_for_user(Path::new(out_path))?;
    let link_ms = nbuild_elapsed_ms(link_started);
    let launcher_started = Instant::now();
    let launcher_artifacts = write_native_launcher_artifacts(
        Path::new(path),
        &stem,
        Path::new(out_path),
        &code_dir,
        &bin_dir,
    )?;
    let launcher_ms = nbuild_elapsed_ms(launcher_started);

    let mut artifacts = vec![
        nbuild_artifact_record(&basal_root.source_path, "basal_root_source")?,
        nbuild_artifact_record(&basal_root.binary_path, "basal_root_binary")?,
        nbuild_artifact_record(&nm_path, "machine_source")?,
        nbuild_artifact_record(&mct_body.llvm_ir_path, "mct_llvm_ir")?,
        nbuild_artifact_record(&optimizer_report_path, "optimized_ir_report")?,
        nbuild_artifact_record(&backend_object_ir_path, "machine_backend_object_ir")?,
        nbuild_artifact_record(&launcher_artifacts.launcher_n, "launcher_n")?,
        nbuild_artifact_record(&obj_path, "object")?,
        nbuild_artifact_record(&bridge_obj_path, "machine_entry_object")?,
        nbuild_artifact_record(Path::new(out_path), "executable")?,
        nbuild_artifact_record(&launcher_artifacts.powershell, "powershell_launcher")?,
        nbuild_artifact_record(&launcher_artifacts.batch, "batch_launcher")?,
    ];
    if let Some(path) = map_path.as_ref() {
        artifacts.push(nbuild_artifact_record(path, "link_map")?);
    }
    if let Some(path) = json_runtime_obj_path.as_ref() {
        artifacts.push(nbuild_artifact_record(path, "runtime_json_native_object")?);
    }
    if let Some(path) = hosted_runtime_obj_path.as_ref() {
        artifacts.push(nbuild_artifact_record(
            path,
            "runtime_hosted_native_object",
        )?);
    }

    let mut pipeline = vec![
        "parse .n",
        "sema",
        "lower high-level IR with target ABI/data-layout",
        "emit optimized MCT LLVM for requested target",
        "materialize target object bytes with clang",
        "materialize referenced JSON runtime ABI object",
        "materialize referenced hosted runtime ABI object",
        "retain .nm machine bridge",
        "check retained .nm through machine",
        "emit .nm bridge object",
    ];
    pipeline.push(if configured_linker {
        "link native executable with configured linker"
    } else {
        "link native OS executable internally"
    });
    let release_fast_auto = nbuild_release_fast_auto_payload(
        "standard-mct-machine",
        execution_profile,
        opt_level,
        effective_opt_level,
        serde_json::json!({
            "optimize_and_object": optimize_and_object_ms,
            "retain_optimizer_and_object_ir": report_ms,
            "machine_bridge": bridge_ms,
            "internal_os_link": link_ms,
            "launcher_generation": launcher_ms,
            "total_backend_route": nbuild_elapsed_ms(route_started),
        }),
        serde_json::json!({
            "llvm_ir_bytes": fs::metadata(&mct_body.llvm_ir_path)?.len(),
            "optimizer_report_bytes": fs::metadata(&optimizer_report_path)?.len(),
            "backend_object_ir_bytes": fs::metadata(&backend_object_ir_path)?.len(),
            "program_object_bytes": fs::metadata(&obj_path)?.len(),
            "machine_entry_object_bytes": fs::metadata(&bridge_obj_path)?.len(),
            "json_runtime_object_bytes": json_runtime_obj_path
                .as_ref()
                .and_then(|path| fs::metadata(path).ok())
                .map(|metadata| metadata.len()),
            "hosted_runtime_object_bytes": hosted_runtime_obj_path
                .as_ref()
                .and_then(|path| fs::metadata(path).ok())
                .map(|metadata| metadata.len()),
            "executable_bytes": fs::metadata(out_path)?.len(),
            "link_map_bytes": map_path
                .as_ref()
                .and_then(|path| fs::metadata(path).ok())
                .map(|metadata| metadata.len()),
        }),
        &mct_payload,
    );
    let mut payload = serde_json::json!({
        "schema": "nc.compiler.standard_native_build.v1",
        "status": "native_build_passed",
        "source": path,
        "source_sha256": sha256_hex_file(Path::new(path))?,
        "artifact_root": artifact_root.display().to_string(),
        "project_linkage": project_linkage,
        "profile": execution_profile.label(),
        "optimization": {
            "requested": nbuild_requested_optimization_label(opt_level),
            "effective": effective_opt_level,
            "route": "standard-mct-machine"
        },
        "target": {
            "target": nbuild_machine_arch(&profile.target_spec),
            "triple": profile.target_spec.target_triple.clone(),
            "object_format": render_backend_object_format_label(&profile.target_spec.object_format),
            "arch": nbuild_machine_arch(&profile.target_spec),
            "abi": nbuild_machine_abi(&profile.target_spec),
            "pointer_width": profile.target_spec.pointer_width,
        },
        "entry": mct_body.entry_symbol,
        "paths": {
            "basal_root_source": basal_root.source_path.display().to_string(),
            "basal_root_binary": basal_root.binary_path.display().to_string(),
            "machine_source": nm_path.display().to_string(),
            "mct_llvm_ir": mct_body.llvm_ir_path.display().to_string(),
            "optimized_ir_report": optimizer_report_path.display().to_string(),
            "machine_backend_object_ir": backend_object_ir_path.display().to_string(),
            "launcher_n": launcher_artifacts.launcher_n.display().to_string(),
            "machine_entry_object": bridge_obj_path.display().to_string(),
            "object": obj_path.display().to_string(),
            "executable": out_path,
            "powershell_launcher": launcher_artifacts.powershell.display().to_string(),
            "batch_launcher": launcher_artifacts.batch.display().to_string(),
            "manifest": manifest_path.display().to_string(),
            "runtime_json_source": serde_json::Value::Null,
            "runtime_json_object": json_runtime_obj_path.as_ref().map(|path| path.display().to_string()),
            "runtime_hosted_source": serde_json::Value::Null,
            "runtime_hosted_object": hosted_runtime_obj_path.as_ref().map(|path| path.display().to_string()),
        },
        "lowering_lineage": {
            "schema": "nc.lowering.source-root-machine-nbo.v2",
            "status": "materialized",
            "basal": nbuild_basal_root_lineage_payload(basal_root),
            "machine": {
                "path": nm_path.display().to_string(),
                "sha256": sha256_hex_file(&nm_path)?,
                "container_matches_file_stem": true
            },
            "backend_object": {
                "path": backend_object_ir_path.display().to_string(),
                "sha256": sha256_hex_file(&backend_object_ir_path)?,
                "format": "nbo",
                "source_machine_path_embedded": true,
                "source_machine_sha256_embedded": false,
                "integrity_link": ".nm embeds the final .nbo SHA-256; the manifest records both hashes without a circular digest"
            },
            "order": ["source.n", "root.nb", "root.nbb", "machine.nm", "backend-object.nbo", "native-object", "executable"]
        },
        "pipeline": pipeline,
        "artifacts": artifacts,
        "speed_contract": {
            "normal_build_surface": true,
            "release_fast_automatic": execution_profile == NccExecutionProfile::ReleaseFast,
            "effective_optimization": effective_opt_level,
            "special_optimize_command_required": false,
            "measurable_release_fast": true,
            "interpreter_fallback": false,
            "native_machine_execution": true,
            "internal_os_linker": !configured_linker,
            "c_like_artifact_set": [".nb", ".nbb", ".nm", ".obj", ".exe"],
            "complete_artifact_set": [".nb", ".nbb", ".optimizer.json", ".backend-object.nbo", ".standard.mct.ll", ".nm", ".obj", ".entry.obj", ".exe"]
        },
        "optimizer": mct_payload,
        "incremental": mct_body.incremental.clone(),
        "runtime_json": {
            "referenced": mct_body.json_runtime_referenced,
            "source": serde_json::Value::Null,
            "source_materialization": "compiler-embedded-stdin",
            "object": json_runtime_obj_path.as_ref().map(|path| path.display().to_string()),
            "object_sha256": json_runtime_obj_path
                .as_ref()
                .map(|path| sha256_hex_file(path))
                .transpose()?,
            "stage": json_runtime_stage,
            "abi": "nC.runtime.json.v1",
            "implementation": "real target-native PE/ELF/Mach-O object linked as a normal Phase12 input"
        },
        "runtime_hosted": {
            "referenced": mct_body.hosted_runtime_referenced,
            "source": serde_json::Value::Null,
            "source_materialization": "compiler-embedded-stdin",
            "object": hosted_runtime_obj_path.as_ref().map(|path| path.display().to_string()),
            "object_sha256": hosted_runtime_obj_path
                .as_ref()
                .map(|path| sha256_hex_file(path))
                .transpose()?,
            "stage": hosted_runtime_stage,
            "abi": "nC.runtime.hosted.v1",
            "implementation": "compiler-owned provider streamed directly into a target object and linked as a normal Phase12 input; no C source is materialized under .ncc"
        },
        "release_fast_auto": release_fast_auto,
        "machine_backend": {
            "object_ir_retained": true,
            "object_ir_path": backend_object_ir_path.display().to_string(),
            "program_object_bytes": fs::metadata(&obj_path)?.len(),
            "mct_body_object": true,
            "machine_bridge_object": true,
            "abi": {
                "model": nbuild_machine_abi(&profile.target_spec),
                "pointer_width": profile.target_spec.pointer_width,
                "stack_align_bytes": profile.target_spec.data_layout.stack_align_bytes,
                "aggregate_align_bytes": profile.target_spec.data_layout.aggregate_align_bytes,
                "calling_convention_materialized_in_object_ir": true,
                "aggregate_return_contract_materialized": true,
                "direct_machine_call_lowering": true
            },
            "runtime_import_materialization": {
                "schema": "nc.compiler.runtime_import_materialization.v1",
                "status": if configured_linker { "delegated_to_configured_linker" } else { "closed_by_internal_machine_linker" },
                "source_of_truth": "Phase12 native object import plus machine bridge",
                "native_call_admission": nbuild_runtime_native_call_admission_payload(),
                "runtime_symbols": runtime_symbols,
                "direct_machine_materialized": !configured_linker,
                "internal_os_link_format": if configured_linker { None } else { Some(internal_os_link_format) },
                "dynamic_imports": native_link_report.get("dynamic_imports").cloned().unwrap_or(serde_json::Value::Null)
            }
        },
        "link": {
            "route": if configured_linker { "configured-host-linker" } else { nbuild_internal_os_link_route(internal_os_link_format) },
            "native_pe64_internal": !configured_linker && internal_os_link_format == "coff",
            "native_macho_internal": !configured_linker && internal_os_link_format == "macho",
            "internal_os_linker": !configured_linker,
            "internal_os_link_format": if configured_linker { None } else { Some(internal_os_link_format) },
            "configured_linker": if configured_linker { linker } else { None },
            "configured_linker_requested": linker,
            "configured_linker_ignored_by_destructive_policy": linker.is_some() && destructive_null_link,
            "phase12_report": native_link_report,
        },
        "truth_boundary": {
            "object_is_real_backend_output": true,
            "object_is_optimized_backend_output": true,
            "object_body_route": "MCT_LLVM_to_target_object",
            "optimized_ir_report_retained": true,
            "machine_backend_object_ir_retained": true,
            "machine_backend_object_ir_is_direct_lowering_audit": true,
            "executable_is_configured_linker_output": configured_linker,
            "executable_is_native_pe_direct": !configured_linker && internal_os_link_format == "coff",
            "executable_is_internal_os_loader_direct": !configured_linker,
            "runtime_imports_are_machine_linker_visible": !configured_linker,
            "runtime_json_native_object_is_real_target_object": mct_body.json_runtime_referenced
                && json_runtime_obj_path.as_ref().is_some_and(|path| path.is_file()),
            "runtime_hosted_native_object_is_real_target_object": mct_body.hosted_runtime_referenced
                && hosted_runtime_obj_path.as_ref().is_some_and(|path| path.is_file()),
            "abi_contract_is_backend_object_visible": true,
            "nm_is_retained_machine_bridge_not_full_instruction_for_instruction_high_level_lowering": true,
            "launcher_n_generated": true,
            "host_shell_launchers_generated": [".ps1", ".bat"],
            "executable_released_for_external_run": true,
            "missing_tools_are_not_faked": true,
        }
    });
    nbuild_attach_target_init_payload(&mut payload, &target_init)?;
    nbuild_attach_source_execution_payload(&mut payload, source_execution)?;
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&payload)?.as_bytes(),
    )?;

    if !crate::ncc_child_output_quiet() {
        if json {
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!(
                "OK: nbuild exe={} obj={} nm={} manifest={} profile={} route=standard-mct-machine",
                out_path,
                obj_path.display(),
                nm_path.display(),
                manifest_path.display(),
                execution_profile.label()
            );
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct NbuildKernelTargetSpec {
    triple: &'static str,
    arch: &'static str,
    abi: &'static str,
    object: &'static str,
    pointer_width: u32,
    stack_align: u32,
    output_ext: &'static str,
    system_loader_executable: bool,
}

impl NbuildKernelTargetSpec {
    fn entry_symbol(self) -> &'static str {
        "kmain"
    }

    fn entry_abi(self) -> &'static str {
        match self.arch {
            "x86_64" if self.abi == "win64" => "ncc-kernel-entry-win64-v1",
            "x86_64" => "ncc-kernel-entry-sysv64-v1",
            "aarch64" => "ncc-kernel-entry-aapcs64-v1",
            "armv7" | "armv7em" => "ncc-kernel-entry-aapcs32-v1",
            "riscv64" => "ncc-kernel-entry-riscv64-v1",
            "riscv32" => "ncc-kernel-entry-riscv32-ilp32-v1",
            "i386" => "ncc-kernel-entry-i386-cdecl-v1",
            "wasm32" => "ncc-kernel-entry-wasm32-host-v1",
            _ => "ncc-kernel-entry-v1",
        }
    }

    fn boot_info_argument(self) -> &'static str {
        match self.arch {
            "x86_64" if self.abi == "win64" => "rcx",
            "x86_64" => "rdi",
            "aarch64" => "x0",
            "armv7" | "armv7em" => "r0",
            "riscv64" | "riscv32" => "a0",
            "i386" => "stack[0]",
            "wasm32" => "param0",
            _ => "arg0",
        }
    }

    fn stack_pointer_register(self) -> &'static str {
        match self.arch {
            "x86_64" => "rsp",
            "aarch64" | "armv7" | "armv7em" => "sp",
            "riscv64" | "riscv32" => "sp",
            "i386" => "esp",
            "wasm32" => "value-stack",
            _ => "sp",
        }
    }

    fn loader_protocol(self) -> &'static str {
        match self.triple {
            "x86_64-unknown-none" => "ncc-boot-v1 with multiboot2/limine/uefi adapter",
            "aarch64-unknown-none" => "ncc-boot-v1 with uefi/fdt adapter",
            "riscv64-unknown-elf" => "ncc-boot-v1 with opensbi/fdt adapter",
            "riscv32imac-unknown-elf" => "ncc-boot-v1 with opensbi/fdt adapter",
            "i386-unknown-none-elf" => "ncc-boot-v1 with multiboot/e820 adapter",
            "armv7a-unknown-none-eabi" => "ncc-boot-v1 with ARMv7-A firmware/FDT adapter",
            "armv7em-none-eabi" => "Cortex-M vector table with reset handler",
            "wasm32-unknown-unknown" => "ncc-wasm-kernel-host-v1",
            "x86_64-unknown-linux-gnu" => "linux-elf-system-loader-smoke",
            "x86_64-pc-windows-msvc" => "windows-pe-system-loader-smoke",
            "x86_64-apple-darwin" => "darwin-macho-system-loader-smoke",
            "aarch64-unknown-linux-gnu" => "linux-elf-system-loader-smoke",
            "i386-unknown-linux-gnu" => "linux-elf32-system-loader-contract",
            _ => "ncc-boot-v1",
        }
    }

    fn platform_tables(self) -> Vec<&'static str> {
        match self.triple {
            "x86_64-unknown-none" => vec![
                "acpi_rsdp",
                "smbios32",
                "smbios64",
                "smbios_tables",
                "uefi_system_table",
                "uefi_memory_map",
                "uefi_image_handle",
                "uefi_boot_services",
                "multiboot2_tags",
                "multiboot2_boot_device",
                "multiboot2_load_base",
                "multiboot2_vbe",
                "multiboot2_elf_sections",
                "multiboot2_apm",
                "multiboot2_network",
                "multiboot2_framebuffer",
            ],
            "aarch64-unknown-none" => vec![
                "acpi_rsdp",
                "smbios32",
                "smbios64",
                "uefi_system_table",
                "uefi_memory_map",
                "uefi_image_handle",
                "uefi_boot_services",
                "fdt",
            ],
            "riscv64-unknown-elf" | "riscv32imac-unknown-elf" => vec!["fdt", "opensbi_hart"],
            "i386-unknown-none-elf" => vec!["acpi_rsdp", "bios_e820", "multiboot_tags"],
            "wasm32-unknown-unknown" => vec!["wasm_host_imports"],
            _ => vec!["system_loader_process_contract"],
        }
    }

    fn memory_map_contract(self) -> &'static str {
        "ncc_kernel_memory_region_v1 array: base, length, kind, flags"
    }

    fn framebuffer_contract(self) -> &'static str {
        "optional linear framebuffer: base, width, height, pitch, format"
    }

    fn serial_contract(self) -> &'static str {
        match self.arch {
            "x86_64" | "i386" => "optional serial: COM1 0x3f8 or loader-provided MMIO",
            "aarch64" => "optional serial: loader-provided UART MMIO",
            "riscv64" | "riscv32" => "optional serial: SBI console or loader-provided UART MMIO",
            "wasm32" => "optional host console import",
            _ => "optional serial sink",
        }
    }

    fn entry_contract_json(self) -> serde_json::Value {
        serde_json::json!({
            "schema": "nc.kernel.entry.v1",
            "symbol": self.entry_symbol(),
            "abi": self.entry_abi(),
            "calling_convention": self.abi,
            "return": if self.system_loader_executable { "int-status" } else { "never-or-int-status" },
            "boot_info_argument": self.boot_info_argument(),
            "stack": {
                "pointer_register": self.stack_pointer_register(),
                "alignment": self.stack_align,
                "owner": "loader",
                "initial_state": "valid-before-kmain",
                "direction": if self.arch == "wasm32" { "host-defined" } else { "down" },
                "red_zone": if self.arch == "x86_64" { "disabled-for-kernel-entry" } else { "not-applicable" },
                "shadow_space": if self.abi == "win64" { "32-byte caller home space required" } else { "not-required" }
            }
        })
    }

    fn loader_contract_json(self) -> serde_json::Value {
        serde_json::json!({
            "schema": "nc.kernel.loader.v1",
            "protocol": self.loader_protocol(),
            "entry": {
                "symbol": self.entry_symbol(),
                "abi": self.entry_abi(),
                "boot_info_argument": self.boot_info_argument(),
                "stack_pointer": self.stack_pointer_register(),
                "stack_alignment": self.stack_align
            },
            "boot_info": {
                "layout": "ncc_kernel_boot_info_v1",
                "endianness": "little",
                "argument": self.boot_info_argument(),
                "minimum_size": 88,
                "field_layout": [
                    {"name": "magic", "offset": 0, "type": "u64", "required": true},
                    {"name": "version", "offset": 8, "type": "u64", "required": true},
                    {"name": "flags", "offset": 16, "type": "u64", "required": true},
                    {"name": "memory_map_ptr", "offset": 24, "type": "ptr<ncc_kernel_memory_region_v1>", "required": true},
                    {"name": "memory_map_len", "offset": 32, "type": "u64 entries", "required": true},
                    {"name": "framebuffer_ptr", "offset": 40, "type": "ptr<ncc_kernel_framebuffer_v1>|0", "required": false},
                    {"name": "serial_ptr", "offset": 48, "type": "ptr<ncc_kernel_serial_v1>|0", "required": false},
                    {"name": "platform_table_ptr", "offset": 56, "type": "ptr<ncc_kernel_platform_table_v1>|0", "required": true},
                    {"name": "command_line_ptr", "offset": 64, "type": "ptr<u8>|0", "required": false},
                    {"name": "kernel_payload_ptr", "offset": 72, "type": "phys_addr|0", "required": false},
                    {"name": "kernel_payload_size", "offset": 80, "type": "u64 bytes", "required": false}
                ],
                "flag_bits": {
                    "0x1": "memory_map_present",
                    "0x2": "serial_present",
                    "0x4": "platform_table_present",
                    "0x8": "framebuffer_present"
                }
            },
            "memory_map": {
                "contract": self.memory_map_contract(),
                "entry_layout": "ncc_kernel_memory_region_v1",
                "entry_size": NBUILD_KERNEL_X86_MEMORY_REGION_SIZE,
                "fields": [
                    {"name": "base", "offset": 0, "type": "u64"},
                    {"name": "length", "offset": 8, "type": "u64"},
                    {"name": "kind", "offset": 16, "type": "u32"},
                    {"name": "flags", "offset": 20, "type": "u32"},
                    {"name": "source", "offset": 24, "type": "u32"},
                    {"name": "reserved", "offset": 28, "type": "u32"}
                ],
                "kind_catalog": {
                    "0": "reserved",
                    "1": "loader",
                    "2": "kernel_elf_file_image",
                    "3": "kernel_loaded_segment",
                    "4": "initial_stack",
                    "5": "firmware_usable",
                    "6": "acpi_reclaim",
                    "7": "acpi_nvs",
                    "8": "bad_memory",
                    "9": "firmware_runtime",
                    "10": "mmio",
                    "11": "persistent_memory"
                },
                "source_catalog": {
                    "1": "ncc-boot-v1-stage1",
                    "2": "elf64-program-header",
                    "3": "bios-int15-e820",
                    "4": "uefi-memory-map",
                    "5": "multiboot2-memory-map",
                    "6": "fdt-memory-nodes"
                }
            },
            "framebuffer": {
                "contract": self.framebuffer_contract(),
                "layout": "ncc_kernel_framebuffer_v1",
                "fields": ["base", "width", "height", "pitch", "format", "flags"],
                "optional": true
            },
            "serial": {
                "contract": self.serial_contract(),
                "layout": "ncc_kernel_serial_v1",
                "fields": ["magic", "version", "kind", "base", "irq", "baud", "flags", "register_shift"],
                "optional": true
            },
            "platform_table": {
                "layout": "ncc_kernel_platform_table_v1",
                "header_size": 16,
                "entry_size": NBUILD_KERNEL_PLATFORM_ENTRY_SIZE,
                "fields": [
                    {"name": "magic", "offset": 0, "type": "u64"},
                    {"name": "version", "offset": 8, "type": "u32"},
                    {"name": "entry_count", "offset": 12, "type": "u32"},
                    {"name": "entries", "offset": 16, "type": "[ncc_kernel_platform_entry_v1]"}
                ],
                "entry_layout": [
                    {"name": "kind", "offset": 0, "type": "u32"},
                    {"name": "flags", "offset": 4, "type": "u32"},
                    {"name": "ptr", "offset": 8, "type": "u64"},
                    {"name": "length", "offset": 16, "type": "u64"},
                    {"name": "checksum_or_revision", "offset": 24, "type": "u64"}
                ],
                "kind_catalog": {
                    "1": "ncc_boot_v1_own_contract",
                    "2": "acpi_rsdp",
                    "3": "uefi_system_table",
                    "4": "multiboot2_tags",
                    "5": "bios_e820",
                    "6": "fdt",
                    "7": "opensbi_hart",
                    "8": "wasm_host_imports",
                    "9": "system_loader_process_contract",
                    "10": "cpu_topology",
                    "11": "smbios32",
                    "12": "smbios64",
                    "13": "smbios_tables",
                    "14": "multiboot2_boot_device",
                    "15": "uefi_memory_map",
                    "16": "uefi_image_handle",
                    "17": "multiboot2_load_base",
                    "18": "multiboot2_vbe",
                    "19": "multiboot2_elf_sections",
                    "20": "multiboot2_apm",
                    "21": "multiboot2_network",
                    "22": "uefi_boot_services",
                    "23": "multiboot2_framebuffer"
                },
                "flag_bits": {
                    "0x1": "present",
                    "0x2": "required",
                    "0x4": "own_contract",
                    "0x8": "external_adapter",
                    "0x10": "zero_ptr_if_absent",
                    "0x20": "runtime_probe",
                    "0x40": "dynamic_length"
                },
                "entries": self.platform_tables(),
                "own_contract_allowed": true
            },
            "platform_tables": self.platform_tables()
        })
    }

    fn target_json(self) -> serde_json::Value {
        serde_json::json!({
            "triple": self.triple,
            "arch": self.arch,
            "abi": self.abi,
            "object": self.object,
            "pointer_width": self.pointer_width,
            "stack_align": self.stack_align,
            "system_loader_executable": self.system_loader_executable,
            "entry_contract": self.entry_contract_json(),
            "loader_contract": self.loader_contract_json(),
        })
    }
}

fn nbuild_kernel_target_spec(
    target: Option<&str>,
) -> Result<NbuildKernelTargetSpec, Box<dyn std::error::Error>> {
    let raw = target.unwrap_or("x86_64-unknown-none");
    let normalized = raw.to_ascii_lowercase();
    let spec = match normalized.as_str() {
        "x86_64-unknown-none" | "x86_64-unknown-none-elf" => NbuildKernelTargetSpec {
            triple: "x86_64-unknown-none",
            arch: "x86_64",
            abi: "freestanding",
            object: "elf64",
            pointer_width: 64,
            stack_align: 16,
            output_ext: "kernel.elf",
            system_loader_executable: false,
        },
        "x86_64-unknown-linux-gnu" => NbuildKernelTargetSpec {
            triple: "x86_64-unknown-linux-gnu",
            arch: "x86_64",
            abi: "sysv64",
            object: "elf64",
            pointer_width: 64,
            stack_align: 16,
            output_ext: "kernel.elf",
            system_loader_executable: true,
        },
        "x86_64-pc-windows-msvc" => NbuildKernelTargetSpec {
            triple: "x86_64-pc-windows-msvc",
            arch: "x86_64",
            abi: "win64",
            object: "coff",
            pointer_width: 64,
            stack_align: 16,
            output_ext: "kernel.exe",
            system_loader_executable: true,
        },
        "x86_64-apple-darwin" => NbuildKernelTargetSpec {
            triple: "x86_64-apple-darwin",
            arch: "x86_64",
            abi: "darwin",
            object: "macho",
            pointer_width: 64,
            stack_align: 16,
            output_ext: "kernel.macho",
            system_loader_executable: true,
        },
        "aarch64-unknown-none" | "aarch64-unknown-none-elf" => NbuildKernelTargetSpec {
            triple: "aarch64-unknown-none",
            arch: "aarch64",
            abi: "aapcs64",
            object: "elf64",
            pointer_width: 64,
            stack_align: 16,
            output_ext: "kernel.elf",
            system_loader_executable: false,
        },
        "aarch64-unknown-linux-gnu" => NbuildKernelTargetSpec {
            triple: "aarch64-unknown-linux-gnu",
            arch: "aarch64",
            abi: "aapcs64",
            object: "elf64",
            pointer_width: 64,
            stack_align: 16,
            output_ext: "kernel.elf",
            system_loader_executable: true,
        },
        "riscv64-unknown-elf" | "riscv64gc-unknown-none-elf" => NbuildKernelTargetSpec {
            triple: "riscv64-unknown-elf",
            arch: "riscv64",
            abi: "riscv64",
            object: "elf64",
            pointer_width: 64,
            stack_align: 16,
            output_ext: "kernel.elf",
            system_loader_executable: false,
        },
        "riscv32imac-unknown-elf" | "riscv32-unknown-elf" => NbuildKernelTargetSpec {
            triple: "riscv32imac-unknown-elf",
            arch: "riscv32",
            abi: "ilp32",
            object: "elf32",
            pointer_width: 32,
            stack_align: 16,
            output_ext: "kernel.elf",
            system_loader_executable: false,
        },
        "i386-unknown-linux-gnu" => NbuildKernelTargetSpec {
            triple: "i386-unknown-linux-gnu",
            arch: "i386",
            abi: "i386-sysv",
            object: "elf32",
            pointer_width: 32,
            stack_align: 4,
            output_ext: "kernel.nclink",
            system_loader_executable: false,
        },
        "i386-unknown-none" | "i386-unknown-none-elf" => NbuildKernelTargetSpec {
            triple: "i386-unknown-none-elf",
            arch: "i386",
            abi: "i386-sysv",
            object: "elf32",
            pointer_width: 32,
            stack_align: 4,
            output_ext: "kernel.elf",
            system_loader_executable: false,
        },
        "armv7a-unknown-none-eabi" | "armv7-unknown-none-eabi" => NbuildKernelTargetSpec {
            triple: "armv7a-unknown-none-eabi",
            arch: "armv7",
            abi: "aapcs32",
            object: "elf32",
            pointer_width: 32,
            stack_align: 8,
            output_ext: "kernel.elf",
            system_loader_executable: false,
        },
        "armv7em-none-eabi" | "thumbv7em-none-eabi" => NbuildKernelTargetSpec {
            triple: "armv7em-none-eabi",
            arch: "armv7em",
            abi: "aapcs32",
            object: "elf32",
            pointer_width: 32,
            stack_align: 8,
            output_ext: "firmware.bin",
            system_loader_executable: false,
        },
        "wasm32-unknown-unknown" => NbuildKernelTargetSpec {
            triple: "wasm32-unknown-unknown",
            arch: "wasm32",
            abi: "wasm",
            object: "wasm",
            pointer_width: 32,
            stack_align: 16,
            output_ext: "kernel.nclink",
            system_loader_executable: false,
        },
        _ => {
            return Err(actionable_command_error(
                "nbuild",
                format!(
                    "kernel profile target '{}' is not admitted; expected x86_64-unknown-none, x86_64-unknown-linux-gnu, x86_64-pc-windows-msvc, x86_64-apple-darwin, aarch64-unknown-none, aarch64-unknown-linux-gnu, armv7a-unknown-none-eabi, armv7em-none-eabi, riscv64-unknown-elf, riscv32imac-unknown-elf, i386-unknown-linux-gnu, i386-unknown-none-elf or wasm32-unknown-unknown",
                    raw
                ),
            )
            .into())
        }
    };
    Ok(spec)
}

fn default_nbuild_kernel_output_path(
    source_path: &str,
    artifact_root: &Path,
    spec: NbuildKernelTargetSpec,
) -> String {
    let stem = nbuild_source_stem(source_path);
    let safe_target = spec.triple.replace(['/', '\\', ':'], "_");
    artifact_root
        .join("bin")
        .join(format!("{stem}.{safe_target}.{}", spec.output_ext))
        .to_string_lossy()
        .to_string()
}

fn nbuild_kernel_instr(spec: NbuildKernelTargetSpec) -> &'static str {
    match spec.arch {
        "x86_64" => "                mov rax, 20299;\n                ret;",
        "i386" => "                mov eax, 20299;\n                ret;",
        "aarch64" => "                mov x0, 0;\n                ret;",
        "armv7" | "armv7em" => "                mov r0, 0;\n                bx lr;",
        "riscv64" | "riscv32" => "                add a0, zero, zero;\n                ret;",
        "wasm32" => "                i32.const 0;\n                return;",
        _ => "                nop;\n                ret;",
    }
}

fn nbuild_kernel_wrapper_instr(
    spec: NbuildKernelTargetSpec,
    body_lowered: bool,
    kernel_core_runtime: bool,
) -> String {
    if !body_lowered {
        return nbuild_kernel_instr(spec).to_string();
    }
    match (spec.arch, spec.abi, spec.system_loader_executable) {
        ("x86_64", "win64", true) => {
            let setter = if kernel_core_runtime {
                "                call __ncc_kernel_set_boot_info;\n"
            } else {
                ""
            };
            format!("                sub rsp, 40;\n{setter}                call main;\n                add rsp, 40;\n                ret;")
        }
        ("x86_64", _, true) => {
            let setter = if kernel_core_runtime {
                "                call __ncc_kernel_set_boot_info;\n"
            } else {
                ""
            };
            format!("                sub rsp, 8;\n{setter}                call main;\n                add rsp, 8;\n                ret;")
        }
        ("x86_64", _, false) => {
            let setter = if kernel_core_runtime {
                "                call __ncc_kernel_set_boot_info;\n"
            } else {
                ""
            };
            format!("                sub rsp, 8;\n{setter}                call main;\n                add rsp, 8;\n                cmp eax, 0;\n                sete al;\n                movzx eax, al;\n                imul eax, eax, 20299;\n                ret;")
        }
        ("i386", _, _) => {
            let setter = if kernel_core_runtime {
                "                mov eax, [esp + 4];\n                push 0;\n                push eax;\n                call __ncc_kernel_set_boot_info;\n                add esp, 8;\n"
            } else {
                ""
            };
            format!("{setter}                call main;\n                cmp eax, 0;\n                sete al;\n                movzx eax, al;\n                imul eax, eax, 20299;\n                ret;")
        }
        ("aarch64", _, _) => {
            let setter = if kernel_core_runtime {
                "                bl __ncc_kernel_set_boot_info;\n"
            } else {
                ""
            };
            format!("                sub sp, sp, 16;\n                str x30, [sp + 8];\n{setter}                bl main;\n                ldr x30, [sp + 8];\n                add sp, sp, 16;\n                ret;")
        }
        ("armv7" | "armv7em", _, _) => {
            let setter = if kernel_core_runtime {
                "                bl __ncc_kernel_set_boot_info;\n"
            } else {
                ""
            };
            format!("                push lr;\n{setter}                bl main;\n                pop pc;")
        }
        ("riscv64", _, _) => {
            let setter = if kernel_core_runtime {
                "                call __ncc_kernel_set_boot_info;\n"
            } else {
                ""
            };
            format!("                addi sp, sp, -16;\n                sd ra, 8(sp);\n{setter}                call main;\n                ld ra, 8(sp);\n                addi sp, sp, 16;\n                ret;")
        }
        ("riscv32", _, _) => {
            let setter = if kernel_core_runtime {
                "                call __ncc_kernel_set_boot_info;\n"
            } else {
                ""
            };
            format!("                addi sp, sp, -16;\n                sw ra, 12(sp);\n{setter}                call main;\n                lw ra, 12(sp);\n                addi sp, sp, 16;\n                ret;")
        }
        _ => nbuild_kernel_instr(spec).to_string(),
    }
}

fn nbuild_machine_string(raw: &str) -> String {
    nbuild_machine_literal_path(raw)
}

#[derive(Debug, Clone)]
struct NbuildKernelBodyLowering {
    llvm_ir_path: PathBuf,
    object_path: PathBuf,
    profile_label: &'static str,
    target_triple: String,
    clang_tool: String,
    clang_args: Vec<String>,
    module: serde_json::Value,
    stage: serde_json::Value,
    kernel_core_runtime: bool,
}

impl NbuildKernelBodyLowering {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "schema": "nc.compiler.kernel_body_lowering.v1",
            "status": "kernel_body_lowering_materialized",
            "profile": self.profile_label,
            "target_triple": self.target_triple,
            "llvm_ir": self.llvm_ir_path.display().to_string(),
            "object": self.object_path.display().to_string(),
            "llvm_sha256": sha256_hex_file(&self.llvm_ir_path).ok(),
            "object_sha256": sha256_hex_file(&self.object_path).ok(),
            "clang": {
                "tool": self.clang_tool,
                "args": self.clang_args,
                "stage": self.stage,
            },
            "module": self.module,
            "kernel_core_runtime": self.kernel_core_runtime,
            "coverage": {
                "variables": true,
                "calls": true,
                "structs": true,
                "pointers": true,
                "arrays": true,
                "strings": true,
                "control_flow": true,
                "source_of_truth": "MCT high-level .n lowering",
            }
        })
    }
}

#[derive(Debug, Clone)]
enum NbuildKernelBodyLoweringResult {
    Materialized(NbuildKernelBodyLowering),
    NotMaterialized(serde_json::Value),
}

impl NbuildKernelBodyLoweringResult {
    fn is_materialized(&self) -> bool {
        matches!(self, Self::Materialized(_))
    }

    fn materialized(&self) -> Option<&NbuildKernelBodyLowering> {
        match self {
            Self::Materialized(body) => Some(body),
            Self::NotMaterialized(_) => None,
        }
    }

    fn kernel_core_runtime(&self) -> bool {
        self.materialized()
            .is_some_and(|body| body.kernel_core_runtime)
    }

    fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Materialized(body) => body.to_json(),
            Self::NotMaterialized(value) => value.clone(),
        }
    }
}

fn nbuild_kernel_body_profile(
    spec: NbuildKernelTargetSpec,
) -> Option<(&'static str, ncc_ir::BackendLoweringProfileIR)> {
    match spec.triple {
        "x86_64-unknown-none" => {
            let mut profile = ncc_ir::BackendLoweringProfileIR::nc_sysv64();
            profile.calling_convention = ncc_ir::BackendCallingConventionIR::NcAapcs64;
            profile.target_spec = ncc_ir::BackendTargetSpecIR::from_target_and_calling_convention(
                &ncc_ir::BackendTargetIR::X86_64,
                &ncc_ir::BackendCallingConventionIR::NcAapcs64,
            );
            profile.target_spec.target_triple = "x86_64-unknown-none".to_string();
            Some(("nC.kernel.x86_64-freestanding", profile))
        }
        "x86_64-unknown-linux-gnu" => Some((
            "nC.kernel.x86_64-system-loader",
            ncc_ir::BackendLoweringProfileIR::nc_sysv64(),
        )),
        "x86_64-pc-windows-msvc" => Some((
            "nC.kernel.x86_64-win64-loader",
            ncc_ir::BackendLoweringProfileIR::nc_win64(),
        )),
        "x86_64-apple-darwin" => {
            let mut profile = ncc_ir::BackendLoweringProfileIR::nc_sysv64();
            profile.target_spec.target_triple = "x86_64-apple-darwin".to_string();
            profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::MachO;
            Some(("nC.kernel.x86_64-darwin-loader", profile))
        }
        "aarch64-unknown-none" => {
            let mut profile = ncc_ir::BackendLoweringProfileIR::nc_aapcs64();
            profile.target_spec.target_triple = "aarch64-unknown-none".to_string();
            profile.target_spec.target_family = ncc_ir::BackendTargetFamilyIR::Freestanding;
            profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::Elf64;
            profile.target_spec.relocation_model = ncc_ir::BackendRelocationModelIR::Absolute;
            profile.target_spec.code_model = ncc_ir::BackendCodeModelIR::Kernel;
            profile.target_spec.runtime_model = ncc_ir::BackendRuntimeModelIR::Freestanding;
            profile.target_spec.host_services = ncc_ir::BackendHostServicesIR::BareMetal;
            profile.target_spec.debug_info_model = ncc_ir::BackendDebugInfoModelIR::None;
            profile.target_spec.linker_model = ncc_ir::BackendLinkerModelIR::FreestandingInternal;
            profile.target_spec.support_tier = ncc_ir::BackendSupportTierIR::Tier3;
            profile.target_spec.backend_kind = ncc_ir::BackendKindIR::Freestanding;
            profile.target_spec.profile_constraints =
                vec![ncc_ir::BackendProfileConstraintIR::RequiresFreestandingRuntime];
            Some(("nC.kernel.aarch64-freestanding", profile))
        }
        "aarch64-unknown-linux-gnu" => {
            let mut profile = ncc_ir::BackendLoweringProfileIR::nc_aapcs64();
            profile.target_spec.target_triple = "aarch64-unknown-linux-gnu".to_string();
            profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::Elf64;
            Some(("nC.kernel.aarch64-linux-loader", profile))
        }
        "i386-unknown-linux-gnu" | "i386-unknown-none-elf" => {
            let mut profile = ncc_ir::BackendLoweringProfileIR::nc_portable();
            profile.target = ncc_ir::BackendTargetIR::X86_64;
            profile.target_spec.target_triple = if spec.triple == "i386-unknown-none-elf" {
                "i386-unknown-none-elf".to_string()
            } else {
                "i386-unknown-linux-gnu".to_string()
            };
            profile.target_spec.target_family = if spec.triple == "i386-unknown-none-elf" {
                ncc_ir::BackendTargetFamilyIR::Freestanding
            } else {
                ncc_ir::BackendTargetFamilyIR::Hosted
            };
            profile.target_spec.isa = ncc_ir::BackendIsaIR::X86;
            profile.target_spec.abi = ncc_ir::BackendAbiModelIR::I386SysV;
            profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::Elf32;
            profile.target_spec.relocation_model = ncc_ir::BackendRelocationModelIR::Static;
            profile.target_spec.code_model = if spec.triple == "i386-unknown-none-elf" {
                ncc_ir::BackendCodeModelIR::Kernel
            } else {
                ncc_ir::BackendCodeModelIR::Small
            };
            profile.target_spec.data_layout = ncc_ir::BackendDataLayoutIR::i386_sysv();
            profile.target_spec.pointer_width = 32;
            profile.target_spec.runtime_model = if spec.triple == "i386-unknown-none-elf" {
                ncc_ir::BackendRuntimeModelIR::Freestanding
            } else {
                ncc_ir::BackendRuntimeModelIR::Hosted
            };
            profile.target_spec.host_services = if spec.triple == "i386-unknown-none-elf" {
                ncc_ir::BackendHostServicesIR::BareMetal
            } else {
                ncc_ir::BackendHostServicesIR::Os
            };
            profile.target_spec.debug_info_model = if spec.triple == "i386-unknown-none-elf" {
                ncc_ir::BackendDebugInfoModelIR::None
            } else {
                ncc_ir::BackendDebugInfoModelIR::Dwarf
            };
            profile.target_spec.linker_model = if spec.triple == "i386-unknown-none-elf" {
                ncc_ir::BackendLinkerModelIR::FreestandingInternal
            } else {
                ncc_ir::BackendLinkerModelIR::HostNative
            };
            profile.target_spec.support_tier = ncc_ir::BackendSupportTierIR::Tier3;
            profile.target_spec.backend_kind = if spec.triple == "i386-unknown-none-elf" {
                ncc_ir::BackendKindIR::Freestanding
            } else {
                ncc_ir::BackendKindIR::HostedProduction
            };
            profile.target_spec.profile_constraints = if spec.triple == "i386-unknown-none-elf" {
                vec![ncc_ir::BackendProfileConstraintIR::RequiresFreestandingRuntime]
            } else {
                vec![ncc_ir::BackendProfileConstraintIR::RequiresHostedRuntime]
            };
            Some(("nC.kernel.i386-elf32-body", profile))
        }
        "armv7a-unknown-none-eabi" | "armv7em-none-eabi" => {
            let microcontroller = spec.triple == "armv7em-none-eabi";
            let mut profile = ncc_ir::BackendLoweringProfileIR::nc_portable();
            profile.target = ncc_ir::BackendTargetIR::X86_64;
            profile.target_spec.target_triple = if microcontroller {
                "thumbv7em-none-eabi".to_string()
            } else {
                "armv7a-none-eabi".to_string()
            };
            profile.target_spec.target_family = ncc_ir::BackendTargetFamilyIR::Freestanding;
            profile.target_spec.isa = ncc_ir::BackendIsaIR::ArmV7;
            profile.target_spec.abi = ncc_ir::BackendAbiModelIR::Aapcs32;
            profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::Elf32;
            profile.target_spec.relocation_model = ncc_ir::BackendRelocationModelIR::Static;
            profile.target_spec.code_model = ncc_ir::BackendCodeModelIR::Kernel;
            profile.target_spec.data_layout = ncc_ir::BackendDataLayoutIR::ilp32();
            profile.target_spec.pointer_width = 32;
            profile.target_spec.runtime_model = ncc_ir::BackendRuntimeModelIR::Freestanding;
            profile.target_spec.host_services = ncc_ir::BackendHostServicesIR::BareMetal;
            profile.target_spec.debug_info_model = ncc_ir::BackendDebugInfoModelIR::None;
            profile.target_spec.linker_model = ncc_ir::BackendLinkerModelIR::FreestandingInternal;
            profile.target_spec.support_tier = ncc_ir::BackendSupportTierIR::Tier3;
            profile.target_spec.backend_kind = ncc_ir::BackendKindIR::Freestanding;
            profile.target_spec.profile_constraints =
                vec![ncc_ir::BackendProfileConstraintIR::RequiresFreestandingRuntime];
            Some((
                if microcontroller {
                    "nC.firmware.thumbv7em-elf32-body"
                } else {
                    "nC.kernel.armv7a-thumb2-elf32-body"
                },
                profile,
            ))
        }
        "riscv64-unknown-elf" => {
            let mut profile = ncc_ir::BackendLoweringProfileIR::nc_portable();
            profile.target = ncc_ir::BackendTargetIR::X86_64;
            profile.target_spec.target_triple = "riscv64-unknown-elf".to_string();
            profile.target_spec.target_family = ncc_ir::BackendTargetFamilyIR::Freestanding;
            profile.target_spec.isa = ncc_ir::BackendIsaIR::RiscV64;
            profile.target_spec.abi = ncc_ir::BackendAbiModelIR::RiscV64;
            profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::Elf64;
            profile.target_spec.relocation_model = ncc_ir::BackendRelocationModelIR::Static;
            profile.target_spec.code_model = ncc_ir::BackendCodeModelIR::Kernel;
            profile.target_spec.data_layout = ncc_ir::BackendDataLayoutIR::lp64();
            profile.target_spec.pointer_width = 64;
            profile.target_spec.runtime_model = ncc_ir::BackendRuntimeModelIR::Freestanding;
            profile.target_spec.host_services = ncc_ir::BackendHostServicesIR::BareMetal;
            profile.target_spec.debug_info_model = ncc_ir::BackendDebugInfoModelIR::None;
            profile.target_spec.linker_model = ncc_ir::BackendLinkerModelIR::FreestandingInternal;
            profile.target_spec.support_tier = ncc_ir::BackendSupportTierIR::Tier3;
            profile.target_spec.backend_kind = ncc_ir::BackendKindIR::Freestanding;
            profile.target_spec.profile_constraints =
                vec![ncc_ir::BackendProfileConstraintIR::RequiresFreestandingRuntime];
            Some(("nC.kernel.riscv64-elf-body", profile))
        }
        "riscv32imac-unknown-elf" => {
            let mut profile = ncc_ir::BackendLoweringProfileIR::nc_portable();
            profile.target = ncc_ir::BackendTargetIR::X86_64;
            profile.target_spec.target_triple = "riscv32-unknown-elf".to_string();
            profile.target_spec.target_family = ncc_ir::BackendTargetFamilyIR::Freestanding;
            profile.target_spec.isa = ncc_ir::BackendIsaIR::RiscV32;
            profile.target_spec.abi = ncc_ir::BackendAbiModelIR::Ilp32;
            profile.target_spec.object_format = ncc_ir::BackendObjectFormatIR::Elf32;
            profile.target_spec.relocation_model = ncc_ir::BackendRelocationModelIR::Static;
            profile.target_spec.code_model = ncc_ir::BackendCodeModelIR::Kernel;
            profile.target_spec.data_layout = ncc_ir::BackendDataLayoutIR::ilp32();
            profile.target_spec.pointer_width = 32;
            profile.target_spec.runtime_model = ncc_ir::BackendRuntimeModelIR::Freestanding;
            profile.target_spec.host_services = ncc_ir::BackendHostServicesIR::BareMetal;
            profile.target_spec.debug_info_model = ncc_ir::BackendDebugInfoModelIR::None;
            profile.target_spec.linker_model = ncc_ir::BackendLinkerModelIR::FreestandingInternal;
            profile.target_spec.support_tier = ncc_ir::BackendSupportTierIR::Tier3;
            profile.target_spec.backend_kind = ncc_ir::BackendKindIR::Freestanding;
            profile.target_spec.profile_constraints =
                vec![ncc_ir::BackendProfileConstraintIR::RequiresFreestandingRuntime];
            Some(("nC.kernel.riscv32imac-elf32-body", profile))
        }
        _ => None,
    }
}

fn nbuild_kernel_compile_body_object(
    ast: ncc_ast::Program,
    source_path: &str,
    source: &str,
    spec: NbuildKernelTargetSpec,
    stem: &str,
    lowering_dir: &Path,
    obj_dir: &Path,
) -> Result<NbuildKernelBodyLoweringResult, Box<dyn std::error::Error>> {
    let Some((profile_label, profile)) = nbuild_kernel_body_profile(spec) else {
        return Ok(NbuildKernelBodyLoweringResult::NotMaterialized(
            serde_json::json!({
                "schema": "nc.compiler.kernel_body_lowering.v1",
                "status": "not_materialized_for_target",
                "target": spec.triple,
                "reason": "no admitted high-level MCT object profile for this kernel target yet",
                "coverage": {
                    "variables": false,
                    "calls": false,
                    "structs": false,
                    "pointers": false,
                    "arrays": false,
                    "strings": false,
                    "control_flow": false,
                }
            }),
        ));
    };

    let pass_pipeline = ncc_ir::mct_llvm_pass_pipeline(ncc_ir::MctLlvmOptimizationLevelIR::O2);
    let mut ir = ncc_ir::lower_with_backend_profile_and_source_exact(
        ast,
        profile.clone(),
        source_path,
        source,
    );
    ir.target = profile.target.clone();
    ir.target_spec = profile.target_spec.clone();
    let target_attributes = ncc_ir::mct_llvm_target_attributes_for_spec(&ir.target_spec);
    let module = ncc_ir::mct_emit_llvm_module_with_debug_config_and_target_attributes(
        &ir,
        pass_pipeline.clone(),
        super::mct_json::mct_llvm_debug_config_for_path(source_path),
        target_attributes,
    );
    if module.emitted_function_count == 0 || !module.llvm_ir.contains("define i32 @main()") {
        let module_json = super::mct_json::mct_llvm_module_to_json(&module, false, None);
        return Ok(NbuildKernelBodyLoweringResult::NotMaterialized(
            serde_json::json!({
                "schema": "nc.compiler.kernel_body_lowering.v1",
                "status": "not_materialized_by_mct",
                "target": spec.triple,
                "profile": profile_label,
                "reason": "parser and sema accepted the kernel source, but MCT did not emit an executable high-level main body for this unit",
                "emitted_function_count": module.emitted_function_count,
                "rejected_function_count": module.rejected_function_count,
                "module": module_json,
                "coverage": {
                    "variables": false,
                    "calls": false,
                    "structs": false,
                    "pointers": false,
                    "arrays": false,
                    "strings": false,
                    "control_flow": false,
                    "source_of_truth": "MCT high-level .n lowering",
                }
            }),
        ));
    }

    let safe_target = spec.triple.replace(['/', '\\', ':'], "_");
    let ll_path = lowering_dir.join(format!("{stem}.{safe_target}.kernel.body.ll"));
    let body_obj_ext = if spec.object == "coff" { "obj" } else { "o" };
    let body_obj_path = obj_dir.join(format!("{stem}.{safe_target}.kernel.body.{body_obj_ext}"));
    fs::write(&ll_path, module.llvm_ir.as_bytes())?;
    require_non_empty_output_file(
        "nbuild kernel high-level body LLVM",
        ll_path.to_string_lossy().as_ref(),
    )?;

    let clang_tool = super::mct_support::mct_find_tool("clang").ok_or_else(|| {
        actionable_command_error(
            "nbuild",
            "kernel body lowering requires clang to materialize the MCT LLVM body into a freestanding object",
        )
    })?;
    let mut clang_args = vec![
        format!("--target={}", profile.target_spec.target_triple),
        pass_pipeline.clang_opt_arg.to_string(),
        "-ffreestanding".to_string(),
        "-fno-stack-protector".to_string(),
        "-fno-exceptions".to_string(),
        "-fno-unwind-tables".to_string(),
        "-fno-asynchronous-unwind-tables".to_string(),
        "-x".to_string(),
        "ir".to_string(),
        "-c".to_string(),
        ll_path.display().to_string(),
        "-o".to_string(),
        body_obj_path.display().to_string(),
    ];
    if spec.arch == "x86_64" && !spec.system_loader_executable {
        clang_args.insert(6, "-mno-red-zone".to_string());
    }
    if spec.arch == "riscv32" {
        clang_args.insert(1, "-march=rv32imac".to_string());
        clang_args.insert(2, "-mabi=ilp32".to_string());
        clang_args.insert(3, "-mcmodel=medany".to_string());
    } else if spec.arch == "riscv64" {
        clang_args.insert(1, "-march=rv64imac".to_string());
        clang_args.insert(2, "-mabi=lp64".to_string());
        clang_args.insert(3, "-mcmodel=medany".to_string());
    } else if spec.arch == "armv7" {
        clang_args.insert(1, "-mthumb".to_string());
        clang_args.insert(2, "-march=armv7-a".to_string());
        clang_args.insert(3, "-mfloat-abi=soft".to_string());
    } else if spec.arch == "armv7em" {
        clang_args.insert(1, "-mthumb".to_string());
        clang_args.insert(2, "-mcpu=cortex-m4".to_string());
        clang_args.insert(3, "-mfloat-abi=soft".to_string());
    }
    let output = std::process::Command::new(&clang_tool)
        .args(&clang_args)
        .output();
    let stage = match output {
        Ok(output) => serde_json::json!({
            "stage": "clang-kernel-body-object",
            "tool": clang_tool,
            "args": clang_args,
            "status": if output.status.success() { "passed" } else { "failed" },
            "status_code": output.status.code(),
            "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
            "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
        }),
        Err(err) => serde_json::json!({
            "stage": "clang-kernel-body-object",
            "tool": clang_tool,
            "args": clang_args,
            "status": "missing_tool",
            "error": err.to_string(),
        }),
    };
    if stage.get("status").and_then(serde_json::Value::as_str) != Some("passed") {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "kernel body lowering failed while compiling LLVM IR to object: {}",
                serde_json::to_string_pretty(&stage)?
            ),
        )
        .into());
    }
    require_non_empty_output_file(
        "nbuild kernel high-level body object",
        body_obj_path.to_string_lossy().as_ref(),
    )?;

    let module_json = super::mct_json::mct_llvm_module_to_json(&module, false, Some(&ll_path));
    let kernel_core_runtime = module
        .llvm_ir
        .contains("define void @__ncc_kernel_set_boot_info");
    Ok(NbuildKernelBodyLoweringResult::Materialized(
        NbuildKernelBodyLowering {
            llvm_ir_path: ll_path,
            object_path: body_obj_path,
            profile_label,
            target_triple: profile.target_spec.target_triple,
            clang_tool,
            clang_args,
            module: module_json,
            stage,
            kernel_core_runtime,
        },
    ))
}

#[allow(dead_code)]
fn nbuild_kernel_x86_64_boot_sector_bytes() -> Vec<u8> {
    let mut sector = vec![
        0xfa, 0x31, 0xc0, 0x8e, 0xd8, 0x8e, 0xc0, 0x8e, 0xd0, 0xbc, 0x00, 0x7c, 0xe4, 0x92, 0x0c,
        0x02, 0xe6, 0x92, 0xe8, 0x7c, 0x00, 0x66, 0x31, 0xc0, 0xbf, 0x00, 0x10, 0xb9, 0x00, 0x0c,
        0xfc, 0xf3, 0x66, 0xab, 0x66, 0xc7, 0x06, 0x00, 0x10, 0x03, 0x20, 0x00, 0x00, 0x66, 0xc7,
        0x06, 0x04, 0x10, 0x00, 0x00, 0x00, 0x00, 0x66, 0xc7, 0x06, 0x00, 0x20, 0x03, 0x30, 0x00,
        0x00, 0x66, 0xc7, 0x06, 0x04, 0x20, 0x00, 0x00, 0x00, 0x00, 0x66, 0xc7, 0x06, 0x00, 0x30,
        0x83, 0x00, 0x00, 0x00, 0x66, 0xc7, 0x06, 0x04, 0x30, 0x00, 0x00, 0x00, 0x00, 0x0f, 0x01,
        0x16, 0x7f, 0x7d, 0x0f, 0x20, 0xe0, 0x66, 0x83, 0xc8, 0x20, 0x0f, 0x22, 0xe0, 0x66, 0xb8,
        0x00, 0x10, 0x00, 0x00, 0x0f, 0x22, 0xd8, 0x66, 0xb9, 0x80, 0x00, 0x00, 0xc0, 0x0f, 0x32,
        0x66, 0x0d, 0x00, 0x01, 0x00, 0x00, 0x0f, 0x30, 0x0f, 0x20, 0xc0, 0x66, 0x0d, 0x01, 0x00,
        0x00, 0x80, 0x0f, 0x22, 0xc0, 0xea, 0xbc, 0x7c, 0x08, 0x00, 0xba, 0xf9, 0x03, 0x30, 0xc0,
        0xee, 0xba, 0xfb, 0x03, 0xb0, 0x80, 0xee, 0xba, 0xf8, 0x03, 0xb0, 0x03, 0xee, 0xba, 0xf9,
        0x03, 0x30, 0xc0, 0xee, 0xba, 0xfb, 0x03, 0xb0, 0x03, 0xee, 0xba, 0xfa, 0x03, 0xb0, 0xc7,
        0xee, 0xba, 0xfc, 0x03, 0xb0, 0x0b, 0xee, 0xc3, 0x66, 0xb8, 0x10, 0x00, 0x8e, 0xd8, 0x8e,
        0xc0, 0x8e, 0xd0, 0xbc, 0x00, 0x00, 0x09, 0x00, 0x48, 0x8d, 0x3d, 0x36, 0x00, 0x00, 0x00,
        0xe8, 0x03, 0x00, 0x00, 0x00, 0xf4, 0xeb, 0xfd, 0x48, 0x8d, 0x35, 0x6f, 0x00, 0x00, 0x00,
        0xe8, 0x01, 0x00, 0x00, 0x00, 0xc3, 0xac, 0x84, 0xc0, 0x74, 0x14, 0x88, 0xc4, 0x66, 0xba,
        0xfd, 0x03, 0xec, 0xa8, 0x20, 0x74, 0xf7, 0x88, 0xe0, 0x66, 0xba, 0xf8, 0x03, 0xee, 0xeb,
        0xe7, 0xc3, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x4e, 0x43, 0x43, 0x42, 0x4f, 0x4f,
        0x31, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf8, 0x03, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x6e, 0x43, 0x20, 0x6b, 0x65, 0x72, 0x6e, 0x65, 0x6c, 0x20, 0x6c, 0x6f, 0x6e,
        0x67, 0x20, 0x6b, 0x6d, 0x61, 0x69, 0x6e, 0x0d, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x9a, 0xaf, 0x00, 0xff, 0xff, 0x00, 0x00,
        0x00, 0x92, 0xcf, 0x00, 0x1d, 0x00, 0x67, 0x7d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x55,
        0xaa,
    ];
    sector.truncate(sector.len().saturating_sub(2));
    sector.splice(310..310, [0x00, 0x00]);
    sector.splice(334..334, [0x00, 0x00]);
    sector.resize(510, 0x00);
    sector.push(0x55);
    sector.push(0xaa);
    sector
}

fn nbuild_kernel_read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn nbuild_kernel_read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn nbuild_kernel_read_u64_le(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

fn nbuild_kernel_find_unique_bytes(
    haystack: &[u8],
    needle: &[u8],
    what: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let matches = haystack
        .windows(needle.len())
        .enumerate()
        .filter_map(|(idx, window)| (window == needle).then_some(idx))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [idx] => Ok(*idx),
        [] => Err(actionable_command_error(
            "nbuild",
            format!("internal x86_64 kernel boot image template is missing {what}"),
        )
        .into()),
        _ => Err(actionable_command_error(
            "nbuild",
            format!("internal x86_64 kernel boot image template has duplicate {what} markers"),
        )
        .into()),
    }
}

#[derive(Clone)]
struct NbuildKernelElf64LoadSegment {
    offset: u64,
    vaddr: u64,
    paddr: u64,
    filesz: u64,
    memsz: u64,
    flags: u32,
    align: u64,
}

struct NbuildKernelElf64ImageInfo {
    entry: u64,
    phoff: u64,
    phentsize: u16,
    phnum: u16,
    load_segments: Vec<NbuildKernelElf64LoadSegment>,
}

struct NbuildKernelElf32ImageInfo {
    entry: u32,
    phoff: u32,
    phentsize: u16,
    phnum: u16,
    load_segments: Vec<NbuildKernelElf64LoadSegment>,
}

#[derive(Clone)]
struct NbuildKernelMemoryRegionV1 {
    base: u64,
    length: u64,
    kind: u32,
    flags: u32,
    source: u32,
}

const NBUILD_KERNEL_SECTOR_SIZE: usize = 512;
const NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS: u64 = 0x8000;
const NBUILD_KERNEL_X86_STAGE1_RESERVED_BYTES: usize = 0x1000;
const NBUILD_KERNEL_X86_KERNEL_ELF_MEMORY_ADDRESS: u64 = 0x9000;
const NBUILD_KERNEL_X86_BOOT_STACK_TOP: u64 = 0x90000;
const NBUILD_KERNEL_X86_BOOT_STACK_SIZE: u64 = 0x10000;
const NBUILD_KERNEL_X86_BOOT_STACK_LOW: u64 =
    NBUILD_KERNEL_X86_BOOT_STACK_TOP - NBUILD_KERNEL_X86_BOOT_STACK_SIZE;
const NBUILD_KERNEL_X86_BOOT_INFO_SIZE: usize = 88;
const NBUILD_KERNEL_X86_MEMORY_REGION_SIZE: usize = 32;
const NBUILD_KERNEL_PLATFORM_ENTRY_SIZE: usize = 32;
const NBUILD_KERNEL_X86_MAX_STAGE0_CHS_PAYLOAD_SECTORS: usize = 2879;
const NBUILD_KERNEL_X86_PAYLOAD_SIZE_MARKER: u64 = 0x1122_3344_5566_7788;
const NBUILD_KERNEL_X86_E820_MAX_ENTRIES: usize = 16;
const NBUILD_KERNEL_X86_STAGE1_ACPI_PLATFORM_SLOT_PTR_OFFSET: usize = 0x490;
const NBUILD_KERNEL_X86_STAGE1_ACPI_FOUND_OFFSET: usize = 0x492;
const NBUILD_KERNEL_X86_STAGE1_E820_NCC_PTR_OFFSET: usize = 0x498;
const NBUILD_KERNEL_X86_STAGE1_E820_MAX_ENTRIES_OFFSET: usize = 0x49a;
const NBUILD_KERNEL_X86_STAGE1_E820_MEMORY_MAP_LEN_PTR_OFFSET: usize = 0x49c;
const NBUILD_KERNEL_X86_STAGE1_E820_PLATFORM_SLOT_PTR_OFFSET: usize = 0x49e;
const NBUILD_KERNEL_X86_STAGE1_E820_SYNTHETIC_COUNT_OFFSET: usize = 0x4a0;
const NBUILD_KERNEL_X86_STAGE1_E820_RUNTIME_COUNT_OFFSET: usize = 0x4a2;
const NBUILD_KERNEL_X86_STAGE1_REAL_MODE_PROBE_STATE_END_OFFSET: usize = 0x4c0;

const NBUILD_MEM_KIND_RESERVED: u32 = 0;
const NBUILD_MEM_KIND_LOADER: u32 = 1;
const NBUILD_MEM_KIND_KERNEL_FILE: u32 = 2;
const NBUILD_MEM_KIND_KERNEL_SEGMENT: u32 = 3;
const NBUILD_MEM_KIND_STACK: u32 = 4;
const NBUILD_MEM_KIND_USABLE: u32 = 5;
const NBUILD_MEM_KIND_ACPI_RECLAIM: u32 = 6;
const NBUILD_MEM_KIND_ACPI_NVS: u32 = 7;
const NBUILD_MEM_KIND_BAD: u32 = 8;
const NBUILD_MEM_KIND_RUNTIME: u32 = 9;
const NBUILD_MEM_KIND_MMIO: u32 = 10;
const NBUILD_MEM_KIND_PERSISTENT: u32 = 11;
const NBUILD_MEM_FLAG_PRESENT: u32 = 0x0001;
const NBUILD_MEM_FLAG_READ: u32 = 0x0010;
const NBUILD_MEM_FLAG_WRITE: u32 = 0x0020;
const NBUILD_MEM_FLAG_EXECUTE: u32 = 0x0040;
const NBUILD_MEM_FLAG_IDENTITY_MAPPED: u32 = 0x0080;
const NBUILD_MEM_SOURCE_OWN_BOOT: u32 = 1;
const NBUILD_MEM_SOURCE_ELF_PHDR: u32 = 2;
const NBUILD_MEM_SOURCE_BIOS_E820: u32 = 3;
const NBUILD_MEM_SOURCE_UEFI: u32 = 4;
const NBUILD_MEM_SOURCE_MULTIBOOT2: u32 = 5;
const NBUILD_MEM_SOURCE_FDT: u32 = 6;

const NBUILD_PLATFORM_KIND_NCC_BOOT_V1_OWN_CONTRACT: u32 = 1;
const NBUILD_PLATFORM_KIND_ACPI_RSDP: u32 = 2;
const NBUILD_PLATFORM_KIND_UEFI_SYSTEM_TABLE: u32 = 3;
const NBUILD_PLATFORM_KIND_MULTIBOOT2_TAGS: u32 = 4;
const NBUILD_PLATFORM_KIND_BIOS_E820: u32 = 5;
const NBUILD_PLATFORM_KIND_FDT: u32 = 6;
const NBUILD_PLATFORM_KIND_OPENSBI_HART: u32 = 7;
const NBUILD_PLATFORM_KIND_WASM_HOST_IMPORTS: u32 = 8;
const NBUILD_PLATFORM_KIND_SYSTEM_LOADER_PROCESS: u32 = 9;
const NBUILD_PLATFORM_KIND_CPU_TOPOLOGY: u32 = 10;
const NBUILD_PLATFORM_KIND_SMBIOS32: u32 = 11;
const NBUILD_PLATFORM_KIND_SMBIOS64: u32 = 12;
const NBUILD_PLATFORM_KIND_SMBIOS_TABLES: u32 = 13;
const NBUILD_PLATFORM_KIND_MULTIBOOT2_BOOT_DEVICE: u32 = 14;
const NBUILD_PLATFORM_KIND_UEFI_MEMORY_MAP: u32 = 15;
const NBUILD_PLATFORM_KIND_UEFI_IMAGE_HANDLE: u32 = 16;
const NBUILD_PLATFORM_KIND_MULTIBOOT2_LOAD_BASE: u32 = 17;
const NBUILD_PLATFORM_KIND_MULTIBOOT2_VBE: u32 = 18;
const NBUILD_PLATFORM_KIND_MULTIBOOT2_ELF_SECTIONS: u32 = 19;
const NBUILD_PLATFORM_KIND_MULTIBOOT2_APM: u32 = 20;
const NBUILD_PLATFORM_KIND_MULTIBOOT2_NETWORK: u32 = 21;
const NBUILD_PLATFORM_KIND_UEFI_BOOT_SERVICES: u32 = 22;
const NBUILD_PLATFORM_KIND_MULTIBOOT2_FRAMEBUFFER: u32 = 23;

const NBUILD_PLATFORM_FLAG_PRESENT: u32 = 0x0001;
const NBUILD_PLATFORM_FLAG_REQUIRED: u32 = 0x0002;
const NBUILD_PLATFORM_FLAG_OWN_CONTRACT: u32 = 0x0004;
const NBUILD_PLATFORM_FLAG_EXTERNAL_ADAPTER: u32 = 0x0008;
const NBUILD_PLATFORM_FLAG_ZERO_PTR_IF_ABSENT: u32 = 0x0010;
const NBUILD_PLATFORM_FLAG_RUNTIME_PROBE: u32 = 0x0020;
const NBUILD_PLATFORM_FLAG_DYNAMIC_LENGTH: u32 = 0x0040;

fn nbuild_kernel_hex_u64(value: u64) -> String {
    format!("0x{value:x}")
}

fn nbuild_kernel_align_up_u64(value: u64, align: u64) -> u64 {
    if align == 0 {
        return value;
    }
    value.div_ceil(align) * align
}

fn nbuild_kernel_align_vec(buf: &mut Vec<u8>, align: usize) {
    if align == 0 {
        return;
    }
    let aligned = buf.len().div_ceil(align) * align;
    buf.resize(aligned, 0x00);
}

fn nbuild_kernel_push_u32_le(buf: &mut Vec<u8>, value: u32) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn nbuild_kernel_push_u16_le(buf: &mut Vec<u8>, value: u16) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn nbuild_kernel_push_u64_le(buf: &mut Vec<u8>, value: u64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn nbuild_kernel_patch_u32_le(
    bytes: &mut [u8],
    offset: usize,
    value: u32,
    what: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let dst = bytes.get_mut(offset..offset + 4).ok_or_else(|| {
        actionable_command_error(
            "nbuild",
            format!("internal stage1 template cannot patch {what}"),
        )
    })?;
    dst.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn nbuild_kernel_patch_u64_le(
    bytes: &mut [u8],
    offset: usize,
    value: u64,
    what: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let dst = bytes.get_mut(offset..offset + 8).ok_or_else(|| {
        actionable_command_error(
            "nbuild",
            format!("internal stage1 template cannot patch {what}"),
        )
    })?;
    dst.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn nbuild_kernel_patch_u16_le(
    bytes: &mut [u8],
    offset: usize,
    value: u16,
    what: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let dst = bytes.get_mut(offset..offset + 2).ok_or_else(|| {
        actionable_command_error(
            "nbuild",
            format!("internal stage1 template cannot patch {what}"),
        )
    })?;
    dst.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn nbuild_kernel_patch_rel16(
    bytes: &mut [u8],
    disp_offset: usize,
    target_offset: usize,
    what: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let next = disp_offset + 2;
    let rel = target_offset as isize - next as isize;
    if rel < i16::MIN as isize || rel > i16::MAX as isize {
        return Err(actionable_command_error(
            "nbuild",
            format!("internal i386 stage1 {what} rel16 is out of range"),
        )
        .into());
    }
    nbuild_kernel_patch_u16_le(bytes, disp_offset, rel as i16 as u16, what)
}

fn nbuild_kernel_patch_rel32(
    bytes: &mut [u8],
    disp_offset: usize,
    target_offset: usize,
    what: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let next = disp_offset + 4;
    let rel = target_offset as isize - next as isize;
    if rel < i32::MIN as isize || rel > i32::MAX as isize {
        return Err(actionable_command_error(
            "nbuild",
            format!("internal i386 stage1 {what} rel32 is out of range"),
        )
        .into());
    }
    nbuild_kernel_patch_u32_le(bytes, disp_offset, rel as i32 as u32, what)
}

fn nbuild_kernel_x86_real_mode_offset(
    absolute_address: u64,
    what: &str,
) -> Result<u16, Box<dyn std::error::Error>> {
    if absolute_address >= 0x1_0000 {
        return Err(actionable_command_error(
            "nbuild",
            format!("{what} must remain below 64 KiB for BIOS real-mode stage1"),
        )
        .into());
    }
    Ok(absolute_address as u16)
}

fn nbuild_kernel_expect_u16_le(
    bytes: &[u8],
    offset: usize,
    expected: u16,
    what: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let actual = nbuild_kernel_read_u16_le(bytes, offset).ok_or_else(|| {
        actionable_command_error(
            "nbuild",
            format!("internal stage1 template cannot read {what}"),
        )
    })?;
    if actual != expected {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "internal stage1 template {what} moved unexpectedly: expected 0x{expected:04x}, got 0x{actual:04x}"
            ),
        )
        .into());
    }
    Ok(())
}

fn nbuild_kernel_memory_kind_label(kind: u32) -> &'static str {
    match kind {
        NBUILD_MEM_KIND_RESERVED => "reserved",
        NBUILD_MEM_KIND_LOADER => "loader",
        NBUILD_MEM_KIND_KERNEL_FILE => "kernel_elf_file_image",
        NBUILD_MEM_KIND_KERNEL_SEGMENT => "kernel_loaded_segment",
        NBUILD_MEM_KIND_STACK => "initial_stack",
        NBUILD_MEM_KIND_USABLE => "firmware_usable",
        NBUILD_MEM_KIND_ACPI_RECLAIM => "acpi_reclaim",
        NBUILD_MEM_KIND_ACPI_NVS => "acpi_nvs",
        NBUILD_MEM_KIND_BAD => "bad_memory",
        NBUILD_MEM_KIND_RUNTIME => "firmware_runtime",
        NBUILD_MEM_KIND_MMIO => "mmio",
        NBUILD_MEM_KIND_PERSISTENT => "persistent_memory",
        _ => "unknown",
    }
}

fn nbuild_kernel_memory_source_label(source: u32) -> &'static str {
    match source {
        NBUILD_MEM_SOURCE_OWN_BOOT => "ncc-boot-v1-stage1",
        NBUILD_MEM_SOURCE_ELF_PHDR => "elf64-program-header",
        NBUILD_MEM_SOURCE_BIOS_E820 => "bios-int15-e820",
        NBUILD_MEM_SOURCE_UEFI => "uefi-memory-map",
        NBUILD_MEM_SOURCE_MULTIBOOT2 => "multiboot2-memory-map",
        NBUILD_MEM_SOURCE_FDT => "fdt-memory-nodes",
        _ => "unknown",
    }
}

fn nbuild_kernel_platform_kind_label(kind: u32) -> &'static str {
    match kind {
        NBUILD_PLATFORM_KIND_NCC_BOOT_V1_OWN_CONTRACT => "ncc_boot_v1_own_contract",
        NBUILD_PLATFORM_KIND_ACPI_RSDP => "acpi_rsdp",
        NBUILD_PLATFORM_KIND_UEFI_SYSTEM_TABLE => "uefi_system_table",
        NBUILD_PLATFORM_KIND_MULTIBOOT2_TAGS => "multiboot2_tags",
        NBUILD_PLATFORM_KIND_BIOS_E820 => "bios_e820",
        NBUILD_PLATFORM_KIND_FDT => "fdt",
        NBUILD_PLATFORM_KIND_OPENSBI_HART => "opensbi_hart",
        NBUILD_PLATFORM_KIND_WASM_HOST_IMPORTS => "wasm_host_imports",
        NBUILD_PLATFORM_KIND_SYSTEM_LOADER_PROCESS => "system_loader_process_contract",
        NBUILD_PLATFORM_KIND_CPU_TOPOLOGY => "cpu_topology",
        NBUILD_PLATFORM_KIND_SMBIOS32 => "smbios32",
        NBUILD_PLATFORM_KIND_SMBIOS64 => "smbios64",
        NBUILD_PLATFORM_KIND_SMBIOS_TABLES => "smbios_tables",
        NBUILD_PLATFORM_KIND_MULTIBOOT2_BOOT_DEVICE => "multiboot2_boot_device",
        NBUILD_PLATFORM_KIND_UEFI_MEMORY_MAP => "uefi_memory_map",
        NBUILD_PLATFORM_KIND_UEFI_IMAGE_HANDLE => "uefi_image_handle",
        NBUILD_PLATFORM_KIND_MULTIBOOT2_LOAD_BASE => "multiboot2_load_base",
        NBUILD_PLATFORM_KIND_MULTIBOOT2_VBE => "multiboot2_vbe",
        NBUILD_PLATFORM_KIND_MULTIBOOT2_ELF_SECTIONS => "multiboot2_elf_sections",
        NBUILD_PLATFORM_KIND_MULTIBOOT2_APM => "multiboot2_apm",
        NBUILD_PLATFORM_KIND_MULTIBOOT2_NETWORK => "multiboot2_network",
        NBUILD_PLATFORM_KIND_UEFI_BOOT_SERVICES => "uefi_boot_services",
        NBUILD_PLATFORM_KIND_MULTIBOOT2_FRAMEBUFFER => "multiboot2_framebuffer",
        _ => "unknown",
    }
}

fn nbuild_kernel_platform_flags_json(flags: u32) -> serde_json::Value {
    serde_json::json!({
        "raw": flags,
        "present": flags & NBUILD_PLATFORM_FLAG_PRESENT != 0,
        "required": flags & NBUILD_PLATFORM_FLAG_REQUIRED != 0,
        "own_contract": flags & NBUILD_PLATFORM_FLAG_OWN_CONTRACT != 0,
        "external_adapter": flags & NBUILD_PLATFORM_FLAG_EXTERNAL_ADAPTER != 0,
        "zero_ptr_if_absent": flags & NBUILD_PLATFORM_FLAG_ZERO_PTR_IF_ABSENT != 0,
        "runtime_probe": flags & NBUILD_PLATFORM_FLAG_RUNTIME_PROBE != 0,
        "dynamic_length": flags & NBUILD_PLATFORM_FLAG_DYNAMIC_LENGTH != 0,
    })
}

fn nbuild_kernel_memory_flags_json(flags: u32) -> serde_json::Value {
    serde_json::json!({
        "raw": flags,
        "present": flags & NBUILD_MEM_FLAG_PRESENT != 0,
        "read": flags & NBUILD_MEM_FLAG_READ != 0,
        "write": flags & NBUILD_MEM_FLAG_WRITE != 0,
        "execute": flags & NBUILD_MEM_FLAG_EXECUTE != 0,
        "identity_mapped": flags & NBUILD_MEM_FLAG_IDENTITY_MAPPED != 0,
    })
}

fn nbuild_kernel_append_memory_region(buf: &mut Vec<u8>, region: &NbuildKernelMemoryRegionV1) {
    nbuild_kernel_push_u64_le(buf, region.base);
    nbuild_kernel_push_u64_le(buf, region.length);
    nbuild_kernel_push_u32_le(buf, region.kind);
    nbuild_kernel_push_u32_le(buf, region.flags);
    nbuild_kernel_push_u32_le(buf, region.source);
    nbuild_kernel_push_u32_le(buf, 0);
}

fn nbuild_kernel_append_platform_entry(
    buf: &mut Vec<u8>,
    kind: u32,
    flags: u32,
    ptr: u64,
    length: u64,
    checksum_or_revision: u64,
) {
    nbuild_kernel_push_u32_le(buf, kind);
    nbuild_kernel_push_u32_le(buf, flags);
    nbuild_kernel_push_u64_le(buf, ptr);
    nbuild_kernel_push_u64_le(buf, length);
    nbuild_kernel_push_u64_le(buf, checksum_or_revision);
}

fn nbuild_kernel_parse_platform_entries_from_stage1(
    stage1: &[u8],
    platform_offset: usize,
    entry_count: u32,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let mut entries = Vec::new();
    let entries_base = platform_offset + 16;
    for idx in 0..entry_count as usize {
        let base = entries_base + idx * NBUILD_KERNEL_PLATFORM_ENTRY_SIZE;
        let kind = nbuild_kernel_read_u32_le(stage1, base).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 platform entry kind is truncated")
        })?;
        let flags = nbuild_kernel_read_u32_le(stage1, base + 4).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 platform entry flags are truncated")
        })?;
        let ptr = nbuild_kernel_read_u64_le(stage1, base + 8).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 platform entry ptr is truncated")
        })?;
        let length = nbuild_kernel_read_u64_le(stage1, base + 16).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 platform entry length is truncated")
        })?;
        let checksum_or_revision =
            nbuild_kernel_read_u64_le(stage1, base + 24).ok_or_else(|| {
                actionable_command_error(
                    "nbuild",
                    "stage1 platform entry checksum/revision is truncated",
                )
            })?;
        let present = flags & NBUILD_PLATFORM_FLAG_PRESENT != 0;
        let runtime_probe = flags & NBUILD_PLATFORM_FLAG_RUNTIME_PROBE != 0;
        let zero_ptr_if_absent = flags & NBUILD_PLATFORM_FLAG_ZERO_PTR_IF_ABSENT != 0;
        let runtime_external_probe = runtime_probe && zero_ptr_if_absent && ptr == 0 && length == 0;
        if !present && !runtime_probe && (ptr != 0 || length != 0) {
            return Err(actionable_command_error(
                "nbuild",
                "absent stage1 platform entries must retain zero ptr and zero length",
            )
            .into());
        }
        if runtime_probe && !runtime_external_probe && (ptr == 0 || length == 0) {
            return Err(actionable_command_error(
                "nbuild",
                "runtime-probed stage1 platform entries must retain a real target buffer",
            )
            .into());
        }
        entries.push(serde_json::json!({
            "kind": nbuild_kernel_platform_kind_label(kind),
            "kind_code": kind,
            "flags": nbuild_kernel_platform_flags_json(flags),
            "ptr": nbuild_kernel_hex_u64(ptr),
            "length": length,
            "checksum_or_revision": checksum_or_revision,
            "status": if present {
                "present"
            } else if runtime_external_probe {
                "runtime_probe_external"
            } else if runtime_probe {
                "runtime_probe_buffer"
            } else {
                "absent"
            },
            "absent_reason": if present {
                "present"
            } else if runtime_external_probe {
                "filled by stage1 at boot if firmware exposes the table"
            } else if runtime_probe {
                "filled by stage1 at boot if firmware returns data"
            } else {
                "adapter was not the active boot source for this BIOS stage1 image"
            },
        }));
    }
    Ok(entries)
}

fn nbuild_kernel_x86_boot_memory_regions(
    kernel_elf: &[u8],
    load_segments: &[NbuildKernelElf64LoadSegment],
) -> Result<Vec<NbuildKernelMemoryRegionV1>, Box<dyn std::error::Error>> {
    let kernel_file_len = nbuild_kernel_align_up_u64(kernel_elf.len() as u64, 512);
    let kernel_file_end = NBUILD_KERNEL_X86_KERNEL_ELF_MEMORY_ADDRESS
        .checked_add(kernel_file_len)
        .ok_or_else(|| actionable_command_error("nbuild", "kernel ELF memory range overflows"))?;
    if kernel_file_end > NBUILD_KERNEL_X86_BOOT_STACK_LOW {
        return Err(actionable_command_error(
            "nbuild",
            "kernel ELF boot payload overlaps the initial x86 bootstrap stack window",
        )
        .into());
    }

    let mut regions = vec![
        NbuildKernelMemoryRegionV1 {
            base: 0x0000,
            length: 0x0500,
            kind: NBUILD_MEM_KIND_RESERVED,
            flags: NBUILD_MEM_FLAG_PRESENT,
            source: NBUILD_MEM_SOURCE_OWN_BOOT,
        },
        NbuildKernelMemoryRegionV1 {
            base: 0x7c00,
            length: NBUILD_KERNEL_SECTOR_SIZE as u64,
            kind: NBUILD_MEM_KIND_LOADER,
            flags: NBUILD_MEM_FLAG_PRESENT
                | NBUILD_MEM_FLAG_READ
                | NBUILD_MEM_FLAG_EXECUTE
                | NBUILD_MEM_FLAG_IDENTITY_MAPPED,
            source: NBUILD_MEM_SOURCE_OWN_BOOT,
        },
        NbuildKernelMemoryRegionV1 {
            base: NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS,
            length: NBUILD_KERNEL_X86_STAGE1_RESERVED_BYTES as u64,
            kind: NBUILD_MEM_KIND_LOADER,
            flags: NBUILD_MEM_FLAG_PRESENT
                | NBUILD_MEM_FLAG_READ
                | NBUILD_MEM_FLAG_WRITE
                | NBUILD_MEM_FLAG_EXECUTE
                | NBUILD_MEM_FLAG_IDENTITY_MAPPED,
            source: NBUILD_MEM_SOURCE_OWN_BOOT,
        },
        NbuildKernelMemoryRegionV1 {
            base: NBUILD_KERNEL_X86_KERNEL_ELF_MEMORY_ADDRESS,
            length: kernel_file_len,
            kind: NBUILD_MEM_KIND_KERNEL_FILE,
            flags: NBUILD_MEM_FLAG_PRESENT
                | NBUILD_MEM_FLAG_READ
                | NBUILD_MEM_FLAG_WRITE
                | NBUILD_MEM_FLAG_IDENTITY_MAPPED,
            source: NBUILD_MEM_SOURCE_OWN_BOOT,
        },
        NbuildKernelMemoryRegionV1 {
            base: NBUILD_KERNEL_X86_BOOT_STACK_LOW,
            length: NBUILD_KERNEL_X86_BOOT_STACK_SIZE,
            kind: NBUILD_MEM_KIND_STACK,
            flags: NBUILD_MEM_FLAG_PRESENT
                | NBUILD_MEM_FLAG_READ
                | NBUILD_MEM_FLAG_WRITE
                | NBUILD_MEM_FLAG_IDENTITY_MAPPED,
            source: NBUILD_MEM_SOURCE_OWN_BOOT,
        },
    ];

    for segment in load_segments {
        let mut flags = NBUILD_MEM_FLAG_PRESENT | NBUILD_MEM_FLAG_IDENTITY_MAPPED;
        if segment.flags & 0x4 != 0 {
            flags |= NBUILD_MEM_FLAG_READ;
        }
        if segment.flags & 0x2 != 0 {
            flags |= NBUILD_MEM_FLAG_WRITE;
        }
        if segment.flags & 0x1 != 0 {
            flags |= NBUILD_MEM_FLAG_EXECUTE;
        }
        regions.push(NbuildKernelMemoryRegionV1 {
            base: segment.paddr,
            length: nbuild_kernel_align_up_u64(segment.memsz, 0x1000),
            kind: NBUILD_MEM_KIND_KERNEL_SEGMENT,
            flags,
            source: NBUILD_MEM_SOURCE_ELF_PHDR,
        });
    }

    Ok(regions)
}

fn nbuild_kernel_parse_memory_regions_from_stage1(
    stage1: &[u8],
    memory_map_address: u64,
    memory_map_len: u64,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    if memory_map_address < NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS {
        return Err(actionable_command_error(
            "nbuild",
            "stage1 boot_info memory_map_ptr is below the stage1 load address",
        )
        .into());
    }
    let offset = (memory_map_address - NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS) as usize;
    let mut entries = Vec::new();
    for idx in 0..memory_map_len as usize {
        let base = offset + idx * NBUILD_KERNEL_X86_MEMORY_REGION_SIZE;
        let region_base = nbuild_kernel_read_u64_le(stage1, base).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 memory map region base is truncated")
        })?;
        let length = nbuild_kernel_read_u64_le(stage1, base + 8).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 memory map region length is truncated")
        })?;
        let kind = nbuild_kernel_read_u32_le(stage1, base + 16).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 memory map region kind is truncated")
        })?;
        let flags = nbuild_kernel_read_u32_le(stage1, base + 20).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 memory map region flags are truncated")
        })?;
        let source = nbuild_kernel_read_u32_le(stage1, base + 24).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 memory map region source is truncated")
        })?;
        entries.push(serde_json::json!({
            "base": nbuild_kernel_hex_u64(region_base),
            "length": length,
            "end": nbuild_kernel_hex_u64(region_base.saturating_add(length)),
            "kind": nbuild_kernel_memory_kind_label(kind),
            "kind_code": kind,
            "flags": nbuild_kernel_memory_flags_json(flags),
            "source": nbuild_kernel_memory_source_label(source),
            "source_code": source,
        }));
    }
    Ok(entries)
}

fn nbuild_kernel_elf64_image_info(
    bytes: &[u8],
) -> Result<NbuildKernelElf64ImageInfo, Box<dyn std::error::Error>> {
    nbuild_kernel_elf64_image_info_for_machine(bytes, 62, "x86_64")
}

fn nbuild_kernel_elf64_image_info_for_machine(
    bytes: &[u8],
    expected_machine: u16,
    target_label: &str,
) -> Result<NbuildKernelElf64ImageInfo, Box<dyn std::error::Error>> {
    if bytes.get(0..4) != Some(b"\x7fELF")
        || bytes.get(4).copied() != Some(2)
        || bytes.get(5).copied() != Some(1)
    {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "{target_label} kernel boot image requires an ELF64 little-endian executable payload"
            ),
        )
        .into());
    }
    let elf_type = nbuild_kernel_read_u16_le(bytes, 16)
        .ok_or_else(|| actionable_command_error("nbuild", "ELF64 payload is missing e_type"))?;
    let machine = nbuild_kernel_read_u16_le(bytes, 18)
        .ok_or_else(|| actionable_command_error("nbuild", "ELF64 payload is missing e_machine"))?;
    if elf_type != 2 || machine != expected_machine {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "{target_label} kernel boot image requires ELF64 ET_EXEC/e_machine={expected_machine}; got e_type={elf_type} e_machine={machine}"
            ),
        )
        .into());
    }
    let entry = nbuild_kernel_read_u64_le(bytes, 24)
        .ok_or_else(|| actionable_command_error("nbuild", "ELF64 payload is missing e_entry"))?;
    let phoff = nbuild_kernel_read_u64_le(bytes, 32)
        .ok_or_else(|| actionable_command_error("nbuild", "ELF64 payload is missing e_phoff"))?;
    let phentsize = nbuild_kernel_read_u16_le(bytes, 54).ok_or_else(|| {
        actionable_command_error("nbuild", "ELF64 payload is missing e_phentsize")
    })?;
    let phnum = nbuild_kernel_read_u16_le(bytes, 56)
        .ok_or_else(|| actionable_command_error("nbuild", "ELF64 payload is missing e_phnum"))?;
    if phentsize < 56 || phnum == 0 {
        return Err(actionable_command_error(
            "nbuild",
            "ELF64 kernel payload must retain at least one valid program header",
        )
        .into());
    }

    let mut load_segments = Vec::new();
    for idx in 0..phnum as usize {
        let base = phoff as usize + idx * phentsize as usize;
        let p_type = nbuild_kernel_read_u32_le(bytes, base).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF64 program header is truncated")
        })?;
        if p_type != 1 {
            continue;
        }
        let flags = nbuild_kernel_read_u32_le(bytes, base + 4).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF64 program header flags are truncated")
        })?;
        let offset = nbuild_kernel_read_u64_le(bytes, base + 8).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF64 program header offset is truncated")
        })?;
        let vaddr = nbuild_kernel_read_u64_le(bytes, base + 16).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF64 program header vaddr is truncated")
        })?;
        let paddr = nbuild_kernel_read_u64_le(bytes, base + 24).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF64 program header paddr is truncated")
        })?;
        let filesz = nbuild_kernel_read_u64_le(bytes, base + 32).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF64 program header filesz is truncated")
        })?;
        let memsz = nbuild_kernel_read_u64_le(bytes, base + 40).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF64 program header memsz is truncated")
        })?;
        let align = nbuild_kernel_read_u64_le(bytes, base + 48).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF64 program header align is truncated")
        })?;
        if filesz > memsz {
            return Err(actionable_command_error(
                "nbuild",
                "ELF64 kernel payload has PT_LOAD filesz greater than memsz",
            )
            .into());
        }
        let end = offset.saturating_add(filesz) as usize;
        if end > bytes.len() {
            return Err(actionable_command_error(
                "nbuild",
                "ELF64 kernel payload has a PT_LOAD segment outside the file",
            )
            .into());
        }
        load_segments.push(NbuildKernelElf64LoadSegment {
            offset,
            vaddr,
            paddr,
            filesz,
            memsz,
            flags,
            align,
        });
    }
    if load_segments.is_empty() {
        return Err(actionable_command_error(
            "nbuild",
            "ELF64 kernel payload must contain at least one PT_LOAD segment",
        )
        .into());
    }

    Ok(NbuildKernelElf64ImageInfo {
        entry,
        phoff,
        phentsize,
        phnum,
        load_segments,
    })
}

fn nbuild_kernel_elf32_image_info(
    bytes: &[u8],
) -> Result<NbuildKernelElf32ImageInfo, Box<dyn std::error::Error>> {
    nbuild_kernel_elf32_image_info_for_machine(bytes, 3, "i386")
}

fn nbuild_kernel_elf32_image_info_for_machine(
    bytes: &[u8],
    expected_machine: u16,
    target_label: &str,
) -> Result<NbuildKernelElf32ImageInfo, Box<dyn std::error::Error>> {
    if bytes.get(0..4) != Some(b"\x7fELF")
        || bytes.get(4).copied() != Some(1)
        || bytes.get(5).copied() != Some(1)
    {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "{target_label} kernel boot image requires an ELF32 little-endian executable payload"
            ),
        )
        .into());
    }
    let elf_type = nbuild_kernel_read_u16_le(bytes, 16)
        .ok_or_else(|| actionable_command_error("nbuild", "ELF32 payload is missing e_type"))?;
    let machine = nbuild_kernel_read_u16_le(bytes, 18)
        .ok_or_else(|| actionable_command_error("nbuild", "ELF32 payload is missing e_machine"))?;
    if elf_type != 2 || machine != expected_machine {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "{target_label} kernel boot image requires ELF32 ET_EXEC/e_machine={expected_machine}; got e_type={elf_type} e_machine={machine}"
            ),
        )
        .into());
    }
    let entry = nbuild_kernel_read_u32_le(bytes, 24)
        .ok_or_else(|| actionable_command_error("nbuild", "ELF32 payload is missing e_entry"))?;
    let phoff = nbuild_kernel_read_u32_le(bytes, 28)
        .ok_or_else(|| actionable_command_error("nbuild", "ELF32 payload is missing e_phoff"))?;
    let phentsize = nbuild_kernel_read_u16_le(bytes, 42).ok_or_else(|| {
        actionable_command_error("nbuild", "ELF32 payload is missing e_phentsize")
    })?;
    let phnum = nbuild_kernel_read_u16_le(bytes, 44)
        .ok_or_else(|| actionable_command_error("nbuild", "ELF32 payload is missing e_phnum"))?;
    if phentsize < 32 || phnum == 0 {
        return Err(actionable_command_error(
            "nbuild",
            "ELF32 kernel payload must retain at least one valid program header",
        )
        .into());
    }

    let mut load_segments = Vec::new();
    for idx in 0..phnum as usize {
        let base = phoff as usize + idx * phentsize as usize;
        let p_type = nbuild_kernel_read_u32_le(bytes, base).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF32 program header is truncated")
        })?;
        if p_type != 1 {
            continue;
        }
        let offset = nbuild_kernel_read_u32_le(bytes, base + 4).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF32 program header offset is truncated")
        })? as u64;
        let vaddr = nbuild_kernel_read_u32_le(bytes, base + 8).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF32 program header vaddr is truncated")
        })? as u64;
        let paddr = nbuild_kernel_read_u32_le(bytes, base + 12).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF32 program header paddr is truncated")
        })? as u64;
        let filesz = nbuild_kernel_read_u32_le(bytes, base + 16).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF32 program header filesz is truncated")
        })? as u64;
        let memsz = nbuild_kernel_read_u32_le(bytes, base + 20).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF32 program header memsz is truncated")
        })? as u64;
        let flags = nbuild_kernel_read_u32_le(bytes, base + 24).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF32 program header flags are truncated")
        })?;
        let align = nbuild_kernel_read_u32_le(bytes, base + 28).ok_or_else(|| {
            actionable_command_error("nbuild", "ELF32 program header align is truncated")
        })? as u64;
        if filesz > memsz {
            return Err(actionable_command_error(
                "nbuild",
                "ELF32 kernel payload has PT_LOAD filesz greater than memsz",
            )
            .into());
        }
        let end = offset.saturating_add(filesz) as usize;
        if end > bytes.len() {
            return Err(actionable_command_error(
                "nbuild",
                "ELF32 kernel payload has a PT_LOAD segment outside the file",
            )
            .into());
        }
        load_segments.push(NbuildKernelElf64LoadSegment {
            offset,
            vaddr,
            paddr,
            filesz,
            memsz,
            flags,
            align,
        });
    }
    if load_segments.is_empty() {
        return Err(actionable_command_error(
            "nbuild",
            "ELF32 kernel payload must contain at least one PT_LOAD segment",
        )
        .into());
    }

    Ok(NbuildKernelElf32ImageInfo {
        entry,
        phoff,
        phentsize,
        phnum,
        load_segments,
    })
}

fn nbuild_kernel_x86_real_mode_acpi_e820_probe_bytes(
) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
    let source = nbuild_kernel_x86_64_stage1_acpi_e820_template_bytes();
    let acpi_marker = [
        0x60, 0x1e, 0x06, 0x31, 0xc0, 0x8e, 0xd8, 0xc7, 0x06, 0x92, 0x84,
    ];
    let e820_marker = [
        0x60, 0x1e, 0x06, 0x31, 0xc0, 0x8e, 0xd8, 0x8e, 0xc0, 0x66, 0x31, 0xdb,
    ];
    let protected_entry_marker = [
        0x66, 0xb8, 0x10, 0x00, 0x8e, 0xd8, 0x8e, 0xc0, 0x8e, 0xd0, 0xbc, 0x00, 0x00, 0x09, 0x00,
    ];

    let acpi_start = source
        .windows(acpi_marker.len())
        .position(|window| window == acpi_marker)
        .ok_or_else(|| {
            actionable_command_error(
                "nbuild",
                "internal x86 real-mode ACPI probe bytes are missing",
            )
        })?;
    let e820_start = source[acpi_start + acpi_marker.len()..]
        .windows(e820_marker.len())
        .position(|window| window == e820_marker)
        .map(|idx| acpi_start + acpi_marker.len() + idx)
        .ok_or_else(|| {
            actionable_command_error(
                "nbuild",
                "internal x86 real-mode E820 probe bytes are missing",
            )
        })?;
    let probe_end = source[e820_start + e820_marker.len()..]
        .windows(protected_entry_marker.len())
        .position(|window| window == protected_entry_marker)
        .map(|idx| e820_start + e820_marker.len() + idx)
        .ok_or_else(|| {
            actionable_command_error(
                "nbuild",
                "internal x86 real-mode probe block end marker is missing",
            )
        })?;

    Ok((
        source[acpi_start..e820_start].to_vec(),
        source[e820_start..probe_end].to_vec(),
    ))
}

fn nbuild_kernel_i386_stage1_template_bytes() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::<u8>::new();

    bytes.extend([
        0xfa, 0x31, 0xc0, 0x8e, 0xd8, 0x8e, 0xc0, 0x8e, 0xd0, 0xbc, 0x00, 0x80,
    ]);
    bytes.push(0xe8);
    let call_serial_init16 = bytes.len();
    bytes.extend([0x00, 0x00]);
    bytes.push(0xe8);
    let call_acpi_probe16 = bytes.len();
    bytes.extend([0x00, 0x00]);
    bytes.push(0xe8);
    let call_e820_probe16 = bytes.len();
    bytes.extend([0x00, 0x00]);
    bytes.extend([0x0f, 0x01, 0x16]);
    let lgdt_operand = bytes.len();
    bytes.extend([0x00, 0x00]);
    bytes.extend([
        0x0f, 0x20, 0xc0, 0x66, 0x83, 0xc8, 0x01, 0x0f, 0x22, 0xc0, 0xea,
    ]);
    let pm_jump_offset = bytes.len();
    bytes.extend([0x00, 0x00, 0x08, 0x00]);

    let serial_init16 = bytes.len();
    bytes.extend([
        0xba, 0xf9, 0x03, 0x30, 0xc0, 0xee, 0xba, 0xfb, 0x03, 0xb0, 0x80, 0xee, 0xba, 0xf8, 0x03,
        0xb0, 0x03, 0xee, 0xba, 0xf9, 0x03, 0x30, 0xc0, 0xee, 0xba, 0xfb, 0x03, 0xb0, 0x03, 0xee,
        0xba, 0xfa, 0x03, 0xb0, 0xc7, 0xee, 0xba, 0xfc, 0x03, 0xb0, 0x0b, 0xee, 0xc3,
    ]);

    let (acpi_probe16_bytes, e820_probe16_bytes) =
        nbuild_kernel_x86_real_mode_acpi_e820_probe_bytes()?;
    let acpi_probe16 = bytes.len();
    bytes.extend_from_slice(&acpi_probe16_bytes);
    let e820_probe16 = bytes.len();
    bytes.extend_from_slice(&e820_probe16_bytes);

    let pm_entry = bytes.len();
    bytes.extend([
        0x66, 0xb8, 0x10, 0x00, 0x8e, 0xd8, 0x8e, 0xc0, 0x8e, 0xd0, 0xbc, 0x00, 0x00, 0x09, 0x00,
    ]);
    // i686 code generation may legally use x87/SSE before kmain. Clear EM/TS,
    // enable native exception handling and expose the SSE state to the CPU.
    bytes.extend([
        0x0f, 0x20, 0xc0, // mov eax, cr0
        0x83, 0xe0, 0xf3, // and eax, ~(CR0_EM | CR0_TS)
        0x83, 0xc8, 0x22, // or eax, CR0_MP | CR0_NE
        0x0f, 0x22, 0xc0, // mov cr0, eax
        0x0f, 0x20, 0xe0, // mov eax, cr4
        0x0d, 0x00, 0x06, 0x00, 0x00, // or eax, CR4_OSFXSR | CR4_OSXMMEXCPT
        0x0f, 0x22, 0xe0, // mov cr4, eax
        0xdb, 0xe3, // fninit
    ]);
    bytes.extend([0x83, 0x3d]);
    nbuild_kernel_push_u32_le(
        &mut bytes,
        (NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + NBUILD_KERNEL_X86_STAGE1_ACPI_FOUND_OFFSET as u64)
            as u32,
    );
    bytes.extend([0x00, 0x0f, 0x84]);
    let je_skip_acpi_msg = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00, 0xbe]);
    let msg_acpi_imm = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00, 0xe8]);
    let call_puts_acpi = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00]);
    let skip_acpi_msg = bytes.len();
    bytes.extend([0x83, 0x3d]);
    nbuild_kernel_push_u32_le(
        &mut bytes,
        (NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS
            + NBUILD_KERNEL_X86_STAGE1_E820_RUNTIME_COUNT_OFFSET as u64) as u32,
    );
    bytes.extend([0x00, 0x0f, 0x84]);
    let je_skip_e820_msg = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00, 0xbe]);
    let msg_e820_imm = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00, 0xe8]);
    let call_puts_e820 = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00]);
    let skip_e820_msg = bytes.len();
    bytes.push(0xe8);
    let call_load_elf32 = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00]);
    bytes.extend([0x85, 0xc0, 0x0f, 0x84]);
    let je_bad_after_load = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00, 0x68]);
    let boot_info_push_imm = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00, 0xff, 0xd0, 0x83, 0xc4, 0x04]);
    bytes.extend([0x3d, 0x4b, 0x4f, 0x00, 0x00, 0x0f, 0x85]);
    let jne_bad_after_kmain = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00, 0xbe]);
    let msg_ok_imm = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00, 0xe8]);
    let call_puts_ok = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00, 0xf4, 0xeb, 0xfd]);

    let bad_label = bytes.len();
    bytes.push(0xbe);
    let msg_bad_imm = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00, 0xe8]);
    let call_puts_bad = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00, 0xf4, 0xeb, 0xfd]);

    let load_elf32 = bytes.len();
    bytes.extend([
        0xbb, 0x00, 0x90, 0x00, 0x00, 0x81, 0x3b, 0x7f, 0x45, 0x4c, 0x46, 0x0f, 0x85,
    ]);
    let load_fail_magic = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00, 0x80, 0x7b, 0x04, 0x01, 0x0f, 0x85]);
    let load_fail_class = bytes.len();
    bytes.extend([
        0x00, 0x00, 0x00, 0x00, 0x8b, 0x6b, 0x1c, 0x01, 0xdd, 0x0f, 0xb7, 0x4b, 0x2c,
    ]);
    let load_loop = bytes.len();
    bytes.extend([0x85, 0xc9, 0x0f, 0x84]);
    let load_done_jump = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00, 0x83, 0x7d, 0x00, 0x01, 0x0f, 0x85]);
    let load_next_jump = bytes.len();
    bytes.extend([
        0x00, 0x00, 0x00, 0x00, 0x51, 0x55, 0x8b, 0x75, 0x04, 0x01, 0xde, 0x8b, 0x7d, 0x0c, 0x8b,
        0x4d, 0x10, 0x85, 0xc9, 0x0f, 0x84,
    ]);
    let load_copy_done_jump_empty = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00, 0x39, 0xf7, 0x0f, 0x86]);
    let load_copy_forward_jump = bytes.len();
    bytes.extend([
        0x00, 0x00, 0x00, 0x00, 0x8d, 0x74, 0x0e, 0xff, 0x8d, 0x7c, 0x0f, 0xff, 0xfd, 0xf3, 0xa4,
        0xfc, 0xe9,
    ]);
    let load_copy_done_jump_backward = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00]);
    let load_copy_forward = bytes.len();
    bytes.extend([0xfc, 0xf3, 0xa4]);
    let load_copy_done = bytes.len();
    bytes.extend([
        0x8b, 0x7d, 0x0c, 0x03, 0x7d, 0x10, 0x8b, 0x4d, 0x14, 0x2b, 0x4d, 0x10, 0x31, 0xc0, 0xfc,
        0xf3, 0xaa, 0x5d, 0x59,
    ]);
    let load_next = bytes.len();
    bytes.extend([0x83, 0xc5, 0x20, 0x49, 0xe9]);
    let load_loop_back = bytes.len();
    bytes.extend([0x00, 0x00, 0x00, 0x00]);
    let load_done = bytes.len();
    bytes.extend([0x8b, 0x43, 0x18, 0xc3]);
    let load_fail = bytes.len();
    bytes.extend([0x31, 0xc0, 0xc3]);

    let serial_puts32 = bytes.len();
    bytes.extend([
        0xac, 0x84, 0xc0, 0x74, 0x14, 0x88, 0xc4, 0x66, 0xba, 0xfd, 0x03, 0xec, 0xa8, 0x20, 0x74,
        0xf7, 0x88, 0xe0, 0x66, 0xba, 0xf8, 0x03, 0xee, 0xeb, 0xe7, 0xc3,
    ]);

    let msg_ok = bytes.len();
    bytes.extend_from_slice(b"nC kernel i386 ELF kmain\r\n\0");
    let msg_bad = bytes.len();
    bytes.extend_from_slice(b"nC kernel i386 ELF kmain bad\r\n\0");
    let msg_e820 = bytes.len();
    bytes.extend_from_slice(b"nC kernel i386 E820 ok\r\n\0");
    let msg_acpi = bytes.len();
    bytes.extend_from_slice(b"nC kernel i386 ACPI RSDP ok\r\n\0");

    if bytes.len() > NBUILD_KERNEL_X86_STAGE1_ACPI_PLATFORM_SLOT_PTR_OFFSET {
        return Err(actionable_command_error(
            "nbuild",
            "internal i386 stage1 real-mode code overlaps the ACPI/E820 probe state block",
        )
        .into());
    }
    bytes.resize(NBUILD_KERNEL_X86_STAGE1_ACPI_PLATFORM_SLOT_PTR_OFFSET, 0x00);
    nbuild_kernel_push_u16_le(&mut bytes, 0xcafe);
    nbuild_kernel_push_u16_le(&mut bytes, 0x0000);
    nbuild_kernel_push_u16_le(&mut bytes, 0x0000);
    nbuild_kernel_push_u16_le(&mut bytes, 0x0000);
    nbuild_kernel_push_u16_le(&mut bytes, 0xcafe);
    nbuild_kernel_push_u16_le(&mut bytes, 0x0000);
    nbuild_kernel_push_u16_le(&mut bytes, 0xcafe);
    nbuild_kernel_push_u16_le(&mut bytes, 0xcafe);
    nbuild_kernel_push_u16_le(&mut bytes, 0x0000);
    nbuild_kernel_push_u16_le(&mut bytes, 0x0000);
    bytes.resize(
        NBUILD_KERNEL_X86_STAGE1_REAL_MODE_PROBE_STATE_END_OFFSET,
        0x00,
    );

    nbuild_kernel_align_vec(&mut bytes, 8);
    let gdt = bytes.len();
    bytes.extend([0x00; 8]);
    bytes.extend([0xff, 0xff, 0x00, 0x00, 0x00, 0x9a, 0xcf, 0x00]);
    bytes.extend([0xff, 0xff, 0x00, 0x00, 0x00, 0x92, 0xcf, 0x00]);
    let gdt_desc = bytes.len();
    nbuild_kernel_push_u16_le(&mut bytes, 23);
    nbuild_kernel_push_u32_le(
        &mut bytes,
        (NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + gdt as u64) as u32,
    );

    nbuild_kernel_align_vec(&mut bytes, 8);
    let boot_info = bytes.len();
    bytes.extend_from_slice(b"NCCBOO1\0");
    nbuild_kernel_push_u64_le(&mut bytes, 1);
    nbuild_kernel_push_u64_le(&mut bytes, 0);
    nbuild_kernel_push_u64_le(&mut bytes, 0);
    nbuild_kernel_push_u64_le(&mut bytes, 0);
    nbuild_kernel_push_u64_le(&mut bytes, 0);
    nbuild_kernel_push_u64_le(&mut bytes, 0);
    nbuild_kernel_push_u64_le(&mut bytes, 0);
    nbuild_kernel_push_u64_le(&mut bytes, 0);
    nbuild_kernel_push_u64_le(&mut bytes, NBUILD_KERNEL_X86_KERNEL_ELF_MEMORY_ADDRESS);
    nbuild_kernel_push_u64_le(&mut bytes, NBUILD_KERNEL_X86_PAYLOAD_SIZE_MARKER);

    nbuild_kernel_patch_rel16(
        &mut bytes,
        call_serial_init16,
        serial_init16,
        "serial init call",
    )?;
    nbuild_kernel_patch_rel16(
        &mut bytes,
        call_acpi_probe16,
        acpi_probe16,
        "ACPI RSDP real-mode probe call",
    )?;
    nbuild_kernel_patch_rel16(
        &mut bytes,
        call_e820_probe16,
        e820_probe16,
        "BIOS E820 real-mode probe call",
    )?;
    nbuild_kernel_patch_u16_le(
        &mut bytes,
        lgdt_operand,
        nbuild_kernel_x86_real_mode_offset(
            NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + gdt_desc as u64,
            "i386 gdt descriptor",
        )?,
        "i386 lgdt operand",
    )?;
    nbuild_kernel_patch_u16_le(
        &mut bytes,
        pm_jump_offset,
        nbuild_kernel_x86_real_mode_offset(
            NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + pm_entry as u64,
            "i386 protected-mode entry",
        )?,
        "i386 protected-mode far jump",
    )?;
    nbuild_kernel_patch_rel32(&mut bytes, call_load_elf32, load_elf32, "load_elf32 call")?;
    nbuild_kernel_patch_rel32(&mut bytes, je_bad_after_load, bad_label, "bad load branch")?;
    nbuild_kernel_patch_u32_le(
        &mut bytes,
        boot_info_push_imm,
        (NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + boot_info as u64) as u32,
        "i386 boot_info push immediate",
    )?;
    nbuild_kernel_patch_rel32(
        &mut bytes,
        jne_bad_after_kmain,
        bad_label,
        "bad kmain branch",
    )?;
    nbuild_kernel_patch_u32_le(
        &mut bytes,
        msg_ok_imm,
        (NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + msg_ok as u64) as u32,
        "i386 ok message",
    )?;
    nbuild_kernel_patch_u32_le(
        &mut bytes,
        msg_bad_imm,
        (NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + msg_bad as u64) as u32,
        "i386 bad message",
    )?;
    nbuild_kernel_patch_u32_le(
        &mut bytes,
        msg_acpi_imm,
        (NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + msg_acpi as u64) as u32,
        "i386 ACPI RSDP marker message",
    )?;
    nbuild_kernel_patch_u32_le(
        &mut bytes,
        msg_e820_imm,
        (NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + msg_e820 as u64) as u32,
        "i386 E820 marker message",
    )?;
    nbuild_kernel_patch_rel32(
        &mut bytes,
        je_skip_acpi_msg,
        skip_acpi_msg,
        "skip ACPI RSDP marker branch",
    )?;
    nbuild_kernel_patch_rel32(
        &mut bytes,
        je_skip_e820_msg,
        skip_e820_msg,
        "skip E820 marker branch",
    )?;
    nbuild_kernel_patch_rel32(&mut bytes, call_puts_ok, serial_puts32, "puts ok call")?;
    nbuild_kernel_patch_rel32(&mut bytes, call_puts_bad, serial_puts32, "puts bad call")?;
    nbuild_kernel_patch_rel32(
        &mut bytes,
        call_puts_acpi,
        serial_puts32,
        "puts ACPI RSDP marker call",
    )?;
    nbuild_kernel_patch_rel32(
        &mut bytes,
        call_puts_e820,
        serial_puts32,
        "puts E820 marker call",
    )?;
    nbuild_kernel_patch_rel32(
        &mut bytes,
        load_fail_magic,
        load_fail,
        "ELF magic fail branch",
    )?;
    nbuild_kernel_patch_rel32(
        &mut bytes,
        load_fail_class,
        load_fail,
        "ELF class fail branch",
    )?;
    nbuild_kernel_patch_rel32(
        &mut bytes,
        load_done_jump,
        load_done,
        "ELF load done branch",
    )?;
    nbuild_kernel_patch_rel32(
        &mut bytes,
        load_next_jump,
        load_next,
        "ELF load next branch",
    )?;
    nbuild_kernel_patch_rel32(
        &mut bytes,
        load_copy_done_jump_empty,
        load_copy_done,
        "ELF empty segment copy branch",
    )?;
    nbuild_kernel_patch_rel32(
        &mut bytes,
        load_copy_forward_jump,
        load_copy_forward,
        "ELF forward segment copy branch",
    )?;
    nbuild_kernel_patch_rel32(
        &mut bytes,
        load_copy_done_jump_backward,
        load_copy_done,
        "ELF backward segment copy completion branch",
    )?;
    nbuild_kernel_patch_rel32(
        &mut bytes,
        load_loop_back,
        load_loop,
        "ELF phdr loop branch",
    )?;

    Ok(bytes)
}

fn nbuild_kernel_x86_64_stage1_acpi_e820_template_bytes() -> Vec<u8> {
    vec![
        0xfa, 0x31, 0xc0, 0x8e, 0xd8, 0x8e, 0xc0, 0x8e, 0xd0, 0xbc, 0x00, 0x80, 0xe8, 0x96, 0x00,
        0xe8, 0xbe, 0x00, 0xe8, 0xd4, 0x01, 0xe8, 0x39, 0x00, 0x0f, 0x01, 0x16, 0x31, 0x85, 0x0f,
        0x20, 0xe0, 0xb4, 0x06, 0x0c, 0x20, 0x0f, 0x22, 0xe0, 0x66, 0xb8, 0x00, 0x10, 0x00, 0x00,
        0x0f, 0x22, 0xd8, 0x66, 0xb9, 0x80, 0x00, 0x00, 0xc0, 0x0f, 0x32, 0x66, 0x0d, 0x00, 0x01,
        0x00, 0x00, 0x0f, 0x30, 0x0f, 0x20, 0xc0, 0x66, 0x0d, 0x01, 0x00, 0x00, 0x80, 0x0f, 0x22,
        0xc0, 0xea, 0x43, 0x83, 0x08, 0x00, 0x66, 0x31, 0xc0, 0xbf, 0x00, 0x10, 0xb9, 0x00, 0x18,
        0xfc, 0xf3, 0x66, 0xab, 0x66, 0xc7, 0x06, 0x00, 0x10, 0x03, 0x20, 0x00, 0x00, 0x66, 0xc7,
        0x06, 0x04, 0x10, 0x00, 0x00, 0x00, 0x00, 0xbf, 0x00, 0x20, 0x66, 0xb8, 0x03, 0x30, 0x00,
        0x00, 0xb9, 0x04, 0x00, 0x66, 0x89, 0x05, 0x83, 0xc7, 0x08, 0x66, 0x05, 0x00, 0x10, 0x00,
        0x00, 0xe2, 0xf2, 0xbf, 0x00, 0x30, 0x66, 0xb8, 0x83, 0x00, 0x00, 0x00, 0xb9, 0x00, 0x08,
        0x66, 0x89, 0x05, 0x83, 0xc7, 0x08, 0x66, 0x05, 0x00, 0x00, 0x20, 0x00, 0xe2, 0xf2, 0xc3,
        0xba, 0xf9, 0x03, 0x30, 0xc0, 0xee, 0xba, 0xfb, 0x03, 0xb0, 0x80, 0xee, 0xba, 0xf8, 0x03,
        0xb0, 0x03, 0xee, 0xba, 0xf9, 0x03, 0x30, 0xc0, 0xee, 0xba, 0xfb, 0x03, 0xb0, 0x03, 0xee,
        0xba, 0xfa, 0x03, 0xb0, 0xc7, 0xee, 0xba, 0xfc, 0x03, 0xb0, 0x0b, 0xee, 0xc3, 0x60, 0x1e,
        0x06, 0x31, 0xc0, 0x8e, 0xd8, 0xc7, 0x06, 0x92, 0x84, 0x00, 0x00, 0xc7, 0x06, 0x94, 0x84,
        0x00, 0x00, 0xc7, 0x06, 0x96, 0x84, 0x00, 0x00, 0xa1, 0x0e, 0x04, 0x85, 0xc0, 0x74, 0x0c,
        0x8e, 0xc0, 0x31, 0xff, 0xb9, 0x40, 0x00, 0xe8, 0x29, 0x00, 0x72, 0x20, 0xb8, 0x00, 0xe0,
        0x8e, 0xc0, 0x31, 0xff, 0xb9, 0x00, 0x10, 0xe8, 0x1a, 0x00, 0x72, 0x11, 0xb8, 0x00, 0xf0,
        0x8e, 0xc0, 0x31, 0xff, 0xb9, 0x00, 0x10, 0xe8, 0x0b, 0x00, 0x72, 0x02, 0xeb, 0x03, 0xe8,
        0x44, 0x00, 0x07, 0x1f, 0x61, 0xc3, 0x26, 0x66, 0x81, 0x3d, 0x52, 0x53, 0x44, 0x20, 0x75,
        0x15, 0x26, 0x66, 0x81, 0x7d, 0x04, 0x50, 0x54, 0x52, 0x20, 0x75, 0x0a, 0x51, 0xb9, 0x14,
        0x00, 0xe8, 0x0c, 0x00, 0x59, 0x72, 0x07, 0x83, 0xc7, 0x10, 0xe2, 0xdc, 0xf8, 0xc3, 0xf9,
        0xc3, 0x50, 0x53, 0x30, 0xc0, 0x89, 0xfb, 0x26, 0x02, 0x07, 0x43, 0xe2, 0xfa, 0x84, 0xc0,
        0x75, 0x04, 0x5b, 0x58, 0xf9, 0xc3, 0x5b, 0x58, 0xf8, 0xc3, 0xc7, 0x06, 0x92, 0x84, 0x01,
        0x00, 0xc7, 0x06, 0x94, 0x84, 0x14, 0x00, 0x31, 0xc0, 0x26, 0x8a, 0x45, 0x0f, 0xa3, 0x96,
        0x84, 0x3c, 0x02, 0x72, 0x1f, 0x26, 0x66, 0x8b, 0x45, 0x14, 0x66, 0x83, 0xf8, 0x24, 0x72,
        0x14, 0x66, 0x3d, 0x00, 0x10, 0x00, 0x00, 0x77, 0x0c, 0x51, 0x89, 0xc1, 0xe8, 0xb6, 0xff,
        0x59, 0x73, 0x03, 0xa3, 0x94, 0x84, 0x8b, 0x1e, 0x90, 0x84, 0x66, 0xc7, 0x47, 0x04, 0x69,
        0x00, 0x00, 0x00, 0x8c, 0xc0, 0x66, 0x0f, 0xb7, 0xc0, 0x66, 0xc1, 0xe0, 0x04, 0x66, 0x0f,
        0xb7, 0xd7, 0x66, 0x01, 0xd0, 0x66, 0x89, 0x47, 0x08, 0x66, 0xc7, 0x47, 0x0c, 0x00, 0x00,
        0x00, 0x00, 0x66, 0x31, 0xc0, 0xa1, 0x94, 0x84, 0x66, 0x89, 0x47, 0x10, 0x66, 0xc7, 0x47,
        0x14, 0x00, 0x00, 0x00, 0x00, 0x66, 0x31, 0xc0, 0xa1, 0x96, 0x84, 0x66, 0x89, 0x47, 0x18,
        0x66, 0xc7, 0x47, 0x1c, 0x00, 0x00, 0x00, 0x00, 0xc3, 0x60, 0x1e, 0x06, 0x31, 0xc0, 0x8e,
        0xd8, 0x8e, 0xc0, 0x66, 0x31, 0xdb, 0x31, 0xed, 0xbf, 0xa4, 0x84, 0x66, 0xc7, 0x06, 0xb8,
        0x84, 0x01, 0x00, 0x00, 0x00, 0x66, 0xb8, 0x20, 0xe8, 0x00, 0x00, 0x66, 0xba, 0x50, 0x41,
        0x4d, 0x53, 0x66, 0xb9, 0x18, 0x00, 0x00, 0x00, 0xcd, 0x15, 0x0f, 0x82, 0xbf, 0x00, 0x66,
        0x3d, 0x50, 0x41, 0x4d, 0x53, 0x0f, 0x85, 0xb5, 0x00, 0x66, 0x83, 0xf9, 0x14, 0x0f, 0x82,
        0xad, 0x00, 0x66, 0xa1, 0xac, 0x84, 0x66, 0x0b, 0x06, 0xb0, 0x84, 0x0f, 0x84, 0x99, 0x00,
        0x3b, 0x2e, 0x9a, 0x84, 0x0f, 0x83, 0x98, 0x00, 0x57, 0x8b, 0x3e, 0x98, 0x84, 0x89, 0xe8,
        0xc1, 0xe0, 0x05, 0x01, 0xc7, 0x66, 0xa1, 0xa4, 0x84, 0x66, 0x89, 0x05, 0x66, 0xa1, 0xa8,
        0x84, 0x66, 0x89, 0x45, 0x04, 0x66, 0xa1, 0xac, 0x84, 0x66, 0x89, 0x45, 0x08, 0x66, 0xa1,
        0xb0, 0x84, 0x66, 0x89, 0x45, 0x0c, 0x66, 0xa1, 0xb4, 0x84, 0x66, 0xba, 0x00, 0x00, 0x00,
        0x00, 0x66, 0xb9, 0x91, 0x00, 0x00, 0x00, 0x66, 0x83, 0xf8, 0x01, 0x75, 0x0e, 0x66, 0xba,
        0x05, 0x00, 0x00, 0x00, 0x66, 0xb9, 0xb1, 0x00, 0x00, 0x00, 0xeb, 0x28, 0x66, 0x83, 0xf8,
        0x03, 0x75, 0x08, 0x66, 0xba, 0x06, 0x00, 0x00, 0x00, 0xeb, 0x1a, 0x66, 0x83, 0xf8, 0x04,
        0x75, 0x08, 0x66, 0xba, 0x07, 0x00, 0x00, 0x00, 0xeb, 0x0c, 0x66, 0x83, 0xf8, 0x05, 0x75,
        0x06, 0x66, 0xba, 0x08, 0x00, 0x00, 0x00, 0x66, 0x89, 0x55, 0x10, 0x66, 0x89, 0x4d, 0x14,
        0x66, 0xc7, 0x45, 0x18, 0x03, 0x00, 0x00, 0x00, 0x66, 0xa1, 0xb4, 0x84, 0x66, 0x89, 0x45,
        0x1c, 0x45, 0x5f, 0x66, 0x85, 0xdb, 0x0f, 0x85, 0x1d, 0xff, 0x89, 0x2e, 0xa2, 0x84, 0x8b,
        0x3e, 0x9c, 0x84, 0xa1, 0xa0, 0x84, 0x01, 0xe8, 0x66, 0x0f, 0xb7, 0xc0, 0x66, 0x89, 0x05,
        0x66, 0xc7, 0x45, 0x04, 0x00, 0x00, 0x00, 0x00, 0x83, 0xfd, 0x00, 0x74, 0x44, 0x8b, 0x3e,
        0x9e, 0x84, 0x66, 0xc7, 0x45, 0x04, 0x69, 0x00, 0x00, 0x00, 0xa1, 0x98, 0x84, 0x66, 0x0f,
        0xb7, 0xc0, 0x66, 0x89, 0x45, 0x08, 0x66, 0xc7, 0x45, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x89,
        0xe8, 0xc1, 0xe0, 0x05, 0x66, 0x0f, 0xb7, 0xc0, 0x66, 0x89, 0x45, 0x10, 0x66, 0xc7, 0x45,
        0x14, 0x00, 0x00, 0x00, 0x00, 0x66, 0xc7, 0x45, 0x18, 0x20, 0x00, 0x00, 0x00, 0x66, 0xc7,
        0x45, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x07, 0x1f, 0x61, 0xc3, 0x66, 0xb8, 0x10, 0x00, 0x8e,
        0xd8, 0x8e, 0xc0, 0x8e, 0xd0, 0xbc, 0x00, 0x00, 0x09, 0x00, 0x66, 0x83, 0x3d, 0x38, 0x01,
        0x00, 0x00, 0x00, 0x74, 0x0c, 0x48, 0x8d, 0x35, 0x9d, 0x01, 0x00, 0x00, 0xe8, 0xb3, 0x00,
        0x00, 0x00, 0x66, 0x83, 0x3d, 0x32, 0x01, 0x00, 0x00, 0x00, 0x74, 0x0c, 0x48, 0x8d, 0x35,
        0x73, 0x01, 0x00, 0x00, 0xe8, 0x9d, 0x00, 0x00, 0x00, 0xe8, 0x32, 0x00, 0x00, 0x00, 0x48,
        0x85, 0xc0, 0x74, 0x1e, 0x48, 0x8d, 0x3d, 0xa9, 0x00, 0x00, 0x00, 0xff, 0xd0, 0x3d, 0x4b,
        0x4f, 0x00, 0x00, 0x75, 0x0e, 0x48, 0x8d, 0x35, 0x1d, 0x01, 0x00, 0x00, 0xe8, 0x77, 0x00,
        0x00, 0x00, 0xeb, 0x0c, 0x48, 0x8d, 0x35, 0x25, 0x01, 0x00, 0x00, 0xe8, 0x69, 0x00, 0x00,
        0x00, 0xf4, 0xeb, 0xfd, 0xbb, 0x00, 0x90, 0x00, 0x00, 0x81, 0x3b, 0x7f, 0x45, 0x4c, 0x46,
        0x75, 0x56, 0x80, 0x7b, 0x04, 0x02, 0x75, 0x50, 0x4c, 0x8b, 0x63, 0x18, 0x4c, 0x8b, 0x6b,
        0x20, 0x49, 0x01, 0xdd, 0x44, 0x0f, 0xb7, 0x7b, 0x38, 0x44, 0x0f, 0xb7, 0x73, 0x36, 0x45,
        0x85, 0xff, 0x74, 0x32, 0x41, 0x83, 0x7d, 0x00, 0x01, 0x75, 0x23, 0x49, 0x8b, 0x75, 0x08,
        0x48, 0x01, 0xde, 0x49, 0x8b, 0x7d, 0x18, 0x49, 0x8b, 0x55, 0x20, 0x4d, 0x8b, 0x45, 0x28,
        0xfc, 0x48, 0x89, 0xd1, 0xf3, 0xa4, 0x4c, 0x89, 0xc1, 0x48, 0x29, 0xd1, 0x31, 0xc0, 0xf3,
        0xaa, 0x4d, 0x01, 0xf5, 0x41, 0xff, 0xcf, 0xeb, 0xc9, 0x4c, 0x89, 0xe0, 0xc3, 0x31, 0xc0,
        0xc3, 0xac, 0x84, 0xc0, 0x74, 0x14, 0x88, 0xc4, 0x66, 0xba, 0xfd, 0x03, 0xec, 0xa8, 0x20,
        0x74, 0xf7, 0x88, 0xe0, 0x66, 0xba, 0xf8, 0x03, 0xee, 0xeb, 0xe7, 0xc3, 0x00, 0x00, 0x00,
        0x4e, 0x43, 0x43, 0x42, 0x4f, 0x4f, 0x31, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x90, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0xfe, 0xca,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfe, 0xca, 0x00, 0x00, 0xfe, 0xca, 0xfe, 0xca, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6e, 0x43, 0x20,
        0x6b, 0x65, 0x72, 0x6e, 0x65, 0x6c, 0x20, 0x45, 0x4c, 0x46, 0x20, 0x6b, 0x6d, 0x61, 0x69,
        0x6e, 0x0d, 0x0a, 0x00, 0x6e, 0x43, 0x20, 0x6b, 0x65, 0x72, 0x6e, 0x65, 0x6c, 0x20, 0x45,
        0x4c, 0x46, 0x20, 0x6b, 0x6d, 0x61, 0x69, 0x6e, 0x20, 0x62, 0x61, 0x64, 0x0d, 0x0a, 0x00,
        0x6e, 0x43, 0x20, 0x6b, 0x65, 0x72, 0x6e, 0x65, 0x6c, 0x20, 0x45, 0x38, 0x32, 0x30, 0x20,
        0x6f, 0x6b, 0x0d, 0x0a, 0x00, 0x6e, 0x43, 0x20, 0x6b, 0x65, 0x72, 0x6e, 0x65, 0x6c, 0x20,
        0x41, 0x43, 0x50, 0x49, 0x20, 0x52, 0x53, 0x44, 0x50, 0x20, 0x6f, 0x6b, 0x0d, 0x0a, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x9a, 0xaf,
        0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x92, 0xcf, 0x00, 0x17, 0x00, 0x19, 0x85, 0x00, 0x00,
    ]
}

fn nbuild_kernel_x86_stage0_template_bytes() -> Vec<u8> {
    // BIOS stage0: discover CHS geometry (INT 13h/AH=08), then read the
    // stage1+ELF payload one sector at a time across sector/head boundaries.
    let mut image = vec![
        0xfa, 0x31, 0xc0, 0x8e, 0xd8, 0x8e, 0xc0, 0x8e, 0xd0, 0xbc, 0x00, 0x7c, 0x88, 0x16, 0x9a,
        0x7c, 0xe4, 0x92, 0x0c, 0x02, 0xe6, 0x92, 0xb4, 0x08, 0x8a, 0x16, 0x9a, 0x7c, 0xcd, 0x13,
        0x72, 0x17, 0x80, 0xe1, 0x3f, 0xfe, 0xc6, 0x84, 0xc9, 0x74, 0x0e, 0x84, 0xf6, 0x74, 0x0a,
        0x88, 0x0e, 0x9b, 0x7c, 0x88, 0x36, 0x9c, 0x7c, 0xeb, 0x0a, 0xc6, 0x06, 0x9b, 0x7c, 0x12,
        0xc6, 0x06, 0x9c, 0x7c, 0x02, 0x31, 0xc0, 0x8e, 0xc0, 0xbb, 0x00, 0x80, 0x8b, 0x36, 0x9d,
        0x7c, 0x30, 0xed, 0x30, 0xf6, 0xb1, 0x02, 0x85, 0xf6, 0x74, 0x3c, 0x56, 0x53, 0x51, 0x52,
        0x06, 0xb4, 0x02, 0xb0, 0x01, 0x8a, 0x16, 0x9a, 0x7c, 0xcd, 0x13, 0x07, 0x5a, 0x59, 0x5b,
        0x5e, 0x72, 0x2b, 0x81, 0xc3, 0x00, 0x02, 0x73, 0x07, 0x8c, 0xc0, 0x05, 0x00, 0x10, 0x8e,
        0xc0, 0xfe, 0xc1, 0x3a, 0x0e, 0x9b, 0x7c, 0x76, 0x0e, 0xb1, 0x01, 0xfe, 0xc6, 0x3a, 0x36,
        0x9c, 0x7c, 0x72, 0x04, 0x30, 0xf6, 0xfe, 0xc5, 0x4e, 0xeb, 0xc0, 0xea, 0x00, 0x80, 0x00,
        0x00, 0xf4, 0xeb, 0xfd, 0x00, 0x12, 0x02, 0xfe, 0xca,
    ];
    image.resize(510, 0x00);
    image.extend_from_slice(&[0x55, 0xaa]);
    image
}

fn nbuild_kernel_x86_64_stage0_stage1_elf_image_bytes(
    kernel_elf: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let elf_info = nbuild_kernel_elf64_image_info(kernel_elf)?;

    let stage1 = nbuild_kernel_x86_64_stage1_acpi_e820_template_bytes();

    if stage1.len() > NBUILD_KERNEL_X86_STAGE1_RESERVED_BYTES {
        return Err(actionable_command_error(
            "nbuild",
            "internal x86_64 stage1 ELF loader exceeds its reserved load window",
        )
        .into());
    }

    let mut payload = stage1.clone();
    let boot_info_offset =
        nbuild_kernel_find_unique_bytes(&payload, b"NCCBOO1", "stage1 boot_info magic")?;
    if boot_info_offset + NBUILD_KERNEL_X86_BOOT_INFO_SIZE > payload.len() {
        return Err(actionable_command_error(
            "nbuild",
            "internal x86_64 stage1 boot_info descriptor is truncated",
        )
        .into());
    }
    nbuild_kernel_expect_u16_le(
        &payload,
        NBUILD_KERNEL_X86_STAGE1_ACPI_PLATFORM_SLOT_PTR_OFFSET,
        0xcafe,
        "ACPI RSDP platform slot pointer placeholder",
    )?;
    nbuild_kernel_expect_u16_le(
        &payload,
        NBUILD_KERNEL_X86_STAGE1_E820_NCC_PTR_OFFSET,
        0xcafe,
        "E820 buffer pointer placeholder",
    )?;
    nbuild_kernel_expect_u16_le(
        &payload,
        NBUILD_KERNEL_X86_STAGE1_E820_MAX_ENTRIES_OFFSET,
        0,
        "E820 max entries placeholder",
    )?;
    nbuild_kernel_expect_u16_le(
        &payload,
        NBUILD_KERNEL_X86_STAGE1_E820_MEMORY_MAP_LEN_PTR_OFFSET,
        0xcafe,
        "E820 memory_map_len pointer placeholder",
    )?;
    nbuild_kernel_expect_u16_le(
        &payload,
        NBUILD_KERNEL_X86_STAGE1_E820_PLATFORM_SLOT_PTR_OFFSET,
        0xcafe,
        "E820 platform slot pointer placeholder",
    )?;
    nbuild_kernel_expect_u16_le(
        &payload,
        NBUILD_KERNEL_X86_STAGE1_E820_SYNTHETIC_COUNT_OFFSET,
        0,
        "E820 synthetic count placeholder",
    )?;

    let memory_regions =
        nbuild_kernel_x86_boot_memory_regions(kernel_elf, &elf_info.load_segments)?;
    let synthetic_memory_region_count = memory_regions.len();
    nbuild_kernel_align_vec(&mut payload, 8);
    let memory_map_offset = payload.len();
    for region in &memory_regions {
        nbuild_kernel_append_memory_region(&mut payload, region);
    }

    let e820_region_buffer_offset = payload.len();
    payload.resize(
        payload.len() + NBUILD_KERNEL_X86_E820_MAX_ENTRIES * NBUILD_KERNEL_X86_MEMORY_REGION_SIZE,
        0,
    );

    nbuild_kernel_align_vec(&mut payload, 8);
    let serial_descriptor_offset = payload.len();
    payload.extend_from_slice(b"NCCSER1\0");
    nbuild_kernel_push_u32_le(&mut payload, 1);
    nbuild_kernel_push_u32_le(&mut payload, 1);
    nbuild_kernel_push_u64_le(&mut payload, 0x03f8);
    nbuild_kernel_push_u32_le(&mut payload, 4);
    nbuild_kernel_push_u32_le(&mut payload, 115200);
    nbuild_kernel_push_u32_le(
        &mut payload,
        NBUILD_MEM_FLAG_PRESENT | NBUILD_MEM_FLAG_WRITE,
    );
    nbuild_kernel_push_u32_le(&mut payload, 0);

    nbuild_kernel_align_vec(&mut payload, 8);
    let platform_table_offset = payload.len();
    payload.extend_from_slice(b"NCCPLT1\0");
    nbuild_kernel_push_u32_le(&mut payload, 1);
    let platform_entries = [
        (
            NBUILD_PLATFORM_KIND_NCC_BOOT_V1_OWN_CONTRACT,
            NBUILD_PLATFORM_FLAG_PRESENT
                | NBUILD_PLATFORM_FLAG_REQUIRED
                | NBUILD_PLATFORM_FLAG_OWN_CONTRACT,
            NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + boot_info_offset as u64,
            NBUILD_KERNEL_X86_BOOT_INFO_SIZE as u64,
            1,
        ),
        (
            NBUILD_PLATFORM_KIND_ACPI_RSDP,
            NBUILD_PLATFORM_FLAG_EXTERNAL_ADAPTER
                | NBUILD_PLATFORM_FLAG_ZERO_PTR_IF_ABSENT
                | NBUILD_PLATFORM_FLAG_RUNTIME_PROBE
                | NBUILD_PLATFORM_FLAG_DYNAMIC_LENGTH,
            0,
            0,
            0,
        ),
        (
            NBUILD_PLATFORM_KIND_UEFI_SYSTEM_TABLE,
            NBUILD_PLATFORM_FLAG_EXTERNAL_ADAPTER | NBUILD_PLATFORM_FLAG_ZERO_PTR_IF_ABSENT,
            0,
            0,
            0,
        ),
        (
            NBUILD_PLATFORM_KIND_MULTIBOOT2_TAGS,
            NBUILD_PLATFORM_FLAG_EXTERNAL_ADAPTER | NBUILD_PLATFORM_FLAG_ZERO_PTR_IF_ABSENT,
            0,
            0,
            0,
        ),
        (
            NBUILD_PLATFORM_KIND_BIOS_E820,
            NBUILD_PLATFORM_FLAG_EXTERNAL_ADAPTER
                | NBUILD_PLATFORM_FLAG_RUNTIME_PROBE
                | NBUILD_PLATFORM_FLAG_DYNAMIC_LENGTH,
            NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + e820_region_buffer_offset as u64,
            (NBUILD_KERNEL_X86_E820_MAX_ENTRIES * NBUILD_KERNEL_X86_MEMORY_REGION_SIZE) as u64,
            NBUILD_KERNEL_X86_MEMORY_REGION_SIZE as u64,
        ),
    ];
    let bios_e820_platform_slot_index = platform_entries
        .iter()
        .position(|(kind, _, _, _, _)| *kind == NBUILD_PLATFORM_KIND_BIOS_E820)
        .ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 platform table lost E820 slot")
        })?;
    let acpi_platform_slot_index = platform_entries
        .iter()
        .position(|(kind, _, _, _, _)| *kind == NBUILD_PLATFORM_KIND_ACPI_RSDP)
        .ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 platform table lost ACPI RSDP slot")
        })?;
    nbuild_kernel_push_u32_le(&mut payload, platform_entries.len() as u32);
    for &(kind, flags, ptr, length, checksum_or_revision) in &platform_entries {
        nbuild_kernel_append_platform_entry(
            &mut payload,
            kind,
            flags,
            ptr,
            length,
            checksum_or_revision,
        );
    }

    let boot_info_address = NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + boot_info_offset as u64;
    let memory_map_address = NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + memory_map_offset as u64;
    let e820_region_buffer_address =
        NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + e820_region_buffer_offset as u64;
    let serial_descriptor_address =
        NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + serial_descriptor_offset as u64;
    let platform_table_address =
        NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + platform_table_offset as u64;
    let bios_e820_platform_slot_address = NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS
        + platform_table_offset as u64
        + 16
        + (bios_e820_platform_slot_index * NBUILD_KERNEL_PLATFORM_ENTRY_SIZE) as u64;
    let acpi_platform_slot_address = NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS
        + platform_table_offset as u64
        + 16
        + (acpi_platform_slot_index * NBUILD_KERNEL_PLATFORM_ENTRY_SIZE) as u64;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 16,
        0x0007,
        "boot_info flags",
    )?;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 24,
        memory_map_address,
        "boot_info memory_map_ptr",
    )?;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 32,
        memory_regions.len() as u64,
        "boot_info memory_map_len",
    )?;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 40,
        0,
        "boot_info framebuffer_ptr",
    )?;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 48,
        serial_descriptor_address,
        "boot_info serial_ptr",
    )?;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 56,
        platform_table_address,
        "boot_info platform_table_ptr",
    )?;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 64,
        0,
        "boot_info command_line_ptr",
    )?;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 72,
        NBUILD_KERNEL_X86_KERNEL_ELF_MEMORY_ADDRESS,
        "boot_info kernel_elf_load_address",
    )?;
    nbuild_kernel_patch_u16_le(
        &mut payload,
        NBUILD_KERNEL_X86_STAGE1_ACPI_PLATFORM_SLOT_PTR_OFFSET,
        nbuild_kernel_x86_real_mode_offset(acpi_platform_slot_address, "acpi_rsdp platform slot")?,
        "stage1 ACPI RSDP platform slot pointer",
    )?;
    nbuild_kernel_patch_u16_le(
        &mut payload,
        NBUILD_KERNEL_X86_STAGE1_E820_NCC_PTR_OFFSET,
        nbuild_kernel_x86_real_mode_offset(e820_region_buffer_address, "E820 region buffer")?,
        "stage1 E820 region buffer pointer",
    )?;
    nbuild_kernel_patch_u16_le(
        &mut payload,
        NBUILD_KERNEL_X86_STAGE1_E820_MAX_ENTRIES_OFFSET,
        NBUILD_KERNEL_X86_E820_MAX_ENTRIES as u16,
        "stage1 E820 max entries",
    )?;
    nbuild_kernel_patch_u16_le(
        &mut payload,
        NBUILD_KERNEL_X86_STAGE1_E820_MEMORY_MAP_LEN_PTR_OFFSET,
        nbuild_kernel_x86_real_mode_offset(boot_info_address + 32, "boot_info memory_map_len")?,
        "stage1 E820 memory_map_len pointer",
    )?;
    nbuild_kernel_patch_u16_le(
        &mut payload,
        NBUILD_KERNEL_X86_STAGE1_E820_PLATFORM_SLOT_PTR_OFFSET,
        nbuild_kernel_x86_real_mode_offset(
            bios_e820_platform_slot_address,
            "bios_e820 platform slot",
        )?,
        "stage1 E820 platform slot pointer",
    )?;
    nbuild_kernel_patch_u16_le(
        &mut payload,
        NBUILD_KERNEL_X86_STAGE1_E820_SYNTHETIC_COUNT_OFFSET,
        synthetic_memory_region_count as u16,
        "stage1 E820 synthetic memory region count",
    )?;
    let payload_size_marker = nbuild_kernel_find_unique_bytes(
        &payload,
        &NBUILD_KERNEL_X86_PAYLOAD_SIZE_MARKER.to_le_bytes(),
        "stage1 boot_info kernel-elf-size",
    )?;
    if payload_size_marker != boot_info_offset + 80 {
        return Err(actionable_command_error(
            "nbuild",
            "internal x86_64 stage1 boot_info payload size marker moved unexpectedly",
        )
        .into());
    }
    payload[payload_size_marker..payload_size_marker + 8]
        .copy_from_slice(&(kernel_elf.len() as u64).to_le_bytes());
    if payload.len() > NBUILD_KERNEL_X86_STAGE1_RESERVED_BYTES {
        return Err(actionable_command_error(
            "nbuild",
            "internal x86_64 stage1 descriptors exceed the reserved boot loader window",
        )
        .into());
    }
    payload.resize(NBUILD_KERNEL_X86_STAGE1_RESERVED_BYTES, 0x00);
    payload.extend_from_slice(kernel_elf);

    let payload_sectors = payload.len().div_ceil(NBUILD_KERNEL_SECTOR_SIZE);
    if payload_sectors == 0 || payload_sectors > NBUILD_KERNEL_X86_MAX_STAGE0_CHS_PAYLOAD_SECTORS {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "x86_64 kernel ELF boot payload must occupy 1..={} BIOS sectors; got {} bytes ({} sectors)",
                NBUILD_KERNEL_X86_MAX_STAGE0_CHS_PAYLOAD_SECTORS,
                payload.len(),
                payload_sectors
            ),
        )
        .into());
    }

    let mut image = nbuild_kernel_x86_stage0_template_bytes();

    if image.len() > NBUILD_KERNEL_SECTOR_SIZE {
        return Err(actionable_command_error(
            "nbuild",
            "internal x86_64 stage0 template exceeds one BIOS sector",
        )
        .into());
    }
    if image.len() != NBUILD_KERNEL_SECTOR_SIZE || image[510] != 0x55 || image[511] != 0xaa {
        return Err(actionable_command_error(
            "nbuild",
            "internal x86_64 stage0 template must be exactly 512 bytes and end with 55 AA",
        )
        .into());
    }
    let sector_count_marker =
        nbuild_kernel_find_unique_bytes(&image, &[0xfe, 0xca], "stage0 payload sector-count")?;
    image[sector_count_marker..sector_count_marker + 2]
        .copy_from_slice(&(payload_sectors as u16).to_le_bytes());
    image.extend_from_slice(&payload);
    image.resize(
        NBUILD_KERNEL_SECTOR_SIZE + payload_sectors * NBUILD_KERNEL_SECTOR_SIZE,
        0x00,
    );
    Ok(image)
}

fn nbuild_kernel_i386_stage0_stage1_elf_image_bytes(
    kernel_elf: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let elf_info = nbuild_kernel_elf32_image_info(kernel_elf)?;
    let mut payload = nbuild_kernel_i386_stage1_template_bytes()?;
    let boot_info_offset =
        nbuild_kernel_find_unique_bytes(&payload, b"NCCBOO1", "i386 stage1 boot_info magic")?;
    if boot_info_offset + NBUILD_KERNEL_X86_BOOT_INFO_SIZE > payload.len() {
        return Err(actionable_command_error(
            "nbuild",
            "internal i386 stage1 boot_info descriptor is truncated",
        )
        .into());
    }
    nbuild_kernel_expect_u16_le(
        &payload,
        NBUILD_KERNEL_X86_STAGE1_ACPI_PLATFORM_SLOT_PTR_OFFSET,
        0xcafe,
        "i386 ACPI RSDP platform slot pointer placeholder",
    )?;
    nbuild_kernel_expect_u16_le(
        &payload,
        NBUILD_KERNEL_X86_STAGE1_E820_NCC_PTR_OFFSET,
        0xcafe,
        "i386 E820 buffer pointer placeholder",
    )?;
    nbuild_kernel_expect_u16_le(
        &payload,
        NBUILD_KERNEL_X86_STAGE1_E820_MAX_ENTRIES_OFFSET,
        0,
        "i386 E820 max entries placeholder",
    )?;
    nbuild_kernel_expect_u16_le(
        &payload,
        NBUILD_KERNEL_X86_STAGE1_E820_MEMORY_MAP_LEN_PTR_OFFSET,
        0xcafe,
        "i386 E820 memory_map_len pointer placeholder",
    )?;
    nbuild_kernel_expect_u16_le(
        &payload,
        NBUILD_KERNEL_X86_STAGE1_E820_PLATFORM_SLOT_PTR_OFFSET,
        0xcafe,
        "i386 E820 platform slot pointer placeholder",
    )?;
    nbuild_kernel_expect_u16_le(
        &payload,
        NBUILD_KERNEL_X86_STAGE1_E820_SYNTHETIC_COUNT_OFFSET,
        0,
        "i386 E820 synthetic count placeholder",
    )?;

    let memory_regions =
        nbuild_kernel_x86_boot_memory_regions(kernel_elf, &elf_info.load_segments)?;
    let synthetic_memory_region_count = memory_regions.len();
    nbuild_kernel_align_vec(&mut payload, 8);
    let memory_map_offset = payload.len();
    for region in &memory_regions {
        nbuild_kernel_append_memory_region(&mut payload, region);
    }

    let e820_region_buffer_offset = payload.len();
    payload.resize(
        payload.len() + NBUILD_KERNEL_X86_E820_MAX_ENTRIES * NBUILD_KERNEL_X86_MEMORY_REGION_SIZE,
        0,
    );

    nbuild_kernel_align_vec(&mut payload, 8);
    let serial_descriptor_offset = payload.len();
    payload.extend_from_slice(b"NCCSER1\0");
    nbuild_kernel_push_u32_le(&mut payload, 1);
    nbuild_kernel_push_u32_le(&mut payload, 1);
    nbuild_kernel_push_u64_le(&mut payload, 0x03f8);
    nbuild_kernel_push_u32_le(&mut payload, 4);
    nbuild_kernel_push_u32_le(&mut payload, 115200);
    nbuild_kernel_push_u32_le(
        &mut payload,
        NBUILD_MEM_FLAG_PRESENT | NBUILD_MEM_FLAG_WRITE,
    );
    nbuild_kernel_push_u32_le(&mut payload, 0);

    nbuild_kernel_align_vec(&mut payload, 8);
    let platform_table_offset = payload.len();
    payload.extend_from_slice(b"NCCPLT1\0");
    nbuild_kernel_push_u32_le(&mut payload, 1);
    let platform_entries = [
        (
            NBUILD_PLATFORM_KIND_NCC_BOOT_V1_OWN_CONTRACT,
            NBUILD_PLATFORM_FLAG_PRESENT
                | NBUILD_PLATFORM_FLAG_REQUIRED
                | NBUILD_PLATFORM_FLAG_OWN_CONTRACT,
            NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + boot_info_offset as u64,
            NBUILD_KERNEL_X86_BOOT_INFO_SIZE as u64,
            1,
        ),
        (
            NBUILD_PLATFORM_KIND_ACPI_RSDP,
            NBUILD_PLATFORM_FLAG_EXTERNAL_ADAPTER
                | NBUILD_PLATFORM_FLAG_ZERO_PTR_IF_ABSENT
                | NBUILD_PLATFORM_FLAG_RUNTIME_PROBE
                | NBUILD_PLATFORM_FLAG_DYNAMIC_LENGTH,
            0,
            0,
            0,
        ),
        (
            NBUILD_PLATFORM_KIND_MULTIBOOT2_TAGS,
            NBUILD_PLATFORM_FLAG_EXTERNAL_ADAPTER | NBUILD_PLATFORM_FLAG_ZERO_PTR_IF_ABSENT,
            0,
            0,
            0,
        ),
        (
            NBUILD_PLATFORM_KIND_BIOS_E820,
            NBUILD_PLATFORM_FLAG_EXTERNAL_ADAPTER
                | NBUILD_PLATFORM_FLAG_RUNTIME_PROBE
                | NBUILD_PLATFORM_FLAG_DYNAMIC_LENGTH,
            NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + e820_region_buffer_offset as u64,
            (NBUILD_KERNEL_X86_E820_MAX_ENTRIES * NBUILD_KERNEL_X86_MEMORY_REGION_SIZE) as u64,
            NBUILD_KERNEL_X86_MEMORY_REGION_SIZE as u64,
        ),
    ];
    let bios_e820_platform_slot_index = platform_entries
        .iter()
        .position(|(kind, _, _, _, _)| *kind == NBUILD_PLATFORM_KIND_BIOS_E820)
        .ok_or_else(|| {
            actionable_command_error("nbuild", "i386 stage1 platform table lost E820 slot")
        })?;
    let acpi_platform_slot_index = platform_entries
        .iter()
        .position(|(kind, _, _, _, _)| *kind == NBUILD_PLATFORM_KIND_ACPI_RSDP)
        .ok_or_else(|| {
            actionable_command_error("nbuild", "i386 stage1 platform table lost ACPI RSDP slot")
        })?;
    nbuild_kernel_push_u32_le(&mut payload, platform_entries.len() as u32);
    for &(kind, flags, ptr, length, checksum_or_revision) in &platform_entries {
        nbuild_kernel_append_platform_entry(
            &mut payload,
            kind,
            flags,
            ptr,
            length,
            checksum_or_revision,
        );
    }

    let boot_info_address = NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + boot_info_offset as u64;
    let memory_map_address = NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + memory_map_offset as u64;
    let e820_region_buffer_address =
        NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + e820_region_buffer_offset as u64;
    let serial_descriptor_address =
        NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + serial_descriptor_offset as u64;
    let platform_table_address =
        NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + platform_table_offset as u64;
    let bios_e820_platform_slot_address = NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS
        + platform_table_offset as u64
        + 16
        + (bios_e820_platform_slot_index * NBUILD_KERNEL_PLATFORM_ENTRY_SIZE) as u64;
    let acpi_platform_slot_address = NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS
        + platform_table_offset as u64
        + 16
        + (acpi_platform_slot_index * NBUILD_KERNEL_PLATFORM_ENTRY_SIZE) as u64;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 16,
        0x0007,
        "i386 boot_info flags",
    )?;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 24,
        memory_map_address,
        "i386 boot_info memory_map_ptr",
    )?;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 32,
        memory_regions.len() as u64,
        "i386 boot_info memory_map_len",
    )?;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 40,
        0,
        "i386 boot_info framebuffer_ptr",
    )?;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 48,
        serial_descriptor_address,
        "i386 boot_info serial_ptr",
    )?;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 56,
        platform_table_address,
        "i386 boot_info platform_table_ptr",
    )?;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 64,
        0,
        "i386 boot_info command_line_ptr",
    )?;
    nbuild_kernel_patch_u64_le(
        &mut payload,
        boot_info_offset + 72,
        NBUILD_KERNEL_X86_KERNEL_ELF_MEMORY_ADDRESS,
        "i386 boot_info kernel_elf_load_address",
    )?;
    nbuild_kernel_patch_u16_le(
        &mut payload,
        NBUILD_KERNEL_X86_STAGE1_ACPI_PLATFORM_SLOT_PTR_OFFSET,
        nbuild_kernel_x86_real_mode_offset(
            acpi_platform_slot_address,
            "i386 acpi_rsdp platform slot",
        )?,
        "i386 stage1 ACPI RSDP platform slot pointer",
    )?;
    nbuild_kernel_patch_u16_le(
        &mut payload,
        NBUILD_KERNEL_X86_STAGE1_E820_NCC_PTR_OFFSET,
        nbuild_kernel_x86_real_mode_offset(e820_region_buffer_address, "i386 E820 region buffer")?,
        "i386 stage1 E820 region buffer pointer",
    )?;
    nbuild_kernel_patch_u16_le(
        &mut payload,
        NBUILD_KERNEL_X86_STAGE1_E820_MAX_ENTRIES_OFFSET,
        NBUILD_KERNEL_X86_E820_MAX_ENTRIES as u16,
        "i386 stage1 E820 max entries",
    )?;
    nbuild_kernel_patch_u16_le(
        &mut payload,
        NBUILD_KERNEL_X86_STAGE1_E820_MEMORY_MAP_LEN_PTR_OFFSET,
        nbuild_kernel_x86_real_mode_offset(
            boot_info_address + 32,
            "i386 boot_info memory_map_len",
        )?,
        "i386 stage1 E820 memory_map_len pointer",
    )?;
    nbuild_kernel_patch_u16_le(
        &mut payload,
        NBUILD_KERNEL_X86_STAGE1_E820_PLATFORM_SLOT_PTR_OFFSET,
        nbuild_kernel_x86_real_mode_offset(
            bios_e820_platform_slot_address,
            "i386 bios_e820 platform slot",
        )?,
        "i386 stage1 E820 platform slot pointer",
    )?;
    nbuild_kernel_patch_u16_le(
        &mut payload,
        NBUILD_KERNEL_X86_STAGE1_E820_SYNTHETIC_COUNT_OFFSET,
        synthetic_memory_region_count as u16,
        "i386 stage1 E820 synthetic memory region count",
    )?;
    let payload_size_marker = nbuild_kernel_find_unique_bytes(
        &payload,
        &NBUILD_KERNEL_X86_PAYLOAD_SIZE_MARKER.to_le_bytes(),
        "i386 stage1 boot_info kernel-elf-size",
    )?;
    if payload_size_marker != boot_info_offset + 80 {
        return Err(actionable_command_error(
            "nbuild",
            "internal i386 stage1 boot_info payload size marker moved unexpectedly",
        )
        .into());
    }
    payload[payload_size_marker..payload_size_marker + 8]
        .copy_from_slice(&(kernel_elf.len() as u64).to_le_bytes());
    if payload.len() > NBUILD_KERNEL_X86_STAGE1_RESERVED_BYTES {
        return Err(actionable_command_error(
            "nbuild",
            "internal i386 stage1 descriptors exceed the reserved boot loader window",
        )
        .into());
    }
    payload.resize(NBUILD_KERNEL_X86_STAGE1_RESERVED_BYTES, 0x00);
    payload.extend_from_slice(kernel_elf);

    let payload_sectors = payload.len().div_ceil(NBUILD_KERNEL_SECTOR_SIZE);
    if payload_sectors == 0 || payload_sectors > NBUILD_KERNEL_X86_MAX_STAGE0_CHS_PAYLOAD_SECTORS {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "i386 kernel ELF boot payload must occupy 1..={} BIOS sectors; got {} bytes ({} sectors)",
                NBUILD_KERNEL_X86_MAX_STAGE0_CHS_PAYLOAD_SECTORS,
                payload.len(),
                payload_sectors
            ),
        )
        .into());
    }

    let mut image = nbuild_kernel_x86_stage0_template_bytes();
    if image.len() > NBUILD_KERNEL_SECTOR_SIZE {
        return Err(actionable_command_error(
            "nbuild",
            "internal i386 stage0 template exceeds one BIOS sector",
        )
        .into());
    }
    let sector_count_marker =
        nbuild_kernel_find_unique_bytes(&image, &[0xfe, 0xca], "i386 stage0 payload sector-count")?;
    image[sector_count_marker..sector_count_marker + 2]
        .copy_from_slice(&(payload_sectors as u16).to_le_bytes());
    image.extend_from_slice(&payload);
    image.resize(
        NBUILD_KERNEL_SECTOR_SIZE + payload_sectors * NBUILD_KERNEL_SECTOR_SIZE,
        0x00,
    );
    Ok(image)
}

fn nbuild_kernel_i386_boot_image_artifact(
    stem: &str,
    bin_dir: &Path,
    run_dir: &Path,
    kernel_elf: &[u8],
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let safe_target = "i386-unknown-none-elf";
    let boot_path = bin_dir.join(format!("{stem}.{safe_target}.boot.img"));
    let serial_path = run_dir.join(format!("{stem}.{safe_target}.qemu-serial.log"));
    let elf_info = nbuild_kernel_elf32_image_info(kernel_elf)?;
    let image = nbuild_kernel_i386_stage0_stage1_elf_image_bytes(kernel_elf)?;
    let stage1_reserved_bytes = NBUILD_KERNEL_X86_STAGE1_RESERVED_BYTES;
    let kernel_elf_payload_offset = NBUILD_KERNEL_SECTOR_SIZE + stage1_reserved_bytes;
    let payload_sectors = (stage1_reserved_bytes + kernel_elf.len()).div_ceil(512);
    if image.get(kernel_elf_payload_offset..kernel_elf_payload_offset + kernel_elf.len())
        != Some(kernel_elf)
    {
        return Err(actionable_command_error(
            "nbuild",
            "internal i386 boot image lost the exact kernel ELF32 payload bytes",
        )
        .into());
    }
    let stage1 = image
        .get(NBUILD_KERNEL_SECTOR_SIZE..NBUILD_KERNEL_SECTOR_SIZE + stage1_reserved_bytes)
        .ok_or_else(|| {
            actionable_command_error("nbuild", "i386 boot image is missing stage1 bytes")
        })?;
    let boot_info_offset =
        nbuild_kernel_find_unique_bytes(stage1, b"NCCBOO1", "i386 stage1 boot_info magic")?;
    let boot_info_address = NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + boot_info_offset as u64;
    let boot_info_flags =
        nbuild_kernel_read_u64_le(stage1, boot_info_offset + 16).ok_or_else(|| {
            actionable_command_error("nbuild", "i386 stage1 boot_info flags are truncated")
        })?;
    let memory_map_address =
        nbuild_kernel_read_u64_le(stage1, boot_info_offset + 24).ok_or_else(|| {
            actionable_command_error(
                "nbuild",
                "i386 stage1 boot_info memory map ptr is truncated",
            )
        })?;
    let memory_map_len =
        nbuild_kernel_read_u64_le(stage1, boot_info_offset + 32).ok_or_else(|| {
            actionable_command_error(
                "nbuild",
                "i386 stage1 boot_info memory map len is truncated",
            )
        })?;
    let serial_ptr = nbuild_kernel_read_u64_le(stage1, boot_info_offset + 48).ok_or_else(|| {
        actionable_command_error("nbuild", "i386 stage1 boot_info serial ptr is truncated")
    })?;
    let platform_table_ptr =
        nbuild_kernel_read_u64_le(stage1, boot_info_offset + 56).ok_or_else(|| {
            actionable_command_error(
                "nbuild",
                "i386 stage1 boot_info platform table ptr is truncated",
            )
        })?;
    let kernel_elf_load_address = nbuild_kernel_read_u64_le(stage1, boot_info_offset + 72)
        .ok_or_else(|| {
            actionable_command_error(
                "nbuild",
                "i386 stage1 boot_info kernel ELF load ptr is truncated",
            )
        })?;
    let kernel_elf_size =
        nbuild_kernel_read_u64_le(stage1, boot_info_offset + 80).ok_or_else(|| {
            actionable_command_error(
                "nbuild",
                "i386 stage1 boot_info kernel ELF size is truncated",
            )
        })?;
    let memory_map_entries =
        nbuild_kernel_parse_memory_regions_from_stage1(stage1, memory_map_address, memory_map_len)?;
    let serial_offset = serial_ptr
        .checked_sub(NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS)
        .ok_or_else(|| actionable_command_error("nbuild", "i386 stage1 serial ptr is invalid"))?
        as usize;
    let serial_base = nbuild_kernel_read_u64_le(stage1, serial_offset + 16).ok_or_else(|| {
        actionable_command_error("nbuild", "i386 stage1 serial descriptor base is truncated")
    })?;
    let serial_baud = nbuild_kernel_read_u32_le(stage1, serial_offset + 28).ok_or_else(|| {
        actionable_command_error("nbuild", "i386 stage1 serial descriptor baud is truncated")
    })?;
    let platform_offset = platform_table_ptr
        .checked_sub(NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS)
        .ok_or_else(|| {
            actionable_command_error("nbuild", "i386 stage1 platform table ptr is invalid")
        })? as usize;
    let platform_entry_count =
        nbuild_kernel_read_u32_le(stage1, platform_offset + 12).ok_or_else(|| {
            actionable_command_error("nbuild", "i386 stage1 platform table count is truncated")
        })?;
    let platform_entries = nbuild_kernel_parse_platform_entries_from_stage1(
        stage1,
        platform_offset,
        platform_entry_count,
    )?;
    let acpi_rsdp_runtime_entry = platform_entries
        .iter()
        .find(|entry| entry["kind"] == "acpi_rsdp")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let bios_e820_runtime_entry = platform_entries
        .iter()
        .find(|entry| entry["kind"] == "bios_e820")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    fs::write(&boot_path, &image)?;
    require_non_empty_output_file(
        "nbuild i386 kernel boot image",
        boot_path.to_string_lossy().as_ref(),
    )?;

    Ok(serde_json::json!({
        "schema": "nc.kernel.boot_image.v1",
        "status": "stage0_stage1_elf32_kmain_image_written",
        "target": safe_target,
        "format": "bios-stage0-stage1-elf32-loader",
        "entry_symbol": "kmain",
        "executes_entry_symbol": true,
        "entry_symbol_origin": "kernel-elf32-e_entry",
        "executes_machine_object_entry_symbol": true,
        "executes_kernel_elf_entry_symbol": true,
        "handoff_status": "stage0 reads stage1 plus the final kernel ELF32 executable from disk sector 2 into 0x8000; stage1 probes ACPI RSDP and BIOS int15h E820 in real mode, enters i386 protected mode, parses ELF32 PT_LOAD program headers, copies segments to p_paddr, pushes boot_info on the cdecl stack and calls e_entry",
        "cpu_mode": "i386-protected-mode",
        "stage0_loader": {
            "kind": "i386-bios-stage0",
            "disk_read": "int13h-chs-sector-loop",
            "stage1_load_address": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS),
            "kernel_elf_memory_address": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_KERNEL_ELF_MEMORY_ADDRESS),
            "payload_start_sector": 2,
            "payload_sector_size": 512,
            "payload_sectors": payload_sectors,
            "payload_load_address": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS)
        },
        "stage1_loader": {
            "kind": "i386-protected-mode-elf32-loader",
            "load_address": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS),
            "reserved_bytes": stage1_reserved_bytes,
            "kernel_elf_memory_address": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_KERNEL_ELF_MEMORY_ADDRESS),
            "parses": ["ELF32 header", "program headers", "PT_LOAD"],
            "runtime_probes": ["ACPI RSDP search", "BIOS int15h E820 memory map"],
            "segment_copy": "PT_LOAD p_offset -> p_paddr, then zero p_memsz - p_filesz",
            "entry_call": "e_entry",
            "return_contract": "eax == 0x4f4b"
        },
        "boot_info": {
            "layout": "ncc_kernel_boot_info_v1",
            "version": 1,
            "address": nbuild_kernel_hex_u64(boot_info_address),
            "offset_in_stage1": boot_info_offset,
            "size": NBUILD_KERNEL_X86_BOOT_INFO_SIZE,
            "argument_register": "stack[0]",
            "magic": "NCCBOO1",
            "flags": {
                "raw": boot_info_flags,
                "memory_map_present": boot_info_flags & 0x0001 != 0,
                "serial_present": boot_info_flags & 0x0002 != 0,
                "platform_table_present": boot_info_flags & 0x0004 != 0,
                "framebuffer_present": boot_info_flags & 0x0008 != 0
            },
            "memory_map": {
                "address": nbuild_kernel_hex_u64(memory_map_address),
                "entry_count": memory_map_len,
                "static_entry_count": memory_map_len,
                "runtime_entry_count": {
                    "minimum": memory_map_len,
                    "maximum": memory_map_len + NBUILD_KERNEL_X86_E820_MAX_ENTRIES as u64,
                    "updated_by": "stage1 BIOS int15h E820 probe before kmain"
                },
                "entry_size": NBUILD_KERNEL_X86_MEMORY_REGION_SIZE,
                "entries": memory_map_entries,
                "runtime_extensions": {
                    "bios_e820": {
                        "probe": "int15h-e820",
                        "buffer_entry_capacity": NBUILD_KERNEL_X86_E820_MAX_ENTRIES,
                        "buffer_entry_size": NBUILD_KERNEL_X86_MEMORY_REGION_SIZE,
                        "platform_entry": bios_e820_runtime_entry,
                        "updates_memory_map_len": true,
                        "success_serial_marker": "nC kernel i386 E820 ok"
                    }
                }
            },
            "framebuffer": {
                "present": false,
                "ptr": "0x0",
                "reason_if_absent": "bios i386 stage1 path has no linear framebuffer handoff yet"
            },
            "serial": {
                "present": serial_ptr != 0,
                "ptr": nbuild_kernel_hex_u64(serial_ptr),
                "magic": "NCCSER1",
                "kind": "x86_port_io",
                "base": nbuild_kernel_hex_u64(serial_base),
                "baud": serial_baud
            },
            "platform_table": {
                "present": platform_table_ptr != 0,
                "ptr": nbuild_kernel_hex_u64(platform_table_ptr),
                "magic": "NCCPLT1",
                "version": 1,
                "entry_count": platform_entry_count,
                "entry_size": NBUILD_KERNEL_PLATFORM_ENTRY_SIZE,
                "entries": platform_entries,
                "runtime_probes": {
                    "acpi_rsdp": {
                        "probe": "BIOS EBDA plus 0xe0000..0xfffff RSD PTR search",
                        "platform_entry": acpi_rsdp_runtime_entry,
                        "updates_platform_slot": true,
                        "success_serial_marker": "nC kernel i386 ACPI RSDP ok"
                    }
                },
                "external_tables": {
                    "acpi_rsdp": "runtime-probed-by-stage1",
                    "multiboot2_tags": false,
                    "bios_e820": "runtime-probed-by-stage1",
                    "reason": "raw i386 BIOS stage1 uses the NCC-owned contract and probes ACPI RSDP/E820 itself; other external adapters populate typed slots when present"
                }
            },
            "kernel_elf_payload": {
                "load_address": nbuild_kernel_hex_u64(kernel_elf_load_address),
                "size": kernel_elf_size
            },
            "stack": {
                "pointer_register": "esp",
                "top": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_BOOT_STACK_TOP),
                "low": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_BOOT_STACK_LOW),
                "size": NBUILD_KERNEL_X86_BOOT_STACK_SIZE,
                "alignment": 4,
                "owner": "stage1-loader"
            }
        },
        "kernel_elf": {
            "format": "ELF32_ET_EXEC",
            "payload_offset": kernel_elf_payload_offset,
            "payload_sector_size": 512,
            "payload_sectors": payload_sectors,
            "memory_address": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_KERNEL_ELF_MEMORY_ADDRESS),
            "size": kernel_elf.len(),
            "sha256": sha256_hex_bytes(kernel_elf),
            "entry": nbuild_kernel_hex_u64(elf_info.entry as u64),
            "program_header_offset": elf_info.phoff,
            "program_header_entry_size": elf_info.phentsize,
            "program_header_count": elf_info.phnum,
            "load_segments": elf_info.load_segments.iter().map(|segment| serde_json::json!({
                "offset": segment.offset,
                "vaddr": nbuild_kernel_hex_u64(segment.vaddr),
                "paddr": nbuild_kernel_hex_u64(segment.paddr),
                "filesz": segment.filesz,
                "memsz": segment.memsz,
                "flags": segment.flags,
                "align": segment.align
            })).collect::<Vec<_>>(),
            "return_contract": "eax == 0x4f4b"
        },
        "path": boot_path.display().to_string(),
        "size": image.len(),
        "sha256": sha256_hex_file(&boot_path)?,
        "signature": "55aa",
        "serial_expected": "nC kernel i386 ELF kmain",
        "serial_expected_sequence": ["nC kernel i386 ACPI RSDP ok", "nC kernel i386 E820 ok", "nC kernel i386 ELF kmain"],
        "qemu_smoke": {
            "runner": "qemu-system-i386",
            "timeout_seconds": 6,
            "serial_log": serial_path.display().to_string(),
            "command": [
                "qemu-system-i386",
                "-display", "none",
                "-machine", "accel=tcg",
                "-serial", format!("file:{}", serial_path.display()),
                "-drive", format!("format=raw,file={},if=floppy", boot_path.display()),
                "-no-reboot"
            ],
            "expected_serial_contains": "nC kernel i386 ELF kmain",
            "expected_serial_sequence": ["nC kernel i386 ACPI RSDP ok", "nC kernel i386 E820 ok", "nC kernel i386 ELF kmain"]
        }
    }))
}

fn nbuild_kernel_qemu_virt_elf_artifact(
    stem: &str,
    spec: NbuildKernelTargetSpec,
    kernel_path: &Path,
    run_dir: &Path,
    kernel_elf: &[u8],
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let safe_target = spec.triple.replace(['/', '\\', ':'], "_");
    let serial_path = run_dir.join(format!("{stem}.{safe_target}.qemu-serial.log"));
    let (format, machine, entry, load_segments, runner, cpu, expected, loader_mode) =
        match spec.triple {
            "aarch64-unknown-none" => {
                let info = nbuild_kernel_elf64_image_info_for_machine(kernel_elf, 183, "AArch64")?;
                (
                    "elf64-et-exec",
                    "EM_AARCH64",
                    info.entry,
                    info.load_segments,
                    "qemu-system-aarch64",
                    "max",
                    "nC kernel AArch64 kmain",
                    "qemu-aarch64-kernel-elf-entry",
                )
            }
            "riscv64-unknown-elf" => {
                let info = nbuild_kernel_elf64_image_info_for_machine(kernel_elf, 243, "RISC-V64")?;
                (
                    "elf64-et-exec",
                    "EM_RISCV",
                    info.entry,
                    info.load_segments,
                    "qemu-system-riscv64",
                    "rv64",
                    "nC kernel RISC-V64 kmain",
                    "opensbi-fw-dynamic-elf-next-stage",
                )
            }
            "riscv32imac-unknown-elf" => {
                let info = nbuild_kernel_elf32_image_info_for_machine(kernel_elf, 243, "RISC-V32")?;
                (
                    "elf32-et-exec",
                    "EM_RISCV",
                    info.entry as u64,
                    info.load_segments,
                    "qemu-system-riscv32",
                    "rv32",
                    "nC kernel RISC-V32 kmain",
                    "opensbi-fw-dynamic-elf-next-stage",
                )
            }
            other => {
                return Err(actionable_command_error(
                    "nbuild",
                    format!("target '{other}' has no QEMU virt ELF boot adapter"),
                )
                .into())
            }
        };
    let entry_in_executable_segment = load_segments.iter().any(|segment| {
        segment.flags & 1 != 0
            && entry >= segment.vaddr
            && entry < segment.vaddr.saturating_add(segment.memsz)
    });
    if !entry_in_executable_segment {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "{} kernel ELF entry 0x{entry:x} is outside every executable PT_LOAD segment",
                spec.triple
            ),
        )
        .into());
    }
    if fs::read(kernel_path)? != kernel_elf {
        return Err(actionable_command_error(
            "nbuild",
            "QEMU virt boot adapter did not retain the exact final kernel ELF bytes",
        )
        .into());
    }

    let mut opensbi_boot_path = None;
    let opensbi = if spec.arch == "riscv64" || spec.arch == "riscv32" {
        let payload = ncc_machine::build_opensbi_payload(kernel_elf).map_err(|error| {
            actionable_command_error(
                "nbuild",
                format!("cannot materialize OpenSBI next-stage payload: {error}"),
            )
        })?;
        let safe_target = spec.triple.replace(['/', '\\', ':'], "_");
        let payload_path = kernel_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("{stem}.{safe_target}.opensbi-payload.bin"));
        fs::write(&payload_path, &payload.bytes)?;
        opensbi_boot_path = Some(payload_path.clone());
        Some(serde_json::json!({
            "firmware_mode": "OpenSBI flat next-stage with entry trampoline; also valid for FW_JUMP preloading",
            "elf_next_stage": kernel_path.display().to_string(),
            "flat_payload": payload_path.display().to_string(),
            "flat_payload_sha256": sha256_hex_file(&payload_path)?,
            "load_address": nbuild_kernel_hex_u64(payload.load_address),
            "next_address": nbuild_kernel_hex_u64(payload.entry_point),
            "firmware_jump_address": nbuild_kernel_hex_u64(payload.load_address),
            "entry_trampoline_instruction": payload.entry_trampoline.map(|word| format!("0x{word:08x}")),
            "next_mode": "S-mode",
            "entry_registers": { "a0": "boot hart id", "a1": "FDT physical address" },
            "fw_jump_contract": {
                "FW_JUMP_ADDR": nbuild_kernel_hex_u64(payload.entry_point),
                "preload_flat_payload_at": nbuild_kernel_hex_u64(payload.load_address)
            }
        }))
    } else {
        None
    };
    let firmware_boot_path = opensbi_boot_path.as_deref().unwrap_or(kernel_path);
    let artifact_format = if opensbi.is_some() {
        "opensbi-flat-next-stage"
    } else {
        format
    };

    let command = if spec.arch == "aarch64" {
        serde_json::json!([
            runner,
            "-M",
            "virt",
            "-cpu",
            cpu,
            "-m",
            "128M",
            "-display",
            "none",
            "-monitor",
            "none",
            "-serial",
            format!("file:{}", serial_path.display()),
            "-kernel",
            firmware_boot_path.display().to_string(),
            "-no-reboot"
        ])
    } else {
        serde_json::json!([
            runner,
            "-M",
            "virt",
            "-bios",
            "default",
            "-cpu",
            cpu,
            "-m",
            "128M",
            "-display",
            "none",
            "-monitor",
            "none",
            "-serial",
            format!("file:{}", serial_path.display()),
            "-kernel",
            firmware_boot_path.display().to_string(),
            "-no-reboot"
        ])
    };

    Ok(serde_json::json!({
        "schema": "nc.kernel.qemu_virt_elf_boot.v1",
        "status": "materialized",
        "target": spec.triple,
        "arch": spec.arch,
        "format": artifact_format,
        "machine": machine,
        "path": firmware_boot_path.display().to_string(),
        "kernel_elf_path": kernel_path.display().to_string(),
        "size": fs::metadata(firmware_boot_path)?.len(),
        "sha256": sha256_hex_file(firmware_boot_path)?,
        "entry": nbuild_kernel_hex_u64(entry),
        "entry_in_executable_segment": true,
        "loader": {
            "machine": "virt",
            "mode": loader_mode,
            "honors_elf_entry": true,
            "loads_pt_load_segments": spec.arch == "aarch64",
            "entry_transport": if spec.arch == "aarch64" { "ELF e_entry" } else { "RISC-V JAL trampoline from OpenSBI load base to ELF e_entry" },
            "initial_stack": if spec.arch == "aarch64" { "0x47ff0000" } else { "0x87ff0000" },
            "boot_info": "ncc_kernel_boot_info_v1 materialized in writable PT_LOAD",
            "platform_adapter": if spec.arch == "aarch64" { "FDT pointer captured from x0" } else { "OpenSBI hart/FDT captured from mandatory a0/a1 next-stage registers" }
        },
        "opensbi": opensbi,
        "load_segments": load_segments.iter().map(|segment| serde_json::json!({
            "offset": segment.offset,
            "vaddr": nbuild_kernel_hex_u64(segment.vaddr),
            "paddr": nbuild_kernel_hex_u64(segment.paddr),
            "filesz": segment.filesz,
            "memsz": segment.memsz,
            "flags": segment.flags,
            "align": segment.align
        })).collect::<Vec<_>>(),
        "serial_expected": expected,
        "qemu_smoke": {
            "runner": runner,
            "timeout_seconds": 8,
            "serial_log": serial_path.display().to_string(),
            "command": command,
            "expected_serial_contains": expected,
            "expected_serial_sequence": [expected]
        }
    }))
}

fn nbuild_kernel_uefi_artifact(
    stem: &str,
    spec: NbuildKernelTargetSpec,
    kernel_elf: &[u8],
    bin_dir: &Path,
    obj_dir: &Path,
    lowering_dir: &Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let target = match spec.triple {
        "x86_64-unknown-none" => ncc_machine::UefiLoaderTarget::X86_64,
        "aarch64-unknown-none" => ncc_machine::UefiLoaderTarget::Aarch64,
        other => {
            return Err(actionable_command_error(
                "nbuild",
                format!("target '{other}' has no UEFI application ABI"),
            )
            .into())
        }
    };
    let safe_target = spec.triple.replace(['/', '\\', ':'], "_");
    let plan = ncc_machine::UefiLoaderPlan::from_elf(kernel_elf, target).map_err(|error| {
        actionable_command_error(
            "nbuild",
            format!("cannot construct {}: {error}", target.removable_boot_name()),
        )
    })?;
    let llvm_ir = plan.render_llvm_ir().map_err(|error| {
        actionable_command_error(
            "nbuild",
            format!("cannot render {}: {error}", target.removable_boot_name()),
        )
    })?;
    let ll_path = lowering_dir.join(format!("{stem}.{safe_target}.uefi-loader.ll"));
    let obj_path = obj_dir.join(format!("{stem}.{safe_target}.uefi-loader.obj"));
    let efi_path = bin_dir.join(format!(
        "{stem}.{safe_target}.{}",
        target.removable_boot_name()
    ));
    let smoke_disk_path = bin_dir.join(format!("{stem}.{safe_target}.uefi-fat12.img"));
    let disk_path = bin_dir.join(format!("{stem}.{safe_target}.uefi-gpt-fat32.img"));
    fs::write(&ll_path, llvm_ir.as_bytes())?;

    let clang = super::mct_support::mct_find_tool("clang").ok_or_else(|| {
        actionable_command_error(
            "nbuild",
            "physical UEFI materialization requires clang from the LLVM backend toolchain",
        )
    })?;
    let clang_ll_path = super::mct_support::mct_path_for_tool(&clang, &ll_path)
        .map_err(|error| actionable_command_error("nbuild", error))?;
    let clang_obj_path = super::mct_support::mct_path_for_tool(&clang, &obj_path)
        .map_err(|error| actionable_command_error("nbuild", error))?;
    let compile = Command::new(&clang)
        .args([
            "-target",
            target.clang_triple(),
            "-ffreestanding",
            "-fno-stack-protector",
            "-fno-builtin",
            "-O2",
            "-c",
            "-x",
            "ir",
        ])
        .arg(&clang_ll_path)
        .arg("-o")
        .arg(&clang_obj_path)
        .output()?;
    if !compile.status.success() {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "clang failed to materialize the {} UEFI loader: {}",
                target.coff_machine(),
                String::from_utf8_lossy(&compile.stderr).trim()
            ),
        )
        .into());
    }
    require_non_empty_output_file(
        "nbuild UEFI loader object",
        obj_path.to_string_lossy().as_ref(),
    )?;

    let lld = super::mct_support::mct_find_tool("lld-link").ok_or_else(|| {
        actionable_command_error(
            "nbuild",
            "physical UEFI materialization requires lld-link from the LLVM backend toolchain",
        )
    })?;
    let machine = format!("/machine:{}", target.coff_machine());
    let lld_efi_path = super::mct_support::mct_path_for_tool(&lld, &efi_path)
        .map_err(|error| actionable_command_error("nbuild", error))?;
    let lld_obj_path = super::mct_support::mct_path_for_tool(&lld, &obj_path)
        .map_err(|error| actionable_command_error("nbuild", error))?;
    let out = format!("/out:{lld_efi_path}");
    let link = Command::new(&lld)
        .args([
            "/subsystem:efi_application",
            "/entry:efi_main",
            "/nodefaultlib",
            "/dynamicbase",
            "/nxcompat",
            &machine,
            &out,
        ])
        .arg(&lld_obj_path)
        .output()?;
    if !link.status.success() {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "lld-link failed to emit {}: {}",
                target.removable_boot_name(),
                String::from_utf8_lossy(&link.stderr).trim()
            ),
        )
        .into());
    }
    require_non_empty_output_file(
        "nbuild UEFI application",
        efi_path.to_string_lossy().as_ref(),
    )?;
    let efi_bytes = fs::read(&efi_path)?;
    let load_plan = ncc_machine::GuestLoadPlan::parse(&efi_bytes).map_err(|error| {
        actionable_command_error(
            "nbuild",
            format!(
                "generated {} failed PE/COFF validation: {error}",
                target.removable_boot_name()
            ),
        )
    })?;
    if load_plan.protocol != ncc_machine::GuestBootProtocol::UefiLoadImage {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "generated {} is not an EFI application",
                target.removable_boot_name()
            ),
        )
        .into());
    }
    let fat12 = ncc_machine::build_uefi_fat12_disk_named(&efi_bytes, target.fat12_short_name())?;
    fs::write(&smoke_disk_path, &fat12)?;
    require_non_empty_output_file(
        "nbuild UEFI FAT12 smoke image",
        smoke_disk_path.to_string_lossy().as_ref(),
    )?;
    let gpt_fat32 =
        ncc_machine::build_uefi_gpt_fat32_disk_named(&efi_bytes, target.fat12_short_name())?;
    let extracted =
        ncc_machine::validate_uefi_gpt_fat32_disk(&gpt_fat32, target.fat12_short_name())?;
    if extracted != efi_bytes {
        return Err(actionable_command_error(
            "nbuild",
            "generated UEFI GPT/FAT32 image does not reproduce the linked EFI application",
        )
        .into());
    }
    fs::write(&disk_path, &gpt_fat32)?;
    require_non_empty_output_file(
        "nbuild UEFI GPT/FAT32 physical image",
        disk_path.to_string_lossy().as_ref(),
    )?;

    Ok(serde_json::json!({
        "schema": "nc.kernel.uefi_loader.v1",
        "status": "physical_uefi_loader_materialized",
        "target": spec.triple,
        "firmware_architecture": target.coff_machine(),
        "removable_boot_path": format!("/EFI/BOOT/{}", target.removable_boot_name()),
        "entry": "efi_main(image_handle, system_table)",
        "calling_convention": if target == ncc_machine::UefiLoaderTarget::X86_64 { "UEFI Win64" } else { "UEFI AAPCS64" },
        "kernel": {
            "format": "ELF64 ET_EXEC/ET_DYN",
            "entry": nbuild_kernel_hex_u64(plan.entry),
            "sha256": sha256_hex_bytes(kernel_elf),
            "pt_load_segments": plan.segments.len(),
            "allocation_spans": plan.allocations.iter().map(|span| serde_json::json!({
                "base": nbuild_kernel_hex_u64(span.base),
                "pages": span.pages,
                "bytes": span.pages.saturating_mul(4096),
                "uefi_memory_type": if span.memory_type == 1 { "EfiLoaderCode" } else { "EfiLoaderData" }
            })).collect::<Vec<_>>()
        },
        "handoff": {
            "contract": "ncc_kernel_boot_info_v1",
            "memory": "GetMemoryMap final key, dynamic pool growth and ExitBootServices retry",
            "framebuffer": "EFI_GRAPHICS_OUTPUT_PROTOCOL",
            "configuration_tables": ["UEFI system table", "ACPI 1.0/2.0 RSDP", "SMBIOS 2/3", "FDT"],
            "command_line": "EFI_LOADED_IMAGE_PROTOCOL.LoadOptions UTF-16 -> retained UTF-8/ASCII",
            "kernel_loading": "AllocatePages(AllocateAddress, EfiLoaderCode/EfiLoaderData from PT_LOAD permissions) + PT_LOAD copy/BSS zero",
            "runtime_dependencies": []
        },
        "application": {
            "path": efi_path.display().to_string(),
            "size": efi_bytes.len(),
            "sha256": sha256_hex_file(&efi_path)?,
            "pe_relocations_present": efi_bytes.windows(6).any(|window| window == b".reloc"),
            "load_segments": load_plan.segments.len()
        },
        "disk": {
            "path": disk_path.display().to_string(),
            "format": "GPT with redundant CRC headers and FAT32 EFI System Partition",
            "size": gpt_fat32.len(),
            "sha256": sha256_hex_file(&disk_path)?,
            "boot_file": target.removable_boot_name(),
            "validated_payload_sha256": sha256_hex_bytes(&extracted),
            "smoke_superfloppy": {
                "path": smoke_disk_path.display().to_string(),
                "format": "FAT12 superfloppy",
                "size": fat12.len(),
                "sha256": sha256_hex_file(&smoke_disk_path)?
            }
        },
        "retained": {
            "llvm_ir": ll_path.display().to_string(),
            "llvm_sha256": sha256_hex_file(&ll_path)?,
            "object": obj_path.display().to_string(),
            "object_sha256": sha256_hex_file(&obj_path)?,
            "clang": clang,
            "lld_link": lld
        }
    }))
}

fn nbuild_kernel_boot_image_artifact(
    stem: &str,
    spec: NbuildKernelTargetSpec,
    kernel_path: &Path,
    bin_dir: &Path,
    obj_dir: &Path,
    lowering_dir: &Path,
    run_dir: &Path,
    kernel_elf: Option<&[u8]>,
) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error>> {
    if matches!(
        spec.triple,
        "aarch64-unknown-none" | "riscv64-unknown-elf" | "riscv32imac-unknown-elf"
    ) {
        let kernel_elf = kernel_elf.ok_or_else(|| {
            actionable_command_error(
                "nbuild",
                format!(
                    "{} QEMU virt boot requires the final kernel ELF bytes",
                    spec.triple
                ),
            )
        })?;
        let mut artifact =
            nbuild_kernel_qemu_virt_elf_artifact(stem, spec, kernel_path, run_dir, kernel_elf)?;
        if spec.triple == "aarch64-unknown-none" {
            let uefi = nbuild_kernel_uefi_artifact(
                stem,
                spec,
                kernel_elf,
                bin_dir,
                obj_dir,
                lowering_dir,
            )?;
            let disk_path = uefi
                .pointer("/disk/path")
                .and_then(serde_json::Value::as_str)
                .ok_or("AArch64 UEFI artifact is missing disk.path")?
                .to_string();
            let serial_path = artifact
                .pointer("/qemu_smoke/serial_log")
                .and_then(serde_json::Value::as_str)
                .ok_or("AArch64 boot artifact is missing qemu serial_log")?
                .to_string();
            artifact["uefi"] = uefi;
            artifact["execution_path"] = serde_json::json!({
                "mode": "UEFI GPT/FAT32 removable media",
                "firmware": "NCC_AAVMF_CODE or an auto-detected AAVMF/QEMU_EFI image",
                "disk": disk_path,
                "qemu_required": false,
                "physical_firmware_compatible": true
            });
            artifact["qemu_smoke"] = serde_json::json!({
                "runner": "qemu-system-aarch64",
                "timeout_seconds": 12,
                "serial_log": serial_path,
                "boot_mode": "uefi-gpt-fat32",
                "firmware_candidates": [
                    "NCC_AAVMF_CODE",
                    "/usr/share/AAVMF/AAVMF_CODE.no-secboot.fd",
                    "/usr/share/AAVMF/AAVMF_CODE.fd",
                    "/usr/share/qemu-efi-aarch64/QEMU_EFI.fd"
                ],
                "command": [
                    "qemu-system-aarch64",
                    "-M", "virt,accel=tcg",
                    "-cpu", "max",
                    "-m", "256M",
                    "-bios", "${NCC_AAVMF_CODE_OR_SYSTEM_AAVMF}",
                    "-drive", format!("if=none,format=raw,id=nccesp,file={disk_path}"),
                    "-device", "virtio-blk-device,drive=nccesp",
                    "-serial", "file:${SERIAL_LOG}",
                    "-display", "none",
                    "-monitor", "none",
                    "-no-reboot"
                ],
                "expected_serial_contains": "nC kernel AArch64 kmain",
                "expected_serial_sequence": ["nC kernel AArch64 kmain"]
            });
        }
        return Ok(Some(artifact));
    }
    if spec.triple == "i386-unknown-none-elf" {
        let kernel_elf = kernel_elf.ok_or_else(|| {
            actionable_command_error(
                "nbuild",
                "i386 kernel boot image requires the final kernel ELF32 executable bytes",
            )
        })?;
        return Ok(Some(nbuild_kernel_i386_boot_image_artifact(
            stem, bin_dir, run_dir, kernel_elf,
        )?));
    }
    if spec.triple != "x86_64-unknown-none" {
        return Ok(None);
    }

    let safe_target = spec.triple.replace(['/', '\\', ':'], "_");
    let boot_path = bin_dir.join(format!("{stem}.{safe_target}.boot.img"));
    let multiboot_path = bin_dir.join(format!("{stem}.{safe_target}.multiboot2.elf"));
    let ap_trampoline_path = bin_dir.join(format!("{stem}.{safe_target}.ap-trampoline.bin"));
    let multiboot_grub_path = run_dir.join(format!("{stem}.{safe_target}.grub.cfg"));
    let serial_path = run_dir.join(format!("{stem}.{safe_target}.qemu-serial.log"));
    let kernel_elf = kernel_elf.ok_or_else(|| {
        actionable_command_error(
            "nbuild",
            "x86_64 kernel boot image requires the final kernel ELF64 executable bytes",
        )
    })?;
    let elf_info = nbuild_kernel_elf64_image_info(kernel_elf)?;
    let multiboot = ncc_machine::build_multiboot2_x86_64_kernel(kernel_elf).map_err(|error| {
        actionable_command_error(
            "nbuild",
            format!("cannot materialize physical Multiboot2 kernel: {error}"),
        )
    })?;
    let ap_trampoline = ncc_machine::x86_64_ap_trampoline_image().map_err(|error| {
        actionable_command_error(
            "nbuild",
            format!("cannot materialize the x86_64 AP trampoline: {error}"),
        )
    })?;
    let image = nbuild_kernel_x86_64_stage0_stage1_elf_image_bytes(kernel_elf)?;
    let stage1_reserved_bytes = NBUILD_KERNEL_X86_STAGE1_RESERVED_BYTES;
    let kernel_elf_payload_offset = NBUILD_KERNEL_SECTOR_SIZE + stage1_reserved_bytes;
    let payload_sectors = (stage1_reserved_bytes + kernel_elf.len()).div_ceil(512);
    if image.len() < 1024 || image[510] != 0x55 || image[511] != 0xaa {
        return Err(actionable_command_error(
            "nbuild",
            "internal x86_64 kernel boot image must contain stage0, stage1, kernel ELF and BIOS signature 55 AA",
        )
        .into());
    }
    if image.get(kernel_elf_payload_offset..kernel_elf_payload_offset + kernel_elf.len())
        != Some(kernel_elf)
    {
        return Err(actionable_command_error(
            "nbuild",
            "internal x86_64 kernel boot image lost the exact kernel ELF payload bytes",
        )
        .into());
    }
    let stage1 = image
        .get(NBUILD_KERNEL_SECTOR_SIZE..NBUILD_KERNEL_SECTOR_SIZE + stage1_reserved_bytes)
        .ok_or_else(|| actionable_command_error("nbuild", "boot image is missing stage1 bytes"))?;
    let boot_info_offset =
        nbuild_kernel_find_unique_bytes(stage1, b"NCCBOO1", "stage1 boot_info magic")?;
    let boot_info_address = NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS + boot_info_offset as u64;
    let boot_info_flags =
        nbuild_kernel_read_u64_le(stage1, boot_info_offset + 16).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 boot_info flags are truncated")
        })?;
    let memory_map_address =
        nbuild_kernel_read_u64_le(stage1, boot_info_offset + 24).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 boot_info memory map ptr is truncated")
        })?;
    let memory_map_len =
        nbuild_kernel_read_u64_le(stage1, boot_info_offset + 32).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 boot_info memory map len is truncated")
        })?;
    let framebuffer_ptr =
        nbuild_kernel_read_u64_le(stage1, boot_info_offset + 40).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 boot_info framebuffer ptr is truncated")
        })?;
    let serial_ptr = nbuild_kernel_read_u64_le(stage1, boot_info_offset + 48).ok_or_else(|| {
        actionable_command_error("nbuild", "stage1 boot_info serial ptr is truncated")
    })?;
    let platform_table_ptr =
        nbuild_kernel_read_u64_le(stage1, boot_info_offset + 56).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 boot_info platform table ptr is truncated")
        })?;
    let command_line_ptr =
        nbuild_kernel_read_u64_le(stage1, boot_info_offset + 64).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 boot_info command line ptr is truncated")
        })?;
    let kernel_elf_load_address = nbuild_kernel_read_u64_le(stage1, boot_info_offset + 72)
        .ok_or_else(|| {
            actionable_command_error(
                "nbuild",
                "stage1 boot_info kernel ELF load ptr is truncated",
            )
        })?;
    let kernel_elf_size =
        nbuild_kernel_read_u64_le(stage1, boot_info_offset + 80).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 boot_info kernel ELF size is truncated")
        })?;
    if kernel_elf_load_address != NBUILD_KERNEL_X86_KERNEL_ELF_MEMORY_ADDRESS
        || kernel_elf_size != kernel_elf.len() as u64
    {
        return Err(actionable_command_error(
            "nbuild",
            "stage1 boot_info kernel ELF pointer/size does not match the retained payload",
        )
        .into());
    }
    let memory_map_entries =
        nbuild_kernel_parse_memory_regions_from_stage1(stage1, memory_map_address, memory_map_len)?;
    let serial_offset = serial_ptr
        .checked_sub(NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS)
        .ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 serial descriptor ptr is invalid")
        })? as usize;
    if stage1.get(serial_offset..serial_offset + 8) != Some(b"NCCSER1\0".as_slice()) {
        return Err(actionable_command_error(
            "nbuild",
            "stage1 serial descriptor magic is invalid",
        )
        .into());
    }
    let serial_version = nbuild_kernel_read_u32_le(stage1, serial_offset + 8).ok_or_else(|| {
        actionable_command_error("nbuild", "stage1 serial descriptor version is truncated")
    })?;
    let serial_kind = nbuild_kernel_read_u32_le(stage1, serial_offset + 12).ok_or_else(|| {
        actionable_command_error("nbuild", "stage1 serial descriptor kind is truncated")
    })?;
    let serial_base = nbuild_kernel_read_u64_le(stage1, serial_offset + 16).ok_or_else(|| {
        actionable_command_error("nbuild", "stage1 serial descriptor base is truncated")
    })?;
    let serial_irq = nbuild_kernel_read_u32_le(stage1, serial_offset + 24).ok_or_else(|| {
        actionable_command_error("nbuild", "stage1 serial descriptor irq is truncated")
    })?;
    let serial_baud = nbuild_kernel_read_u32_le(stage1, serial_offset + 28).ok_or_else(|| {
        actionable_command_error("nbuild", "stage1 serial descriptor baud is truncated")
    })?;
    let serial_flags = nbuild_kernel_read_u32_le(stage1, serial_offset + 32).ok_or_else(|| {
        actionable_command_error("nbuild", "stage1 serial descriptor flags are truncated")
    })?;
    let serial_reg_shift =
        nbuild_kernel_read_u32_le(stage1, serial_offset + 36).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 serial descriptor reg shift is truncated")
        })?;
    let platform_offset = platform_table_ptr
        .checked_sub(NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS)
        .ok_or_else(|| actionable_command_error("nbuild", "stage1 platform table ptr is invalid"))?
        as usize;
    if stage1.get(platform_offset..platform_offset + 8) != Some(b"NCCPLT1\0".as_slice()) {
        return Err(
            actionable_command_error("nbuild", "stage1 platform table magic is invalid").into(),
        );
    }
    let platform_version =
        nbuild_kernel_read_u32_le(stage1, platform_offset + 8).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 platform table version is truncated")
        })?;
    let platform_entry_count =
        nbuild_kernel_read_u32_le(stage1, platform_offset + 12).ok_or_else(|| {
            actionable_command_error("nbuild", "stage1 platform table count is truncated")
        })?;
    let platform_entries = nbuild_kernel_parse_platform_entries_from_stage1(
        stage1,
        platform_offset,
        platform_entry_count,
    )?;
    let acpi_rsdp_runtime_entry = platform_entries
        .iter()
        .find(|entry| entry["kind"] == "acpi_rsdp")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let bios_e820_runtime_entry = platform_entries
        .iter()
        .find(|entry| entry["kind"] == "bios_e820")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let boot_info_manifest = serde_json::json!({
        "layout": "ncc_kernel_boot_info_v1",
        "version": 1,
        "address": nbuild_kernel_hex_u64(boot_info_address),
        "offset_in_stage1": boot_info_offset,
        "size": NBUILD_KERNEL_X86_BOOT_INFO_SIZE,
        "argument_register": "rdi",
        "magic": "NCCBOO1",
        "flags": {
            "raw": boot_info_flags,
            "memory_map_present": boot_info_flags & 0x0001 != 0,
            "serial_present": boot_info_flags & 0x0002 != 0,
            "platform_table_present": boot_info_flags & 0x0004 != 0,
            "framebuffer_present": boot_info_flags & 0x0008 != 0,
        },
        "field_layout": [
            {"name": "magic", "offset": 0, "type": "u64"},
            {"name": "version", "offset": 8, "type": "u64"},
            {"name": "flags", "offset": 16, "type": "u64"},
            {"name": "memory_map_ptr", "offset": 24, "type": "ptr<ncc_kernel_memory_region_v1>"},
            {"name": "memory_map_len", "offset": 32, "type": "u64 entries"},
            {"name": "framebuffer_ptr", "offset": 40, "type": "ptr<ncc_kernel_framebuffer_v1>|0"},
            {"name": "serial_ptr", "offset": 48, "type": "ptr<ncc_kernel_serial_v1>|0"},
            {"name": "platform_table_ptr", "offset": 56, "type": "ptr<ncc_kernel_platform_table_v1>|0"},
            {"name": "command_line_ptr", "offset": 64, "type": "ptr<u8>|0"},
            {"name": "kernel_elf_load_address", "offset": 72, "type": "phys_addr"},
            {"name": "kernel_elf_size", "offset": 80, "type": "u64 bytes"}
        ],
        "memory_map": {
            "address": nbuild_kernel_hex_u64(memory_map_address),
            "entry_count": memory_map_len,
            "static_entry_count": memory_map_len,
            "runtime_entry_count": {
                "minimum": memory_map_len,
                "maximum": memory_map_len + NBUILD_KERNEL_X86_E820_MAX_ENTRIES as u64,
                "updated_by": "stage1 BIOS int15h E820 probe before kmain"
            },
            "entry_size": NBUILD_KERNEL_X86_MEMORY_REGION_SIZE,
            "entries": memory_map_entries,
            "runtime_extensions": {
                "bios_e820": {
                    "probe": "int15h-e820",
                    "buffer_entry_capacity": NBUILD_KERNEL_X86_E820_MAX_ENTRIES,
                    "buffer_entry_size": NBUILD_KERNEL_X86_MEMORY_REGION_SIZE,
                    "platform_entry": bios_e820_runtime_entry,
                    "updates_memory_map_len": true,
                    "success_serial_marker": "nC kernel E820 ok"
                }
            }
        },
        "framebuffer": {
            "present": framebuffer_ptr != 0,
            "ptr": nbuild_kernel_hex_u64(framebuffer_ptr),
            "reason_if_absent": if framebuffer_ptr == 0 { "bios-stage1 path has no linear framebuffer handoff yet" } else { "present" }
        },
        "serial": {
            "present": serial_ptr != 0,
            "ptr": nbuild_kernel_hex_u64(serial_ptr),
            "magic": "NCCSER1",
            "version": serial_version,
            "kind": if serial_kind == 1 { "x86_port_io" } else { "unknown" },
            "base": nbuild_kernel_hex_u64(serial_base),
            "irq": serial_irq,
            "baud": serial_baud,
            "flags": nbuild_kernel_memory_flags_json(serial_flags),
            "register_shift": serial_reg_shift
        },
        "platform_table": {
            "present": platform_table_ptr != 0,
            "ptr": nbuild_kernel_hex_u64(platform_table_ptr),
            "magic": "NCCPLT1",
            "version": platform_version,
            "entry_count": platform_entry_count,
            "entry_size": NBUILD_KERNEL_PLATFORM_ENTRY_SIZE,
            "entries": platform_entries,
            "runtime_probes": {
                "acpi_rsdp": {
                    "probe": "BIOS EBDA plus 0xe0000..0xfffff RSD PTR search",
                    "platform_entry": acpi_rsdp_runtime_entry,
                    "updates_platform_slot": true,
                    "success_serial_marker": "nC kernel ACPI RSDP ok"
                }
            },
            "external_tables": {
                "acpi_rsdp": "runtime-probed-by-stage1",
                "uefi_system_table": false,
                "multiboot2_tags": false,
                "bios_e820": "runtime-probed-by-stage1",
                "reason": "raw BIOS stage1 path uses the NCC-owned contract and probes ACPI RSDP/E820 itself; other external adapters populate typed slots when present"
            }
        },
        "command_line": {
            "present": command_line_ptr != 0,
            "ptr": nbuild_kernel_hex_u64(command_line_ptr)
        },
        "kernel_elf_payload": {
            "load_address": nbuild_kernel_hex_u64(kernel_elf_load_address),
            "size": kernel_elf_size
        },
        "stack": {
            "pointer_register": "rsp",
            "top": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_BOOT_STACK_TOP),
            "low": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_BOOT_STACK_LOW),
            "size": NBUILD_KERNEL_X86_BOOT_STACK_SIZE,
            "alignment": 16,
            "owner": "stage1-loader",
            "red_zone": "disabled-by-kernel-contract"
        }
    });

    fs::write(&boot_path, &image)?;
    fs::write(&multiboot_path, &multiboot.bytes)?;
    fs::write(&ap_trampoline_path, ap_trampoline.bytes)?;
    fs::write(
        &multiboot_grub_path,
        format!(
            "set timeout=0\nset default=0\nserial --unit=0 --speed=115200\nterminal_output console serial\nmenuentry 'nC physical kernel' {{\n    multiboot2 /boot/{}\n    boot\n}}\n",
            multiboot_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("kernel.multiboot2.elf")
        ),
    )?;
    require_non_empty_output_file(
        "nbuild kernel boot image",
        boot_path.to_string_lossy().as_ref(),
    )?;
    require_non_empty_output_file(
        "nbuild Multiboot2 kernel",
        multiboot_path.to_string_lossy().as_ref(),
    )?;
    require_non_empty_output_file(
        "nbuild x86_64 AP trampoline",
        ap_trampoline_path.to_string_lossy().as_ref(),
    )?;

    let mut artifact = serde_json::json!({
        "schema": "nc.kernel.boot_image.v1",
        "status": "stage0_stage1_elf64_kmain_image_written",
        "target": spec.triple,
        "format": "bios-stage0-stage1-elf64-loader",
        "entry_symbol": spec.entry_symbol(),
        "executes_entry_symbol": true,
        "entry_symbol_origin": "kernel-elf64-e_entry",
        "executes_machine_object_entry_symbol": true,
        "executes_kernel_elf_entry_symbol": true,
        "handoff_status": "stage0 reads stage1 plus the final kernel ELF64 executable from disk sector 2 into 0x8000; stage1 probes ACPI RSDP and BIOS int15h E820, enters long mode, parses ELF64 PT_LOAD program headers, copies segments to p_paddr, passes boot_info in rdi and calls e_entry",
        "cpu_mode": "x86_64-long-mode",
        "stage0_loader": {
            "kind": "x86_64-bios-stage0",
            "disk_read": "int13h-chs-sector-loop",
            "stage1_load_address": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS),
            "kernel_elf_memory_address": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_KERNEL_ELF_MEMORY_ADDRESS),
            "payload_start_sector": 2,
            "payload_sector_size": 512,
            "payload_sectors": payload_sectors,
            "payload_load_address": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS),
            "payload_max_stage0_chs_sectors": NBUILD_KERNEL_X86_MAX_STAGE0_CHS_PAYLOAD_SECTORS
        },
        "stage1_loader": {
            "kind": "x86_64-long-mode-elf64-loader",
            "load_address": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_STAGE1_LOAD_ADDRESS),
            "reserved_bytes": stage1_reserved_bytes,
            "kernel_elf_memory_address": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_KERNEL_ELF_MEMORY_ADDRESS),
            "initial_stack": boot_info_manifest["stack"].clone(),
            "parses": ["ELF64 header", "program headers", "PT_LOAD"],
            "runtime_probes": ["ACPI RSDP search", "BIOS int15h E820 memory map"],
            "segment_copy": "PT_LOAD p_offset -> p_paddr, then zero p_memsz - p_filesz",
            "entry_call": "e_entry",
            "return_contract": "rax == 0x4f4b"
        },
        "boot_info": boot_info_manifest,
        "kernel_elf": {
            "format": "ELF64_ET_EXEC",
            "payload_offset": kernel_elf_payload_offset,
            "payload_sector_size": 512,
            "payload_sectors": payload_sectors,
            "payload_max_stage0_chs_sectors": NBUILD_KERNEL_X86_MAX_STAGE0_CHS_PAYLOAD_SECTORS,
            "memory_address": nbuild_kernel_hex_u64(NBUILD_KERNEL_X86_KERNEL_ELF_MEMORY_ADDRESS),
            "size": kernel_elf.len(),
            "sha256": sha256_hex_bytes(kernel_elf),
            "entry": nbuild_kernel_hex_u64(elf_info.entry),
            "program_header_offset": elf_info.phoff,
            "program_header_entry_size": elf_info.phentsize,
            "program_header_count": elf_info.phnum,
            "load_segments": elf_info.load_segments.iter().map(|segment| serde_json::json!({
                "offset": segment.offset,
                "vaddr": nbuild_kernel_hex_u64(segment.vaddr),
                "paddr": nbuild_kernel_hex_u64(segment.paddr),
                "filesz": segment.filesz,
                "memsz": segment.memsz,
                "flags": segment.flags,
                "align": segment.align
            })).collect::<Vec<_>>(),
            "return_contract": "rax == 0x4f4b"
        },
        "path": boot_path.display().to_string(),
        "size": image.len(),
        "sha256": sha256_hex_file(&boot_path)?,
        "signature": "55aa",
        "serial_expected": "nC kernel ELF kmain",
        "serial_expected_sequence": ["nC kernel ACPI RSDP ok", "nC kernel E820 ok", "nC kernel ELF kmain"],
        "qemu_smoke": {
            "runner": "qemu-system-x86_64",
            "timeout_seconds": 6,
            "serial_log": serial_path.display().to_string(),
            "command": [
                "qemu-system-x86_64",
                "-display", "none",
                "-machine", "accel=tcg",
                "-serial", format!("file:{}", serial_path.display()),
                "-drive", format!("format=raw,file={},if=floppy", boot_path.display()),
                "-no-reboot"
            ],
            "expected_serial_contains": "nC kernel ELF kmain",
            "expected_serial_sequence": ["nC kernel ACPI RSDP ok", "nC kernel E820 ok", "nC kernel ELF kmain"]
        }
    });
    artifact["multiboot2"] = serde_json::json!({
        "schema": "nc.kernel.multiboot2.physical.v1",
        "status": "dual_bios_efi64_entry_materialized",
        "path": multiboot_path.display().to_string(),
        "size": multiboot.bytes.len(),
        "sha256": sha256_hex_file(&multiboot_path)?,
        "header_scan_limit": 32768,
        "architecture": "i386 protected-mode handoff to x86_64 long mode",
        "protected_entry": nbuild_kernel_hex_u64(multiboot.protected_entry),
        "efi64_entry": nbuild_kernel_hex_u64(multiboot.efi64_entry),
        "kernel_entry": nbuild_kernel_hex_u64(multiboot.kernel_entry),
        "stack_top": nbuild_kernel_hex_u64(multiboot.stack_top),
        "page_tables": "identity-mapped first 4 GiB with 2 MiB leaves",
        "handoff": {
            "boot_magic": "0x36d76289",
            "information_register": "ebx/rbx",
            "normalizer": "__ncc_kernel_normalize_boot_info",
            "result_contract": "eax == 0x4f4b",
            "serial_expected": "nC kernel Multiboot2 kmain"
        },
        "requested_information_tags": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21],
        "grub_config": multiboot_grub_path.display().to_string(),
        "physical_install": {
            "kernel_destination": format!("/boot/{}", multiboot_path.file_name().and_then(|name| name.to_str()).unwrap_or("kernel.multiboot2.elf")),
            "bootloader": "GRUB 2 Multiboot2 on BIOS or x86_64 UEFI",
            "qemu_required": false
        }
    });
    artifact["physical_smp"] = serde_json::json!({
        "schema": "nc.kernel.physical_smp.x86_64.v1",
        "status": "init_sipi_trampoline_materialized",
        "ap_trampoline": {
            "path": ap_trampoline_path.display().to_string(),
            "size": ap_trampoline.bytes.len(),
            "sha256": sha256_hex_file(&ap_trampoline_path)?,
            "load_address": nbuild_kernel_hex_u64(ap_trampoline.load_address),
            "mailbox_address": nbuild_kernel_hex_u64(ap_trampoline.mailbox_address),
            "mailbox_size": ncc_machine::SecondaryCpuMailboxV1::BYTE_SIZE,
            "entry_modes": ["real16", "protected32", "long64"],
            "page_table": "current BSP CR3 below 4 GiB",
            "startup": ["INIT assert", "INIT deassert", "SIPI", "SIPI"],
            "runtime_copy": "__ncc_kernel_x86_ap_prepare",
            "online_publication": "secondary entry calls cpuMarkOnline(context/logical_cpu)",
            "return_policy": "secondary entry return transitions the logical CPU to parked"
        }
    });
    artifact["uefi"] =
        nbuild_kernel_uefi_artifact(stem, spec, kernel_elf, bin_dir, obj_dir, lowering_dir)?;
    Ok(Some(artifact))
}

fn nbuild_kernel_machine_source(
    source_path: &str,
    container: &str,
    spec: NbuildKernelTargetSpec,
    semantic_entry: &str,
    image_path: &Path,
    body_lowered: bool,
    kernel_core_runtime: bool,
) -> String {
    let entry_symbol = spec.entry_symbol();
    let instr = nbuild_kernel_wrapper_instr(spec, body_lowered, kernel_core_runtime);
    let relocs = nbuild_kernel_body_reloc_section(spec, body_lowered, kernel_core_runtime);
    let externs = if body_lowered {
        if kernel_core_runtime {
            "\n    extern:\n        main:\n        __ncc_kernel_set_boot_info:\n"
        } else {
            "\n    extern:\n        main:\n"
        }
    } else {
        ""
    };
    let machine_source = format!(
        "{container} :: machine {{\n    target:\n        arch: {};\n        abi: {};\n        object: {};\n        endian: little;\n        pointerWidth: {};\n        profile: kernel;\n\n    debug:\n        file: \"{}\";\n        source: \"{}\";\n        entry: \"{}\";\n        producer: \"ncc build --profile kernel .n -> root -> .nm -> .nbo -> .obj -> kernel image\";\n        macho_unwind: compact;\n{externs}\n    symbol:\n        {entry_symbol}:\n            section: .text;\n            binding: global;\n            type: routine;\n\n    entry:\n        {entry_symbol};\n\n    proc:\n        {entry_symbol}:\n\n            instr:\n{instr}\n\n    verify:\n        target;\n        abi;\n        registers;\n        operands;\n        symbols;\n        encoding;\n        object;\n\n    seal:\n        artifact: kernel;\n        require: verified;\n        hash: required;\n\n    evidence:\n        source: \"{}\";\n        semanticEntry: \"{}\";\n        highLevelBodyLowered: \"{}\";\n        image: \"{}\";\n        kernelEntry: \"{}\";\n        loaderContract: \"{}\";\n        bootInfo: \"{}\";\n        initialStack: \"{}\";\n}}\n",
        spec.arch,
        spec.abi,
        spec.object,
        spec.pointer_width,
        nbuild_machine_string(source_path),
        nbuild_machine_string(source_path),
        nbuild_machine_string(semantic_entry),
        nbuild_machine_string(source_path),
        nbuild_machine_string(semantic_entry),
        if body_lowered { "true" } else { "false" },
        nbuild_machine_string(&image_path.display().to_string()),
        spec.entry_symbol(),
        nbuild_machine_string(spec.loader_protocol()),
        nbuild_machine_string(spec.boot_info_argument()),
        nbuild_machine_string(spec.stack_pointer_register()),
    );
    machine_source.replace("\n\n    verify:", &format!("\n{relocs}\n    verify:"))
}

fn nbuild_kernel_body_reloc_section(
    spec: NbuildKernelTargetSpec,
    body_lowered: bool,
    kernel_core_runtime: bool,
) -> String {
    if !body_lowered {
        return String::new();
    }
    match spec.arch {
        "aarch64" => {
            if kernel_core_runtime {
                "\n    reloc:\n        call_boot_info_setter:\n            type: R_AARCH64_CALL26;\n            symbol: __ncc_kernel_set_boot_info;\n            section: .text;\n            offset: 8;\n            addend: 0;\n        call_main:\n            type: R_AARCH64_CALL26;\n            symbol: main;\n            section: .text;\n            offset: 12;\n            addend: 0;\n"
                    .to_string()
            } else {
                "\n    reloc:\n        call_main:\n            type: R_AARCH64_CALL26;\n            symbol: main;\n            section: .text;\n            offset: 8;\n            addend: 0;\n"
                    .to_string()
            }
        }
        "armv7" | "armv7em" => {
            if kernel_core_runtime {
                "\n    reloc:\n        call_boot_info_setter:\n            type: R_ARM_THM_CALL;\n            symbol: __ncc_kernel_set_boot_info;\n            section: .text;\n            offset: 2;\n            addend: 0;\n        call_main:\n            type: R_ARM_THM_CALL;\n            symbol: main;\n            section: .text;\n            offset: 6;\n            addend: 0;\n"
                    .to_string()
            } else {
                "\n    reloc:\n        call_main:\n            type: R_ARM_THM_CALL;\n            symbol: main;\n            section: .text;\n            offset: 2;\n            addend: 0;\n"
                    .to_string()
            }
        }
        "i386" => {
            if kernel_core_runtime {
                "\n    reloc:\n        call_boot_info_setter:\n            type: R_386_PC32;\n            symbol: __ncc_kernel_set_boot_info;\n            section: .text;\n            offset: 8;\n            addend: -4;\n        call_main:\n            type: R_386_PC32;\n            symbol: main;\n            section: .text;\n            offset: 16;\n            addend: -4;\n"
                    .to_string()
            } else {
                "\n    reloc:\n        call_main:\n            type: R_386_PC32;\n            symbol: main;\n            section: .text;\n            offset: 1;\n            addend: -4;\n"
                    .to_string()
            }
        }
        "riscv32" | "riscv64" => {
            if kernel_core_runtime {
                "\n    reloc:\n        call_boot_info_setter:\n            type: R_RISCV_CALL;\n            symbol: __ncc_kernel_set_boot_info;\n            section: .text;\n            offset: 8;\n            addend: 0;\n        call_main:\n            type: R_RISCV_CALL;\n            symbol: main;\n            section: .text;\n            offset: 16;\n            addend: 0;\n"
                    .to_string()
            } else {
                "\n    reloc:\n        call_main:\n            type: R_RISCV_CALL;\n            symbol: main;\n            section: .text;\n            offset: 8;\n            addend: 0;\n"
                    .to_string()
            }
        }
        _ => String::new(),
    }
}

fn nbuild_validate_kernel_library_layers(
    ast: &ncc_ast::Program,
    allocator_available: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    for use_decl in &ast.uses {
        if let Err(code) =
            ncc_ast::validate_freestanding_library_path(&use_decl.path, allocator_available)
        {
            let path = use_decl.path.join(".");
            let detail = match code {
                "E_KERNEL_ALLOC_UNAVAILABLE_001" => format!(
                    "{code}: kernel import '{path}' belongs to alloc; configure the explicit freestanding allocator with --kernel-alloc"
                ),
                _ => format!(
                    "{code}: kernel import '{path}' belongs to stoud and would pull hosted OS/libc services into a freestanding image"
                ),
            };
            return Err(actionable_command_error("nbuild", detail).into());
        }
    }
    Ok(())
}

fn nbuild_kernel_machine_plan(
    spec: NbuildKernelTargetSpec,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if spec.arch == "wasm32" {
        return Ok(serde_json::json!({
            "status": "portable_stack_machine_profile",
            "target": spec.triple,
            "coordinator": "ncc-machine",
            "native_kernel_subsystems": false,
            "manual_authority": {
                "source_coordinator": ".n",
                "source_mode": "manual_raw",
                "machine_dependency_live": true,
                "native_register_authority": false,
                "compiler_inserts": [],
                "hosted_runtime": false
            }
        }));
    }
    let coordination_target = match spec.arch {
        "x86_64" => "x86_64-unknown-none",
        "i386" => "i386-unknown-none-elf",
        "aarch64" => "aarch64-unknown-none",
        "riscv64" => "riscv64-unknown-elf",
        "riscv32" => "riscv32imac-unknown-elf",
        "armv7" => "armv7a-unknown-none-eabi",
        "armv7em" => "armv7em-none-eabi",
        other => {
            return Err(actionable_command_error(
                "nbuild",
                format!("machine coordinator has no kernel architecture profile for '{other}'"),
            )
            .into())
        }
    };
    let plan = ncc_machine::coordinate_target_kernel(coordination_target).map_err(|err| {
        actionable_command_error(
            "nbuild",
            format!("E_KERNEL_MACHINE_COORDINATION_001: {err}"),
        )
    })?;
    let manual = plan.manual_authority();
    Ok(serde_json::json!({
        "status": "authorized",
        "coordinator": "ncc-machine",
        "requested_target": spec.triple,
        "machine_target": plan.profile.target,
        "architecture": plan.profile.architecture,
        "abi": plan.profile.abi,
        "pointer_width": plan.profile.pointer_width,
        "manifest": plan.canonical_manifest(),
        "subsystems": plan.ordered_subsystems.iter()
            .map(|subsystem| subsystem.canonical_name())
            .collect::<Vec<_>>(),
        "features": plan.required_features.iter()
            .map(|feature| feature.canonical_name())
            .collect::<Vec<_>>(),
        "registers": plan.profile.registers,
        "instructions": plan.profile.instructions,
        "relocations": plan.profile.relocations,
        "manual_authority": {
            "manifest": manual.canonical_manifest(),
            "source_coordinator": manual.source_coordinator,
            "source_mode": manual.source_mode,
            "compact_surface": manual.compact_surface,
            "machine_dependency_live": true,
            "raw_memory": manual.raw_memory,
            "unchecked_out_of_range": manual.unchecked_out_of_range,
            "unchecked_aliasing": manual.unchecked_aliasing,
            "explicit_resource_lifetime": manual.explicit_resource_lifetime,
            "explicit_interrupt_state": manual.explicit_interrupt_state,
            "explicit_register_state": manual.explicit_register_state,
            "explicit_abi": manual.explicit_abi,
            "explicit_relocations": manual.explicit_relocations,
            "compiler_inserts": [],
            "hidden_bounds_checks": manual.hidden_bounds_checks,
            "hidden_overflow_checks": manual.hidden_overflow_checks,
            "hidden_heap": manual.hidden_heap,
            "hidden_cleanup": manual.hidden_cleanup,
            "hidden_scheduler": manual.hidden_scheduler,
            "hosted_runtime": manual.hosted_runtime,
            "programmer_owns": [
                "memory-address-validity",
                "aliasing",
                "resource-lifetime",
                "interrupt-state",
                "register-clobbers",
                "calling-convention",
                "section-placement",
                "relocations"
            ],
            "target_rule": "only instructions, registers, relocations, and ABI forms materializable by the selected machine profile are accepted"
        },
    }))
}

fn run_nbuild_kernel_profile_command(
    path: &str,
    out_path: &str,
    artifact_root: &Path,
    target_override: Option<&str>,
    entry: Option<&str>,
    allocator_available: bool,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ast = load_ast_from_path(path)?;
    nbuild_validate_kernel_library_layers(&ast, allocator_available)?;
    sema_check_with_warnings(&ast)?;
    ncc_sema::validate_main_entries(&ast, ncc_sema::MainEntryRequirement::Required)
        .map_err(|err| -> Box<dyn std::error::Error> { err.into() })?;
    let source = fs::read_to_string(path)?;

    let spec = nbuild_kernel_target_spec(target_override)?;
    let basal_root =
        nbuild_materialize_basal_root(path, artifact_root, spec.pointer_width as usize)?;
    let machine_plan = nbuild_kernel_machine_plan(spec)?;
    let semantic_entry = resolve_native_entry_symbol(&ast, entry);
    let entry_contract = spec.entry_contract_json();
    let loader_contract = spec.loader_contract_json();
    let stem = nbuild_source_stem(path);
    let code_dir = artifact_root.join("code");
    let obj_dir = artifact_root.join("obj");
    let run_dir = artifact_root.join("run");
    let bin_dir = artifact_root.join("bin");
    let lowering_dir = artifact_root.join("lowering");
    let incremental_dir = artifact_root.join("incremental");
    let cache_dir = artifact_root.join("cache");
    let ide_dir = artifact_root.join("ide");
    for dir in [
        &code_dir,
        &obj_dir,
        &run_dir,
        &bin_dir,
        &lowering_dir,
        &incremental_dir,
        &cache_dir,
        &ide_dir,
    ] {
        fs::create_dir_all(dir)?;
    }
    super::compiler::compiler_native_remove_legacy_runtime_sources(&lowering_dir)?;
    let source_execution = nbuild_write_source_execution_artifact(path, artifact_root, &ast, true)?;

    let safe_target = spec.triple.replace(['/', '\\', ':'], "_");
    let machine_stem = nbuild_machine_ident(&format!("{stem}_{safe_target}_kernel"));
    let nm_path = lowering_dir.join(format!("{machine_stem}.nm"));
    let obj_path = obj_dir.join(format!("{stem}.{safe_target}.kernel.obj"));
    let manifest_path = run_dir.join(format!("{stem}.{safe_target}.kernel-build.json"));
    let map_path = run_dir.join(format!("{stem}.{safe_target}.kernel.map"));
    let lowering_path = lowering_dir.join(format!("{stem}.{safe_target}.kernel.lowering.json"));
    let legacy_obj_path = obj_dir.join(format!("{stem}.kernel.obj"));
    let legacy_manifest_path = run_dir.join(format!("{stem}.kernel-build.json"));
    let legacy_map_path = run_dir.join(format!("{stem}.kernel.map"));
    let legacy_lowering_path = lowering_dir.join(format!("{stem}.kernel.lowering.json"));

    let body_lowering = nbuild_kernel_compile_body_object(
        ast.clone(),
        path,
        &source,
        spec,
        &stem,
        &lowering_dir,
        &obj_dir,
    )?;
    let body_lowered = body_lowering.is_materialized();
    if allocator_available && !body_lowered {
        return Err(actionable_command_error(
            "nbuild",
            format!(
                "E_KERNEL_ALLOC_BODY_NOT_MATERIALIZED_001: the freestanding allocator was requested, but the high-level kernel body did not lower to a native object: {}",
                serde_json::to_string(&body_lowering.to_json())?
            ),
        )
        .into());
    }

    let nm_source = nbuild_kernel_machine_source(
        path,
        &machine_stem,
        spec,
        &semantic_entry,
        Path::new(out_path),
        body_lowered,
        body_lowering.kernel_core_runtime(),
    );
    fs::write(&nm_path, nm_source.as_bytes())?;
    require_non_empty_output_file(
        "nbuild kernel machine source",
        nm_path.to_string_lossy().as_ref(),
    )?;
    super::machine::check_machine_source_path(nm_path.to_string_lossy().as_ref())?;

    super::machine::emit_machine_source_object_to_path(
        nm_path.to_string_lossy().as_ref(),
        spec.triple,
        obj_path.to_string_lossy().as_ref(),
    )?;
    require_non_empty_output_file("nbuild kernel object", obj_path.to_string_lossy().as_ref())?;
    let target_init = nbuild_materialize_target_init(
        &stem,
        artifact_root,
        spec.triple,
        spec.arch,
        spec.abi,
        spec.object,
        spec.pointer_width as usize,
        spec.entry_symbol(),
        &nm_path,
        &obj_path,
        "kernel-machine-object",
    )?;

    ensure_output_parent_dir(out_path)?;
    let mut link_inputs = vec![obj_path.to_string_lossy().to_string()];
    if let Some(body) = body_lowering.materialized() {
        link_inputs.push(body.object_path.to_string_lossy().to_string());
    }
    let link_report = super::machine::link_machine_object_paths_to_kernel_artifact_silent(
        &link_inputs,
        spec.triple,
        out_path,
        Some(map_path.to_string_lossy().as_ref()),
    )?;
    require_non_empty_output_file("nbuild kernel image", out_path)?;
    if spec.system_loader_executable {
        release_native_executable_for_user(Path::new(out_path))?;
    }

    let kernel_elf_payload = if matches!(
        spec.triple,
        "x86_64-unknown-none"
            | "i386-unknown-none-elf"
            | "aarch64-unknown-none"
            | "riscv64-unknown-elf"
            | "riscv32imac-unknown-elf"
    ) {
        Some(fs::read(out_path)?)
    } else {
        None
    };
    let boot_artifact = nbuild_kernel_boot_image_artifact(
        &stem,
        spec,
        Path::new(out_path),
        &bin_dir,
        &obj_dir,
        &lowering_dir,
        &run_dir,
        kernel_elf_payload.as_deref(),
    )?;
    let boot_image_path = boot_artifact
        .as_ref()
        .and_then(|artifact| {
            if spec.triple == "aarch64-unknown-none" {
                artifact.pointer("/uefi/disk/path")
            } else {
                artifact.get("path")
            }
        })
        .and_then(|path| path.as_str())
        .map(str::to_string);
    let uefi_artifact = boot_artifact
        .as_ref()
        .and_then(|artifact| artifact.get("uefi"));
    let uefi_application_path = uefi_artifact
        .and_then(|artifact| artifact.pointer("/application/path"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let uefi_disk_path = uefi_artifact
        .and_then(|artifact| artifact.pointer("/disk/path"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let uefi_smoke_disk_path = uefi_artifact
        .and_then(|artifact| artifact.pointer("/disk/smoke_superfloppy/path"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let uefi_llvm_path = uefi_artifact
        .and_then(|artifact| artifact.pointer("/retained/llvm_ir"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let uefi_object_path = uefi_artifact
        .and_then(|artifact| artifact.pointer("/retained/object"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let opensbi_payload_path = boot_artifact
        .as_ref()
        .and_then(|artifact| artifact.pointer("/opensbi/flat_payload"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let multiboot2_path = boot_artifact
        .as_ref()
        .and_then(|artifact| artifact.pointer("/multiboot2/path"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let multiboot2_grub_path = boot_artifact
        .as_ref()
        .and_then(|artifact| artifact.pointer("/multiboot2/grub_config"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let ap_trampoline_path = boot_artifact
        .as_ref()
        .and_then(|artifact| artifact.pointer("/physical_smp/ap_trampoline/path"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let boot_image_materialized = boot_artifact.is_some();
    let boot_image_format = match spec.triple {
        "x86_64-unknown-none" if boot_image_materialized => "bios-stage0-stage1-elf64-loader",
        "i386-unknown-none-elf" if boot_image_materialized => "bios-stage0-stage1-elf32-loader",
        "aarch64-unknown-none" if boot_image_materialized => "uefi-pe-coff-gpt-fat32",
        "riscv64-unknown-elf" if boot_image_materialized => "opensbi-flat-next-stage-rv64",
        "riscv32imac-unknown-elf" if boot_image_materialized => "opensbi-flat-next-stage-rv32",
        _ => "not-materialized-for-target",
    };
    let boot_image_kmain_origin = match spec.triple {
        "x86_64-unknown-none" if boot_image_materialized => "kernel-elf64-e_entry",
        "i386-unknown-none-elf" if boot_image_materialized => "kernel-elf32-e_entry",
        "aarch64-unknown-none" | "riscv64-unknown-elf" if boot_image_materialized => {
            "kernel-elf64-e_entry"
        }
        "riscv32imac-unknown-elf" if boot_image_materialized => "kernel-elf32-e_entry",
        _ => "not-materialized-for-target",
    };
    let bootloader_implementation_scope = match spec.triple {
        "x86_64-unknown-none" if boot_image_materialized => {
            "x86_64-bios-stage0-stage1-elf64-loader"
        }
        "i386-unknown-none-elf" if boot_image_materialized => {
            "i386-bios-stage0-stage1-elf32-loader"
        }
        "aarch64-unknown-none" if boot_image_materialized => "aarch64-uefi-pe-coff-elf-loader",
        "riscv64-unknown-elf" if boot_image_materialized => "riscv64-opensbi-fdt-next-stage",
        "riscv32imac-unknown-elf" if boot_image_materialized => "riscv32-opensbi-fdt-next-stage",
        _ => "not-materialized-for-target",
    };
    let boot_image_long_mode_handoff =
        boot_image_materialized && spec.triple == "x86_64-unknown-none";
    let boot_image_protected_mode_handoff =
        boot_image_materialized && spec.triple == "i386-unknown-none-elf";
    let boot_image_page_tables_materialized = boot_image_long_mode_handoff;
    let internal_bootloader_materialized = boot_image_materialized
        && matches!(spec.triple, "x86_64-unknown-none" | "i386-unknown-none-elf");
    let qemu_virt_elf_loader_materialized = boot_image_materialized
        && matches!(
            spec.triple,
            "riscv64-unknown-elf" | "riscv32imac-unknown-elf"
        );
    let physical_uefi_loader_materialized = boot_image_materialized
        && matches!(spec.triple, "x86_64-unknown-none" | "aarch64-unknown-none");
    let artifact_magic = link_report
        .get("artifact_magic")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let kernel_elf64_executable = artifact_magic.starts_with("ELF64_");
    let kernel_elf32_executable = artifact_magic.starts_with("ELF32_");
    let kernel_link_artifact = link_report
        .get("kernel_link_artifact")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let body_lowering_json = body_lowering.to_json();
    let body_llvm_path = body_lowering
        .materialized()
        .map(|body| body.llvm_ir_path.display().to_string());
    let body_object_path = body_lowering
        .materialized()
        .map(|body| body.object_path.display().to_string());
    let library_layers = serde_json::json!({
        "core": {
            "enabled": true,
            "provider": "intrinsic-and-freestanding-runtime",
            "requires_os": false,
            "requires_libc": false
        },
        "alloc": {
            "enabled": allocator_available,
            "provider": if allocator_available {
                "ncc-internal-freestanding-allocator"
            } else {
                "disabled"
            },
            "requires_os": false,
            "requires_libc": false
        },
        "stoud": {
            "enabled": false,
            "provider": "forbidden-in-kernel-profile",
            "requires_os": true,
            "requires_libc": false
        }
    });

    let lowering_payload = serde_json::json!({
        "schema": "nc.compiler.kernel_lowering.v1",
        "source": path,
        "profile": "kernel",
        "target": spec.target_json(),
        "machine_plan": machine_plan.clone(),
        "semantic_entry": semantic_entry.clone(),
        "machine_entry": spec.entry_symbol(),
        "entry_contract": entry_contract.clone(),
        "loader_contract": loader_contract.clone(),
        "boot_artifact": boot_artifact.clone(),
        "machine_entry_source": nm_path.display().to_string(),
        "object": obj_path.display().to_string(),
        "high_level_body": body_lowering_json.clone(),
        "high_level_body_llvm_ir": body_llvm_path.clone(),
        "high_level_body_object": body_object_path.clone(),
        "image": out_path,
        "boot_image": boot_image_path.clone(),
        "library_layers": library_layers.clone(),
        "lowering": {
            "normal_n_source": true,
            "manual_raw_default": true,
            "compact_surface_is_source_syntax": true,
            "legacy_kernel_context_required": false,
            "kernel_import_required": false,
            "machine_dependency_live": true,
            "compiler_inserted_bounds_checks": false,
            "compiler_inserted_overflow_checks": false,
            "compiler_inserted_heap": false,
            "compiler_inserted_cleanup": false,
            "compiler_inserted_scheduler": false,
            "programmer_controls_registers_abi_and_relocations": true,
            "entry_contract_materialized_as_real_machine_bytes": true,
            "kmain_symbol_materialized": true,
            "loader_contract_materialized": true,
            "initial_stack_contract_materialized": true,
            "boot_info_layout_materialized": true,
            "boot_smoke_image_materialized": boot_image_materialized,
            "boot_smoke_image_executes_kmain": boot_image_materialized,
            "boot_image_long_mode_handoff": boot_image_long_mode_handoff,
            "boot_image_protected_mode_handoff": boot_image_protected_mode_handoff,
            "boot_image_boot_info_pointer_materialized": boot_image_materialized,
            "boot_image_executes_machine_object_kmain": boot_image_materialized,
            "boot_image_executes_kernel_elf_entry": boot_image_materialized,
            "boot_image_stage1_elf_loader_materialized": internal_bootloader_materialized,
            "boot_image_qemu_virt_elf_loader_materialized": qemu_virt_elf_loader_materialized,
            "boot_image_physical_uefi_loader_materialized": physical_uefi_loader_materialized,
            "full_high_level_kernel_body_instruction_lowering": body_lowered,
            "high_level_body_lowered_to_llvm": body_lowered,
            "high_level_body_lowered_to_native_object": body_lowered,
            "kmain_calls_high_level_main": body_lowered
        }
    });
    fs::write(
        &lowering_path,
        serde_json::to_string_pretty(&lowering_payload)?.as_bytes(),
    )?;

    let mut artifacts = vec![
        nbuild_artifact_record(&basal_root.source_path, "basal_root_source")?,
        nbuild_artifact_record(&basal_root.binary_path, "basal_root_binary")?,
        nbuild_artifact_record(&nm_path, "kernel_machine_source")?,
        nbuild_artifact_record(&obj_path, "kernel_object")?,
        nbuild_artifact_record(Path::new(out_path), "kernel_image")?,
        nbuild_artifact_record(&map_path, "kernel_map")?,
        nbuild_artifact_record(&lowering_path, "kernel_lowering")?,
    ];
    if let Some(body) = body_lowering.materialized() {
        artifacts.push(nbuild_artifact_record(
            &body.llvm_ir_path,
            "kernel_high_level_body_llvm_ir",
        )?);
        artifacts.push(nbuild_artifact_record(
            &body.object_path,
            "kernel_high_level_body_object",
        )?);
    }
    if let Some(path) = boot_image_path
        .as_ref()
        .filter(|path| Path::new(path) != Path::new(out_path))
    {
        artifacts.push(nbuild_artifact_record(
            Path::new(path),
            "kernel_boot_object_image",
        )?);
    }
    for (path, kind) in [
        (uefi_application_path.as_deref(), "kernel_uefi_application"),
        (uefi_disk_path.as_deref(), "kernel_uefi_physical_boot_disk"),
        (
            uefi_smoke_disk_path.as_deref(),
            "kernel_uefi_smoke_superfloppy",
        ),
        (uefi_llvm_path.as_deref(), "kernel_uefi_loader_llvm_ir"),
        (uefi_object_path.as_deref(), "kernel_uefi_loader_object"),
        (
            opensbi_payload_path.as_deref(),
            "kernel_opensbi_flat_payload",
        ),
        (multiboot2_path.as_deref(), "kernel_multiboot2_physical_elf"),
        (
            multiboot2_grub_path.as_deref(),
            "kernel_multiboot2_grub_config",
        ),
        (ap_trampoline_path.as_deref(), "kernel_x86_64_ap_trampoline"),
    ] {
        if let Some(path) = path {
            artifacts.push(nbuild_artifact_record(Path::new(path), kind)?);
        }
    }
    let mut payload = serde_json::json!({
        "schema": "nc.compiler.kernel_build.v1",
        "status": "kernel_build_passed",
        "source": path,
        "source_sha256": sha256_hex_file(Path::new(path))?,
        "artifact_root": artifact_root.display().to_string(),
        "profile": "kernel",
        "target": spec.target_json(),
        "machine_plan": machine_plan,
        "entry": semantic_entry,
        "machine_entry": spec.entry_symbol(),
        "entry_contract": entry_contract,
        "loader_contract": loader_contract,
        "boot_artifact": boot_artifact.clone(),
        "library_layers": library_layers,
        "paths": {
            "basal_root_source": basal_root.source_path.display().to_string(),
            "basal_root_binary": basal_root.binary_path.display().to_string(),
            "machine_source": nm_path.display().to_string(),
            "object": obj_path.display().to_string(),
            "high_level_body_llvm_ir": body_llvm_path,
            "high_level_body_object": body_object_path,
            "image": out_path,
            "boot_image": boot_image_path,
            "uefi_application": uefi_application_path,
            "uefi_disk": uefi_disk_path,
            "uefi_smoke_disk": uefi_smoke_disk_path,
            "uefi_loader_llvm_ir": uefi_llvm_path,
            "uefi_loader_object": uefi_object_path,
            "opensbi_flat_payload": opensbi_payload_path,
            "multiboot2_kernel": multiboot2_path,
            "multiboot2_grub_config": multiboot2_grub_path,
            "x86_64_ap_trampoline": ap_trampoline_path,
            "manifest": manifest_path.display().to_string(),
            "map": map_path.display().to_string(),
            "lowering": lowering_path.display().to_string()
        },
        "lowering_lineage": {
            "schema": "nc.lowering.source-root-machine.v2",
            "status": "materialized",
            "basal": nbuild_basal_root_lineage_payload(&basal_root),
            "machine": {
                "path": nm_path.display().to_string(),
                "sha256": sha256_hex_file(&nm_path)?,
                "container_matches_file_stem": true
            },
            "order": ["source.n", "root.nb", "root.nbb", "machine.nm", "kernel-object", "kernel-image"]
        },
        "pipeline": [
            "parse .n",
            "sema",
            "materialize Basal .nb root and verified NBB0 binary",
            "validate normal main entry",
            "lower full high-level .n body through MCT",
            "compile high-level body LLVM to native object when target admits it",
            "lower kernel entry contract to retained .nm",
            "machine check",
            "emit native object bytes",
            "link kernel image or link artifact"
        ],
        "link": {
            "route": "machine-phase12-kernel-profile",
            "phase12_report": link_report
        },
        "artifacts": artifacts,
        "kernel_contract": {
            "normal_n_source": true,
            "kernel_attribute_required": false,
            "kernel_import_required": false,
            "hosted_stoud_forbidden": true,
            "libc_linked": false,
            "alloc_requires_explicit_provider": true,
            "freestanding_allocator_enabled": allocator_available,
            "all_admitted_machine_targets": true,
            "object_is_real_machine_output": true,
            "high_level_body_object_is_real_native_output": body_lowered,
            "image_or_link_artifact_is_real_bytes": true,
            "entry_symbol": spec.entry_symbol(),
            "boot_info_layout": "ncc_kernel_boot_info_v1",
            "loader_contract": spec.loader_protocol(),
            "kernel_elf64_executable": kernel_elf64_executable,
            "kernel_elf32_executable": kernel_elf32_executable,
            "boot_smoke_image": boot_image_materialized,
            "boot_smoke_image_executes_kmain": boot_image_materialized,
            "boot_image_long_mode_handoff": boot_image_long_mode_handoff,
            "boot_image_protected_mode_handoff": boot_image_protected_mode_handoff,
            "boot_image_executes_machine_object_kmain": boot_image_materialized,
            "boot_image_executes_kernel_elf_entry": boot_image_materialized,
            "boot_image_qemu_virt_elf_loader_materialized": qemu_virt_elf_loader_materialized,
            "boot_image_physical_uefi_loader_materialized": physical_uefi_loader_materialized,
            "full_high_level_kernel_body_instruction_lowering": body_lowered,
            "kmain_calls_high_level_main": body_lowered
        },
        "truth_boundary": {
            "entry_contract_materialized_as_real_machine_bytes": true,
            "kmain_symbol_materialized": true,
            "loader_contract_materialized": true,
            "initial_stack_contract_declared": true,
            "memory_map_contract_declared": true,
            "framebuffer_contract_declared": true,
            "serial_contract_declared": true,
            "platform_table_contract_declared": true,
            "full_high_level_kernel_body_instruction_lowering": body_lowered,
            "high_level_body_lowered_to_llvm": body_lowered,
            "high_level_body_lowered_to_native_object": body_lowered,
            "kmain_calls_high_level_main": body_lowered,
            "boot_image": boot_image_materialized,
            "boot_image_format": boot_image_format,
            "boot_image_executes_kmain": boot_image_materialized,
            "boot_image_kmain_origin": boot_image_kmain_origin,
            "boot_image_executes_machine_object_kmain": boot_image_materialized,
            "boot_image_executes_kernel_elf_entry": boot_image_materialized,
            "boot_image_stage1_elf_loader_materialized": internal_bootloader_materialized,
            "boot_image_qemu_virt_elf_loader_materialized": qemu_virt_elf_loader_materialized,
            "boot_image_physical_uefi_loader_materialized": physical_uefi_loader_materialized,
            "boot_image_long_mode_handoff": boot_image_long_mode_handoff,
            "boot_image_protected_mode_handoff": boot_image_protected_mode_handoff,
            "boot_image_page_tables_materialized": boot_image_page_tables_materialized,
            "boot_image_gdt_materialized": internal_bootloader_materialized,
            "boot_image_boot_info_pointer_materialized": boot_image_materialized,
            "system_loader_executable": spec.system_loader_executable,
            "kernel_link_artifact": kernel_link_artifact,
            "kernel_elf64_executable": kernel_elf64_executable,
            "kernel_elf32_executable": kernel_elf32_executable,
            "bootloader_implementation": internal_bootloader_materialized,
            "qemu_virt_elf_loader_adapter": qemu_virt_elf_loader_materialized,
            "physical_uefi_loader_adapter": physical_uefi_loader_materialized,
            "bootloader_implementation_scope": bootloader_implementation_scope,
            "full_platform_bootloader_implementation": boot_image_materialized,
            "stage0_boot_smoke_implementation": internal_bootloader_materialized,
            "stage0_long_mode_bootloader_implementation": boot_image_long_mode_handoff,
            "stage0_protected_mode_bootloader_implementation": boot_image_protected_mode_handoff,
            "missing_bootloader_is_not_faked": true
        }
    });
    nbuild_attach_target_init_payload(&mut payload, &target_init)?;
    nbuild_attach_source_execution_payload(&mut payload, &source_execution)?;
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&payload)?.as_bytes(),
    )?;

    // Keep non-machine historical aliases for IDEs and scripts that consume
    // one target at a time. Machine lowering has one canonical .nm only.
    for (source, legacy) in [
        (&obj_path, &legacy_obj_path),
        (&manifest_path, &legacy_manifest_path),
        (&map_path, &legacy_map_path),
        (&lowering_path, &legacy_lowering_path),
    ] {
        fs::copy(source, legacy)?;
    }
    let object_metadata_path =
        PathBuf::from(format!("{}.machine-link-meta.json", obj_path.display()));
    if object_metadata_path.is_file() {
        fs::copy(
            &object_metadata_path,
            PathBuf::from(format!(
                "{}.machine-link-meta.json",
                legacy_obj_path.display()
            )),
        )?;
    }
    if let Some(body) = body_lowering.materialized() {
        fs::copy(
            &body.llvm_ir_path,
            lowering_dir.join(format!("{stem}.kernel.body.ll")),
        )?;
        let extension = body
            .object_path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("o");
        fs::copy(
            &body.object_path,
            obj_dir.join(format!("{stem}.kernel.body.{extension}")),
        )?;
    }

    if !crate::ncc_child_output_quiet() {
        if json {
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!(
                "OK: nbuild kernel image={} obj={} nm={} manifest={} target={} route=machine-phase12-kernel-profile",
                out_path,
                obj_path.display(),
                nm_path.display(),
                manifest_path.display(),
                spec.triple
            );
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub(crate) struct KernelEmulatorRunOutcome {
    pub(crate) exit_code: i32,
    pub(crate) route: &'static str,
}

#[derive(Debug, Clone)]
enum KernelQemuCommand {
    Native(PathBuf),
    Wsl(String),
}

fn kernel_qemu_responds(program: &Path) -> bool {
    Command::new(program)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn kernel_qemu_compatible_names(name: &str) -> Vec<&str> {
    match name {
        // The x86_64 system emulator boots both long-mode and 32-bit x86 guests.
        "qemu-system-i386" => vec!["qemu-system-i386", "qemu-system-x86_64"],
        _ => vec![name],
    }
}

fn kernel_serial_contains_exact_line(observed: &str, expected: &str) -> bool {
    observed
        .lines()
        .any(|line| line.trim_end_matches('\r') == expected)
}

#[cfg(test)]
mod kernel_qemu_discovery_tests {
    use super::{
        kernel_qemu_compatible_names, kernel_serial_contains_exact_line,
        nbuild_kernel_body_reloc_section, nbuild_kernel_i386_stage1_template_bytes,
        nbuild_kernel_target_spec, nbuild_kernel_wrapper_instr,
        nbuild_kernel_x86_64_stage1_acpi_e820_template_bytes,
    };

    #[test]
    fn i386_can_use_the_x86_64_system_emulator() {
        assert_eq!(
            kernel_qemu_compatible_names("qemu-system-i386"),
            ["qemu-system-i386", "qemu-system-x86_64"]
        );
        assert_eq!(
            kernel_qemu_compatible_names("qemu-system-aarch64"),
            ["qemu-system-aarch64"]
        );
    }

    #[test]
    fn i386_stage1_enables_x87_and_sse_before_calling_kmain() {
        let stage1 = nbuild_kernel_i386_stage1_template_bytes().expect("i386 stage1 must encode");
        let cpu_state_setup = [
            0x0f, 0x20, 0xc0, 0x83, 0xe0, 0xf3, 0x83, 0xc8, 0x22, 0x0f, 0x22, 0xc0, 0x0f, 0x20,
            0xe0, 0x0d, 0x00, 0x06, 0x00, 0x00, 0x0f, 0x22, 0xe0, 0xdb, 0xe3,
        ];
        assert!(
            stage1
                .windows(cpu_state_setup.len())
                .any(|window| window == cpu_state_setup),
            "i386 stage1 must clear CR0.EM/TS, enable CR4.OSFXSR/OSXMMEXCPT and run fninit"
        );
    }

    #[test]
    fn i386_stage1_memmoves_overlapping_elf_segments() {
        let stage1 = nbuild_kernel_i386_stage1_template_bytes().expect("i386 stage1 must encode");
        let backward_copy = [
            0x8d, 0x74, 0x0e, 0xff, // lea esi, [esi + ecx - 1]
            0x8d, 0x7c, 0x0f, 0xff, // lea edi, [edi + ecx - 1]
            0xfd, 0xf3, 0xa4, 0xfc, // std; rep movsb; cld
        ];
        let bss_start = [
            0xfc, 0xf3, 0xa4, // forward copy
            0x8b, 0x7d, 0x0c, // reload destination
            0x03, 0x7d, 0x10, // advance by filesz
        ];
        assert!(
            stage1
                .windows(backward_copy.len())
                .any(|window| window == backward_copy),
            "i386 ELF loader must copy destination-after-source segments backwards"
        );
        assert!(
            stage1
                .windows(bss_start.len())
                .any(|window| window == bss_start),
            "i386 ELF loader must rebuild the BSS start after either copy direction"
        );
    }

    #[test]
    fn i386_kernel_wrapper_zero_extends_boot_pointer_for_i64_setter() {
        let spec =
            nbuild_kernel_target_spec(Some("i386-unknown-none-elf")).expect("i386 kernel target");
        let instructions = nbuild_kernel_wrapper_instr(spec, true, true);
        assert!(instructions.contains("push 0;\n                push eax;"));
        assert!(instructions.contains("add esp, 8;"));

        let relocs = nbuild_kernel_body_reloc_section(spec, true, true);
        assert!(relocs.contains(
            "symbol: __ncc_kernel_set_boot_info;\n            section: .text;\n            offset: 8;"
        ));
        assert!(
            relocs.contains("symbol: main;\n            section: .text;\n            offset: 16;")
        );
    }

    #[test]
    fn x86_64_stage1_enables_sse_before_calling_kmain() {
        let stage1 = nbuild_kernel_x86_64_stage1_acpi_e820_template_bytes();
        let cr4_setup = [
            0x0f, 0x20, 0xe0, // mov eax, cr4
            0xb4, 0x06, // mov ah, 0x06: OSFXSR | OSXMMEXCPT
            0x0c, 0x20, // or al, 0x20: PAE
            0x0f, 0x22, 0xe0, // mov cr4, eax
        ];
        assert!(
            stage1
                .windows(cr4_setup.len())
                .any(|window| window == cr4_setup),
            "x86_64 stage1 must enable CR4.PAE/OSFXSR/OSXMMEXCPT before kmain"
        );
    }

    #[test]
    fn x86_64_stage1_identity_maps_the_first_four_gibibytes() {
        let stage1 = nbuild_kernel_x86_64_stage1_acpi_e820_template_bytes();
        let clear_six_table_pages = [
            0xbf, 0x00, 0x10, // mov di, 0x1000
            0xb9, 0x00, 0x18, // mov cx, 0x1800 (0x6000 bytes)
            0xfc, 0xf3, 0x66, 0xab, // cld; rep stosd
        ];
        let build_four_pdpt_entries = [
            0xbf, 0x00, 0x20, // mov di, 0x2000
            0x66, 0xb8, 0x03, 0x30, 0x00, 0x00, // mov eax, 0x3003
            0xb9, 0x04, 0x00, // mov cx, 4
            0x66, 0x89, 0x05, // mov [di], eax
            0x83, 0xc7, 0x08, // add di, 8
            0x66, 0x05, 0x00, 0x10, 0x00, 0x00, // add eax, 0x1000
            0xe2, 0xf2, // loop
        ];
        let build_two_thousand_forty_eight_pdes = [
            0xbf, 0x00, 0x30, // mov di, 0x3000
            0x66, 0xb8, 0x83, 0x00, 0x00, 0x00, // mov eax, 0x83
            0xb9, 0x00, 0x08, // mov cx, 2048
            0x66, 0x89, 0x05, // mov [di], eax
            0x83, 0xc7, 0x08, // add di, 8
            0x66, 0x05, 0x00, 0x00, 0x20, 0x00, // add eax, 2 MiB
            0xe2, 0xf2, // loop
        ];
        for (name, sequence) in [
            ("page-table clear", clear_six_table_pages.as_slice()),
            ("PDPT construction", build_four_pdpt_entries.as_slice()),
            (
                "2 MiB page-directory construction",
                build_two_thousand_forty_eight_pdes.as_slice(),
            ),
        ] {
            assert!(
                stage1
                    .windows(sequence.len())
                    .any(|window| window == sequence),
                "x86_64 stage1 is missing {name} for its 4 GiB identity map"
            );
        }
    }

    #[test]
    fn kernel_serial_success_requires_a_complete_matching_line() {
        assert!(kernel_serial_contains_exact_line(
            "nC kernel ACPI RSDP ok\r\nnC kernel ELF kmain\r\n",
            "nC kernel ELF kmain"
        ));
        assert!(!kernel_serial_contains_exact_line(
            "nC kernel ELF kmain bad\r\n",
            "nC kernel ELF kmain"
        ));
    }
}

fn kernel_find_qemu_command(name: &str) -> Result<KernelQemuCommand, Box<dyn std::error::Error>> {
    let compatible_names = kernel_qemu_compatible_names(name);
    for candidate in &compatible_names {
        let path_candidate = PathBuf::from(candidate);
        if kernel_qemu_responds(&path_candidate) {
            return Ok(KernelQemuCommand::Native(path_candidate));
        }

        if let Some(home) = env::var_os("HOME") {
            let local = PathBuf::from(home)
                .join(".local")
                .join("bin")
                .join(candidate);
            if kernel_qemu_responds(&local) {
                return Ok(KernelQemuCommand::Native(local));
            }
        }
    }

    if cfg!(windows) {
        for candidate in &compatible_names {
            let probe = format!(
                "command -v {candidate} 2>/dev/null || if [ -x \"$HOME/.local/bin/{candidate}\" ]; then printf '%s\\n' \"$HOME/.local/bin/{candidate}\"; fi"
            );
            let output = Command::new("wsl.exe")
                .args(["sh", "-lc", &probe])
                .stdin(Stdio::null())
                .output()?;
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Ok(KernelQemuCommand::Wsl(path));
                }
            }
        }
    }

    Err(actionable_command_error(
        "run",
        format!(
            "kernel execution requires one of [{}]; install QEMU natively or in the active WSL distribution",
            compatible_names.join(", ")
        ),
    )
    .into())
}

fn kernel_find_aarch64_uefi_firmware(
    qemu: &KernelQemuCommand,
) -> Result<String, Box<dyn std::error::Error>> {
    const CANDIDATES: &[&str] = &[
        "/usr/share/AAVMF/AAVMF_CODE.no-secboot.fd",
        "/usr/share/AAVMF/AAVMF_CODE.fd",
        "/usr/share/qemu-efi-aarch64/QEMU_EFI.fd",
    ];

    match qemu {
        KernelQemuCommand::Native(_) => {
            if let Some(configured) = env::var_os("NCC_AAVMF_CODE") {
                let path = PathBuf::from(configured);
                if path.is_file() {
                    return Ok(path.display().to_string());
                }
            }
            for candidate in CANDIDATES {
                if Path::new(candidate).is_file() {
                    return Ok((*candidate).to_string());
                }
            }
        }
        KernelQemuCommand::Wsl(_) => {
            let probe = r#"for p in "$NCC_AAVMF_CODE" /usr/share/AAVMF/AAVMF_CODE.no-secboot.fd /usr/share/AAVMF/AAVMF_CODE.fd /usr/share/qemu-efi-aarch64/QEMU_EFI.fd; do if [ -n "$p" ] && [ -f "$p" ]; then printf '%s' "$p"; exit 0; fi; done; exit 1"#;
            let output = Command::new("wsl.exe")
                .args(["--exec", "sh", "-lc", probe])
                .stdin(Stdio::null())
                .output()?;
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Ok(path);
                }
            }
        }
    }

    Err(actionable_command_error(
        "run",
        "AArch64 UEFI execution requires AAVMF_CODE; install AAVMF/qemu-efi-aarch64 or set NCC_AAVMF_CODE",
    )
    .into())
}

fn kernel_wsl_path(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let absolute = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let output = Command::new("wsl.exe")
        .args(["--exec", "wslpath", "-a", "-u"])
        .arg(&absolute)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "run could not translate '{}' into the active WSL filesystem",
            absolute.display()
        )
        .into());
    }
    let translated = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if translated.is_empty() {
        return Err(format!(
            "run received an empty WSL path for '{}'",
            absolute.display()
        )
        .into());
    }
    Ok(translated)
}

fn kernel_default_run_target() -> Result<&'static str, Box<dyn std::error::Error>> {
    match std::env::consts::ARCH {
        "x86_64" => Ok("x86_64-unknown-none"),
        "x86" => Ok("i386-unknown-none-elf"),
        arch => Err(actionable_command_error(
            "run",
            format!(
                "kernel run has no implicit boot-image target for host architecture '{}'; pass --target with an executable kernel target",
                arch
            ),
        )
        .into()),
    }
}

pub(crate) fn build_and_run_kernel_source(
    source_path: &Path,
    target_override: Option<&str>,
    entry: Option<&str>,
    allocator_available: bool,
) -> Result<KernelEmulatorRunOutcome, Box<dyn std::error::Error>> {
    let source_path = fs::canonicalize(source_path).unwrap_or_else(|_| source_path.to_path_buf());
    let source = source_path.to_string_lossy().to_string();
    let target = target_override.unwrap_or(kernel_default_run_target()?);
    let spec = nbuild_kernel_target_spec(Some(target))?;
    let artifact_root = source_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".ncc");
    let output_path = default_nbuild_kernel_output_path(&source, &artifact_root, spec);
    run_nbuild_kernel_profile_command(
        &source,
        &output_path,
        &artifact_root,
        Some(spec.triple),
        entry,
        allocator_available,
        false,
    )?;

    let stem = nbuild_source_stem(&source);
    let safe_target = spec.triple.replace(['/', '\\', ':'], "_");
    let manifest_path = artifact_root
        .join("run")
        .join(format!("{stem}.{safe_target}.kernel-build.json"));
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let smoke = manifest
        .get("boot_artifact")
        .and_then(|value| value.get("qemu_smoke"))
        .ok_or_else(|| {
            actionable_command_error(
                "run",
                format!(
                    "target '{}' produced a real kernel link artifact but no directly bootable QEMU image; use `ncc build` for this target",
                    spec.triple
                ),
            )
        })?;
    let qemu_name = smoke
        .get("runner")
        .and_then(serde_json::Value::as_str)
        .ok_or("kernel build manifest is missing qemu_smoke.runner")?;
    let serial_path = smoke
        .get("serial_log")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .ok_or("kernel build manifest is missing qemu_smoke.serial_log")?;
    let boot_path = if spec.triple == "aarch64-unknown-none" {
        manifest
            .pointer("/boot_artifact/uefi/disk/path")
            .and_then(serde_json::Value::as_str)
            .map(PathBuf::from)
            .ok_or("AArch64 kernel build manifest is missing boot_artifact.uefi.disk.path")?
    } else {
        manifest
            .get("paths")
            .and_then(|value| value.get("boot_image"))
            .and_then(serde_json::Value::as_str)
            .map(PathBuf::from)
            .ok_or("kernel build manifest is missing paths.boot_image")?
    };
    let expected = smoke
        .get("expected_serial_contains")
        .and_then(serde_json::Value::as_str)
        .ok_or("kernel build manifest is missing qemu_smoke.expected_serial_contains")?;
    let timeout_seconds = smoke
        .get("timeout_seconds")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(6);

    if let Some(parent) = serial_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if serial_path.exists() {
        fs::remove_file(&serial_path)?;
    }

    let qemu = kernel_find_qemu_command(qemu_name)?;
    let aarch64_uefi_firmware = if spec.triple == "aarch64-unknown-none" {
        Some(kernel_find_aarch64_uefi_firmware(&qemu)?)
    } else {
        None
    };
    let (mut command, route, rendered_program, rendered_boot, rendered_serial) = match qemu {
        KernelQemuCommand::Native(program) => {
            let rendered_program = program.display().to_string();
            (
                Command::new(&program),
                "kernel-qemu-native",
                rendered_program,
                boot_path.display().to_string(),
                serial_path.display().to_string(),
            )
        }
        KernelQemuCommand::Wsl(program) => {
            let boot = kernel_wsl_path(&boot_path)?;
            let serial = kernel_wsl_path(&serial_path)?;
            let mut command = Command::new("wsl.exe");
            command.args(["--exec", &program]);
            (command, "kernel-qemu-wsl", program, boot, serial)
        }
    };
    match spec.triple {
        "x86_64-unknown-none" | "i386-unknown-none-elf" => {
            command
                .args([
                    "-display",
                    "none",
                    "-machine",
                    "accel=tcg",
                    "-monitor",
                    "none",
                ])
                .arg("-serial")
                .arg(format!("file:{rendered_serial}"))
                .arg("-drive")
                .arg(format!("format=raw,file={rendered_boot},if=floppy"))
                .arg("-no-reboot");
        }
        "aarch64-unknown-none" => {
            let firmware = aarch64_uefi_firmware
                .as_deref()
                .ok_or("AArch64 UEFI firmware path was not resolved")?;
            command
                .args([
                    "-M",
                    "virt,accel=tcg",
                    "-cpu",
                    "max",
                    "-m",
                    "128M",
                    "-display",
                    "none",
                    "-monitor",
                    "none",
                    "-bios",
                    firmware,
                ])
                .arg("-serial")
                .arg(format!("file:{rendered_serial}"))
                .arg("-drive")
                .arg(format!("if=none,format=raw,id=nccesp,file={rendered_boot}"))
                .args(["-device", "virtio-blk-device,drive=nccesp"])
                .arg("-no-reboot");
        }
        "riscv64-unknown-elf" | "riscv32imac-unknown-elf" => {
            let cpu = if spec.arch == "riscv64" {
                "rv64"
            } else {
                "rv32"
            };
            command
                .args([
                    "-M",
                    "virt,accel=tcg",
                    "-bios",
                    "default",
                    "-cpu",
                    cpu,
                    "-m",
                    "128M",
                    "-display",
                    "none",
                    "-monitor",
                    "none",
                ])
                .arg("-serial")
                .arg(format!("file:{rendered_serial}"))
                .arg("-kernel")
                .arg(&rendered_boot)
                .arg("-no-reboot");
        }
        other => {
            return Err(actionable_command_error(
                "run",
                format!("target '{other}' has no directly executable QEMU boot adapter"),
            )
            .into())
        }
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = command.spawn().map_err(|error| {
        format!(
            "kernel QEMU process '{}' failed to start: {}",
            rendered_program, error
        )
    })?;
    let deadline = Instant::now() + Duration::from_secs(timeout_seconds);
    let mut observed = String::new();
    let mut exited = None;
    while Instant::now() < deadline {
        if let Ok(bytes) = fs::read(&serial_path) {
            observed = String::from_utf8_lossy(&bytes).to_string();
            if kernel_serial_contains_exact_line(&observed, expected) {
                let _ = child.kill();
                let _ = child.wait();
                print!("{observed}");
                let evidence_path = artifact_root
                    .join("run")
                    .join(format!("{stem}.kernel-run.json"));
                let evidence = serde_json::json!({
                    "schema": "nc.compiler.kernel_run.v1",
                    "status": "kernel_run_passed",
                    "source": source,
                    "target": spec.triple,
                    "route": route,
                    "emulator": rendered_program,
                    "boot_image": boot_path.display().to_string(),
                    "serial_log": serial_path.display().to_string(),
                    "expected_serial": expected,
                    "observed_serial": observed,
                    "exit_code": 0
                });
                fs::write(evidence_path, serde_json::to_vec_pretty(&evidence)?)?;
                return Ok(KernelEmulatorRunOutcome {
                    exit_code: 0,
                    route,
                });
            }
        }
        if let Some(status) = child.try_wait()? {
            exited = Some(status);
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let _ = child.kill();
    let _ = child.wait();
    Err(actionable_command_error(
        "run",
        format!(
            "kernel QEMU smoke did not observe '{}' (status={:?}, serial='{}')",
            expected,
            exited,
            observed.trim()
        ),
    )
    .into())
}

fn run_nbuild_machine_source_command(
    path: &str,
    out_path: &str,
    artifact_root: &Path,
    target_override: Option<&str>,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let stem = nbuild_source_stem(path);
    let code_dir = artifact_root.join("code");
    let obj_dir = artifact_root.join("obj");
    let run_dir = artifact_root.join("run");
    let bin_dir = artifact_root.join("bin");
    for dir in [&code_dir, &obj_dir, &run_dir, &bin_dir] {
        fs::create_dir_all(dir)?;
    }

    let target = match target_override {
        Some(target) => target.to_string(),
        None => super::machine::machine_source_declared_target(path)?,
    };
    let nm_path = PathBuf::from(path);
    let obj_path = obj_dir.join(format!("{stem}.obj"));
    let manifest_path = run_dir.join(format!("{stem}.native-build.json"));
    let map_path = run_dir.join(format!("{stem}.machine.map"));

    require_non_empty_output_file("nbuild machine source", nm_path.to_string_lossy().as_ref())?;
    super::machine::check_machine_source_path(nm_path.to_string_lossy().as_ref())?;

    super::machine::emit_machine_source_object_to_path(
        nm_path.to_string_lossy().as_ref(),
        &target,
        obj_path.to_string_lossy().as_ref(),
    )?;
    require_non_empty_output_file("nbuild object", obj_path.to_string_lossy().as_ref())?;

    ensure_output_parent_dir(out_path)?;
    let link_report = super::machine::link_machine_object_paths_to_os_executable_silent(
        &[obj_path.to_string_lossy().to_string()],
        &target,
        out_path,
        Some(map_path.to_string_lossy().as_ref()),
    )?;
    require_non_empty_output_file("nbuild executable", out_path)?;
    release_native_executable_for_user(Path::new(out_path))?;
    let launcher_artifacts = write_native_launcher_artifacts(
        Path::new(path),
        &stem,
        Path::new(out_path),
        &code_dir,
        &bin_dir,
    )?;

    let artifacts = vec![
        nbuild_artifact_record(&nm_path, "machine_source")?,
        nbuild_artifact_record(&launcher_artifacts.launcher_n, "launcher_n")?,
        nbuild_artifact_record(&obj_path, "object")?,
        nbuild_artifact_record(Path::new(out_path), "executable")?,
        nbuild_artifact_record(&launcher_artifacts.powershell, "powershell_launcher")?,
        nbuild_artifact_record(&launcher_artifacts.batch, "batch_launcher")?,
    ];
    let payload = serde_json::json!({
        "schema": "nc.compiler.standard_machine_build.v1",
        "status": "native_build_passed",
        "source": path,
        "source_sha256": sha256_hex_file(Path::new(path))?,
        "machine_source_materialization": "original_source_reused_without_copy",
        "artifact_root": artifact_root.display().to_string(),
        "target": target,
        "paths": {
            "machine_source": nm_path.display().to_string(),
            "launcher_n": launcher_artifacts.launcher_n.display().to_string(),
            "object": obj_path.display().to_string(),
            "executable": out_path,
            "powershell_launcher": launcher_artifacts.powershell.display().to_string(),
            "batch_launcher": launcher_artifacts.batch.display().to_string(),
            "manifest": manifest_path.display().to_string(),
            "map": map_path.display().to_string(),
        },
        "pipeline": [
            "parse .nm",
            "machine check",
            "emit native object bytes",
            "link native executable internally"
        ],
        "link": {
            "route": "machine-phase12-internal",
            "phase12_report": link_report,
        },
        "artifacts": artifacts,
        "speed_contract": {
            "normal_build_surface": true,
            "interpreter_fallback": false,
            "native_machine_execution": true,
            "c_like_artifact_set": [".nm", ".obj", ".exe"]
        },
        "truth_boundary": {
            "machine_source_is_original_input": true,
            "machine_source_copy_emitted": false,
            "object_is_real_machine_output": true,
            "executable_is_native_os_loader_direct": true,
            "launcher_n_generated": true,
            "host_shell_launchers_generated": [".ps1", ".bat"],
            "executable_released_for_external_run": true,
            "missing_tools_are_not_faked": true
        }
    });
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&payload)?.as_bytes(),
    )?;

    if !crate::ncc_child_output_quiet() {
        if json {
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!(
                "OK: nbuild machine exe={} obj={} nm={} manifest={} route=machine-phase12-internal",
                out_path,
                obj_path.display(),
                nm_path.display(),
                manifest_path.display()
            );
        }
    }
    Ok(())
}

pub(crate) fn run_nbuild_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!(
            "{}",
            command_help("nbuild").expect("nbuild help must exist")
        );
        return Ok(());
    }
    if args.is_empty() {
        return Err(actionable_command_error(
            "nbuild",
            "nbuild syntax: nbuild <file.n|file.nm> [out-exe] [--out-dir <dir>] [--json] [--real|--native-fast] [--profile <release-fast|kernel>] [--kernel-alloc] [--cc <profile>] [--target <triple>] [--linker <bin>] [--entry <symbol>]",
        )
        .into());
    }
    let invocation_dir = std::env::current_dir().map_err(|err| {
        actionable_command_error(
            "nbuild",
            format!("nbuild could not resolve its invocation directory: {err}"),
        )
    })?;

    let mut path: Option<String> = None;
    let mut out_path: Option<String> = None;
    let mut artifact_root: Option<PathBuf> = None;
    let mut profile_args: Vec<String> = Vec::new();
    let mut execution_profile = NccExecutionProfile::ReleaseFast;
    let mut linker: Option<String> = None;
    let mut entry: Option<String> = None;
    let mut target_override: Option<String> = None;
    let mut real_requested = false;
    let mut kernel_profile_requested = false;
    let mut kernel_allocator_available = false;
    let mut json = false;
    let mut quiet_output = false;
    let mut opt_level = "backend-default".to_string();
    let mut content_cache_enabled = true;
    let mut content_cache_dir: Option<PathBuf> = None;

    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "-h" | "--help" | "help" => {
                println!(
                    "{}",
                    command_help("nbuild").expect("nbuild help must exist")
                );
                return Ok(());
            }
            "--real" | "--native" | "--native-fast" => {
                real_requested = true;
                execution_profile = NccExecutionProfile::ReleaseFast;
                idx += 1;
            }
            "--json" => {
                json = true;
                idx += 1;
            }
            "--quiet" => {
                quiet_output = true;
                idx += 1;
            }
            "--incremental" => {
                content_cache_enabled = true;
                idx += 1;
            }
            "--no-incremental" => {
                content_cache_enabled = false;
                idx += 1;
            }
            "--content-cache-dir" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "nbuild",
                        "nbuild syntax: --content-cache-dir requires a path",
                    )
                })?;
                content_cache_dir = Some(PathBuf::from(value));
                idx += 2;
            }
            "--kernel-alloc" => {
                kernel_allocator_available = true;
                idx += 1;
            }
            "--out-dir" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("nbuild", "nbuild syntax: --out-dir requires a path")
                })?;
                artifact_root = Some(PathBuf::from(value.clone()));
                idx += 2;
            }
            "--interpreted" | "--runtime" => {
                return Err(actionable_command_error(
                    "nbuild",
                    "nbuild: build is backend-only; use `ncc run <file.n> --interpreted` for the runtime executor",
                )
                .into());
            }
            "--cc" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("nbuild", "nbuild syntax: --cc requires a value")
                })?;
                profile_args.push("--cc".to_string());
                profile_args.push(value.clone());
                idx += 2;
            }
            "--cpu" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("nbuild", "nbuild syntax: --cpu requires a value")
                })?;
                profile_args.push("--cpu".to_string());
                profile_args.push(value.clone());
                idx += 2;
            }
            "--tune-profile" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "nbuild",
                        "nbuild syntax: --tune-profile requires a value",
                    )
                })?;
                profile_args.push("--tune-profile".to_string());
                profile_args.push(value.clone());
                idx += 2;
            }
            "--profile" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("nbuild", "nbuild syntax: --profile requires a value")
                })?;
                if matches!(
                    value.to_ascii_lowercase().as_str(),
                    "kernel" | "kernel-fast" | "freestanding-kernel"
                ) {
                    kernel_profile_requested = true;
                    execution_profile = NccExecutionProfile::ReleaseFast;
                } else {
                    execution_profile = parse_ncc_profile_value(args, idx, "nbuild")?;
                }
                idx += 2;
            }
            "--opt-level" | "-O" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error(
                        "nbuild",
                        "nbuild syntax: --opt-level requires a value",
                    )
                })?;
                opt_level = value.clone();
                idx += 2;
            }
            raw if raw.starts_with("-O") && raw.len() > 2 => {
                opt_level = raw.trim_start_matches("-O").to_string();
                idx += 1;
            }
            "--linker" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("nbuild", "nbuild syntax: --linker requires a value")
                })?;
                linker = Some(value.clone());
                idx += 2;
            }
            "--entry" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("nbuild", "nbuild syntax: --entry requires a value")
                })?;
                entry = Some(value.clone());
                idx += 2;
            }
            "--target" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("nbuild", "nbuild syntax: --target requires a value")
                })?;
                target_override = Some(value.clone());
                idx += 2;
            }
            other if other.starts_with("--") => {
                return Err(actionable_command_error(
                    "nbuild",
                    format!("nbuild: unknown option '{}'", other),
                )
                .into());
            }
            _ => {
                if path.is_none() {
                    path = Some(args[idx].clone());
                    idx += 1;
                    continue;
                }
                if out_path.is_none() {
                    out_path = Some(args[idx].clone());
                    idx += 1;
                    continue;
                }
                return Err(actionable_command_error(
                    "nbuild",
                    "nbuild command expects one source file and at most one output executable path",
                )
                .into());
            }
        }
    }

    let _quiet_guard = quiet_output.then(crate::NccOutputQuietGuard::activate);

    let path = path.ok_or_else(|| {
        actionable_command_error(
            "nbuild",
            "nbuild syntax: missing source file path before build options",
        )
    })?;
    let path = {
        let candidate = PathBuf::from(&path);
        if candidate.is_absolute() {
            candidate
        } else {
            invocation_dir.join(candidate)
        }
    };
    if !path.is_file() {
        return Err(actionable_command_error(
            "nbuild",
            format!("nbuild source '{}' does not exist", path.display()),
        )
        .into());
    }
    let path = path.display().to_string();

    if !kernel_profile_requested && is_high_level_source_path(&path) {
        let inferred = load_ast_from_path(&path)?;
        if program_has_kernel_context(&inferred) {
            kernel_profile_requested = true;
            execution_profile = NccExecutionProfile::ReleaseFast;
        }
    }

    let artifact_root = match artifact_root {
        Some(root) if root.is_absolute() => root,
        Some(root) => invocation_dir.join(root),
        None => out_path
            .as_ref()
            .map(PathBuf::from)
            .filter(|out| out.is_absolute())
            .and_then(|out| out.parent().map(|parent| parent.join(".ncc")))
            .unwrap_or_else(|| invocation_dir.join(".ncc")),
    };
    if std::env::var_os("NCC_DISABLE_CONTENT_CACHE").is_some() {
        content_cache_enabled = false;
    }
    let content_cache_root = content_cache_enabled.then(|| {
        let explicit = content_cache_dir.as_ref().map(|path| {
            if path.is_absolute() {
                path.clone()
            } else {
                invocation_dir.join(path)
            }
        });
        crate::content_cache::content_cache_root(explicit.as_deref(), &artifact_root)
    });
    let kernel_target_spec = if kernel_profile_requested {
        Some(nbuild_kernel_target_spec(target_override.as_deref())?)
    } else {
        None
    };
    let explicit_out_path = out_path.is_some();
    let out_path = if let Some(out) = out_path {
        out
    } else if let Some(spec) = kernel_target_spec {
        default_nbuild_kernel_output_path(&path, &artifact_root, spec)
    } else if real_requested {
        default_nbuild_output_path(&path)
    } else {
        default_nbuild_standard_output_path(&path, &artifact_root, target_override.as_deref())
    };

    if kernel_profile_requested {
        if Path::new(&path)
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("nm"))
        {
            return Err(actionable_command_error(
                "nbuild",
                "kernel profile is the official normal .n lowering route; use a .n source here, or call `ncc machine` directly for handwritten .nm",
            )
            .into());
        }
        return run_nbuild_kernel_profile_command(
            &path,
            &out_path,
            &artifact_root,
            target_override.as_deref(),
            entry.as_deref(),
            kernel_allocator_available,
            json,
        );
    }

    if Path::new(&path)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("nm"))
    {
        return run_nbuild_machine_source_command(
            &path,
            &out_path,
            &artifact_root,
            target_override.as_deref(),
            json,
        );
    }

    let project_build_input = super::project::load_project_native_build_input(Path::new(&path))?;
    let project_linkage = project_build_input
        .as_ref()
        .map(|input| input.identity_payload());
    let ast = match project_build_input {
        Some(input) => input.program,
        None => load_ast_from_path(&path)?,
    };
    sema_check_with_warnings(&ast)?;
    let source_execution =
        nbuild_write_source_execution_artifact(&path, &artifact_root, &ast, false)?;
    let mut profile = parse_backend_profile_from_args(&profile_args)?;
    if let Some(target) = target_override.as_deref() {
        nbuild_apply_standard_target_override(&mut profile, target)?;
    }
    let basal_root =
        nbuild_materialize_basal_root(&path, &artifact_root, profile.target_spec.pointer_width)?;
    if matches!(profile.target_spec.isa, ncc_ir::BackendIsaIR::Wasm32) {
        return run_nbuild_wasm_os_lane_command(
            &path,
            out_path.as_str(),
            explicit_out_path,
            &artifact_root,
            ast,
            profile,
            execution_profile,
            &opt_level,
            json,
            &source_execution,
            &basal_root,
        );
    }
    validate_backend_artifact_profile(BackendArtifactKind::BuildExe, &profile)?;
    ncc_sema::validate_main_entries(&ast, ncc_sema::MainEntryRequirement::Required)
        .map_err(|err| -> Box<dyn std::error::Error> { err.into() })?;
    let resolved_entry = resolve_native_entry_symbol(&ast, entry.as_deref());
    let route_started = Instant::now();
    let effective_opt_level =
        nbuild_effective_optimization_level_label(&opt_level, execution_profile)?;
    if nbuild_standard_target_requires_mct_body(&profile) {
        return run_nbuild_standard_mct_machine_command(
            &path,
            out_path.as_str(),
            &artifact_root,
            ast,
            profile,
            execution_profile,
            &opt_level,
            linker.as_deref(),
            entry.as_deref(),
            json,
            project_linkage.clone(),
            &source_execution,
            &basal_root,
            content_cache_root.as_deref(),
        );
    }
    let optimize_started = Instant::now();
    let direct_native =
        emit_optimized_backend_object_ir_from_ast_with_profile(&ast, profile.clone()).and_then(
            |artifact| {
                let object_bytes = emit_backend_object_bytes_for_format(&artifact.object)?;
                Ok((artifact, object_bytes))
            },
        );
    let optimize_and_object_ms = nbuild_elapsed_ms(optimize_started);
    if direct_native
        .as_ref()
        .is_ok_and(|(artifact, _)| nbuild_object_requires_json_runtime_object(&artifact.object))
    {
        return run_nbuild_llvm_fallback(
            &path,
            out_path.as_str(),
            &artifact_root,
            &profile_args,
            execution_profile.label(),
            &opt_level,
            json,
            project_linkage.as_ref().map(|_| ast.clone()),
            project_linkage.clone(),
            &source_execution,
            &basal_root,
            content_cache_root.as_deref(),
            &profile,
            &resolved_entry,
        );
    }
    if direct_native
        .as_ref()
        .is_ok_and(|(artifact, _)| nbuild_object_requires_mct_memory_map_runtime(&artifact.object))
    {
        return run_nbuild_standard_mct_machine_command(
            &path,
            out_path.as_str(),
            &artifact_root,
            ast,
            profile,
            execution_profile,
            &opt_level,
            linker.as_deref(),
            entry.as_deref(),
            json,
            project_linkage.clone(),
            &source_execution,
            &basal_root,
            content_cache_root.as_deref(),
        );
    }
    let (optimized_artifact, object_bytes) = match direct_native {
        Ok(value) => value,
        Err(err) if nbuild_should_use_llvm_fallback(&err.to_string()) => {
            return run_nbuild_llvm_fallback(
                &path,
                out_path.as_str(),
                &artifact_root,
                &profile_args,
                execution_profile.label(),
                &opt_level,
                json,
                project_linkage.as_ref().map(|_| ast.clone()),
                project_linkage.clone(),
                &source_execution,
                &basal_root,
                content_cache_root.as_deref(),
                &profile,
                &resolved_entry,
            );
        }
        Err(err) => return Err(err),
    };
    let object = &optimized_artifact.object;
    if let Some(target) = target_override.as_deref() {
        let canonical_target = if target == "riscv32imac-unknown-elf" {
            "riscv32imac-unknown-elf"
        } else {
            target
        };
        if object.target_spec.target_triple != canonical_target {
            return Err(actionable_command_error(
                "nbuild",
                format!(
                    "normal .n target '{}' was requested, but the high-level backend materialized '{}'; refusing to label a different ISA/object as the requested target",
                    target, object.target_spec.target_triple
                ),
            )
            .into());
        }
    }
    let entry_returns_bool = native_entry_returns_bool(&ast, entry.as_deref());

    let stem = nbuild_source_stem(&path);
    let code_dir = artifact_root.join("code");
    let lowering_dir = artifact_root.join("lowering");
    let obj_dir = artifact_root.join("obj");
    let run_dir = artifact_root.join("run");
    let bin_dir = artifact_root.join("bin");
    for dir in [&code_dir, &lowering_dir, &obj_dir, &run_dir, &bin_dir] {
        fs::create_dir_all(dir)?;
    }
    super::compiler::compiler_native_remove_legacy_runtime_sources(&lowering_dir)?;
    let nm_path = lowering_dir.join(format!("{stem}.nm"));
    let optimizer_report_path = lowering_dir.join(format!("{stem}.optimizer.json"));
    let backend_object_ir_path = lowering_dir.join(format!("{stem}.backend-object.nbo"));
    let obj_path = obj_dir.join(format!("{stem}.obj"));
    let manifest_path = run_dir.join(format!("{stem}.native-build.json"));
    let optimizer_payload = internal_optimizer_payload(&optimized_artifact);
    let report_started = Instant::now();
    fs::write(
        &optimizer_report_path,
        serde_json::to_string_pretty(&optimizer_payload)?.as_bytes(),
    )?;
    require_non_empty_output_file(
        "nbuild optimized IR report",
        optimizer_report_path.to_string_lossy().as_ref(),
    )?;
    let backend_object_text = nbuild_backend_object_with_material_lineage(
        &stem,
        &path,
        &basal_root,
        &ncc_ir::render_object_text(object),
    );
    fs::write(&backend_object_ir_path, backend_object_text.as_bytes())?;
    require_non_empty_output_file(
        "nbuild machine backend object IR",
        backend_object_ir_path.to_string_lossy().as_ref(),
    )?;
    fs::write(&obj_path, &object_bytes)?;
    require_non_empty_output_file("nbuild object", obj_path.to_string_lossy().as_ref())?;
    let report_ms = nbuild_elapsed_ms(report_started);

    let bridge_started = Instant::now();
    let internal_os_link_format = nbuild_internal_os_link_format(object, linker.as_deref());
    let native_internal_link = internal_os_link_format.is_some();
    let native_pe_internal = internal_os_link_format == Some("coff");
    let nm_source = if native_pe_internal {
        nbuild_retained_machine_bridge_source(
            &path,
            &stem,
            object,
            &resolved_entry,
            entry_returns_bool,
            &obj_path,
            Path::new(&out_path),
        )
    } else {
        nbuild_hosted_machine_entry_source(
            &path,
            &stem,
            object,
            &resolved_entry,
            entry_returns_bool,
            &obj_path,
            Path::new(&out_path),
            Some(&backend_object_ir_path),
            None,
            native_internal_link,
        )
    };
    fs::write(&nm_path, nm_source.as_bytes())?;
    require_non_empty_output_file("nbuild machine source", nm_path.to_string_lossy().as_ref())?;
    super::machine::check_machine_source_path(nm_path.to_string_lossy().as_ref())?;

    ensure_output_parent_dir(&out_path)?;
    let entry_object_path: Option<PathBuf>;
    let mut native_link_report = None::<serde_json::Value>;
    let link_route = if let Some(format) = internal_os_link_format {
        let bridge_obj_path = obj_dir.join(format!("{stem}.entry.obj"));
        super::machine::emit_machine_source_object_to_path(
            nm_path.to_string_lossy().as_ref(),
            object.target_spec.target_triple.as_str(),
            bridge_obj_path.to_string_lossy().as_ref(),
        )?;
        require_non_empty_output_file(
            "nbuild machine entry object",
            bridge_obj_path.to_string_lossy().as_ref(),
        )?;
        let map_path = run_dir.join(format!(
            "{stem}.{}.map",
            nbuild_internal_os_link_map_suffix(format)
        ));
        let link_started = Instant::now();
        let inputs = vec![
            bridge_obj_path.to_string_lossy().to_string(),
            obj_path.to_string_lossy().to_string(),
        ];
        native_link_report = Some(
            super::machine::link_machine_object_paths_to_os_executable_with_missing_symbol_policy_silent(
                &inputs,
                object.target_spec.target_triple.as_str(),
                &out_path,
                Some(map_path.to_string_lossy().as_ref()),
                profile.destructive_policy.missing_symbol_policy,
                &object.unresolved_symbols(),
            )?,
        );
        entry_object_path = Some(bridge_obj_path);
        let route = nbuild_internal_os_link_route(format);
        (route, nbuild_elapsed_ms(link_started))
    } else {
        let bridge_obj_path = obj_dir.join(format!("{stem}.entry.obj"));
        super::machine::emit_machine_source_object_to_path(
            nm_path.to_string_lossy().as_ref(),
            object.target_spec.target_triple.as_str(),
            bridge_obj_path.to_string_lossy().as_ref(),
        )?;
        require_non_empty_output_file(
            "nbuild machine entry object",
            bridge_obj_path.to_string_lossy().as_ref(),
        )?;
        let link_started = Instant::now();
        crate::link_hosted_machine_objects_with_linker(
            object,
            &out_path,
            linker.as_deref(),
            &bridge_obj_path,
            &obj_path,
        )?;
        entry_object_path = Some(bridge_obj_path);
        ("host-linker-machine-entry", nbuild_elapsed_ms(link_started))
    };
    let target_init = nbuild_materialize_target_init(
        &stem,
        &artifact_root,
        object.target_spec.target_triple.as_str(),
        nbuild_machine_arch(&object.target_spec),
        nbuild_machine_abi(&object.target_spec),
        nbuild_machine_object(&object.target_spec),
        object.target_spec.pointer_width,
        &resolved_entry,
        &nm_path,
        entry_object_path
            .as_deref()
            .ok_or_else(|| actionable_command_error("nbuild", "machine entry object is missing"))?,
        "machine-entry-object",
    )?;
    let bridge_ms = nbuild_elapsed_ms(bridge_started).saturating_sub(link_route.1);
    let link_ms = link_route.1;
    let link_route = link_route.0;
    require_non_empty_output_file("nbuild executable", &out_path)?;
    release_native_executable_for_user(Path::new(&out_path))?;
    let launcher_started = Instant::now();
    let launcher_artifacts = write_native_launcher_artifacts(
        Path::new(&path),
        &stem,
        Path::new(&out_path),
        &code_dir,
        &bin_dir,
    )?;
    let launcher_ms = nbuild_elapsed_ms(launcher_started);

    let mut artifacts = vec![
        nbuild_artifact_record(&basal_root.source_path, "basal_root_source")?,
        nbuild_artifact_record(&basal_root.binary_path, "basal_root_binary")?,
        nbuild_artifact_record(&nm_path, "machine_source")?,
        nbuild_artifact_record(&optimizer_report_path, "optimized_ir_report")?,
        nbuild_artifact_record(&backend_object_ir_path, "machine_backend_object_ir")?,
        nbuild_artifact_record(&launcher_artifacts.launcher_n, "launcher_n")?,
        nbuild_artifact_record(&obj_path, "object")?,
        nbuild_artifact_record(Path::new(&out_path), "executable")?,
        nbuild_artifact_record(&launcher_artifacts.powershell, "powershell_launcher")?,
        nbuild_artifact_record(&launcher_artifacts.batch, "batch_launcher")?,
    ];
    if let Some(path) = &entry_object_path {
        artifacts.push(nbuild_artifact_record(path, "machine_entry_object")?);
    }
    let pipeline = if native_internal_link {
        vec![
            "parse .n",
            "sema",
            "lower optimized IR",
            "run SSA/loop/alias/register/instruction-selection optimizer",
            "emit machine BackendObjectIR with ABI-classified calls",
            "emit native object bytes",
            "retain .nm machine bridge",
            "check retained .nm through machine",
            "emit .nm bridge object",
            "link native OS executable internally",
        ]
    } else {
        vec![
            "parse .n",
            "sema",
            "lower optimized IR",
            "run SSA/loop/alias/register/instruction-selection optimizer",
            "emit machine BackendObjectIR with ABI-classified calls",
            "emit native object bytes",
            "retain .nm machine bridge",
            "check retained .nm through machine",
            "link hosted executable",
        ]
    };
    let release_fast_auto = nbuild_release_fast_auto_payload(
        "native-fast-backend",
        execution_profile,
        &opt_level,
        effective_opt_level,
        serde_json::json!({
            "optimize_and_object": optimize_and_object_ms,
            "retain_optimizer_and_object_ir": report_ms,
            "machine_bridge": bridge_ms,
            "link": link_ms,
            "launcher_generation": launcher_ms,
            "total_backend_route": nbuild_elapsed_ms(route_started),
        }),
        serde_json::json!({
            "optimizer_report_bytes": fs::metadata(&optimizer_report_path)?.len(),
            "backend_object_ir_bytes": fs::metadata(&backend_object_ir_path)?.len(),
            "program_object_bytes": object_bytes.len(),
            "program_object_file_bytes": fs::metadata(&obj_path)?.len(),
            "machine_entry_object_bytes": entry_object_path
                .as_ref()
                .and_then(|path| fs::metadata(path).ok().map(|meta| meta.len())),
            "executable_bytes": fs::metadata(&out_path)?.len(),
            "sections": object.sections.len(),
            "symbols": object.symbols.len(),
            "relocations": object.relocations.len(),
        }),
        &optimizer_payload,
    );
    let mut payload = serde_json::json!({
        "schema": "nc.compiler.standard_native_build.v1",
        "status": "native_build_passed",
        "source": path.clone(),
        "project_linkage": project_linkage,
        "source_sha256": sha256_hex_file(Path::new(&path))?,
        "artifact_root": artifact_root.display().to_string(),
        "profile": execution_profile.label(),
        "optimization": {
            "requested": nbuild_requested_optimization_label(&opt_level),
            "effective": effective_opt_level,
            "route": "native-fast-backend"
        },
        "target": {
            "target": render_backend_target_label(&object.target),
            "triple": object.target_spec.target_triple.clone(),
            "object_format": render_backend_object_format_label(&object.target_spec.object_format),
            "arch": nbuild_machine_arch(&object.target_spec),
            "abi": nbuild_machine_abi(&object.target_spec),
            "pointer_width": object.target_spec.pointer_width,
        },
        "entry": resolved_entry.clone(),
        "paths": {
            "basal_root_source": basal_root.source_path.display().to_string(),
            "basal_root_binary": basal_root.binary_path.display().to_string(),
            "machine_source": nm_path.display().to_string(),
            "optimized_ir_report": optimizer_report_path.display().to_string(),
            "machine_backend_object_ir": backend_object_ir_path.display().to_string(),
            "launcher_n": launcher_artifacts.launcher_n.display().to_string(),
            "machine_entry_object": entry_object_path.as_ref().map(|path| path.display().to_string()),
            "object": obj_path.display().to_string(),
            "executable": out_path.clone(),
            "powershell_launcher": launcher_artifacts.powershell.display().to_string(),
            "batch_launcher": launcher_artifacts.batch.display().to_string(),
            "manifest": manifest_path.display().to_string(),
        },
        "lowering_lineage": {
            "schema": "nc.lowering.source-root-machine-nbo.v2",
            "status": "materialized",
            "basal": nbuild_basal_root_lineage_payload(&basal_root),
            "machine": {
                "path": nm_path.display().to_string(),
                "sha256": sha256_hex_file(&nm_path)?,
                "container_matches_file_stem": true
            },
            "backend_object": {
                "path": backend_object_ir_path.display().to_string(),
                "sha256": sha256_hex_file(&backend_object_ir_path)?,
                "format": "nbo",
                "source_machine_path_embedded": true,
                "source_machine_sha256_embedded": false,
                "integrity_link": ".nm embeds the final .nbo SHA-256; the manifest records both hashes without a circular digest"
            },
            "order": ["source.n", "root.nb", "root.nbb", "machine.nm", "backend-object.nbo", "native-object", "executable"]
        },
        "pipeline": pipeline,
        "artifacts": artifacts,
        "speed_contract": {
            "normal_build_surface": true,
            "release_fast_automatic": execution_profile == NccExecutionProfile::ReleaseFast,
            "effective_optimization": effective_opt_level,
            "special_optimize_command_required": false,
            "measurable_release_fast": true,
            "interpreter_fallback": false,
            "native_machine_execution": true,
            "internal_os_linker": native_internal_link,
            "c_like_artifact_set": [".nb", ".nbb", ".nm", ".obj", ".exe"],
            "complete_artifact_set": [".nb", ".nbb", ".optimizer.json", ".backend-object.nbo", ".nm", ".obj", ".entry.obj", ".exe"]
        },
        "optimizer": optimizer_payload,
        "release_fast_auto": release_fast_auto,
        "machine_backend": {
            "object_ir_retained": true,
            "object_ir_path": backend_object_ir_path.display().to_string(),
            "program_object_bytes": object_bytes.len(),
            "sections": object.sections.len(),
            "symbols": object.symbols.len(),
            "relocations": object.relocations.len(),
            "abi": {
                "model": nbuild_machine_abi(&object.target_spec),
                "pointer_width": object.target_spec.pointer_width,
                "stack_align_bytes": object.target_spec.data_layout.stack_align_bytes,
                "aggregate_align_bytes": object.target_spec.data_layout.aggregate_align_bytes,
                "calling_convention_materialized_in_object_ir": true,
                "aggregate_return_contract_materialized": true,
                "direct_machine_call_lowering": true
            },
            "runtime_import_materialization": nbuild_runtime_import_materialization_payload(object, internal_os_link_format)
        },
        "link": {
            "route": link_route,
            "native_pe64_internal": native_pe_internal,
            "internal_os_linker": native_internal_link,
            "internal_os_link_format": internal_os_link_format,
            "phase12_report": native_link_report,
        },
        "truth_boundary": {
            "object_is_real_backend_output": true,
            "object_is_optimized_backend_output": true,
            "optimized_ir_report_retained": true,
            "machine_backend_object_ir_retained": true,
            "executable_is_configured_linker_output": !native_internal_link,
            "executable_is_native_pe_direct": native_pe_internal,
            "executable_is_internal_os_loader_direct": native_internal_link,
            "runtime_imports_are_machine_linker_visible": true,
            "abi_contract_is_backend_object_visible": true,
            "nm_is_retained_machine_bridge_not_full_instruction_for_instruction_high_level_lowering": true,
            "launcher_n_generated": true,
            "host_shell_launchers_generated": [".ps1", ".bat"],
            "executable_released_for_external_run": true,
            "missing_tools_are_not_faked": true,
        }
    });
    nbuild_attach_target_init_payload(&mut payload, &target_init)?;
    nbuild_attach_source_execution_payload(&mut payload, &source_execution)?;
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&payload)?.as_bytes(),
    )?;

    if !crate::ncc_child_output_quiet() {
        if json {
            println!("{}", serde_json::to_string_pretty(&payload)?);
        } else {
            println!(
                "OK: nbuild exe={} obj={} nm={} manifest={} profile={} route=native-fast-path",
                out_path,
                obj_path.display(),
                nm_path.display(),
                manifest_path.display(),
                execution_profile.label()
            );
        }
    }
    Ok(())
}

pub(crate) fn run_ncc_test_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!("{}", command_help("test").expect("test help must exist"));
        return Ok(());
    }

    let mut show_return = false;
    let mut main_profile: Option<String> = None;
    let mut strict_source = false;
    let mut path: Option<String> = None;
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--show-return" => {
                show_return = true;
                idx += 1;
            }
            "--strict-source" => {
                strict_source = true;
                idx += 1;
            }
            "--main-profile" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("test", "test syntax: --main-profile requires a value")
                })?;
                main_profile = Some(value.clone());
                idx += 2;
            }
            "-h" | "--help" | "help" => {
                println!("{}", command_help("test").expect("test help must exist"));
                return Ok(());
            }
            other if other.starts_with("--") => {
                return Err(actionable_command_error(
                    "test",
                    format!("test: unknown option '{}'", other),
                )
                .into());
            }
            _ => {
                if path.is_some() {
                    return Err(actionable_command_error(
                        "test",
                        "test command expects exactly one source file path argument",
                    )
                    .into());
                }
                path = Some(args[idx].clone());
                idx += 1;
            }
        }
    }

    let path =
        path.ok_or_else(|| actionable_command_error("test", "test syntax: test <file.n>"))?;
    let src = fs::read_to_string(&path)?;
    validate_import_surface_source_gates(&path, &src)?;
    let tokens = ncc_lexer::lex(&src)?;
    let ast = ncc_parser::parse(tokens)?;
    let source_warnings = validate_source_conventions(&ast, &path, strict_source)?;
    let sema = ncc_sema::analyze(&ast).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    verify_type_contract_execution_path(&ast)?;
    ncc_sema::validate_main_entries(&ast, ncc_sema::MainEntryRequirement::Required)
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    for w in source_warnings {
        eprintln!("warning: {w}");
    }
    for w in sema.warnings {
        eprintln!("warning: {w}");
    }
    let ret = run_main_with_profile(&ast, main_profile.as_deref())?;
    if show_return {
        println!(
            "OK: test passed for {} [return: {}]",
            path,
            render_value(&ret)
        );
    } else {
        println!("OK: test passed for {}", path);
    }
    Ok(())
}

pub(crate) fn run_ncc_doc_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == 1 && is_help_token(&args[0]) {
        println!("{}", command_help("doc").expect("doc help must exist"));
        return Ok(());
    }

    let mut path: Option<String> = None;
    let mut out_path: Option<String> = None;
    let mut json_mode = false;
    let mut idx = 0usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--json" => {
                json_mode = true;
                idx += 1;
            }
            "--out" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    actionable_command_error("doc", "doc syntax: --out requires a file path")
                })?;
                out_path = Some(value.clone());
                idx += 2;
            }
            "-h" | "--help" | "help" => {
                println!("{}", command_help("doc").expect("doc help must exist"));
                return Ok(());
            }
            other if other.starts_with("--") => {
                return Err(actionable_command_error(
                    "doc",
                    format!("doc: unknown option '{}'", other),
                )
                .into());
            }
            _ => {
                if path.is_some() {
                    return Err(actionable_command_error(
                        "doc",
                        "doc command expects exactly one source file path argument",
                    )
                    .into());
                }
                path = Some(args[idx].clone());
                idx += 1;
            }
        }
    }

    let path = path.ok_or_else(|| actionable_command_error("doc", "doc syntax: doc <file.n>"))?;
    let src = fs::read_to_string(&path)?;
    validate_import_surface_source_gates(&path, &src)?;
    let tokens = ncc_lexer::lex(&src)?;
    let ast = ncc_parser::parse(tokens)?;
    sema_check_with_warnings(&ast)?;
    let payload = serde_json::json!({
        "schema": "nc.doc.source_report.v1",
        "status": "ok",
        "file": path.clone(),
        "mode": if json_mode { "json" } else { "json_default" },
        "outline": commands::nide::build_nide_outline_report(&path, &ast),
        "symbols": commands::nide::build_nide_symbols_report(&path, &ast),
    });
    let rendered = serde_json::to_string_pretty(&payload)?;
    if let Some(out) = out_path {
        fs::write(&out, rendered.as_bytes())?;
        println!("OK: doc wrote {}", out);
    } else {
        println!("{}", rendered);
    }
    Ok(())
}

#[cfg(test)]
mod nbuild_runtime_import_tests {
    use super::*;

    #[test]
    fn mct_main_rename_preserves_void_and_sysv_process_signatures() {
        for (source, expected) in [
            (
                "define i32 @main() { ret i32 0 }",
                "define i32 @unit_entry() { ret i32 0 }",
            ),
            (
                "define i32 @main(i32 %argc, ptr %argv, ptr %envp) { ret i32 0 }",
                "define i32 @unit_entry(i32 %argc, ptr %argv, ptr %envp) { ret i32 0 }",
            ),
        ] {
            assert_eq!(
                nbuild_rename_mct_main_wrapper(source, "unit_entry").expect("rename main"),
                expected
            );
        }
    }

    #[test]
    fn machine_strings_strip_windows_verbatim_path_prefixes() {
        assert_eq!(
            nbuild_machine_string(r"\\?\C:\work\kernel.n"),
            "C:/work/kernel.n"
        );
        assert!(!nbuild_machine_string(r"\\?\C:\work\kernel.n").starts_with("//"));
    }

    #[test]
    fn nbuild_target_init_matrix_retains_target_specific_startup_bytes() {
        let unique = format!(
            "ncc-target-init-matrix-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        );
        let artifact_root = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&artifact_root).expect("target init artifact root");
        let machine = artifact_root.join("matrix.nm");
        let startup = artifact_root.join("startup.bin");
        std::fs::write(&machine, b"matrix :: machine { entry: main; }").expect("Machine source");
        std::fs::write(&startup, [0x4e, 0x43, 0x49, 0x54, 0x01]).expect("startup bytes");

        for (target, arch, abi, object, pointer_width, payload_kind) in [
            (
                "x86_64-unknown-linux-gnu",
                "x86_64",
                "sysv64",
                "elf64",
                64,
                "elf-entry-object",
            ),
            (
                "x86_64-pc-windows-msvc",
                "x86_64",
                "win64",
                "coff",
                64,
                "coff-entry-object",
            ),
            (
                "x86_64-apple-darwin",
                "x86_64",
                "sysv64",
                "macho64",
                64,
                "macho-entry-object",
            ),
            (
                "i386-unknown-linux-gnu",
                "i386",
                "cdecl",
                "elf32",
                32,
                "elf-entry-object",
            ),
            (
                "aarch64-pc-windows-msvc",
                "aarch64",
                "aapcs64",
                "coff",
                64,
                "coff-entry-object",
            ),
            (
                "aarch64-apple-darwin",
                "aarch64",
                "aapcs64",
                "macho64",
                64,
                "macho-entry-object",
            ),
            (
                "aarch64-unknown-linux-gnu",
                "aarch64",
                "aapcs64",
                "elf64",
                64,
                "elf-entry-object",
            ),
            (
                "armv7-unknown-linux-gnueabihf",
                "armv7",
                "aapcs32",
                "elf32",
                32,
                "elf-entry-object",
            ),
            (
                "riscv32imac-unknown-elf",
                "riscv32",
                "ilp32",
                "elf32",
                32,
                "elf-entry-object",
            ),
            (
                "riscv64gc-unknown-linux-gnu",
                "riscv64",
                "lp64",
                "elf64",
                64,
                "elf-entry-object",
            ),
            ("wasm32-wasi", "wasm32", "wasm", "wasm", 32, "wasm-module"),
        ] {
            let artifact = nbuild_materialize_target_init(
                "matrix",
                &artifact_root,
                target,
                arch,
                abi,
                object,
                pointer_width,
                "main",
                &machine,
                &startup,
                payload_kind,
            )
            .expect("target-specific .init");
            let retained = std::fs::read(&artifact.path).expect("retained .init");
            let header_end = retained
                .windows(2)
                .position(|window| window == b"\n\n")
                .map(|offset| offset + 2)
                .expect(".init header terminator");
            let header = std::str::from_utf8(&retained[..header_end]).expect("UTF-8 header");
            assert!(header.contains(&format!("target={target}\n")), "{header}");
            assert!(header.contains(&format!("arch={arch}\n")), "{header}");
            assert!(header.contains(&format!("abi={abi}\n")), "{header}");
            assert!(header.contains(&format!("object={object}\n")), "{header}");
            assert!(
                header.contains(&format!(
                    "executable-kind={}\n",
                    nbuild_target_executable_kind(target)
                )),
                "{header}"
            );
            assert!(
                header.contains(&format!("loader={}\n", nbuild_target_loader_kind(target))),
                "{header}"
            );
            assert!(
                header.contains(&format!("pointer-width={pointer_width}\n")),
                "{header}"
            );
            assert_eq!(&retained[header_end..], [0x4e, 0x43, 0x49, 0x54, 0x01]);
            assert_eq!(artifact.report["target"], target);
            assert_eq!(artifact.report["arch"], arch);
        }

        std::fs::remove_dir_all(&artifact_root).expect("remove target init artifact root");
    }

    #[test]
    fn nbuild_standard_output_name_matches_target_executable_format() {
        let root = Path::new(".ncc");
        let windows =
            default_nbuild_standard_output_path("program.n", root, Some("x86_64-pc-windows-msvc"));
        let linux = default_nbuild_standard_output_path(
            "program.n",
            root,
            Some("aarch64-unknown-linux-gnu"),
        );
        let darwin =
            default_nbuild_standard_output_path("program.n", root, Some("aarch64-apple-darwin"));
        let wasm = default_nbuild_standard_output_path("program.n", root, Some("wasm32-wasi"));

        assert_eq!(
            Path::new(&windows)
                .extension()
                .and_then(|value| value.to_str()),
            Some("exe")
        );
        assert!(Path::new(&linux).extension().is_none(), "{linux}");
        assert!(Path::new(&darwin).extension().is_none(), "{darwin}");
        assert_eq!(
            Path::new(&wasm)
                .extension()
                .and_then(|value| value.to_str()),
            Some("wasm")
        );
    }

    fn parse_layer_import(source: &str) -> ncc_ast::Program {
        ncc_parser::parse(ncc_lexer::lex(source).expect("layer import lex"))
            .expect("layer import parse")
    }

    #[test]
    fn kernel_library_audit_allows_core_and_rejects_hosted_stoud() {
        let core = parse_layer_import("use::core::sync::<atomic>;");
        nbuild_validate_kernel_library_layers(&core, false).expect("core must be freestanding");

        let stoud = parse_layer_import("use::stoud::fs::<read>;");
        let err = nbuild_validate_kernel_library_layers(&stoud, true)
            .expect_err("stoud must not enter kernel image")
            .to_string();
        assert!(err.contains("E_KERNEL_HOSTED_DEPENDENCY_001"), "{err}");
        assert!(err.contains("libc"), "{err}");
    }

    #[test]
    fn kernel_alloc_layer_requires_an_explicit_allocator_contract() {
        let alloc = parse_layer_import("use::alloc::heap::<alloc>;");
        let err = nbuild_validate_kernel_library_layers(&alloc, false)
            .expect_err("alloc without allocator must fail")
            .to_string();
        assert!(err.contains("E_KERNEL_ALLOC_UNAVAILABLE_001"), "{err}");
        nbuild_validate_kernel_library_layers(&alloc, true)
            .expect("--kernel-alloc enables the freestanding allocator layer");
    }

    #[test]
    fn nbuild_materializes_and_attaches_source_execution_contract() {
        let source = r#"
package artifact_mode { profile: release_fast; }

artifactMode[@runtime] {
    fn:
        run() -> bool {
            public:
                @items >> list<int> : [1, 2];
                listAdd(@items, 3);
                return true;
        }

    fa:
        return true;
}
"#;
        let program = parse_layer_import(source);
        let unique = format!(
            "ncc-source-execution-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        );
        let artifact_root = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&artifact_root).expect("temporary artifact root");

        let artifact = nbuild_write_source_execution_artifact(
            "fixtures/source mode.n",
            &artifact_root,
            &program,
            false,
        )
        .expect("source execution artifact");
        assert_eq!(
            artifact.path.file_name().and_then(|name| name.to_str()),
            Some("source_mode.source-execution.json")
        );
        assert_eq!(artifact.payload["status"], "materialized");
        assert_eq!(artifact.payload["contract"]["mode"], "manual_raw");
        assert_eq!(
            artifact.payload["contract"]["hidden_work"]["hidden_allocation"],
            false
        );
        assert_eq!(
            artifact.payload["manual_equivalence"]["convertible_to_manual_ir"],
            true
        );
        assert_eq!(
            artifact.payload["manual_authority"]["compiler_role"],
            "mechanical_translation_and_optimization_only"
        );
        assert_eq!(
            artifact.payload["manual_authority"]["hidden_runtime_work"],
            false
        );
        assert_eq!(
            artifact.payload["expansion"]["compiler_owned_runtime_effects"],
            0
        );
        assert!(artifact.payload["expansion"]["total_occurrences"]
            .as_u64()
            .is_some_and(|count| count >= 2));

        let mut manifest = serde_json::json!({
            "paths": {},
            "artifacts": []
        });
        nbuild_attach_source_execution_payload(&mut manifest, &artifact)
            .expect("attach source execution contract");
        assert_eq!(manifest["source_execution"], artifact.payload);
        assert_eq!(
            manifest["paths"]["source_execution"],
            artifact.path.display().to_string()
        );
        assert!(manifest["artifacts"].as_array().is_some_and(|artifacts| {
            artifacts.iter().any(|record| {
                record["kind"] == "source_execution_contract"
                    && record["path"] == artifact.path.display().to_string()
            })
        }));

        std::fs::remove_dir_all(&artifact_root).expect("remove temporary artifact root");
    }

    #[test]
    fn nbuild_detects_json_runtime_object_before_direct_pe_linking() {
        let source = r#"
use nC.runtime.json;
jsonBuildRoute[@runtime] {
    fn:
        run() -> int {
            public:
                return jsonParse("{}");
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#;
        let ast = ncc_parser::parse(ncc_lexer::lex(source).expect("JSON route lex"))
            .expect("JSON route parse");
        let profile =
            parse_backend_profile_from_args(&["--cc".to_string(), "nC.win64".to_string()])
                .expect("Win64 backend profile");
        let artifact = emit_optimized_backend_object_ir_from_ast_with_profile(&ast, profile)
            .expect("direct backend artifact");

        assert!(artifact
            .object
            .unresolved_symbols()
            .iter()
            .any(|symbol| symbol.rsplit('.').next() == Some("jsonParse")));
        assert!(nbuild_object_requires_json_runtime_object(&artifact.object));
    }

    #[test]
    fn kernel_x86_stage0_discovers_and_uses_bios_chs_geometry() {
        let stage0 = nbuild_kernel_x86_stage0_template_bytes();
        assert_eq!(stage0.len(), NBUILD_KERNEL_SECTOR_SIZE);
        assert_eq!(&stage0[510..], &[0x55, 0xaa]);
        assert_eq!(
            nbuild_kernel_find_unique_bytes(&stage0, &[0xfe, 0xca], "stage0 payload sector-count")
                .expect("unique payload marker"),
            0x9d
        );
        assert!(
            stage0
                .windows(8)
                .any(|bytes| bytes == [0xb4, 0x08, 0x8a, 0x16, 0x9a, 0x7c, 0xcd, 0x13]),
            "stage0 must query BIOS drive geometry with INT 13h/AH=08"
        );
        assert!(
            stage0
                .windows(4)
                .any(|bytes| bytes == [0x3a, 0x0e, 0x9b, 0x7c]),
            "sector rollover must compare against discovered sectors-per-track"
        );
        assert!(
            stage0
                .windows(4)
                .any(|bytes| bytes == [0x3a, 0x36, 0x9c, 0x7c]),
            "head rollover must compare against discovered head count"
        );
    }

    #[test]
    fn nbuild_runtime_import_symbol_admission_covers_official_runtime_catalog() {
        for family in ncc_ast::runtime_api_family_specs() {
            for function in &family.functions {
                assert!(
                    nbuild_is_runtime_import_symbol(&function.name),
                    "runtime function leaf '{}' from {} must be admitted",
                    function.name,
                    family.import_path
                );
                assert!(
                    nbuild_is_runtime_import_symbol(&format!(
                        "{}.{}",
                        family.import_path, function.name
                    )),
                    "runtime function full symbol '{}.{}' must be admitted",
                    family.import_path,
                    function.name
                );
            }
        }
    }

    #[test]
    fn nbuild_runtime_import_symbol_admission_covers_backend_import_manifest() {
        let manifest = ncc_ast::use_backend_lowering_manifest_all();
        for entry in manifest.entries {
            for symbol in &entry.runtime_symbols {
                assert!(
                    nbuild_is_runtime_import_symbol(symbol),
                    "backend import '{}' runtime symbol '{}' must be admitted",
                    entry.surface_import,
                    symbol
                );
            }
        }
    }

    #[test]
    fn nbuild_runtime_import_symbol_admission_covers_host_c_and_os_symbols() {
        for symbol in [
            "printf",
            "write",
            "strlen",
            "memcpy",
            "ShellExecuteA",
            "keybd_event",
            "Sleep",
            "usleep",
            "GetStdHandle",
            "ReadFile",
            "WriteFile",
            "CreateFileA",
            "CloseHandle",
            "ExitProcess",
        ] {
            assert!(
                nbuild_is_runtime_import_symbol(symbol),
                "host C/OS runtime bridge symbol '{symbol}' must be admitted"
            );
        }
    }

    #[test]
    fn nbuild_runtime_import_symbol_admission_keeps_catalog_only_activation_out() {
        assert!(
            !nbuild_is_runtime_import_symbol("__nc_import_activate_stoud_io_entropy"),
            "catalog-only activation symbols must not be reported as native runtime calls"
        );
    }

    #[test]
    fn nbuild_runtime_native_call_admission_payload_records_catalog_authority() {
        let payload = nbuild_runtime_native_call_admission_payload();
        assert_eq!(payload["status"], "closed_catalog_derived");
        assert_eq!(payload["manual_runtime_allowlist"], false);
        assert_eq!(payload["catalog_only_activation_symbols_excluded"], true);
        assert!(
            payload["official_runtime_function_count"]
                .as_u64()
                .is_some_and(|count| count > 0),
            "{payload}"
        );
        assert!(
            payload["admitted_symbol_alias_count"]
                .as_u64()
                .is_some_and(|count| count > 0),
            "{payload}"
        );
    }
}
