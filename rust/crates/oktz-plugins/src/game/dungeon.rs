//! `plugins/game/dungeon.js` — lobi dungeon interaktif.
//!
//! `ponytail:` `dungeon.js` mengekspor `handler` DAN `dungeonAnswerHandler`.
//! Yang kedua dipanggil router global saat user me-reply (bukan sebuah
//! command), dan `oktz-plugin-api` hanya punya satu titik masuk: `fn(Ctx)`.
//! Yang dipindah di sini `handler` — lobi plus pembuatan sesi. Tahap
//! `encounter` (pilih lokasi, serang, lari) butuh `dungeonAnswerHandler`
//! dan harus dipasang router sebelum alur ini bisa dilanjutkan.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

/// `SESSION_TIMEOUT` (`dungeon.js:147`) — 5 menit.
pub(crate) const SESSION_TIMEOUT_MS: i64 = 5 * 60 * 1000;

/// `Date.now() - session.time > SESSION_TIMEOUT` — strict, jadi sesi yang
/// umurnya persis 5 menit masih hidup. Dipisah dari handler karena batasnya
/// hanya bisa dikunci persis kalau `now` dikontrol test; lewat handler,
/// `now_ms()` di dalam selalu beberapa ms lebih besar dari yang dibaca test.
pub(crate) fn session_still_alive(now: i64, at: i64) -> bool {
    now - at <= SESSION_TIMEOUT_MS
}

/// `DUNGEONS` — id, nama, level minimal. Monster/reward tidak dipakai lobi.
pub(crate) const DUNGEONS: [(i64, &str, i64); 10] = [
    (1, "🌲 Hutan Gelap", 1),
    (2, "🍄 Rawa Beracun", 5),
    (3, "🏰 Kastil Tua", 10),
    (4, "🏜️ Padang Pasir Kematian", 15),
    (5, "🌋 Gunung Api", 20),
    (6, "🧊 Gua Es Abadi", 25),
    (7, "☁️ Reruntuhan Langit", 30),
    (8, "🌊 Lautan Bayangan", 35),
    (9, "🕳️ Jurang Ketiadaan", 40),
    (10, "👹 Neraka Terdalam", 50),
];

pub(crate) const SESSION_FIELD: &str = "rpg.dungeon_session";

/// `if (!user.rpg.X) user.rpg.X = default` — `0` dan `null` ikut memakai default.
fn default_field(rpg: Option<&Value>, key: &str, default: i64) -> i64 {
    let v = rpg.and_then(|r| r.get(key));
    if crate::common::js_truthy(v) {
        v.and_then(Value::as_i64).unwrap_or(default)
    } else {
        default
    }
}

/// Balasan blokir selama ada sesi yang belum kedaluwarsa.
pub(crate) const ACTIVE: &str = "⚔️ *SESI DUNGEON MASIH AKTIF*\n\n\
     Kamu sedang berada di pertengahan eksplorasi!\n\
     > Balas chat terakhir bot untuk membatalkan (ketik `batal`) atau melanjutkan aksi (ketik `serang` / `lari`).";

/// Cabang `available.length === 0`. Tidak terjangkau lewat handler: `levelReq`
/// paling rendah adalah 1 dan `user.level || 1` minimal 1. Tetap dibawa agar
/// ambang level tidak bisa diam-diam berubah tanpa guard.
pub(crate) fn too_low(user_level: i64) -> String {
    format!(
        "❌ *LEVEL TERLALU RENDAH*\n\n> Level kamu saat ini adalah *{user_level}*. \
         Kamu butuh minimal level *1* untuk memasuki dungeon paling mudah."
    )
}

pub(crate) fn lobby(user_level: i64, stamina: i64) -> String {
    let mut txt = format!(
        "🏰 *LOBI DUNGEON*\n\n📊 *Statistik Kamu:*\n> Level: *{user_level}*\n> Stamina: *{stamina}/100*\n\n\
         Pilih lokasi yang ingin kamu jelajahi:\n\n"
    );
    for (id, name, req) in DUNGEONS {
        if user_level >= req {
            txt.push_str(&format!("🔓 *{id}.* {name} (Lv {req}+)\n"));
        } else {
            txt.push_str(&format!("> 🔒 *{id}.* {name} (Butuh Lv {req})\n"));
        }
    }
    txt.push_str(
        "\n> 💡 Balas pesan ini dengan *angka* lokasi yang 🔓 (contoh: `1`) atau ketik `batal` untuk keluar.",
    );
    txt
}

