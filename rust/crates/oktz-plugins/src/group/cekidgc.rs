//! `plugins/group/cekidgc.js` — cek ID dan info lengkap grup.
//!
//! `//! BLOKIRAN:` JS membangun `interactiveMessage` lengkap (header media PP
//! yang di-resize, `nativeFlowMessage` tombol, `contextInfo` newsletter) lewat
//! `onigis` + `sock.relayMessage`. API `Reply` Rust tidak punya variant itu,
//! jadi hanya teks `infoText` yang dikirim; tombol interaktif dan header PP
//! diabaikan. Teks balasan sama persis dengan JS.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::{json, Value};

const BOT_NAMA_DEFAULT: &str = "Bot";

#[plugin(
    name = "cekidgc",
    alias = ["idgc", "idgrup", "groupid"],
    category = "group",
    description = "Cek ID dan info lengkap grup",
    usage = ".cekidgc [link grup]",
    example = ".cekidgc https://chat.whatsapp.com/xxxxx",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_enabled = true
)]
pub fn cekidgc(ctx: Ctx) -> Reply {
    let input = ctx.text.trim();

    let (group_jid, group_meta) = match cari_grup(&ctx, input) {
        Some(pasangan) => pasangan,
        None => {
            return balasan_tidak_ditemukan(&ctx.prefix, "✘");
        }
    };

    let group_name = group_meta
        .get("subject")
        .and_then(Value::as_str)
        .unwrap_or("Unknown")
        .to_string();
    let participants = group_meta
        .get("participants")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or(Value::Array(Vec::new()));
    let member_count = participants.as_array().map(|a| a.len()).unwrap_or(0)
        .max(group_meta.get("size").and_then(Value::as_u64).unwrap_or(0) as usize);
    let admin_count = participants
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|p| {
                    p.get("admin").and_then(Value::as_str).map_or(false, |a| {
                        a == "admin" || a == "superadmin"
                    })
                })
                .count()
        })
        .unwrap_or(0);
    let group_owner = group_meta
        .get("owner")
        .or_else(|| group_meta.get("subjectOwner"))
        .and_then(Value::as_str)
        .unwrap_or("—");
    let group_owner_tanpa_server = group_owner.split('@').next().unwrap_or(group_owner);
    let created_at = format_date(group_meta.get("creation").and_then(Value::as_i64).unwrap_or(0));
    let group_desc = group_meta.get("desc").and_then(Value::as_str).unwrap_or("—");
    let desc_preview = if group_desc.len() > 120 {
        format!("{}…", &group_desc[..120])
    } else {
        group_desc.to_string()
    };
    let is_restrict = if group_meta.get("restrict").and_then(Value::as_bool).unwrap_or(false) {
        "Admin Only"
    } else {
        "Semua Member"
    };
    let is_announce = if group_meta.get("announce").and_then(Value::as_bool).unwrap_or(false) {
        "Aktif"
    } else {
        "Nonaktif"
    };
    let is_community = if group_meta.get("isCommunity").and_then(Value::as_bool).unwrap_or(false) {
        "✓ Ya"
    } else {
        "✘ Tidak"
    };
    let join_mode = if group_meta.get("joinApprovalMode").and_then(Value::as_bool).unwrap_or(false) {
        "Perlu Approval"
    } else {
        "Bebas"
    };

    let _pp = ctx.sock().foto_profil(&group_jid, "image"); // diabaikan: tidak ada variant interactive

    let bot_nama = ctx.config().bot.name.clone();
    let bot_nama = if bot_nama.is_empty() { BOT_NAMA_DEFAULT } else { &bot_nama };

    let info_text = format!(
        "── .✦ 𝗚𝗥𝗢𝗨𝗣 𝗜𝗡𝗙𝗢 ✦. ── 𝜗ৎ\n\n\
         ╭─〔 {} 〕───⬣\n\
         │  ✦ ɴᴀᴍᴀ        : *{}*\n\
         │  ✦ ɪᴅ             : `{}`\n\
         │  ✦ ᴍᴇᴍʙᴇʀ     : *{}*\n\
         │  ✦ ᴀᴅᴍɪɴ        : *{}*\n\
         │  ✦ ᴏᴡɴᴇʀ       : @{}\n\
         │  ✦ ᴅɪʙᴜᴀᴛ       : *{}*\n\
         │  ✦ ᴋᴏᴍᴜɴɪᴛᴀs : *{}*\n\
         │  ✦ ᴇᴅɪᴛ ɪɴꜰᴏ   : *{}*\n\
         │  ✦ ᴀɴɴᴏᴜɴᴄᴇ : *{}*\n\
         │  ✦ ᴊᴏɪɴ ᴍᴏᴅᴇ  : *{}*\n\
         │  ✦ ᴅᴇsᴋʀɪᴘsɪ  : {}\n\
         ╰──────────────⬣\n\n\
         .☘︎ ݁˖ © {}",
        group_name,
        group_name,
        group_jid,
        member_count,
        admin_count,
        group_owner_tanpa_server,
        created_at,
        is_community,
        is_restrict,
        is_announce,
        join_mode,
        desc_preview,
        bot_nama
    );

    Reply::text(info_text).with_reaction("✓")
}

