use crate::ast::{Declaration, Rule, Stylesheet};

pub fn parse(input: &str) -> Result<Stylesheet, String> {
    let mut rules = Vec::new();

    let chunks = input.split('}');

    for chunk in chunks {
        let chunk = chunk.trim();
        if chunk.is_empty() || !chunk.contains('{') {
            continue;
        }

        let (selector_part, body_part) =
            chunk.split_once('{').ok_or("Invalid syntax: missing '{'")?;

        let selector = selector_part.trim().to_string();
        let mut declarations = Vec::new();

        for line in body_part.lines() {
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            let line = line.trim_end_matches(';');

            if let Some((prop_part, value_part)) = line.split_once(':') {
                let property = prop_part.trim().to_string();
                let value = value_part.trim().to_string();

                declarations.push(Declaration { property, value });
            } else {
                return Err(format!("Invalid declaration line: '{}'", line));
            }
        }

        rules.push(Rule {
            selector,
            declarations,
        });
    }
    Ok(rules)
}
