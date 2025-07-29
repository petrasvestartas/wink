# Wink Demo

This page demonstrates the Wink WASM application running in the browser.

<WinkDemo />

## How it works

1. The Rust code is compiled to WebAssembly using `wasm-pack`
2. A Vue component loads and initializes the WASM module
3. The WASM code creates a canvas and handles window events
4. You can interact with the application just like the native version

## Controls

- **Escape**: Close the application
- The canvas should respond to window resize events

## Source Code

The source code for this demo is available in the [GitHub repository](https://github.com/your-username/wink).