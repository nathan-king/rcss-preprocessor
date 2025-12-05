use crate::ast::{Declaration, MediaBlock, Rule, Stylesheet};
use crate::error::Span;
use std::collections::HashMap;

enum PresetScope {
    Root,
    Dark,
    Media {
        query: &'static str,
        selector: &'static str,
    },
}

struct MediaPresetSpec {
    name: String,
    query: &'static str,
    selector: &'static str,
}

pub fn parse(input: &str) -> Result<Stylesheet, String> {
    let mut rules = Vec::new();

    // Extract preset directives and strip them from the input
    let mut base_presets: Vec<String> = Vec::new();
    let mut dark_presets: Vec<String> = Vec::new();
    let mut media_presets: Vec<MediaPresetSpec> = Vec::new();
    let mut variables: HashMap<String, String> = HashMap::new();
    let mut cleaned = String::new();
    let mut blocks: HashMap<String, Vec<Declaration>> = HashMap::new();
    let mut lines = input.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.starts_with('%') {
            let content = trimmed.trim_start_matches('%').trim();
            if !content.is_empty() {
                let mut dark_flag = false;
                for token in content.split_whitespace() {
                    if token == "dark" {
                        dark_flag = true;
                        continue;
                    }

                    let scope = if dark_flag {
                        dark_flag = false;
                        PresetScope::Dark
                    } else {
                        preset_scope_from_name(token)
                    };

                    match scope {
                        PresetScope::Root => base_presets.push(token.to_string()),
                        PresetScope::Dark => dark_presets.push(token.to_string()),
                        PresetScope::Media { query, selector } => {
                            media_presets.push(MediaPresetSpec {
                                name: token.to_string(),
                                query,
                                selector,
                            })
                        }
                    }
                }
            }
            continue;
        }
        if let Some((name, val)) = trimmed.strip_prefix('$').and_then(|s| s.split_once(':')) {
            variables.insert(
                name.trim().to_string(),
                val.trim().trim_end_matches(';').to_string(),
            );
            continue;
        }
        if trimmed.starts_with('$') && trimmed.ends_with('{') {
            let name = trimmed
                .trim_start_matches('$')
                .trim_end_matches('{')
                .trim()
                .to_string();
            let mut body = String::new();
            while let Some(inner) = lines.next() {
                let inner_trim = inner.trim();
                if inner_trim == "}" {
                    break;
                }
                body.push_str(inner);
                body.push('\n');
            }
            let decls = parse_declarations(&body)?;
            blocks.insert(name, decls);
            continue;
        }
        cleaned.push_str(line);
        cleaned.push('\n');
    }

    let bytes = cleaned.as_bytes();
    let len = bytes.len();
    let mut pos = 0;

    while pos < len {
        // skip whitespace
        while pos < len && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos >= len {
            break;
        }

        // find selector end '{'
        let open_idx = match cleaned[pos..].find('{') {
            Some(rel) => pos + rel,
            None => break,
        };
        let selector = cleaned[pos..open_idx].trim().to_string();
        if selector.is_empty() {
            return Err("Missing selector before '{'".to_string());
        }

        // find matching closing brace for this rule
        let mut depth = 1;
        let body_start = open_idx + 1;
        let mut body_end = body_start;
        for (i, ch) in cleaned[body_start..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        body_end = body_start + i;
                        pos = body_end + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        let body = cleaned[body_start..body_end].to_string();

        let selector_terms = split_selector_terms(&selector);
        let (declarations, media, mut nested_rules) =
            parse_rule_body(&body, &selector, &selector_terms, &blocks)?;

        rules.push(Rule {
            selector,
            declarations,
            media,
        });
        rules.append(&mut nested_rules);
    }

    let mut base_list = base_presets;
    if base_list.is_empty() {
        base_list.push("base-16".to_string());
    }
    let mut insert_at = 0;
    if let Some(root_rule) = build_preset_rule(&base_list) {
        rules.insert(insert_at, root_rule);
        insert_at += 1;
    }
    if let Some(dark_rule) = build_dark_preset_rule(&dark_presets) {
        rules.insert(insert_at, dark_rule);
        insert_at += 1;
    }
    for media_rule in build_media_preset_rules(&media_presets) {
        rules.insert(insert_at, media_rule);
        insert_at += 1;
    }

    Ok(Stylesheet { rules, variables })
}

