use crate::ast::{Declaration, Stylesheet};
use crate::error::Span;
use crate::theme::{ShorthandDef, Theme};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub fn resolve(mut stylesheet: Stylesheet, theme: &Theme) -> Result<Stylesheet, String> {
    let variables = stylesheet.variables.clone();
    for rule in &mut stylesheet.rules {
        let mut new_decls = Vec::new();
        let mut display_defined = rule.declarations.iter().any(|d| d.property == "display");
        let mut radius_entries: Vec<(String, String)> = Vec::new();

        let mut grid_block_entries: Vec<GridBlockEntry> = Vec::new();
        let mut flex_props_used = false;
        for mut decl in rule.declarations.drain(..) {
            let decl_span = decl.span;
            if decl.property == "grid" {
                let commands = parse_grid_commands(&decl.value)?;
                let (mut grid_decls, display_added) = build_grid_declarations(
                    &commands,
                    theme,
                    display_defined,
                    decl_span,
                    &variables,
                )?;
                display_defined |= display_added;
                new_decls.append(&mut grid_decls);
                continue;
            }

            if let Some(entry) = extract_grid_block_entry(&decl.property, &decl.value) {
                grid_block_entries.push(entry);
                continue;
            }

            if let Some(key) = extract_radius_key(&decl.property) {
                radius_entries.push((key.to_string(), decl.value));
                continue;
            }

            // Handle shorthands first
            if let Some(expanded) = expand_shorthand(&decl.property, &decl.value, theme)? {
                let mut resolved_expanded = Vec::new();
                for (prop, val, append) in expanded {
                    let resolved_value = resolve_value(&val, &prop, theme, decl_span, &variables)?;
                    resolved_expanded.push((prop, resolved_value, append));
                }
                merge_declarations(&mut new_decls, resolved_expanded);
                continue;
            }

            if let Some(mapped) = map_border_subproperty(&decl.property) {
                let resolved = resolve_value(&decl.value, mapped, theme, decl_span, &variables)?;
                new_decls.push(Declaration {
                    property: mapped.to_string(),
                    value: resolved,
                    span: decl_span,
                });
                continue;
            }

            if let Some(mapped) = map_flex_subproperty(&decl.property) {
                let resolved = resolve_value(&decl.value, mapped, theme, decl_span, &variables)?;
                new_decls.push(Declaration {
                    property: mapped.to_string(),
                    value: resolved,
                    span: decl_span,
                });
                flex_props_used = true;
                continue;
            }

            decl.value = resolve_value(&decl.value, &decl.property, theme, decl_span, &variables)?;
            new_decls.push(decl);
        }

        if !radius_entries.is_empty() {
            let mut expanded = expand_radius_entries(&radius_entries, theme, &variables)?;
            new_decls.append(&mut expanded);
        }

        if !grid_block_entries.is_empty() {
            let commands = build_grid_commands_from_block(&grid_block_entries)?;
            let (mut grid_decls, display_added) = build_grid_declarations(
                &commands,
                theme,
                display_defined,
                Span::dummy(),
                &variables,
            )?;
            display_defined |= display_added;
            new_decls.append(&mut grid_decls);
        }

        if flex_props_used && !display_defined {
            new_decls.insert(
                0,
                Declaration {
                    property: "display".to_string(),
                    value: "flex".to_string(),
                    span: Span::dummy(),
                },
            );
        }

        rule.declarations = new_decls;

        // Resolve media blocks
        for media in &mut rule.media {
            media.query = resolve_media_query(&media.query, theme)?;
            for decl in &mut media.declarations {
                decl.value = resolve_value(
                    &decl.value,
                    &decl.property,
                    theme,
                    Span::dummy(),
                    &variables,
                )?;
            }
        }
    }
    Ok(stylesheet)
}

fn resolve_media_query(query: &str, theme: &Theme) -> Result<String, String> {
    let trimmed = query.trim();
    if let Some(inner) = trimmed
        .strip_prefix("screen(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let token = inner.trim().trim_start_matches('@');
        let width = resolve_from_collection("screens", token, theme)?;
        let width_str = value_to_css(&width, "screens", token)?;
        return Ok(format!("(min-width: {})", width_str));
    }

    if trimmed == "dark" {
        return Ok("(prefers-color-scheme: dark)".to_string());
    }

    if trimmed == "light" {
        return Ok("(prefers-color-scheme: light)".to_string());
    }

    // Passthrough for raw queries
    Ok(trimmed.to_string())
}

fn resolve_value(
    value: &str,
    property: &str,
    theme: &Theme,
    span: Span,
    variables: &HashMap<String, String>,
) -> Result<String, String> {
    let interpolated = resolve_variables(value, variables, span)?;
    let resolved = resolve_interpolations(&interpolated, property, theme, span)?;
    Ok(apply_color_functions(&resolved))
}

fn resolve_variables(
    value: &str,
    vars: &HashMap<String, String>,
    span: Span,
) -> Result<String, String> {
    resolve_variables_inner(value, vars, span, 0)
}

fn resolve_variables_inner(
    value: &str,
    vars: &HashMap<String, String>,
    span: Span,
    depth: usize,
) -> Result<String, String> {
    if depth > 16 {
        return Err(span_error(
            span,
            "RCSS variable error: maximum recursion depth exceeded",
        ));
    }

    let mut out = String::new();
    let mut idx = 0;

    while idx < value.len() {
        let ch = value[idx..].chars().next().unwrap();
        let ch_len = ch.len_utf8();

        if ch == '$' {
            let prev = prev_char(value, idx);
            if !is_variable_boundary(prev) {
                out.push(ch);
                idx += ch_len;
                continue;
            }

            let start = idx + ch_len;
            if let Some((name, consumed)) = consume_variable_name(value, start) {
                if consumed == 0 {
                    out.push(ch);
                    idx += ch_len;
                    continue;
                }

                let replacement = vars.get(&name).ok_or_else(|| {
                    span_error(
                        span.with_offset(idx),
                        format!("RCSS variable error: unknown variable '${}'", name),
                    )
                })?;
                let resolved_replacement =
                    resolve_variables_inner(replacement, vars, span.with_offset(idx), depth + 1)?;
                out.push_str(&resolved_replacement);
                idx = start + consumed;
                continue;
            }
        }

        out.push(ch);
        idx += ch_len;
    }

    Ok(out)
}

