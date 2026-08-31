# Signal Protocol Rust Rewrite — oktz-signal

## Objective

Replace `libsignal@6.0.0` (GPL-3.0, 2270 baris JS) dengan `oktz-signal` — package npm baru berisi Rust native module (napi-rs) + JS wrapper tipis, lisensi MIT. Hot path per-pesan (Double Ratchet decrypt/encrypt) + init session (X3DH) + protobuf wire format semuanya di Rust. JS hanya pegang storage async.

Bot produksi `ourin-md` output: `oktz-baileys` tanpa dep `libsignal`, semua Signal protocol logic di `oktz-signal` yang ditarik transitif. Satu install, MIT murni.

## Context

### libsignal@6.0.0 Breakdown

| File | Baris | Fungsi | Fate |
|------|-------|--------|------|
| `WhisperTextProtocol.js` | 933 | protobufjs generated: WhisperMessage, PreKeyWhisperMessage, KeyExchangeMessage | Rewrite hand-written minimal codec di Rust |
| `session_cipher.js` | 336 | Double Ratchet encrypt/decrypt + storage orchestration | Rust pure (ratchet.rs) + JS wrapper orchestrate IO |
| `session_record.js` | 316 | SessionRecord state machine: current session + archived + V1 migration | Rust pure (session.rs), format kompatibel |
| `session_builder.js` | 164 | X3DH prekey session build | Rust pure (x3dh.rs) |
| `curve.js` | 142 | XEdDSA + X25519 | Copy dari `oktz-curve25519` crate (MIT kita) |
| `crypto.js` | 98 | AES-CBC + HMAC-SHA256 + HKDF via node:crypto | RustCrypto (`aes`, `hmac`, `sha2`, `hkdf`) |
| `numeric_fingerprint.js` | 72 | Safety number fingerprint | SKIP — tidak dipakai baileys (YAGNI) |
| `queue_job.js` | 69 | Async job serializer per-address | Reimplement di JS wrapper |
| `keyhelper.js` | 45 | generatePreKey, generateSignedPreKey | SKIP — baileys pakai generateRegistrationId sendiri |
| util (errors, proto address, dll) | ~82 | Error classes, ProtocolAddress | Implement di JS wrapper |
| **Total** | **2270** | | **~1758 rewrite, ~512 skip** |

### API Surface yang Dipakai Baileys

Dari `lib/Signal/libsignal.js` + `lib/Signal/Group/` + `lib/Socket/messages-send.js`:

- **SessionCipher(storage, addr)** — `encrypt(data)`, `decryptWhisperMessage(buf)`, `decryptPreKeyWhisperMessage(buf)`
- **SessionBuilder(storage, addr)** — `processPreKey(preKeyObj)`, `initOutgoing(session)`
- **SessionRecord** — `deserialize(jsonStr)` (static), instance: `haveOpenSession()`, `serialize()`
- **ProtocolAddress(name, deviceId)** — `toString()`
- **PreKeyWhisperMessage** protobuf — `decode(buffer)` (ekstrak identity key untuk identity change detection)
- **crypto.js helpers** — `encrypt(key, data, iv)`, `decrypt(key, data, iv)`, `calculateMAC(key, data)`, `deriveSecrets(input, salt, info, chunks)`, `hash(data)`, `verifyMAC(data, key, mac, length)` — dipakai Group files (group_cipher, sender-chain-key, sender-message-key)
- **errors** — `NoSessionError`, `InvalidKeyError`, `InvalidMessageType` — catch generic di baileys

Yang **TIDAK** dipakai baileys → SKIP YAGNI: `numeric_fingerprint`, `keyhelper`, `KeyExchangeMessage` proto, `queue_job` (internal, reimplement).

## Keputusan Design

Ditetapkan melalui Q&A (8 keputusan):

