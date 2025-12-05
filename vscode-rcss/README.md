# RCSS VS Code syntax (TextMate)

A minimal TextMate grammar for RCSS that now mirrors CSS scope names so it inherits whatever VS Code color theme you’re using.

## Build RCSS on save

The extension can run the RCSS compiler whenever you save an `.rcss` file (and exposes a command to run it manually).

- Default build command: `cargo run -p rcss-cli -- build ${file}` (requires Rust toolchain in your PATH). `${file}` is replaced with the saved file’s absolute path.
- Settings (search for “RCSS”):
  - `rcss.buildOnSave` — toggle auto-build (default: true).
  - `rcss.buildCommand` — override the command if you prefer a pre-built binary (e.g., `./target/debug/rcss build ${file}`).
  - `rcss.buildCwd` — optional working directory; falls back to the workspace folder containing the file.
- Manual command: “RCSS: Build Current File”.

## Install locally
1) From this repo root, create a VSIX (requires VS Code’s packaging tool):
   ```bash
   cd vscode-rcss
   pnpm dlx @vscode/vsce package   # or: npx @vscode/vsce package
   ```
   This produces `rcss-syntax-0.0.3.vsix`.
2) In VS Code, run “Extensions: Install from VSIX…” and pick that file.

## What it highlights
- Comments, presets (`%base-16`), variables (`$foo`), tokens (`@blue-500`, `@(path)`).
- Properties, selectors, shorthand keywords (`apply`, `screen`, `dark`, `light`).

Grammar source: `syntaxes/rcss.tmLanguage.json`. Language config (comments, brackets) lives in `language-configuration.json`.

## Optional coloring suggestions
Add this to your VS Code settings to tune colors and remove italics on properties:
```json
{
  "editor.tokenColorCustomizations": {
    "textMateRules": [
      { "scope": "support.type.property-name.css.rcss", "settings": { "fontStyle": "" } },
      { "scope": "variable.other.readwrite.rcss", "settings": { "foreground": "#f9c513" } },
      { "scope": "punctuation.definition.variable.rcss", "settings": { "foreground": "#b58900" } },
      { "scope": "constant.other.rcss", "settings": { "foreground": "#66d9ef" } },
      { "scope": "punctuation.separator.constant.rcss", "settings": { "foreground": "#8ec7ff" } },
      { "scope": "punctuation.definition.constant.rcss", "settings": { "foreground": "#2aa198" } }
    ]
  }
}
```
Tweak the hex colors to match your theme (use darker shades for dark themes, lighter for light themes). Tokens and prefixes are scoped separately so you can downplay the `@` while keeping token names bright; same for `$` prefixes on variables.

Alternatively, select the bundled color theme “RCSS Accent” (Command Palette → Preferences: Color Theme → RCSS Accent) to apply these overrides automatically, including removing italics on properties.