fn consume_variable_name(value: &str, start: usize) -> Option<(String, usize)> {
    if start >= value.len() {
        return None;
    }

    let mut cursor = start;
    let first = value[cursor..].chars().next()?;
    if !is_variable_start(first) {
        return None;
    }

    let mut name = String::new();
    loop {
        if cursor >= value.len() {
            break;
        }
        let ch = value[cursor..].chars().next().unwrap();
        if name.is_empty() {
            if !is_variable_start(ch) {
                break;
            }
        } else if !is_variable_char(ch) {
            break;
        }
        name.push(ch);
        cursor += ch.len_utf8();
    }

    Some((name, cursor - start))
}

fn is_variable_boundary(prev: Option<char>) -> bool {
    match prev {
        None => true,
        Some(ch) => !is_variable_char(ch),
    }
}

fn is_variable_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_variable_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

fn resolve_interpolations(
    value: &str,
    property: &str,
    theme: &Theme,
    span: Span,
) -> Result<String, String> {
    let mut out = String::new();
    let mut idx = 0;
    let mut depth_double = false;
    let mut depth_single = false;

    while idx < value.len() {
        let ch = value[idx..].chars().next().unwrap();
        let ch_len = ch.len_utf8();

        if ch == '"' && !depth_single {
            depth_double = !depth_double;
            out.push(ch);
            idx += ch_len;
            continue;
        }

        if ch == '\'' && !depth_double {
            depth_single = !depth_single;
            out.push(ch);
            idx += ch_len;
            continue;
        }

        if ch == '@' && !depth_double && !depth_single {
            if idx + ch_len <= value.len() {
                if let Some(next_ch) = value[idx + ch_len..].chars().next() {
                    if next_ch == '(' {
                        let start = idx + ch_len;
                        match consume_parenthesized(value, start) {
                            Ok((inner, consumed)) => {
                                let inner_span = span.with_offset(start + 1);
                                let resolved_inner =
                                    resolve_interpolations(&inner, property, theme, inner_span)?;
                                out.push_str(&format!("url(\"{}\")", resolved_inner));
                                idx = start + consumed;
                                continue;
                            }
                            Err(message) => {
                                return Err(span_error(
                                    span.with_offset(idx),
                                    format!("RCSS URL error: {}", message),
                                ));
                            }
                        }
                    }
                }
            }

            let prev = prev_char(value, idx);
            if !is_token_boundary(prev) {
                return Err(span_error(
                    span.with_offset(idx),
                    "RCSS token error: tokens must be separated from surrounding characters",
                ));
            }

            let mut token = String::new();
            let mut cursor = idx + ch_len;
            while cursor < value.len() {
                let next = value[cursor..].chars().next().unwrap();
                if next == '.' {
                    let after = value[cursor + next.len_utf8()..].chars().next();
                    if after.map(|c| c.is_ascii_digit()).unwrap_or(false) {
                        token.push(next);
                        cursor += next.len_utf8();
                        continue;
                    }
                    break;
                }
                if is_token_char(next) {
                    token.push(next);
                    cursor += next.len_utf8();
                } else {
                    break;
                }
            }

            if token.is_empty() {
                out.push(ch);
                idx += ch_len;
                continue;
            }

            let resolved = resolve_token(property, &token, theme).map_err(|e| {
                span_error(span.with_offset(idx), format!("RCSS token error: {}", e))
            })?;
            out.push_str(&resolved);
            idx = cursor;
            continue;
        }

        out.push(ch);
        idx += ch_len;
    }

    Ok(out)
}

fn consume_parenthesized(value: &str, start: usize) -> Result<(String, usize), String> {
    let mut depth = 0;
    for (offset, ch) in value[start..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let inner = value[start + 1..start + offset].to_string();
                    return Ok((inner, offset + 1));
                }
            }
            _ => {}
        }
    }
    Err("Unterminated @(...) expression".to_string())
}

fn prev_char(value: &str, idx: usize) -> Option<char> {
    if idx == 0 {
        return None;
    }
    value[..idx].chars().rev().next()
}

fn is_token_boundary(prev: Option<char>) -> bool {
    match prev {
        None => true,
        Some(ch) => {
            ch.is_whitespace()
                || matches!(
                    ch,
                    '(' | ')'
                        | '{'
                        | '}'
                        | '['
                        | ']'
                        | ','
                        | ';'
                        | ':'
                        | '+'
                        | '-'
                        | '*'
                        | '/'
                        | '%'
                )
        }
    }
}

fn span_error(span: Span, message: impl Into<String>) -> String {
    let msg = message.into();
    if span.line == 0 && span.column == 0 {
        msg
    } else {
        format!("{}: {}", span, msg)
    }
}

fn is_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '/' || ch == '.'
}

fn apply_color_functions(value: &str) -> String {
    let mut out = String::new();
    let mut idx = 0;

    while idx < value.len() {
        if let Some((name, inner, consumed)) = consume_color_function(value, idx) {
            let expanded = expand_color_function(name, &inner);
            out.push_str(&expanded);
            idx += consumed;
            continue;
        }

        let ch = value[idx..].chars().next().unwrap();
        out.push(ch);
        idx += ch.len_utf8();
    }

    out
}