fn parse_rule_body(
    body: &str,
    selector: &str,
    selector_terms: &[String],
    blocks: &HashMap<String, Vec<Declaration>>,
) -> Result<(Vec<Declaration>, Vec<MediaBlock>, Vec<Rule>), String> {
    let normalized_body = normalize_braces(body);
    let mut reader = LineReader::new(&normalized_body);
    parse_rule_body_from_reader(selector, selector_terms, &mut reader, blocks)
}

fn parse_rule_body_from_reader<'a>(
    selector: &str,
    selector_terms: &[String],
    reader: &mut LineReader<'a>,
    blocks: &HashMap<String, Vec<Declaration>>,
) -> Result<(Vec<Declaration>, Vec<MediaBlock>, Vec<Rule>), String> {
    let mut declarations = Vec::new();
    let mut media = Vec::new();
    let mut nested_rules = Vec::new();

    while let Some((raw_line, line_number)) = reader.next_line() {
        let trimmed_line = raw_line.trim();
        if trimmed_line.is_empty()
            || trimmed_line.starts_with("//")
            || trimmed_line.starts_with("/*")
        {
            continue;
        }

        let mut search_offset = 0;
        for fragment_raw in raw_line.split(';') {
            let fragment = fragment_raw.trim();
            if fragment.is_empty() || fragment == "}" {
                search_offset += fragment_raw.len();
                continue;
            }

            let fragment_start = raw_line[search_offset..]
                .find(fragment_raw)
                .map(|pos| search_offset + pos)
                .unwrap_or(search_offset);
            search_offset = fragment_start + fragment_raw.len();

            if fragment.ends_with('{') {
                let header = fragment.trim_end_matches('{').trim();
                if is_media_header(header) {
                    let query = header.to_string();
                    let mut inner_decls = Vec::new();
                    while let Some((_inner_raw, _inner_line)) = reader.next_line() {
                        let inner = _inner_raw.trim();
                        if inner.is_empty() || inner.starts_with("//") || inner.starts_with("/*") {
                            continue;
                        }
                        if inner.ends_with('{') {
                            return Err(
                                "Nested blocks deeper than one level are not supported".to_string()
                            );
                        }
                        if inner == "}" {
                            break;
                        }
                        let inner = inner.trim_end_matches(';');
                        if let Some((prop_part, value_part)) = inner.split_once(':') {
                            let property = prop_part.trim().to_string();
                            let value = value_part.trim().to_string();
                            inner_decls.push(Declaration {
                                property,
                                value,
                                span: Span::dummy(),
                            });
                        } else {
                            return Err(format!("Invalid declaration line: '{}'", inner));
                        }
                    }

                    media.push(MediaBlock {
                        query,
                        declarations: inner_decls,
                    });
                } else if is_property_block_header(header) {
                    let (block_decls, mut nested_from_block) =
                        parse_property_block(header, reader, selector, selector_terms, blocks)?;
                    declarations.extend(block_decls);
                    nested_rules.append(&mut nested_from_block);
                } else {
                    let block_body = collect_block_body(reader)?;
                    let nested_selectors = combine_selectors(selector_terms, header);
                    for nested_selector in nested_selectors {
                        let nested_terms = split_selector_terms(&nested_selector);
                        let (inner_decls, inner_media, mut inner_nested) =
                            parse_rule_body(&block_body, &nested_selector, &nested_terms, blocks)?;
                        nested_rules.push(Rule {
                            selector: nested_selector.clone(),
                            declarations: inner_decls,
                            media: inner_media,
                        });
                        nested_rules.append(&mut inner_nested);
                    }
                }
                continue;
            }

            let fragment = fragment.trim_end_matches(';');
            if let Some((prop_part, value_part)) = fragment.split_once(':') {
                let value_trim = value_part.trim();
                let property_trim = prop_part.trim();
                let _property_col = fragment_start + fragment_raw.find(property_trim).unwrap_or(0);
                let value_col = fragment_start
                    + fragment_raw
                        .find(value_trim)
                        .unwrap_or(property_trim.len() + 1);
                let span = Span {
                    line: line_number,
                    column: value_col + 1,
                };

                let value = value_trim.to_string();
                if property_trim == "apply" && value.starts_with('$') {
                    let key = value.trim_start_matches('$').trim();
                    if let Some(block_decls) = blocks.get(key) {
                        declarations.extend(block_decls.iter().cloned());
                        continue;
                    }
                }

                declarations.push(Declaration {
                    property: property_trim.to_string(),
                    value,
                    span,
                });
            } else {
                return Err(format!("Invalid declaration line: '{}'", fragment));
            }
        }
    }

    Ok((declarations, media, nested_rules))
}