| # | Aspek | Keputusan |
|---|-------|-----------|
| 1 | **Sumber** | Clean-room dari spesifikasi publik Signal. Spesifikasi: [X3DH](https://signal.org/docs/specifications/x3dh/), [Double Ratchet](https://signal.org/docs/specifications/doubleratchet/), [Session Management](https://signal.org/docs/specifications/sessions/). Wire format = interop fact. |
| 2 | **Bahasa implementasi** | Rust penuh. Protocol logic (X3DH, Double Ratchet, session state) di Rust crate. RustCrypto: aes, hmac, sha2, hkdf. |
| 3 | **Package** | `oktz-signal` npm package terpisah. `oktz-baileys` depend (transitive, 1 install). Curve source (MIT kita) di-copy ke crate. |
| 4 | **Scope fase 1** | 1:1 fitur libsignal yang dipakai baileys. Group sender-key protocol tetap di baileys, pakai crypto.js helpers dari oktz-signal. |
| 5 | **Storage** | Rust pure state machine, JS pegang IO. Rust fungsi pure (session JSON in → session JSON out + message), JS wrapper urus load/store async. |
| 6 | **Verifikasi** | Cross-interop oracle. libsignal di devDependencies oktz-signal (bukan runtime). Encrypt-decrypt roundtrip kedua arah. |
| 7 | **Platform binary** | linux-x64 glibc (sama dengan oktz-curve25519). |
| 8 | **Rollout** | Staged: oktz-signal v0.1.0 publish dulu (test oracle), lalu baileys PR kecil swap import + hapus dep libsignal. |

## Arsitektur

### Package Layout oktz-signal

```
oktz-signal/
├── package.json            # type: module, files: index.js, src/, native/*.node
├── index.js                # ESM re-export: SessionCipher, SessionBuilder, SessionRecord, ProtocolAddress, errors, crypto
├── src/
│   ├── session-cipher.js   # JS wrapper: queueJob, storage async, panggil Rust pure
│   ├── session-builder.js  # JS wrapper: processPreKey, initOutgoing, storage IO
│   ├── session-record.js   # deserialize/serialize wrapper
│   ├── protocol-address.js # {name, deviceId, toString()}
│   ├── errors.js           # Error classes (MIT)
│   ├── queue-job.js        # Async job serializer per-address (69 baris, clean-room dari queue pattern)
│   └── crypto.js           # encrypt/decrypt/calculateMAC/deriveSecrets/hash/verifyMAC — node:crypto wrapper (MIT)
├── native/
│   └── signal/
│       ├── Cargo.toml      # napi-rs, RustCrypto, x25519-dalek
│       ├── build.rs
│       └── src/
│           ├── lib.rs      # napi-rs exports
│           ├── curve.rs    # XEdDSA + X25519 (copy dari kode MIT oktz-curve25519, 3806/3806 oracle)
│           ├── x3dh.rs     # X3DH key agreement (SessionBuilder.processPreKey)
│           ├── ratchet.rs  # Double Ratchet encrypt/decrypt (RustCrypto)
│           ├── session.rs  # SessionRecord: state machine, JSON serialization, archive
│           └── proto.rs    # WhisperMessage + PreKeyWhisperMessage codec (hand-written protobuf minimal)
└── tests/
    └── oracle/
        └── interop.test.mjs  # Cross-interop test vs libsignal (devDependency)
```

### Rust Crate Dependencies (Cargo.toml)

```toml
[dependencies]
napi = { version = "2", features = ["napi8"] }
napi-derive = "2"
x25519-dalek = "2"           # X25519 DH (curve.rs juga punya sendiri, kalau double coverage)
serde = { version = "1", features = ["derive"] }
serde_json = "1"
aes = "0.8"                  # AES-CBC
block-modes = "0.9"          # CBC mode
hmac = "0.12"                # HMAC-SHA256
sha2 = "0.10"                # SHA-256, SHA-512
hkdf = "0.12"                # HKDF (RFC 5869)
rand = "0.8"                 # Random nonce, ephemeral key
```

Catatan: `curve.rs` bisa pakai implementasi sendiri (sudah oracle 3806/3806) atau `x25519-dalek` untuk DH. Copy curve.rs lebih aman — sudah teruji bit-exact dengan libsignal. `x25519-dalek` opsional sebagai referensi.

### Rust Pure Functions (napi-rs exports)

```rust
// SessionBuilder
fn signal_build_initial_session(
    identity_priv: &[u8], identity_pub: &[u8],
    signed_prekey_pub: &[u8], signed_prekey_sig: &[u8],
    prekey_pub: Option<&[u8]>, prekey_id: Option<u32>,
    recipient_pub: &[u8], recipient_prekey: &[u8],
    registration_id: u32
) -> Result<String> // JSON session record

// SessionCipher encrypt
fn signal_ratchet_encrypt(
    session_json: &str, plaintext: &[u8]
) -> Result<String> // JSON { session: ..., message: [u8], type: 3|1 }

// SessionCipher decrypt PreKeyWhisperMessage
// Handle dua kasus internal: session kosong → X3DH build dari isi pkmsg (init flow);
// session ada → normal ratchet decrypt. Output session disimpan JS.
fn signal_ratchet_decrypt_pkmsg(
    session_json: &str, ciphertext: &[u8]
) -> Result<String> // JSON { session: ..., plaintext: [u8] }

// SessionCipher decrypt WhisperMessage
// identity key tidak perlu param — berasal dari session record (indexInfo.remoteIdentityKey)
fn signal_ratchet_decrypt_msg(
    session_json: &str, ciphertext: &[u8]
) -> Result<String> // JSON { session: ..., plaintext: [u8] }

// SessionRecord
fn session_deserialize(json: &str) -> Result<String> // normalized internal JSON
fn session_serialize(session_json: &str) -> Result<String> // canonical output JSON
fn session_have_open_session(session_json: &str) -> Result<bool>

// proto
fn proto_decode_pkmsg(bytes: &[u8]) -> Result<String> // JSON { fields... }
fn proto_encode_pkmsg(fields_json: &str) -> Result<Vec<u8>>
fn proto_decode_whisper(bytes: &[u8]) -> Result<String>
fn proto_encode_whisper(fields_json: &str) -> Result<Vec<u8>>
```

### JS Wrapper (ESM) — Data Flow

**Encrypt:**
```
JS: SessionCipher.encrypt(data)
  → queueJob(addr, async () => {
      session_json = await storage.loadSession(addr)
      result = Rust.signal_ratchet_encrypt(session_json, data)
      // result = { session: new_session_json, message: Vec<u8>, type: 3|1 }
      await storage.storeSession(addr, result.session)
      return { type: result.type, body: result.message }
    })
```

**Decrypt (PreKeyWhisperMessage):**
```
JS: SessionCipher.decryptPreKeyWhisperMessage(buf)
  → queueJob(addr, async () => {
      session_json = await storage.loadSession(addr)
      // Kalau null (first time), session empty → Rust create session
      // atau Rust handle empty session (return error → baileys catch)
      result = Rust.signal_ratchet_decrypt_pkmsg(session_json || "{}", buf)
      await storage.storeSession(addr, result.session)
      return result.plaintext
    })
```

**SessionBuilder.processPreKey:**
```
JS: SessionBuilder.processPreKey(preKeyObj)
  → queueJob(addr, async () => {
      identity = await storage.getOurIdentity()
      regId = await storage.getOurRegistrationId()
      signedPreKey = await storage.loadSignedPreKey()
      recipientKey = await storage.loadIdentityKey(addr)
      session_json = Rust.signal_build_initial_session(
        identity.priv, identity.pub,
        signedPreKey.pubKey, signedPreKey.signature,
        preKeyObj.preKey?.pubKey, preKeyObj.preKey?.keyId,
        recipientKey, preKeyObj.identityKey,
        regId
      )
      await storage.storeSession(addr, session_json)
    })
```

### Session Record Format Kompatibilitas

Session record dari produksi menggunakan format JSON libsignal. `session.rs` harus:
1. Mendeserialize format JSON yang sama (field names, nesting)
2. Round-trip lossless (serialize → deserialize → serialize = identik)
3. Saat deserialize session lama, data yang tidak dikenal (future fields) disimpan/diteruskan

Format (interop fact, di-derive dari data produksi, bukan dari kode):

```json
{
  "registrationId": 12345,
  "currentRatchet": {
    "ephemeralKeyPair": { "privKey": "base64", "pubKey": "base64" },
    "lastRemoteEphemeralKey": "base64",
    "previousCounter": 0,
    "rootKey": "base64"
  },
  "indexInfo": {
    "baseKey": "base64",
    "baseKeyType": 0,
    "closed": false,
    "used": false,
    "created": 1234567890,
    "remoteIdentityKey": "base64",
    "ratchetDirection": 0,
    "ratchetCounter": 0
  },
  "sessions": [
    {
      "session": {
        "registrationId": 12345,
        "currentRatchet": { ... },
        "indexInfo": { ... },
        "oldRatchetList": [ ... ]
      },
      "chain": { ... },
      "indexInfo": { ... },
      "timestamp": 1234567890
    }
  ],
  "version": "v1"
}
```

Catatan: Rust `serde_json` dengan `Value::Object` dapat menangani unknown fields secara otomatis — forward compatibility.

### Protobuf Wire Codec (proto.rs)

WhisperMessage dan PreKeyWhisperMessage adalah protobuf message sederhana (4-5 field tiap). Hand-write protobuf reader/writer tanpa prost:

- **WhisperMessage**: ephemeralKey (bytes), counter (uint32), previousCounter (uint32), ciphertext (bytes)
- **PreKeyWhisperMessage**: preKeyId (uint32), baseKey (bytes), identityKey (bytes), message (WhisperMessage bytes), registrationId (uint32), signedPreKeyId (uint32)

Protobuf wire format = public spec (interop fact). Codec ~150 baris Rust.

### QueueJob (JS, clean-room)

QueueJob dari libsignal (69 baris) — serial queue per-address untuk mencegah race condition pada session record yang sama. Reimplement di JS:

```js
// queue-job.js (MIT, clean-room)
class QueueJob {
  constructor() { this.queues = new Map() }
  async add(key, fn) {
    let queue = this.queues.get(key)
    if (!queue) {
      queue = Promise.resolve()
      this.queues.set(key, queue)
    }
    queue = queue.then(fn, fn) // run after previous, regardless of error
    this.queues.set(key, queue)
    return queue
  }
}
```

## Testing (Oracle)

### Cross-Interop Oracle

libsignal@6.0.0 sebagai `devDependencies` (bukan runtime). Test menjalankan kedua engine dan membandingkan output:

```
test vector generator (libsignal):
  encrypt(data) → {type, body}
  → oktz-signal decrypt(body) → harus = data
  → libsignal decrypt(body) → harus = data

oktz-signal encrypt:
  encrypt(data) → {type, body}
  → libsignal decrypt(body) → harus = data
  → oktz-signal decrypt(body) → harus = data
```

### Test Vectors

1. **Session build** — X3DH dengan berbagai kombinasi: prekey ada, prekey none, signed prekey, identity key
2. **Ratchet encrypt/decrypt** — 100+ pesan sesi yang sama (ratchet ratchet step)
3. **Session archive** — simulate 5+ session archive, verify roundtrip
4. **Wire format** — PreKeyWhisperMessage encode/decode, bit-exact vs libsignal
5. **Session record** — JSON fixture dari produksi (kalau ada) → deserialize → haveOpenSession → serialize → roundtrip

### Oracle 3806 Curve

Test suite curve dari fase 1 (`node_modules/oktz-curve25519/tests/`) dijalankan ulang untuk memverifikasi `curve.rs` copy.

## Rollout

### Phase A: oktz-signal v0.1.0

1. `git init` repo `oktz-signal`, MIT license
2. Implementasi Rust crate + JS wrapper
3. Test oracle penuh: cross-interop, session roundtrip, 3806 curve
4. `npm publish`
5. Verifikasi: `npm install oktz-signal` di clean test → tes oracle jalan

### Phase B: oktz-baileys 9.1.0

1. `lib/Signal/libsignal.js`: ganti import `'libsignal'` → `'oktz-signal'`
2. `lib/Signal/Group/{group_cipher,sender-chain-key,sender-message-key}.js`: ganti import `'libsignal/src/crypto.js'` → `'oktz-signal'`
3. `package.json`: hapus `"libsignal": "^6.0.0"`, tambah `"oktz-signal": "^0.1.0"`
4. Test suite 104/104 pass
5. `npm version 9.1.0 && npm publish`

### Phase C: ourin-md

1. `npm update oktz-baileys` (pull 9.1.0)
2. `npm install` (oktz-signal transitif, libsignal dihapus)
3. Smoke test produksi: `.menu`, decrypt pesan nyata, encrypt balasan
4. Observasi 1-2 jam: log error rate, session migration

## Session Migration

Bot produksi punya session record di `auth.keys` dengan format libsignal. Saat Phase C:

- Session record lama (JSON libsignal) → Rust `session_deserialize` → SessionRecord Rust → operasi → `session_serialize` → JSON baru (format sama)
- Tidak ada format baru — output JSON sama dengan input. Session lama terbaca mulus.
- `haveOpenSession()` harus return false untuk session yang benar-benar expired (regenerasi dilakukan oleh baileys logic — `validateSession` → `{exists: false}` → fetch new prekey bundle)

## Out of Scope (Fase 1)

| Fitur | Alasan |
|-------|--------|
| numeric_fingerprint | Tidak dipakai baileys |
| keyhelper (generatePreKey, generateSignedPreKey) | Baileys pakai punya sendiri (generics.js) |
| KeyExchangeMessage proto | Tidak dipakai baileys |
| Multi-platform binary | YAGNI — produksi linux x64 |
| Async FFI (Rust async) | JS pegang IO, Rust pure sync |
| Group sender-key protocol rewrite | Sudah jalan, pakai crypto.js dari oktz-signal (node:crypto) |
| protobufjs dependency | Hand-written minimal codec di Rust, tidak perlu protobufjs |

## Risiko

| Risiko | Mitigasi |
|--------|----------|
| Session record format inkompatibel | Oracle test dengan fixture produksi. Forward compat via serde_json Value |
| Wire format bit berbeda | Cross-interop oracle dua arah |
| Performance regresi (Rust FFI overhead lebih besar dari JS? — Rust lebih cepat, tapi FFI boundary ada cost) | Acceptable — Double Ratchet per-pesan bukan hot path (1-2x per pesan, bukan per-byte) |
| Clean-room integrity — kontaminasi GPL | Proses: hanya spesifikasi publik + test oracle. Tidak ada baca kode libsignal saat implementasi. RustCrypto copyleft (LGPL) — oke untuk MIT (dynamically linked, bukan derivatif) |
| RustCrypto LGPL — dynamically linked napi-rs crate | RustCrypto crates LGPL-2.1+ — dynamic linking ke .node binary. LGPL mengizinkan penggunaan dari program Apache/MIT tanpa syarat copyleft. Aman. |

## Spesifikasi Referensi

- [X3DH Key Agreement Protocol](https://signal.org/docs/specifications/x3dh/)
- [Double Ratchet Algorithm](https://signal.org/docs/specifications/doubleratchet/)
- [Signal Protocol Session Management](https://signal.org/docs/specifications/sessions/)
- [Protobuf Encoding](https://protobuf.dev/programming-guides/encoding/) — varint, wire types
- [RFC 5869 — HMAC-based Extract-and-Expand Key Derivation Function (HKDF)](https://datatracker.ietf.org/doc/html/rfc5869)
- RustCrypto: `aes`, `hmac`, `sha2`, `hkdf` crates
- `x25519-dalek` crate (X25519 DH reference)