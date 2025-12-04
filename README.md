# RCSS — Rusty Cascading Style Sheets

RCSS is an experimental CSS preprocessor written in Rust.  
It's very early in development, and the goal right now is simple:  
parse a stylesheet, resolve Tailwind‑style design tokens, and output clean CSS.

This project is not stable, not feature‑complete, and not ready for production.  
But it works, and it will grow.

---

## What RCSS does today

- Loads a JSON theme containing spacing, colors, and font sizes  
- Parses a small subset of CSS‑like syntax  
- Replaces token values (e.g., `@4`, `@blue-500`) with real CSS values  
- Prints a resolved stylesheet to stdout (temporary behaviour)

Example input:

```css
.button {
    padding: @4;
    color: @blue-500;
}
```

Example output:

```css
.button {
    padding: 1rem;
    color: #3b82f6;
}
```

This is just the beginning.

---

## Project layout

```
crates/
  rcss-core/   # parser, AST, resolver
  rcss-cli/    # command-line interface
theme/
  tokens.json  # Tailwind-style design tokens
```

---

## Goals (short-term)

These are the next concrete aims:

1. Clean compiler pipeline  
2. Emit valid CSS to a user-specified output file  
3. Add nested selectors  
4. Add responsive blocks (`@sm`, `@md`, etc.)  
5. Add user-defined variables (`$something`)  
6. Add strict vs allow modes

---

## Goals (long-term)

- Plugin system  
- VS Code extension  
- Stable grammar  
- Full Tailwind-compatible token support  
- Zero-runtime integration for frameworks (Next.js, Astro, Leptos, etc.)

---

## Running the project

Right now everything is local. Run the CLI with:

```sh
cargo run -p rcss-cli
```

The CLI currently loads:

- `theme/tokens.json`
- a small hardcoded `.rcss` sample for demo purposes

Future versions will support:

```
rcss build styles.rcss -o styles.css
```

---

## Theme file format

RCSS expects a simple JSON theme.  
Example:

```json
{
  "spacing": { "4": "1rem", "6": "1.5rem" },
  "colors": {
    "blue": { "500": "#3b82f6" },
    "gray": { "700": "#374151" }
  },
  "font_size": {
    "sm": { "size": "0.875rem", "lineHeight": "1.25rem" }
  }
}
```

This format will expand over time.

---

## Philosophy (early draft)

The idea behind RCSS:

- enforce a design system by default  
- provide a nicer authoring syntax than Tailwind classes
- avoid classes hell
- stay strict and predictable, not permissive  
- avoid runtime CSS generation  
- use Rust for fast, deterministic builds  

RCSS is not a Tailwind replacement — it is a structured authoring layer built *on top* of a Tailwind‑style design system.

---

## Status

RCSS is experimental, minimal, and actively being built.

Repo: https://github.com/nathan-king/rcss-preprocessor

