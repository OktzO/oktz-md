//! `plugins/game/kyubigame.js` — lobi misi shinobi.
//!
//! `ponytail:` sama seperti `dungeon.rs`: `kyubigame.js` mengekspor `handler`
//! dan `kyubigameAnswerHandler`, dan yang kedua bukan command — router global
//! yang memanggilnya. Yang dipindah di sini `handler`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

pub(crate) const SESSION_TIMEOUT_MS: i64 = 5 * 60 * 1000;

/// `now - at <= SESSION_TIMEOUT_MS` (`kyubigame.js:214`) — `<=`, jadi sesi
/// yang umurnya persis 5 menit masih aktif. Dipisah dari handler karena
/// batasnya hanya bisa dikunci persis kalau `now` dikontrol test; lewat
/// handler, `now_ms()` di dalam selalu beberapa ms lebih besar dari yang
/// dibaca test.
pub(crate) fn session_still_running(now: i64, at: i64) -> bool {
    now - at <= SESSION_TIMEOUT_MS
}

pub(crate) const SESSION_FIELD: &str = "rpg.kyubigame_session";

/// `LOCATIONS` — id, nama, level minimal.
pub(crate) const LOCATIONS: [(i64, &str, i64); 7] = [
    (1, "🍃 Desa Konoha", 1),
    (2, "🌳 Hutan Kematian", 5),
    (3, "☁️ Padang Petir", 10),
    (4, "🦇 Gua Akatsuki", 15),
    (5, "🌊 Lembah Akhir", 25),
    (6, "💥 Medan Perang Shinobi", 35),
    (7, "🦊 Kurama's Cage", 50),
];

pub(crate) const ACTIVE: &str = "⚔️ *MISI SHINOBI MASIH AKTIF*\n\n\
     Kamu masih berada di medan pertempuran!\n\
     > Balas pesan terakhir bot dengan (`serang` / `lari`) atau batalkan misi (ketik `batal`).";

/// Cabang `available.length === 0`. Tidak terjangkau lewat handler — lokasi
/// termurah butuh level 1 dan `user.level || 1` minimal 1 — tapi tetap dibawa
/// supaya ambang level tidak bisa berubah diam-diam.
pub(crate) fn too_low(user_level: i64) -> String {
    format!(
        "❌ *LEVEL TERLALU RENDAH*\n\n> Level kamu saat ini adalah *{user_level}*. \
         Kamu butuh minimal level *1* untuk memulai petualangan shinobi."
    )
}

pub(crate) fn lobby(user_level: i64, stamina: i64) -> String {
    let mut txt = format!(
        "⛩️ *LOBI SHINOBI*\n\n📊 *Statistik Shinobi:*\n> Level: *{user_level}*\n> Stamina: *{stamina}/100*\n\n\
         Pilih lokasi misi yang ingin kamu jelajahi:\n\n"
    );
    for (id, name, req) in LOCATIONS {
        if user_level >= req {
            txt.push_str(&format!("🔓 *{id}.* {name} (Lv {req}+)\n"));
        } else {
            txt.push_str(&format!("> 🔒 *{id}.* {name} (Butuh Lv {req})\n"));
        }
    }
    txt.push_str(
        "\n> 💡 Balas pesan ini dengan *angka* lokasi misi (contoh: `1`) atau ketik `batal` untuk keluar.",
    );
    txt
}

fn default_field(rpg: Option<&Value>, key: &str, default: i64) -> i64 {
    let v = rpg.and_then(|r| r.get(key));
    if crate::common::js_truthy(v) {
        v.and_then(Value::as_i64).unwrap_or(default)
    } else {
        default
    }
}

