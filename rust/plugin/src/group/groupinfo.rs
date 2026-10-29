//! `plugins/group/groupinfo.js` — informasi lengkap grup.

use oktz_plugin_api::MediaSource;
use oktz_plugin_api::registry::{GroupRoster, Participant};
use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::{date_long, error_template, jid_number, js_truthy_value};

/// `featureStatus` (`groupinfo.js:21-24`): hanya `true` atau string `"on"`.
fn feature_status(value: Option<&Value>) -> &'static str {
    match value {
        Some(Value::Bool(true)) => "✅",
        Some(Value::String(s)) if s == "on" => "✅",
        _ => "❌",
    }
}

fn teks(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

/// `getParticipantJid`/`resolveAnyLidToJid`: owner `@lid` dicari pasangannya di
/// daftar peserta metadata (`lid` → `id`/`phoneNumber`).
fn resolve_jid(jid: &str, meta: &Value, roster: &GroupRoster) -> Option<String> {
    if !jid.ends_with("@lid") {
        return Some(jid.to_string());
    }
    if let Some(list) = meta.get("participants").and_then(Value::as_array) {
        for p in list {
            if p.get("lid").and_then(Value::as_str) != Some(jid) {
                continue;
            }
            for key in ["id", "phoneNumber"] {
                if let Some(kandidat) = teks(p.get(key))
                    && !kandidat.ends_with("@lid")
                {
                    return Some(kandidat.to_string());
                }
            }
        }
    }
    roster
        .participants
        .iter()
        .find(|p| p.jid == jid && !p.jid.ends_with("@lid"))
        .map(|p| p.jid.clone())
}

fn owner_jid(meta: &Value, roster: &GroupRoster) -> Option<String> {
    let dari_meta = teks(meta.get("owner"))
        .and_then(|o| resolve_jid(o, meta, roster))
        .filter(|o| !o.is_empty());
    if dari_meta.is_some() {
        return dari_meta;
    }
    let superadmin = roster.participants.iter().find(|p| p.is_owner);
    if let Some(p) = superadmin
        && let Some(j) = resolve_jid(&p.jid, meta, roster)
    {
        return Some(j);
    }
    roster
        .participants
        .iter()
        .find(|p| p.is_admin)
        .and_then(|p| resolve_jid(&p.jid, meta, roster))
}

fn jumlah_peserta(meta: &Value, roster: &GroupRoster) -> (usize, usize) {
    match meta.get("participants").and_then(Value::as_array) {
        Some(list) => {
            let admin = list
                .iter()
                .filter(|p| p.get("admin").is_some_and(js_truthy_value))
                .count();
            (list.len(), admin)
        }
        None => (
            roster.participants.len(),
            roster.participants.iter().filter(|p: &&Participant| p.is_admin).count(),
        ),
    }
}

/// Susun teks `INFO GRUP` (`groupinfo.js:44-96`).
fn susun_info(
    meta: &Value,
    roster: &GroupRoster,
    chat: &str,
    group: &Value,
    owner: Option<&str>,
) -> String {
    let (total, admin) = jumlah_peserta(meta, roster);
    let owner_display = owner
        .filter(|o| !o.split('@').next().unwrap_or("").contains(':'))
        .map(|o| format!("@{}", jid_number(o)))
        .unwrap_or_else(|| "Tidak diketahui".to_string());

    let created = match meta.get("creation") {
        Some(v) if js_truthy_value(v) => v
            .as_i64()
            .map(date_long)
            .unwrap_or_else(|| "Tidak diketahui".to_string()),
        _ => "Tidak diketahui".to_string(),
    };
    let is_open = !meta.get("announce").is_some_and(js_truthy_value);

    let mut text = String::from("👥 *INFO GRUP*\n\n");
    text.push_str(&format!(
        "Nama: *{}*\n",
        teks(meta.get("subject")).unwrap_or("undefined")
    ));
    text.push_str(&format!("ID: {chat}\n"));
    text.push_str(&format!("Owner: {owner_display}\n"));
    text.push_str(&format!("Dibuat: {created}\n"));
    text.push_str(&format!(
        "Status: {}\n\n",
        if is_open { "🔓 Terbuka" } else { "🔒 Tertutup" }
    ));

    text.push_str("📊 *MEMBER*\n");
    text.push_str(&format!("Total: {total}\n"));
    text.push_str(&format!("Admin: {admin}\n"));
    text.push_str(&format!("Member: {}\n\n", total.saturating_sub(admin)));

    text.push_str("🔧 *FITUR AKTIF*\n");
    for (label, key) in [
        ("Welcome", "welcome"),
        ("Goodbye", "goodbye"),
        ("Autoreply", "autoreply"),
        ("AutoAI", "autoai"),
        ("AutoDL", "autodl"),
        ("AutoSticker", "autosticker"),
        ("AutoMedia", "automedia"),
    ] {
        text.push_str(&format!(
            "{label}: {}\n",
            feature_status(group.get(key))
        ));
    }
    text.push('\n');

    text.push_str("🛡️ *PROTEKSI*\n");
    for (label, key) in [
        ("AntiLink", "antilink"),
        ("AntiBot", "antibot"),
        ("AntiToxic", "antitoxic"),
        ("AntiRemove", "antiremove"),
        ("AntiHidetag", "antihidetag"),
        ("AntiSticker", "antisticker"),
        ("AntiMedia", "antimedia"),
        ("AntiDocument", "antidocument"),
    ] {
        text.push_str(&format!(
            "{label}: {}\n",
            feature_status(group.get(key))
        ));
    }
    // JS tidak menutup baris terakhir dengan newline.
    if text.ends_with('\n') {
        text.pop();
    }

    if let Some(desc) = meta.get("desc").and_then(Value::as_str).filter(|s| !s.is_empty()) {
        text.push_str(&format!("\n\n📝 *DESKRIPSI*\n{desc}"));
    }
    text
}

fn mentions_of(owner: Option<&str>) -> Vec<String> {
    owner
        .filter(|o| !o.contains(':'))
        .map(|o| vec![o.to_string()])
        .unwrap_or_default()
}

#[plugin(
    name = "groupinfo",
    alias = ["infogroup", "gcinfo", "infogc", "gc"],
    category = "group",
    description = "Menampilkan informasi lengkap grup",
    usage = ".groupinfo",
    example = ".groupinfo",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn groupinfo(ctx: Ctx) -> Reply {
    let meta = match ctx.sock.metadata_grup(&ctx.chat) {
        Ok(m) => m,
        Err(_) => return Reply::text(error_template(&ctx)),
    };
    let owner = owner_jid(&meta, &ctx.roster);
    let text = susun_info(&meta, &ctx.roster, &ctx.chat, &ctx.group_data, owner.as_deref());
    let mentions = mentions_of(owner.as_deref());

    match ctx.sock.foto_profil(&ctx.chat, "image") {
        Ok(Some(url)) if !url.trim().is_empty() => Reply::Image {
            media: Some(MediaSource::url(url)),
            caption: text,
            mentions,
        },
        _ => Reply::Text { text, mentions },
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn roster() -> GroupRoster {
        GroupRoster {
            participants: vec![
                Participant {
                    jid: "628111@s.whatsapp.net".into(),
                    is_admin: true,
                    is_owner: true,
                    data: Value::Null,
                },
                Participant {
                    jid: "628222@s.whatsapp.net".into(),
                    is_admin: true,
                    is_owner: false,
                    data: Value::Null,
                },
                Participant {
                    jid: "628333@s.whatsapp.net".into(),
                    is_admin: false,
                    is_owner: false,
                    data: Value::Null,
                },
            ],
            is_bot_admin: true,
        }
    }

    fn meta() -> Value {
        json!({
            "id": "628999@g.us",
            "subject": "Grup Uji",
            "owner": "628111@s.whatsapp.net",
            "creation": 1767225600000i64,
            "announce": true,
            "desc": "Deskripsi",
            "participants": [
                {"id": "628111@s.whatsapp.net", "admin": "superadmin"},
                {"id": "628222@s.whatsapp.net", "admin": "admin"},
                {"id": "628333@s.whatsapp.net", "admin": null}
            ]
        })
    }

    #[test]
    fn feature_status_hanya_true_dan_on() {
        assert_eq!(feature_status(Some(&json!(true))), "✅");
        assert_eq!(feature_status(Some(&json!("on"))), "✅");
        assert_eq!(feature_status(Some(&json!("off"))), "❌");
        assert_eq!(feature_status(Some(&json!(1))), "❌");
        assert_eq!(feature_status(None), "❌");
    }

    #[test]
    fn susun_info_mengikuti_baris_javascript() {
        let group = json!({"welcome": "on", "antilink": true});
        let text = susun_info(&meta(), &roster(), "628999@g.us", &group, Some("628111@s.whatsapp.net"));
        assert!(text.starts_with("👥 *INFO GRUP*\n\nNama: *Grup Uji*\n"));
        assert!(text.contains("ID: 628999@g.us\n"));
        assert!(text.contains("Owner: @628111\n"));
        assert!(text.contains("Dibuat: 1 Januari 2026\n"));
        assert!(text.contains("Status: 🔒 Tertutup\n\n"));
        assert!(text.contains("Total: 3\nAdmin: 2\nMember: 1\n\n"));
        assert!(text.contains("Welcome: ✅\nGoodbye: ❌\n"));
        assert!(text.contains("AntiLink: ✅\nAntiBot: ❌\n"));
        assert!(text.ends_with("AntiDocument: ❌\n\n📝 *DESKRIPSI*\nDeskripsi"));
    }

    #[test]
    fn owner_lid_diselesaikan_lewat_peserta_metadata() {
        let m = json!({
            "owner": "111@lid",
            "participants": [
                {"id": "628111@s.whatsapp.net", "lid": "111@lid", "phoneNumber": "628111@s.whatsapp.net"}
            ]
        });
        assert_eq!(
            owner_jid(&m, &GroupRoster::default()).as_deref(),
            Some("628111@s.whatsapp.net")
        );
    }

    #[test]
    fn owner_tanpa_metadata_jatuh_ke_superadmin() {
        let m = json!({"participants": []});
        assert_eq!(
            owner_jid(&m, &roster()).as_deref(),
            Some("628111@s.whatsapp.net")
        );
    }
}
