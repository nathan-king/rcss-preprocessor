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

    if let Some((color_token, opacity_token)) = token.split_once('/') {
        return resolve_color_with_opacity(property, color_token, opacity_token, theme);
    }

    if is_color_property(property) {
        return resolve_color(token, theme);
    }

    if is_font_size_property(property) {
        return resolve_font_size(token, theme);
    }

    if property == "opacity" {
        return resolve_global_opacity(token, theme);
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

fn is_font_size_property(property: &str) -> bool {
    property == "font-size"
}

fn resolve_font_size(token: &str, theme: &Theme) -> Result<String, String> {
    if let Some(entry) = theme.font_size.get(token) {
        Ok(entry.size.clone())
    } else {
        Err(format!("Unknown font size token: {}", token))
    }
}

fn resolve_color_with_opacity(
    _property: &str, //to support opacity behaviour based on property
    color_token: &str,
    opacity_token: &str,
    theme: &Theme,
) -> Result<String, String> {
    let color = resolve_color(color_token, theme)?;

    let opacity_key = opacity_token.trim_start_matches('@');
    let opacity = theme
        .opacity
        .get(opacity_key)
        .ok_or_else(|| format!("Unknown opacity token '@{}'", opacity_key))?;

    inject_alpha(color, opacity)
}

fn inject_alpha(color: String, opacity: &str) -> Result<String, String> {
    if color.starts_with("oklch(") {
        let base = color.trim_end_matches(')');
        return Ok(format!("{} / {}", base, opacity));
    }

    if color.starts_with('#') {
        return hex_to_rgba(&color, opacity);
    }

    if color.contains('(') {
        let base = color.trim_end_matches(')');
        return Ok(format!("{} / {}", base, opacity));
    }

    Err(format!("Unsupported color format: {}", color))
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

fn resolve_global_opacity(token: &str, theme: &Theme) -> Result<String, String> {
    if token.parse::<f32>().is_ok() {
        return Ok(token.to_string());
    }

    let key = token.trim_start_matches('@');

    theme
        .opacity
        .get(key)
        .cloned() // <— required to convert &String → String
        .ok_or_else(|| format!("Unknown opacity token: {}", key))
}
