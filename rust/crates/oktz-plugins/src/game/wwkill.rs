//! `plugins/game/wwkill.js` — aksi malam Werewolf: pilih target untuk dibunuh.
//!
//! Keempat plugin `ww*` hanya membungkus `nightActionHandler` dari
//! `werewolf.js`, dan itu bukan fungsi plugin — ia berbagi state global
//! `global.werewolfGames` milik game. State itu dipetakan ke key session
//! `werewolfGames`; nama globalnya dipertahankan supaya whoever mem-port
//! `werewolf.js` memakai key yang sama tanpa perlu tahu pemetaan lain.
//!
//! `ponytail:` router hanya mendaftarkan handler plugin, bukan "answer handler"
//! terpisah. Yang dipindah di sini murni jalur aksi malam; fase siang, vote,
//! dan pengaturan ulang malam tetap milik `werewolf.js`.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::{Map, Value};
#[cfg(test)]
use serde_json::json;

pub(crate) const GAMES_KEY: &str = "werewolfGames";

/// `ROLES` (`werewolf.js:35`): kunci, emoji, nama, tim.
pub(crate) const ROLES: [(&str, &str, &str, &str); 5] = [
    ("werewolf", "🐺", "Werewolf", "wolf"),
    ("seer", "🔮", "Seer", "village"),
    ("guardian", "🛡️", "Guardian", "village"),
    ("sorcerer", "🧙", "Sorcerer", "wolf"),
    ("villager", "👨‍🌾", "Villager", "village"),
];

fn lookup(key: &str) -> Option<(&'static str, &'static str, &'static str)> {
    ROLES
        .iter()
        .find(|(k, ..)| *k == key)
        .map(|(_, emoji, name, team)| (*emoji, *name, *team))
}

/// `ROLES[role]?.name || "Unknown"`.
pub(crate) fn role_name(key: &str) -> &'static str {
    lookup(key).map_or("Unknown", |(_, name, _)| name)
}

/// `te(prefix, command, pushName)` dengan template default (`error.js`).
/// `ponytail:` `Ctx` tidak membawa `pushName` dan plugin tidak bisa membaca
/// `config.errorTemplate`, jadi keduanya jatuh ke fallback JS.
pub(crate) fn error_reply(prefix: &str, command: &str) -> Reply {
    Reply::text(format!(
        "☢ *ᴇʀʀᴏʀ*\n\n> Terjadi kesalahan pada command `{prefix}{command}`\n\
         > Silahkan coba lagi nanti, User\n\n_Jika masalah berlanjut, hubungi owner_"
    ))
}

/// Hasil aksi malam: teks balasan + apa yang harus ditulis ke state game.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Plan {
    pub text: String,
    pub mentions: Vec<String>,
    /// `game.nightActions.kill`
    pub kill: Option<String>,
    /// `game.nightActions.protect`
    pub protect: Option<String>,
    /// `player.skillUsed = true` — di empat cabang sukses.
    pub skill_used: bool,
}

/// Menolak aksi malam tanpa efek samping. `NoTarget` dipisah dari `Message`
/// karena teksnya dirangkai router (butuh `prefix` + `command`), sedangkan
/// pesan lain sudah lengkap.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    NoTarget,
    /// `wwsee` meledak `TypeError` atas role yang tidak dikenal; `wwsee.js`
    /// menangkapnya dan membalas template error.
    Template,
    Message(&'static str),
}

