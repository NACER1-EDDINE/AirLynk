// AirLynk native ↔ WASM cipher cross-check.
//
// Verifies that the WASM build of airlynk-crypto agrees byte-for-byte with the
// native implementation. The native side writes fixture-vectors.json via
// `cargo test -p airlynk-crypto print_wasm_fixture`; this script instantiates
// the wasm, reproduces the native encrypt with the WASM exports, and decrypts
// the native ciphertext back. A mismatch here would fail only on a real phone,
// at the worst possible moment — hence the check.
//
// Run:  node scripts/wasm-cross-check.mjs
// (requires target-wasm/wasm32-unknown-unknown/release/airlynk_crypto.wasm)

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const wasmPath = join(root, "target-wasm", "wasm32-unknown-unknown", "release", "airlynk_crypto.wasm");
const fixturePath = join(root, "crates", "airlynk-crypto", "fixture-vectors.json");

const wasmBytes = readFileSync(wasmPath);
const fixture = JSON.parse(readFileSync(fixturePath, "utf8"));

const hex = (bytes) => [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");
const unhex = (s) => new Uint8Array(s.match(/../g).map((h) => parseInt(h, 16)));

const key = unhex(fixture.key);
const base = unhex(fixture.base);
const pt = unhex(fixture.pt);
const nativeCt = unhex(fixture.ct);
const idx = fixture.idx;

let failures = 0;
const check = (name, cond) => {
  console.log(`${cond ? "PASS" : "FAIL"}  ${name}`);
  if (!cond) failures++;
};

const { instance } = await WebAssembly.instantiate(wasmBytes, {});
const { memory, airlynk_encrypt, airlynk_decrypt, airlynk_encrypt_len, airlynk_decrypt_len } = instance.exports;

const KEY_LEN = 32;
const NONCE_LEN = 12;

// Layout inside wasm linear memory: key, base, pt, ct, out.
const keyOff = 0;
const baseOff = keyOff + KEY_LEN;
const ptOff = baseOff + NONCE_LEN;
const ctOff = ptOff + pt.length;
const outOff = ctOff + pt.length + 16 + 4; // room for framed

let needed = outOff + pt.length + 16 + 4 + 64;
if (memory.buffer.byteLength < needed) {
  memory.grow(Math.ceil((needed - memory.buffer.byteLength) / 65536));
}
const mem = () => new Uint8Array(memory.buffer);

// Write inputs.
{
  const m = mem();
  m.set(key, keyOff);
  m.set(base, baseOff);
  m.set(pt, ptOff);
}

// 1. WASM encrypt must reproduce the native ciphertext exactly.
{
  const outLen = airlynk_encrypt_len(pt.length);
  check(`encrypt_len(${pt.length}) = ${pt.length + 4 + 16}`, outLen === pt.length + 4 + 16);
  const rc = airlynk_encrypt(keyOff, KEY_LEN, baseOff, NONCE_LEN, idx, 0, ptOff, pt.length, outOff);
  check("wasm encrypt rc = 0", rc === 0);
  // fresh view: the wasm allocator may have grown memory during the call
  const m = mem();
  const framed = m.slice(outOff, outOff + outLen);
  const ct = framed.subarray(4);
  check("wasm encrypt == native encrypt (byte-for-byte)", hex(ct) === hex(nativeCt));
  const prefix = new DataView(framed.buffer, framed.byteOffset, 4).getUint32(0);
  check("length prefix carries plaintext length", prefix === pt.length);
}

// 2. WASM decrypt of the NATIVE ciphertext must recover the plaintext.
{
  mem().set(nativeCt, ctOff);
  const outLen = airlynk_decrypt_len(nativeCt.length + 4);
  check(`decrypt_len(${nativeCt.length + 4}) = ${nativeCt.length - 16}`, outLen === nativeCt.length - 16);
  const rc = airlynk_decrypt(keyOff, KEY_LEN, baseOff, NONCE_LEN, idx, 0, ctOff, nativeCt.length, outOff);
  check("wasm decrypt of native ct rc = 0", rc === 0);
  const m = mem();
  const plain = m.slice(outOff, outOff + outLen);
  check("wasm decrypt(native ct) == plaintext", hex(plain) === hex(pt));
}

// 3. Tamper: a flipped ciphertext byte must fail authentication.
{
  const tampered = nativeCt.slice();
  tampered[0] ^= 0x01;
  mem().set(tampered, ctOff);
  const rc = airlynk_decrypt(keyOff, KEY_LEN, baseOff, NONCE_LEN, idx, 0, ctOff, tampered.length, outOff);
  check("tampered ct rejected (rc = 1)", rc === 1);
}

// 4. Wrong chunk index must fail authentication (nonce derivation matters).
{
  mem().set(nativeCt, ctOff);
  const rc = airlynk_decrypt(keyOff, KEY_LEN, baseOff, NONCE_LEN, idx + 1, 0, ctOff, nativeCt.length, outOff);
  check("wrong chunk index rejected (rc = 1)", rc === 1);
}

// 5. Bad arguments are rejected without panicking across the boundary.
{
  const rc = airlynk_decrypt(keyOff, 31, baseOff, NONCE_LEN, idx, 0, ctOff, nativeCt.length, outOff);
  check("wrong key length rejected (rc = 2)", rc === 2);
}

console.log(failures === 0 ? "\nALL CROSS-CHECKS PASSED" : `\n${failures} CHECK(S) FAILED`);
process.exit(failures === 0 ? 0 : 1);
