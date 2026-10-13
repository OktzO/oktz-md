//! `plugins/clan/clancreate.js` — buat clan baru, potong koin, simpan ke store global.
//!
//! `ponytail:` JS menyimpan seluruh clan di `db.db.data.clans`, satu map global
//! yang bukan milik user maupun grup. `Ctx` hanya membuka tiga wearer state
//! (user, group, settings), jadi map itu dipindah ke key global `clans` di
//! `settings` — satu-satunya map global yang bisa dibaca plugin. Bentuk
//! isinya sama dengan JS; hanya lokasinya yang berbeda.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value, json};

use crate::common::jid_number;
use crate::tools::kalkulatormbg::thousands_id;

/// Key global tempat peta `clans` disimpan — lihat catatan `ponytail:` di atas.
pub(crate) const CLANS_KEY: &str = "clans";

/// Batas member per clan. `clancreate.js` tidak punya konstanta ini (ia
/// menulis `50` inline), empat plugin lain menyebut angka yang sama.
pub(crate) const MAX_MEMBERS: usize = 50;

pub(crate) const CLAN_CREATE_COST: f64 = 50_000.0;

pub(crate) const MAX_CLAN_NAME: usize = 20;

/// `updateKoin` JS: `Math.min(MAX_KOIN, koin + amount)` dengan
/// `MAX_KOIN = 9000000000000` (`database.js:650`). Bulat di semua sisi, jadi
/// clamp tidak pernah menghasilkan pecahan.
pub(crate) const MAX_KOIN: i64 = 9_000_000_000_000;

pub(crate) const CLAN_EMBLEMS: [&str; 10] = [
    "🐉", "🦅", "🐺", "🦁", "🔥", "⚡", "🌙", "☀️", "💎", "🗡️",
];

/// Peta clan global, atau map kosong kalau key belum pernah ditulis — sama
/// dengan `if (!db.db.data.clans) db.db.data.clans = {}` di tiap plugin.
pub(crate) fn clans(ctx: &Ctx) -> Map<String, Value> {
    ctx.setting(CLANS_KEY)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn save_clans(clans: Map<String, Value>) -> Mutation {
    Mutation::SetSetting {
        key: CLANS_KEY.to_string(),
        value: Value::Object(clans),
    }
}

/// `clans[id] || values.find(c => c.name.toLowerCase() === id.toLowerCase())
/// || values.find(c => c.id.toLowerCase() === id.toLowerCase())`.
///
/// Tiga langkah itu dipakai identik oleh `claninfo`, `clanjoin` dan `clanwar`,
/// jadi urutannya tidak boleh diubah per plugin.
pub(crate) fn find_clan<'a>(clans: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    find_clan_entry(clans, key).map(|(_, c)| c)
}

/// Versi [`find_clan`] yang juga mengembalikan kunci peta. `clanjoin` dan
/// `clanleave` butuh kunci itu: JS menulis `db.setUser(sender, { clanId })`
/// dengan nilai yang sama persis dengan indeks `db.db.data.clans` yang dipakai
/// untuk lookup, bukan dengan `clan.id`.
pub(crate) fn find_clan_entry<'a>(
    clans: &'a Map<String, Value>,
    key: &str,
) -> Option<(&'a String, &'a Value)> {
    if let Some((k, v)) = clans.get_key_value(key) {
        return Some((k, v));
    }
    let want = key.to_lowercase();
    clans
        .iter()
        .find(|(_, c)| field_str(c, "name").eq_ignore_ascii_case(&want))
        .or_else(|| {
            clans
                .iter()
                .find(|(_, c)| field_str(c, "id").eq_ignore_ascii_case(&want))
        })
}

/// `clan.x || default` untuk string — `""` dan `null` sama-sama falsy di JS.
pub(crate) fn field_str(clan: &Value, key: &str) -> String {
    match clan.get(key).and_then(Value::as_str) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => String::new(),
    }
}

pub(crate) fn field_str_or(clan: &Value, key: &str, default: &str) -> String {
    let v = field_str(clan, key);
    if v.is_empty() {
        default.to_string()
    } else {
        v
    }
}