fn consume_color_function(value: &str, start: usize) -> Option<(&'static str, String, usize)> {
    const FUNCTIONS: [&str; 7] = ["mix", "lighten", "darken", "alpha", "shade", "tint", "tone"];

    for name in FUNCTIONS {
        if let Some(bytes) = value.get(start..) {
            if bytes.len() < name.len() + 1 {
                continue;
            }
            if !bytes.starts_with(&format!("{}(", name)) {
                continue;
            }
            let open_idx = start + name.len();
            if value.as_bytes().get(open_idx) != Some(&b'(') {
                continue;
            }

            let mut depth = 1;
            let mut i = open_idx + 1;
            while i < value.len() {
                match value.as_bytes()[i] {
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            let inner = value[open_idx + 1..i].to_string();
                            return Some((name, inner, i - start + 1));
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        }
    }

    None
}

fn expand_color_function(name: &str, inner: &str) -> String {
    let args = split_color_args(inner);
    let resolved: Vec<String> = args
        .into_iter()
        .map(|arg| apply_color_functions(&arg))
        .collect();

    match name {
        "mix" if resolved.len() == 3 => {
            let color1 = resolved[0].trim();
            let color2 = resolved[1].trim();
            let percent = resolved[2].trim();
            format!("color-mix(in srgb, {} {}, {})", color2, percent, color1)
        }
        "lighten" if resolved.len() == 2 => {
            let color = resolved[0].trim();
            let amount = resolved[1].trim();
            format!("color-mix(in srgb, white {}, {})", amount, color)
        }
        "darken" if resolved.len() == 2 => {
            let color = resolved[0].trim();
            let amount = resolved[1].trim();
            format!("color-mix(in srgb, black {}, {})", amount, color)
        }
        "shade" if resolved.len() == 2 => {
            let color = resolved[0].trim();
            let amount = resolved[1].trim();
            format!("color-mix(in srgb, black {}, {})", amount, color)
        }
        "tint" if resolved.len() == 2 => {
            let color = resolved[0].trim();
            let amount = resolved[1].trim();
            format!("color-mix(in srgb, white {}, {})", amount, color)
        }
        "tone" if resolved.len() == 2 => {
            let color = resolved[0].trim();
            let amount = resolved[1].trim();
            format!("color-mix(in srgb, gray {}, {})", amount, color)
        }
        "alpha" if resolved.len() == 2 => {
            let color = resolved[0].trim();
            let amount = resolved[1].trim();
            format!("color-mix(in srgb, {} {}, transparent)", color, amount)
        }
        _ => format!("{}({})", name, inner),
    }
}

fn split_color_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for ch in input.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                if depth > 0 {
                    depth -= 1;
                }
                current.push(ch);
            }
            ',' if depth == 0 => {
                args.push(current.trim().to_string());
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.trim().is_empty() {
        args.push(current.trim().to_string());
    }

    args
}

#[derive(Debug)]
enum GridCommand {
    Masonry,
    Cols(String),
    Rows(Vec<String>),
    Gap(Vec<String>),
    Areas(Vec<String>),
    Columns(String),
}

struct GridBlockEntry {
    key: String,
    value: String,
}

fn parse_grid_commands(value: &str) -> Result<Vec<GridCommand>, String> {
    let mut commands = Vec::new();
    for segment in split_grid_segments(value) {
        commands.push(parse_grid_segment(&segment)?);
    }
    if commands.is_empty() {
        return Err("RCSS grid error: grid() requires at least one subcommand".to_string());
    }
    Ok(commands)
}

fn split_grid_segments(value: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut in_quote = false;

    for ch in value.chars() {
        if ch == '"' {
            in_quote = !in_quote;
            current.push(ch);
            continue;
        }

        if !in_quote {
            match ch {
                '(' => {
                    depth += 1;
                    current.push(ch);
                    continue;
                }
                ')' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    current.push(ch);
                    continue;
                }
                c if c.is_whitespace() && depth == 0 => {
                    if !current.trim().is_empty() {
                        segments.push(current.trim().to_string());
                        current.clear();
                    }
                    continue;
                }
                _ => {}
            }
        }

        current.push(ch);
    }

    if !current.trim().is_empty() {
        segments.push(current.trim().to_string());
    }

    segments
}

fn parse_grid_segment(segment: &str) -> Result<GridCommand, String> {
    let trimmed = segment.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower == "masonry" {
        return Ok(GridCommand::Masonry);
    }

    if let Some(inner) = extract_parenthesized(trimmed, "cols") {
        return Ok(GridCommand::Cols(inner.to_string()));
    }
    if let Some(inner) = extract_parenthesized(trimmed, "rows") {
        return Ok(GridCommand::Rows(split_arguments(inner)));
    }
    if let Some(inner) = extract_parenthesized(trimmed, "gap") {
        let args = split_arguments(inner);
        if args.is_empty() || args.len() > 2 {
            return Err("RCSS grid error: gap() expects one or two values".to_string());
        }
        return Ok(GridCommand::Gap(args));
    }
    if let Some(inner) = extract_parenthesized(trimmed, "areas") {
        let lines = parse_areas(inner)?;
        return Ok(GridCommand::Areas(lines));
    }
    if let Some(inner) = extract_parenthesized(trimmed, "columns") {
        return Ok(GridCommand::Columns(inner.to_string()));
    }

    Err(format!(
        "RCSS grid error: unknown grid command \"{}\"",
        trimmed
    ))
}

fn extract_parenthesized<'a>(input: &'a str, keyword: &str) -> Option<&'a str> {
    let low = input.to_ascii_lowercase();
    if !low.starts_with(&format!("{}(", keyword)) || !input.ends_with(')') {
        return None;
    }
    let start = keyword.len() + 1;
    input.get(start..input.len() - 1).map(str::trim)
}

fn split_arguments(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut in_quote = false;

    for ch in input.chars() {
        if ch == '"' {
            in_quote = !in_quote;
            current.push(ch);
            continue;
        }

        if !in_quote {
            match ch {
                '(' => {
                    depth += 1;
                    current.push(ch);
                    continue;
                }
                ')' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    current.push(ch);
                    continue;
                }
                c if c.is_whitespace() && depth == 0 => {
                    if !current.trim().is_empty() {
                        args.push(current.trim().to_string());
                        current.clear();
                    }
                    continue;
                }
                _ => {}
            }
        }

        current.push(ch);
    }

    if !current.trim().is_empty() {
        args.push(current.trim().to_string());
    }

    args
}

fn extract_grid_block_entry(property: &str, value: &str) -> Option<GridBlockEntry> {
    if let Some(rest) = property.strip_prefix("grid.") {
        if rest.is_empty() {
            return None;
        }
        return Some(GridBlockEntry {
            key: rest.to_string(),
            value: value.to_string(),
        });
    }
    None
}

