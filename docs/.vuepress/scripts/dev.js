#!/usr/bin/env node
/*
 Cross-platform dev orchestrator:
 1) Copies WASM artifacts into VuePress public dir.
 2) Starts geometry watcher and VuePress dev server in parallel.
*/
const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');

const docsDir = path.resolve(__dirname, '..', '..');

function run(cmd, args, opts = {}) {
  const child = spawn(cmd, args, {
    cwd: docsDir,
    stdio: 'inherit',
    shell: true, // ensure cross-platform resolution of local CLIs like npx
    ...opts,
  });
  return child;
}

function onExit(code) {
  process.exit(code ?? 0);
}

function resolveLocalVuePressCLI() {
  try {
    // Find local vuepress package.json
    const pkgPath = require.resolve('vuepress/package.json', { paths: [docsDir] });
    const pkgDir = path.dirname(pkgPath);
    const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
    let binRel = typeof pkg.bin === 'string' ? pkg.bin : (pkg.bin && pkg.bin.vuepress);
    if (!binRel) {
      // Fallback common location
      binRel = 'bin/vuepress.js';
    }
    const cli = path.resolve(pkgDir, binRel);
    if (fs.existsSync(cli)) return cli;
  } catch (e) {
    // not found
  }
  return null;
}

function ensureDocsDepsInstalled(next) {
  const hasVuePress = fs.existsSync(path.join(docsDir, 'node_modules', 'vuepress'));
  if (hasVuePress) return next();
  console.log('[dev] Installing docs dependencies...');
  const installer = run('npm', ['install', '--no-fund', '--no-audit']);
  installer.on('exit', (code) => {
    if (code !== 0) {
      console.warn(`[dev] npm install exited with code ${code}. Continuing, but dev may fail.`);
    }
    next();
  });
}

function startDev() {
  const watcher = run('node', ['.vuepress/scripts/watch-geometry.js']);
  const port = process.env.PORT || '8080';
  const cliPath = resolveLocalVuePressCLI();
  let vuepress;
  if (cliPath) {
    vuepress = run('node', [cliPath, 'dev', '--port', port, '--host', '127.0.0.1']);
  } else {
    console.warn('[dev] Local VuePress CLI not found. Falling back to npx vuepress@^2.');
    vuepress = run('npx', ['vuepress@^2', 'dev', '--port', port, '--host', '127.0.0.1']);
  }

  let exiting = false;
  function cleanup(code) {
    if (exiting) return;
    exiting = true;
    try { watcher.kill(); } catch (_) {}
    try { vuepress.kill(); } catch (_) {}
    onExit(code);
  }

  process.on('SIGINT', () => cleanup(0));
  process.on('SIGTERM', () => cleanup(0));

  watcher.on('exit', (code) => {
    console.log(`[dev] watch-geometry exited with code ${code}`);
  });
  vuepress.on('exit', (code) => {
    console.log(`[dev] vuepress exited with code ${code}`);
    cleanup(code);
  });
}

(function main() {
  ensureDocsDepsInstalled(() => {
    const copy = run('node', ['.vuepress/scripts/copy-wasm.js']);
    copy.on('exit', (code) => {
      if (code && code !== 0) {
        console.warn(`[dev] copy-wasm exited with code ${code}, continuing...`);
      }
      startDev();
    });
  });
})();
