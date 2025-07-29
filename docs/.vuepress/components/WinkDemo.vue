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
        demoStarted: false,
        loading: false,
        error: "",
        autoLoad: true, // Auto-load the demo on page load
      };
    },
    methods: {
      async loadDemo() {
        this.loading = true;
        this.error = "";
        
        try {
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