fn build_grid_commands_from_block(entries: &[GridBlockEntry]) -> Result<Vec<GridCommand>, String> {
    if entries.is_empty() {
        return Err("RCSS grid error: grid block requires at least one entry".to_string());
    }

    let mut commands = Vec::new();
    for entry in entries {
        let key = entry.key.to_ascii_lowercase();
        match key.as_str() {
            "masonry" => commands.push(GridCommand::Masonry),
            "cols" => commands.push(GridCommand::Cols(entry.value.clone())),
            "columns" => commands.push(GridCommand::Columns(entry.value.clone())),
            "rows" => commands.push(GridCommand::Rows(split_arguments(&entry.value))),
            "gap" => commands.push(GridCommand::Gap(split_arguments(&entry.value))),
            "areas" => {
                let lines = parse_areas(&entry.value)?;
                commands.push(GridCommand::Areas(lines));
            }
            other => {
                return Err(format!(
                    "RCSS grid error: unknown grid block command \"{}\"",
                    other
                ));
            }
        }
    }

    Ok(commands)
}

fn parse_areas(input: &str) -> Result<Vec<String>, String> {
    let mut areas = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }
        if ch != '"' {
            return Err(format!(
                "RCSS grid error: areas() expects quoted strings, got '{}'",
                ch
            ));
        }

        let mut content = String::new();
        while let Some(next) = chars.next() {
            if next == '"' {
                break;
            }
            content.push(next);
        }
        areas.push(format!("\"{}\"", content));
    }

    if areas.is_empty() {
        return Err("RCSS grid error: areas() requires at least one quoted string".to_string());
    }

    Ok(areas)
}

fn build_grid_declarations(
    commands: &[GridCommand],
    theme: &Theme,
    display_defined: bool,
    span: Span,
    variables: &HashMap<String, String>,
) -> Result<(Vec<crate::ast::Declaration>, bool), String> {
    use GridCommand::*;

    let mut masonry_mode = false;
    let mut grid_sourced = false;
    let mut cols_arg: Option<String> = None;
    let mut rows_arg: Option<Vec<String>> = None;
    let mut gap_arg: Option<Vec<String>> = None;
    let mut areas_arg: Option<Vec<String>> = None;
    let mut columns_arg: Option<String> = None;

    for command in commands {
        match command {
            Masonry => {
                if grid_sourced {
                    return Err(
                        "RCSS grid error: cannot mix masonry and grid subcommands in one grid: declaration"
                            .to_string(),
                    );
                }
                masonry_mode = true;
            }
            Cols(value) => {
                if masonry_mode {
                    return Err(
                        "RCSS grid error: cannot mix masonry and grid subcommands in one grid: declaration"
                            .to_string(),
                    );
                }
                grid_sourced = true;
                cols_arg = Some(value.clone());
            }
            Rows(values) => {
                if masonry_mode {
                    return Err(
                        "RCSS grid error: cannot mix masonry and grid subcommands in one grid: declaration"
                            .to_string(),
                    );
                }
                grid_sourced = true;
                rows_arg = Some(values.clone());
            }
            Gap(values) => {
                gap_arg = Some(values.clone());
            }
            Areas(values) => {
                if masonry_mode {
                    return Err(
                        "RCSS grid error: cannot mix masonry and grid subcommands in one grid: declaration"
                            .to_string(),
                    );
                }
                grid_sourced = true;
                areas_arg = Some(values.clone());
            }
            Columns(value) => {
                columns_arg = Some(value.clone());
            }
        }
    }

    if columns_arg.is_some() && !masonry_mode {
        return Err("RCSS grid error: columns() expects masonry mode".to_string());
    }

    let needs_display = masonry_mode
        || cols_arg.is_some()
        || rows_arg.is_some()
        || areas_arg.is_some()
        || gap_arg.is_some();

    let mut declarations = Vec::new();
    let mut display_added = false;

    if masonry_mode {
        if !display_defined && needs_display {
            declarations.push(Declaration {
                property: "display".to_string(),
                value: "block".to_string(),
                span,
            });
            display_added = true;
        }

        if let Some(columns) = columns_arg {
            let count = resolve_grid_integer(columns, "columns()", theme, span, variables)?;
            declarations.push(Declaration {
                property: "column-count".to_string(),
                value: count.to_string(),
                span,
            });
        }

        if let Some(gap_values) = gap_arg {
            if gap_values.is_empty() {
                return Err("RCSS grid error: gap() expects a value".to_string());
            }
            if gap_values.len() > 1 {
                return Err(
                    "RCSS grid error: masonry gap() only supports a single value".to_string(),
                );
            }
            let column_gap = resolve_value(&gap_values[0], "gap", theme, span, variables)?;
            declarations.push(Declaration {
                property: "column-gap".to_string(),
                value: column_gap,
                span,
            });
        }

        return Ok((declarations, display_added));
    }

    if needs_display && !display_defined {
        declarations.push(Declaration {
            property: "display".to_string(),
            value: "grid".to_string(),
            span,
        });
        display_added = true;
    }

    if let Some(cols) = cols_arg {
        let count = resolve_grid_integer(cols, "cols()", theme, span, variables)?;
        let value = format!("repeat({}, minmax(0, 1fr))", count);
        declarations.push(Declaration {
            property: "grid-template-columns".to_string(),
            value,
            span,
        });
    }

    if let Some(rows) = rows_arg {
        let resolved = resolve_value_list(&rows, "gap", theme, span, variables)?;
        declarations.push(Declaration {
            property: "grid-template-rows".to_string(),
            value: resolved.join(" "),
            span,
        });
    }

    if let Some(gap_values) = gap_arg {
        match gap_values.len() {
            1 => {
                let resolved = resolve_value(&gap_values[0], "gap", theme, span, variables)?;
                declarations.push(Declaration {
                    property: "gap".to_string(),
                    value: resolved,
                    span,
                });
            }
            2 => {
                let row_gap = resolve_value(&gap_values[0], "gap", theme, span, variables)?;
                let col_gap = resolve_value(&gap_values[1], "gap", theme, span, variables)?;
                declarations.push(Declaration {
                    property: "row-gap".to_string(),
                    value: row_gap,
                    span,
                });
                declarations.push(Declaration {
                    property: "column-gap".to_string(),
                    value: col_gap,
                    span,
                });
            }
            _ => {}
        }
    }

    if let Some(areas) = areas_arg {
        let joined = areas.join("\n    ");
        let value = format!("\n    {}", joined);
        declarations.push(Declaration {
            property: "grid-template-areas".to_string(),
            value,
            span,
        });
    }

    Ok((declarations, display_added))
}

