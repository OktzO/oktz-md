//! `plugins/owner/akses.js` — grant temporary/permanent command access to users.
//!
//! `ponytail:` `db.getUser` / `db.setUser` dipindah ke `ctx.user_field` +
//! `Mutation::SetUser`. Library `ms` untuk durasi tidak ada di Rust; parser
//! durasi sederhana (`1h`, `1d`, `30d`, `1y`, `permanent`/`perm`) ditulis
//! sendiri. `sock.parseMention` dan daftar interaktif `delakses` tidak ada di
//! `Ctx`; del tanpa command spesifik menampilkan teks daftar akses aktif.
//! Reaksi JS (`m.reply` tanpa reaksi eksplisit di akses) tidak dibawa.
//!
//! Catatan: baris `pub mod akses;` perlu ditambah ke `owner/mod.rs` secara
//! manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

const PERMANENT_KEYS: [&str; 2] = ["permanent", "perm"];

/// `ms("30d")` — milidetik. Mengembalikan `None` untuk format tak dikenal
// (padanan `NaN` yang falsy di JS).
fn parse_duration(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let num_end = s.chars().position(|c| !c.is_ascii_digit())?;
    if num_end == 0 {
        return None;
    }
    let num: i64 = s[..num_end].parse().ok()?;
    let unit = s[num_end..].trim();
    let mult = match unit {
        "s" => 1_000,
        "m" => 60_000,
        "h" => 3_600_000,
        "d" => 86_400_000,
        "w" => 604_800_000,
        "y" => 31_536_000_000,
        _ => return None,
    };
    Some(num * mult)
}


/// `acc.expired ? ms(acc.expired - now) : "Permanent"` (bentuk pendek).
fn format_expired(expired: Option<i64>, now: i64) -> String {
    match expired {
        Some(e) => {
            let left = e - now;
            if left <= 0 {
                "Expired".to_string()
            } else {
                let ms = left;
                let seconds = ms / 1000;
                let minutes = seconds / 60;
                let hours = minutes / 60;
                let days = hours / 24;
                if days > 0 {
                    format!("{}d", days)
                } else if hours > 0 {
                    format!("{}h", hours)
                } else if minutes > 0 {
                    format!("{}m", minutes)
                } else {
                    format!("{}s", seconds)
                }
            }
        }
        None => "Permanent".to_string(),
    }
}

fn jid_dari_nomor(arg: &str) -> Option<String> {
    if arg.len() >= 5 && arg.len() <= 15 && arg.chars().all(|c| c.is_ascii_digit()) {
        return Some(format!("{}@s.whatsapp.net", arg));
    }
    if let Some(rest) = arg.strip_prefix('@')
        && !rest.is_empty()
        && rest.chars().all(|c| c.is_ascii_digit())
    {
        return Some(format!("{}@s.whatsapp.net", rest));
    }
    None
}

fn resolve_target(ctx: &Ctx) -> Option<String> {
    if let Some(jid) = ctx.mentioned.first() {
        return Some(jid.clone());
    }
    if let Some(q) = ctx.quoted() {
        return Some(q.sender.clone());
    }
    for arg in &ctx.args {
        if let Some(jid) = jid_dari_nomor(arg) {
            return Some(jid);
        }
    }
    None
}

