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
        default: false
      }
    },
    data() {
      return {
        demoStarted: false,
        loading: false,
        error: "",
        autoLoad: true, // Auto-load the demo on page load
        resizeTimeout: null,
      };
    },
    methods: {
      async loadDemo() {
        this.loading = true;
        this.error = "";
        
        try {
          // Set canvas size to be responsive before loading WASM
          this.setupCanvas();
          
          // Load the demo.js file using script tag approach since it's in /public
          const script = document.createElement('script');
          script.type = 'module';
          script.src = '/wasm/demo.js';
          
          // Wait for script to load
          await new Promise((resolve, reject) => {
            script.onload = resolve;
            script.onerror = reject;
            document.head.appendChild(script);
          });
          
          this.demoStarted = true;
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
          // Get the wrapper's dimensions
          const rect = wrapper.getBoundingClientRect();
          const containerWidth = wrapper.clientWidth;
          
          // Calculate optimal size with some padding
          const padding = 20;
          const maxWidth = Math.min(containerWidth - padding, 800);
          const width = Math.max(400, maxWidth); // Minimum 400px width
          const height = Math.round((width * 3) / 4); // 4:3 aspect ratio
          
          // Set both canvas internal resolution and display size
          canvas.width = width;
          canvas.height = height;
          canvas.style.width = width + 'px';
          canvas.style.height = height + 'px';
          
          console.log(`Canvas resized to: ${width}x${height}`);
        }
      },
      
      handleResize() {
        // Debounce resize events to avoid excessive calls
        if (this.resizeTimeout) {
          clearTimeout(this.resizeTimeout);
        }
        
        this.resizeTimeout = setTimeout(() => {
          if (this.demoStarted) {
            this.setupCanvas();
            
            // Trigger a custom event that the WASM code can listen to
            const canvas = document.getElementById('canvas');
            if (canvas) {
              const event = new CustomEvent('canvasResize', {
                detail: { width: canvas.width, height: canvas.height }
              });
              canvas.dispatchEvent(event);
            }
          }
        }, 100); // 100ms debounce
      }
    },
    async mounted() {
      await this.$nextTick();
      
      // Setup responsive canvas on window resize with debouncing
      window.addEventListener('resize', this.handleResize);
      
      if (this.autoLoad) {
        await this.loadDemo();
      }
    },
    
    beforeUnmount() {
      // Clean up event listeners and timeouts
      window.removeEventListener('resize', this.handleResize);
      if (this.resizeTimeout) {
        clearTimeout(this.resizeTimeout);
      }
    }
  };
  </script>
  
  <style scoped>
  .demo-container {
    text-align: center;
    margin: 20px auto;
    width: 100%;
    max-width: 900px;
    padding: 0 20px;
    box-sizing: border-box;
  }
  
  .canvas-wrapper {
    display: flex;
    justify-content: center;
    align-items: center;
    margin: 20px 0;
    width: 100%;
    min-height: 300px;
  }
  
  #canvas {
    border: 2px solid #333;
    background-color: #000;
    display: block;
    border-radius: 4px;
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
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