/// `cekidgc.js:43-82` — tentukan JID + metadata dari input / chat saat ini.
fn cari_grup(ctx: &Ctx, input: &str) -> Option<(String, Value)> {
    if input.contains("chat.whatsapp.com/") {
        let kode = input
            .split("chat.whatsapp.com/")
            .nth(1)
            .and_then(|s| s.split([' ', '?']).next());
        let kode = kode?.trim();
        if kode.is_empty() {
            return None;
        }
        return match ctx.sock().info_undangan(kode) {
            Ok(meta) => meta.get("id").and_then(Value::as_str).map(|jid| (jid.to_string(), meta)),
            Err(_) => None,
        };
    }
    if input.ends_with("@g.us") {
        return match ctx.sock().metadata_grup(input) {
            Ok(meta) => Some((input.to_string(), meta)),
            Err(_) => None,
        };
    }
    if ctx.is_group {
        return match ctx.sock().metadata_grup(&ctx.chat) {
            Ok(meta) => Some((ctx.chat.clone(), meta)),
            Err(_) => None,
        };
    }
    None
}

fn balasan_tidak_ditemukan(prefix: &str, reaksi: &str) -> Reply {
    Reply::text(
        "── .✦ ──\n\n> Tidak dapat menemukan info grup .☘︎ ݁˖",
    ).with_reaction(reaksi)
}

