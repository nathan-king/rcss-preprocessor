use crate::ast::{Declaration, Rule, Stylesheet};
use std::collections::HashMap;
use std::sync::OnceLock;

pub fn emit_css(stylesheet: &Stylesheet) -> String {
    let mut out = String::new();

    for rule in stylesheet {
        if !rule.declarations.is_empty() {
            emit_rule(rule, &mut out);
            out.push('\n');
        }
        for media in &rule.media {
            emit_media_rule(rule, media, &mut out);
            out.push('\n');
        }
    }
    out
}

fn emit_rule(rule: &Rule, out: &mut String) {
    out.push_str(&rule.selector);
    out.push_str(" {\n");

    for decl in &rule.declarations {
        emit_declaration(decl, out);
    }
    out.push_str("}\n");
}

fn emit_declaration(decl: &Declaration, out: &mut String) {
    let mut prefixed = false;
    if let Some(actions) = autoprefix_rules().get(decl.property.as_str()) {
        for action in actions {
            if (action.condition)(&decl.value) {
                emit_single_declaration(action.property, &(action.transform)(&decl.value), out);
                prefixed = true;
            }
        }
    }

    emit_single_declaration(&decl.property, &decl.value, out);

    if prefixed {
        out.push('\n');
    }
}

fn emit_single_declaration(property: &str, value: &str, out: &mut String) {
    out.push_str("    ");
    out.push_str(property);
    out.push_str(": ");
    out.push_str(value);
    out.push_str(";\n");
}

fn emit_media_rule(rule: &Rule, media: &crate::ast::MediaBlock, out: &mut String) {
    out.push_str("@media ");
    out.push_str(&media.query);
    out.push_str(" {\n");
    out.push_str("  ");
    out.push_str(&rule.selector);
    out.push_str(" {\n");
    for decl in &media.declarations {
        emit_declaration(decl, out);
    }
    out.push_str("  }\n");
    out.push_str("}\n");
}

fn autoprefix_rules() -> &'static HashMap<&'static str, Vec<PrefixAction>> {
    static RULES: OnceLock<HashMap<&'static str, Vec<PrefixAction>>> = OnceLock::new();
    RULES.get_or_init(build_autoprefix_map)
}

fn build_autoprefix_map() -> HashMap<&'static str, Vec<PrefixAction>> {
    let mut map: HashMap<&'static str, Vec<PrefixAction>> = HashMap::new();

    map.entry("border-radius").or_default().extend(vec![
        PrefixAction::new("-webkit-border-radius", always_true, identity),
        PrefixAction::new("-moz-border-radius", always_true, identity),
    ]);

    map.entry("box-shadow").or_default().push(PrefixAction::new(
        "-webkit-box-shadow",
        always_true,
        identity,
    ));

    map.entry("transform").or_default().extend(vec![
        PrefixAction::new("-webkit-transform", always_true, identity),
        PrefixAction::new("-ms-transform", always_true, identity),
    ]);

    map.entry("filter").or_default().push(PrefixAction::new(
        "-webkit-filter",
        always_true,
        identity,
    ));

    map.entry("backdrop-filter")
        .or_default()
        .push(PrefixAction::new(
            "-webkit-backdrop-filter",
            always_true,
            identity,
        ));

    map.entry("appearance").or_default().extend(vec![
        PrefixAction::new("-webkit-appearance", always_true, identity),
        PrefixAction::new("-moz-appearance", always_true, identity),
    ]);

    map.entry("background").or_default().push(PrefixAction::new(
        "background",
        gradient_condition,
        gradient_transform,
    ));

    map.entry("background-image")
        .or_default()
        .push(PrefixAction::new(
            "background-image",
            gradient_condition,
            gradient_transform,
        ));

    map.entry("display").or_default().extend(vec![
        PrefixAction::new("display", is_flex_display, flex_webkit_box),
        PrefixAction::new("display", is_flex_display, flex_ms_flexbox),
    ]);

    map.entry("cursor").or_default().push(PrefixAction::new(
        "cursor",
        is_grab_cursor,
        grab_cursor_prefix,
    ));

    map
}

struct PrefixAction {
    property: &'static str,
    condition: fn(&str) -> bool,
    transform: fn(&str) -> String,
}

impl PrefixAction {
    const fn new(
        property: &'static str,
        condition: fn(&str) -> bool,
        transform: fn(&str) -> String,
    ) -> Self {
        PrefixAction {
            property,
            condition,
            transform,
        }
    }
}

fn always_true(_value: &str) -> bool {
    true
}

fn identity(value: &str) -> String {
    value.to_string()
}

fn gradient_condition(value: &str) -> bool {
    let trimmed = value.trim_start();
    let lower = trimmed.to_ascii_lowercase();
    if trimmed.starts_with("-webkit-") {
        return false;
    }
    lower.starts_with("linear-gradient")
        || lower.starts_with("radial-gradient")
        || lower.starts_with("conic-gradient")
        || lower.starts_with("repeating-linear-gradient")
        || lower.starts_with("repeating-radial-gradient")
}

fn gradient_transform(value: &str) -> String {
    let trimmed = value.trim_start();
    format!("-webkit-{}", trimmed)
}

fn is_flex_display(value: &str) -> bool {
    value.trim() == "flex"
}

fn flex_webkit_box(_: &str) -> String {
    "-webkit-box".to_string()
}

fn flex_ms_flexbox(_: &str) -> String {
    "-ms-flexbox".to_string()
}

fn is_grab_cursor(value: &str) -> bool {
    matches!(value.trim(), "grab" | "grabbing")
}

fn grab_cursor_prefix(value: &str) -> String {
    format!("-webkit-{}", value.trim())
}
