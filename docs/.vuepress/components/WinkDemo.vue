<template>
    <div id="wink-demo">
      <div class="demo-container">
        <div class="canvas-wrapper">
          <canvas id="canvas"></canvas>
        </div>
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
        default: true
      },
      // Quality scale multiplier for internal canvas resolution (supersampling)
      // 1.0 = devicePixelRatio, 1.5 = 1.5x DPR, 2.0 = 2x DPR (GPU cost increases with scale)
      qualityScale: {
        type: Number,
        default: 1.5,
      }
    },
    data() {
      return {
        demoStarted: false,
        loading: false,
        error: "",
        resizeRaf: 0,
        ro: null,
      };
    },
    methods: {
      async loadDemo() {
        this.loading = true;
        this.error = "";
        
        try {
          // Set canvas size to be responsive before loading WASM
          this.setupCanvas();
          
          // Load and initialize the WASM module built by wasm-pack using a runtime module script.
          // This avoids importing files from /public in source code, which Vite disallows.
          await new Promise((resolve, reject) => {
            const winkJsUrl = '/wasm/wink.js';
            const wasmUrl = '/wasm/wink_bg.wasm';
            // Setup a one-shot global to signal when init finishes
            const doneKey = '__wink_on_init';
            const timeout = setTimeout(() => {
              // Fail if init never signaled within 15s
              reject(new Error('WASM init timeout'));
            }, 15000);
            window[doneKey] = () => {
              clearTimeout(timeout);
              try { delete window[doneKey]; } catch (_) {}
              resolve();
            };
            const code = `import init from '${winkJsUrl}';\n` +
                         `init({ module_or_path: '${wasmUrl}' })\n` +
                         `  .then(() => window.${doneKey} && window.${doneKey}())\n` +
                         `  .catch(e => { console.error('WASM init error:', e); });`;
            const s = document.createElement('script');
            s.type = 'module';
            s.textContent = code;
            s.onerror = () => reject(new Error('Failed to load module script'));
            document.head.appendChild(s);
          });

          this.demoStarted = true;
          // Start real-time canvas resize loop (every frame)
          this.startCanvasResizeLoop();
        } catch (e) {
          this.error = `Failed to load WASM demo: ${e.message}`;
          console.error('WASM loading error:', e);
          this.demoStarted = false;
        }
        
        this.loading = false;
      },
      
      setupCanvas() {
        const canvas = document.getElementById('canvas');
        const wrapper = canvas?.parentElement;
        
        if (canvas && wrapper) {
          // Measure the actual rendered size of the wrapper/canvas (CSS pixels)
          const rect = wrapper.getBoundingClientRect();
          const dprDevice = Math.max(1, window.devicePixelRatio || 1);
          // Supersample factor to reduce aliasing on WebGL (no MSAA). Can be tuned via prop.
          const q = (typeof this.qualityScale === 'number' && this.qualityScale > 0) ? this.qualityScale : 1.0;
          const dpr = dprDevice * q;
          // Round after multiplying by DPR to avoid 1px oscillation
          let targetW = Math.round(rect.width * dpr);
          let targetH = Math.round(rect.height * dpr);

          // WebGL2 minimum guaranteed max texture size is 2048. Since we use the GL backend on Web,
          // cap the internal canvas resolution to avoid wgpu validation errors when creating textures.
          const MAX_DIM_GL = 2048;
          if (targetW > MAX_DIM_GL || targetH > MAX_DIM_GL) {
            const scale = Math.min(MAX_DIM_GL / targetW, MAX_DIM_GL / targetH);
            targetW = Math.floor(targetW * scale);
            targetH = Math.floor(targetH * scale);
          } else {
            targetW = Math.floor(targetW);
            targetH = Math.floor(targetH);
          }

          // Quantize to reduce rapid reconfigure thrash (helps reduce flicker while dragging)
          const Q = 2; // multiple of pixels in internal backing store
          targetW = Math.max(1, Math.round(targetW / Q) * Q);
          targetH = Math.max(1, Math.round(targetH / Q) * Q);

          // Do NOT set inline CSS width/height; CSS already uses 100% and will scale fluidly with the window.
          // Only update the internal resolution if it actually changed.
          if (canvas.width !== targetW || canvas.height !== targetH) {
            canvas.width = targetW;
            canvas.height = targetH;
            // Notify listeners (e.g., WASM) of the new internal size
            const event = new CustomEvent('canvasResize', {
              detail: { width: canvas.width, height: canvas.height }
            });
            canvas.dispatchEvent(event);
          }
        }
      },
      
      startCanvasResizeLoop() {
        const tick = () => {
          this.setupCanvas();
          this.resizeRaf = requestAnimationFrame(tick);
        };
        if (!this.resizeRaf) {
          this.resizeRaf = requestAnimationFrame(tick);
        }
      },
      
      handleResize() {
        // Immediate resize on event; continuous RAF loop ensures real-time updates
        this.setupCanvas();
      }
    },
    async mounted() {
      await this.$nextTick();
      
      // Setup responsive canvas on window resize with debouncing
      window.addEventListener('resize', this.handleResize);
      // Also observe the wrapper element for size changes (more reliable on some browsers/layouts)
      const canvas = document.getElementById('canvas');
      const wrapper = canvas?.parentElement;
      if (wrapper && 'ResizeObserver' in window) {
        this.ro = new ResizeObserver(() => this.handleResize());
        this.ro.observe(wrapper);
      }
      
      if (this.autoLoad) {
        await this.loadDemo();
      }
    },
    
    beforeUnmount() {
      // Clean up event listeners and timeouts
      window.removeEventListener('resize', this.handleResize);
      if (this.resizeRaf) {
        try { cancelAnimationFrame(this.resizeRaf); } catch (_) {}
        this.resizeRaf = 0;
      }
      if (this.ro) {
        try { this.ro.disconnect(); } catch (_) {}
        this.ro = null;
      }
    }
  };
  </script>
  
  <style scoped>
  .demo-container {
    position: fixed;
    inset: 0; /* top:0; right:0; bottom:0; left:0 */
    width: 100vw;
    height: 100vh;
    margin: 0;
    padding: 0;
    box-sizing: border-box;
    overflow: hidden;
    /* Match renderer clear color (0.9,0.9,0.9,1.0) to hide buffer reallocation flash */
    background-color: #e6e6e6;
  }

  .canvas-wrapper {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    margin: 0;
  }

  #canvas {
    /* Match renderer clear color to hide any transient clears during resize */
    background-color: #e6e6e6;
    display: block;
    width: 100%;
    height: 100%;
  }

  .error {
    color: #d63384;
    background-color: #f8d7da;
    border: 1px solid #f5c2c7;
    border-radius: 4px;
    padding: 10px;
    margin: 10px auto;
    max-width: 600px;
  }
  
  .start-button {
    background-color: #0d6efd;
    color: white;
    border: none;
    padding: 12px 24px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 16px;
    font-weight: 500;
    transition: background-color 0.2s ease;
  }
  
  .start-button:hover:not(:disabled) {
    background-color: #0b5ed7;
  }
  
  .start-button:disabled {
    background-color: #6c757d;
    cursor: not-allowed;
  }
  
  .instructions {
    margin-top: 15px;
    color: #666;
    font-size: 14px;
  }
  
  kbd {
    background-color: #f8f9fa;
    border: 1px solid #dee2e6;
    border-radius: 3px;
    padding: 2px 6px;
    font-size: 0.875em;
    font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
  }
  
  /* Responsive design */
  @media (max-width: 768px) {
    .demo-container {
      padding: 0 10px;
    }
    
    .canvas-wrapper {
      margin: 15px 0;
      min-height: 250px;
    }
    
    #canvas {
      border-width: 1px;
    }
  }
  </style>