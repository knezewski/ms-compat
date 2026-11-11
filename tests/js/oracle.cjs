'use strict';

const fs = require('node:fs');
const original = require('./ms-original.cjs');

// Read the entire request before writing output so the parent's pipes cannot
// deadlock while it sends the corpus. Hex preserves f64 bits and string bytes.
const requests = fs.readFileSync(0, 'utf8').split('\n');
if (requests.at(-1) === '') requests.pop();
const number = Buffer.alloc(8);
const results = requests.map((request) => {
  const [kind, hex] = request.split('\t');
  if (kind === 'F') {
    const value = Buffer.from(hex, 'hex').readDoubleBE();
    if (!Number.isFinite(value)) {
      // Check that the reference actually rejects each non-finite value.
      for (const long of [false, true]) {
        let threw = false;
        try { original.format(value, { long }); } catch { threw = true; }
        if (!threw) throw new Error('Reference accepted a non-finite value');
      }
      return 'F_ERR';
    }
    return 'F\t' + original.format(value) + '\t' + original.format(value, { long: true });
  }
  if (kind === 'P') {
    const input = Buffer.from(hex, 'hex').toString('utf8');
    let value;
    try {
      value = original.parse(input);
    } catch (error) {
      if (input.length === 0 || input.length > 100) return 'P_ERR_LENGTH';
      throw error; // Unexpected reference errors must fail the test.
    }
    if (Number.isNaN(value)) return 'P_ERR_FORMAT';
    number.writeDoubleBE(value);
    return 'P\t' + number.toString('hex');
  }
  throw new Error('Unknown request: ' + request);
});

process.stdout.write(results.join('\n') + '\n');