fn is_media_header(header: &str) -> bool {
    let trimmed = header.trim();
    trimmed.starts_with("screen(") || trimmed == "dark" || trimmed == "light"
}

fn parse_property_block<'a>(
    prefix: &str,
    reader: &mut LineReader<'a>,
    parent_selector: &str,
    parent_terms: &[String],
    blocks: &HashMap<String, Vec<Declaration>>,
) -> Result<(Vec<Declaration>, Vec<Rule>), String> {
    let mut decls = Vec::new();
    let mut nested_rules = Vec::new();

    while let Some((_inner_raw, _line)) = reader.next_line() {
        let inner = _inner_raw.trim();
        if inner.is_empty() || inner.starts_with("//") || inner.starts_with("/*") {
            continue;
        }
        if inner == "}" {
            break;
        }

        let mut search_offset = 0;
        for fragment_raw in _inner_raw.split(';') {
            let fragment = fragment_raw.trim();
            if fragment.is_empty() || fragment == "}" {
                search_offset += fragment_raw.len();
                continue;
            }

            let fragment_start = _inner_raw[search_offset..]
                .find(fragment_raw)
                .map(|pos| search_offset + pos)
                .unwrap_or(search_offset);
            search_offset = fragment_start + fragment_raw.len();

            if fragment.ends_with('{') {
                let header = fragment.trim_end_matches('{').trim();
                if is_selector_header(header) {
                    let block_body = collect_block_body(reader)?;
                    let nested_selectors = combine_selectors(parent_terms, header);
                    for nested_selector in nested_selectors {
                        let nested_terms = split_selector_terms(&nested_selector);
                        let (inner_decls, inner_media, mut inner_nested) =
                            parse_rule_body(&block_body, &nested_selector, &nested_terms, blocks)?;
                        nested_rules.push(Rule {
                            selector: nested_selector.clone(),
                            declarations: inner_decls,
                            media: inner_media,
                        });
                        nested_rules.append(&mut inner_nested);
                    }
                    continue;
                }

                let nested_prefix = format!("{}.{}", prefix, header);
                let (nested_block, mut nested_from_block) = parse_property_block(
                    &nested_prefix,
                    reader,
                    parent_selector,
                    parent_terms,
                    blocks,
                )?;
                decls.extend(nested_block);
                nested_rules.append(&mut nested_from_block);
                continue;
            }

            let fragment = fragment.trim_end_matches(';');
            if let Some((prop_part, value_part)) = fragment.split_once(':') {
                let property = format!("{}.{}", prefix, prop_part.trim());
                let value = value_part.trim().to_string();
                decls.push(Declaration {
                    property,
                    value,
                    span: Span::dummy(),
                });
            } else {
                return Err(format!("Invalid declaration line: '{}'", fragment));
            }
        }
    }

    Ok((decls, nested_rules))
}

fn collect_block_body<'a>(reader: &mut LineReader<'a>) -> Result<String, String> {
    let mut depth = 1;
    let mut body = String::new();

    while let Some((line, _)) = reader.next_line() {
        for ch in line.chars() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(body);
                    }
                }
                _ => {}
            }
        }
        body.push_str(line);
        body.push('\n');
    }

    Err("Unexpected end of nested block".to_string())
}

