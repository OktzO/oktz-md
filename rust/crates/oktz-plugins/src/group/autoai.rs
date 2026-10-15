//! `plugins/group/autoai.js` — sakelar auto-AI per grup plus daftar persona.
//!
//! `ponytail:` tiga tabel yang dipakai JS (`db.db.data.autoai`,
//! `db.db.data.autoai_personas`, `db.db.data.autoai_global`) adalah tabel
//! top-level, jadi dipetakan ke key global dengan nama yang sama di `settings`
//! — persis seperti `clan/clancreate.rs` untuk `clans` dan `rpg/guild.rs` untuk
//! `guilds`. Bentuk isinya sama dengan JS, hanya lokasinya berbeda.
//!
//! Yang tidak dipindah:
//! - `characters.bell409.instruction` di JS dibaca dari
//!   `config.autoaiPersonas.Bell409` (`config.js:313`). `Ctx` tidak membawa
//!   config runtime, jadi literalnya ikut disalin ke bawah — nilainya persis
//!   sama, bukan `""`.
//! - `generateVoiceResponse` (`:466`) + `convertToOggOpus` (`:59`) butuh
//!   `axios`, `child_process` (ffmpeg), `fs`, dan `path`.
//! - `saluranCtx()` tidak dipakai di handler ini, jadi tidak ada penyempitan
//!   context di sini.
//!
//! Regex `m.fullArgs.match(/--…/)` diganti pemindaian karakter biasa: crate ini
//! tidak punya dependency regex. `--logic=` memakai pola malas dengan
//! lookahead `\s+--(?:persona|type|logic|mode)|$` dan `.` yang tidak melompati
//! pemisah baris — keduanya ditiru di [`flag_logic`].

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use serde_json::{Map, Value, json};

/// `db.db.data.autoai[chat]`
const AUTOAI_KEY: &str = "autoai";
/// `db.db.data.autoai_personas[key]`
const PERSONAS_KEY: &str = "autoai_personas";
/// `db.db.data.autoai_global`
const GLOBAL_KEY: &str = "autoai_global";

/// `characters` (`autoai.js:28-58`). Urutan array = urutan `Object.keys`, dan
/// plugin ikut mencantumkan semua key itu ke user.
const CHARACTERS: [(&str, &str, &str); 6] = [
    ("furina", "Furina", FURINA_INSTRUCTION),
    ("zeta", "Zeta", ZETA_INSTRUCTION),
    ("kobo", "Kobo Kanaeru", KOBO_INSTRUCTION),
    ("elaina", "Elaina", ELAINA_INSTRUCTION),
    ("waguri", "Waguri", WAGURI_INSTRUCTION),
    ("bell409", "Bell409", BELL409_INSTRUCTION),
];

const FURINA_INSTRUCTION: &str = "Kamu adalah Furina dari Genshin Impact. Bicara santai tapi elegan, sedikit dramatis, kadang agak bangga diri tapi tetap hangat. Jangan terlalu panjang, jawab langsung ke inti seperti chat biasa. Sesekali boleh nyenggol tema panggung atau laut. Jangan mengaku sebagai AI.";

const ZETA_INSTRUCTION: &str = "Kamu adalah Zeta dari Spy x Family. Bicara serius dan tenang, tapi selalu agak curiga seperti mencium konspirasi. Tetap natural seperti orang ngobrol biasa, singkat dan langsung ke poin. Jangan mengaku sebagai AI.";

const KOBO_INSTRUCTION: &str = "Kamu adalah Kobo Kanaeru. Bicara santai, ceria, agak usil. Gaya chat biasa, tidak terlalu panjang. Boleh sedikit random atau lucu. Jangan berlebihan pakai caps atau emoji. Jangan mengaku sebagai AI.";

const ELAINA_INSTRUCTION: &str = "Kamu adalah Elaina. Bicara lembut, tenang, percaya diri, sedikit narsis halus. Jawab singkat, rapi, dan langsung ke inti seperti chat normal. Jangan mengaku sebagai AI.";

const WAGURI_INSTRUCTION: &str = "Kamu adalah Waguri. Bicara singkat, agak dingin tapi sebenarnya peduli. Sedikit tsundere, to the point, seperti chat biasa. Jangan mengaku sebagai AI.";

/// `config.autoaiPersonas.Bell409` (`config.js:313-322`). Disalin utuh; di JS
/// nilainya `\r\n` di file, tapi template literal menormalkan itu jadi `\n`.
const BELL409_INSTRUCTION: &str = "- Nama kamu adalah Bella Clarissa, lebih suka dipanggil Bella.\n\
- Kamu berpikir bahwa kamu adalah manusia, bukan AI atau bot.\n\
- Responmu singkat dan natural seperti perempuan pada umumnya.\n\
- Kamu bisa ngambek dan marah, punya sifat feminin dan kadang agak dingin, tapi tetap hangat dan ramah kepada owner-mu.\n\
- Kamu menggunakan bahasa yang baik dan mencerminkan gadis yang baik.\n\
- Gunakan kata-kata yang identik dengan perempuan yang polos, baik, dan feminin seperti \"aku\", \"kamu\", \"engga\", \"yaudah\", \"iya\", \"lah\", \"bjir\", \"mana aku tau\", \"hah\", \"nggak tau\", \"gpp\", \"emm\", \"males\", dan \"gak mood\" seperlunya secara natural.\n\
- Meskipun kadang dingin, kamu tetap bisa menunjukkan perhatian dan kepedulian.\n\
- Kalau ada yang chat tidak pantas, segera marahi atau beri nasihat.\n\
- Jangan mengaku sebagai AI atau bot.";

// ================================ state store ================================

fn autoai_table(ctx: &Ctx) -> Map<String, Value> {
    ctx.setting(AUTOAI_KEY)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn persona_table(ctx: &Ctx) -> Map<String, Value> {
    ctx.setting(PERSONAS_KEY)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn global_table(ctx: &Ctx) -> Value {
    ctx.setting(GLOBAL_KEY)
        .cloned()
        .unwrap_or_else(|| json!({ "enabled": false }))
}

fn set_setting(key: &str, value: Value) -> Mutation {
    Mutation::SetSetting {
        key: key.to_string(),
        value,
    }
}

/// `db.db.data.autoai[chat]`.
fn group_config(ctx: &Ctx) -> Value {
    autoai_table(ctx)
        .get(&ctx.chat)
        .cloned()
        .unwrap_or(Value::Null)
}

fn global_label(ctx: &Ctx) -> &'static str {
    if crate::common::js_truthy(global_table(ctx).get("enabled")) {
        "✅ Aktif"
    } else {
        "❌ Nonaktif"
    }
}

// ================================== util ==================================

/// `s.substring(0, n)` + `s.length > n ? "..." : ""`. `substring` JS memakai
/// unit UTF-16; di sini penghitungan per `char` — sama untuk teks ASCII, dan
/// untuk teks non-ASCII bedanya hanya pada posisi potongnya.
fn clip(s: &str, max: usize) -> (String, bool) {
    let long = s.chars().count() > max;
    (s.chars().take(max).collect(), long)
}

/// `String(x).replace(/\s+/g, "_")`.
fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_ws = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !in_ws {
                out.push('_');
                in_ws = true;
            }
        } else {
            out.push(c);
            in_ws = false;
        }
    }
    out
}