#[plugin(
    name = "kyubigame",
    alias = ["kyubi", "naruto", "shinobi"],
    category = "game",
    description = "Jelajahi dunia shinobi dan hadapi musuh Ninja terkuat",
    usage = ".kyubigame",
    example = ".kyubigame",
    cooldown = 60,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn kyubigame(ctx: Ctx) -> Reply {
    // `user.level || 1` — `0`/`null` jadi 1, negatif tetap negatif (truthy).
    let raw_level = crate::common::int_field(&ctx, "level");
    let user_level = if raw_level == 0 { 1 } else { raw_level };
    let rpg = ctx.user_field("rpg");
    let stamina = default_field(rpg, "stamina", 100);

    if let Some(session) = ctx.user_field(SESSION_FIELD)
        && !session.is_null()
    {
        let at = session.get("time").and_then(Value::as_i64).unwrap_or(0);
        if session_still_running(crate::common::now_ms(), at) {
            return Reply::text(ACTIVE);
        }
    }

    if !LOCATIONS.iter().any(|(_, _, req)| user_level >= *req) {
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
        block.insert(key.into(), serde_json::json!(default_field(rpg, key, default)));
    }
    block.insert(
        "kyubigame_session".into(),
        serde_json::json!({ "stage": "lobi", "time": crate::common::now_ms() }),
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

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "kyubigame".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: String::new(),
            args: Vec::new(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("kyubigame").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("kyubigame").unwrap();
        assert_eq!(p.meta.name, "kyubigame");
        assert_eq!(p.meta.alias, ["kyubi", "naruto", "shinobi"]);
        assert_eq!(p.meta.category, "game");
        assert_eq!(
            p.meta.description,
            "Jelajahi dunia shinobi dan hadapi musuh Ninja terkuat"
        );
        assert_eq!(p.meta.usage, ".kyubigame");
        assert_eq!(p.meta.example, ".kyubigame");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["kyubi", "naruto", "shinobi"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "kyubigame");
        }
    }

    #[test]
    fn a_fresh_user_gets_the_lobby_and_a_lobi_session() {
        let r = run(serde_json::json!({}));
        let t = text_of(&r);
        assert!(t.starts_with("⛩️ *LOBI SHINOBI*"), "{t}");
        assert!(t.contains("> Level: *1*"), "{t}");
        assert!(t.contains("> Stamina: *100/100*"), "{t}");
        assert!(t.contains("🔓 *1.* 🍃 Desa Konoha (Lv 1+)"), "{t}");
        assert!(t.contains("🔒 *3.* ☁️ Padang Petir (Butuh Lv 10)"), "{t}");

        let data = written(&r);
        assert_eq!(data["rpg"]["kyubigame_session"]["stage"], serde_json::json!("lobi"));
        assert!(data["rpg"]["kyubigame_session"]["time"].as_i64().unwrap() > 0);
        assert_eq!(data["rpg"]["attack"], serde_json::json!(10));
        assert_eq!(data["inventory"], serde_json::json!({}));
    }

    #[test]
    fn existing_rpg_values_survive_and_zero_falls_back_to_the_default() {
        let r = run(serde_json::json!({"rpg": {"attack": 77, "stamina": 0}}));
        let data = written(&r);
        assert_eq!(data["rpg"]["attack"], serde_json::json!(77));
        assert_eq!(data["rpg"]["stamina"], serde_json::json!(100));
    }

    #[test]
    fn a_live_mission_blocks_a_second_run() {
        let now = crate::common::now_ms();
        let r = run(serde_json::json!({"rpg": {"kyubigame_session": {"stage": "encounter", "time": now}}}));
        assert_eq!(text_of(&r), ACTIVE);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_expired_mission_is_replaced_by_a_fresh_lobi_session() {
        let stale = crate::common::now_ms() - SESSION_TIMEOUT_MS - 1;
        let r = run(serde_json::json!({"rpg": {"kyubigame_session": {"stage": "encounter", "time": stale}}}));
        assert!(text_of(&r).starts_with("⛩️ *LOBI SHINOBI*"), "{}", text_of(&r));
        assert_eq!(written(&r)["rpg"]["kyubigame_session"]["stage"], serde_json::json!("lobi"));
    }

    #[test]
    fn a_mission_exactly_at_the_timeout_is_still_running() {
        let t0 = 1_000_000_000_000;
        assert!(
            session_still_running(t0 + SESSION_TIMEOUT_MS, t0),
            "tepat 5 menit masih aktif"
        );
        assert!(
            !session_still_running(t0 + SESSION_TIMEOUT_MS + 1, t0),
            "1 ms lewat timeout sudah expired"
        );
        assert!(session_still_running(t0 + 1, t0), "sesi baru aktif");
    }

    #[test]
    fn a_just_short_of_the_timeout_is_dispatched_by_the_handler() {
        // Batasnya dikunci di `session_still_running`; lewat handler cukup
        // membuktikan cabang "masih aktif" benar-benar dipilih, dengan
        // margin 1 menit supaya jam tidak bisa ikut menentukan hasilnya.
        let fresh = crate::common::now_ms() - SESSION_TIMEOUT_MS + 60_000;
        let r = run(serde_json::json!({"rpg": {"kyubigame_session": {"stage": "lobi", "time": fresh}}}));
        assert_eq!(text_of(&r), ACTIVE);
    }

    #[test]
    fn a_session_without_a_timestamp_is_treated_as_expired() {
        let r = run(serde_json::json!({"rpg": {"kyubigame_session": {}}}));
        assert!(text_of(&r).starts_with("⛩️ *LOBI SHINOBI*"), "{}", text_of(&r));
    }

    #[test]
    fn the_lock_line_follows_the_level_threshold_per_location() {
        let t = owned(run(serde_json::json!({"level": 5})));
        assert!(t.contains("🔓 *2.* 🌳 Hutan Kematian (Lv 5+)"), "{t}");
        assert!(t.contains("🔒 *3.* ☁️ Padang Petir"), "{t}");

        let t = owned(run(serde_json::json!({"level": 50})));
        assert!(t.contains("🔓 *7.* 🦊 Kurama's Cage (Lv 50+)"), "{t}");
    }

    #[test]
    fn level_zero_becomes_one_and_a_negative_level_hits_the_guard() {
        let t = owned(run(serde_json::json!({"level": 0})));
        assert!(t.contains("> Level: *1*"), "{t}");

        let r = run(serde_json::json!({"level": -3}));
        assert_eq!(text_of(&r), too_low(-3));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_level_guard_survives_as_dead_branches_are_forbidden() {
        assert!(too_low(-1).starts_with("❌ *LEVEL TERLALU RENDAH*"));
        assert!(LOCATIONS.iter().all(|(_, _, req)| *req >= 1));
    }

    #[test]
    fn a_null_rpg_object_is_handled_without_panicking() {
        for user in [serde_json::json!(null), serde_json::json!({}), serde_json::json!({"rpg": null}), serde_json::json!({"rpg": 7})] {
            let r = run(user.clone());
            assert!(text_of(&r).starts_with("⛩️ *LOBI SHINOBI*"), "input {user}");
            assert_eq!(written(&r)["rpg"]["health"], serde_json::json!(100), "input {user}");
        }
    }
}