fn resolve_value_list(
    values: &[String],
    property: &str,
    theme: &Theme,
    span: Span,
    variables: &HashMap<String, String>,
) -> Result<Vec<String>, String> {
    values
        .iter()
        .map(|v| resolve_value(v, property, theme, span, variables))
        .collect()
}

fn resolve_grid_integer(
    value: String,
    context: &str,
    theme: &Theme,
    span: Span,
    variables: &HashMap<String, String>,
) -> Result<i32, String> {
    let raw = value.trim();
    if let Ok(num) = raw.parse::<i32>() {
        return Ok(num);
    }
    if let Some(stripped) = raw.strip_prefix('@') {
        if let Ok(num) = stripped.trim().parse::<i32>() {
            return Ok(num);
        }
    }
    let resolved = resolve_value(raw, "columns", theme, span, variables).map_err(|_| {
        span_error(
            span,
            format!(
                "RCSS grid error: {} expects a number, got \"{}\"",
                context, raw
            ),
        )
    })?;
    resolved.trim().parse::<i32>().map_err(|_| {
        span_error(
            span,
            format!(
                "RCSS grid error: {} expects a number, got \"{}\"",
                context, raw
            ),
        )
    })
}

fn expand_shorthand(
    property: &str,
    value: &str,
    theme: &Theme,
) -> Result<Option<Vec<(String, String, bool)>>, String> {
    let property_key = normalize_property(property);
    let def: &ShorthandDef = match theme.shorthands.get(&property_key) {
        Some(s) => s,
        None => return Ok(None),
    };

    let assignments = parse_assignments(value, def.order.as_deref(), &property_key)?;
    let mut resolved: HashMap<String, String> = HashMap::new();

    for (name, token) in assignments {
        let prop_name = if name == "token" {
            property_key.clone()
        } else {
            normalize_property(&name)
        };

        let resolved_value = if token.starts_with('@') {
            let cleaned = token.trim_start_matches('@');
            resolve_token(&prop_name, cleaned, theme)?
        } else {
            token.clone()
        };

        // Store under original name, camelCase, and kebab-case to satisfy template lookups
        resolved.insert(name.clone(), resolved_value.clone());
        let camel = normalize_property(&name);
        resolved.insert(camel, resolved_value.clone());
        let kebab = name.replace('_', "-");
        resolved.insert(kebab, resolved_value);
    }

    let mut out = Vec::new();
    for step in &def.steps {
        match apply_template(&step.template, &resolved) {
            Some(rendered) => out.push((step.property.clone(), rendered, step.append)),
            None if step.optional => continue,
            None => {
                return Err(format!(
                    "Missing required value for shorthand '{}' template '{}'",
                    property, step.template
                ));
            }
        }
    }

    Ok(Some(out))
}

fn parse_assignments(
    value: &str,
    positional: Option<&[String]>,
    shorthand_key: &str,
) -> Result<Vec<(String, String)>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("Shorthand value cannot be empty".to_string());
    }

    let alias_map = build_alias_map(shorthand_key, positional);
    let mut result = Vec::new();
    let mut used_keys: HashSet<String> = HashSet::new();
    let mut positional_index = 0;

    for part in trimmed.split_whitespace() {
        if let Some((key, val)) = part.split_once('=') {
            let canonical = canonical_key(key.trim(), &alias_map);
            used_keys.insert(canonical.clone());
            result.push((canonical, val.trim().to_string()));
            continue;
        }

        if part.ends_with(')') && part.contains('(') {
            if let Some(idx) = part.find('(') {
                let key = part[..idx].trim();
                let val = part[idx + 1..part.len() - 1].trim();
                if key.is_empty() || val.is_empty() {
                    return Err(format!("Invalid shorthand part '{}'", part));
                }
                let canonical = canonical_key(key, &alias_map);
                used_keys.insert(canonical.clone());
                result.push((canonical, val.to_string()));
                continue;
            }
        }

        if let Some(keys) = positional {
            while positional_index < keys.len() && used_keys.contains(&keys[positional_index]) {
                positional_index += 1;
            }
            if positional_index < keys.len() {
                let key = keys[positional_index].to_string();
                positional_index += 1;
                used_keys.insert(key.clone());
                result.push((key, part.to_string()));
                continue;
            }
        }

        if part.starts_with('@') {
            // single token -> use placeholder "token"
            result.push(("token".to_string(), part.to_string()));
        } else {
            return Err(format!("Invalid shorthand part '{}'", part));
        }
    }

    Ok(result)
}

fn build_alias_map(shorthand_key: &str, positional: Option<&[String]>) -> HashMap<String, String> {
    let mut aliases = HashMap::new();

    if let Some(keys) = positional {
        for canonical in keys {
            insert_alias(&mut aliases, canonical, canonical);
            insert_alias(&mut aliases, &normalize_property(canonical), canonical);
            insert_alias(&mut aliases, &to_kebab_case(canonical), canonical);

            if let Some(stripped) = canonical.strip_prefix(shorthand_key) {
                let alias = lowercase_first(stripped);
                if !alias.is_empty() {
                    insert_alias(&mut aliases, &alias, canonical);
                    insert_alias(&mut aliases, &to_kebab_case(&alias), canonical);
                }
            }
        }
    }

    aliases
}

fn canonical_key(key: &str, aliases: &HashMap<String, String>) -> String {
    let attempts = [key.to_string(), normalize_property(key), to_kebab_case(key)];

    for candidate in &attempts {
        if let Some(mapped) = aliases.get(candidate) {
            return mapped.clone();
        }
    }

    key.to_string()
}

fn insert_alias(target: &mut HashMap<String, String>, alias: &str, canonical: &str) {
    target
        .entry(alias.to_string())
        .or_insert_with(|| canonical.to_string());
}

fn lowercase_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_lowercase(), chars.as_str()),
        None => String::new(),
    }
}