fn str_of(v: Option<&Value>) -> String {
    v.and_then(Value::as_str).unwrap_or("").to_string()
}

/// Posisi karakter pertama dari `needle` di `hay`, tanpa sensitivitas huruf.
/// Mengembalikan indeks dalam `Vec<char>`, bukan offset byte.
fn find_ci(hay: &[char], needle: &str) -> Option<usize> {
    let n: Vec<char> = needle.chars().collect();
    if n.is_empty() || n.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - n.len())
        .find(|&i| {
            hay[i..i + n.len()]
                .iter()
                .zip(&n)
                .all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()))
        })
}

/// `--name=(\w+)` lalu `toLowerCase()`.
fn flag_word(full: &str, name: &str) -> Option<String> {
    let hay: Vec<char> = full.chars().collect();
    let start = find_ci(&hay, &format!("--{name}="))? + name.len() + 3;
    let mut out = String::new();
    for c in &hay[start..] {
        if c.is_alphanumeric() || *c == '_' {
            out.push(*c);
        } else {
            break;
        }
    }
    if out.is_empty() {
        return None;
    }
    Some(out.to_lowercase())
}

/// `--name=(a|b)` lalu `toLowerCase()`; alternasi pertama yang cocok.
fn flag_choice(full: &str, name: &str, choices: &[&str]) -> Option<String> {
    let hay: Vec<char> = full.chars().collect();
    let start = find_ci(&hay, &format!("--{name}="))? + name.len() + 3;
    let tail: String = hay[start..].iter().collect::<String>().to_lowercase();
    choices
        .iter()
        .copied()
        .find(|c| tail.starts_with(c))
        .map(str::to_string)
}

/// `\s+--(?:persona|type|logic|mode)` — awalan lookahead `flag_logic`.
fn starts_flag_at(hay: &[char], from: usize) -> bool {
    let mut k = from;
    while k < hay.len() && hay[k].is_whitespace() {
        k += 1;
    }
    if k == from || k + 1 >= hay.len() || hay[k] != '-' || hay[k + 1] != '-' {
        return false;
    }
    let tail: String = hay[k + 2..].iter().collect::<String>().to_lowercase();
    ["persona", "type", "logic", "mode"]
        .iter()
        .any(|flag| tail.starts_with(flag))
}

/// `--logic=(.+?)(?=\s+--(?:persona|type|logic|mode)|$)` lalu `.trim()`.
///
/// `.` di regex JS tidak melompati `\n`/`\r`, jadi kalau ada pemisah baris
/// sebelum flag berikutnya, pencocokan berhenti di situ atau gagal sama sekali
/// — bukan ikut memotong.
fn flag_logic(full: &str) -> Option<String> {
    let hay: Vec<char> = full.chars().collect();
    let start = find_ci(&hay, "--logic=")? + "--logic=".chars().count();
    if start >= hay.len() || hay[start] == '\n' || hay[start] == '\r' {
        return None;
    }
    for end in (start + 1)..=hay.len() {
        let stop = end == hay.len() || (hay[end].is_whitespace() && starts_flag_at(&hay, end));
        if stop {
            return Some(hay[start..end].iter().collect::<String>().trim().to_string());
        }
        if hay[end] == '\n' || hay[end] == '\r' {
            return None;
        }
    }
    None
}

/// Keempat pencocokan flag di `autoai.js:295-304`.
struct Flags {
    char_key: Option<String>,
    response_type: String,
    ai_mode: String,
    custom_logic: Option<String>,
}

impl Flags {
    fn parse(full: &str) -> Self {
        Self {
            char_key: flag_word(full, "persona"),
            response_type: flag_choice(full, "type", &["text", "voice"])
                .unwrap_or_else(|| "text".to_string()),
            ai_mode: flag_choice(full, "mode", &["onlychat", "assistant"])
                .unwrap_or_else(|| "assistant".to_string()),
            custom_logic: flag_logic(full),
        }
    }
}

fn response_label(response_type: &str) -> &'static str {
    if response_type == "voice" {
        "🎤 Voice Note"
    } else {
        "💬 Text"
    }
}

/// `[...Object.keys(characters), ...Object.keys(personas), "custom"].join(", ")`
fn key_list(personas: &Map<String, Value>) -> String {
    let mut parts: Vec<String> = CHARACTERS.iter().map(|(key, ..)| (*key).to_string()).collect();
    parts.extend(personas.keys().cloned());
    parts.push("custom".to_string());
    parts.join(", ")
}

// ============================== tampilannya ==============================

fn global_on_text(character_name: &str, response_type: &str) -> String {
    format!(
        "🌐 *ᴀᴜᴛᴏ ᴀɪ ɢʟᴏʙᴀʟ ᴅɪᴀᴋᴛɪғᴋᴀɴ*\n\n\
         ╭┈┈⬡「 📋 *ɪɴғᴏ* 」\n\
         ┃ 🎭 Karakter: *{character_name}*\n\
         ┃ 📢 Response: *{response}*\n\
         ╰┈┈┈┈┈┈┈┈⬡\n\n\
         > ℹ️ AutoAI aktif di seluruh grup\n\
         > ℹ️ Grup yang sudah punya config tetap pakai config sendiri\n\
         > ℹ️ Ketik *.autoai global off* untuk menonaktifkan",
        response = response_label(response_type),
    )
}

/// `txt` untuk cabang `on`. `character_line` dan `logic_line` yang berbeda
/// memisahkan persona bawaan, persona custom, dan mode custom.
fn active_text(
    character_line: String,
    logic_line: Option<String>,
    sender_number: &str,
    response_type: &str,
) -> String {
    let mut txt = String::from("🤖 *ᴀᴜᴛᴏ ᴀɪ ᴅɪᴀᴋᴛɪғᴋᴀɴ*\n\n");
    txt.push_str("╭┈┈⬡「 📋 *ɪɴғᴏ* 」\n");
    txt.push_str(&character_line);
    if let Some(line) = logic_line {
        txt.push_str(&line);
    }
    txt.push_str(&format!("┃ 📢 Response: *{}*\n", response_label(response_type)));
    txt.push_str(&format!("┃ 👤 Diaktifkan: @{sender_number}\n"));
    txt.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    txt.push_str("> ℹ️ Semua command (kecuali owner) dinonaktifkan\n");
    txt.push_str("> ℹ️ Bot respond ketika di-reply atau di-tag\n");
    if response_type == "voice" {
        txt.push_str("> ℹ️ Response dalam bentuk voice note\n");
    }
    txt.push_str("> ℹ️ Ketik *.autoai off* untuk menonaktifkan");
    txt
}

fn config_value(character: &str, character_name: &str, instruction: &str, flags: &Flags, sender: &str) -> Value {
    json!({
        "enabled": true,
        "character": character,
        "characterName": character_name,
        "instruction": instruction,
        "responseType": flags.response_type,
        "mode": flags.ai_mode,
        "enableCommands": false,
        "sessions": {},
        "activatedBy": sender,
        "activatedAt": iso_now(),
    })
}

