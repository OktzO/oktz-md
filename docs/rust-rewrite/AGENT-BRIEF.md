# Briefing agent porting plugin

Baca ini dulu. Setelah itu baca `docs/rust-rewrite/PORTING.md` (kontrak
JS → Rust dan peta API `Ctx`/`Reply`).

## Tugas

Port daftar file JS yang kamu terima menjadi plugin Rust. Satu file JS = satu
file Rust dengan nama yang sama.

Acuan tunggal adalah file JS-nya. Kalau JS melakukan hal yang aneh, Rust
melakukan hal yang sama. Jangan memperbaiki, jangan menyederhanakan, jangan
menambah pesan sendiri.

## File yang boleh kamu buat

Hanya file `.rs` BARU di direktori kategori yang sesuai:

```
rust/crates/oktz-plugins/src/<kategori>/<nama>.rs
```

Kategori mengikuti folder JS asalnya (`plugins/ai/ai.js` →
`rust/crates/oktz-plugins/src/ai/ai.rs`). Kalau folder kategori belum ada,
buat foldernya.

## File yang TIDAK boleh kamu sentuh

- `crates/oktz-plugins/src/mod.rs` (memang tidak ada, tapi kalau ada: jangan)
- `crates/oktz-plugins/src/lib.rs` — milik orkestrator
- `crates/oktz-plugins/src/common.rs` — milik orkestrator
- File milik batch agent lain
- File JS asli

Kalau butuh helper baru yang dipakai beberapa plugin, taruh di file plugin
mu sendiri sebagai `pub(crate)`, dan sebutkan di laporan. Orkopestrator yang
memindahkannya ke `common.rs`.

## Bentuk plugin

```rust
use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "namaplagin",
    alias = ["alias1", "alias2"],
    category = "ai",
    description = "deskripsi dari JS",
    usage = ".namaplagin <argumen>",
    example = ".namaplagin contoh",
    cooldown = 5,
    energi = 1,
    is_private = false
)]
pub fn namaplagin(ctx: Ctx) -> Reply {
    let teks = ctx.args.join(" ");
    Reply::text(format!("hasil: {teks}"))
}
```

Nilai `description`, `usage`, `example`, `cooldown`, `energi`, `is_private`,
`is_owner`, `is_premium`, `is_group`, `is_admin` diambil dari file JS, bukan
ditebak. Kalau manifest punya entry untuk plugin itu, pakai
`docs/rust-rewrite/plugin-manifest.json` sebagai cek silang.

`category` = nama folder JS-nya.

## Balasan

- Teks: `Reply::text(...)`.
- Butuh kutipan pesan: `Reply::text_quoted(...)`.
- Butuh mention orang tertentu: `Reply::text_mention(teks, jid)`.
- Mengutip tapi tidak mau mention: `Reply::text_no_mention(teks)`.
- Reaksi: `Reply::text(teks).with_reaction("✅")`.
- Gambar/stiker: `Reply::image(caption)` / `Reply::sticker()` lalu sumber
  medianya lewat `.with_media(...)` — baca `PORTING.md` untuk bentuk
  pamungkasnya.
- Butuh HTTP: `ctx.http.get(url)` — baca `PORTING.md` §HTTP. Status di atas
  399 jadi `Err`, jadi selalu tangani hasilnya.
- Perlu simpan state: `ctx.sessions` dan `ctx.set(key, value)` — baca
  `PORTING.md`.

## Test

Setiap plugin wajib punya `mod tests` di file yang sama, minimal:

- kasus normal (input wajar menghasilkan balasan yang benar)
- kasus input aneh (kosong, sangat panjang, karakter aneh)

Test tidak boleh bergantung jaringan. Kalau plugin memanggil HTTP, test harus
memakai `Ctx` dengan `NoHttp` dan hanya menguji jalur gagalnya, atau menguji
fungsi murni yang sudah dipisah.

## Yang TIDAK boleh kamu lakukan

- Menjalankan `cargo`. Target directory dipakai agent lain secara paralel;
  orkestrator yang menjalankan test. Kamu menulis kode dan test-nya saja.
- Mengklaim test hijau kalau tidak kamu jalankan. Tulis laporan jujur:
  "test ditulis, belum dijalankan".
- Mengarang string tampilan. Kalau tidak ada di JS, tidak ada di Rust.
- Menulis komentar berbahasa Inggris. Komentar kode bahasa Indonesia, seperlunya.

## Toolchain (kalau kamu perlu lihat API Rust)

`export RUSTUP_HOME=/tmp/opencode/rust/rustup CARGO_HOME=/tmp/opencode/rust/cargo`
`export PATH="/tmp/opencode/rust/cargo/bin:$PATH"`
`export CARGO_TARGET_DIR=/tmp/opencode/rust/target`

Jangan jalankan cargo build/test (lihat di atas). Reader `grep`/ Read cukup.

## Laporan akhir

Tulis ringkas:
1. Daftar file yang dibuat.
2. Plugin mana yang tidak bisa diport dan alasannya (persis, Capability apa
   yang hilang).
3. Test yang ditulis.
4. Helper `pub(crate)` yang kamu tandemkan untuk dipindah orkestrator.