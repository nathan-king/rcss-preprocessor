# RCSS — Rusty Cascading Style Sheets

RCSS is a token-first CSS preprocessor written in Rust. Rather than reinvent selectors and mixins, it layers a small declarative grammar on top of real CSS with the following design goals:

1. **Resolve design tokens before anything else** — colors, spacing, grids, typography, and motion values come from JSON theme files so the syntax stays expressive but deterministic.
2. **Emit honest CSS** — no runtime helpers or custom selectors. The parser expands new shorthands/blocks into ordinary declarations, then autoprefixes critical properties before writing the result.
3. **Blend style systems** — RCSS mixes Tailwind-like tokens, preset-based theming, block-style helpers (`border { ... }`, `flex { ... }`, `radius { ... }`, `grid { ... }`), and reusable foundation classes in a single file while staying deterministic.

The repo ships a demo (`demo.rcss`) that exercises every supported feature and an auto-generated `demo.css`.

---

## Core syntax overview

### Tokens & variables

- Tokens look like `@blue-500`, `@lg`, arithmetic-ready numbers like `@4`, semantic keywords such as `@sans` or `@spin`, and media/URL shortcuts such as `screen(@md)` or `@(/img/pic.png)`.
- Tokens may now be placed anywhere inside a declaration value—`calc(100% - @4)`, `linear-gradient(to right, @red-500, @blue-400/50)`, or even `@(images/@blue-500.svg)` have their `@` references replaced at compile time, and malformed placements like `0px@4` raise errors with line/column spans.
- Variables are defined with `$name: value;` and may be interpolated literally later via `$name` in other declarations (act as raw string replacements).
- Block mixins exist via `$card { ... }` + `apply: $card;`.

### Presets

- Include `%base-16`, `%base-18`, `%system`, `%reading`, `%fluid-type`, `%light-ui`, `%dark-ui`, `%smooth`, `%snappy`, and `%reduced-motion`. `:%base-16` runs by default unless you set `%no-base`.
- `%dark ...` is shorthand for wrapping declarations inside the `dark { ... }` media block. You can combine multiple presets on the same line.

### Block-style helpers

RCSS introduces nested property blocks that expand to multiple CSS rules:

- `border { color: ...; width: ...; style: ...; radius { ... } }` ⇒ `border-color`, `border-width`, `border-style`, and all four `border-*-*-radius` longhands. Radius blocks accept `all`, logical groups (`inline`, `block`), and per-corner overrides (`top-left`, `bottom-right`, etc.).
- `radius { ... }` can be used standalone to target only radii.
- `flex { ... }` exposes `direction`, `wrap`, `justify`, `align`, `content`, and `gap`, automatically inserting `display: flex`.
- `grid { ... }` mirrors the inline `grid:` shorthand (see below) for readability.
- Nested selectors (e.g., `&:hover`, `& .child`, `.icon`, `&-primary` or comma-separated lists) may now live inside any declaration block—including property helpers like `border`, `flex`, `grid`, and `radius`—and are emitted by combining with the current selector context at every nesting level.

### Imports
- Use `@import "./partials/base.rcss";` (CSS-style) to inline other RCSS files before parsing; relative paths resolve from the importing file.
- Imports are deduplicated, cycle-checked, and merged so variables/mixins/presets flow through the combined stylesheet.

### Shorthands and functions

- `shadow`, `ring`, `ring-color`, `ring-offset-width`, `transform`, `filter`, `gradient`, `font`, and others are defined in `theme/shorthands.json`.
- RCSS now understands token-aware color helpers: `mix()`, `lighten()`, `darken()`, `alpha()`, `shade()`, `tint()`, and `tone()`. Tokens inside these functions resolve before building `color-mix()`/`rgba()` expressions.
- Autoprefixer rules cover transforms, filters/backdrop filters, `appearance`, gradients, flex display values, and grab cursors. Prefixed declarations are emitted before the unprefixed version.

### Grid shorthand