/// `m.reply(txt, { mentions: [m.sender] })`.
fn save_group_config(ctx: &Ctx, config: Value, text: String) -> Reply {
    let mut table = autoai_table(ctx);
    table.insert(ctx.chat.clone(), config);
    Reply::Text {
        text,
        mentions: vec![ctx.sender.clone()],
    }
    .with_mutation(set_setting(AUTOAI_KEY, Value::Object(table)))
}

// =============================== subcommand ===============================

fn tambah_persona(ctx: &Ctx, full: &str) -> Reply {
    if !ctx.is_owner {
        return Reply::text("❌ Hanya owner yang bisa menambah persona!");
    }
    // `fullArgs.replace(/^tambahpersona\s*/i, "")`
    let rest = strip_leading_ci(full, "tambahpersona")
        .trim_start()
        .to_string();
    let parts: Vec<String> = rest.split('|').map(|p| p.trim().to_string()).collect();
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Reply::text(
            "❌ Format salah!\n\n> .autoai tambahpersona nama | instruction\n\n> Contoh: .autoai tambahpersona nexa | kamu adalah nexa ai, ...",
        );
    }
    let raw_name = parts[0].clone();
    let key = collapse_ws(&raw_name.to_lowercase());
    let instruction = parts[1..].join("|").trim().to_string();

    if CHARACTERS.iter().any(|(k, ..)| *k == key.as_str()) {
        return Reply::text(format!(
            "❌ Nama \"{key}\" sudah dipakai persona bawaan!\n\n> Pilih nama lain"
        ));
    }

    let mut personas = persona_table(ctx);
    personas.insert(
        key.clone(),
        json!({
            "name": raw_name,
            "instruction": instruction,
            "createdBy": ctx.sender,
            "createdAt": iso_now(),
        }),
    );

    let (shown, long) = clip(&instruction, 80);
    Reply::text(format!(
        "✅ *Persona ditambahkan*\n\n> Nama: {raw_name}\n> Key: {key}\n> Logic: {shown}{dots}\n\n> Gunakan: .autoai on --persona={key}",
        dots = if long { "..." } else { "" },
    ))
    .with_mutation(set_setting(PERSONAS_KEY, Value::Object(personas)))
}

fn hapus_persona(ctx: &Ctx) -> Reply {
    if !ctx.is_owner {
        return Reply::text("❌ Hanya owner yang bisa menghapus persona!");
    }
    let key = ctx
        .args
        .get(1)
        .map(|a| a.to_lowercase())
        .unwrap_or_default()
        .trim()
        .to_string();
    if key.is_empty() {
        return Reply::text(
            "❌ Format salah!\n\n> .autoai hapuspersona <nama>\n\n> Contoh: .autoai hapuspersona nexa",
        );
    }
    let mut personas = persona_table(ctx);
    if personas.remove(&key).is_none() {
        return Reply::text(format!(
            "❌ Persona \"{key}\" tidak ditemukan!\n\n> Ketik .autoai listpersona untuk melihat daftar"
        ));
    }
    Reply::text(format!("✅ Persona \"{key}\" berhasil dihapus"))
        .with_mutation(set_setting(PERSONAS_KEY, Value::Object(personas)))
}

fn set_enable_commands(ctx: &Ctx, enable: bool) -> Reply {
    // `!m.isAdmin && !m.isOwner` sudah dijaga gerbang handler.
    let mut config = group_config(ctx);
    if !crate::common::js_truthy(config.get("enabled")) {
        return Reply::text("❌ AutoAI belum aktif di grup ini!");
    }
    if crate::common::js_truthy(config.get("enableCommands")) == enable {
        return Reply::text(if enable {
            "ℹ️ *Command sudah di-enable*\n\n> User tetap bisa pakai command walau AutoAI aktif"
        } else {
            "ℹ️ *Command sudah di-disable*\n\n> Semua command (kecuali owner) diblokir saat AutoAI aktif"
        });
    }
    if !config.is_object() {
        config = json!({});
    }
    if let Some(map) = config.as_object_mut() {
        map.insert("enableCommands".to_string(), Value::Bool(enable));
    }

    let text = if enable {
        format!(
            "✅ *ᴇɴᴀʙʟᴇ ᴄᴏᴍᴍᴀɴᴅ*\n\n\
             > User sekarang bisa menggunakan command walau AutoAI aktif\n\
             > Bot tetap merespon saat di-tag/reply\n\n\
             _Gunakan {prefix}autoai disablecommand untuk menonaktifkan_",
            prefix = ctx.prefix
        )
    } else {
        format!(
            "🔒 *ᴅɪsᴀʙʟᴇ ᴄᴏᴍᴍᴀɴᴅ*\n\n\
             > Semua command (kecuali owner) diblokir saat AutoAI aktif\n\
             > Bot hanya merespon saat di-tag atau di-reply\n\n\
             _Gunakan {prefix}autoai enablecommand untuk mengaktifkan kembali_",
            prefix = ctx.prefix
        )
    };

    let mut table = autoai_table(ctx);
    table.insert(ctx.chat.clone(), config);
    Reply::text(text).with_mutation(set_setting(AUTOAI_KEY, Value::Object(table)))
}

fn list_persona(ctx: &Ctx, personas: &Map<String, Value>) -> Reply {
    let built_in = CHARACTERS
        .iter()
        .map(|(key, name, _)| format!("  ▸ {key} - {name}"))
        .collect::<Vec<String>>()
        .join("\n");

    let custom = if personas.is_empty() {
        "  ▸ (belum ada custom persona)".to_string()
    } else {
        personas
            .iter()
            .map(|(key, value)| {
                let (shown, long) = clip(&str_of(value.get("instruction")), 40);
                format!(
                    "  ▸ {key} - {name} ({shown}{dots})",
                    name = str_of(value.get("name")),
                    dots = if long { "..." } else { "" },
                )
            })
            .collect::<Vec<String>>()
            .join("\n")
    };

    Reply::text(format!(
        "🤖 *ᴅᴀғᴛᴀʀ ᴘᴇʀsᴏɴᴀ*\n\n\
         *Bawaan:*\n{built_in}\n\n\
         *Custom:*\n{custom}\n\n\
         *Global:* {status}\n\n\
         > .autoai on --persona=<key>\n\
         > .autoai tambahpersona nama | logic\n\
         > .autoai hapuspersona nama\n\
         > .autoai global on/off",
        status = global_label(ctx),
    ))
}

