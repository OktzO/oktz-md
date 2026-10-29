# oktz-rs

Inti bot WhatsApp `ourin-md` versi Rust. Plugin, format balasan, dan perilaku
mengikuti bot JS produksi — JS adalah acuan, bukan sumber inspirasi.

## Isi repo

- `src/` + `lib/*` + `plugin/` — binary, crate pendukung, dan plugin (workspace cargo).
- `config.json` — konfigurasi ala `config.js` (pairing, owner, Turso, API key).
- `launcher.js` — launcher Node.js untuk panel Pterodactyl egg Node.js.
- `assets/`, `data/`, `test/`, `tests/` — aset, data, dan test.

## Build

```bash
cargo build --release     # binary: target/release/oktz
```

## Konfigurasi

Semua yang dulu hardcode di JS kini di `config.json` (sample ikut dikirim).
Aturan timpa: nilai yang **diisi** di file menang atas `.env`; string kosong
berarti "jangan sentuh", jadi API key di `.env` tidak mati karena file contoh
dibiarkan kosong. Lokasi file bisa dipindah lewat `OKTZ_CONFIG`.

Yang bisa diatur: `owner` (nama + nomor), `bot`, `info`, `session`
(`pairingNumber`, `usePairingCode`, QR), `database`, `turso`, `sticker`,
`saluran`, `apiKeys`.

## Pairing

```bash
./target/release/oktz pair          # code/QR mengikuti config.json
./target/release/oktz pair-code     # nomor dari session.pairingNumber
./target/release/oktz pair-code 628xxx   # nomor dari argumen
./target/release/oktz pair-qr
```

Hasil pairing tersimpan sebagai creds; setelah itu jalankan mode produksi:

```bash
./target/release/oktz run            # daemon: jalan sampai SIGINT/SIGTERM
./target/release/oktz run ../storage/session/creds.json settings.json 60   # uji 60 detik
```

Sesi ratchet disimpan di `database/wa/oktz-run.db` dan dipakai ulang selama
sidik creds-nya cocok — restart tidak memaksa pairing ulang.

## Panel Pterodactyl (egg Node.js)

Panel memanggil `node launcher.js`; launcher menjalankan binary Rust sebagai
child process, meneruskan stdio dan sinyal, dan memakai exit code anak.

```
cargo build --release
node launcher.js            # sama dengan `oktz run`
node launcher.js pair       # pairing
```

Binary bisa dipindah lewat `OKTZ_BIN`.

## Test

```bash
cargo test --release
cargo clippy --release --all-targets
```