#[plugin(
    name = "dungeon",
    alias = ["dg", "labirin"],
    category = "game",
    description = "Jelajahi dungeon dan lawan monster secara interaktif",
    usage = ".dungeon",
    example = ".dungeon",
    cooldown = 60,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn dungeon(ctx: Ctx) -> Reply {
    // `user.level || 1` — `0`, `null`, dan `NaN` jadi 1; level negatif
    // TIDAK, karena angka negatif truthy di JS dan harus jatuh ke guard level.
    let raw_level = crate::common::int_field(&ctx, "level");
    let user_level = if raw_level == 0 { 1 } else { raw_level };
    let rpg = ctx.user_field("rpg");

    // `if (!user.rpg.X) user.rpg.X = 10/100` — dievaluasi ulang di bawah saat
    // menulis, karena JS menyimpan objeknya in-place lalu `db.save()`.
    let stamina = default_field(rpg, "stamina", 100);

    if let Some(session) = ctx.user_field(SESSION_FIELD)
        && !session.is_null()
    {
        let at = session.get("time").and_then(Value::as_i64).unwrap_or(0);
        if session_still_alive(crate::common::now_ms(), at) {
            return Reply::text(ACTIVE);
        }
        // `delete user.rpg.dungeon_session` lalu jatuh ke lobby — penulisan
        // terjadi di bawah bersama sesi barunya.
    }

    if !DUNGEONS.iter().any(|(_, _, req)| user_level >= *req) {
        return Reply::text(too_low(user_level));
    }

    let mut block = serde_json::Map::new();
    for (key, default) in [
        ("attack", 10),
        ("health", 100),
        ("maxHealth", 100),
        ("stamina", 100),
        ("maxStamina", 100),
    ] {
        block.insert(key.into(), json!(default_field(rpg, key, default)));
    }
    block.insert(
        "dungeon_session".into(),
        json!({ "stage": "lobi", "time": crate::common::now_ms() }),
    );

    let mut data = serde_json::Map::new();
    data.insert("rpg".into(), Value::Object(block));
    if !crate::common::js_truthy(ctx.user_field("inventory")) {
        data.insert("inventory".into(), Value::Object(Default::default()));
    }

    Reply::text(lobby(user_level, stamina)).with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: Value::Object(data),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "dungeon".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: String::new(),
            args: Vec::new(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("dungeon").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    /// Varian dimiliki: untuk `owned(run(..))` di dalam assert.
    fn owned(r: Reply) -> String {
        r.text_body().map(str::to_string).unwrap_or_default()
    }

    fn written(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, SENDER);
                data
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("dungeon").unwrap();
        assert_eq!(p.meta.name, "dungeon");
        assert_eq!(p.meta.alias, ["dg", "labirin"]);
        assert_eq!(p.meta.category, "game");
        assert_eq!(p.meta.description, "Jelajahi dungeon dan lawan monster secara interaktif");
        assert_eq!(p.meta.usage, ".dungeon");
        assert_eq!(p.meta.example, ".dungeon");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["dg", "labirin"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "dungeon");
        }
    }

    #[test]
    fn a_fresh_user_gets_the_lobby_and_a_lobi_session() {
        let r = run(json!({}));
        let t = text_of(&r);
        assert!(t.starts_with("🏰 *LOBI DUNGEON*"), "{t}");
        assert!(t.contains("> Level: *1*"), "{t}");
        assert!(t.contains("> Stamina: *100/100*"), "{t}");
        assert!(t.contains("🔓 *1.* 🌲 Hutan Gelap (Lv 1+)"), "{t}");
        assert!(t.contains("🔒 *3.* 🏰 Kastil Tua (Butuh Lv 10)"), "{t}");

        let data = written(&r);
        assert_eq!(data["rpg"]["dungeon_session"]["stage"], json!("lobi"));
        assert!(data["rpg"]["dungeon_session"]["time"].as_i64().unwrap() > 0);
        assert_eq!(data["rpg"]["attack"], json!(10));
        assert_eq!(data["rpg"]["maxStamina"], json!(100));
        assert_eq!(data["inventory"], json!({}), "`if (!user.inventory)`");
    }

    #[test]
    fn existing_rpg_values_are_preserved_and_zero_falls_back_to_the_default() {
        let r = run(json!({"rpg": {"attack": 55, "stamina": 0, "health": 30}}));
        let data = written(&r);
        assert_eq!(data["rpg"]["attack"], json!(55), "nilai ada tidak ditimpa");
        assert_eq!(data["rpg"]["health"], json!(30));
        assert_eq!(data["rpg"]["stamina"], json!(100), "`!0` → default");
    }

    #[test]
    fn an_existing_inventory_is_not_reset() {
        let r = run(json!({"inventory": {"iron": 3}}));
        let data = written(&r);
        assert!(
            data.get("inventory").is_none(),
            "tidak ada key inventory di patch: {data}"
        );
    }

    #[test]
    fn a_live_session_blocks_a_second_run_and_writes_nothing() {
        let now = crate::common::now_ms();
        let r = run(json!({"rpg": {"dungeon_session": {"stage": "lobi", "time": now}}}));
        assert_eq!(text_of(&r), ACTIVE);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_expired_session_is_replaced_by_a_new_one() {
        let stale = crate::common::now_ms() - SESSION_TIMEOUT_MS - 1;
        let r = run(json!({"rpg": {"dungeon_session": {"stage": "encounter", "time": stale}}}));
        assert!(text_of(&r).starts_with("🏰 *LOBI DUNGEON*"), "{}", text_of(&r));
        let data = written(&r);
        assert_eq!(data["rpg"]["dungeon_session"]["stage"], json!("lobi"));
    }

    #[test]
    fn a_session_exactly_at_the_timeout_is_still_alive() {
        let t0 = 1_000_000_000_000;
        assert!(
            session_still_alive(t0 + SESSION_TIMEOUT_MS, t0),
            "tepat 5 menit masih hidup"
        );
        assert!(
            !session_still_alive(t0 + SESSION_TIMEOUT_MS + 1, t0),
            "1 ms lewat timeout sudah mati"
        );
        assert!(session_still_alive(t0 + 1, t0), "sesi baru hidup");
    }

    #[test]
    fn a_session_a_minute_short_of_the_timeout_keeps_the_lobby_reply() {
        // Batasnya dikunci di `session_still_alive`; lewat handler cukup
        // membuktikan cabang "masih hidup" yang dipilih, dengan margin 1 menit
        // supaya pencatatan jam di test tidak ikut menentukan hasilnya.
        let fresh = crate::common::now_ms() - SESSION_TIMEOUT_MS + 60_000;
        let r = run(json!({"rpg": {"dungeon_session": {"stage": "lobi", "time": fresh}}}));
        assert_eq!(text_of(&r), ACTIVE);
    }

    #[test]
    fn a_session_without_a_timestamp_is_treated_as_expired() {
        let r = run(json!({"rpg": {"dungeon_session": {"stage": "lobi"}}}));
        assert!(text_of(&r).starts_with("🏰 *LOBI DUNGEON*"), "{}", text_of(&r));
    }

    #[test]
    fn the_unlock_line_follows_the_level_threshold_per_dungeon() {
        let t = owned(run(json!({"level": 5})));
        assert!(t.contains("🔓 *1.* 🌲 Hutan Gelap"), "{t}");
        assert!(t.contains("🔓 *2.* 🍄 Rawa Beracun (Lv 5+)"), "{t}");
        assert!(t.contains("🔒 *3.* 🏰 Kastil Tua"), "{t}");
        assert!(t.contains("> Level: *5*"), "{t}");

        let t = owned(run(json!({"level": 50})));
        for (id, _, _) in DUNGEONS {
            assert!(t.contains(&format!("🔓 *{id}.*")), "dungeon {id} belum terbuka: {t}");
        }
    }

    #[test]
    fn level_zero_is_read_as_one_because_of_the_js_or_fallback() {
        let t = owned(run(json!({"level": 0})));
        assert!(t.contains("> Level: *1*"), "{t}");
        assert!(t.contains("🔓 *1.*"), "{t}");
    }

    #[test]
    fn a_negative_level_falls_through_to_the_level_guard() {
        // `-5 || 1` = -5 di JS, jadi level negatif bukan 1 dan guard level
        // yang menangkapnya.
        let r = run(json!({"level": -5}));
        assert_eq!(text_of(&r), too_low(-5));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_level_guard_exists_even_though_the_table_covers_level_one() {
        // Dungeon termurah butuh level 1, jadi cabang ini tidak bisa dipicu
        // lewat handler — diuji langsung supaya tetap hidup.
        assert_eq!(too_low(0), "❌ *LEVEL TERLALU RENDAH*\n\n> Level kamu saat ini adalah *0*. Kamu butuh minimal level *1* untuk memasuki dungeon paling mudah.");
        assert!(DUNGEONS.iter().all(|(_, _, req)| *req >= 1));
    }

    #[test]
    fn a_null_rpg_object_is_handled_without_panicking() {
        for user in [json!(null), json!({}), json!({"rpg": null}), json!({"rpg": "bukan object"})] {
            let r = run(user.clone());
            assert!(text_of(&r).starts_with("🏰 *LOBI DUNGEON*"), "input {user}");
            assert_eq!(written(&r)["rpg"]["stamina"], json!(100), "input {user}");
        }
    }
}
