//! `plugins/owner/setwelcometype.js` — mengatur variant tampilan welcome message.
//!
//! `//! BLOKIRAN:` plugin ini tidak dapat di-port sepenuhnya karena:
//!   1. `sock.sendButton(...)` — tidak ada di trait `Sock` (hanya ada
//!      `metadata_grup`, `keluar_grup`, `kode_undangan`, dst).
//!   2. `getAssetBuffer("foto")` — tidak ada di `Ctx`; `config.assets.get("foto")`
//!      hanya mengembalikan path string, tidak buffer.
//!   3. `interactiveButtons` / `nativeFlowMessage` — tidak ada di `Reply`.
//!
//! Bagian yang *bisa* di-port (set/get `welcomeType` via `Mutation::SetSetting`,
//! daftar varian 1-8) tertunda sampai `sendButton` tersedia di `Sock`.
//!
//! `ponytail:` saat `Sock::send_button` dan kirim asset buffer tersedia,
//! implementasikan ulang dan tambahkan `pub mod set_welcome_type;`
//! di `src/owner/mod.rs` secara manual.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

#[plugin(
    name = "setwelcometype",
    alias = ["welcometype", "welcomevariant", "welcomestyle"],
    category = "owner",
    description = "Mengatur variant tampilan welcome message — BLOKIRAN",
    usage = ".setwelcometype",
    example = ".setwelcometype",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn setwelcometype(ctx: Ctx) -> Reply {
    let current = ctx
        .setting("welcomeType")
        .and_then(Value::as_i64)
        .unwrap_or(1);

    let variant = ctx.args.first().map(|s| s.to_lowercase());
    if let Some(v) = &variant {
        if v.len() >= 2 && v.starts_with('v') {
            let id_str = &v[1..];
            if let Ok(id) = id_str.parse::<i64>() {
                if (1..=8).contains(&id) {
                    let mut r = Reply::text(format!(
                        "✅ *WELCOME TYPE DIUBAH*\n\n*V{id}* — varian {id}\n_Deskripsi varian {id}_",
                    ));
                    r = r.with_mutation(Mutation::SetSetting {
                        key: "welcomeType".into(),
                        value: Value::from(id),
                    });
                    return r;
                }
            }
        }
    }

    Reply::text(format!(
        "👋🎨 *WELCOME TYPE*\n\n\
         Tipe aktif saat ini: *V{current}*\n\n\
         Port Rust tertunda karena butuh `sock.sendButton` dan `getAssetBuffer`.\n\
         Gunakan `.setwelcometype v1`..`v8` untuk mengubah (nanti tersedia).",
    ))
}