fn to_kebab_case(input: &str) -> String {
    let mut out = String::new();
    for (i, ch) in input.chars().enumerate() {
        if ch == '_' {
            out.push('-');
            continue;
        }

        if ch.is_uppercase() {
            if i != 0 {
                out.push('-');
            }
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn apply_template(template: &str, values: &HashMap<String, String>) -> Option<String> {
    let mut out = String::new();
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '@' && chars.peek() == Some(&'{') {
            // consume '{'
            chars.next();
            let mut name = String::new();
            while let Some(&ch) = chars.peek() {
                chars.next();
                if ch == '}' {
                    break;
                }
                name.push(ch);
            }

            if let Some(val) = values.get(&name) {
                out.push_str(val);
            } else {
                return None;
            }
        } else {
            out.push(c);
        }
    }

    Some(out)
}

fn merge_declarations(
    target: &mut Vec<crate::ast::Declaration>,
    expanded: Vec<(String, String, bool)>,
) {
    for (prop, val, append) in expanded {
        let mut joined = false;

        if let Some(existing) = target.iter_mut().rev().find(|d| d.property == prop) {
            if append || prop == "box-shadow" {
                existing.value = format!("{}, {}", existing.value, val);
                joined = true;
            } else {
                existing.value = val.clone();
                joined = true;
            }
        }

        if joined {
            continue;
        }
        target.push(crate::ast::Declaration {
            property: prop,
            value: val,
            span: Span::dummy(),
        });
    }
}

fn map_border_subproperty(property: &str) -> Option<&'static str> {
    match property {
        "border.color" => Some("border-color"),
        "border.width" => Some("border-width"),
        "border.style" => Some("border-style"),
        _ => None,
    }
}

fn map_flex_subproperty(property: &str) -> Option<&'static str> {
    match property {
        "flex.direction" => Some("flex-direction"),
        "flex.wrap" => Some("flex-wrap"),
        "flex.justify" => Some("justify-content"),
        "flex.align" => Some("align-items"),
        "flex.content" => Some("align-content"),
        "flex.gap" => Some("gap"),
        _ => None,
    }
}

fn extract_radius_key(property: &str) -> Option<&str> {
    if property.starts_with("radius.") {
        return property.strip_prefix("radius.");
    }
    if let Some(idx) = property.find(".radius.") {
        return property.get(idx + ".radius.".len()..);
    }
    None
}

fn expand_radius_entries(
    entries: &[(String, String)],
    theme: &Theme,
    variables: &HashMap<String, String>,
) -> Result<Vec<Declaration>, String> {
    let mut spec = RadiusSpec::default();
    for (key, value) in entries {
        spec.apply_entry(key, value.clone());
    }

    let mut declarations = Vec::new();
    for &corner in &["top-left", "top-right", "bottom-right", "bottom-left"] {
        if let Some(raw) = spec.corner_value(corner) {
            let property = format!("border-{}-radius", corner);
            let resolved = resolve_radius_value(&raw, &property, theme, variables)?;
            declarations.push(Declaration {
                property,
                value: resolved,
                span: Span::dummy(),
            });
        }
    }

    Ok(declarations)
}

fn resolve_radius_value(
    raw: &str,
    property: &str,
    theme: &Theme,
    variables: &HashMap<String, String>,
) -> Result<String, String> {
    match resolve_value(raw, property, theme, Span::dummy(), variables) {
        Ok(value) => Ok(value),
        Err(original) => {
            let trimmed = raw.trim();
            if let Some(stripped) = trimmed.strip_prefix('@') {
                let resolved = resolve_from_collection("spacing", stripped, theme)?;
                return value_to_css(&resolved, "spacing", stripped);
            }
            Err(original)
        }
    }
}

fn resolve_color_token(token: &str, theme: &Theme) -> Result<String, String> {
    if let Some((base_token, opacity_token)) = token.split_once('/') {
        let base_value = resolve_from_collection("colors", base_token, theme)?;
        let opacity_value = resolve_from_collection("opacity", opacity_token, theme)?;
        return inject_alpha(base_value, opacity_value);
    }

    let value = resolve_from_collection("colors", token, theme)?;
    value_to_css(&value, "textColor", token)
}

#[derive(Default)]
struct RadiusSpec {
    all: Option<String>,
    inline: Option<Vec<String>>,
    inline_start: Option<String>,
    inline_end: Option<String>,
    block: Option<Vec<String>>,
    block_start: Option<String>,
    block_end: Option<String>,
    corners: HashMap<String, String>,
}

impl RadiusSpec {
    fn apply_entry(&mut self, key: &str, value: String) {
        let normalized = key.replace('_', "-");
        match normalized.as_str() {
            "all" => self.all = Some(value),
            "inline" => self.inline = Some(split_values(&value)),
            "inline-start" => self.inline_start = Some(value),
            "inline-end" => self.inline_end = Some(value),
            "block" => self.block = Some(split_values(&value)),
            "block-start" => self.block_start = Some(value),
            "block-end" => self.block_end = Some(value),
            corner => {
                if ["top-left", "top-right", "bottom-left", "bottom-right"].contains(&corner) {
                    self.corners.insert(corner.to_string(), value);
                }
            }
        }
    }

    fn corner_value(&self, corner: &str) -> Option<String> {
        if let Some(val) = self.corners.get(corner) {
            return Some(val.clone());
        }

        let mut current = self.all.clone();

        if let Some(val) = self.inline_value(corner) {
            current = Some(val);
        }

        if let Some(val) = self.block_value(corner) {
            current = Some(val);
        }

        current
    }

    fn inline_value(&self, corner: &str) -> Option<String> {
        if is_inline_start(corner) {
            if let Some(val) = &self.inline_start {
                return Some(val.clone());
            }
            return self.inline.as_ref().and_then(|vals| vals.get(0).cloned());
        }

        if let Some(val) = &self.inline_end {
            return Some(val.clone());
        }

        self.inline.as_ref().and_then(|vals| {
            if vals.len() > 1 {
                vals.get(1).cloned()
            } else {
                vals.get(0).cloned()
            }
        })
    }

    fn block_value(&self, corner: &str) -> Option<String> {
        if is_block_start(corner) {
            if let Some(val) = &self.block_start {
                return Some(val.clone());
            }
            return self.block.as_ref().and_then(|vals| vals.get(0).cloned());
        }

        if let Some(val) = &self.block_end {
            return Some(val.clone());
        }

        self.block.as_ref().and_then(|vals| {
            if vals.len() > 1 {
                vals.get(1).cloned()
            } else {
                vals.get(0).cloned()
            }
        })
    }
}

