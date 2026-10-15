//! `plugins/group/autoreply.js` — kelola trigger balasan otomatis per grup,
//! global, dan mode private.
//!
//! Yang tidak dipindah:
//! - Unduhan gambar (`m.quoted.download()` `:271`, `m.download()` `:277`) dan
//!   deteksi gambar langsung (`m.isImage`/`m.type` `:267`) tidak ada di `Ctx`
//!   (BLOKIR §1). JS sendiri menelan kegagalan unduh (`catch` →
//!   `console.error`, `:272-274`) lalu lanjut menyimpan trigger tanpa gambar —
//!   jalur gagal-unduh itulah yang selalu diambil di sini: `image` tetap
//!   `null` dan baris `• Image: ✅ Tersimpan` tidak pernah muncul.
//! - `fs`/`path` (`:23-27`, `:284-287`, `:309-313`, `:351-357`, `:411-417`)
//!   tidak di-port (BLOKIR §4): berkas media tidak dibuat/dihapus. Tidak
//!   mengubah balasan apa pun — JS membungkus semua `unlinkSync` dalam
//!   `try/catch` kosong — hanya berkas lama yang tidak ikut terhapus.
//! - `m.body` direkonstruksi lewat `common::body_stripped` (selisih satu
//!   karakter, lihat doc helper-nya); ekstraksi trigger/reply tetap sama.
//! - Fallback `config.features?.smartTriggers` (`:177`) = literal `false`
//!   statis di `config.js`, jadi `?? false` di ujung sudah mencakupnya.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::common::{body_stripped, js_truthy, now_ms};

/// `defaultTriggers` (`autoreply.js:373-378`).
const DEFAULT_TRIGGERS: [(&str, &str); 4] = [
    ("@mention", "👋 Hai! Ada yang manggil bot?"),
    ("p", "💬 Budayakan salam sebelum percakapan!"),
    ("bot", "🤖 Bot aktif dan siap!"),
    ("assalamualaikum", "Waalaikumsalam saudaraku"),
];

/// `String.prototype.substring(0, n)` memotong pada unit UTF-16, bukan char
/// atau byte — emoji dihitung dua unit. Hasil: teks terpotong dan apakah
/// `length > n` (penentu sufiks `...`).
fn potong_utf16(s: &str, n: usize) -> (String, bool) {
    let mut out = String::new();
    let mut units = 0usize;
    for c in s.chars() {
        let w = c.len_utf16();
        if units + w > n {
            return (out, true);
        }
        out.push(c);
        units += w;
    }
    (out, false)
}

/// `db.setting('autoreplyPrivate') ?? false` — kebenaran JS (`:34`, `:55`),
/// bukan `as_bool`: string `"0"` tetap truthy di JS.
fn setting_privat(ctx: &Ctx) -> bool {
    js_truthy(ctx.setting("autoreplyPrivate"))
}

/// `db.setting('smartTriggers') ?? config.features?.smartTriggers ?? false`
/// (`:177`) — fallback config bernilai `false` statis, jadi kebenaran nilai
/// store sudah cukup; tidak ada (atau `null`) berarti `false`.
fn global_smart(ctx: &Ctx) -> bool {
    js_truthy(ctx.setting("smartTriggers"))
}

