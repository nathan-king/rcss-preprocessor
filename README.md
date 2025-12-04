scalecss
========

Generate the Tailwind default design system snapshot (JSON) from the installed package:

```sh
node - <<'NODE'
const fs = require('fs');
const defaultTheme = require('tailwindcss/defaultTheme');

function resolveTheme(value, themeFn) {
  if (Array.isArray(value)) return value.map(v => resolveTheme(v, themeFn));
  if (value && typeof value === 'object') {
    const out = {};
    for (const k of Object.keys(value)) {
      if (k === '__BARE_VALUE__') continue; // skip dynamic placeholders
      out[k] = resolveTheme(value[k], themeFn);
    }
    return out;
  }
  if (typeof value === 'function') return resolveTheme(value({ theme: themeFn }), themeFn);
  return value;
}

function buildResolved(theme) {
  const cache = {};
  const themeFn = key => {
    if (cache[key] !== undefined) return cache[key];
    return (cache[key] = resolveTheme(theme[key], themeFn));
  };
  return resolveTheme(theme, themeFn);
}

const resolvedTheme = buildResolved(defaultTheme);
fs.writeFileSync('tailwind-theme-resolved.json', JSON.stringify(resolvedTheme, null, 2));
console.log('wrote tailwind-theme-resolved.json');
NODE
```

This writes `tailwind-theme-resolved.json` with all Tailwind default tokens fully expanded (colors, spacing, radii, shadows, font settings, breakpoints, etc.), ready for Rust ingestion. Re-run after upgrading Tailwind to keep the snapshot current.