fn split_values(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .map(|token| token.to_string())
        .collect()
}

fn is_inline_start(corner: &str) -> bool {
    corner.ends_with("left")
}

fn is_block_start(corner: &str) -> bool {
    corner.starts_with("top")
}
fn resolve_token(property: &str, token: &str, theme: &Theme) -> Result<String, String> {
    let lookup_target = if property.starts_with("--") {
        "color"
    } else {
        property
    };

    let property_key = normalize_property(lookup_target);

    // direct url syntax: @(...) -> url("...")
    if token.starts_with('(') && token.ends_with(')') {
        let inner = token.trim_start_matches('(').trim_end_matches(')');
        return Ok(format!("url(\"{}\")", inner));
    }
    let mapping = property_mapping(&property_key, theme);

    if let Some((base_token, opacity_token)) = token.split_once('/') {
        if let Some(mapping) = &mapping {
            if let Ok(base_value) = resolve_from_mapping(base_token, mapping, theme) {
                let opacity_value = resolve_from_collection("opacity", opacity_token, theme)?;
                return inject_alpha(base_value, opacity_value);
            }
        }

        let base_value = resolve_from_collection("colors", base_token, theme)?;
        let opacity_value = resolve_from_collection("opacity", opacity_token, theme)?;
        return inject_alpha(base_value, opacity_value);
    }

    if let Some(mapping) = &mapping {
        if let Ok(value) = resolve_from_mapping(token, mapping, theme) {
            return value_to_css(&value, &property_key, token);
        }
        return resolve_color_token(token, theme);
    }

    Err(format!("Unknown property '{}'", property))
}

fn property_mapping<'a>(
    property_key: &str,
    theme: &'a Theme,
) -> Option<&'a crate::theme::PropertyMapping> {
    // CSS aliases to theme property keys
    if property_key == "color" {
        return theme.properties.get("textColor");
    }

    if property_key == "background" {
        return theme.properties.get("backgroundColor");
    }

    if property_key == "from" || property_key == "via" || property_key == "to" {
        return theme.properties.get("gradientColorStops");
    }

    if property_key == "shadow" {
        return theme.properties.get("boxShadow");
    }

    if property_key == "offsetWidth" {
        return theme.properties.get("ringOffsetWidth");
    }

    if property_key == "offsetColor" {
        return theme.properties.get("ringOffsetColor");
    }

    if property_key == "family" {
        return theme.properties.get("fontFamily");
    }

    if property_key == "size" {
        return theme.properties.get("fontSize");
    }

    if property_key == "weight" {
        return theme.properties.get("fontWeight");
    }

    if property_key == "lineHeight" {
        return theme.properties.get("lineHeight");
    }

    if property_key == "radius" {
        return theme.properties.get("borderRadius");
    }

    if property_key.starts_with("border")
        && property_key.ends_with("Radius")
        && property_key != "borderRadius"
    {
        return theme.properties.get("borderRadius");
    }

    if let Some(m) = theme.properties.get(property_key) {
        return Some(m);
    }

    None
}

fn resolve_from_mapping(
    token: &str,
    mapping: &crate::theme::PropertyMapping,
    theme: &Theme,
) -> Result<Value, String> {
    if let Some(override_value) = mapping.overrides.get(token) {
        return Ok(override_value.clone());
    }

    resolve_from_collection(&mapping.collection, token, theme)
}

fn resolve_from_collection(
    collection_name: &str,
    token: &str,
    theme: &Theme,
) -> Result<Value, String> {
    let collection = theme
        .collections
        .get(collection_name)
        .ok_or_else(|| format!("Unknown collection '{}'", collection_name))?;

    // direct key match (for flat keys containing dashes)
    if let Value::Object(map) = collection {
        if let Some(val) = map.get(token) {
            return Ok(val.clone());
        }
    }

    let mut current = collection;
    for part in token.split('-') {
        match current {
            Value::Object(map) => {
                current = map
                    .get(part)
                    .ok_or_else(|| format!("Unknown token '{}' in {}", token, collection_name))?;
            }
            _ => {
                return Err(format!(
                    "Token '{}' does not map into collection '{}'",
                    token, collection_name
                ));
            }
        }
    }
    Ok(current.clone())
}

fn normalize_property(property: &str) -> String {
    if !property.contains('-') {
        return property.to_string();
    }

    let mut result = String::new();
    for (i, part) in property.split('-').enumerate() {
        if i == 0 {
            result.push_str(part);
        } else {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                result.push(first.to_ascii_uppercase());
                result.push_str(chars.as_str());
            }
        }
    }
    result
}

fn value_to_css(value: &Value, property: &str, token: &str) -> Result<String, String> {
    if let Some(url) = value
        .as_str()
        .and_then(|s| s.strip_prefix("@("))
        .and_then(|s| s.strip_suffix(')'))
    {
        return Ok(format!("url(\"{}\")", url));
    }

    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Array(items) => {
            if let Some(Value::String(s)) = items.get(0) {
                Ok(s.clone())
            } else {
                serde_json::to_string(value).map_err(|e| {
                    format!(
                        "Cannot stringify token '@{}' for {}: {}",
                        token, property, e
                    )
                })
            }
        }
        Value::Object(_) => serde_json::to_string(value).map_err(|e| {
            format!(
                "Cannot stringify token '@{}' for {}: {}",
                token, property, e
            )
        }),
        Value::Null => Err(format!("Token '@{}' for {} is null", token, property)),
    }
}

fn inject_alpha(base: Value, opacity: Value) -> Result<String, String> {
    let base_str = match base {
        Value::String(s) => s,
        other => {
            serde_json::to_string(&other).map_err(|e| format!("Cannot stringify color: {}", e))?
        }
    };

    let opacity_str = match opacity {
        Value::String(s) => s,
        Value::Number(n) => n.to_string(),
        other => {
            serde_json::to_string(&other).map_err(|e| format!("Cannot stringify opacity: {}", e))?
        }
    };

    if base_str.starts_with("oklch(") || base_str.contains('(') {
        let base = base_str.trim_end_matches(')');
        return Ok(format!("{} / {})", base, opacity_str));
    }

    if base_str.starts_with('#') {
        return hex_to_rgba(&base_str, &opacity_str);
    }

    Err(format!("Unsupported color format: {}", base_str))
}