/// `Number(clan.x || 0)` — nilai non-angka jadi 0, sama seperti `|| 0`.
pub(crate) fn field_num(clan: &Value, key: &str) -> f64 {
    let raw = clan.get(key);
    if !crate::common::js_truthy(raw) {
        return 0.0;
    }
    raw.and_then(Value::as_f64).unwrap_or(0.0)
}

pub(crate) fn field_i64(clan: &Value, key: &str) -> i64 {
    field_num(clan, key) as i64
}

/// `clan.members` sebagai daftar JID. Field yang bukan array diperlakukan
/// sebagai array kosong, bukan `undefined` — pemanggil selalu meng-`len()`-nya.
pub(crate) fn members(clan: &Value) -> Vec<String> {
    clan.get("members")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn members_value(list: &[String]) -> Value {
    Value::Array(list.iter().map(|s| Value::String(s.clone())).collect())
}

/// `clan.emblem || '🏰'`.
pub(crate) fn emblem(clan: &Value) -> String {
    field_str_or(clan, "emblem", "🏰")
}

/// `Math.floor(Math.random() * CLAN_EMBLEMS.length)`.
pub(crate) fn random_emblem() -> &'static str {
    CLAN_EMBLEMS[crate::common::random_index(CLAN_EMBLEMS.len())]
}

/// `name.replace(/[^a-zA-Z]/g, '').toUpperCase()` — angka dan spasi ikut hilang,
/// jadi `Dragon  Slayer 1` menjadi `DRAGONSLAYER`.
pub(crate) fn clean_name(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphabetic)
        .collect::<String>()
        .to_uppercase()
}

fn take(clean: &str, n: usize) -> String {
    clean.chars().take(n).collect()
}

/// `Math.random().toString(36).slice(2, 5).toUpperCase()` — tiga digit base-36.
/// JS memakai tiga digit pertama pecahan, yang tidak pernah `0` di depan;
/// undian penuh 0..36³ di sini menyertakan Awalan `0` untuk 1 dari 36³ hasil.
/// ID ini hanya dipakai sebagai cadangan saat 104 kandidat lain sudah terpakai.
fn random_suffix() -> String {
    let mut n = crate::common::roll(36 * 36 * 36);
    let mut out = String::with_capacity(3);
    for _ in 0..3 {
        out.push(char::from_digit(n % 36, 36).unwrap_or('0'));
        n /= 36;
    }
    out.to_uppercase()
}

/// `generateShortId(name, existingClans)`.
///
/// Berulang: 3 huruf, lalu 4 huruf, lalu 3 huruf + angka 1..99, lalu 3 huruf
/// acak. Jumlah kandidat sudah pasti (3 + 1 + 99 + 1), jadi fungsi ini tidak
/// pernah gagal dan tidak butuh `Option`.
pub(crate) fn short_id(name: &str, taken: impl Fn(&str) -> bool) -> String {
    let clean = clean_name(name);

    let three = take(&clean, 3);
    let padded = if clean.chars().count() >= 3 {
        three.clone()
    } else {
        format!("{}{}", three, "X".repeat(3 - three.chars().count()))
    };
    if !taken(&padded) {
        return padded;
    }

    let four = take(&clean, 4);
    if !four.is_empty() && !taken(&four) {
        return four;
    }

    for i in 1..=99 {
        let attempt = format!("{three}{i}");
        if !taken(&attempt) {
            return attempt;
        }
    }

    format!("{}{}", take(&clean, 2), random_suffix())
}

/// `updateKoin(jid, amount)` (`database.js:646`).
///
/// Dua guard yang tidak boleh dilewati: `koin === -1` adalah sentinel
/// "unlimited" yang Freeze, dan hasil dijepit ke `0..=MAX_KOIN`.
pub(crate) fn add_koin(current: i64, amount: i64) -> i64 {
    if current == -1 {
        return -1;
    }
    current.saturating_add(amount).clamp(0, MAX_KOIN)
}

pub(crate) fn help() -> String {
    format!(
        "⚔️ *CREATE CLAN*\n\n\
         Buat clan dan kumpulkan member!\n\n\
         Biaya: *Rp {}*\n\
         Max nama: *{MAX_CLAN_NAME} karakter*\n\n\
         Contoh: *.clancreate DragonSlayer*",
        thousands_id(CLAN_CREATE_COST)
    )
}

