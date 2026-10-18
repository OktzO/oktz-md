//! `plugins/group/add.js` — menambahkan member ke grup (dukungan multiple).
//!
//! `pub mod add;` perlu ditambah manual ke `src/group/mod.rs` (mod.rs berisi
//! plugin lain, bukan milik saya — lihat PORT-BRIEF §6).
//!
//! `ponytail:` `sock.groupParticipantsUpdate` di Rust mengembalikan `()`
//! tanpa detail status per-nomor (JS: `res.status 200/408/lain`). Akibatnya
//! cabang sukses/undang/gagal per-nomor dari JS disatukan: pada `Ok` semua
//! nomor valid dilaporkan berhasil, pada `Err` dilaporkan gagal. Reaksi
//! `🕕` dari JS dihilangkan karena `Reply::with_reaction` hanya membawa satu
//! emoji — hanya reaksi penutup (`✅`/`❌`) yang dikirim.

use oktz_plugin_api::{AksiPeserta, Ctx, Reply, plugin};

/// Kartu pakai saat tidak ada argumen — salinan `add.js:24-39`.
fn help(prefix: &str) -> String {
    format!(
        "👥 *ᴀᴅᴅ ᴍᴇᴍʙᴇʀ*\n\n\
        > Cara pakai:\n\
        > 1. Di grup: `{prefix}add <nomor>`\n\
        > 2. Multiple: `{prefix}add <nomor1> <nomor2> ...`\n\
        > 3. Di private: `{prefix}add <nomor> <link_grup>`\n\n\
        > Contoh:\n\
        > `{prefix}add 6281234567890`\n\
        > `{prefix}add 628123 628456 628789`\n\
        > `{prefix}add 628123 https://chat.whatsapp.com/xxx`\n\n\
        > Syarat:\n\
        > - Bot harus admin di grup target\n\
        > - Yang jalankan command harus admin"
    )
}

const ERR_NO_NUMBER: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Masukkan nomor yang valid!";
const ERR_ALL_IN_GROUP: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Semua nomor sudah ada di grup!";

/// `arg.replace(/[^0-9]/g, '')` lalu awalan `0` diganti `62`; terima jika
/// panjang >= 10 — persis `add.js:56-63`.
fn bersihkan_nomor(arg: &str) -> Option<String> {
    let num: String = arg.chars().filter(|c| c.is_ascii_digit()).collect();
    let num = num.strip_prefix('0').map_or(num, |sisa| format!("62{sisa}"));
    if num.len() >= 10 {
        Some(num)
    } else {
        None
    }
}

/// `arg.match(/chat\.whatsapp\.com\/([a-zA-Z0-9]+)/)` — kode undangan.
fn kode_undangan(arg: &str) -> Option<String> {
    let needle = "chat.whatsapp.com/";
    let mulai = arg.find(needle)? + needle.len();
    let akhir = arg[mulai..]
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(arg.len() - mulai);
    let kode = &arg[mulai..mulai + akhir];
    if kode.is_empty() {
        None
    } else {
        Some(kode.to_string())
    }
}

