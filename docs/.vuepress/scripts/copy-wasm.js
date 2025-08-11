#!/usr/bin/env node
/*
 Cross-platform copy of WASM package files into VuePress public directory.
 - Copies: *.js, *.wasm, *.d.ts from <repo>/pkg to <repo>/docs/.vuepress/public/wasm
 - Does not fail the build if pkg does not exist yet (mimics `|| true`).
*/
const fs = require('fs');
const path = require('path');

function ensureDir(dir) {
  try {
    fs.mkdirSync(dir, { recursive: true });
  } catch (e) {
    // ignore
  }
}

function copyFile(src, dest) {
  try {
    fs.copyFileSync(src, dest);
    return true;
  } catch (e) {
    console.warn(`[copy-wasm] Failed to copy ${src} -> ${dest}: ${e.message}`);
    return false;
  }
}

(function main() {
  const docsDir = path.resolve(__dirname, '..', '..'); // docs/
  const repoRoot = path.resolve(docsDir, '..');
  const srcDir = path.resolve(repoRoot, 'pkg');
  const destDir = path.resolve(docsDir, '.vuepress', 'public', 'wasm');

  ensureDir(destDir);

  if (!fs.existsSync(srcDir)) {
    console.warn(`[copy-wasm] Source directory not found: ${srcDir}. Skipping copy.`);
    process.exit(0);
  }

  const allowedExt = new Set(['.js', '.wasm', '.d.ts']);
  let copied = 0;
  try {
    const entries = fs.readdirSync(srcDir, { withFileTypes: true });
    for (const entry of entries) {
      if (!entry.isFile()) continue;
      const ext = path.extname(entry.name);
      if (!allowedExt.has(ext)) continue;
      const srcPath = path.join(srcDir, entry.name);
      const destPath = path.join(destDir, entry.name);
      if (copyFile(srcPath, destPath)) copied++;
    }
  } catch (e) {
    console.warn(`[copy-wasm] Failed to read directory ${srcDir}: ${e.message}`);
    // Do not fail
    process.exit(0);
  }

  console.log(`[copy-wasm] Copied ${copied} file(s) from ${srcDir} -> ${destDir}`);
})();
