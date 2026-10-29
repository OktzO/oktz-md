//! `plugins/owner/prefix.js` — manajemen prefix bot (simpan ke database/prefix.json).
//!
//! JS satu handler menangani 6 command (`addprefix`, `gantiprefix`/`setprefix`,
//! `delprefix`, `listprefix`, `resetprefix`). Di Rust satu `#[plugin]` = satu
//! `name`, jadi daftarkan `name = "addprefix"` dan sisanya sebagai `alias`;
//! `PluginMeta::matches` mencocokkan nama atau alias.
//!
//! `ponytail:` `m.command?.toLowerCase()` di JS sudah lowercase lewat router,
//! jadi `ctx.command` dipakai langsung.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::{Map, Value};

const PREF_DB_PATH: &str = "database/prefix.json";
const CONFIG_PREFIX: &str = ".";

/// `prefix.js:35-49` — pluginConfig. Nama pertama array JS = "addprefix";
/// sisanya alias (pencocokan nama/alias sama-sama lewat `PluginMeta::matches`).
#[plugin(
    name = "addprefix",
    alias = ["gantiprefix", "setprefix", "delprefix", "listprefix", "resetprefix"],
    category = "owner",
    description = "Manajemen prefix bot",
    usage = ".addprefix <prefix1> <prefix2>...",
    example = ".addprefix ! # $ 😚",
    cooldown = 5,
    energi = 0,
    is_owner = true,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_enabled = true
)]
pub fn prefix(ctx: Ctx) -> Reply {
    let data = muat_awalan(&ctx);
    let cmd = ctx.command.as_str();

    match cmd {
        "addprefix" => cmd_addprefix(&ctx, &data),
        "gantiprefix" | "setprefix" => cmd_ganti_awalan(&ctx, &data),
        "delprefix" => cmd_hapus_awalan(&ctx, &data),
        "listprefix" => Reply::text(daftar_awalan(&ctx, &data)),
        "resetprefix" => cmd_reset_awalan(&ctx, &data),
        _ => Reply::text("❌ Command prefix tidak dikenal."),
    }
}

/// `prefix.js:6-13` — baca database/prefix.json, gagal diam-diam → default.
fn muat_awalan(ctx: &Ctx) -> Value {
    match ctx.berkas().baca_teks(PREF_DB_PATH) {
        Ok(teks) => serde_json::from_str(&teks).unwrap_or(Value::Null),
        Err(_) => json_default(),
    }
}

fn json_default() -> Value {
    let mut m = Map::new();
    m.insert("prefixes".to_string(), Value::Array(Vec::new()));
    m.insert("noprefix".to_string(), Value::Bool(false));
    Value::Object(m)
}

/// `prefix.js:15-21` — tulis JSON, buat folder induk jika perlu.
fn simpan_awalan(ctx: &Ctx, data: &Value) {
    let _ = ctx.berkas().buat_folder("database");
    let _ = ctx.berkas().tulis_teks(PREF_DB_PATH, &serde_json::to_string_pretty(data).unwrap_or_default());
}