pub(crate) fn players(game: &Value) -> Vec<Value> {
    game.get("players")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// `ww[id].players.some(p => p.id === m.sender && p.alive) && phase === "night"`.
pub(crate) fn in_night(game: &Value, sender: &str) -> bool {
    game.get("phase").and_then(Value::as_str) == Some("night")
        && players(game).iter().any(|p| {
            p.get("id").and_then(Value::as_str) == Some(sender)
                && p.get("alive") == Some(&Value::Bool(true))
        })
}

/// `nightActionHandler` (`werewolf.js:1042`) setelah game ditemukan.
///
/// Urutan guard di sini persis JS dan tidak boleh ditukar: `skillUsed`
/// diperiksa SEBELUM target dibaca, jadi player yang sudah memakai skill
/// dapat pesan "sudah menggunakan skill", bukan "Masukkan nomor target".
pub(crate) fn plan(
    game: &Value,
    sender: &str,
    cmd: &str,
    target_num: Option<i64>,
) -> Result<Plan, Refusal> {
    let list = players(game);
    fn role_of(p: &Value) -> &str {
        p.get("role").and_then(Value::as_str).unwrap_or("")
    }

    let Some(player) = list.iter().find(|p| p.get("id").and_then(Value::as_str) == Some(sender))
    else {
        return Err(Refusal::Message(
            "❌ Kamu sudah mati atau bukan player!",
        ));
    };
    if player.get("alive") != Some(&Value::Bool(true)) {
        return Err(Refusal::Message(
            "❌ Kamu sudah mati atau bukan player!",
        ));
    }
    if crate::common::js_truthy(player.get("skillUsed")) {
        return Err(Refusal::Message(
            "❌ Kamu sudah menggunakan skill malam ini!",
        ));
    }

    let Some(target_num) = target_num else {
        return Err(Refusal::NoTarget);
    };
    let Some(target) = list.iter().find(|p| {
        p.get("number").and_then(Value::as_i64) == Some(target_num)
            && p.get("alive") == Some(&Value::Bool(true))
    }) else {
        return Err(Refusal::Message(
            "❌ Target tidak valid atau sudah mati!",
        ));
    };

    let target_id = target.get("id").and_then(Value::as_str).unwrap_or("").to_string();
    let target_role = role_of(target);
    let my_role = role_of(player);
    let num = crate::common::jid_number(&target_id);
    let mention = || vec![target_id.clone()];

    match (cmd, my_role) {
        ("wwkill", "werewolf") => {
            if target_role == "werewolf" || target_role == "sorcerer" {
                return Err(Refusal::Message("❌ Tidak bisa membunuh sesama team!"));
            }
            Ok(Plan {
                text: format!(
                    "🐺 *TARGET TERPILIH*\n\nTarget: @{num}\n> Menunggu malam berakhir..."
                ),
                mentions: mention(),
                kill: Some(target_id),
                protect: None,
                skill_used: true,
            })
        }
        ("wwprotect", "guardian") => Ok(Plan {
            text: format!(
                "🛡️ *TARGET DILINDUNGI*\n\nMelindungi: @{num}\n> Menunggu malam berakhir..."
            ),
            mentions: mention(),
            kill: None,
            protect: Some(target_id),
            skill_used: true,
        }),
        ("wwsee", "seer") => {
            let Some((emoji, name, team)) = lookup(target_role) else {
                return Err(Refusal::Template);
            };
            Ok(Plan {
                text: format!(
                    "🔮 *HASIL PENGLIHATAN*\n\n@{num} adalah:\n{emoji} *{name}*\n\n> Team: {}",
                    if team == "wolf" { "🐺 Wolf" } else { "👨‍🌾 Village" }
                ),
                mentions: mention(),
                kill: None,
                protect: None,
                skill_used: true,
            })
        }
        ("wwsorcerer", "sorcerer") => Ok(Plan {
            text: format!(
                "🧙 *HASIL INVESTIGASI*\n\n@{num}\n{}\n\n> Lanjutkan membantu Werewolf!",
                if target_role == "seer" {
                    "✅ *adalah SEER!*"
                } else {
                    "❌ *bukan Seer*"
                }
            ),
            mentions: mention(),
            kill: None,
            protect: None,
            skill_used: true,
        }),
        _ => Ok(Plan {
            text: format!(
                "❌ Kamu tidak memiliki kemampuan ini!\n> Role kamu: {}",
                role_name(my_role)
            ),
            mentions: Vec::new(),
            kill: None,
            protect: None,
            skill_used: false,
        }),
    }
}

/// Terapkan `Plan` ke salinan game dan kembalikan versi barunya.
pub(crate) fn apply(game: &Value, sender: &str, plan: &Plan) -> Value {
    let mut g = game.clone();
    if let Some(actions) = g.get_mut("nightActions").and_then(Value::as_object_mut) {
        if let Some(t) = &plan.kill {
            actions.insert("kill".into(), Value::String(t.clone()));
        }
        if let Some(t) = &plan.protect {
            actions.insert("protect".into(), Value::String(t.clone()));
        }
    }
    if plan.skill_used
        && let Some(list) = g.get_mut("players").and_then(Value::as_array_mut)
    {
        for p in list.iter_mut() {
            if p.get("id").and_then(Value::as_str) == Some(sender)
                && let Some(o) = p.as_object_mut()
            {
                o.insert("skillUsed".into(), Value::Bool(true));
            }
        }
    }
    g
}

pub(crate) const NOT_IN_GAME: &str =
    "❌ Kamu tidak sedang dalam game werewolf atau bukan fase malam!";

/// Badan penuh `nightActionHandler`: cari game, rencanakan, tulis balik.
pub(crate) fn night_action(ctx: &Ctx, cmd: &str) -> Reply {
    let prefix = if ctx.prefix.is_empty() {
        "."
    } else {
        &ctx.prefix
    };
    let target_num = ctx.args.first().and_then(|a| crate::common::parse_int(a));

    let mut out: Option<Reply> = None;
    ctx.session_update(GAMES_KEY, |slot| {
        let mut games: Map<String, Value> = slot.as_object().cloned().unwrap_or_default();

        let Some(chat) = games
            .iter()
            .find(|(_, g)| in_night(g, &ctx.sender))
            .map(|(k, _)| k.clone())
        else {
            out = Some(Reply::text(NOT_IN_GAME));
            return;
        };

        let game = games[&chat].clone();
        match plan(&game, &ctx.sender, cmd, target_num) {
            Err(Refusal::NoTarget) => out = Some(Reply::text(format!(
                "❌ Masukkan nomor target! Contoh: `{prefix}{cmd} 2`"
            ))),
            Err(Refusal::Template) => out = Some(error_reply(prefix, cmd)),
            Err(Refusal::Message(msg)) => out = Some(Reply::text(msg)),
            Ok(plan) => {
                games.insert(chat, apply(&game, &ctx.sender, &plan));
                *slot = Value::Object(games);
                out = Some(Reply::Text {
                    text: plan.text,
                    mentions: plan.mentions,
                });
            }
        }
    });

    out.unwrap_or_else(|| Reply::text(NOT_IN_GAME))
}

#[cfg(test)]
pub(crate) fn game_fixture(phase: &str, players: Value) -> Value {
    json!({
        "chat": "120363@g.us",
        "phase": phase,
        "nightActions": {},
        "players": players,
    })
}

#[cfg(test)]
pub(crate) fn player(id: &str, number: i64, role: &str, alive: bool, skill_used: bool) -> Value {
    json!({
        "id": id,
        "number": number,
        "role": role,
        "alive": alive,
        "skillUsed": skill_used,
    })
}

/// Bangun `Ctx` untuk aksi malam atas state game tertentu.
#[cfg(test)]
pub(crate) fn night_ctx(
    sender: &str,
    cmd: &str,
    arg: &str,
    games: Map<String, Value>,
) -> Ctx {
    Ctx {
        prefix: ".".into(),
        command: cmd.into(),
        sender: sender.into(),
        chat: sender.into(),
        text: arg.into(),
        args: if arg.is_empty() {
            Vec::new()
        } else {
            vec![arg.into()]
        },
        sessions: {
            let store = oktz_plugin_api::new_session_store();
            store.lock().unwrap().insert(GAMES_KEY.into(), Value::Object(games));
            store
        },
        ..Default::default()
    }
}

#[plugin(
    name = "wwkill",
    alias = ["wolfkill", "wk"],
    category = "game",
    description = "Werewolf night action - Kill target",
    usage = ".wwkill <nomor>",
    example = ".wwkill 2",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = true,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn wwkill(ctx: Ctx) -> Reply {
    night_action(&ctx, "wwkill")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const WOLF: &str = "628111@s.whatsapp.net";
    const OTHER: &str = "628999@s.whatsapp.net";

    fn night_game() -> Value {
        game_fixture(
            "night",
            json!([
                player(WOLF, 1, "werewolf", true, false),
                player(OTHER, 2, "villager", true, false),
            ]),
        )
    }

    fn games(game: Option<Value>) -> Map<String, Value> {
        let mut m = Map::new();
        if let Some(g) = game {
            m.insert("120363@g.us".into(), g);
        }
        m
    }

    fn run(cmd: &str, game: Option<Value>, arg: &str) -> Reply {
        let ctx = night_ctx(WOLF, cmd, arg, games(game));
        night_action(&ctx, cmd)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    /// Varian dimiliki: untuk `owned(run(..))` di dalam assert.
    fn owned(r: Reply) -> String {
        r.text_body().map(str::to_string).unwrap_or_default()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("wwkill").unwrap();
        assert_eq!(p.meta.name, "wwkill");
        assert_eq!(p.meta.alias, ["wolfkill", "wk"]);
        assert_eq!(p.meta.category, "game");
        assert_eq!(p.meta.description, "Werewolf night action - Kill target");
        assert_eq!(p.meta.usage, ".wwkill <nomor>");
        assert_eq!(p.meta.example, ".wwkill 2");
        assert_eq!(p.meta.cooldown, 0);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn outside_a_night_game_every_action_is_refused() {
        for cmd in ["wwkill", "wwprotect", "wwsee", "wwsorcerer"] {
            assert_eq!(owned(run(cmd, None, "1")), NOT_IN_GAME, "cmd {cmd}");
        }
        let day = game_fixture("day", json!([player(WOLF, 1, "werewolf", true, false)]));
        assert_eq!(owned(run("wwkill", Some(day), "1")), NOT_IN_GAME);
    }

    #[test]
    fn a_dead_sender_is_refused_by_the_membership_check() {
        let dead = game_fixture(
            "night",
            json!([
                player(WOLF, 1, "werewolf", false, false),
                player(OTHER, 2, "villager", true, false),
            ]),
        );
        assert_eq!(owned(run("wwkill", Some(dead), "2")), NOT_IN_GAME);

        // `in_night` sudah menyaring, jadi cabang "bukan player" di `plan` cuma
        // bisa diuji langsung.
        assert_eq!(
            plan(&night_game(), "628000@s.whatsapp.net", "wwkill", Some(2)),
            Err(Refusal::Message("❌ Kamu sudah mati atau bukan player!"))
        );
    }

    #[test]
    fn an_already_used_skill_is_refused_before_the_target_is_read() {
        let g = game_fixture(
            "night",
            json!([
                player(WOLF, 1, "werewolf", true, true),
                player(OTHER, 2, "villager", true, false),
            ]),
        );
        assert_eq!(
            plan(&g, WOLF, "wwkill", None),
            Err(Refusal::Message("❌ Kamu sudah menggunakan skill malam ini!"))
        );
        assert_eq!(
            owned(run("wwkill", Some(g), "")),
            "❌ Kamu sudah menggunakan skill malam ini!",
            "skillUsed dicek sebelum `isNaN(target)`"
        );
    }

    #[test]
    fn a_missing_or_unparsable_target_number_shows_the_usage_hint() {
        for cmd in ["wwkill", "wwprotect", "wwsee", "wwsorcerer"] {
            assert_eq!(
                owned(run(cmd, Some(night_game()), "")),
                format!("❌ Masukkan nomor target! Contoh: `.{cmd} 2`"),
                "cmd {cmd}"
            );
            assert!(
                owned(run(cmd, Some(night_game()), "abc"))
                    .starts_with("❌ Masukkan nomor target!"),
                "cmd {cmd}: {}",
                owned(run(cmd, Some(night_game()), "abc"))
            );
        }
    }

    #[test]
    fn a_dead_or_unknown_target_is_refused() {
        let g = game_fixture(
            "night",
            json!([
                player(WOLF, 1, "werewolf", true, false),
                player(OTHER, 2, "villager", false, false),
            ]),
        );
        for n in [2, 99] {
            assert_eq!(
                plan(&g, WOLF, "wwkill", Some(n)),
                Err(Refusal::Message("❌ Target tidak valid atau sudah mati!")),
                "target {n}"
            );
        }
    }

    #[test]
    fn the_wolf_can_target_the_village_but_not_its_own_team() {
        let p = plan(&night_game(), WOLF, "wwkill", Some(2)).expect("harus berhasil");
        assert_eq!(p.kill.as_deref(), Some(OTHER));
        assert!(p.skill_used);
        assert!(p.text.contains("🐺 *TARGET TERPILIH*"), "{}", p.text);
        assert!(p.text.contains("Target: @628999"), "{}", p.text);
        assert_eq!(p.mentions, vec![OTHER.to_string()]);

        for ally in ["sorcerer", "werewolf"] {
            let g = game_fixture(
                "night",
                json!([
                    player(WOLF, 1, "werewolf", true, false),
                    player(OTHER, 2, ally, true, false),
                ]),
            );
            assert_eq!(
                plan(&g, WOLF, "wwkill", Some(2)),
                Err(Refusal::Message("❌ Tidak bisa membunuh sesama team!")),
                "ally {ally}"
            );
        }
    }

    #[test]
    fn a_wrong_role_falls_through_to_the_no_ability_message() {
        let g = night_game();
        let p = plan(&g, WOLF, "wwprotect", Some(2)).expect("branch akhir");
        assert_eq!(
            p.text,
            "❌ Kamu tidak memiliki kemampuan ini!\n> Role kamu: Werewolf",
            "`ROLES[role].name` tidak ikut memuat emoji — emoji field terpisah"
        );
        assert!(p.kill.is_none() && p.protect.is_none());
        assert!(!p.skill_used && p.mentions.is_empty());

        // Alias bukan nama command kanonik → tidak cocok cabang mana pun.
        let p = plan(&g, WOLF, "wk", Some(2)).expect("alias");
        assert!(p.text.contains("tidak memiliki kemampuan ini"), "{}", p.text);
    }

    #[test]
    fn applying_a_plan_writes_the_night_action_and_only_the_users_skill_flag() {
        let g = night_game();
        let p = plan(&g, WOLF, "wwkill", Some(2)).unwrap();
        let after = apply(&g, WOLF, &p);
        assert_eq!(after["nightActions"]["kill"], json!(OTHER));
        assert_eq!(after["players"][0]["skillUsed"], json!(true));
        assert_eq!(
            after["players"][1]["skillUsed"],
            json!(false),
            "player lain tidak tersentuh"
        );
    }

    #[test]
    fn a_guard_reply_leaves_the_session_untouched() {
        let ctx = night_ctx(WOLF, "wwkill", "", games(Some(night_game())));
        let r = night_action(&ctx, "wwkill");
        assert!(text_of(&r).starts_with("❌ Masukkan nomor target!"));
        let g = ctx.session_get(GAMES_KEY).unwrap();
        assert!(
            g["120363@g.us"]["nightActions"].as_object().unwrap().is_empty(),
            "tidak ada nightActions yang tertulis"
        );
    }

    #[test]
    fn a_successful_action_persists_into_the_session() {
        let ctx = night_ctx(WOLF, "wwkill", "2", games(Some(night_game())));
        let r = night_action(&ctx, "wwkill");
        assert!(text_of(&r).contains("TARGET TERPILIH"));
        let g = ctx.session_get(GAMES_KEY).unwrap();
        assert_eq!(g["120363@g.us"]["nightActions"]["kill"], json!(OTHER));
        assert_eq!(g["120363@g.us"]["players"][0]["skillUsed"], json!(true));
        match r.without_mutations() {
            Reply::Text { mentions, .. } => assert_eq!(mentions, &[OTHER.to_string()]),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn the_role_table_matches_werewolf_js() {
        assert_eq!(role_name("villager"), "Villager");
        assert_eq!(role_name("wizard"), "Unknown");
        assert_eq!(ROLES.len(), 5);
        assert_eq!(lookup("seer").map(|(_, n, _)| n), Some("Seer"));
        assert_eq!(lookup("werewolf").map(|(_, _, t)| t), Some("wolf"));
        assert_eq!(lookup("guardian").map(|(_, _, t)| t), Some("village"));
    }

    #[test]
    fn parse_int_target_semantics_follow_the_helper() {
        // `parseInt("2abc")` = 2 di JS, bukan NaN — target-nya tetap ditemukan.
        let ctx = night_ctx(WOLF, "wwkill", "2abc", games(Some(night_game())));
        let r = night_action(&ctx, "wwkill");
        assert!(text_of(&r).contains("TARGET TERPILIH"), "{}", text_of(&r));
    }
}