/// `cekidgc.js:24-33` — format tanggal, asumsikan zona Jakarta (UTC+7).
fn format_date(ms: i64) -> String {
    if ms == 0 {
        return "—".to_string();
    }
    let (_y, m, d, h, mi, _s) = crate::common::jakarta_parts(ms);
    format!(
        "{:02}/{:02}/{} {:02}:{:02}",
        d, m, _y, h, mi
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::sock::{AdaWhatsapp, AksiPeserta, Sock, SockError};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct StubSock {
        meta: Mutex<Option<Value>>,
        undangan: Mutex<Option<Value>>,
        foto: Mutex<Option<String>>,
    }

    impl Sock for StubSock {
        fn ada_whatsapp(&self, _: &[String]) -> Result<Vec<AdaWhatsapp>, SockError> {
            Err(SockError::baru("x"))
        }
        fn foto_profil(&self, _jid: &str, _tipe: &str) -> Result<Option<String>, SockError> {
            Ok(self.foto.lock().unwrap().clone())
        }
        fn metadata_grup(&self, jid: &str) -> Result<Value, SockError> {
            Ok(self.meta.lock().unwrap().clone().unwrap_or_else(|| {
                serde_json::json!({
                    "subject": "Grup " + jid,
                    "id": jid,
                    "owner": "628111@s.whatsapp.net",
                    "creation": 1735689600000i64,
                    "participants": [
                        {"jid": "628111@s.whatsapp.net", "admin": "superadmin"},
                        {"jid": "628222@s.whatsapp.net", "admin": "admin"},
                        {"jid": "628333@s.whatsapp.net", "admin": null}
                    ],
                    "desc": "ini deskripsi grup yang cukup panjang untuk uji potongan di atas 120 karakter ............"
                })
            }))
        }
        fn semua_grup(&self) -> Result<Value, SockError> { Err(SockError::baru("x")) }
        fn ubah_peserta(&self, _: &str, _: &[String], _: AksiPeserta) -> Result<(), SockError> { Err(SockError::baru("x")) }
        fn set_foto_profil(&self, _: &str, _: &[u8]) -> Result<(), SockError> { Err(SockError::baru("x")) }
        fn keluar_grup(&self, _: &str) -> Result<(), SockError> { Err(SockError::baru("x")) }
        fn kode_undangan(&self, _: &str) -> Result<String, SockError> { Err(SockError::baru("x")) }
        fn info_undangan(&self, _kode: &str) -> Result<Value, SockError> {
            Ok(self.undangan.lock().unwrap().clone().unwrap_or_else(|| {
                serde_json::json!({"id": "629999@g.us", "subject": "Grup Undangan"})
            }))
        }
        fn buat_grup(&self, _: &str, _: &[String]) -> Result<Value, SockError> { Err(SockError::baru("x")) }
        fn terima_undangan(&self, _: &str) -> Result<String, SockError> { Err(SockError::baru("x")) }
        fn tandai_dibaca(&self, _: &[Value]) -> Result<(), SockError> { Err(SockError::baru("x")) }
        fn presence(&self, _: &str, _: &str) -> Result<(), SockError> { Err(SockError::baru("x")) }
        fn status_teks(&self, _: &str) -> Result<Value, SockError> { Err(SockError::baru("x")) }
        fn nama_kontak(&self, _: &str) -> Result<String, SockError> { Err(SockError::baru("x")) }
        fn profil_bisnis(&self, _: &str) -> Result<Option<Value>, SockError> { Err(SockError::baru("x")) }
    }

    fn grup_meta() -> Value {
        serde_json::json!({
            "subject": "Coba Grup",
            "id": "621234567890@g.us",
            "owner": "628111@s.whatsapp.net",
            "creation": 1735689600000i64,
            "participants": [
                {"jid": "628111@s.whatsapp.net", "admin": "superadmin"},
                {"jid": "628222@s.whatsapp.net", "admin": "admin"},
                {"jid": "628333@s.whatsapp.net", "admin": null}
            ],
            "desc": "ini deskripsi grup yang cukup panjang untuk uji potongan di atas 120 karakter ............"
        })
    }

    fn run(chat: &str, is_group: bool, text: &str) -> Reply {
        let sock = Arc::new(StubSock::default());
        sock.meta.lock().unwrap().replace(grup_meta());
        let ctx = Ctx {
            chat: chat.to_string(),
            is_group,
            text: text.to_string(),
            command: "cekidgc".into(),
            sock,
            config: Arc::new(oktz_plugin_api::config::Runtime::default()),
            ..Ctx::default()
        };
        cekidgc(ctx)
    }

    #[test]
    fn format_tanggal_jakarta() {
        assert_eq!(format_date(0), "—");
        assert_eq!(format_date(1735689600000), "01/01/2025 07:00");
    }

    #[test]
    fn info_grup_sama_dengan_js() {
        let r = run("621234567890@g.us", true, "");
        let t = r.without_mutations().text_body().unwrap();
        assert!(t.contains("╭─〔 Coba Grup 〕───⬣"));
        assert!(t.contains("│  ✦ ɪᴅ             : `621234567890@g.us`"));
        assert!(t.contains("│  ✦ ᴍᴇᴍʙᴇʀ     : *3*"));
        assert!(t.contains("│  ✦ ᴀᴅᴍɪɴ        : *2*"));
        assert!(t.contains("│  ✦ ᴏᴡɴᴇʀ       : @628111"));
        assert!(t.contains("│  ✦ ᴅɪʙᴜᴀᴛ       : *01/01/2025 07:00*"));
        assert!(t.contains("│  ✦ ᴋᴏᴍᴜɴɪᴛᴀs : *✘ Tidak*"));
        assert!(t.contains("│  ✦ ᴇᴅɪᴛ ɪɴꜰᴏ   : *Semua Member*"));
        assert!(t.contains("│  ✦ ᴀɴɴᴏᴜɴᴄᴇ : *Nonaktif*"));
        assert!(t.contains("│  ✦ ᴊᴏɪɴ ᴍᴏᴅᴇ  : *Bebas*"));
        assert!(t.contains(".☘︎ ݁˖ © Bot"));
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✓"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn deskripsi_dipotong_120_karakter() {
        let r = run("621234567890@g.us", true, "");
        let t = r.without_mutations().text_body().unwrap();
        let idx = t.find("│  ✦ ᴅᴇsᴋʀɪᴘsɪ  :").unwrap();
        let baris = &t[idx..idx + 140];
        assert!(baris.contains("…"), "harus ada titik suspensi: {baris}");
    }

    #[test]
    fn di_bukan_grup_tanpa_input_mengirim_bantuan() {
        let sock = Arc::new(StubSock::default());
        let ctx = Ctx {
            chat: "628111@s.whatsapp.net".into(),
            is_group: false,
            text: "".into(),
            command: "cekidgc".into(),
            sock,
            ..Ctx::default()
        };
        let r = cekidgc(ctx);
        let t = r.without_mutations().text_body().unwrap();
        assert!(t.contains("── .✦ 𝗖𝗘𝗞 𝗜𝗗 𝗚𝗥𝗨𝗣 ✦. ──"));
    }

    #[test]
    fn link_grup_menggunakan_info_undangan() {
        let sock = Arc::new(StubSock::default());
        sock.undangan.lock().unwrap().replace(serde_json::json!({"id": "629999@g.us", "subject": "Grup Undangan"}));
        let ctx = Ctx {
            chat: "628111@s.whatsapp.net".into(),
            is_group: false,
            text: "https://chat.whatsapp.com/ABCDE12345".into(),
            command: "cekidgc".into(),
            sock,
            ..Ctx::default()
        };
        let r = cekidgc(ctx);
        let t = r.without_mutations().text_body().unwrap();
        assert!(t.contains("Grup Undangan"));
        assert!(t.contains("629999@g.us"));
    }

    #[test]
    fn link_grup_tidak_valid_dapat_reaksi_sil() {
        let sock = Arc::new(StubSock::default());
        // undangan tanpa id → meta.get("id") = None → tidak ditemukan
        sock.undangan.lock().unwrap().replace(serde_json::json!({"subject": "Tanpa ID"}));
        let ctx = Ctx {
            chat: "628111@s.whatsapp.net".into(),
            is_group: false,
            text: "https://chat.whatsapp.com/".into(),
            command: "cekidgc".into(),
            sock,
            ..Ctx::default()
        };
        let r = cekidgc(ctx);
        let t = r.without_mutations().text_body().unwrap();
        assert_eq!(t, "── .✦ ──\n\n> Tidak dapat menemukan info grup .☘︎ ݁˖");
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✘"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }
}