fn split_selector_terms(selector: &str) -> Vec<String> {
    selector
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn combine_selectors(parent_terms: &[String], header: &str) -> Vec<String> {
    let child_terms = split_selector_terms(header);
    let mut results = Vec::new();

    for parent in parent_terms {
        for child in &child_terms {
            if parent.is_empty() || child.contains('&') {
                results.push(child.replace('&', parent));
            } else {
                let trimmed = child.trim();
                if trimmed.is_empty() {
                    results.push(parent.clone());
                } else {
                    results.push(format!("{} {}", parent, trimmed));
                }
            }
        }
    }

    results
}

fn is_property_block_header(header: &str) -> bool {
    matches!(header, "border" | "flex" | "grid" | "radius")
}

fn is_selector_header(header: &str) -> bool {
    let trimmed = header.trim();
    trimmed.contains('&')
        || trimmed.contains('.')
        || trimmed.contains('#')
        || trimmed.contains(':')
        || trimmed.contains('[')
        || trimmed.contains('>')
        || trimmed.contains('+')
        || trimmed.contains('~')
        || trimmed.contains('*')
}

fn normalize_braces(input: &str) -> String {
    let mut out = String::new();
    let mut in_string = false;

    for ch in input.chars() {
        if ch == '"' {
            in_string = !in_string;
            out.push(ch);
            continue;
        }

        if in_string {
            out.push(ch);
            continue;
        }

        match ch {
            '{' => {
                out.push('{');
                out.push('\n');
            }
            '}' => {
                out.push('\n');
                out.push('}');
                out.push('\n');
            }
            _ => out.push(ch),
        }
    }

    out
}

struct LineReader<'a> {
    lines: Vec<&'a str>,
    idx: usize,
}

impl<'a> LineReader<'a> {
    fn new(body: &'a str) -> Self {
        Self {
            lines: body.lines().collect(),
            idx: 0,
        }
    }

    fn next_line(&mut self) -> Option<(&'a str, usize)> {
        if self.idx >= self.lines.len() {
            None
        } else {
            let line = self.lines[self.idx];
            self.idx += 1;
            Some((line, self.idx))
        }
    }
}

fn parse_declarations(body: &str) -> Result<Vec<Declaration>, String> {
    let mut decls = Vec::new();
    for raw_line in body.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with("/*") {
            continue;
        }
        let line = line.trim_end_matches(';');
        if let Some((prop_part, value_part)) = line.split_once(':') {
            let property = prop_part.trim().to_string();
            let value = value_part.trim().to_string();
            decls.push(Declaration {
                property,
                value,
                span: Span::dummy(),
            });
        } else {
            return Err(format!("Invalid declaration line: '{}'", line));
        }
    }
    Ok(decls)
}

fn build_preset_rule(presets: &[String]) -> Option<Rule> {
    if presets.iter().any(|p| p == "no-base") {
        return None;
    }

    let mut declarations: Vec<Declaration> = Vec::new();
    for preset in presets {
        merge_declarations(&mut declarations, preset_declaration_entries(preset));
    }

    if declarations.is_empty() {
        return None;
    }

    Some(Rule {
        selector: ":root".to_string(),
        declarations,
        media: Vec::new(),
    })
}

fn build_dark_preset_rule(presets: &[String]) -> Option<Rule> {
    if presets.is_empty() {
        return None;
    }

    let mut declarations: Vec<Declaration> = Vec::new();
    for preset in presets {
        merge_declarations(&mut declarations, preset_declaration_entries(preset));
    }

    if declarations.is_empty() {
        return None;
    }

    Some(Rule {
        selector: ":root".to_string(),
        declarations: Vec::new(),
        media: vec![MediaBlock {
            query: "(prefers-color-scheme: dark)".to_string(),
            declarations,
        }],
    })
}

