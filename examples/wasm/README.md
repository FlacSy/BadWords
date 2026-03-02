# BadWords WASM Examples

WebAssembly build for JavaScript/TypeScript — browser and Node.js.

## Build

From project root:

```bash
# Browser (ES modules, async init)
make wasm

# Node.js (CommonJS, sync)
make wasm-nodejs
```

Requires: `cargo install wasm-pack`

## Browser

```bash
make wasm
# Option 1: from project root
npx serve . -p 3000

# Option 2: from example folder
cd examples/wasm/browser && npm start
```

Open: **http://localhost:3000/examples/wasm/browser/**

## Node.js

```bash
make wasm-nodejs
node examples/wasm/node/index.js
```

## Node.js (TypeScript)

```bash
make wasm-nodejs
npx tsx examples/wasm/node/index.ts
```