/// `groupData.customReplies || []` (`:182`, `:289`, `:344`, `:371`).
fn custom_replies(ctx: &Ctx) -> Vec<Value> {
    ctx.group_field("customReplies")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// Informasi mode private saat dipakai di chat pribadi (`:161-170`).
fn info_private(ctx: &Ctx) -> String {
    format!(
        "📱 *SISTEM AUTOREPLY*\n\n\
         Autoreply Private: *{status}*\n\n\
         *PERINTAH TERSEDIA:*\n\
         • *{p}autoreply private on/off* — Toggle private\n\
         • *{p}autoreply global add/del/list* — Global triggers\n\n\
         _Catatan: Untuk setting autoreply grup, gunakan perintah ini di dalam grup._",
        status = if setting_privat(ctx) { "✅ AKTIF" } else { "❌ NONAKTIF" },
        p = ctx.prefix,
    )
}

/// Cabang status grup (`:179-213`).
fn status_text(ctx: &Ctx, group_status: Option<&Value>, custom_len: usize, efektif: bool) -> String {
    let mut text = format!(
        "🤖 *SISTEM AUTOREPLY GRUP*\n\n\
         Status Global: *{global}*\n\
         Status Grup Ini: *{grup}*\n\
         Status Private: *{privat}*\n\
         Efektif di Grup: *{efektif}*\n\
         Total Custom Reply (Grup): *{custom_len}*\n\n\
         *MANAJEMEN GRUP:*\n\
         • *{p}autoreply on* — Aktifkan di grup ini\n\
         • *{p}autoreply off* — Nonaktifkan di grup ini\n\
         • *{p}autoreply add <trigger>|<reply>* — Tambah custom reply\n\
         • *{p}autoreply del <trigger>* — Hapus custom reply\n\
         • *{p}autoreply list* — Lihat semua trigger di grup ini\n\
         • *{p}autoreply reset* — Hapus SEMUA custom di grup ini\n\n",
        global = if global_smart(ctx) { "✅ AKTIF" } else { "❌ NONAKTIF" },
        // `groupStatus === undefined` — ketat: `null` tersimpan tetap NONAKTIF.
        grup = match group_status {
            None => "DEFAULT".to_string(),
            Some(v) => {
                if js_truthy(Some(v)) {
                    "✅ AKTIF".to_string()
                } else {
                    "❌ NONAKTIF".to_string()
                }
            }
        },
        privat = if setting_privat(ctx) { "✅ AKTIF" } else { "❌ NONAKTIF" },
        efektif = if efektif { "✅ AKTIF" } else { "❌ NONAKTIF" },
        custom_len = custom_len,
        p = ctx.prefix,
    );
    if ctx.is_owner {
        text.push_str(&format!(
            "*MANAJEMEN GLOBAL (OWNER):*\n\
             • *{p}autoreply global add <trigger>|<reply>*\n\
             • *{p}autoreply global del <trigger>*\n\
             • *{p}autoreply global list* — Trigger yang berlaku aktif\n\
             • *{p}autoreply private on/off* — Toggle bot reply di DM\n\n",
            p = ctx.prefix,
        ));
    }
    text.push_str(&format!(
        "*CARA PENAMBAHAN GAMBAR:*\n\
         1. Kirim gambar beserta caption: *{p}autoreply add trigger|reply*\n\
         2. Atau reply gambar dengan: *{p}autoreply add trigger|reply*\n\n\
         *DAPAT MENGGUNAKAN PLACEHOLDER:*\n\
         {{name}} • {{tag}} • {{sender}} • {{botname}} • {{time}} • {{date}}",
        p = ctx.prefix,
    ));
    text
}

/// Pesan format salah cabang `add` grup (`:232-247`).
fn format_salah_grup(prefix: &str) -> String {
    format!(
        "❌ *FORMAT SALAH*\n\n\
         Gunakan format: *trigger|reply*\n\n\
         *Text Only:*\n\
         • {prefix}ar add halo|Hai {{name}}! 👋\n\n\
         *Dengan Gambar:*\n\
         1. Reply gambar + {prefix}ar add trigger|caption\n\
         2. Kirim gambar + caption {prefix}ar add trigger|caption\n\n\
         *Placeholder:*\n\
         • {{name}} - Nama user\n\
         • {{tag}} - Tag @user\n\
         • {{sender}} - Nomor user\n\
         • {{botname}} - Nama bot\n\
         • {{time}} - Waktu sekarang\n\
         • {{date}} - Tanggal sekarang"
    )
}

#[plugin(
    name = "autoreply",
    alias = ["smarttrigger", "smarttriggers", "ar"],
    category = "group",
    description = "Mengatur autoreply/smart triggers per grup",
    usage = ".autoreply on/off/add/del/list/private",
    example = ".autoreply on",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn autoreply(ctx: Ctx) -> Reply {
    let action = ctx.args.first().map(|a| a.to_lowercase());

    // ---- cabang `private` (`:36-63`) — milik owner -------------------------
    if action.as_deref() == Some("private") {
        if !ctx.is_owner {
            return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Hanya owner yang bisa mengatur autoreply private!");
        }
        return match ctx.args.get(1).map(|a| a.to_lowercase()).as_deref() {
            Some("on") => Reply::text(
                "✅ *ᴀᴜᴛᴏʀᴇᴘʟʏ ᴘʀɪᴠᴀᴛᴇ ᴅɪᴀᴋᴛɪꜰᴋᴀɴ*\n\n> Bot akan merespon otomatis di private chat",
            )
            .with_reaction("✅")
            .with_mutation(Mutation::SetSetting {
                key: "autoreplyPrivate".into(),
                value: json!(true),
            }),
            Some("off") => Reply::text(
                "❌ *ᴀᴜᴛᴏʀᴇᴘʟʏ ᴘʀɪᴠᴀᴛᴇ ᴅɪɴᴏɴᴀᴋᴛɪꜰᴋᴀɴ*\n\n> Bot tidak akan merespon otomatis di private chat",
            )
            .with_reaction("❌")
            .with_mutation(Mutation::SetSetting {
                key: "autoreplyPrivate".into(),
                value: json!(false),
            }),
            _ => Reply::text(format!(
                "📱 *AUTOREPLY PRIVATE*\n\n\
                 Status: *{status}*\n\n\
                 *PERINTAH TERSEDIA:*\n\
                 • *{p}autoreply private on* — Aktifkan private\n\
                 • *{p}autoreply private off* — Nonaktifkan private",
                status = if setting_privat(&ctx) { "✅ AKTIF" } else { "❌ NONAKTIF" },
                p = ctx.prefix,
            )),
        };
    }

    // ---- cabang `global` (`:65-159`) — milik owner -------------------------
    if action.as_deref() == Some("global") {
        if !ctx.is_owner {
            return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Hanya owner yang bisa mengatur global autoreply!");
        }
        let sub = ctx.args.get(1).map(|a| a.to_lowercase());
        let mut list = ctx
            .setting("globalCustomReplies")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        if sub.as_deref() == Some("add") {
            let body = body_stripped(&ctx);
            let Some(pipe) = body.find('|') else {
                return Reply::text(format!(
                    "❌ *ꜰᴏʀᴍᴀᴛ sᴀʟᴀʜ*\n\n\
                     > Gunakan format: `trigger|reply`\n\n\
                     > Contoh:\n\
                     > `{p}autoreply global add halo|Hai {{name}}!`",
                    p = ctx.prefix,
                ));
            };
            // `indexOf('global add ') + 11` — `-1 + 11 = 10` di JS kalau tidak
            // ketemu; tidak ada jalur masuk yang menghasilkannya, tapi
            // aritmetikanya dipertahankan.
            let lower = body.to_lowercase();
            let start = lower.find("global add ").map_or(10, |i| i + 11);
            let trigger = body.get(start..pipe).unwrap_or("").trim().to_string();
            let reply = body.get(pipe + 1..).unwrap_or("").to_string();
            if trigger.is_empty() || reply.is_empty() {
                return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Trigger dan reply tidak boleh kosong!");
            }
            // Duplikat: hanya `reply` yang diganti, posisi dan nilai trigger
            // lama dipertahankan (`:96`).
            let posisi = list.iter().position(|r| {
                r.get("trigger")
                    .and_then(Value::as_str)
                    .map(|t| t.to_lowercase())
                    .as_deref()
                    == Some(trigger.to_lowercase()).as_deref()
            });
            match posisi {
                Some(i) => {
                    if let Some(obj) = list[i].as_object_mut() {
                        obj.insert("reply".into(), json!(reply));
                    }
                }
                None => list.push(json!({ "trigger": trigger.to_lowercase(), "reply": reply })),
            }
            return Reply::text(format!(
                "✅ *GLOBAL AUTOREPLY DITAMBAHKAN*\n\n\
                 • Trigger: *{trigger}*\n\
                 • Total: *{len}* replies\n\n\
                 _Aktif di semua grup dan private chat_",
                len = list.len(),
            ))
            .with_reaction("✅")
            .with_mutation(Mutation::SetSetting {
                key: "globalCustomReplies".into(),
                value: Value::Array(list),
            });
        }

        if matches!(sub.as_deref(), Some("del") | Some("rm")) {
            let trigger = ctx.args[2..].join(" ").to_lowercase().trim().to_string();
            if trigger.is_empty() {
                return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Masukkan trigger yang mau dihapus!");
            }
            let posisi = list
                .iter()
                .position(|r| r.get("trigger").and_then(Value::as_str) == Some(trigger.as_str()));
            let Some(i) = posisi else {
                return Reply::text(format!("❌ *ɢᴀɢᴀʟ*\n\n> Trigger `{trigger}` tidak ditemukan!"));
            };
            list.remove(i);
            return Reply::text(format!(
                "🗑️ *GLOBAL AUTOREPLY DIHAPUS*\n\nTrigger *{trigger}* berhasil dihapus!"
            ))
            .with_reaction("🗑️")
            .with_mutation(Mutation::SetSetting {
                key: "globalCustomReplies".into(),
                value: Value::Array(list),
            });
        }

        if sub.as_deref() == Some("list") || sub.is_none() {
            if list.is_empty() {
                return Reply::text(format!(
                    "📋 *GLOBAL AUTOREPLY*\n\n\
                     Status: *❌ TIDAK ADA DATA*\n\n\
                     *PERINTAH TERSEDIA:*\n\
                     • *{p}autoreply global add <trigger>|<reply>*",
                    p = ctx.prefix,
                ));
            }
            let mut text = format!(
                "📋 *GLOBAL AUTOREPLY*\n\n\
                 Total: *{len}* replies\n\
                 Berlaku di: *Semua Grup & Private Chat*\n\n\
                 *DAFTAR TRIGGER:*\n",
                len = list.len(),
            );
            for (i, r) in list.iter().enumerate() {
                let has_image = if js_truthy(r.get("image")) { "🖼️" } else { "" };
                let trigger = match r.get("trigger") {
                    Some(Value::String(s)) => s.clone(),
                    Some(Value::Number(n)) => n.to_string(),
                    Some(Value::Bool(b)) => b.to_string(),
                    _ => "undefined".to_string(),
                };
                let reply = r.get("reply").and_then(Value::as_str).unwrap_or("");
                let (potong, lebih) = potong_utf16(reply, 30);
                text.push_str(&format!(
                    "{n}. *{trigger}* {has_image}\n   ↳ {potong}{}\n\n",
                    if lebih { "..." } else { "" },
                    n = i + 1,
                ));
            }
            return Reply::text(text.trim().to_string());
        }

        return Reply::text(format!(
            "📱 *ɢʟᴏʙᴀʟ ᴀᴜᴛᴏʀᴇᴘʟʏ*\n\n\
             > `{p}autoreply global add trigger|reply`\n\
             > `{p}autoreply global del trigger`\n\
             > `{p}autoreply global list`",
            p = ctx.prefix,
        ));
    }

    // ---- di luar grup: info saja (`:161-170`) ------------------------------
    if !ctx.is_group {
        return Reply::text(info_private(&ctx));
    }

    // ---- sisanya perlu admin grup (`:172-174`) -----------------------------
    if !ctx.roster.is_admin(&ctx.sender) && !ctx.is_owner {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Hanya admin yang bisa mengatur autoreply di grup!");
    }

    let group_status = ctx.group_field("autoreply");

    // ---- status (`:179-213`) ----------------------------------------------
    if action.is_none() || action.as_deref() == Some("status") {
        // `groupStatus ?? globalSmartTriggers` — hanya undefined/null yang
        // jatuh ke global; `false` dan `0` tetap NONAKTIF.
        let efektif = match group_status {
            None | Some(Value::Null) => global_smart(&ctx),
            Some(v) => js_truthy(Some(v)),
        };
        return Reply::text(status_text(&ctx, group_status, custom_replies(&ctx).len(), efektif));
    }

    match action.as_deref() {
        // ---- on / off (`:215-225`) ----------------------------------------
        Some("on") => {
            Reply::text("✅ *ᴀᴜᴛᴏʀᴇᴘʟʏ ᴅɪᴀᴋᴛɪꜰᴋᴀɴ*\n\n> Bot akan merespon otomatis di grup ini")
                .with_reaction("✅")
                .with_mutation(Mutation::SetGroup {
                    jid: ctx.chat.clone(),
                    data: json!({ "autoreply": true }),
                })
        }
        Some("off") => {
            Reply::text("❌ *ᴀᴜᴛᴏʀᴇᴘʟʏ ᴅɪɴᴏɴᴀᴋᴛɪꜰᴋᴀɴ*\n\n> Bot tidak akan merespon otomatis di grup ini")
                .with_reaction("❌")
                .with_mutation(Mutation::SetGroup {
                    jid: ctx.chat.clone(),
                    data: json!({ "autoreply": false }),
                })
        }

        // ---- add (`:227-335`) ---------------------------------------------
        Some("add") => {
            let body = body_stripped(&ctx);
            let Some(pipe) = body.find('|') else {
                return Reply::text(format_salah_grup(&ctx.prefix));
            };
            // `indexOf('add ') + 4` — `-1 + 4 = 3` di JS kalau tidak ketemu.
            let lower = body.to_lowercase();
            let start = lower.find("add ").map_or(3, |i| i + 4);
            let trigger = body.get(start..pipe).unwrap_or("").trim().to_string();
            if trigger.is_empty() {
                return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Trigger tidak boleh kosong!");
            }
            let reply = body.get(pipe + 1..).unwrap_or("").to_string();

            let mut list = custom_replies(&ctx);
            let posisi = list.iter().position(|r| {
                r.get("trigger")
                    .and_then(Value::as_str)
                    .map(|t| t.to_lowercase())
                    .as_deref()
                    == Some(trigger.to_lowercase()).as_deref()
            });
            // Unduhan gambar tidak di-port — `imagePath` selalu null (jalur
            // gagal-unduh JS, `:272-281`). Entri lama DIGANTI DI TEMPAT
            // (`customReplies[existingIndex] = replyData`, `:314`), dan
            // `keepsOldImage = !imagePath && !!previousImage` berarti gambar
            // lama (kalau ada, dari era JS) tetap dipakai.
            let mut image = Value::Null;
            if let Some(i) = posisi {
                let lama = list[i].get("image").cloned().unwrap_or(Value::Null);
                if js_truthy(Some(&lama)) {
                    image = lama;
                }
            }
            let reply_data = json!({
                "trigger": trigger.to_lowercase(),
                "reply": reply,
                "image": image,
                "createdAt": now_ms(),
            });
            match posisi {
                Some(i) => list[i] = reply_data,
                None => list.push(reply_data),
            }

            let mut detail =
                format!("✅ *AUTOREPLY DITAMBAHKAN*\n\n*DETAIL:*\n• Trigger: *{trigger}*\n");
            if !reply.is_empty() {
                let (potong, lebih) = potong_utf16(&reply, 50);
                detail.push_str(&format!("• Reply: {potong}{}\n", if lebih { "..." } else { "" }));
            }
            detail.push_str(&format!("\nTotal: *{}* replies di grup ini", list.len()));
            Reply::text(detail)
                .with_reaction("✅")
                .with_mutation(Mutation::SetGroup {
                    jid: ctx.chat.clone(),
                    data: json!({ "customReplies": list }),
                })
        }

        // ---- del / rm / remove (`:337-368`) -------------------------------
        Some("del") | Some("rm") | Some("remove") => {
            let trigger = ctx.args[1..].join(" ").to_lowercase().trim().to_string();
            if trigger.is_empty() {
                return Reply::text(format!(
                    "❌ *ɢᴀɢᴀʟ*\n\n> Masukkan trigger yang mau dihapus!\n\n`{p}autoreply del halo`",
                    p = ctx.prefix,
                ));
            }
            let mut list = custom_replies(&ctx);
            let posisi = list
                .iter()
                .position(|r| r.get("trigger").and_then(Value::as_str) == Some(trigger.as_str()));
            let Some(i) = posisi else {
                return Reply::text(format!("❌ *ɢᴀɢᴀʟ*\n\n> Trigger `{trigger}` tidak ditemukan!"));
            };
            // `unlinkSync` pada berkas gambar (`:351-357`) tidak di-port —
            // di JS dibungkus `try/catch` kosong, balasan tidak berubah.
            list.remove(i);
            Reply::text(format!(
                "🗑️ *AUTOREPLY DIHAPUS*\n\nTrigger *{trigger}* berhasil dihapus!\nSisa: *{}* replies",
                list.len()
            ))
            .with_reaction("🗑️")
            .with_mutation(Mutation::SetGroup {
                jid: ctx.chat.clone(),
                data: json!({ "customReplies": list }),
            })
        }

        // ---- list (`:370-407`) --------------------------------------------
        Some("list") => {
            let list = custom_replies(&ctx);
            let mut text = String::from("📋 *DAFTAR AUTOREPLY GRUP*\n\n*DEFAULT TRIGGERS:*\n");
            for (t, r) in DEFAULT_TRIGGERS {
                text.push_str(&format!("• *{t}*\n  ↳ {r}\n"));
            }
            text.push('\n');
            if !list.is_empty() {
                text.push_str("*CUSTOM TRIGGERS:*\n");
                for r in &list {
                    let has_image = if js_truthy(r.get("image")) { "🖼️" } else { "" };
                    let trigger = match r.get("trigger") {
                        Some(Value::String(s)) => s.clone(),
                        Some(Value::Number(n)) => n.to_string(),
                        Some(Value::Bool(b)) => b.to_string(),
                        _ => "undefined".to_string(),
                    };
                    text.push_str(&format!("• *{trigger}* {has_image}\n"));
                    if js_truthy(r.get("reply")) {
                        let reply = r.get("reply").and_then(Value::as_str).unwrap_or("");
                        let (potong, lebih) = potong_utf16(reply, 35);
                        text.push_str(&format!("  ↳ {potong}{}\n", if lebih { "..." } else { "" }));
                    }
                }
                text.push('\n');
            } else {
                text.push_str("*CUSTOM TRIGGERS:*\n_Belum ada custom trigger di grup ini_\n\n");
            }
            text.push_str("_Catatan: Default triggers bawaan bot tidak bisa di-edit._");
            Reply::text(text)
        }

        // ---- reset / clear (`:409-422`) -----------------------------------
        Some("reset") | Some("clear") => {
            // `unlinkSync` per entri (`:411-417`) tidak di-port.
            Reply::text("🗑️ *ᴀᴜᴛᴏʀᴇᴘʟʏ ᴅɪʀᴇsᴇᴛ*\n\n> Semua autoreply custom dihapus!")
                .with_reaction("🗑️")
                .with_mutation(Mutation::SetGroup {
                    jid: ctx.chat.clone(),
                    data: json!({ "customReplies": [] }),
                })
        }

        _ => Reply::text(
            "❌ *ᴀᴄᴛɪᴏɴ ᴛɪᴅᴀᴋ ᴠᴀʟɪᴅ*\n\n> Gunakan: `on`, `off`, `private on/off`, `add`, `del`, `list`, `reset`",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    /// Konteks grup dengan sender admin; `command = "ar"` untuk melatih
    /// `body_stripped`.
    fn ctx_grup(args: &[&str], text: &str) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "ar".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            args: args.iter().map(|s| s.to_string()).collect(),
            text: text.into(),
            is_group: true,
            roster: oktz_plugin_api::registry::GroupRoster {
                participants: vec![oktz_plugin_api::Participant {
                    jid: "628111@s.whatsapp.net".into(),
                    is_admin: true,
                    is_owner: false,
                    data: Value::Null,
                }],
                is_bot_admin: true,
            },
            ..Default::default()
        }
    }

    fn ctx_owner(mut c: Ctx) -> Ctx {
        c.is_owner = true;
        c
    }

    fn run(ctx: Ctx) -> Reply {
        (find("autoreply").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    fn reaction_of(r: &Reply) -> &str {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => emoji,
            other => panic!("harus dibungkus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("autoreply").unwrap();
        assert_eq!(p.meta.name, "autoreply");
        assert_eq!(p.meta.alias, ["smarttrigger", "smarttriggers", "ar"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Mengatur autoreply/smart triggers per grup");
        assert_eq!(p.meta.usage, ".autoreply on/off/add/del/list/private");
        assert_eq!(p.meta.example, ".autoreply on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin && !p.meta.is_owner);
        assert!(!p.meta.usage.is_empty() && !p.meta.example.is_empty());
    }

    #[test]
    fn every_alias_resolves_here() {
        for a in ["smarttrigger", "smarttriggers", "ar"] {
            assert_eq!(find(a).map(|e| e.meta.name.as_str()), Some("autoreply"), "{a}");
        }
    }

    // ---- private -----------------------------------------------------------

    #[test]
    fn private_on_writes_the_setting_and_reacts() {
        let r = run(ctx_owner(ctx_grup(&["private", "on"], "private on")));
        assert_eq!(reaction_of(&r), "✅");
        assert!(text_of(&r).contains("ᴘʀɪᴠᴀᴛᴇ ᴅɪᴀᴋᴛɪꜰᴋᴀɴ"));
        assert_eq!(
            r.mutations(),
            &[Mutation::SetSetting {
                key: "autoreplyPrivate".into(),
                value: json!(true),
            }]
        );
    }

    #[test]
    fn private_status_shows_the_stored_value() {
        let mut c = ctx_grup(&["private"], "private");
        c.is_owner = true;
        c.settings = json!({ "autoreplyPrivate": true });
        assert!(text_of(&run(c)).contains("Status: *✅ AKTIF*"));

        let mut c = ctx_grup(&["private"], "private");
        c.is_owner = true;
        assert!(text_of(&run(c)).contains("Status: *❌ NONAKTIF*"));
    }

    #[test]
    fn private_branch_refuses_non_owners() {
        let c = ctx_grup(&["private", "on"], "private on");
        assert_eq!(
            text_of(&run(c)),
            "❌ *ɢᴀɢᴀʟ*\n\n> Hanya owner yang bisa mengatur autoreply private!"
        );
    }

    // ---- global ------------------------------------------------------------

    #[test]
    fn global_add_stores_the_pair() {
        let r = run(ctx_owner(ctx_grup(
            &["global", "add", "halo|Hai"],
            "global add halo|Hai",
        )));
        assert_eq!(reaction_of(&r), "✅");
        let t = text_of(&r);
        assert!(t.contains("• Trigger: *halo*"), "{t}");
        assert!(t.contains("• Total: *1* replies"), "{t}");
        assert_eq!(
            r.mutations(),
            &[Mutation::SetSetting {
                key: "globalCustomReplies".into(),
                value: json!([{ "trigger": "halo", "reply": "Hai" }]),
            }]
        );
    }

    #[test]
    fn global_add_without_a_pipe_asks_for_the_format() {
        let c = ctx_owner(ctx_grup(&["global", "add", "halo"], "global add halo"));
        let t = text_of(&run(c));
        assert!(t.contains("*ꜰᴏʀᴍᴀᴛ sᴀʟᴀʜ*"), "{t}");
        assert!(t.contains("`.autoreply global add halo|Hai {name}!`"), "{t}");
    }

    #[test]
    fn global_add_replaces_the_reply_without_touching_the_old_entry() {
        let mut c = ctx_owner(ctx_grup(
            &["global", "add", "HALO|baru"],
            "global add HALO|baru",
        ));
        c.settings = json!({ "globalCustomReplies": [
            { "trigger": "halo", "reply": "lama", "image": "/tmp/x.jpg" },
            { "trigger": "lain", "reply": "x" }
        ] });
        let r = run(c);
        // JS: `globalCustomReplies[existingIndex].reply = reply` — field lain
        // dan urutan tetap.
        assert_eq!(
            r.mutations(),
            &[Mutation::SetSetting {
                key: "globalCustomReplies".into(),
                value: json!([
                    { "trigger": "halo", "reply": "baru", "image": "/tmp/x.jpg" },
                    { "trigger": "lain", "reply": "x" }
                ]),
            }]
        );
        assert!(text_of(&r).contains("• Total: *2* replies"));
    }

    #[test]
    fn global_add_rejects_empty_trigger_or_reply() {
        let c = ctx_owner(ctx_grup(&["global", "add", "|isi"], "global add |isi"));
        assert_eq!(
            text_of(&run(c)),
            "❌ *ɢᴀɢᴀʟ*\n\n> Trigger dan reply tidak boleh kosong!"
        );
    }

    #[test]
    fn global_list_without_data_says_so() {
        let c = ctx_owner(ctx_grup(&["global"], "global"));
        assert!(text_of(&run(c)).contains("*❌ TIDAK ADA DATA*"));
    }

    #[test]
    fn global_list_renders_entries_with_a_utf16_preview() {
        let mut c = ctx_owner(ctx_grup(&["global"], "global"));
        c.settings = json!({ "globalCustomReplies": [{ "trigger": "halo", "reply": "a" }] });
        let t = text_of(&run(c));
        // `m.reply(text.trim())` (`autoreply.js:150`) membuang newline terakhir,
// tapi spasi setelah `*halo*` tetap ada karena `hasImage` kosong.
        assert!(t.contains("1. *halo* \n   ↳ a"), "{t}");
        assert!(t.contains("Total: *1* replies"), "{t}");
        // `autoreply.js:145` memakai bentuk TUNGGAL — beda dari daftar grup.
        assert!(t.contains("*DAFTAR TRIGGER:*\n"), "{t}");
    }

    #[test]
    fn global_del_removes_by_exact_trigger() {
        let mut c = ctx_owner(ctx_grup(&["global", "del", "halo"], "global del halo"));
        c.settings = json!({ "globalCustomReplies": [{ "trigger": "halo", "reply": "x" }] });
        let r = run(c);
        assert_eq!(reaction_of(&r), "🗑️");
        assert_eq!(
            r.mutations(),
            &[Mutation::SetSetting {
                key: "globalCustomReplies".into(),
                value: json!([]),
            }]
        );
    }

    #[test]
    fn global_unknown_sub_shows_the_usage() {
        let c = ctx_owner(ctx_grup(&["global", "apaaja"], "global apaaja"));
        let t = text_of(&run(c));
        assert!(t.starts_with("📱 *ɢʟᴏʙᴀʟ ᴀᴜᴛᴏʀᴇᴘʟʏ*"), "{t}");
    }

    // ---- luar grup & gerbang admin ------------------------------------------

    #[test]
    fn outside_a_group_it_shows_the_private_info() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "ar".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        };
        let t = text_of(&run(ctx));
        assert!(t.starts_with("📱 *SISTEM AUTOREPLY*"), "{t}");
        assert!(t.contains("Autoreply Private: *❌ NONAKTIF*"), "{t}");
    }

    #[test]
    fn group_branches_refuse_non_admins() {
        let mut c = ctx_grup(&["on"], "on");
        c.roster.participants[0].is_admin = false;
        assert_eq!(
            text_of(&run(c)),
            "❌ *ɢᴀɢᴀʟ*\n\n> Hanya admin yang bisa mengatur autoreply di grup!"
        );
    }

    // ---- on / off / status --------------------------------------------------

    #[test]
    fn on_and_off_write_the_group_flag() {
        let r = run(ctx_grup(&["on"], "on"));
        assert_eq!(reaction_of(&r), "✅");
        assert!(text_of(&r).contains("ᴀᴜᴛᴏʀᴇᴘʟʏ ᴅɪᴀᴋᴛɪꜰᴋᴀɴ"));
        assert_eq!(
            r.mutations(),
            &[Mutation::SetGroup {
                jid: "120363@g.us".into(),
                data: json!({ "autoreply": true }),
            }]
        );

        let r = run(ctx_grup(&["off"], "off"));
        assert_eq!(reaction_of(&r), "❌");
        assert_eq!(
            r.mutations(),
            &[Mutation::SetGroup {
                jid: "120363@g.us".into(),
                data: json!({ "autoreply": false }),
            }]
        );
    }

    #[test]
    fn status_defaults_and_falls_back_to_the_global_switch() {
        let t = text_of(&run(ctx_grup(&[], "")));
        assert!(t.contains("Status Global: *❌ NONAKTIF*"), "{t}");
        assert!(t.contains("Status Grup Ini: *DEFAULT*"), "{t}");
        assert!(t.contains("Efektif di Grup: *❌ NONAKTIF*"), "{t}");

        let mut c = ctx_grup(&["status"], "status");
        c.settings = json!({ "smartTriggers": true });
        let t = text_of(&run(c));
        assert!(t.contains("Status Global: *✅ AKTIF*"), "{t}");
        assert!(t.contains("Status Grup Ini: *DEFAULT*"), "{t}");
        assert!(t.contains("Efektif di Grup: *✅ AKTIF*"), "{t}");
    }

    #[test]
    fn stored_null_is_nonaktif_but_still_nullish_for_effective() {
        let mut c = ctx_grup(&[], "");
        c.group_data = json!({ "autoreply": null });
        let t = text_of(&run(c));
        assert!(t.contains("Status Grup Ini: *❌ NONAKTIF*"), "{t}");

        let mut c = ctx_grup(&[], "");
        c.group_data = json!({ "autoreply": null });
        c.settings = json!({ "smartTriggers": true });
        let t = text_of(&run(c));
        assert!(t.contains("Efektif di Grup: *✅ AKTIF*"), "{t}");
    }

    #[test]
    fn stored_false_is_falsy_and_not_nullish() {
        let mut c = ctx_grup(&[], "");
        c.group_data = json!({ "autoreply": false });
        c.settings = json!({ "smartTriggers": true });
        let t = text_of(&run(c));
        assert!(t.contains("Status Grup Ini: *❌ NONAKTIF*"), "{t}");
        assert!(
            t.contains("Efektif di Grup: *❌ NONAKTIF*"),
            "`false` tidak boleh jatuh ke global: {t}"
        );
    }

    #[test]
    fn status_shows_the_owner_section_only_for_owners() {
        let t = text_of(&run(ctx_grup(&[], "")));
        assert!(!t.contains("*MANAJEMEN GLOBAL (OWNER):*"), "{t}");
        let mut c = ctx_grup(&[], "");
        c.is_owner = true;
        assert!(text_of(&run(c)).contains("*MANAJEMEN GLOBAL (OWNER):*"));
    }

    // ---- add (grup) ---------------------------------------------------------

    #[test]
    fn group_add_stores_trigger_reply_and_created_at() {
        let r = run(ctx_grup(
            &["add", "halo|Hai {name}!"],
            "add halo|Hai {name}!",
        ));
        assert_eq!(reaction_of(&r), "✅");
        let t = text_of(&r);
        assert!(t.starts_with("✅ *AUTOREPLY DITAMBAHKAN*"), "{t}");
        assert!(t.contains("• Trigger: *halo*"), "{t}");
        assert!(t.contains("• Reply: Hai {name}!"), "{t}");
        assert!(t.contains("Total: *1* replies di grup ini"), "{t}");
        assert!(!t.contains("• Image:"), "jalur unduh tidak pernah sukses: {t}");
        let map = match &r.mutations()[0] {
            Mutation::SetGroup { data, .. } => data,
            other => panic!("harus SetGroup, dapat {other:?}"),
        };
        let list = map.get("customReplies").unwrap().as_array().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["trigger"], "halo");
        assert_eq!(list[0]["reply"], "Hai {name}!");
        assert!(list[0]["image"].is_null());
        assert!(list[0]["createdAt"].as_i64().unwrap_or(0) > 0);
    }

    #[test]
    fn group_add_without_a_pipe_asks_for_the_format() {
        let t = text_of(&run(ctx_grup(&["add", "halo"], "add halo")));
        assert!(t.starts_with("❌ *FORMAT SALAH*"), "{t}");
        assert!(t.contains("• .ar add halo|Hai {name}! 👋"), "{t}");
    }

    #[test]
    fn group_add_with_an_empty_trigger_is_refused() {
        let t = text_of(&run(ctx_grup(&["add", "|isi"], "add |isi")));
        assert_eq!(t, "❌ *ɢᴀɢᴀʟ*\n\n> Trigger tidak boleh kosong!");
    }

    #[test]
    fn group_add_replaces_in_place_and_keeps_the_old_image() {
        let mut c = ctx_grup(&["add", "halo|baru"], "add halo|baru");
        c.group_data = json!({ "customReplies": [
            { "trigger": "halo", "reply": "lama", "image": "/tmp/lama.jpg", "createdAt": 1 }
        ] });
        let r = run(c);
        let map = match &r.mutations()[0] {
            Mutation::SetGroup { data, .. } => data,
            other => panic!("harus SetGroup, dapat {other:?}"),
        };
        let list = map.get("customReplies").unwrap().as_array().unwrap();
        assert_eq!(list.len(), 1, "diganti di tempat, bukan ditambah");
        assert_eq!(list[0]["reply"], "baru");
        assert_eq!(list[0]["image"], "/tmp/lama.jpg", "gambar lama dipertahankan");
        assert!(text_of(&r).contains("• Reply: baru"));
        assert!(text_of(&r).contains("Total: *1* replies di grup ini"));
    }

    #[test]
    fn group_add_preview_is_cut_at_fifty_utf16_units() {
        let panjang = "x".repeat(60);
        let text = format!("add t|{panjang}");
        let r = run(ctx_grup(&["add", "t"], &text));
        let t = text_of(&r);
        assert!(t.contains(&format!("• Reply: {}...", "x".repeat(50))), "{t}");
        assert!(
            !t.contains(&format!("• Reply: {}x", "x".repeat(51))),
            "lebih dari 50 unit tidak boleh ikut: {t}"
        );
    }

    // ---- del / list / reset --------------------------------------------------

    #[test]
    fn group_del_removes_and_reports_the_remaining_count() {
        let mut c = ctx_grup(&["del", "halo"], "del halo");
        c.group_data = json!({ "customReplies": [
            { "trigger": "halo", "reply": "x" },
            { "trigger": "bye", "reply": "y" }
        ] });
        let r = run(c);
        assert_eq!(reaction_of(&r), "🗑️");
        assert!(text_of(&r).contains("Sisa: *1* replies"));
        let map = match &r.mutations()[0] {
            Mutation::SetGroup { data, .. } => data,
            other => panic!("harus SetGroup, dapat {other:?}"),
        };
        let list = map.get("customReplies").unwrap().as_array().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["trigger"], "bye");
    }

    #[test]
    fn group_del_unknown_trigger_says_so() {
        let t = text_of(&run(ctx_grup(&["del", "gakada"], "del gakada")));
        assert_eq!(t, "❌ *ɢᴀɢᴀʟ*\n\n> Trigger `gakada` tidak ditemukan!");
    }

    #[test]
    fn group_del_without_a_trigger_asks_for_one() {
        let t = text_of(&run(ctx_grup(&["del"], "del")));
        assert!(t.contains("Masukkan trigger yang mau dihapus!"), "{t}");
        assert!(t.contains("`.autoreply del halo`"), "{t}");
    }

    #[test]
    fn group_list_shows_defaults_and_customs() {
        let mut c = ctx_grup(&["list"], "list");
        c.group_data = json!({ "customReplies": [
            { "trigger": "halo", "reply": "hai juga", "image": "/tmp/x.jpg" }
        ] });
        let t = text_of(&run(c));
        assert!(t.starts_with("📋 *DAFTAR AUTOREPLY GRUP*"), "{t}");
        assert!(
            t.contains("• *@mention*\n  ↳ 👋 Hai! Ada yang manggil bot?\n"),
            "{t}"
        );
        assert!(
            t.contains("*CUSTOM TRIGGERS:*\n• *halo* 🖼️\n  ↳ hai juga\n"),
            "{t}"
        );
        assert!(
            t.contains("_Catatan: Default triggers bawaan bot tidak bisa di-edit._"),
            "{t}"
        );
    }

    #[test]
    fn group_list_without_customs_says_empty() {
        let t = text_of(&run(ctx_grup(&["list"], "list")));
        assert!(t.contains("_Belum ada custom trigger di grup ini_"), "{t}");
        assert!(!t.contains("• *halo*"), "{t}");
    }

    #[test]
    fn group_reset_clears_everything() {
        let mut c = ctx_grup(&["reset"], "reset");
        c.group_data = json!({ "customReplies": [{ "trigger": "a", "reply": "b" }] });
        let r = run(c);
        assert_eq!(reaction_of(&r), "🗑️");
        assert_eq!(
            r.mutations(),
            &[Mutation::SetGroup {
                jid: "120363@g.us".into(),
                data: json!({ "customReplies": [] }),
            }]
        );
    }

    #[test]
    fn an_unknown_action_is_rejected() {
        let t = text_of(&run(ctx_grup(&["apaaja"], "apaaja")));
        assert!(t.starts_with("❌ *ᴀᴄᴛɪᴏɴ ᴛɪᴅᴀᴋ ᴠᴀʟɪᴅ*"), "{t}");
    }

    // ---- helper --------------------------------------------------------------

    #[test]
    fn utf16_cutting_matches_javascript_units() {
        let emoji = "🎆".repeat(5);
        assert_eq!(potong_utf16(&emoji, 10), (emoji.clone(), false));
        assert_eq!(potong_utf16(&emoji, 9), ("🎆".repeat(4), true));
        assert_eq!(potong_utf16("halo", 4), ("halo".into(), false));
        assert_eq!(potong_utf16("halo", 3), ("hal".into(), true));
        assert_eq!(potong_utf16("", 3), ("".into(), false));
    }
}
