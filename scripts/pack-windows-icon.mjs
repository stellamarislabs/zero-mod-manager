// Build every Windows ICO frame from the generated, shared-brand PNGs.
import { readFileSync, writeFileSync } from 'node:fs';
const sizes = [32, 64, 128, 256];
const names = ['32x32.png', '64x64.png', '128x128.png', '128x128@2x.png'];
const frames = names.map(name => readFileSync(`src-tauri/icons/${name}`));
const header = Buffer.alloc(6 + frames.length * 16);
header.writeUInt16LE(1, 2);
header.writeUInt16LE(frames.length, 4);
let offset = header.length;
frames.forEach((frame, i) => {
  if (frame.readUInt32BE(16) !== sizes[i] || frame.readUInt32BE(20) !== sizes[i]) throw new Error('Unexpected icon dimensions');
  const at = 6 + i * 16;
  header[at] = header[at + 1] = sizes[i] % 256;
  header.writeUInt16LE(1, at + 4);
  header.writeUInt16LE(32, at + 6);
  header.writeUInt32LE(frame.length, at + 8);
  header.writeUInt32LE(offset, at + 12);
  offset += frame.length;
});
writeFileSync('src-tauri/icons/icon.ico', Buffer.concat([header, ...frames]));
console.log('Windows ICO: 32, 64, 128 and 256px from shared brand PNGs.');