pub(crate) fn not_enough(koin: f64) -> String {
    format!(
        "❌ Koin tidak cukup\n\n\
         Butuh: *Rp {}*\n\
         Punya: *Rp {}*",
        thousands_id(CLAN_CREATE_COST),
        thousands_id(koin)
    )
}

pub(crate) fn created(emblem: &str, name: &str, sender_num: &str, clan_id: &str) -> String {
    format!(
        "{emblem} *CLAN CREATED*\n\n\
         *{name}*\n\
         Leader: @{sender_num}\n\
         Status: Open · 1/{MAX_MEMBERS} members\n\n\
         _-Rp {}_\n\n\
         Invite teman: *.claninvite @user*\n\
         Atau share ID: *{clan_id}*",
        thousands_id(CLAN_CREATE_COST)
    )
}

#[plugin(
    name = "clancreate",
    alias = ["createclan", "guildcreate"],
    category = "clan",
    description = "Buat clan baru",
    usage = ".clancreate <nama>",
    example = ".clancreate DragonSlayer",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn clancreate(ctx: Ctx) -> Reply {
    let clan_name = ctx.text.trim().to_string();

    if clan_name.is_empty() {
        return Reply::text(help());
    }

    // `clanName.length` di JS menghitung UTF-16 code unit. `ponytail:` untuk
    // nama clan Latin keduanya sama; nama astral (emoji) dihitung 2 di JS dan
    // 1 di Rust, jadi batas 20 EMU di sini longgar 20 char.
    if clan_name.chars().count() > MAX_CLAN_NAME {
        return Reply::text(format!("❌ Nama clan maksimal {MAX_CLAN_NAME} karakter"));
    }

    // `/^[a-zA-Z0-9\s]+$/` — `\s` JS adalah whitespace Unicode, hampir sama
    // dengan `char::is_whitespace`; selisihnya cuma `\u{FEFF}` yang `\s` JS
    // anggap spasi dan Rust tidak, dan itu tidak muncul di nama clan.
    if !clan_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c.is_whitespace())
    {
        return Reply::text("❌ Nama clan hanya boleh huruf, angka, dan spasi");
    }

    if crate::common::js_truthy(ctx.user_field("clanId")) {
        return Reply::text("❌ Kamu sudah punya clan\nKeluar dulu: *.clanleave*");
    }

    let mut clans = clans(&ctx);
    let already = clans
        .values()
        .any(|c| field_str(c, "name").eq_ignore_ascii_case(&clan_name));
    if already {
        return Reply::text(format!("❌ Nama *{clan_name}* sudah dipakai"));
    }

    let koin = ctx
        .user_field("koin")
        .and_then(Value::as_i64)
        .or_else(|| ctx.user_field("koin").and_then(Value::as_f64).map(|f| f as i64))
        .unwrap_or(0);
    if (koin as f64) < CLAN_CREATE_COST {
        return Reply::text(not_enough(koin as f64));
    }

    let emblem = random_emblem();
    let clan_id = short_id(&clan_name, |id| clans.contains_key(id));

    let clan = json!({
        "id": clan_id,
        "name": clan_name,
        "emblem": emblem,
        "leader": ctx.sender,
        "members": [ctx.sender],
        "exp": 0,
        "level": 1,
        "wins": 0,
        "losses": 0,
        "createdAt": iso_now(),
        "description": "Belum ada deskripsi",
        "isOpen": true,
    });
    clans.insert(clan_id.clone(), clan);

    let text = created(emblem, &clan_name, jid_number(&ctx.sender), &clan_id);
    Reply::Text {
        text,
        mentions: vec![ctx.sender.clone()],
    }
    .with_mutation(save_clans(clans))
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({
            "clanId": clan_id,
            "koin": add_koin(koin, -(CLAN_CREATE_COST as i64)),
        }),
    })
}

