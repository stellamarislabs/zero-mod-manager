// Check the shared source, every Windows ICO frame, and installer/runtime paths.
// --render also re-renders the SVG in a temporary directory using Tauri's CLI.
import assert from 'node:assert/strict';
import { readFileSync, mkdtempSync, rmSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const read = path => readFileSync(path);
const hash = value => createHash('sha256').update(value).digest('hex');
const manifest = JSON.parse(read('src-tauri/icons/brand-manifest.json'));
assert.equal(manifest.source, 'src/assets/icon.svg');
assert.equal(hash(read(manifest.source)), manifest.sourceSha256, 'SVG changed: run npm run icons:generate');
assert.deepEqual(read(manifest.source), read('src-tauri/icons/app-icon.svg'));
const ico = read('src-tauri/icons/icon.ico');
assert.equal(hash(ico), manifest.icoSha256);
assert.equal(manifest.resourceName, `zero-brand-${hash(ico).slice(0, 12)}.ico`);
assert.equal(ico.readUInt16LE(0), 0);
assert.equal(ico.readUInt16LE(2), 1);
assert.equal(ico.readUInt16LE(4), 4);
for (const [i, [name, digest]] of Object.entries(manifest.frames).entries()) {
  const png = read(`src-tauri/icons/${name}`);
  assert.equal(hash(png), digest, `Untracked frame change: ${name}`);
  const offset = 6 + i * 16;
  const size = ico.readUInt32LE(offset + 8);
  const start = ico.readUInt32LE(offset + 12);
  assert.deepEqual(ico.subarray(start, start + size), png, `ICO frame differs: ${name}`);
  assert.equal(ico[offset] || 256, png.readUInt32BE(16));
  assert.equal(ico[offset + 1] || 256, png.readUInt32BE(20));
}
const config = JSON.parse(read('src-tauri/tauri.conf.json'));
const document = read('index.html').toString();
assert.match(document, /<title>Zero Mod Manager<\/title>/);
assert.match(document, /rel="icon"[^>]+href="\/src\/assets\/icon\.svg"/);
assert.equal(config.identifier, 'app.zeromodmanager.desktop');
assert.equal(config.bundle.resources['icons/icon.ico'], manifest.resourceName);
assert.equal(config.bundle.windows.nsis.installerIcon, 'icons/icon.ico');
assert.equal(config.bundle.windows.nsis.uninstallerIcon, 'icons/icon.ico');
assert.equal(config.bundle.windows.nsis.installerHooks, '../scripts/installer-hooks.nsh');
assert.match(read('src-tauri/icons/windows-icon.nsh').toString(), new RegExp(manifest.resourceName.replaceAll('.', '\\.')));
assert.match(read('src-tauri/src/window_branding.rs').toString(), /include_bytes!\("\.\.\/icons\/icon\.ico"\)/);

if (process.argv.includes('--render')) {
  const temporary = mkdtempSync(join(tmpdir(), 'zero-brand-verify-'));
  try {
    const result = spawnSync(process.execPath, [resolve('node_modules/@tauri-apps/cli/tauri.js'), 'icon', resolve(manifest.source), '--output', temporary], { encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr || result.stdout);
    for (const name of Object.keys(manifest.frames)) {
      assert.deepEqual(read(join(temporary, name)), read(`src-tauri/icons/${name}`), `SVG render differs: ${name}`);
    }
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}
console.log(`Branding verified: shared SVG, 4 ICO frames, installer/uninstaller, ${manifest.resourceName}.`);
