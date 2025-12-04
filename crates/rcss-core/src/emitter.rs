use crate::ast::{Declaration, Rule, Stylesheet};

pub fn emit_css(stylesheet: &Stylesheet) -> String {
    let mut out = String::new();

    for rule in stylesheet {
        emit_rule(rule, &mut out);
        out.push('\n');
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
    out.push_str("    ");
    out.push_str(&decl.property);
    out.push_str(": ");
    out.push_str(&decl.value);
    out.push_str(";\n");
}