/// `new Date().toISOString()` — format yang sama dengan yang ditulis JS, jadi
/// `claninfo` bisa membacanya kembali. Timestamp di-shift -7h supaya
/// `jakarta_parts` (yang selalu +7) mengembalikan dinding jam UTC.
pub(crate) fn iso_now() -> String {
    let (y, mo, d, h, mi, s) = crate::common::jakarta_parts(crate::common::now_ms());
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.000Z")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(user: Value, text: &str, clans: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "clancreate".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            user_data: user,
            settings: json!({ CLANS_KEY: clans }),
            ..Default::default()
        };
        (oktz_plugin_api::find("clancreate").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn set_user(r: &Reply) -> &Value {
        let m = r
            .mutations()
            .iter()
            .find(|m| matches!(m, Mutation::SetUser { .. }))
            .expect("harus ada SetUser");
        match m {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data
            }
            _ => unreachable!(),
        }
    }

    fn set_clans(r: &Reply) -> &Map<String, Value> {
        let m = r
            .mutations()
            .iter()
            .find(|m| matches!(m, Mutation::SetSetting { .. }))
            .expect("harus ada SetSetting");
        match m {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, CLANS_KEY);
                value.as_object().expect("harus object")
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("clancreate").unwrap();
        assert_eq!(p.meta.name, "clancreate");
        assert_eq!(p.meta.alias, ["createclan", "guildcreate"]);
        assert_eq!(p.meta.category, "clan");
        assert_eq!(p.meta.description, "Buat clan baru");
        assert_eq!(p.meta.usage, ".clancreate <nama>");
        assert_eq!(p.meta.example, ".clancreate DragonSlayer");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["createclan", "guildcreate"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "clancreate");
        }
    }

    #[test]
    fn an_empty_name_shows_help_and_writes_nothing() {
        for msg in ["", "   "] {
            let r = run(json!({"koin": 100_000}), msg, json!({}));
            assert!(text_of(&r).starts_with("⚔️ *CREATE CLAN*"), "input {msg:?}");
            assert!(text_of(&r).contains("Biaya: *Rp 50.000*"), "{}", text_of(&r));
            assert!(r.mutations().is_empty(), "input {msg:?}");
        }
    }

    #[test]
    fn a_successful_create_writes_the_clan_and_charges_koin() {
        let r = run(json!({"koin": 120_000}), "DragonSlayer", json!({}));
        let clans = set_clans(&r);
        assert_eq!(clans.len(), 1);
        let clan = &clans["DRA"];
        assert_eq!(clan["name"], json!("DragonSlayer"));
        assert_eq!(clan["leader"], json!("628111@s.whatsapp.net"));
        assert_eq!(clan["members"], json!(["628111@s.whatsapp.net"]));
        assert_eq!(clan["level"], json!(1));
        assert_eq!(clan["wins"], json!(0));
        assert_eq!(clan["isOpen"], json!(true));
        assert_eq!(set_user(&r)["clanId"], json!("DRA"));
        assert_eq!(set_user(&r)["koin"], json!(70_000));
        assert!(text_of(&r).contains("Status: Open · 1/50 members"));
        assert!(text_of(&r).contains("Leader: @628111"));
    }

    #[test]
    fn the_short_id_widens_then_numbers_when_the_prefix_is_taken() {
        // Suffix angka mulai dari 3 huruf + angka, bukan dari 4 huruf.
        let taken: Vec<String> = vec!["DRA".into(), "DRAG".into()];
        let id = short_id("DragonSlayer", |c| taken.iter().any(|t| t == c));
        assert_eq!(id, "DRA1");
        assert!(!id.starts_with("DRAG"), "4 huruf sudah dipakai");

        // Semua kandidat dipakai → jatuh ke `clean.slice(0,2)` + 3 huruf acak.
        let mut taken: Vec<String> = vec!["DRA".into(), "DRAG".into()];
        taken.extend((1..=99).map(|i| format!("DRA{i}")));
        let id = short_id("DragonSlayer", |c| taken.iter().any(|t| t == c));
        assert!(id.starts_with("DR"), "jatuh ke `clean.slice(0,2)`: {id}");
        assert_eq!(id.chars().count(), 5);
    }

    #[test]
    fn a_short_name_is_padded_with_x_and_a_non_alpha_name_becomes_all_x() {
        assert_eq!(short_id("ab", |_| false), "ABX");
        assert_eq!(short_id("123", |_| false), "XXX");
        assert_eq!(short_id("", |_| false), "XXX");
    }

    #[test]
    fn invalid_names_are_refused_before_anything_is_written() {
        let r = run(json!({"koin": 100_000}), "clan@evil", json!({}));
        assert_eq!(
            text_of(&r),
            "❌ Nama clan hanya boleh huruf, angka, dan spasi"
        );
        assert!(r.mutations().is_empty());

        let long = "a".repeat(MAX_CLAN_NAME + 1);
        let r = run(json!({"koin": 100_000}), &long, json!({}));
        assert_eq!(
            text_of(&r),
            format!("❌ Nama clan maksimal {MAX_CLAN_NAME} karakter")
        );
        assert!(r.mutations().is_empty());

        // Tepat 20 huruf masih diterima.
        let ok = "a".repeat(MAX_CLAN_NAME);
        let r = run(json!({"koin": 100_000}), &ok, json!({}));
        assert_eq!(set_clans(&r).len(), 1);
    }

    #[test]
    fn an_existing_clan_name_blocks_creation_case_insensitively() {
        let existing = json!({ "ABC": { "id": "ABC", "name": "Dragon" } });
        let r = run(json!({"koin": 100_000}), "dragon", existing);
        assert_eq!(text_of(&r), "❌ Nama *dragon* sudah dipakai");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_member_of_another_clan_is_refused_before_the_coin_check() {
        // `user.clanId` dicek sebelum koin, jadi guild member tanpa koin tetap
        // dapat pesan "sudah punya clan".
        let r = run(json!({"clanId": "XYZ"}), "Baru", json!({}));
        assert_eq!(text_of(&r), "❌ Kamu sudah punya clan\nKeluar dulu: *.clanleave*");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn not_enough_koin_reports_both_sides_and_never_deducts() {
        let r = run(json!({"koin": 49_999}), "Baru", json!({}));
        assert_eq!(text_of(&r), not_enough(49_999.0));
        assert!(text_of(&r).contains("Punya: *Rp 49.999*"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());

        // Tepat cukup → boleh.
        let r = run(json!({"koin": 50_000}), "Baru", json!({}));
        assert_eq!(set_user(&r)["koin"], json!(0), "50.000 - 50.000");
        assert_eq!(set_user(&r)["koin"].as_i64(), Some(0));
    }

    #[test]
    fn a_negative_or_unlimited_koin_never_drops_below_zero() {
        assert_eq!(add_koin(10_000, -50_000), 0);
        assert_eq!(add_koin(-1, -50_000), -1, "sentinel -1 di-Freeze");
        assert_eq!(add_koin(MAX_KOIN, 50_000), MAX_KOIN);
        assert_eq!(add_koin(100, 50_000), 50_100);
    }

    #[test]
    fn a_user_row_without_koin_is_treated_as_zero() {
        for user in [json!(null), json!({}), json!({"koin": null})] {
            let r = run(user.clone(), "Baru", json!({}));
            assert_eq!(text_of(&r), not_enough(0.0), "input {user}");
            assert!(r.mutations().is_empty(), "input {user}");
        }
    }

    #[test]
    fn missing_clan_map_reads_as_an_empty_registry() {
        let ctx = Ctx::default();
        assert!(clans(&ctx).is_empty());
        assert_eq!(clans(&Ctx { settings: json!("bukan object"), ..Default::default() }).len(), 0);
    }

    #[test]
    fn field_readers_mirror_js_falsy_fallbacks() {
        let c = json!({"name": "", "wins": 0, "level": null, "exp": 7, "isOpen": true});
        assert_eq!(field_str(&c, "name"), "");
        assert_eq!(field_str_or(&c, "name", "🏰"), "🏰");
        assert_eq!(field_num(&c, "wins"), 0.0);
        assert_eq!(field_num(&c, "level"), 0.0);
        assert_eq!(field_i64(&c, "exp"), 7);
        assert_eq!(field_num(&c, "tidak-ada"), 0.0);
        assert!(crate::common::js_truthy(c.get("isOpen")));
    }

    #[test]
    fn find_clan_falls_back_from_id_to_name_to_lowercase_id() {
        let map: Map<String, Value> = json!({
            "ABC": { "id": "AbC", "name": "Naga" },
        })
        .as_object()
        .unwrap()
        .clone();
        assert!(find_clan(&map, "ABC").is_some(), "kunci langsung");
        assert!(find_clan(&map, "naga").is_some(), "cocok nama");
        assert!(find_clan(&map, "abc").is_some(), "cocok id lowercase");
        assert!(find_clan(&map, "Zzz").is_none());
    }
}
