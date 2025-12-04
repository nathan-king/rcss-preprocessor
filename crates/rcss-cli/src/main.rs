mod cli;

use clap::Parser;
use cli::{Cli, Commands};

use std::fs;

use rcss_core::{emitter, parser, resolver, theme::Theme};

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Build { input, output } => {
            run_build(&input, output);
        }
    }
}

fn run_build(input_path: &str, output_override: Option<String>) {
    // Determine output path
    let output_path = match output_override {
        Some(custom) => custom,
        None => auto_output_name(input_path),
    };

    let theme = Theme::load_from_dir("theme").expect("Failed to load theme");
    let src = fs::read_to_string(input_path).expect("Failed to read input RCSS file");

    let stylesheet = parser::parse(&src).expect("Failed to parse RCSS");

    let stylesheet = resolver::resolve(stylesheet, &theme).expect("Failed to resolve tokens");

    let css = emitter::emit_css(&stylesheet);

    fs::write(&output_path, css).expect("Failed to write CSS output");

    println!("✓ Built {} → {}", input_path, output_path);
}

fn auto_output_name(input: &str) -> String {
    if let Some(stripped) = input.strip_suffix(".rcss") {
        format!("{}.css", stripped)
    } else {
        format!("{}.css", input)
    }
}