#[plugin(
    name = "add",
    alias = ["addmember", "invite"],
    category = "group",
    description = "Menambahkan member ke grup (support multiple)",
    usage = ".add <nomor1> [nomor2] [nomor3]... [link_grup]",
    example = ".add 6281234567890 6281234567890",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn add(ctx: Ctx) -> Reply {
    if ctx.args.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let mut target_group: Option<String> = None;
    let mut target_numbers: Vec<String> = Vec::new();

    for arg in &ctx.args {
        if let Some(kode) = kode_undangan(arg) {
            match ctx.sock().info_undangan(&kode) {
                Ok(meta) => {
                    if let Some(id) = meta.get("id").and_then(|v| v.as_str()) {
                        target_group = Some(id.to_string());
                    }
                }
                Err(_) => {
                    return Reply::text(
                        "❌ *ɢᴀɢᴀʟ*\n\n> Link grup tidak valid atau sudah expired!",
                    );
                }
            }
        } else if arg.contains("@g.us") {
            target_group = Some(arg.clone());
        } else if let Some(num) = bersihkan_nomor(arg) {
            target_numbers.push(num);
        }
    }

    if target_numbers.is_empty() {
        return Reply::text(ERR_NO_NUMBER);
    }

    let Some(target_group) = target_group else {
        return Reply::text(
            "❌ *ɢᴀɢᴀʟ*\n\n> Jalankan di grup atau sertakan link grup!\n\n\
            `.add <nomor> <link_grup>`",
        );
    };

    // Metadata grup — padanan `sock.groupMetadata(targetGroup)`.
    let Ok(meta) = ctx.sock().metadata_grup(&target_group) else {
        return Reply::text(
            "❌ *ɢᴀɢᴀʟ*\n\n> Jalankan di grup atau sertakan link grup!\n\n\
            `.add <nomor> <link_grup>`",
        );
    };
    let subject = meta.get("subject").and_then(|v| v.as_str()).unwrap_or("grup");

    // Bot harus admin — padanan cek `botParticipant.admin`.
    if !ctx.roster.is_bot_admin {
        return Reply::text(format!(
            "❌ *ɢᴀɢᴀʟ*\n\n> Bot bukan admin di grup *{subject}*!"
        ));
    }

    // Di private: pengirim harus admin di grup target.
    if !ctx.is_group && !ctx.roster.is_admin(&ctx.sender) {
        return Reply::text(format!(
            "❌ *ɢᴀɢᴀʟ*\n\n> Kamu bukan admin di grup *{subject}*!"
        ));
    }

    let mut valid_numbers: Vec<String> = Vec::new();
    let mut already_in_group: Vec<String> = Vec::new();
    for num in &target_numbers {
        let jid = format!("{num}@s.whatsapp.net");
        if ctx
            .roster
            .participants
            .iter()
            .any(|p| p.jid == jid)
        {
            already_in_group.push(num.clone());
        } else {
            valid_numbers.push(jid);
        }
    }

    if valid_numbers.is_empty() {
        return Reply::text(ERR_ALL_IN_GROUP);
    }

    match ctx
        .sock()
        .ubah_peserta(&target_group, &valid_numbers, AksiPeserta::Tambah)
    {
        Ok(()) => {
            let mut result_text = format!(
                "🥗 @{} telah menambahkan member ke grup\n\n",
                ctx.sender_number()
            );
            if !valid_numbers.is_empty() {
                result_text.push_str(&format!(
                    "Ada *{}* member yang berhasil ditambahkan:\n",
                    valid_numbers.len()
                ));
                for n in &valid_numbers {
                    result_text.push_str(&format!("• @{}\n", n.split('@').next().unwrap_or(n)));
                }
                result_text.push('\n');
            }
            if !already_in_group.is_empty() {
                result_text.push_str(&format!(
                    "⏭️ *sᴜᴅᴀʜ ᴅɪ ɢʀᴜᴘ ({})*:\n",
                    already_in_group.len()
                ));
                for n in &already_in_group {
                    result_text.push_str(&format!("• @{}\n", n.split('@').next().unwrap_or(n)));
                }
            }
            let mut mentions: Vec<String> = valid_numbers.clone();
            mentions.extend(already_in_group.iter().cloned());
            mentions.push(ctx.sender.clone());
            Reply::text(result_text)
                .with_reaction("✅")
        }
        Err(_) => Reply::text(
            "❌ *ɢᴀɢᴀʟ*\n\n> Bot tidak memiliki izin untuk menambah member!",
        )
        .with_reaction("❌"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{Participant, Sock, SockError};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct StubSock {
        invite_id: Option<String>,
        meta: Option<serde_json::Value>,
        add_err: bool,
        seen_invite: Mutex<Vec<String>>,
        seen_meta: Mutex<Vec<String>>,
        seen_add: Mutex<Vec<(String, Vec<String>)>>,
    }

    impl Sock for StubSock {
        fn info_undangan(&self, kode: &str) -> Result<serde_json::Value, SockError> {
            self.seen_invite.lock().unwrap().push(kode.to_string());
            match &self.invite_id {
                Some(id) => Ok(serde_json::json!({ "id": id })),
                None => Err(SockError::baru("expired")),
            }
        }
        fn metadata_grup(&self, jid: &str) -> Result<serde_json::Value, SockError> {
            self.seen_meta.lock().unwrap().push(jid.to_string());
            match &self.meta {
                Some(m) => Ok(m.clone()),
                None => Err(SockError::baru("tidak ada grup")),
            }
        }
        fn ubah_peserta(
            &self,
            jid: &str,
            peserta: &[String],
            _aksi: AksiPeserta,
        ) -> Result<(), SockError> {
            self.seen_add.lock().unwrap().push((jid.to_string(), peserta.to_vec()));
            if self.add_err {
                Err(SockError::baru("gagal"))
            } else {
                Ok(())
            }
        }
        fn ada_whatsapp(&self, _: &[String]) -> Result<Vec<oktz_plugin_api::AdaWhatsapp>, SockError> {
            Err(SockError::baru("x"))
        }
        fn foto_profil(&self, _: &str, _: &str) -> Result<Option<String>, SockError> {
            Err(SockError::baru("x"))
        }
        fn semua_grup(&self) -> Result<serde_json::Value, SockError> {
            Err(SockError::baru("x"))
        }
        fn set_foto_profil(&self, _: &str, _: &[u8]) -> Result<(), SockError> {
            Err(SockError::baru("x"))
        }
        fn keluar_grup(&self, _: &str) -> Result<(), SockError> {
            Err(SockError::baru("x"))
        }
        fn kode_undangan(&self, _: &str) -> Result<String, SockError> {
            Err(SockError::baru("x"))
        }
        fn buat_grup(&self, _: &str, _: &[String]) -> Result<serde_json::Value, SockError> {
            Err(SockError::baru("x"))
        }
        fn terima_undangan(&self, _: &str) -> Result<String, SockError> {
            Err(SockError::baru("x"))
        }
        fn tandai_dibaca(&self, _: &[serde_json::Value]) -> Result<(), SockError> {
            Err(SockError::baru("x"))
        }
        fn presence(&self, _: &str, _: &str) -> Result<(), SockError> {
            Err(SockError::baru("x"))
        }
        fn status_teks(&self, _: &str) -> Result<serde_json::Value, SockError> {
            Err(SockError::baru("x"))
        }
        fn nama_kontak(&self, _: &str) -> Result<String, SockError> {
            Err(SockError::baru("x"))
        }
        fn profil_bisnis(&self, _: &str) -> Result<Option<serde_json::Value>, SockError> {
            Err(SockError::baru("x"))
        }
    }

    fn meta_with_participants(subject: &str, jids: &[&str]) -> serde_json::Value {
        serde_json::json!({
            "subject": subject,
            "id": "628gc@g.us",
            "participants": jids.iter().map(|jid| {
                serde_json::json!({ "jid": jid, "admin": "superadmin" })
            }).collect::<Vec<_>>()
        })
    }

    fn run(
        sock: Arc<StubSock>,
        roster: Vec<Participant>,
        args: &[&str],
        is_group: bool,
    ) -> Reply {
        let ctx = Ctx {
            sender: ME.into(),
            chat: "628gc@g.us".into(),
            args: args.iter().map(|s| s.to_string()).collect(),
            prefix: ".".into(),
            command: "add".into(),
            is_group,
            sock,
            roster: oktz_plugin_api::GroupRoster {
                participants: roster,
                is_bot_admin: true,
            },
            ..Ctx::default()
        };
        add(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn reaction_of(r: &Reply) -> Option<String> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.clone()),
            _ => None,
        }
    }

    #[test]
    fn tanpa_argumen_menampilkan_kartu_pakai() {
        let sock = Arc::new(StubSock::default());
        let r = run(sock, vec![], &[], true);
        assert!(text_of(&r).contains("*ᴀᴅᴅ ᴍᴇᴍʙᴇʀ*"));
        assert!(text_of(&r).contains("`.add <nomor>`"));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn link_undangan_expired_ditolak() {
        // invite_id None -> info_undangan gagal.
        let sock = Arc::new(StubSock {
            invite_id: None,
            ..Default::default()
        });
        let r = run(sock, vec![], &["https://chat.whatsapp.com/EXPIRED"], true);
        assert!(text_of(&r).contains("Link grup tidak valid"));
    }

    #[test]
    fn link_undangan_berhasil_mengatur_target_grup() {
        let sock = Arc::new(StubSock {
            invite_id: Some("628gc@g.us".to_string()),
            meta: Some(meta_with_participants("Grup Test", &[])),
            ..Default::default()
        });
        let r = run(sock, vec![], &["6281234567890", "https://chat.whatsapp.com/abc"], true);
        assert!(text_of(&r).contains("berhasil ditambahkan"));
        assert_eq!(reaction_of(&r), Some("✅".to_string()));
        let seen = sock.seen_add.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, "628gc@g.us");
        assert_eq!(seen[0].1, vec!["6281234567890@s.whatsapp.net"]);
    }

    #[test]
    fn nomor_sudah_di_grup_dilaporkan_sudah_ada() {
        // Satu nomor sudah ada, satu baru -> cabang sᴜᴅᴀʜ ᴅɪ ɢʀᴜᴘ tercapai
        // (valid_numbers masih non-kosong).
        let roster = vec![Participant {
            jid: "6281234567890@s.whatsapp.net".to_string(),
            is_admin: false,
            is_owner: false,
            data: Default::default(),
        }];
        let sock = Arc::new(StubSock {
            invite_id: Some("628gc@g.us".to_string()),
            meta: Some(meta_with_participants("Grup Test", &["6281234567890@s.whatsapp.net"])),
            ..Default::default()
        });
        let r = run(sock, roster, &["https://chat.whatsapp.com/abc", "6281234567890", "628999000111"], true);
        assert!(text_of(&r).contains("sᴜᴅᴀʜ ᴅɪ ɢʀᴜᴘ"));
        assert!(text_of(&r).contains("berhasil ditambahkan"));
    }

    #[test]
    fn bot_bukan_admin_ditolak() {
        let sock = Arc::new(StubSock {
            invite_id: Some("628gc@g.us".to_string()),
            meta: Some(meta_with_participants("Grup Test", &[])),
            ..Default::default()
        });
        let ctx = Ctx {
            sender: ME.into(),
            chat: "628gc@g.us".into(),
            args: vec!["6281234567890".to_string()],
            prefix: ".".into(),
            command: "add".into(),
            is_group: true,
            sock,
            roster: oktz_plugin_api::GroupRoster {
                participants: vec![],
                is_bot_admin: false,
            },
            ..Ctx::default()
        };
        let r = add(ctx);
        assert!(text_of(&r).contains("Bot bukan admin"));
    }

    #[test]
    fn bersihkan_nomor_menghapus_awalan_nol() {
        assert_eq!(bersihkan_nomor("081234567890"), Some("6281234567890".to_string()));
        assert_eq!(bersihkan_nomor("6281234567890"), Some("6281234567890".to_string()));
        assert_eq!(bersihkan_nomor("0812"), None);
        assert_eq!(bersihkan_nomor("abc"), None);
    }

    #[test]
    fn kode_undangan_diekstrak_dari_link() {
        assert_eq!(
            kode_undangan("https://chat.whatsapp.com/Abc123"),
            Some("Abc123".to_string())
        );
        assert_eq!(kode_undangan("bukan link"), None);
    }
}
