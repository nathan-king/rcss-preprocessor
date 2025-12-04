use rcss_core::{emitter, parser, resolver, theme::Theme};

fn main() {
    // Load theme
    let theme = Theme::load("theme/tokens.json").expect("Failed to load theme");

    // Load embedded demo stylesheet
    let src = include_str!("demo.rcss");

    // Parse
    let stylesheet = parser::parse(src).expect("Failed to parse RCSS");

    // Resolve
    let stylesheet = resolver::resolve(stylesheet, &theme).expect("Failed to resolve tokens");

    // Emit CSS
    let css = emitter::emit_css(&stylesheet);

    println!("{}", css);
}
