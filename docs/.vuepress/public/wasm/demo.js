import init, { run_web } from './wink.js';

async function run() {
    // Initialize the WASM module
    await init();
    
    // Run your application
    run_web();
}

// Start the application
run();