fn build_media_preset_rules(media_presets: &[MediaPresetSpec]) -> Vec<Rule> {
    let mut rules = Vec::new();

    for preset in media_presets {
        let entries = preset_declaration_entries(&preset.name);
        if entries.is_empty() {
            continue;
        }

        let mut declarations: Vec<Declaration> = Vec::new();
        merge_declarations(&mut declarations, entries);

        rules.push(Rule {
            selector: preset.selector.to_string(),
            declarations: Vec::new(),
            media: vec![MediaBlock {
                query: preset.query.to_string(),
                declarations,
            }],
        });
    }

    rules
}

fn merge_declarations(target: &mut Vec<Declaration>, entries: Vec<(String, String)>) {
    for (prop, val) in entries {
        if let Some(existing) = target.iter_mut().find(|d| d.property == prop) {
            existing.value = val;
        } else {
            target.push(Declaration {
                property: prop,
                value: val,
                span: Span::dummy(),
            });
        }
    }
}

fn preset_declaration_entries(name: &str) -> Vec<(String, String)> {
    match name {
        "base-14" => preset_pairs(&[("font-size", "14px")]),
        "base-16" => preset_pairs(&[
            ("font-size", "16px"),
            ("line-height", "1.5"),
            ("font-family", "sans-serif"),
        ]),
        "base-18" => preset_pairs(&[
            ("font-size", "18px"),
            ("line-height", "1.55"),
            ("font-family", "sans-serif"),
        ]),
        "spacious" => preset_pairs(&[
            ("font-size", "16px"),
            ("line-height", "1.7"),
            ("letter-spacing", "0.01em"),
        ]),
        "reading" => preset_pairs(&[
            ("line-height", "1.75"),
            ("max-width", "65ch"),
            ("font-weight", "400"),
        ]),
        "compact" => preset_pairs(&[("line-height", "1.4"), ("letter-spacing", "0")]),
        "system" => preset_pairs(&[(
            "font-family",
            "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, Helvetica, Arial, sans-serif",
        )]),
        "fluid-type" => preset_pairs(&[("font-size", "clamp(14px, 2.2vw, 18px)")]),
        "light-ui" => preset_pairs(&[
            ("--background", "#ffffff"),
            ("--foreground", "#111111"),
            ("--muted", "#f3f3f3"),
            ("--border", "#e5e5e5"),
        ]),
        "dark-ui" => preset_pairs(&[
            ("--background", "#0f0f0f"),
            ("--foreground", "#fafafa"),
            ("--muted", "#1b1b1b"),
            ("--border", "#2a2a2a"),
        ]),
        "smooth" => preset_pairs(&[
            ("--ease", "cubic-bezier(0.4, 0.0, 0.2, 1)"),
            ("--duration", "150ms"),
        ]),
        "snappy" => preset_pairs(&[
            ("--ease", "cubic-bezier(0.2, 0.0, 0.0, 1)"),
            ("--duration", "100ms"),
        ]),
        "reduced-motion" => preset_pairs(&[
            ("animation", "none !important"),
            ("transition", "none !important"),
        ]),
        "code" => preset_pairs(&[
            (
                "font-family",
                "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, \"Liberation Mono\", \"Courier New\", monospace",
            ),
            ("font-size", "14px"),
            ("line-height", "1.5"),
        ]),
        "accessible-lg" => preset_pairs(&[
            ("font-size", "20px"),
            ("line-height", "1.7"),
            ("letter-spacing", "0.01em"),
        ]),
        "print" => preset_pairs(&[
            ("font-size", "12px"),
            ("line-height", "1.4"),
            ("color", "#000"),
            ("background-color", "#fff"),
        ]),
        _ => Vec::new(),
    }
}

fn preset_pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|(p, v)| (p.to_string(), v.to_string()))
        .collect()
}

fn preset_scope_from_name(name: &str) -> PresetScope {
    if name == "reduced-motion" {
        return PresetScope::Media {
            query: "(prefers-reduced-motion: reduce)",
            selector: "*",
        };
    }
    if name.starts_with("dark-") {
        return PresetScope::Dark;
    }
    PresetScope::Root
}
