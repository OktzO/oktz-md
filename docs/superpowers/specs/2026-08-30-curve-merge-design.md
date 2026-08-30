# Merge native curve25519 ke dalam oktz-baileys (satu kesatuan)

## Latar belakang

Saat ini native curve25519 (XEdDSA sign/verify Rust, DH/keygen node:crypto) hidup di
package terpisah `oktz-curve25519@1.0.0`. `ourin-md` menggunakan npm `overrides` untuk
memaksa libsignal (`libsignal/src/curve.js`) memakai native—tanpa overrides, libsignal
resolve `curve25519-js` JS asli (lambat).

Tujuan: **satu package oktz-baileys** — konsumen install oktz-baileys, langsung native,
tanpa overrides, tanpa config.

## Arsitektur

```
oktz-baileys@9.0.24 (npm publish)
  dependencies:
    curve25519-js → npm:oktz-curve25519@0.0.4  (native Rust XEdDSA + node:crypto X25519)
    libsignal → ^6.0.0                           (dapat native via hoist)
    (other deps: pino, protobufjs, ws, dll)

  files: ["lib/**/*", "WAProto/**/*"]

Flow native:
  libsignal/src/curve.js
    → require('curve25519-js')             (npm hoist → root node_modules)
    → oktz-curve25519@0.0.4/index.cjs
        → generateKeyPair: node:crypto     (X25519, sudah native)
        → sharedKey:       node:crypto     (diffieHellman, sudah native)
        → sign:            Rust .node      (XEdDSA, tidak ada di Node native)
        → verify:          Rust .node      (XEdDSA, tidak ada di Node native)
```

## Key insight: version 0.0.4

libsignal minta `"curve25519-js": "^0.0.4"`. Saat ini native di version `1.0.0`,
npm melihat `^1.0.0` ≠ `^0.0.4` → nest copy JS asli `0.0.4` di
`libsignal/node_modules/curve25519-js` → **libsignal pakai JS**, bukan native.

Solusi: **native version `0.0.4`** → satisfy `^0.0.4` → npm gak nest→
semua resolve native. Test `fxcC` verified.

## Perubahan

### oktz-baileys

1. `package.json`:
   - version bump: `9.0.23` → `9.0.24`
   - tambah dep: `"curve25519-js": "npm:oktz-curve25519@0.0.4"`
   - files unchanged (native tidak perlu masuk files — sudah via npm alias dep)

2. Tidak ada perubahan file lib/*. Import ke `libsignal/src/curve.js` tetap.
   libsignal resolve curve via npm alias mechanism.

### oktz-curve25519 (native package)

1. Publish version `0.0.4` (sama dengan version yang di-require curve25519-js asli)
2. Deprecate version `1.0.0`
3. Source code: tidak berubah (index.cjs + .node + Rust crate)
4. Rust crate tetap di `oktz-baileys/native/curve25519/` (source of truth)

### ourin-md

1. `package.json`:
   - Hapus `overrides.curve25519-js`
   - Update `dependencies.ourin` → `"npm:oktz-baileys@^9.0.24"`
   - `npm install` → lock resolve native via baileys dep chain

## Testing (anti-leak verification)

### Leak-check test (WAJIB):
```js
const lp = require.resolve('libsignal/src/curve.js');
const r = require('module').createRequire(lp);
const p = r.resolve('curve25519-js');
// HARUS: resolve ke native index.cjs (oktz-curve25519@0.0.4)
// JANGAN: resolve ke lib/index.js (curve25519-js asli JS)
```

### Oracle test:
- 3806/3806 match (sign 3006 + verify 800)
- sha256 .node binary: `5d95f4500a11adfa...`

### Roundtrip:
- generateKeyPair → sign → verify = true
- isInit skip verify = true
- calculateAgreement = 32 byte

### Full suite:
- `ourin-md/test`: 104/104 pass

## Risiko & mitigasi

| Risiko | Mitigasi |
|--------|----------|
| Libsignal internal resolve curve25519-js JS asli (leak) | Version 0.0.4 → satisfy range, gak ada nesting. Verifikasi leak-check test. |
| npm alias tidak hoist di semua npm version | Test dengan npm 10.8.2 (Node 20.19.1) — terbukti hoist. |
| .node binary hanya linux-x64-gnu | User confirmed: linux x64 saja. |
| oktz-curve25519@1.0.0 masih terpakai konsumen lain | Deprecate, konsumen migrasi ke 0.0.4. |
| Rust rebuild butuh toolchain | Cargo 1.98.0 tersedia. Binary di-commit verified. |

## Build & publish flow

1. `cd oktz-baileys && cd native/curve25519 && cargo build --release`
   → menghasilkan `curve25519.linux-x64-gnu.node`
2. Copy `.node` ke `native/curve25519/`
3. `cd native/curve25519 && npm version 0.0.4 && npm publish`
4. `cd ../../ && npm version 9.0.24 && npm publish`
5. Hapus overrides di ourin-md, update dep, `npm install`
6. Leak-check + test suite + smoke test `.menu` di produksi