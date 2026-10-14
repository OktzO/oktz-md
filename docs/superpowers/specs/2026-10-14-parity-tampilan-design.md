# Parity Tampilan 1:1 (WhatsApp + Terminal)

Status: menunggu persetujuan manusia. Tanggal: 2026-10-14.
Reviewer: sudah dibaca builder-check; semua butir yang ia angkat terjawab di
§9 (dari mandat "JS yang menang", bukan dari pertanyaan baru).

## 1. Intent

Bot JS `ourin-md` sudah dipakai user nyata. Port ke Rust hanya boleh mengganti
**bahasa**, bukan hasil yang dilihat user. Kalau ada user yang membandingkan
chat dari bot JS dengan chat dari bot Rust dan menemukan perbedaan satu
karakter pun pada tampilan, port itu gagal — itu keluhan, bukan detail.

Dua tampilan yang dimaksud user:

1. **Di dalam WhatsApp** — apa yang bot kirim: teks, `quoted`, `mentions`,
   `contextInfo`, `footer`, kartu interactive/tombol/template, gambar/stiker
   yang dibungkus.
2. **Di terminal** — apa yang muncul saat bot jalan: banner boot, tag kotak
   `[  OK  ]`, baris command masuk, baris plugin, status koneksi, error,
   spinner.

Logika boleh dirombak jadi lebih efisien; yang tidak boleh berubah adalah
byte yang dilihat mata.

## 2. Goals / Non-goals / Anti-goals

**Goals**

- Setiap plugin yang ter-port menampilkan output identik dengan versi JS.
- Baris terminal yang dilihat user identik dengan `src/lib/logger.js`.
- Bukti parity bisa dijalankan ulang, bukan klaim.

**Non-goals**

- Menambah fitur, plugin, atau perintah baru.
- Menata ulang arsitektur Rust yang tidak kelihatan user.

**Anti-goals (bentuk kegagalan yang harus dihindari)**

- "Agak mirip", "menyederhanakan", "lebih rapi dari JS" → gagal.
- Menempelkan helper tampilan baru yang tidak ada di JS, walau lebih bagus.
- Membiarkan teks demo/placeholder lolos ke user. Contoh yang sudah ada:
  `cards.rs` masih memakai teks demo.

## 3. Acuan

Sumber kebenaran adalah kode JS yang berjalan produksi. Daftar file di bawah
ini bersifat tertutup: file di luar daftar ini bukan acuan untuk tampilan,
kecuali disebut eksplisit di §4.

| Tampilan | Sumber JS | Padanan Rust sekarang |
|---|---|---|
| Balasan plugin | `src/handler.js`, `src/lib/message.js` | `oktz-wa/src/pipeline.rs` (`rendered`, `deliver`) |
| Bentuk pesan & varian balasan | `src/lib/serialize.js` | `oktz-router/src/serialize.rs` |
| Format teks | `src/lib/formatter.js` | belum ada sebagai helper |
| Kartu/tombol | `src/lib/rich-messages.js`, `src/lib/builder.js`, `src/lib/socket.js` | `oktz-wa/src/cards.rs` (masih teks demo) |
| Izin/tenang/energi | `src/lib/middleware.js`, `src/handler.js` | `oktz-router/src/permission.rs`, `pipeline.rs` (teks hardcode) |
| Teks konfigurasi | `config.js`, `src/lib/context.js`, `src/lib/time.js` | `oktz-core::config` (belum ada nama bot/website/pesan) |
| Terminal | `src/lib/logger.js`, `src/connection.js`, `src/index.js` | `tracing_subscriber` |

Kalau JS dan Rust berbeda, **Rust yang mengikuti JS** — termasuk kalau yang
diikuti itu aneh atau keliru.

## 4. Bentuk pekerjaan

### 4.1 Default balasan (ikut JS, tanpa tanya)

`serialize.js` secara default mengutip pesan pemicu pada setiap balasan
(`quotedMsg = options.quoted !== false ? msg : undefined`) dan menyebut
pengirim lewat `mentionedJid` (`options.mentions || [sender] || []`; `[]`
bernilai benar di JS, jadi plugin yang mengirim `mentions: []` berarti tanpa
mention). Akibatnya balasan WA adalah `extendedTextMessage` + `contextInfo`,
bukan `conversation` polos.

