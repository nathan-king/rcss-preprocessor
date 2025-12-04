use rss_core::Theme;
use rss_core::parser;
use rss_core::resolver;

fn main() {
    let theme = Theme::load("theme/tokens.json").expect("failed to load theme");

    let input = r#"
    .button {
        padding: @4;
        color: @blue-500;
    }
    "#;

    let ast = parser::parse(input).expect("parse failed");

    let resolved = resolver::resolve(ast, &theme).expect("resolve failed");

    println!("{:#?}", resolved);
}
