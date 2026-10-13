//! `plugins/game/suitpvp.js` — suit (batu gunting kertas) dengan player lain.
//!
//! `ponytail:` `answerHandler` bukan command; router global yang memanggilnya.
//! Yang dipindah di sini `handler` saja — sama seperti `kyubigame.rs`.
//!
//! `ponytail:` JS memakai `setTimeout` untuk mengakhiri ruang duel. Tanpa
//! timer, ruang yang sudah lewat `TIMEOUT` diabaikan saat handler dipanggil
//! berikutnya — efek yang sama, tanpa pengusir yang berjalan sendiri.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::{Value, json};

/// `TIMEOUT` (`suitpvp.js:20`) — 90 detik.
pub(crate) const TIMEOUT_MS: i64 = 90_000;

/// `global.suitGames` (`suitpvp.js:18`) dipetakan ke session store dengan
/// nama yang sama supaya whoever mem-port router suit tidak perlu tahu.
pub(crate) const GAMES_KEY: &str = "suitGames";

/// `suitpvp.js:38` — sender masih punya duel yang belum selesai.
pub(crate) const BUSY: &str = "❌ Kamu masih dalam game suit!\n\n> Selesaikan game kamu dulu.";

/// `suitpvp.js:59` — menantang diri sendiri.
pub(crate) const SELF: &str = "❌ Tidak bisa menantang diri sendiri!";

/// `suitpvp.js:67` — target sedang duel dengan orang lain.
pub(crate) const TARGET_BUSY: &str = "❌ Orang itu sedang bermain suit dengan orang lain!";

/// `suitpvp.js:51-55` — kartu penggunaan.
pub(crate) fn usage() -> String {
    String::from(
        "✊✌️✋ *sᴜɪᴛ ᴘᴠᴘ*\n\n\
         > Tag orang yang mau kamu tantang!\n\n\
         *Contoh:*\n\
         > `.suit @628xxx`",
    )
}

/// `suitpvp.js:93-98` — teks tantangan.
pub(crate) fn challenge(target: &str) -> String {
    format!(
        "Kamu menantang @{} untuk adu suit\n\n\
         ╭┈┈⬡「 💬 *ʀᴇsᴘᴏɴ* 」\n\
         ┃ ✅ Ketik *terima* / *gas* / *ok*\n\
         ┃ ❌ Ketik *tolak* / *gabisa*\n\
         ╰┈┈┈┈┈┈┈┈⬡\n\n\
         Waktu: 90 detik",
        crate::common::jid_number(target)
    )
}

/// Kamar yang masih hidup untuk `jid`, di-scan pada urutan key yang sama dengan
/// `Object.values(global.suitGames)` di JS.
pub(crate) fn room_of(sessions: Option<&Value>, jid: &str, now: i64) -> Option<String> {
    let games = sessions?.as_object()?;
    for (id, room) in games {
        let created = room.get("createdAt").and_then(Value::as_i64).unwrap_or(0);
        if now - created > TIMEOUT_MS {
            continue;
        }
        if room.get("p").and_then(Value::as_str) == Some(jid)
            || room.get("p2").and_then(Value::as_str) == Some(jid)
        {
            return Some(id.clone());
        }
    }
    None
}