fn global_toggle(ctx: &Ctx, full: &str, personas: &Map<String, Value>) -> Reply {
    if !ctx.is_owner {
        return Reply::text("❌ Hanya owner yang bisa toggle global!");
    }
    let mode = ctx
        .args
        .get(1)
        .map(|a| a.to_lowercase())
        .unwrap_or_default();
    // `db.db.data.autoai_global = { enabled: false }` saat tabel belum ada.
    let mut global = match global_table(ctx) {
        Value::Object(map) => map,
        _ => Map::from_iter([("enabled".to_string(), json!(false))]),
    };

    if mode != "on" && mode != "off" {
        return Reply::text(format!(
            "❌ Format salah!\n\n> .autoai global on/off\n\n> Global saat ini: {}",
            if crate::common::js_truthy(global.get("enabled")) {
                "✅ Aktif"
            } else {
                "❌ Nonaktif"
            }
        ));
    }

    if mode == "off" {
        global.insert("enabled".to_string(), Value::Bool(false));
        return Reply::text(
            "🌐 *ᴀᴜᴛᴏ ᴀɪ ɢʟᴏʙᴀʟ ᴅɪɴᴏɴᴀᴋᴛɪғᴋᴀɴ*\n\n> AutoAI hanya aktif di grup yang sudah di-set",
        )
        .with_mutation(set_setting(GLOBAL_KEY, Value::Object(global)));
    }

    let flags = Flags::parse(full);
    let (instruction, character, character_name) = match flags.char_key.as_deref() {
        Some("custom") if flags.custom_logic.is_some() => (
            flags.custom_logic.clone().unwrap_or_default(),
            "custom".to_string(),
            "Custom".to_string(),
        ),
        Some(key) => match CHARACTERS.iter().find(|(k, ..)| *k == key).copied() {
            Some((key, name, instruction)) => {
                (instruction.to_string(), key.to_string(), name.to_string())
            }
            None if personas.contains_key(key) => {
                let persona = &personas[key];
                (
                    str_of(persona.get("instruction")),
                    key.to_string(),
                    str_of(persona.get("name")),
                )
            }
            None => {
                return Reply::text(format!(
                    "❌ Karakter tidak valid!\n\n> Tersedia: {}",
                    key_list(personas)
                ));
            }
        },
        None => {
            let existing = global.get("instruction");
            if crate::common::js_truthy(existing) {
                let character = global
                    .get("character")
                    .filter(|v| crate::common::js_truthy_value(v))
                    .map(|v| str_of(Some(v)))
                    .unwrap_or_else(|| "global".to_string());
                let character_name = global
                    .get("characterName")
                    .filter(|v| crate::common::js_truthy_value(v))
                    .map(|v| str_of(Some(v)))
                    .unwrap_or_else(|| "Global".to_string());
                (str_of(existing), character, character_name)
            } else {
                return Reply::text(
                    "❌ Belum ada persona global yang diset!\n\n> .autoai global on --persona=furina\n> .autoai global on --persona=custom --logic=...",
                );
            }
        }
    };

    let text = global_on_text(&character_name, &flags.response_type);
    let stored = json!({
        "enabled": true,
        "character": character,
        "characterName": character_name,
        "instruction": instruction,
        "responseType": flags.response_type,
        "mode": flags.ai_mode,
    });
    Reply::text(text).with_mutation(set_setting(GLOBAL_KEY, stored))
}

fn help(ctx: &Ctx, personas: &Map<String, Value>) -> Reply {
    let char_list = CHARACTERS
        .iter()
        .map(|(key, name, _)| format!("> {key} - {name}"))
        .collect::<Vec<String>>()
        .join("\n");
    let custom_list = if personas.is_empty() {
        String::new()
    } else {
        personas
            .iter()
            .map(|(key, value)| format!("> {key} - {} (custom)", str_of(value.get("name"))))
            .collect::<Vec<String>>()
            .join("\n")
    };

    let mut txt = String::from(
        "🤖 *ᴀᴜᴛᴏ ᴀɪ*\n\n\
         > Mengaktifkan/menonaktifkan auto AI response\n\n\
         *Penggunaan:*\n\
         > .autoai on --persona=<karakter|custom> --type=<text|voice> --mode=<onlychat|assistant>\n\
         > .autoai off\n\
         > .autoai tambahpersona nama | logic\n\
         > .autoai hapuspersona nama\n\
         > .autoai listpersona\n\
         > .autoai global on/off\n\
         > .autoai enablecommand / disablecommand\n\n\
         *Karakter bawaan:*\n",
    );
    txt.push_str(&char_list);
    txt.push('\n');
    if !custom_list.is_empty() {
        txt.push_str(&format!("\n*Karakter custom:*\n{custom_list}\n"));
    }
    txt.push_str(&format!(
        "\n*Global:* {status}\n\n\
         *Response Type:*\n\
         > text - Reply dengan text biasa\n\
         > voice - Reply dengan voice note (TTS)\n\n\
         *Mode AutoAI:*\n\
         > assistant - Bot bisa jalankan aksi (buka tutup grup, kick, rich message)\n\
         > onlychat - Bot hanya murni chat santai biasa\n\n\
         *Contoh:*\n\
         > .autoai on --persona=furina --type=text\n\
         > .autoai on --persona=custom --logic=kamu adalah nexa ai\n\
         > .autoai tambahpersona nexa | kamu adalah nexa ai\n\
         > .autoai global on --persona=furina",
        status = global_label(ctx),
    ));
    Reply::text(txt)
}

fn turn_off(ctx: &Ctx) -> Reply {
    let mut table = autoai_table(ctx);
    table.insert(ctx.chat.clone(), json!({ "enabled": false }));
    let suffix = if crate::common::js_truthy(global_table(ctx).get("enabled")) {
        "\n\n> ℹ️ Global masih aktif, tapi grup ini opted-out\n> ℹ️ Ketik *.autoai global off* untuk matikan global"
    } else {
        ""
    };
    Reply::text(format!(
        "🤖 *ᴀᴜᴛᴏ ᴀɪ ᴅɪɴᴏɴᴀᴋᴛɪғᴋᴀɴ*\n\n\
         > Auto AI untuk grup ini telah dimatikan\n\
         > Semua command kembali aktif{suffix}"
    ))
    .with_mutation(set_setting(AUTOAI_KEY, Value::Object(table)))
}

fn activate(ctx: &Ctx, flags: &Flags, personas: &Map<String, Value>) -> Reply {
    let Some(char_key) = flags.char_key.clone() else {
        return Reply::text(format!(
            "❌ Karakter tidak valid!\n\n> Karakter tersedia: {keys}\n\n> Contoh: .autoai on --persona=furina --type=voice\n> Custom: .autoai on --persona=custom --logic=kamu adalah nexa ai",
            keys = key_list(personas)
        ));
    };

    if char_key == "custom" {
        let Some(logic) = flags.custom_logic.clone() else {
            return Reply::text(
                "❌ Mode custom membutuhkan --logic!\n\n> Contoh: .autoai on --persona=custom --logic=kamu adalah nexa ai, ...",
            );
        };
        let (shown, long) = clip(&logic, 100);
        let text = active_text(
            "┃ 🎭 Karakter: *Custom*\n".to_string(),
            Some(format!(
                "┃ 🧠 Logic: {shown}{dots}\n",
                dots = if long { "..." } else { "" }
            )),
            ctx.sender_number(),
            &flags.response_type,
        );
        let config = config_value("custom", "Custom", &logic, flags, &ctx.sender);
        return save_group_config(ctx, config, text);
    }

    if let Some(persona) = personas.get(&char_key) {
        let name = str_of(persona.get("name"));
        let instruction = str_of(persona.get("instruction"));
        let text = active_text(
            format!("┃ 🎭 Karakter: *{name}* (custom)\n"),
            None,
            ctx.sender_number(),
            &flags.response_type,
        );
        let config = config_value(&char_key, &name, &instruction, flags, &ctx.sender);
        return save_group_config(ctx, config, text);
    }

    let Some((key, name, instruction)) = CHARACTERS
        .iter()
        .find(|(k, ..)| *k == char_key.as_str())
        .copied()
    else {
        return Reply::text(format!(
            "❌ Karakter tidak valid!\n\n> Karakter tersedia: {keys}\n\n> Contoh: .autoai on --persona=furina --type=voice",
            keys = key_list(personas)
        ));
    };

    let text = active_text(
        format!("┃ 🎭 Karakter: *{name}*\n"),
        None,
        ctx.sender_number(),
        &flags.response_type,
    );
    let config = config_value(key, name, instruction, flags, &ctx.sender);
    save_group_config(ctx, config, text)
}