fn hex_to_rgba(hex: &str, opacity: &str) -> Result<String, String> {
    let hex = hex.trim_start_matches('#');

    let (r, g, b) = match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
            (r, g, b)
        }
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).unwrap();
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).unwrap();
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).unwrap();
            (r, g, b)
        }
        _ => return Err(format!("Invalid hex color: {}", hex)),
    };

    Ok(format!("rgba({},{},{},{})", r, g, b, opacity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::{emitter, parser};

    fn render_css(input: &str) -> String {
        let theme_dir = format!("{}/../../theme", env!("CARGO_MANIFEST_DIR"));
        let theme = Theme::load_from_dir(&theme_dir).expect("load theme");
        let stylesheet = parser::parse(input).expect("parse rc");
        let resolved = resolve(stylesheet, &theme).expect("resolve");
        emitter::emit_css(&resolved)
    }

    #[test]
    fn grid_cols_and_gap() {
        let css = render_css(".demo { grid: cols(4) gap(@2); }");
        assert!(css.contains("display: grid;"));
        assert!(css.contains("grid-template-columns: repeat(4, minmax(0, 1fr));"));
        assert!(css.contains("gap: 0.5rem;"));
    }

    #[test]
    fn grid_template_areas() {
        let css = render_css(".demo { grid: areas(\"a a\" \"b c\"); }");
        assert!(css.contains("grid-template-areas:"));
        assert!(css.contains("\n    \"a a\""));
        assert!(css.contains("\n    \"b c\""));
    }

    #[test]
    fn masonry_columns_gap() {
        let css = render_css(".demo { grid: masonry columns(3) gap(@4); }");
        assert!(css.contains("display: block;"));
        assert!(css.contains("column-count: 3;"));
        assert!(css.contains("column-gap: 1rem;"));
    }

    #[test]
    fn grid_with_tokens() {
        let css = render_css(".demo { grid: cols(@3) rows(@4 auto 1fr); }");
        assert!(css.contains("grid-template-columns: repeat(3, minmax(0, 1fr));"));
        assert!(css.contains("grid-template-rows: 1rem auto 1fr;"));
    }

    #[test]
    fn color_functions() {
        let css = render_css(
            ".demo { color: mix(@blue-500, @red-400, 40%); background: lighten(@blue-500, 20%); border-color: darken(@slate-500, 10%); }",
        );
        assert!(css.contains("color-mix(in srgb"));
        assert!(css.contains("background: color-mix(in srgb, white 20%"));
        assert!(css.contains("border-color: color-mix(in srgb, black 10%"));
    }

    #[test]
    fn radius_block_expansion() {
        let css =
            render_css(".demo { radius { all: @lg; block: @2 @4; inline: @4; top-left: @sm; } }");
        assert!(css.contains("border-top-left-radius:"));
        assert!(css.contains("border-top-right-radius:"));
        assert!(css.contains("border-bottom-right-radius:"));
    }

    #[test]
    fn border_block_expansion() {
        let css = render_css(
            ".demo { border { color: @slate-500; width: @2; style: solid; radius { all: @lg; } } }",
        );
        assert!(css.contains("border-color:"));
        assert!(css.contains("border-width:"));
        assert!(css.contains("border-top-left-radius:"));
    }

    #[test]
    fn flex_block_expansion() {
        let css = render_css(
            ".demo { display: flex; flex { direction: row; wrap: wrap; justify: center; align: center; content: between; gap: @4; } }",
        );
        assert!(css.contains("flex-direction: row;"));
        assert!(css.contains("flex-wrap: wrap;"));
        assert!(css.contains("justify-content: center;"));
        assert!(css.contains("align-items: center;"));
        assert!(css.contains("align-content: between;"));
        assert!(css.contains("gap: 1rem;"));
    }

    #[test]
    fn inline_tokens_in_calc() {
        let css = render_css(".demo { width: calc(100% - @4); }");
        assert!(css.contains("calc(100% - 1rem)"));
    }

    #[test]
    fn gradient_tokens_interpolation() {
        let css =
            render_css(".demo { background: linear-gradient(to right, @red-500, @blue-400/50); }");
        assert!(css.contains("linear-gradient(to right,"));
        assert!(css.contains("oklch"));
    }

    #[test]
    fn tokens_inside_url_wrapper() {
        let css = render_css(".demo { background: @(images/@blue-500.svg); }");
        assert!(css.contains("url(\"images/oklch"));
    }

    #[test]
    fn tokens_adjacent_to_text_error() {
        let theme_dir = format!("{}/../../theme", env!("CARGO_MANIFEST_DIR"));
        let theme = Theme::load_from_dir(&theme_dir).expect("load theme");
        let stylesheet = parser::parse(".demo { width: 0px@4; }").expect("parse rc");
        let err = resolve(stylesheet, &theme).expect_err("expected token error");
        assert!(err.contains("tokens must be separated"));
    }

    #[test]
    fn variable_substitution_simple() {
        let css = render_css("$spacing: @3;\n.demo { padding: $spacing; }");
        assert!(css.contains("padding: 0.75rem;"));
    }

    #[test]
    fn variable_substitution_calc() {
        let css = render_css("$spacing: @4;\n.demo { width: calc(100% - $spacing); }");
        assert!(css.contains("calc(100% - 1rem)"));
    }

    #[test]
    fn variable_substitution_boundaries() {
        let css = render_css("$sm: @4;\n$small: @3;\n.demo { padding: $small; }");
        assert!(css.contains("padding: 0.75rem;"));
    }

    #[test]
    fn unknown_variable_errors() {
        let theme_dir = format!("{}/../../theme", env!("CARGO_MANIFEST_DIR"));
        let theme = Theme::load_from_dir(&theme_dir).expect("load theme");
        let stylesheet = parser::parse(".demo { border-width: $missing; }").expect("parse rc");
        let err = resolve(stylesheet, &theme).expect_err("expected variable error");
        assert!(err.contains("unknown variable '$missing'"));
    }
}