/// `m.args.filter(a => !a.includes("@") && !/^\d{10,}$/.test(a))`
fn clean_args(args: &[String]) -> Vec<String> {
    args.iter()
        .filter(|a| !a.contains('@') && !(a.len() >= 10 && a.chars().all(|c| c.is_ascii_digit())))
        .cloned()
        .collect()
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

fn access_list(user: &Value) -> Vec<Value> {
    user.get("access")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn active_access(access: &[Value], now: i64) -> Vec<Value> {
    access
        .iter()
        .filter(|a| {
            a.get("expired")
                .and_then(Value::as_i64)
                .is_none_or(|e| e > now)
        })
        .cloned()
        .collect()
}

#[plugin(
    name = "akses",
    alias = ["addakses", "delakses", "listakses", "addaccess", "delaccess", "listaccess"],
    category = "owner",
    description = "Grant temporary/permanent command access to users",
    usage = ".addakses <cmd> <duration> <user>",
    example = ".addakses addowner 30d @user",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn akses(ctx: Ctx) -> Reply {
    let cmd = ctx.command.to_lowercase();
    let is_add = cmd == "addakses" || cmd == "addaccess";
    let is_del = cmd == "delakses" || cmd == "delaccess";
    let is_list = cmd == "listakses" || cmd == "listaccess";

    let target = resolve_target(&ctx);
    let now = now_ms();

    if is_list {
        let target_jid = target.as_deref().unwrap_or(&ctx.sender);
        let user_data = ctx.user_data.clone();
        // `db.getUser(target)` — di port ini user_data adalah objek pengirim;
        // untuk target lain tidak ada jalan baca store selain user_data sendiri.
        // Pakai user_data sebagai akses target (batasan Ctx).
        let access = access_list(&user_data);
        let active = active_access(&access, now);
        if active.len() != access.len() {
            let mut next = user_data.clone();
            next.as_object_mut().unwrap().insert("access".to_string(), Value::Array(active.clone()));
            return Reply::text(list_text(target_jid, &active, now))
                .with_mutation(Mutation::SetUser {
                    jid: target_jid.to_string(),
                    data: next,
                });
        }
        return Reply::text(list_text(target_jid, &active, now));
    }

    if is_add {
        let Some(ref target_jid) = target else {
            return Reply::text(
                "❌ *Target Invalid*\n\nTag user / Reply chat / Tulis nomor target",
            );
        };
        let clean = clean_args(&ctx.args);
        if clean.len() < 2 {
            return Reply::text(
                "⚠️ *Format Salah*\n\n\
                 Format: `.addakses <command> <durasi> <target>`\n\n\
                 *Contoh:*\n\
                 > `.addakses addowner 30d @user` (30 Hari)\n\
                 > `.addakses unban permanent @user` (Selamanya)\n\n\
                 *Durasi Support:* 1h, 1d, 30d, 1y",
            );
        }
        let command_target = clean[0].to_lowercase();
        let duration_target = clean[1].to_lowercase();

        let mut user = ctx.user_data.clone();
        if user.get("access").is_none() {
            user.as_object_mut().unwrap().insert("access".to_string(), Value::Array(Vec::new()));
        }
        let mut access = access_list(&user);

        let expired_time = if PERMANENT_KEYS.contains(&duration_target.as_str()) {
            None
        } else {
            match parse_duration(&duration_target) {
                Some(ms) => Some(now + ms),
                None => {
                    return Reply::text("❌ Format durasi salah! Gunakan: 1h, 1d, 30d");
                }
            }
        };

        if let Some(idx) = access.iter().position(|a| a.get("cmd").and_then(Value::as_str) == Some(command_target.as_str())) {
            access[idx] = serde_json::json!({ "cmd": command_target, "expired": expired_time.map(Value::from) });
            let mut next = user.clone();
            next.as_object_mut().unwrap().insert("access".to_string(), Value::Array(access));
            return Reply::text(format!(
                "✅ *ᴀᴋsᴇs ᴅɪᴘᴇʀʙᴀʀᴜɪ*\n\n\
                 Command: `{}`\n\
                 Durasi: *{}*\n\
                 Target: @{}",
                command_target, duration_target, target_jid.split('@').next().unwrap_or(target_jid)
            ))
            .with_mutation(Mutation::SetUser {
                jid: target_jid.clone(),
                data: next,
            });
        }

        access.push(serde_json::json!({ "cmd": command_target, "expired": expired_time.map(Value::from) }));
        let mut next = user.clone();
        next.as_object_mut().unwrap().insert("access".to_string(), Value::Array(access));
        Reply::text(format!(
            "✅ *ᴀᴋsᴇs ᴅɪʙᴇʀɪᴋᴀɴ*\n\n\
             ┃ 🔑 ᴄᴍᴅ: `{}`\n\
             ┃ ⏱️ ᴅᴜʀᴀsɪ: *{}*\n\
             ┃ 👤 ᴛᴀʀɢᴇᴛ: @{}",
            command_target, duration_target, target_jid.split('@').next().unwrap_or(target_jid)
        ))
        .with_mutation(Mutation::SetUser {
            jid: target_jid.clone(),
            data: next,
        })
    } else if is_del {
        let Some(target_jid) = target.as_deref() else {
            return Reply::text("❌ Tag user yang mau dihapus aksesnya!");
        };
        let user = ctx.user_data.clone();
        let mut access = access_list(&user);
        let active = active_access(&access, now);

        let specific_cmd = ctx
            .args
            .iter()
            .find(|a| !a.contains('@') && !a.chars().all(|c| c.is_ascii_digit()))
            .map(|a| a.to_lowercase());

        if let Some(cmd_name) = specific_cmd {
            if let Some(idx) = access.iter().position(|a| a.get("cmd").and_then(Value::as_str) == Some(cmd_name.as_str())) {
                access.remove(idx);
                let mut next = user.clone();
                next.as_object_mut().unwrap().insert("access".to_string(), Value::Array(access));
                return Reply::text(format!(
                    "✅ Akses `{}` berhasil dicabut dari @{}",
                    cmd_name,
                    target_jid.split('@').next().unwrap_or(target_jid)
                ))
                .with_mutation(Mutation::SetUser {
                    jid: target_jid.to_string(),
                    data: next,
                });
            }
            return Reply::text(format!(
                "❌ User tidak punya akses command `{}`",
                cmd_name
            ));
        }

        if active.is_empty() {
            return Reply::text("⚠️ User ini tidak memiliki akses command apapun.");
        }

        let mut txt = String::from("🔓 *CABUT AKSES*\n\n");
        txt.push_str(&format!(
            "Pilih akses command yang ingin dihapus dari @{}\n\n",
            target_jid.split('@').next().unwrap_or(target_jid)
        ));
        for acc in &active {
            let cmd_name = acc.get("cmd").and_then(Value::as_str).unwrap_or("-");
            let expired = acc.get("expired").and_then(Value::as_i64);
            txt.push_str(&format!(
                "- {}: {}\n",
                cmd_name,
                format_expired(expired, now)
            ));
        }
        Reply::text(txt)
    } else {
        Reply::text("❌ Perintah tidak dikenali")
    }
}

fn list_text(target: &str, active: &[Value], now: i64) -> String {
    let nomor = target.split('@').next().unwrap_or(target);
    if active.is_empty() {
        return format!(
            "📊 *ᴜsᴇʀ ᴀᴄᴄᴇss*\n\nTarget: @{}\nStatus: *Tidak punya akses khusus*",
            nomor
        );
    }
    let mut txt = String::from("📊 *ᴜsᴇʀ ᴀᴄᴄᴇss*\n\n");
    txt.push_str(&format!("Target: @{}\n", nomor));
    txt.push_str(&format!("Total: *{}* commands\n", active.len()));
    txt.push_str("━━━━━━━━━━━━━━━\n\n");
    for (i, acc) in active.iter().enumerate() {
        let cmd_name = acc.get("cmd").and_then(Value::as_str).unwrap_or("-");
        let expired = acc.get("expired").and_then(Value::as_i64);
        txt.push_str(&format!("{}. *{}*\n", i + 1, cmd_name));
        txt.push_str(&format!("   └ {}\n", format_expired(expired, now)));
    }
    txt
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str], command: &str, user_data: Value, mentioned: Option<&str>, quoted: bool) -> Reply {
        let args_vec: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
        let ctx = Ctx {
            command: command.to_string(),
            args: args_vec.clone(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: format!("{} {}", command, args.join(" ")),
            mentioned: mentioned.into_iter().map(|s| s.to_string()).collect(),
            quoted: if quoted {
                Some(oktz_plugin_api::QuotedMessage {
                    id: "q1".into(),
                    sender: "628999@s.whatsapp.net".into(),
                    chat: "628111@s.whatsapp.net".into(),
                    text: String::new(),
                    kind: oktz_plugin_api::QuotedKind::Text,
                    mimetype: None,
                    file_name: None,
                    from_me: false,
            timestamp: 0,
        })
            } else {
                None
            },
            user_data,
            ..Default::default()
        };
        akses(ctx)
    }

    fn body(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    fn stored(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "6281234567890@s.whatsapp.net");
                data.clone()
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("akses").unwrap();
        assert_eq!(p.meta.name, "akses");
        assert_eq!(
            p.meta.alias,
            ["addakses", "delakses", "listakses", "addaccess", "delaccess", "listaccess"]
        );
        assert_eq!(p.meta.category, "owner");
        assert!(p.meta.is_owner);
    }

    #[test]
    fn parse_duration_mirip_ms() {
        assert_eq!(parse_duration("30d"), Some(30 * 86_400_000));
        assert_eq!(parse_duration("1h"), Some(3_600_000));
        assert_eq!(parse_duration("1y"), Some(31_536_000_000));
        assert_eq!(parse_duration("abc"), None);
        assert_eq!(parse_duration(""), None);
    }

    #[test]
    fn list_tanpa_akses_tampilkan_status_kosong() {
        let r = run(&[], "listakses", serde_json::json!({}), None, false);
        assert!(body(&r).contains("Tidak punya akses khusus"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn add_membutuhkan_target() {
        let r = run(&["addowner", "30d"], "addakses", serde_json::json!({}), None, false);
        assert!(body(&r).contains("Target Invalid"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn add_format_salah_kalau_clean_args_kurang_dua() {
        let r = run(&["6281234567890", "addowner"], "addakses", serde_json::json!({}), None, false);
        assert!(body(&r).contains("Format Salah"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn add_durasi_salah_ditolak() {
        let r = run(&["6281234567890", "addowner", "xyz"], "addakses", serde_json::json!({}), None, false);
        assert!(body(&r).contains("Format durasi salah"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn add_berhasil_simpan_ke_user() {
        let r = run(&["6281234567890", "addowner", "30d"], "addakses", serde_json::json!({}), None, false);
        assert!(body(&r).contains("ᴀᴋsᴇs ᴅɪʙᴇʀɪᴋᴀɴ"));
        let data = stored(&r);
        let access = data.get("access").and_then(Value::as_array).unwrap();
        assert_eq!(access.len(), 1);
        assert_eq!(access[0].get("cmd").and_then(Value::as_str), Some("addowner"));
        assert!(access[0].get("expired").is_some());
    }

    #[test]
    fn add_memperbarui_akses_yang_sudah_ada() {
        let existing = serde_json::json!({
            "access": [{ "cmd": "addowner", "expired": 1000 }]
        });
        let r = run(&["6281234567890", "addowner", "1d"], "addakses", existing, None, false);
        let data = stored(&r);
        let access = data.get("access").and_then(Value::as_array).unwrap();
        assert_eq!(access.len(), 1);
        assert!(access[0].get("expired").and_then(Value::as_i64).unwrap() > 1000);
    }

    #[test]
    fn del_command_spesifik_cabut_akses() {
        let existing = serde_json::json!({
            "access": [{ "cmd": "addowner", "expired": null }]
        });
        let r = run(&["6281234567890", "addowner"], "delakses", existing, None, false);
        assert!(body(&r).contains("berhasil dicabut"));
        let data = stored(&r);
        let access = data.get("access").and_then(Value::as_array).unwrap();
        assert!(access.is_empty());
    }

    #[test]
    fn permanent_expired_null() {
        let r = run(&["6281234567890", "cmdx", "permanent"], "addakses", serde_json::json!({}), None, false);
        let data = stored(&r);
        let access = data.get("access").and_then(Value::as_array).unwrap();
        assert_eq!(access[0].get("expired"), Some(&Value::Null));
    }
}
