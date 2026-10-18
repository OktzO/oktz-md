//! `plugins/pushkontak/pushkontak.js` — push pesan ke semua member grup + auto simpan kontak VCF.
//!
//! `//! BLOKIRAN:` plugin ini tidak dapat di-port karena:
//!   1. `global.pushkontakSessions` / `global.statuspush` — state global antar
//!      pesan tidak tersedia lewat `Ctx` (hanya `sessions` per-kunci).
//!   2. `resolveAnyLidToJid`, `getCachedJid`, `isLidConverted` — resolusi LID
//!      tidak tersedia.
//!   3. `sock.user.id` — nomor bot sendiri tidak ada di trait `Sock`.
//!   4. `m.groupMetadata` — tersedia via `sock.metadata_grup`, namun `resolveParticipants`
//!      butuh LID resolusi.
//!   5. `nativeFlowMsg` / `interactiveButtons` / `quick_reply` — tidak ada di `Reply`.
//!   6. `sendVcf` — dokumen VCF ke `ownerJid` bisa via `Reply::Document`, namun
//!      alur push per-member dengan jeda tidak bisa direplikasi tanpa `relay_message`.
//!   7. `getAssetBuffer("foto")` — tidak ada di `Ctx`.
//!   8. Answer handler `pushkontakAnswerHandler` — tersedia via `#[answer_handler]`,
//!      namun butuh state sesi yang sama.
//!
//! Berkas ini dibuat sebagai catatan; implementasi ditunda sampai kapasitas
//! di atas tersedia.
//!
//! `ponytail:` saat state sesi global, resolusi LID, dan interactive buttons
//! tersedia, implementasikan ulang. `mod.rs` sudah memuat modul ini.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "pushkontak",
    alias = [
        "puskontak", "push", "stoppush", "setjedapush",
        "pushkontak_start", "kelolapush", "autovcf_on", "autovcf_off",
        "kodeunik_on", "kodeunik_off", "vcftarget_private", "vcftarget_group",
        "skipadmin_on", "skipadmin_off",
    ],
    category = "pushkontak",
    description = "Push pesan ke semua member grup + auto simpan kontak VCF — BLOKIRAN",
    usage = ".pushkontak",
    example = ".pushkontak",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false,
    is_owner = true
)]
pub fn pushkontak(ctx: Ctx) -> Reply {
    let _ = ctx;
    Reply::text(
        "❌ *PUSH KONTAK — TIDAK TERSEDIA*\n\n\
         Port Rust tertunda karena butuh:\n\
         • state sesi global antar pesan (`pushkontakSessions`)\n\
         • resolusi LID (`resolveAnyLidToJid`)\n\
         • `sock.user.id` (nomor bot sendiri)\n\
         • interactive buttons / native flow\n\
         • kirim dokumen VCF per member dengan jeda\n\n\
         Contoh JS: `.pushkontak`",
    )
}