Rust sekarang mengirim `conversation` tanpa kutipan untuk hampir semua plugin.
Jadi ini harus berubah: `Reply::text` jadi mengutip dan menyebut pengirim
kecuali plugin menyatakan lain. Ini menyentuh jalur render dan seluruh plugin
ter-port — memang itu inti pekerjaan parity.

### 4.2 Varian balasan

`serialize.js` memilih satu dari 11 bentuk balasan dari setting runtime
`replyVariant` (default 1 bila setting hilang; nilai yang aktif di deployment
sekarang **9**). Bentuknya berbeda jauh: varian 2/6 mengirim dokumen, 3 video,
4 dan 11 menambahkan `config.info.website` di depan teks, 5/8/9 membuat
kutipan `orderMessage`, 10 `requestPaymentMessage`, 7 interactive viewOnce.

Semua 11 varian di-port. Varian 9 (yang aktif) dikerjakan lebih dulu.

### 4.3 Kartu dan interactive — tiga jalur, tiga oracle

Tidak ada satu jalur saja; produksi memakai tiga bentuk yang berbeda dan
tiap-tiapnya punya acuan sendiri:

| Jalur | Acuan JS | Pemakai |
|---|---|---|
| `sendButton` → `viewOnceMessage` + `deviceListMetadata` v2 + node `biz/native_flow`, `footer` dihilangkan kalau kosong | `src/lib/socket.js:358-421` | 18 plugin; `cards.rs` sudah mengikutinya |
| `Button`/`ButtonV2`/`Carousel` → `interactiveMessage`/`buttonsMessage`/`carouselMessage`, tanpa pembungkus viewOnce, `footer.text` selalu ada | `src/lib/builder.js:751-791` | 2 plugin (`ai.js`, `brat.js`) |
| `generateMenuCard`/tabel/`AIRich` → `botForwardedMessage.richResponseMessage` dengan `unifiedResponse` base64 | `src/lib/rich-messages.js` | 7 plugin termasuk `/menu` |

Ketiganya di-port ke oracle masing-masing, bukan digabung satu bentuk.
Tipe-tipe yang dibutuhkan sudah ada di `waproto` 0.7 (`whatsapp.proto` memuat
`botForwardedMessage`, `AIRichResponseMessage`, `carouselMessage`), jadi tidak
perlu fork encoder.

### 4.4 Helper tampilan bersama

Fungsi murni dari `formatter.js`, `serialize.js`, `rich-messages.js`, dan
`builder.js` di-port ke satu modul Rust dengan nama fungsi yang sama; argumen
dan nilai balik mengikuti JS.

**Pengecualian yang disengaja: format angka tidak diseragamkan.** Tidak ada
satu fungsi angka di JS yang dipakai semua plugin, dan tiap plugin punya
bentuk sendiri (`buyenergi.js` separator biasa, `addenergi.js`/`delenergi.js`
menghasilkan `∞ Unlimited` untuk -1, `leaderboard.js` K/M/B/T, `profile.js`
`toLocaleString("id-ID")`, `tiktokdl.js` satu desimal M/K). Menyeragamkan
mereka akan mengubah angka yang dibaca user. Helper bersama hanya untuk yang
benar-benar identik di JS.

### 4.5 Teks konfigurasi

Nama bot, website, newsletter, dan pesan penolakan (`ownerOnly`, `premiumOnly`,
`energiExceeded`, `banned`, `groupOnly`, `adminOnly`, `botAdminOnly`) diambil
dari `config.js` ke satu tempat di Rust dengan key yang sama. Plugin tidak
lagi menulis `BOT_NAME`/`WEBSITE` sendiri. Teks yang sekarang hardcode di
`permission.rs` dan `pipeline.rs` diganti nilai dari `config.js`.

### 4.6 Terminal