fn prefixes_arr(data: &Value) -> Vec<String> {
    data.get("prefixes")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn noprefix_aktif(data: &Value) -> bool {
    data.get("noprefix").and_then(Value::as_bool).unwrap_or(false)
}

/// `prefix.js:23-28` — gabung config prefix + database, dedup.
fn semua_awalan(_ctx: &Ctx, data: &Value) -> Vec<String> {
    let mut gabung: Vec<String> = vec![CONFIG_PREFIX.to_string()];
    for p in prefixes_arr(data) {
        if !gabung.contains(&p) {
            gabung.push(p);
        }
    }
    gabung
}

/// `prefix.js:60-105` — addprefix.
fn cmd_addprefix(ctx: &Ctx, data: &Value) -> Reply {
    if ctx.args.is_empty() {
        return Reply::text(format!(
            "✏️ *ᴀᴅᴅ ᴘʀᴇғɪx*\n\n> Tambah prefix baru untuk bot\n\n*Format:*\n> `{0}addprefix <prefix1> <prefix2> ...`\n\n*Contoh:*\n> `{0}addprefix ! # $ 😚`\n\n*Special:*\n> `{0}addprefix <noprefix>` - Tanpa prefix",
            ctx.prefix
        ));
    }

    if ctx.args.iter().any(|a| a == "<noprefix>" || a == "noprefix") {
        let mut data = data.clone();
        if let Some(obj) = data.as_object_mut() {
            obj.insert("noprefix".to_string(), Value::Bool(true));
        }
        simpan_awalan(ctx, &data);
        return Reply::text(
            "✅ *ɴᴏᴘʀᴇғɪx ᴅɪᴀᴋᴛɪғᴋᴀɴ*\n\n> Bot sekarang bisa dijalankan tanpa prefix\n> Ketik langsung nama command (misal: `menu`)"
        );
    }

    let baru: Vec<String> = ctx
        .args
        .iter()
        .filter(|p| !p.is_empty() && p.len() <= 5 && !prefixes_arr(data).contains(p))
        .cloned()
        .collect();

    if baru.is_empty() {
        return Reply::text("❌ Tidak ada prefix baru yang valid!");
    }

    let mut data = data.clone();
    let mut semua = prefixes_arr(&data);
    for p in &baru {
        if !semua.contains(p) {
            semua.push(p.clone());
        }
    }
    if let Some(obj) = data.as_object_mut() {
        obj.insert("prefixes".to_string(), Value::Array(semua.iter().map(|s| Value::String(s.clone())).collect()));
    }
    simpan_awalan(ctx, &data);

    let all = semua_awalan(ctx, &data);
    let noprefix_suffix = if noprefix_aktif(&data) {
        "\n> + *noprefix* aktif"
    } else {
        ""
    };
    Reply::text(format!(
        // "ᴅɪᴛᴀᴍʙᴀʜᴋᴀɴ" = U+029C (ʜ), persis `prefix.js:98`.
        "✅ *ᴘʀᴇғɪx ᴅɪᴛᴀᴍʙᴀʜᴋᴀɴ*\n\n> Added: `{}`\n\n*Semua prefix aktif:*\n> `{}`{}",
        baru.join("` `"),
        all.join("` `"),
        noprefix_suffix
    ))
}

/// `prefix.js:107-151` — setprefix/gantiprefix.
fn cmd_ganti_awalan(ctx: &Ctx, data: &Value) -> Reply {
    if ctx.args.is_empty() {
        return Reply::text(format!(
            "🔄 *ɢᴀɴᴛɪ/sᴇᴛ ᴘʀᴇғɪx*\n\n> Ganti semua prefix dengan yang baru\n\n*Format:*\n> `{0}{1} <prefix1> <prefix2> ...`\n\n*Contoh:*\n> `{0}{1} ! G #`\n\n*Special:*\n> `{0}{1} <noprefix>` - Tanpa prefix saja\n> `{0}{1} . <noprefix>` - Prefix . + noprefix\n\n⚠️ Ini akan menghapus semua prefix lama di database!",
            ctx.prefix, ctx.command
        ));
    }

    let has_noprefix = ctx.args.iter().any(|a| a == "<noprefix>" || a == "noprefix");
    let baru: Vec<String> = ctx
        .args
        .iter()
        .filter(|p| *p != "<noprefix>" && *p != "noprefix" && !p.is_empty() && p.len() <= 5)
        .cloned()
        .collect();

    let mut data = data.clone();
    let semua_baru: Vec<Value> = baru.iter().map(|s| Value::String(s.clone())).collect();
    if let Some(obj) = data.as_object_mut() {
        obj.insert("prefixes".to_string(), Value::Array(semua_baru));
        obj.insert("noprefix".to_string(), Value::Bool(has_noprefix));
    }
    simpan_awalan(ctx, &data);

    let mut reply_text = "✅ *ᴘʀᴇғɪx ᴅɪɢᴀɴᴛɪ*\n\n".to_string();
    if !baru.is_empty() {
        reply_text.push_str(&format!("> New prefixes: `{}`\n", baru.join("` `")));
    }
    if has_noprefix {
        reply_text.push_str("> *Noprefix: Aktif* (bisa ketik command langsung)\n");
    }
    reply_text.push_str("\n*Semua prefix aktif:*\n");
    let all = semua_awalan(ctx, &data);
    reply_text.push_str(&format!("> `{}`", all.join("` `")));
    if has_noprefix {
        reply_text.push_str("\n> + *noprefix* aktif");
    }
    Reply::text(reply_text)
}

/// `prefix.js:153-193` — delprefix.
fn cmd_hapus_awalan(ctx: &Ctx, data: &Value) -> Reply {
    if ctx.args.is_empty() {
        return Reply::text(format!(
            "🗑️ *ᴅᴇʟᴇᴛᴇ ᴘʀᴇғɪx*\n\n> Hapus prefix dari database\n\n*Format:*\n> `{0}delprefix <prefix1> <prefix2> ...`\n\n*Contoh:*\n> `{0}delprefix ! $`\n> `{0}delprefix <noprefix>` - Nonaktifkan noprefix",
            ctx.prefix
        ));
    }

    if ctx.args.iter().any(|a| a == "<noprefix>" || a == "noprefix") {
        let mut data = data.clone();
        if let Some(obj) = data.as_object_mut() {
            obj.insert("noprefix".to_string(), Value::Bool(false));
        }
        simpan_awalan(ctx, &data);
        return Reply::text("✅ *ɴᴏᴘʀᴇғɪx ᴅɪɴᴏɴᴀᴋᴛɪғᴋᴀɴ*");
    }

    let mut data = data.clone();
    let semua = prefixes_arr(&data);
    let mut deleted: Vec<String> = Vec::new();
    let sisa: Vec<String> = semua.into_iter().filter(|p| {
        if ctx.args.contains(p) {
            deleted.push(p.clone());
            false
        } else {
            true
        }
    }).collect();

    if let Some(obj) = data.as_object_mut() {
        obj.insert("prefixes".to_string(), Value::Array(sisa.iter().map(|s| Value::String(s.clone())).collect()));
    }
    simpan_awalan(ctx, &data);

    let all = semua_awalan(ctx, &data);
    let noprefix_suffix = if noprefix_aktif(&data) {
        "\n> + *noprefix* aktif"
    } else {
        ""
    };
    Reply::text(format!(
        "✅ *ᴘʀᴇғɪx ᴅɪʜᴀᴘᴜs*\n\n> Deleted: `{}`\n\n*Semua prefix aktif:*\n> `{}`{}",
        if deleted.is_empty() { "None".to_string() } else { deleted.join("` `") },
        all.join("` `"),
        noprefix_suffix
    ))
}

/// `prefix.js:195-219` — listprefix.
fn daftar_awalan(ctx: &Ctx, data: &Value) -> String {
    let all = semua_awalan(ctx, data);
    let mut text = format!(
        "📋 *ʟɪsᴛ ᴘʀᴇғɪx*\n\n╭┈┈⬡「 ⚙️ *ᴄᴏɴғɪɢ* 」\n┃ Default: `{}`\n┃ Noprefix: {}\n╰┈┈┈┈┈┈┈┈⬡\n\n",
        CONFIG_PREFIX,
        if noprefix_aktif(data) { "✅ Aktif" } else { "❌ Nonaktif" }
    );

    let db = prefixes_arr(data);
    if !db.is_empty() {
        text.push_str("╭┈┈⬡「 📁 *ᴅᴀᴛᴀʙᴀsᴇ* 」\n");
        for (i, p) in db.iter().enumerate() {
            text.push_str(&format!("┃ {}.\t`{}`\n", i + 1, p));
        }
        text.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    }

    text.push_str(&format!("*Total prefix aktif:* {}", all.len()));
    if noprefix_aktif(data) {
        text.push_str(" + noprefix");
    }
    text.push_str(&format!("\n> `{}`", all.join("` `")));
    text
}

/// `prefix.js:221-234` — resetprefix.
fn cmd_reset_awalan(ctx: &Ctx, data: &Value) -> Reply {
    let mut data = data.clone();
    if let Some(obj) = data.as_object_mut() {
        obj.insert("prefixes".to_string(), Value::Array(Vec::new()));
        obj.insert("noprefix".to_string(), Value::Bool(false));
    }
    simpan_awalan(ctx, &data);

    Reply::text(format!(
        "✅ *ᴘʀᴇғɪx ᴅɪʀᴇsᴇᴛ*\n\n> Semua prefix di database dihapus!\n> Noprefix dinonaktifkan!\n> Hanya tersisa prefix dari config.js\n\n*Prefix aktif:* `{}`",
        CONFIG_PREFIX
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::berkas::BerkasPalsu;
    use std::sync::Arc;

    fn ctx() -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "addprefix".into(),
            berkas: Arc::new(BerkasPalsu::baru()),
            ..Ctx::default()
        }
    }

    #[test]
    fn muat_awalan_default_kalau_belum_ada() {
        let c = ctx();
        let d = muat_awalan(&c);
        assert_eq!(prefixes_arr(&d).len(), 0);
        assert!(!noprefix_aktif(&d));
    }

    #[test]
    fn tambah_prefix_baru() {
        // `prefix.js:61-64`: tanpa argumen handler hanya menampilkan bantuan.
        let c = Ctx {
            args: vec!["!".into()],
            ..ctx()
        };
        let r = cmd_addprefix(&c, &muat_awalan(&c));
        assert!(r.text_body().unwrap().contains("ᴘʀᴇғɪx ᴅɪᴛᴀᴍʙᴀʜᴋᴀɴ"));
    }

    #[test]
    fn daftar_prefix_tampil_database_dan_config() {
        let c = ctx();
        let data = muat_awalan(&c);
        let text = daftar_awalan(&c, &data);
        assert!(text.contains("Default: `.`"));
        assert!(text.contains("*Total prefix aktif:* 1"));
    }
}
