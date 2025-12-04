# RCSS — Rusty Style Sheets

RCSS is a strict, token-driven CSS preprocessor written in Rust.
It provides Tailwind-compatible design tokens, a clean nested rule syntax,
and compile-time guarantees that ensure consistent, predictable styling.

RCSS compiles `.rcss` files into plain, valid CSS with no runtime overhead.

```css
.button {
    padding: @4;
    color: @blue-500;

    &:hover {
        color: @blue-700;
    }
}
```

Compiles to:

```css
.button {
    padding: 1rem;
    color: #3b82f6;
}
.button:hover {
    color: #1d4ed8;
}
```

---

## Features

### Token-first syntax
RCSS enforces a strict design system:

- spacing tokens like `@4`, `@6`
- color tokens like `@blue-500`
- future support for typography, radii, shadows, transitions, etc.

Tokens come from a JSON theme file (Tailwind-compatible).

### Nested rules
Write CSS in a structured and readable way:

```rcss
.card {
    padding: @6;

    .title {
        font-size: @md;
    }

    &:hover {
        background-color: @gray-100;
    }
}
```

### Designed for design systems
RCSS is intended for teams who want:

- strict mode (tokens required for design-system-controlled properties)
- predictable output
- no arbitrary values unless explicitly allowed

### Rust-powered speed
The parser, resolver, and emitter are implemented in Rust.
No Node.js is required to compile stylesheets.

### Portable and framework-agnostic
RCSS works with any front-end or back-end framework, including:

- Next.js
- React
- Svelte
- Astro
- Leptos, Yew, and other Rust frameworks
- Traditional HTML/CSS projects

---

## Project Structure

```
crates/
  rcss-core/   # parser, AST, token resolver, emitter
  rcss-cli/    # command-line interface for running the compiler
theme/
  tokens.json  # design tokens (spacing, colors, sizes, etc.)
```

---

## Usage

RCSS is not yet published on crates.io. Expect this to change as the project stabilises.

### Run the CLI:

```sh
cargo run -p rcss-cli
```

By default, the CLI loads:

- `theme/tokens.json`
- a sample `.rcss` source defined in the CLI (temporary during development)

In a future release:

```sh
rcss build styles.rcss -o styles.css
```

---

## Theme File

The theme file is a Tailwind-compatible JSON snapshot.
Example (`theme/tokens.json`):

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

---

## Philosophy

RCSS exists because:

- SCSS allows arbitrary styling without design-system enforcement
- Tailwind enforces design systems but is class-based
- CSS custom properties alone do not enforce token usage
- Design systems benefit from a dedicated, predictable authoring syntax
- Rust enables fast, safe, and reliable compilation

RCSS is not intended to replace Tailwind.
It is a design-system-first authoring language that compiles to clean CSS.

---

## Roadmap

### v0.1 — Core prototype (current)
- Parser
- Design token resolver
- AST representation
- Basic CLI
- Theme ingestion

### v0.2
- CSS emitter (real file output)
- CLI flags
- File watching

### v0.3
- Nested selectors
- Responsive blocks (`@md`, `@sm`, `@lg`)
- User-level variables (`$var`)

### v0.4
- Strict mode (tokens required for design-system properties)
- Allow mode for migration (accepts raw CSS values)

### v0.5
- VS Code syntax highlighting
- `.rcss` language extension

### v1.0
- Stable grammar
- Full documentation
- Plugin system

---

## License

MIT License. See `LICENSE` for details.

---

## Contributing

RCSS is in early development and open to contributions.
Detailed contribution guidelines will be added soon.

---

## Acknowledgements

RCSS is influenced by:

- Tailwind CSS design tokens
- SCSS syntax
- Lightning CSS
- The Rust ecosystem

---

## Status

RCSS is experimental but functional.
The parser and design token resolver are implemented.
Repository: https://github.com/nathan-king/rcss-preprocessor
