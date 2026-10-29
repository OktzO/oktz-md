# Sistem pesan kaya (kartu native-flow & tabel AI)

Acuan: bot JS produksi di `ourin-md`. Semua bentuk di bawah disalin dari
`src/lib/rich-messages.js`, `src/lib/socket.js`, `src/lib/builder.js`, dan
pemakaian nyata di `plugins/`.

## Ringkasan

| Kemampuan | Model (plugin) | Render (oktz-wa) | Status |
|---|---|---|---|
| Kartu native-flow (body, footer, tombol) | `oktz_plugin_api::kartu::Kartu` | `oktz-wa/src/cards.rs` | ada |
| Header kartu + gambar | `KartuHeader { title, subtitle, media }` | upload di `MessageSender::send_card` | ada |
| Tombol native-flow | `Tombol { aksi, params }` | `AksiTombol` -> `NativeFlowButton` | ada |
| `messageParamsJson` (bottom_sheet, limited_time_offer) | `Kartu::params` | `NativeFlowMessage.message_params_json` | ada |
| `contextInfo` (mention, quote, forwarded) | `Kartu::mentions/quote/forwarded/...` | `kartu_context_info` | ada |
| Teks cadangan saat kartu gagal | `Kartu::fallback_teks` | `Pipeline::deliver` | ada |
| Tabel AI (`richResponseMessage`) | `oktz_plugin_api::tabel::Tabel` | `oktz-wa/src/tabel.rs` | ada |
| `AIRich` v2 (`unifiedResponse`, `botMetadata`) | — | — | belum (JS `sendTableV2`) |
| `carouselMessage`, `templateMessage` | — | — | belum |

## Bentuk yang WA benar-benar render

1. `interactiveMessage` dibungkus `viewOnceMessage` — field itu bertipe
   `FutureProofMessage`, bukan `ViewOnceMessage` (`WAProto.proto:2095`).
2. `messageContextInfo` memuat `deviceListMetadata` +
   `deviceListMetadataVersion: 2`.
3. Pesan dikirim bersama extra stanza node
   `biz > engagement + interactive(type=native_flow,v=1) > native_flow(v=9,name=mixed)`
   (`onigis/lib/WABinary/generic-utils.js:139`).

Tanpa nomor 3, relay sukses tapi kartu tidak muncul di chat. Bentuk legacy
(`ListMessage`, `interactiveButtons` telanjang) WA buang diam-diam.

Tabel AI (`sock.sendTable`) berbeda: dikirim sebagai `botForwardedMessage` →
`richResponseMessage` dengan submessages (teks opsional, `tableMetadata`,
teks opsional) dan TANPA node `biz`.

## Pakai dari plugin

```rust
use oktz_plugin_api::{Ctx, Reply, Tombol, KartuHeader, Daftar, Bagian, Baris, MediaSource, plugin};

let daftar = Daftar::baru("🍃 List Menu")
    .bagian(
        Bagian::baru("Berikut adalah pilihan nya (2 kategori)")
            .baris(Baris::baru("🏠 main", ".menucat main").description("12 command")),
    )
    .icon("DEFAULT");

Reply::interactive("Isi kartu")
    .with_footer("🍔 Silahkan pilih dari salah satu tombol di bawah")
    .with_header(
        KartuHeader::baru()
            .judul("")
            .subjudul("Oktz Bot")
            .media(MediaSource::bytes(ASET_FOTO.to_vec()).with_mimetype("image/jpeg")),
    )
    .with_button(Tombol::pilih(&daftar))
    .with_button(Tombol::balas_cepat("🧀 Owner", ".owner"))
    .with_mentions(vec![ctx.sender.clone()])
    .with_fallback_teks(teks_menu)
```

Tombol yang tersedia (`AksiTombol`):

| Aksi | String WA | Constructor |
|---|---|---|
| BalasanCepat | `quick_reply` | `Tombol::balas_cepat(label, id)` |
| PilihSatuan | `single_select` | `Tombol::pilih(&Daftar)` |
| BukaUrl | `cta_url` | `Tombol::buka_url(label, url)` |
| Salin | `cta_copy` | `Tombol::salin(label, id, copy_code)` |
| Telepon | `cta_call` | `Tombol::telepon(label, nomor)` |
| Pengingat | `cta_reminder` | `Tombol::baru(AksiTombol::Pengingat, params)` |
| BatalPengingat | `cta_cancel_reminder` | `Tombol::baru(..)` |
| Alamat | `address_message` | `Tombol::baru(..)` |
| Lokasi | `send_location` | `Tombol::baru(..)` |

Batas WA: native-flow maks 6 tombol (grid 2x3), `single_select` maks 3 baris
x 2 kolom.

## Jalur kirim

1. `render_reply` (oktz-wa) mengubah `Reply::Interactive` jadi
   `Rendered { card, card_header_media, extra_nodes: [biz] }`. Proto belum
   dirakit karena media header harus di-upload dulu.
2. `Pipeline::deliver` memanggil `MessageSender::send_card`.
3. `ClientSender::send_card` meng-upload gambar header (`Client::upload`),
   merakit `interactiveMessage`, lalu mengirim dengan node `biz`.
4. Kalau kirim gagal dan `Kartu::fallback_teks` ada, teks cadangan dikirim
   (pola `menu.js:1379`).

## Balasan tombol (interactive reply)

Tombol yang ditekan masuk sebagai `interactiveResponseMessage`. Rust sudah
membacanya di `oktz-router/src/serialize.rs` (`MessageKind::InteractiveResponse`)
dan `ctx.text` berisi id tombol/row yang dipilih — persis precedence JS
`getMessageBody` (`src/lib/serialize.js:327`).

## Plugin yang sudah memakai

- `main/menu` — kartu varian 2 (header gambar + `single_select` kategori +
  tombol Owner/Allmenu).
- `main/menucat` — kartu per kategori (varian 2).
- `main/allmenu` — kartu semua command per kategori.
- `main/sc` — kartu `cta_url` ke repo.
- `owner/setmenu` — menyimpan setting `menuVariant` (v2/v3) lewat
  `Mutation::SetSetting`; `main/menu` membacanya.
- `main/ping2`, `main/stats`, `main/totalfitur` — tabel AI.
- `sticker/brat`, `search/nerdfont`, `tools/styleteks`, `search/*` — kartu
  `single_select`/`cta_copy`.
