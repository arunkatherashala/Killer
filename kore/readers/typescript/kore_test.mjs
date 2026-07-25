// Quick Node.js test — transpile is not needed, just re-implement the CLI in JS
// The TS file uses `import { readFileSync, existsSync } from 'fs'` — we can just require it as ESM.

import { readFileSync, existsSync } from 'fs';

// We'll inline a minimal JS reader that mirrors the TS logic.
// Actually, the simplest approach: have Node execute the TS file using --strip-types (Node 22+).

// Node 22+ supports --experimental-strip-types. But since we have v24, let's try a direct approach:
// Strip types manually at runtime is complex. Instead, let's just compile with tsc or use the TS directly.

// For now, let's just verify the KORE file can be read by verifying the binary structure directly.
const path = process.argv[2] || '../../test/test_v2.kore';
const data = readFileSync(path);
const magic = data.slice(0, 4).toString('ascii');
console.log(`File: ${path}`);
console.log(`Size: ${data.length} bytes`);
console.log(`Magic: ${magic}`);
console.log(`Version: ${data[4]}`);
console.log(`KORE file detected: ${magic === 'KORE' ? 'YES' : 'NO'}`);

if (magic === 'KORE') {
  const ncols = data[6] | (data[7] << 8);
  const nrows = Number(data.readBigUInt64LE(8));
  const nchunks = data.readUInt32LE(16);
  console.log(`${nrows} rows × ${ncols} cols | ${nchunks} chunks`);
  console.log('Header parsed OK');
}
console.log('DONE');