#[plugin(
    name = "suitpvp",
    alias = ["suit", "rps", "janken"],
    category = "game",
    description = "Main suit (batu gunting kertas) dengan player lain",
    usage = ".suit @tag",
    example = ".suit @628xxx",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn suitpvp(ctx: Ctx) -> Reply {
    let now = crate::common::now_ms();
    let games = ctx.session_get(GAMES_KEY);

    if room_of(games.as_ref(), &ctx.sender, now).is_some() {
        return Reply::text(BUSY);
    }

    // `m.quoted.sender` menang atas mention — `suitpvp.js:43-47`.
    let target = ctx
        .quoted()
        .map(|q| q.sender.clone())
        .or_else(|| ctx.mentioned.first().cloned());

    let Some(target) = target.filter(|t| !t.is_empty()) else {
        return Reply::text(usage());
    };

    if target == ctx.sender {
        return Reply::text(SELF);
    }

    if room_of(games.as_ref(), &target, now).is_some() {
        return Reply::text(TARGET_BUSY);
    }

    let room_id = format!("suit_{now}");
    let chat = ctx.chat.clone();
    let p = ctx.sender.clone();
    ctx.session_update(GAMES_KEY, |sessions| {
        let entry = sessions
            .as_object_mut()
            .expect("suitGames selalu objek")
            .entry(room_id.clone())
            .or_insert(Value::Null);
        *entry = json!({
            "id": room_id,
            "chat": chat,
            "p": p,
            "p2": target,
            "status": "waiting",
            "pilih": Value::Null,
            "pilih2": Value::Null,
            "createdAt": now,
        });
    });

    Reply::Text {
        text: challenge(&target),
        mentions: vec![target],
    }
    .with_reaction("✊")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::QuotedMessage;

    const CHAT: &str = "120363@g.us";
    const SENDER: &str = "628111@s.whatsapp.net";
    const OTHER: &str = "628999@s.whatsapp.net";

    fn ctx_with(games: Value) -> Ctx {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "suit".into(),
            sender: SENDER.into(),
            chat: CHAT.into(),
            is_group: true,
            ..Default::default()
        };
        ctx.session_set(GAMES_KEY, games);
        ctx
    }

    fn run(ctx: Ctx) -> Reply {
        (oktz_plugin_api::find("suitpvp").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn room(p: &str, p2: &str, created: i64) -> Value {
        json!({
            "id": "suit_1",
            "chat": CHAT,
            "p": p,
            "p2": p2,
            "status": "waiting",
            "pilih": Value::Null,
            "pilih2": Value::Null,
            "createdAt": created,
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("suitpvp").unwrap();
        assert_eq!(p.meta.name, "suitpvp");
        assert_eq!(p.meta.alias, ["suit", "rps", "janken"]);
        assert_eq!(p.meta.category, "game");
        assert_eq!(p.meta.description, "Main suit (batu gunting kertas) dengan player lain");
        assert_eq!(p.meta.usage, ".suit @tag");
        assert_eq!(p.meta.example, ".suit @628xxx");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["suit", "rps", "janken"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "suitpvp");
        }
    }

    #[test]
    fn without_a_mention_or_a_quote_the_usage_card_appears() {
        let r = run(ctx_with(json!({})));
        assert_eq!(text_of(&r), usage());
        assert!(text_of(&r).contains("Tag orang yang mau kamu tantang!"));
    }

    #[test]
    fn a_sender_still_in_a_room_is_refused_before_the_target_is_read() {
        // Urutan guard di JS: "masih dalam game" diperiksa lebih dulu, jadi
        // comando tanpa target pun tetap kena_branch ini.
        let now = crate::common::now_ms();
        let r = run(ctx_with(json!({ "suit_1": room(SENDER, OTHER, now) })));
        assert_eq!(text_of(&r), BUSY);
    }

    #[test]
    fn a_room_left_by_its_opponent_still_blocks_the_sender() {
        let now = crate::common::now_ms();
        let r = run(ctx_with(json!({ "suit_1": room(OTHER, SENDER, now) })));
        assert_eq!(text_of(&r), BUSY, "pemain kedua juga sedang duel");
    }

    #[test]
    fn challenging_yourself_is_refused_and_writes_nothing() {
        let mut ctx = ctx_with(json!({}));
        ctx.mentioned = vec![SENDER.into()];
        let r = run(ctx.clone());
        assert_eq!(text_of(&r), SELF);
        assert!(
            ctx.session_get(GAMES_KEY)
                .map_or(0, |v| v.as_object().map_or(0, |o| o.len())) == 0,
            "kamar tidak boleh dibuat"
        );
    }

    #[test]
    fn a_target_already_duelling_someone_else_is_refused() {
        let now = crate::common::now_ms();
        let mut ctx = ctx_with(json!({ "suit_1": room("628000@s.whatsapp.net", OTHER, now) }));
        ctx.mentioned = vec![OTHER.into()];
        let r = run(ctx.clone());
        assert_eq!(text_of(&r), TARGET_BUSY);
        assert!(
            ctx.session_get(GAMES_KEY).unwrap().as_object().unwrap().len() == 1,
            "kamar lama tidak boleh ditambahi"
        );
    }

    #[test]
    fn a_mention_creates_the_room_and_answers_with_the_challenge() {
        let mut ctx = ctx_with(json!({}));
        ctx.mentioned = vec![OTHER.into()];
        let r = run(ctx.clone());
        assert_eq!(text_of(&r), challenge(OTHER));
        assert!(text_of(&r).contains("@628999"), "{}", text_of(&r));
        // `with_reaction` membungkus balasan, jadi `mentions` dibaca dari isi
        // `Reaction`, bukan dari `without_mutations()` yang berhenti di sana.
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => match reply.without_mutations() {
                Reply::Text { mentions, .. } => assert_eq!(mentions, &[OTHER.to_string()]),
                other => panic!("harus Text di dalam Reaction, dapat {other:?}"),
            },
            other => panic!("harus dibungkus Reaction, dapat {other:?}"),
        }
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "✊"),
            "harus bereaksi ✊"
        );

        let games = ctx.session_get(GAMES_KEY).unwrap();
        assert_eq!(games.as_object().unwrap().len(), 1);
        let room = &games[games.as_object().unwrap().keys().next().unwrap()];
        assert_eq!(room["chat"], json!(CHAT));
        assert_eq!(room["p"], json!(SENDER));
        assert_eq!(room["p2"], json!(OTHER));
        assert_eq!(room["status"], json!("waiting"));
        assert_eq!(room["pilih"], Value::Null);
        assert_eq!(room["pilih2"], Value::Null);
    }

    #[test]
    fn the_quoted_sender_wins_over_the_mention() {
        // `suitpvp.js:43-47`: `m.quoted` dicek lebih dulu.
        let mut ctx = ctx_with(json!({}));
        ctx.mentioned = vec!["628000@s.whatsapp.net".into()];
        ctx.quoted = Some(QuotedMessage {
            id: "MSG1".into(),
            sender: OTHER.into(),
            chat: CHAT.into(),
            text: String::new(),
            kind: oktz_plugin_api::QuotedKind::Text,
            mimetype: None,
            file_name: None,
            from_me: false,
        });
        run(ctx.clone());
        let games = ctx.session_get(GAMES_KEY).unwrap();
        let key = games.as_object().unwrap().keys().next().unwrap();
        assert_eq!(games[key]["p2"], json!(OTHER));
    }

    #[test]
    fn a_room_older_than_ninety_seconds_no_longer_blocks_anyone() {
        // Tanpa `setTimeout`, kamar basi harus tetap bisa dibuang — kalau
        // tidak, satu duel yang ditinggalkan membekukan kedua pemainnya.
        let now = crate::common::now_ms();
        let stale = room(SENDER, OTHER, now - TIMEOUT_MS - 1_000);
        let mut ctx = ctx_with(json!({ "suit_1": stale }));
        ctx.mentioned = vec!["628000@s.whatsapp.net".into()];
        let r = run(ctx.clone());
        assert!(text_of(&r).starts_with("Kamu menantang"), "{}", text_of(&r));
        assert_eq!(
            ctx.session_get(GAMES_KEY).unwrap().as_object().unwrap().len(),
            2,
            "kamar basi masih ada tapi tidak lagi memblokir"
        );
    }

    #[test]
    fn a_fresh_room_is_still_blocking_at_exactly_ninety_seconds() {
        let now = crate::common::now_ms();
        let games = json!({ "suit_1": room(SENDER, OTHER, now - TIMEOUT_MS + 5_000) });
        assert_eq!(text_of(&run(ctx_with(games))), BUSY);
    }
}