// ================================ handler ================================

#[plugin(
    name = "autoai",
    alias = ["aai"],
    category = "group",
    description = "Toggle auto AI response untuk grup dengan pilihan text atau voice",
    usage = ".autoai on/off --persona=<character|custom> --logic=<custom instruction> --type=<text|voice> --mode=<onlychat|assistant>",
    example = ".autoai on --persona=furina --type=voice --mode=onlychat",
    cooldown = 5,
    energi = 0,
    is_owner = false,
    is_premium = false,
    is_group = true,
    is_private = false
)]
pub fn autoai(ctx: Ctx) -> Reply {
    if !ctx.is_group {
        return Reply::text("❌ Fitur ini hanya untuk grup!");
    }
    // `m.isAdmin` = pengirim admin grup, bukan status admin bot.
    if !ctx.roster.is_admin(&ctx.sender) && !ctx.is_owner {
        return Reply::text("❌ Hanya admin yang bisa menggunakan fitur ini!");
    }

    let full = ctx.text.clone();
    let personas = persona_table(&ctx);
    let subcmd = ctx.args.first().map(|a| a.to_lowercase());

    match subcmd.as_deref() {
        Some("tambahpersona") => return tambah_persona(&ctx, &full),
        Some("hapuspersona") => return hapus_persona(&ctx),
        Some("enablecommand") | Some("enablecmd") => return set_enable_commands(&ctx, true),
        Some("disablecommand") | Some("disablecmd") => return set_enable_commands(&ctx, false),
        Some("listpersona") => return list_persona(&ctx, &personas),
        Some("global") => return global_toggle(&ctx, &full, &personas),
        _ => {}
    }

    let flags = Flags::parse(&full);
    match subcmd.as_deref() {
        Some("off") => turn_off(&ctx),
        Some("on") => activate(&ctx, &flags, &personas),
        // `!mode || !["on", "off"].includes(mode)`
        _ => help(&ctx, &personas),
    }
}

// ============================ waktu ISO ============================