Port `logger.js` ke modul log baru (`oktz-core::jslog`) yang menulis baris
dengan bentuk sama: `makeTag`, `writeLog`, `logCommand`, `logPlugin`,
`logConnection`, `logErrorBox`, `printBanner`, `playBootSequence`, spinner,
`typeLine`. Yang direproduksi adalah **setiap call site yang benar-benar
dijalankan JS di produksi**, termasuk baris per batch masuk
(`logger.debug("upsert"…)`) dan baris prekey/turso/rate-limit/join grup — bukan
lima kategori besar saja.

Baris JSON `{"phase":…}` milik `oktz-bin` (alat smoke dan skrip) tetap ke
stdout dan tidak digabung dengan `jslog`.

`tracing` boleh tetap ada untuk log internal, tetapi tidak boleh menjadi
sumber tunggal baris yang dilihat user.

### 4.7 Plugin

Tiap plugin ter-port dibandingkan satu-satu dengan file JS-nya; yang berbeda
diperbaiki ke arah JS. Test Rust yang sengaja mengunci perilaku berbeda dari JS
ditulis ulang — JS menang.

## 5. Verifikasi

1. **Golden byte dari JS.** Fungsi tampilan JS dijalankan dengan Node di
   `ourin-md` untuk banyak input, hasilnya disimpan sebagai fixture; test Rust
   membandingkan byte keluaran Rust untuk input yang sama. Fungsi yang
   memakai `randomUUID`, `randomBytes`, `Date.now`, atau `Math.random`
   (`generateMenuCard`, `generateTableContent*`, `generateLinkContent*`,
   `prepareRichMessageContent`, `wrapToBotForwardedMessage`, `AIRich.build`,
   `createContextInfo`, varian 5/8/9) dibandingkan setelah bagian itu
   dinormalkan; daftar fungsi yang dinormalkan ditulis eksplisit di fixture.
2. **Per plugin.** Balasan tiap plugin ter-port dibandingkan dengan hasil Node
   atas `Ctx` yang sama, memakai HTTP tersimpan untuk panggilan jaringan.
   Layar pertama yang dikerjakan: `menu`, `ping`, `owner`, `profile`,
   `leaderboard`, `buyenergi`, `addenergi`, `delenergi`, `ai`, `brat`.
3. **Spot-check live** memakai device WA asli. Jalur ini sudah terbukti
   bekerja: creds + 527 sesi ratchet dari Turso, dan pesan masuk ter-decrypt
   jadi plaintext. Hasil kiriman dibandingkan dengan fixture lapis 1–2.
4. `cargo clippy --release --all-targets` tanpa warning dan
   `cargo test --release` hijau.

## 6. Batasan yang diketahui

- 485 dari 828 entri manifest belum punya padanan Rust. Paritas berlaku untuk
  yang ter-port (343 plugin terdaftar di `oktz boot`); porting lanjutan tunduk
  pada kontrak yang sama.
- `handler.js`, `builder.js`, `rich-messages.js`, dan `group-protection.js`
  belum ter-port utuh. Selama belum ada, output yang bergantung padanya harus
  terlihat jelas belum jadi, bukan dianggap selesai.
- Toolchain media (canvas/ffmpeg) belum ada; plugin yang butuh itu tidak
  bisa dibuktikan parity-nya.

## 7. Keputusan builder

- Nama modul helper dan crate tempatnya diletakkan.
- Format fixture golden di `rust/crates/*/tests/fixtures/`.
- Nama fungsi dan tanda tangan mengikuti JS; jika JS punya parameter yang
  tidak pernah dipakai, parameter itu tetap ada.
- Fixture Node dijalankan dengan warna dipaksa aktif dan stdout bukan TTY
  didefinisikan sebagai pemeriksaan waktu tulis, mengikuti `chalk`.
- Waktu memakai `Asia/Jakarta`; nama bulan/hari id-ID memakai tabel Rust
  yang diverifikasi terhadap keluaran Node.

## 8. Yang masih perlu diputuskan manusia

Tidak ada. Semua keputusan substantif sudah diucapkan: parity 1:1, dua
permukaan tampilan, logika bebas efisien, JS menang saat berbeda, dan test
Rust yang mengunci perilaku berbeda dari JS boleh ditulis ulang.