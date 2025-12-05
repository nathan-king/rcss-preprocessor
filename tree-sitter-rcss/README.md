# Tree-sitter RCSS: VS Code highlighting

This repo already ships the RCSS grammar. To get syntax highlighting in VS Code, add highlight queries and load the grammar via the community “Tree-sitter” extension.

## Files added
- `queries/highlights.scm` — basic scopes for selectors, properties, tokens (`@foo`), variables (`$foo`), presets (`%base-16`), functions, numbers, etc.

## Build the WebAssembly parser
```bash
cd tree-sitter-rcss
npm install
npx tree-sitter build-wasm   # produces ./tree-sitter-rcss.wasm
```

## Hook into VS Code
1) Install the VS Code extension “Tree-sitter” by George Fraser (marketplace id: `georgewfraser.vscode-tree-sitter`).  
2) Add this to your VS Code settings (User or Workspace):
```json
{
  "tree-sitter.languages": {
    "rcss": {
      "wasm": "${workspaceFolder}/tree-sitter-rcss/tree-sitter-rcss.wasm",
      "queries": "${workspaceFolder}/tree-sitter-rcss/queries"
    }
  },
  "files.associations": {
    "*.rcss": "rcss"
  }
}
```

That’s it: open a `.rcss` file and the Tree-sitter extension will load the wasm + queries for highlighting. If you change the grammar, rerun `npx tree-sitter build-wasm`.