- `grid:` accepts subcommands `cols()`, `rows()`, `gap()`, `areas()`, `masonry`, and `columns()`. Each argument can be raw CSS or tokens; tokens resolve before validation.
- `cols(@3)` → `grid-template-columns: repeat(3, minmax(0, 1fr))`.
- `rows(@lg auto 1fr)` resolves tokens individually before emitting `grid-template-rows`.
- `gap(@4)` emits `gap`; two values emit `row-gap`/`column-gap`.
- `areas("header header" "sidebar main")` keeps the quoted strings on separate lines.
- `grid: masonry columns(4) gap(@2)` emits a multi-column layout with `display: block`, `column-count`, and `column-gap`.
- Mixing masonry and grid subcommands in the same declaration raises a compile-time error.

### Media sugar

- `screen(@md) { ... }` wraps the block inside an `@media (min-width: ...)` query.
- `dark { ... }` and `light { ... }` expand to `@media (prefers-color-scheme: dark)`/`light`. Raw `@media ...` blocks pass through unchanged.

### CLI entry point

- Build a file with `cargo run -p rcss-cli -- build input.rcss` (output defaults to `input.css`). Use `-o` to override.
- The CLI always loads the theme from the `theme/` directory (see below) and applies parser/resolver/emitter phases.

---

## Demo reference

`demo.rcss` is structured as category sections:

- An `import.rcss` file defines reusable typography helpers; the demo shows how `import "./import.rcss";` inlines that helper before the rest of the stylesheet.

- Foundations: `.foundation-spacing`, `.foundation-typography`, and `.foundation-card` show tokens, backgrounds, and shared mixins.
- Grid systems: `.grid-inline`, `.grid-block`, `.grid-areas`, `.grid-masonry` cover inline shorthand, blocks, named areas, and masonry fallbacks.
- Border/radius: `.border-outline`, `.border-detail`, `.radius-only` highlight the new block syntax plus logical radius helpers.
- Color/effects: `.color-func-demo`, `.gradient-demo`, `.token-mix` exercise color functions, gradient helper, token math, and `border { ... }`.
- Autoprefixed effects: `.prefixed-effects` demonstrates the autoprefix layer (transforms, filters, appearance, cursor, box-shadow).
- Flex: `.flex-demo` uses the flex block and verifies auto `display: flex`.

The generated CSS lives in `demo.css` and can be inspected or served as a visual gallery after you run the CLI.

---

## Theme files

- `theme/tokens.json` defines token collections (colors, spacing, typography, etc.) and per-property mappings. It’s generated from Tailwind equivalents and is imported by `Theme::load`.
- `theme/shorthands.json` defines multi-step shorthands (`shadow`, `ring`, `transform`, etc.) with templates for token interpolation and optional ordering.
- `theme/presets.json` documents presets; `parser.rs` hardcodes the core ones but you can reference this file for future expansion.

If you need to extend tokens/shorthands, edit the JSON files, then rebuild via the CLI.

---

## Development notes

- Core crates:
  - `crates/rcss-core/` handles parsing (including nested blocks), AST generation, token resolution, shorthand expansion, grid parsing, color helpers, and the autoprefixing emitter.
  - `crates/rcss-cli/` wires the theme load + CLI arguments + file I/O.
- Run `cargo fmt` after code changes and `cargo test` if you add logic branches.
- The CLI lacks watch mode or configurable paths; it strictly reads from `theme/` and writes the CSS next to the RCSS input unless `-o` is set.

---

## Known limitations

1. Token resolution expects the entire value to either be a token or a mixin; complex inline expressions require explicit functions.
2. Variables (`$foo`) are simple string replaces with no scoping or parameterization.
3. Parser blocks don’t track spans, so errors reference only line numbers in panic messages.
4. Autoprefixing covers curated categories; other CSS properties emit only standard declarations.

This project is suitable for demos and prototypes. Expect breaking changes as the grammar and themes evolve.

See `vscode-rcss/README.md` for editor-specific instructions and the VS Code extension workflow.