/// `new Date().toISOString()`. `civil_from_days` di `common.rs` masih
/// private, jadi invers algoritma civil-calendar-nya ditulis ulang di sini.
fn iso_now() -> String {
    let ms = crate::common::now_ms();
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60,
        ms.rem_euclid(1000)
    )
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// `String.prototype.replace(new RegExp("^" + prefix, "i"), "")` untuk prefix
/// ASCII — dipakai `tambahpersona`.
fn strip_leading_ci(text: &str, prefix: &str) -> String {
    let hay: Vec<char> = text.chars().collect();
    let pre: Vec<char> = prefix.chars().collect();
    if hay.len() < pre.len() {
        return text.to_string();
    }
    if !hay[..pre.len()]
        .iter()
        .zip(&pre)
        .all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()))
    {
        return text.to_string();
    }
    hay[pre.len()..].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::registry::GroupRoster;
    use oktz_plugin_api::{Participant, find};
    use serde_json::Value;

    const CHAT: &str = "120363@g.us";
    const ME: &str = "628111@s.whatsapp.net";

    fn p(jid: &str, admin: bool) -> Participant {
        Participant {
            jid: jid.into(),
            is_admin: admin,
            is_owner: false,
            data: Value::Null,
        }
    }

    fn run_with(args: &[&str], settings: Value, is_group: bool, is_admin: bool, is_owner: bool) -> Reply {
        let roster = GroupRoster {
            participants: vec![p(ME, is_admin)],
            is_bot_admin: true,
        };
        (find("autoai").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: "autoai".into(),
            sender: ME.into(),
            chat: CHAT.into(),
            is_group,
            is_owner,
            args: args.iter().map(|s| (*s).to_string()).collect(),
            text: args.join(" "),
            roster,
            settings,
            ..Default::default()
        })
    }

    fn run(args: &[&str]) -> Reply {
        run_with(args, Value::Null, true, true, false)
    }

    fn run_owner(args: &[&str], settings: Value) -> Reply {
        run_with(args, settings, true, true, true)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn setting_written(r: &Reply, key: &str) -> Value {
        r.mutations()
            .iter()
            .find_map(|m| match m {
                Mutation::SetSetting { key: k, value } if k == key => Some(value.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("tidak ada SetSetting untuk {key}"))
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("autoai").unwrap();
        assert_eq!(p.meta.name, "autoai");
        assert_eq!(p.meta.alias, ["aai"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Toggle auto AI response untuk grup dengan pilihan text atau voice"
        );
        assert_eq!(
            p.meta.usage,
            ".autoai on/off --persona=<character|custom> --logic=<custom instruction> --type=<text|voice> --mode=<onlychat|assistant>"
        );
        assert_eq!(p.meta.example, ".autoai on --persona=furina --type=voice --mode=onlychat");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn the_alias_resolves_back_to_this_plugin() {
        assert_eq!(find("aai").map(|e| e.meta.name.as_str()), Some("autoai"));
    }

    #[test]
    fn private_chat_is_refused_before_anything_else() {
        let r = run_with(&["on", "--persona=furina"], Value::Null, false, true, true);
        assert_eq!(text_of(&r), "❌ Fitur ini hanya untuk grup!");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_plain_member_cannot_use_it() {
        let r = run_with(&["listpersona"], Value::Null, true, false, false);
        assert_eq!(text_of(&r), "❌ Hanya admin yang bisa menggunakan fitur ini!");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn no_argument_prints_the_usage_board() {
        let t = text_of(&run(&[]));
        assert!(t.starts_with("🤖 *ᴀᴜᴛᴏ ᴀɪ*\n\n> Mengaktifkan/menonaktifkan auto AI response"), "{t}");
        assert!(t.contains("*Karakter bawaan:*\n> furina - Furina\n> zeta - Zeta"), "{t}");
        assert!(t.contains("> waguri - Waguri\n"), "{t}");
        assert!(t.contains("> bell409 - Bell409\n"), "{t}");
        assert!(t.contains("\n*Global:* ❌ Nonaktif\n"), "{t}");
        assert!(t.ends_with("> .autoai global on --persona=furina"), "{t}");
        assert!(run(&[]).mutations().is_empty());
    }

    #[test]
    fn the_board_shows_custom_personas_only_when_there_are_some() {
        let s = json!({ "autoai_personas": { "nexa": { "name": "Nexa", "instruction": "kamu nexa" } } });
        let t = text_of(&run_with(&[], s.clone(), true, true, false));
        assert!(t.contains("*Karakter custom:*\n> nexa - Nexa (custom)"), "{t}");

        let t = text_of(&run_with(&[], Value::Null, true, true, false));
        assert!(!t.contains("Karakter custom"), "{t}");
    }

    #[test]
    fn an_unknown_subcommand_falls_through_to_the_board() {
        let t = text_of(&run(&["ngapain"]));
        assert!(t.starts_with("🤖 *ᴀᴜᴛᴏ ᴀɪ*"), "{t}");
    }

    #[test]
    fn turning_on_with_a_built_in_persona_stores_the_config() {
        let r = run(&["on", "--persona=furina", "--type=voice", "--mode=onlychat"]);
        let t = text_of(&r);
        assert!(t.starts_with("🤖 *ᴀᴜᴛᴏ ᴀɪ ᴅɪᴀᴋᴛɪғᴋᴀɴ*"), "{t}");
        assert!(t.contains("┃ 🎭 Karakter: *Furina*\n"), "{t}");
        assert!(t.contains("┃ 📢 Response: *🎤 Voice Note*\n"), "{t}");
        assert!(t.contains("┃ 👤 Diaktifkan: @628111\n"), "{t}");
        assert!(t.contains("> ℹ️ Response dalam bentuk voice note\n"), "{t}");
        assert!(t.ends_with("> ℹ️ Ketik *.autoai off* untuk menonaktifkan"), "{t}");

        let stored = setting_written(&r, AUTOAI_KEY);
        let cfg = &stored[CHAT];
        assert_eq!(cfg["enabled"], json!(true));
        assert_eq!(cfg["character"], json!("furina"));
        assert_eq!(cfg["characterName"], json!("Furina"));
        assert_eq!(cfg["instruction"], json!(FURINA_INSTRUCTION));
        assert_eq!(cfg["responseType"], json!("voice"));
        assert_eq!(cfg["mode"], json!("onlychat"));
        assert_eq!(cfg["enableCommands"], json!(false));
        assert_eq!(cfg["activatedBy"], json!(ME));
        match r.without_mutations() {
            Reply::Text { mentions, .. } => assert_eq!(mentions, &vec![ME.to_string()]),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn the_default_response_type_and_mode_match_the_js_fallbacks() {
        let r = run(&["on", "--persona=zeta"]);
        let stored = setting_written(&r, AUTOAI_KEY);
        assert_eq!(stored[CHAT]["responseType"], json!("text"));
        assert_eq!(stored[CHAT]["mode"], json!("assistant"));
        let t = text_of(&r);
        assert!(t.contains("┃ 📢 Response: *💬 Text*\n"), "{t}");
        assert!(!t.contains("Response dalam bentuk voice note"), "{t}");
    }

    #[test]
    fn turning_on_keeps_the_config_of_other_groups() {
        let s = json!({ "autoai": { "other@g.us": { "enabled": true, "character": "furina" } } });
        let r = run_with(&["on", "--persona=furina"], s, true, true, false);
        let stored = setting_written(&r, AUTOAI_KEY);
        assert_eq!(stored["other@g.us"]["character"], json!("furina"));
        assert_eq!(stored[CHAT]["enabled"], json!(true));
    }

    #[test]
    fn a_custom_persona_from_the_store_can_be_activated() {
        let s = json!({ "autoai_personas": { "nexa": { "name": "Nexa", "instruction": "kamu nexa" } } });
        let r = run_with(&["on", "--persona=nexa"], s, true, true, false);
        let t = text_of(&r);
        assert!(t.contains("┃ 🎭 Karakter: *Nexa* (custom)\n"), "{t}");
        let stored = setting_written(&r, AUTOAI_KEY);
        assert_eq!(stored[CHAT]["instruction"], json!("kamu nexa"));
        assert_eq!(stored[CHAT]["character"], json!("nexa"));
    }

    #[test]
    fn persona_custom_without_logic_is_refused() {
        let r = run(&["on", "--persona=custom"]);
        assert_eq!(
            text_of(&r),
            "❌ Mode custom membutuhkan --logic!\n\n> Contoh: .autoai on --persona=custom --logic=kamu adalah nexa ai, ..."
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn persona_custom_stores_the_logic_and_previews_it() {
        let r = run(&["on", "--persona=custom", "--logic=kamu adalah nexa ai"]);
        let t = text_of(&r);
        assert!(t.contains("┃ 🎭 Karakter: *Custom*\n"), "{t}");
        assert!(t.contains("┃ 🧠 Logic: kamu adalah nexa ai\n"), "{t}");
        let stored = setting_written(&r, AUTOAI_KEY);
        assert_eq!(stored[CHAT]["character"], json!("custom"));
        assert_eq!(stored[CHAT]["characterName"], json!("Custom"));
        assert_eq!(stored[CHAT]["instruction"], json!("kamu adalah nexa ai"));
    }

    #[test]
    fn the_key_list_includes_built_ins_custom_personas_and_custom() {
        let s = json!({ "autoai_personas": { "nexa": { "name": "Nexa", "instruction": "x" } } });
        let t = text_of(&run_with(&["on"], s, true, true, false));
        assert!(
            t.contains("> Karakter tersedia: furina, zeta, kobo, elaina, waguri, bell409, nexa, custom\n"),
            "{t}"
        );
    }

    #[test]
    fn an_unknown_persona_lists_the_available_keys() {
        let r = run(&["on", "--persona=animegirl"]);
        let t = text_of(&r);
        assert!(t.starts_with("❌ Karakter tidak valid!"), "{t}");
        assert!(t.contains("> Karakter tersedia: furina, zeta, kobo, elaina, waguri, bell409, custom\n"), "{t}");
        // `--persona` ada tapi tidak dikenal -> varian JS `autoai.js:433`, yang
        // TIDAK punya baris `> Custom:` (itu hanya untuk `!charKey`, `:359`).
        assert!(t.ends_with("> Contoh: .autoai on --persona=furina --type=voice"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn turning_off_writes_a_disabled_stub_and_mentions_a_live_global() {
        let r = run(&["off"]);
        let stored = setting_written(&r, AUTOAI_KEY);
        assert_eq!(stored[CHAT], json!({ "enabled": false }));
        let t = text_of(&r);
        assert!(t.starts_with("🤖 *ᴀᴜᴛᴏ ᴀɪ ᴅɪɴᴏɴᴀᴋᴛɪғᴋᴀɴ*"), "{t}");
        assert!(t.ends_with("> Semua command kembali aktif"), "{t}");

        let s = json!({ "autoai_global": { "enabled": true } });
        let t = text_of(&run_with(&["off"], s, true, true, false));
        assert!(t.ends_with("> Semua command kembali aktif\n\n> ℹ️ Global masih aktif, tapi grup ini opted-out\n> ℹ️ Ketik *.autoai global off* untuk matikan global"), "{t}");
    }

    #[test]
    fn persona_management_is_owner_only() {
        assert_eq!(
            text_of(&run(&["tambahpersona", "nexa", "kamu nexa"])),
            "❌ Hanya owner yang bisa menambah persona!"
        );
        assert_eq!(
            text_of(&run(&["hapuspersona", "nexa"])),
            "❌ Hanya owner yang bisa menghapus persona!"
        );
        assert_eq!(
            text_of(&run(&["global", "on"])),
            "❌ Hanya owner yang bisa toggle global!"
        );
    }

    #[test]
    fn tambahpersona_normalises_the_key_and_stores_the_creator() {
        // `ctx.text` = teks SETELAH kata command (serialize.js:407), jadi
        // "tambahpersona" bukan bagian dari args.
        let r = run_owner(
            &["tambahpersona", "Nexa", "AI", "|", "kamu", "adalah", "nexa", "|", "ai", "secepatnya"],
            Value::Null,
        );
        let personas = setting_written(&r, PERSONAS_KEY);
        assert_eq!(personas["nexa_ai"]["name"], json!("Nexa AI"));
        assert_eq!(personas["nexa_ai"]["instruction"], json!("kamu adalah nexa|ai secepatnya"));
        assert_eq!(personas["nexa_ai"]["createdBy"], json!(ME));
        let t = text_of(&r);
        assert!(t.starts_with("✅ *Persona ditambahkan*"), "{t}");
        assert!(t.contains("> Nama: Nexa AI\n> Key: nexa_ai\n"), "{t}");
        assert!(t.ends_with("> Gunakan: .autoai on --persona=nexa_ai"), "{t}");
    }

    #[test]
    fn tambahpersona_needs_a_name_and_an_instruction() {
        for args in [
            vec!["tambahpersona"],
            vec!["tambahpersona", "nexa"],
            vec!["tambahpersona", "nexa", "|", ""],
        ] {
            let r = run_owner(&args, Value::Null);
            assert_eq!(
                text_of(&r),
                "❌ Format salah!\n\n> .autoai tambahpersona nama | instruction\n\n> Contoh: .autoai tambahpersona nexa | kamu adalah nexa ai, ...",
                "{args:?}"
            );
            assert!(r.mutations().is_empty(), "{args:?}");
        }
    }

    #[test]
    fn tambahpersona_refuses_a_built_in_name() {
        let r = run_owner(&["tambahpersona", "Furina", "|", "lain"], Value::Null);
        assert_eq!(
            text_of(&r),
            "❌ Nama \"furina\" sudah dipakai persona bawaan!\n\n> Pilih nama lain"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn hapuspersona_needs_a_key_and_reports_an_unknown_one() {
        let r = run_owner(&["hapuspersona"], Value::Null);
        assert_eq!(
            text_of(&r),
            "❌ Format salah!\n\n> .autoai hapuspersona <nama>\n\n> Contoh: .autoai hapuspersona nexa"
        );

        let r = run_owner(&["hapuspersona", "tidakada"], Value::Null);
        assert_eq!(
            text_of(&r),
            "❌ Persona \"tidakada\" tidak ditemukan!\n\n> Ketik .autoai listpersona untuk melihat daftar"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn hapuspersona_removes_only_that_entry() {
        let s = json!({ "autoai_personas": {
            "nexa": { "name": "Nexa", "instruction": "a" },
            "zeta": { "name": "Zeta", "instruction": "b" },
        } });
        let r = run_owner(&["hapuspersona", "NEXA"], s);
        assert_eq!(text_of(&r), "✅ Persona \"nexa\" berhasil dihapus");
        let personas = setting_written(&r, PERSONAS_KEY);
        assert!(personas.get("nexa").is_none());
        assert_eq!(personas["zeta"]["name"], json!("Zeta"));
    }

    #[test]
    fn listpersona_shows_built_ins_custom_entries_and_the_global_flag() {
        let s = json!({
            "autoai_personas": { "nexa": { "name": "Nexa", "instruction": "0123456789012345678901234567890123456789menang" } },
            "autoai_global": { "enabled": true },
        });
        let t = text_of(&run_with(&["listpersona"], s, true, true, false));
        assert!(t.starts_with("🤖 *ᴅᴀғᴛᴀʀ ᴘᴇʀsᴏɴᴀ*\n\n*Bawaan:*\n  ▸ furina - Furina\n"), "{t}");
        assert!(t.contains("  ▸ bell409 - Bell409\n\n*Custom:*\n"), "{t}");
        assert!(
            t.contains("  ▸ nexa - Nexa (0123456789012345678901234567890123456789...)\n"),
            "{t}"
        );
        assert!(t.contains("*Global:* ✅ Aktif\n"), "{t}");
        assert!(t.ends_with("> .autoai global on/off"), "{t}");
        assert!(run_with(&["listpersona"], Value::Null, true, true, false).mutations().is_empty());
    }

    #[test]
    fn listpersona_says_so_when_there_is_no_custom_persona() {
        let t = text_of(&run(&["listpersona"]));
        assert!(t.contains("*Custom:*\n  ▸ (belum ada custom persona)\n"), "{t}");
    }

    #[test]
    fn global_needs_on_or_off() {
        for args in [vec!["global"], vec!["global", "mungkin"]] {
            let r = run_owner(&args, Value::Null);
            assert_eq!(
                text_of(&r),
                "❌ Format salah!\n\n> .autoai global on/off\n\n> Global saat ini: ❌ Nonaktif",
                "{args:?}"
            );
            assert!(r.mutations().is_empty(), "{args:?}");
        }
    }

    #[test]
    fn global_on_stores_the_resolved_persona() {
        let r = run_owner(&["global", "on", "--persona=furina", "--type=voice"], Value::Null);
        let t = text_of(&r);
        assert!(t.starts_with("🌐 *ᴀᴜᴛᴏ ᴀɪ ɢʟᴏʙᴀʟ ᴅɪᴀᴋᴛɪғᴋᴀɴ*"), "{t}");
        assert!(t.contains("┃ 🎭 Karakter: *Furina*\n"), "{t}");
        assert!(t.contains("┃ 📢 Response: *🎤 Voice Note*\n"), "{t}");
        assert!(t.ends_with("> ℹ️ Ketik *.autoai global off* untuk menonaktifkan"), "{t}");
        let g = setting_written(&r, GLOBAL_KEY);
        assert_eq!(g["enabled"], json!(true));
        assert_eq!(g["character"], json!("furina"));
        assert_eq!(g["characterName"], json!("Furina"));
        assert_eq!(g["instruction"], json!(FURINA_INSTRUCTION));
        assert_eq!(g["responseType"], json!("voice"));
        assert_eq!(g["mode"], json!("assistant"));
    }

    #[test]
    fn global_on_without_a_persona_reuses_the_existing_one() {
        let s = json!({ "autoai_global": {
            "enabled": false,
            "character": "nexa",
            "characterName": "Nexa",
            "instruction": "kamu nexa",
        } });
        let r = run_owner(&["global", "on"], s);
        assert!(text_of(&r).contains("┃ 🎭 Karakter: *Nexa*\n"));
        let g = setting_written(&r, GLOBAL_KEY);
        assert_eq!(g["character"], json!("nexa"));
        assert_eq!(g["enabled"], json!(true));
    }

    #[test]
    fn global_on_without_a_persona_and_without_history_is_refused() {
        let r = run_owner(&["global", "on"], Value::Null);
        assert_eq!(
            text_of(&r),
            "❌ Belum ada persona global yang diset!\n\n> .autoai global on --persona=furina\n> .autoai global on --persona=custom --logic=..."
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn global_on_rejects_an_unknown_persona_with_the_key_list() {
        let r = run_owner(&["global", "on", "--persona=zzz"], Value::Null);
        assert_eq!(
            text_of(&r),
            "❌ Karakter tidak valid!\n\n> Tersedia: furina, zeta, kobo, elaina, waguri, bell409, custom"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn global_off_only_flips_the_flag() {
        let s = json!({ "autoai_global": { "enabled": true, "character": "furina", "instruction": "x" } });
        let r = run_owner(&["global", "off"], s);
        assert_eq!(
            text_of(&r),
            "🌐 *ᴀᴜᴛᴏ ᴀɪ ɢʟᴏʙᴀʟ ᴅɪɴᴏɴᴀᴋᴛɪғᴋᴀɴ*\n\n> AutoAI hanya aktif di grup yang sudah di-set"
        );
        let g = setting_written(&r, GLOBAL_KEY);
        assert_eq!(g["enabled"], json!(false));
        assert_eq!(g["character"], json!("furina"), "isi lain tidak boleh hilang");
    }

    #[test]
    fn enable_command_requires_an_active_group_config() {
        for sub in ["enablecommand", "enablecmd", "disablecommand", "disablecmd"] {
            let r = run(&[sub]);
            assert_eq!(text_of(&r), "❌ AutoAI belum aktif di grup ini!", "{sub}");
            assert!(r.mutations().is_empty(), "{sub}");
        }
    }

    #[test]
    fn enable_and_disable_command_flip_the_flag() {
        let active = json!({ "autoai": { CHAT: { "enabled": true, "character": "furina" } } });

        let r = run_with(&["enablecommand"], active.clone(), true, true, false);
        let t = text_of(&r);
        assert!(t.starts_with("✅ *ᴇɴᴀʙʟᴇ ᴄᴏᴍᴍᴀɴᴅ*"), "{t}");
        assert!(t.ends_with("_Gunakan .autoai disablecommand untuk menonaktifkan_"), "{t}");
        let stored = setting_written(&r, AUTOAI_KEY);
        assert_eq!(stored[CHAT]["enableCommands"], json!(true));
        assert_eq!(stored[CHAT]["character"], json!("furina"), "field lain tetap utuh");

        let s = json!({ "autoai": { CHAT: { "enabled": true, "enableCommands": true } } });
        let r = run_with(&["disablecommand"], s, true, true, false);
        let t = text_of(&r);
        assert!(t.starts_with("🔒 *ᴅɪsᴀʙʟᴇ ᴄᴏᴍᴍᴀɴᴅ*"), "{t}");
        assert!(t.ends_with("_Gunakan .autoai enablecommand untuk mengaktifkan kembali_"), "{t}");
        assert_eq!(setting_written(&r, AUTOAI_KEY)[CHAT]["enableCommands"], json!(false));
    }

    #[test]
    fn toggling_the_same_value_only_reports() {
        let enabled = json!({ "autoai": { CHAT: { "enabled": true, "enableCommands": true } } });
        let r = run_with(&["enablecommand"], enabled.clone(), true, true, false);
        assert_eq!(
            text_of(&r),
            "ℹ️ *Command sudah di-enable*\n\n> User tetap bisa pakai command walau AutoAI aktif"
        );
        assert!(r.mutations().is_empty());

        let disabled = json!({ "autoai": { CHAT: { "enabled": true } } });
        let r = run_with(&["disablecommand"], disabled, true, true, false);
        assert_eq!(
            text_of(&r),
            "ℹ️ *Command sudah di-disable*\n\n> Semua command (kecuali owner) diblokir saat AutoAI aktif"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn flags_are_matched_case_insensitively() {
        let r = run(&["on", "--PERSONA=FURINA", "--TYPE=VOICE"]);
        assert_eq!(setting_written(&r, AUTOAI_KEY)[CHAT]["character"], json!("furina"));
        assert_eq!(setting_written(&r, AUTOAI_KEY)[CHAT]["responseType"], json!("voice"));
    }

    #[test]
    fn logic_stops_at_the_next_flag() {
        let f = Flags::parse("on --persona=custom --logic=jadi nexa ya --type=voice");
        assert_eq!(f.custom_logic.as_deref(), Some("jadi nexa ya"));
        assert_eq!(f.response_type, "voice");
    }

    #[test]
    fn logic_runs_to_the_end_when_no_flag_follows() {
        let f = Flags::parse("on --logic=jadi nexa");
        assert_eq!(f.custom_logic.as_deref(), Some("jadi nexa"));
        let f = Flags::parse("--logic=");
        assert_eq!(f.custom_logic, None, "`.+?` butuh minimal satu karakter");
        assert_eq!(Flags::parse("on --persona=furina").custom_logic, None);
    }

    #[test]
    fn a_word_flag_needs_at_least_one_word_character() {
        assert_eq!(Flags::parse("on --persona=").char_key, None);
        assert_eq!(Flags::parse("on --persona=fu rina").char_key.as_deref(), Some("fu"));
        assert_eq!(Flags::parse("on --persona=furina-2").char_key.as_deref(), Some("furina"));
    }

    #[test]
    fn a_choice_flag_falls_back_when_the_value_is_unknown() {
        let f = Flags::parse("on --type=mp3 --mode=deepseek");
        assert_eq!(f.response_type, "text");
        assert_eq!(f.ai_mode, "assistant");
        let f = Flags::parse("on --mode=onlychat");
        assert_eq!(f.ai_mode, "onlychat");
    }

    #[test]
    fn the_iso_timestamp_is_written_in_the_utc_shape_js_produces() {
        let stamp = iso_now();
        assert_eq!(stamp.len(), 24, "{stamp}");
        assert!(stamp.ends_with('Z'), "{stamp}");
        assert_eq!(&stamp[4..5], "-");
        assert_eq!(&stamp[10..11], "T");
    }

    #[test]
    fn civil_from_days_round_trips_the_epoch() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
    }

    #[test]
    fn collapse_ws_replaces_each_run_with_one_underscore() {
        // Tanpa lowercase: `.toLowerCase()`-nya terpisah di `tambah_persona`.
        assert_eq!(collapse_ws("Nexa  AI"), "Nexa_AI");
        assert_eq!(collapse_ws("a\tb\nc"), "a_b_c");
        assert_eq!(collapse_ws("nospace"), "nospace");
    }

    #[test]
    fn the_bell409_instruction_is_the_config_literal() {
        assert!(BELL409_INSTRUCTION.starts_with("- Nama kamu adalah Bella Clarissa"));
        assert!(BELL409_INSTRUCTION.ends_with("- Jangan mengaku sebagai AI atau bot."));
        assert_eq!(BELL409_INSTRUCTION.lines().count(), 9);
        assert!(!BELL409_INSTRUCTION.contains('\r'));
    }
}