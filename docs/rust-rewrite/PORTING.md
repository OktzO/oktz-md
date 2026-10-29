# Cara port plugin JS → Rust

Satu plugin satu file: `plugins/<dir>/<nama>.js` → `rust/plugin/src/<dir>/<nama>.rs`.

## Aturan keras

1. **Satu plugin per file.** Jangan menggabung dua plugin JS ke satu file.
2. **Satu file per kategori.** `pub mod <nama>;` ditambahkan ke `src/<dir>/mod.rs`.
   Kalau `mod.rs` belum ada, buat.
3. **Tidak boleh ada helper lintas-plugin di file plugin.** Kalau dua plugin butuh
   helper yang sama, taruh di `src/common.rs` — tapi hanya kalau memang dipakai
   minimal dua plugin.
4. **Tidak boleh sentuh file milik orang lain.** Kalau dua agent bekerja paralel,
   masing-masing hanya boleh membuat file barunya sendiri.
5. **Tidak boleh menambah crate/dependency baru.**
6. **Komentar hanya seperlunya.** Hanya di kode produksi, hanya yang mencegah bug
   reintroduce. Test tidak perlu komentar. Panjang penjelasan → commit message.

## Bentuk file

```rust
//! `plugins/group/contoh.js` — <satu kalimat>.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::jid_number;

pub(crate) const NO_DATA: &str = "❌ Data tidak ada.";

#[plugin(
    name = "contoh",
    alias = ["cth"],
    category = "group",
    description = "Contoh plugin",
    usage = ".contoh <teks>",
    example = ".contoh halo",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn contoh(ctx: Ctx) -> Reply {
    Reply::text(NO_DATA)
}

#[cfg(test)]
mod tests { /* ... */ }
```

`#[plugin(...)]` WAJIB menyalin nilai dari `pluginConfig` di file JS. Nilai
yang tidak ada di JS berarti `false`, bukan `true`.

## Yang tersedia di `Ctx`

| JS | Rust |
|---|---|
| `m.text` | `ctx.text` |
| `m.args` | `ctx.args` (`Vec<String>`) |
| `m.sender` | `ctx.sender` |
| `m.chat` | `ctx.chat` |
| `m.mentionedJid` | `ctx.mentioned` |
| `m.command` | `ctx.command` |
| `m.prefix` | `ctx.prefix` |
| `m.quoted` | `ctx.quoted()` → `Option<&QuotedMessage>` |
| `m.group` / `isGroup` | `ctx.is_group` |
| `m.isOwner` | `ctx.is_owner` |
| `m.uptime` | `ctx.uptime_secs` |
| `m.groupMetadata.participants` | `ctx.roster.participants` |
| `bot admin?` | `ctx.roster.is_bot_admin` |
| `db.getUser(m.sender)` | `ctx.user_data` / `ctx.user_field("a.b")` / `ctx.user_list("k")` |
| `db.getUser(peserta)` | `ctx.roster.participants[i].data` |
| `db.getGroup(m.chat)` | `ctx.group_data` / `ctx.group_field("a.b")` / `ctx.group_list("k")` |
| `db.setting(k)` | `ctx.setting(k)` / `setting_bool(k, default)` / `setting_i64(k)` / `setting_list(k)` / `setting_entries(k)` |
| `db.setUser(jid, data)` | `Reply::….with_mutation(Mutation::SetUser { jid, data })` |
| `db.setGroup(jid, data)` | `Mutation::SetGroup { jid, data }` |
| `db.setting(k, v)` | `Mutation::SetSetting { key, v }` |
| `global.fooSessions` | `ctx.session_get("foo")` / `session_set` / `session_take` / `session_update` |
| `axios.get(url)` | `ctx.http.get(url)` → `Result<HttpResponse, HttpError>` |
| `axios({ method, url, data, headers })` | `ctx.http.call(&HttpCall::post(url).header(…).json(&v))` |
| `axios.get(url, { validateStatus: () => true })` | `HttpCall::….terima_semua_status()` |
| `m.isMedia` / `m.isImage` / `m.mimetype` / `m.fileName` | `ctx.media: Option<IncomingMedia>` (+ `ctx.is_media()`, `ctx.media_type()`, `ctx.is_media_kind(..)`) |
| `await m.download()` | `ctx.download_media()` → `Result<Vec<u8>, MediaError>` |
| `await m.quoted.download()` | `ctx.download_quoted_media()` |
| `m.react(emoji)` | `Reply::….with_reaction("emoji")` |
| `m.reply(teks, {mentions})` | `Reply::Text { text, mentions }` atau `Reply::TextQuoted` |

`ctx.mention_or_sender()` = `m.mentionedJid?.[0] || m.sender`.

`QuotedMessage` punya: `id`, `sender`, `chat`, `text`, `kind` (`QuotedKind`),
`mimetype`, `file_name`, `from_me`, plus `is_media()`. `QuotedKind` punya
`wire_type()` (`"imageMessage"` dll) dan `is_image/is_video/is_audio/
is_sticker/is_document/is_media`.

## Balasan

- `Reply::text(s)`
- `Reply::Text { text, mentions }` — `mentions` = JID penuh yang harus di-mention
- `Reply::TextQuoted { text, mentions }` — kalau JS pakai `m.reply` dengan quote
- `Reply::image(MediaSource::url(path), caption)` / `MediaSource::bytes(...)`
- `Reply::sticker(MediaSource::…)`
- `Reply::video(src, caption)` / `Reply::video_gif(src, caption)` — `type: 'video'`
- `Reply::audio(src)` / `Reply::voice_note(src)` — `type: 'audio'`, `ptt: true`
- `Reply::document(src, "nama.pdf")` — `type: 'document'`; tanpa `type` di JS
  berarti **document**, bukan image (`socket.js:756-820`)
- `Reply::Interactive { body }` — kartu WA
- `.with_reaction("✅")`, `.with_mutation(...)`, `.with_mutations(vec![...])`

`mutating` ditulis SEBELUM pesan dikirim oleh pipeline, jadi `.with_mutation()`
aman dipakai.

## Yang TIDAK tersedia

Kalau plugin butuh salah satu ini, **jangan port** — catat di laporan akhir:
`sock.*` (kirim/hapus/promote/demote/ubah nama grup/edit grup),
`m.download(filename)` (tulis berkas lokal), `fs`, `child_process`,
`sharp`/`ffmpeg`/`canvas`/`jimp`, `config.APIkey.*`, `config.saluran`,
`baileys` khusus (viewOnceMessage, ephemeral, poll vote), fungsi server.

`m.download()` **sudah ada** (`ctx.download_media()`); yang belum adalah versi
yang menulis ke `storage/temp` dan retry `sock.updateMediaMessage` — lihat
`BLOCKED.md` §1.

`Math.floor(Math.random()*N)` → `crate::common::roll(n)`.
`Math.random()` untuk pilih item → `crate::common::random_item(&[..])`.
Array `.length` → `.len()`. `.charAt`/`.slice` pada string → `.chars()`, `.len()`
Rust itu BYTE bukan UTF-16 — pakai `crate::common::jid_number` dll.

## Test

Tiap plugin WAJIB punya `#[cfg(test)] mod tests` yang menjalankan handler lewat
registry:

```rust
fn run(c: Ctx) -> Reply {
    (oktz_plugin_api::find("contoh").unwrap().handler)(c)
}
```

Minimal: 1 test meta (nilai `pluginConfig` tersalin benar), 1 test cabang
normal, 1 test cabang fallback/guard. Jangan pakai angka/jumlah plugin sebagai
assert — itu rapuh.