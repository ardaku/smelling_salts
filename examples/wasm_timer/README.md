# Getting Started With Smelling Salts For Web Assembly

## Install wasm32-unknown-unknown
Run the following command to start:
```bash
rustup install wasm32-unknown-unknown
```

## Testing
This example was designed to be able to be used with any of:
 - wasm-pack
 - cargo-web
 - cargo-cala

In order to make smelling salts work without either of stdweb or wasm\_bindgen,
this means that extra files are necessary depending on what you're using:

### `stdweb`
- `Web.toml`
- `src/static/cala.js` - 
- `src/static/cala.wasm` - 
- `src/static/index.html` - 

### `wasm-pack`
- `src/static/cala.js` - 
- `src/static/cala.wasm` - 
- `src/static/index.html` - 

### `cargo-cala`
- `Cala.muon` - 
