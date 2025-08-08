// Watch and copy geometry JSON files from src/openmodel/ into docs/.vuepress/public/geometry/
// Works during `npm run dev` to enable live updates on the web without rebuilding WASM.

const fs = require('fs');
const path = require('path');

const repoRoot = path.resolve(__dirname, '../../..');
const srcDir = path.join(repoRoot, 'src', 'openmodel');
const dstDir = path.resolve(__dirname, '../public/geometry');

const files = [
  'all_geometry.json',
  'geometry_data.json',
];

function ensureDir(dir) {
  try { fs.mkdirSync(dir, { recursive: true }); } catch (_) {}
}

function copyIfExists(src, dst) {
  try {
    ensureDir(path.dirname(dst));
    const data = fs.readFileSync(src);
    fs.writeFileSync(dst, data);
    console.log(`[watch-geometry] Copied ${src} -> ${dst} (${data.length} bytes)`);
  } catch (e) {
    // File may not exist yet; ignore
  }
}

function watchFile(src, dst) {
  // Try chokidar first if available; fall back to fs.watch
  try {
    const chokidar = require('chokidar');
    const watcher = chokidar.watch(src, { ignoreInitial: false, persistent: true });
    watcher.on('add', () => copyIfExists(src, dst));
    watcher.on('change', () => copyIfExists(src, dst));
    watcher.on('error', (err) => console.warn('[watch-geometry] chokidar error:', err));
    console.log(`[watch-geometry] Watching (chokidar): ${src}`);
  } catch (_) {
    console.log('[watch-geometry] chokidar not found, using fs.watch');
    // Initial copy
    copyIfExists(src, dst);
    // Basic directory watch
    try {
      fs.watch(path.dirname(src), { persistent: true }, (event, filename) => {
        if (!filename) return;
        if (path.basename(filename) === path.basename(src)) {
          // Debounce a bit
          setTimeout(() => copyIfExists(src, dst), 50);
        }
      });
      console.log(`[watch-geometry] Watching (fs.watch): ${src}`);
    } catch (err) {
      console.error('[watch-geometry] fs.watch error:', err);
    }
  }
}

function main() {
  ensureDir(dstDir);
  for (const f of files) {
    const src = path.join(srcDir, f);
    const dst = path.join(dstDir, f);
    // Do an initial copy and then watch
    copyIfExists(src, dst);
    watchFile(src, dst);
  }
}

main();
