# wink

Wink is a Rust library for wgpu viewer.

## Build locally using cargo

```bash
cargo check
cargo build
```

## Web

## Create pkg/demo.js

```javascript
import init, { run_web } from './wink.js';

async function run() {
    // Initialize the WASM module
    await init();
    
    // Run your application
    run_web();
}

// Start the application
run();
```

### Create VuePress Project

```bash
cd /Users/petras/rust/wink
mkdir docs
cd docs
npm init -y
npm install -D vuepress@next @vuepress/client@next @vuepress/bundler-vite@next
npm install -D @vuepress/theme-default@next
npm install -D sass-embedded
```

### Create the VuePress Directory Structure

```bash
mkdir -p .vuepress/components
mkdir -p .vuepress/public
```

### Create the VuePress Configuration File in docs/.vuepress/config.js

```javascript
import { defineUserConfig } from 'vuepress'
import { defaultTheme } from '@vuepress/theme-default'
import { viteBundler } from '@vuepress/bundler-vite'

export default defineUserConfig({
  lang: 'en-US',
  title: 'Wink - Rust WASM Demo',
  description: 'A Rust WASM application using winit and wgpu',
  
  theme: defaultTheme({
    navbar: [
      {
        text: 'Home',
        link: '/',
      },
      {
        text: 'Demo',
        link: '/demo/',
      },
    ],
  }),
  
  bundler: viteBundler({
    viteOptions: {
      server: {
        fs: {
          allow: ['..']  // Allow access to parent directory for WASM files
        }
      }
    }
  }),
})
```

### Create the Vue Component in docs/.vuepress/components/WinkDemo.vue

```vue
<template>
  <div id="wink-demo">
    <h2>Wink WASM Demo</h2>
    <div class="demo-container">
      <canvas id="canvas" width="800" height="600"></canvas>
      <div class="error" v-if="error">
        <strong>Error:</strong> {{ error }}
      </div>
      <div class="controls">
        <button 
          v-if="!demoStarted && !autoLoad" 
          @click="loadDemo()" 
          :disabled="loading"
          class="start-button"
        >
          {{ loading ? 'Loading...' : 'Start Wink Demo' }}
        </button>
        <p v-if="demoStarted" class="instructions">
          Press <kbd>Escape</kbd> to close the demo
        </p>
      </div>
    </div>
  </div>
</template>

<script>
export default {
  name: "WinkDemo",
  props: { 
    autoLoad: {
      type: Boolean,
      default: false
    }
  },
  data() {
    return {
      error: "",
      loading: false,
      demoStarted: false,
    };
  },
  methods: {
    async loadDemo() {
      this.loading = true;
      this.error = "";
      
      try {
        // Import the demo.js file from your pkg directory
        await import('../../pkg/demo.js');
        this.demoStarted = true;
      } catch (e) {
        this.error = `Failed to load WASM demo: ${e.message}`;
        console.error('WASM loading error:', e);
        this.demoStarted = false;
      }
      
      this.loading = false;
    },
  },
  async mounted() {
    await this.$nextTick();
    if (this.autoLoad) {
      await this.loadDemo();
    }
  }
};
</script>

<style scoped>
.demo-container {
  text-align: center;
  margin: 20px 0;
}

#canvas {
  border: 2px solid #333;
  background-color: #000;
  display: block;
  margin: 0 auto 20px auto;
}

.error {
  color: #d63384;
  background-color: #f8d7da;
  border: 1px solid #f5c2c7;
  border-radius: 4px;
  padding: 10px;
  margin: 10px 0;
}

.start-button {
  background-color: #0d6efd;
  color: white;
  border: none;
  padding: 10px 20px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 16px;
}

.start-button:hover:not(:disabled) {
  background-color: #0b5ed7;
}

.start-button:disabled {
  background-color: #6c757d;
  cursor: not-allowed;
}

.instructions {
  margin-top: 10px;
  color: #6c757d;
}

kbd {
  background-color: #f8f9fa;
  border: 1px solid #dee2e6;
  border-radius: 3px;
  padding: 2px 4px;
  font-family: monospace;
}
</style>
```

### Create Content Pages in docs/demo/index.md

```markdown
---
home: true
title: Wink
heroText: Wink WASM Demo
tagline: A Rust application using winit and wgpu compiled to WebAssembly
actions:
  - text: Try the Demo
    link: /demo/
    type: primary
features:
  - title: Rust + WASM
    details: Built with Rust and compiled to WebAssembly for web deployment
  - title: winit + wgpu
    details: Uses winit for windowing and wgpu for graphics rendering
  - title: VuePress Integration
    details: Seamlessly integrated into a VuePress documentation site
---


## About Wink

Wink is a Rust library for wgpu viewer, demonstrating how to build native applications that can also run in the browser through WebAssembly.

## Features

- Native desktop support with `cargo run`
- Web browser support through WASM
- Cross-platform windowing with winit
- Modern graphics with wgpu

```

### Create docs/demo/README.md:

```markdown
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
```

### Update docs/package.json 

```json
{
  "name": "wink-docs",
  "version": "1.0.0",
  "description": "Documentation for Wink WASM demo",
  "scripts": {
    "docs:dev": "vuepress dev",
    "docs:build": "vuepress build"
  },
  "devDependencies": {
    "vuepress": "^2.0.0-rc.0",
    "@vuepress/client": "^2.0.0-rc.0",
    "@vuepress/bundler-vite": "^2.0.0-rc.0"
  }
}
```

### Test the Setup

```bash
cd docs
npm run docs:dev
```


This should start a development server at http://localhost:8080 where you can test your WASM integration.
Would you like me to help you create these files step by step? We can start with the basic setup and then test the WASM integration!





