use crate::ast::Stylesheet;
use crate::theme::Theme;

pub fn resolve(mut stylesheet: Stylesheet, theme: &Theme) -> Result<Stylesheet, String> {
    for rule in &mut stylesheet {
        for decl in &mut rule.declarations {
            if let Some(token) = decl.value.strip_prefix('@') {
                let resolved = resolve_token(&decl.property, token, theme)?;
                decl.value = resolved;
            }
        }
    }
    Ok(stylesheet)
}

fn resolve_token(property: &str, token: &str, theme: &Theme) -> Result<String, String> {
    if is_spacing_property(property) {
        return resolve_spacing(token, theme);
    }

    if is_color_property(property) {
        return resolve_color(token, theme);
    }

    Err(format!("Don't know how to resolve token: {}", token))
}

fn is_spacing_property(property: &str) -> bool {
    property == "padding"
        || property.starts_with("padding-")
        || property == "margin"
        || property.starts_with("margin-")
        || property == "gap"
        || property == "row-gap"
        || property == "column-gap"
}

fn resolve_spacing(token: &str, theme: &Theme) -> Result<String, String> {
    if let Some(value) = theme.spacing.get(token) {
        Ok(value.to_string())
    } else {
        Err(format!("Unknown spacing token: {}", token))
    }
}

fn is_color_property(property: &str) -> bool {
    property == "background-color" || property == "color" || property.ends_with("-color")
}

fn resolve_color(token: &str, theme: &Theme) -> Result<String, String> {
    let (family, shade) = token
        .split_once('-')
        .ok_or_else(|| format!("Color token '@{}' must be in the form 'name-shade'", token))?;

    let family_map = theme
        .colors
        .get(family)
        .ok_or_else(|| format!("Unknown color family '@{}'", family))?;

    let value = family_map
        .get(shade)
        .ok_or_else(|| format!("Unknown color shade '{}-{}'", family, shade))?;

    Ok(value.clone())
}
