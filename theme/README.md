# Theme tokens format

This folder contains a deduped token manifest in `tokens.json`. The goal is to avoid repeating identical token sets across CSS properties (e.g., `backgroundColor`, `textColor`, `borderColor` all share the same color palette) while still recording which properties consume which tokens.

## File layout

`tokens.json` has two top-level keys:

- `collections`: unique token sets. Each entry is the canonical source for a group of token values (e.g., `colors`, `spacing`, `opacity`, `fontSize`, etc.).
- `properties`: a mapping of CSS-property-like keys to the collection they use. Example:
  ```json
  {
    "backgroundColor": { "collection": "colors" },
    "textColor": { "collection": "colors" },
    "opacity": { "collection": "opacity" }
  }
  ```

`shorthands.json` describes multi-property expansions (e.g., `shadow`, `ring`, `transform`, `filter`, `gradient`). Each shorthand is an array of steps with:

- `property`: the CSS property to emit
- `template`: a string with `@{placeholder}` slots
- `append` (optional): if `true`, append the rendered value to an existing property value (comma-join for `box-shadow`)
- `optional` (optional): if placeholders are missing, skip the step instead of erroring

Shorthand usage in RCSS:

```css
.card {
    shadow: @lg;
    ring: ringWidth=@2 ringColor=@red-500/50 ringOffsetWidth=@1 ringOffsetColor=@blue-200/50;
    transform: translate=@4 rotate=@45 skew=@6 scale=@95;
    filter: blur=@sm brightness=@90 contrast=@125 hueRotate=@15 saturate=@150;
    gradient: from=@blue-500 via=@blue-300 to=@white;
}
```

The placeholders map to properties/collections by name (with aliases like `from`/`via`/`to` → `gradientColorStops`, `shadow` → `boxShadow`). Values are tokens prefixed with `@` (for a single token) or `name=@token` assignments for multi-slot shorthands.

## How deduping works

- Start from the Tailwind-resolved theme (`tailwind-theme-resolved.json`).
- Identity entries are removed (where a token name exactly equals its value, e.g., `"auto": "auto"`).
- Properties that have identical token maps are grouped into a single collection.
- Each property points to its collection via `properties.<prop>.collection`.

## Overriding per-property differences

If two properties mostly share tokens but need different values for a subset, add an `overrides` block under that property:

```json
"fontWeight": {
  "collection": "colors",
  "overrides": { "black": "900" }
}
```

During resolution: pick the collection for the property, then apply any `overrides` by key.

## Notes

- Collection names are chosen heuristically for readability; the authoritative mapping is in the `properties` section.
- The current `tokens.json` is generated from `tailwind-theme-resolved.json` with the rules above. Regenerate the file with the same process if you update the source theme.
