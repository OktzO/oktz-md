//! `plugins/game/wwsorcerer.js` — aksi malam Werewolf: cek apakah target Seer.
//!
//! Seluruh logika ada di [`crate::game::wwkill`]; file ini hanya mendaftarkan
//! config plugin-nya, persis seperti `wwsorcerer.js` yang meneruskan ke
//! `nightActionHandler` tanpa `try/catch`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::game::wwkill::night_action;

#[plugin(
    name = "wwsorcerer",
    alias = ["sorcerer", "wws"],
    category = "game",
    description = "Sorcerer night action - Check if target is Seer",
    usage = ".wwsorcerer <nomor>",
    example = ".wwsorcerer 3",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = true,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn wwsorcerer(ctx: Ctx) -> Reply {
    night_action(&ctx, "wwsorcerer")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::wwkill::{GAMES_KEY, game_fixture, night_ctx, player};
    use serde_json::{Map, Value};

    const SORC: &str = "628111@s.whatsapp.net";
    const TARGET: &str = "628999@s.whatsapp.net";

    fn game(target_role: &str) -> Value {
        game_fixture(
            "night",
            serde_json::json!([
                player(SORC, 1, "sorcerer", true, false),
                player(TARGET, 3, target_role, true, false),
            ]),
        )
    }

    fn run(arg: &str, g: Option<Value>) -> Reply {
        let mut games = Map::new();
        if let Some(g) = g {
            games.insert("120363@g.us".into(), g);
        }
        let ctx = night_ctx(SORC, "wwsorcerer", arg, games);
        (oktz_plugin_api::find("wwsorcerer").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("wwsorcerer").unwrap();
        assert_eq!(p.meta.name, "wwsorcerer");
        assert_eq!(p.meta.alias, ["sorcerer", "wws"]);
        assert_eq!(p.meta.category, "game");
        assert_eq!(p.meta.description, "Sorcerer night action - Check if target is Seer");
        assert_eq!(p.meta.usage, ".wwsorcerer <nomor>");
        assert_eq!(p.meta.example, ".wwsorcerer 3");
        assert_eq!(p.meta.cooldown, 0);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["sorcerer", "wws"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "wwsorcerer");
        }
    }

    #[test]
    fn a_seer_target_is_flagged_and_anything_else_is_not() {
        let r = run("3", Some(game("seer")));
        let t = text_of(&r);
        assert!(t.contains("🧙 *HASIL INVESTIGASI*"), "{t}");
        assert!(t.contains("@628999\n✅ *adalah SEER!*"), "{t}");

        for role in ["villager", "werewolf", "guardian", "wizard"] {
            let r = run("3", Some(game(role)));
            assert!(
                text_of(&r).contains("❌ *bukan Seer*"),
                "role {role}: {}",
                text_of(&r)
            );
        }
    }

    #[test]
    fn an_unknown_target_role_does_not_fall_back_to_the_error_template() {
        // `wwsorcerer.js` tidak punya `try/catch`, jadi role asing hanya
        // berarti "bukan Seer" — berbeda dari `wwsee`.
        let r = run("3", Some(game("wizard")));
        assert!(text_of(&r).contains("bukan Seer"), "{}", text_of(&r));
        assert!(!text_of(&r).contains("Kayaknya command "));
    }

    #[test]
    fn the_skill_is_consumed_and_night_actions_stay_untouched() {
        let games: Map<String, Value> = [("120363@g.us".to_string(), game("seer"))]
            .into_iter()
            .collect();
        let ctx = night_ctx(SORC, "wwsorcerer", "3", games);
        let r = (oktz_plugin_api::find("wwsorcerer").unwrap().handler)(ctx.clone());
        assert!(text_of(&r).contains("adalah SEER"));
        let g = ctx.session_get(GAMES_KEY).unwrap_or_default();
        assert!(g["120363@g.us"]["nightActions"].as_object().unwrap().is_empty());
        assert_eq!(g["120363@g.us"]["players"][0]["skillUsed"], serde_json::json!(true));
    }

    #[test]
    fn without_a_number_the_usage_hint_appears() {
        assert_eq!(
            owned(run("", Some(game("seer")))),
            "❌ Masukkan nomor target! Contoh: `.wwsorcerer 2`"
        );
    }
}
