//! `plugins/game/wwsee.js` — aksi malam Werewolf: intip role target.
//!
//! Seluruh logika ada di [`crate::game::wwkill`]; file ini hanya mendaftarkan
//! config plugin-nya, persis seperti `wwsee.js` yang hanya meneruskan ke
//! `nightActionHandler`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::game::wwkill::night_action;

#[plugin(
    name = "wwsee",
    alias = ["seer", "vision", "wse"],
    category = "game",
    description = "Seer night action - See target role",
    usage = ".wwsee <nomor>",
    example = ".wwsee 1",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = true,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn wwsee(ctx: Ctx) -> Reply {
    night_action(&ctx, "wwsee")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::wwkill::{GAMES_KEY, game_fixture, night_ctx, player};
    use serde_json::{Map, Value};

    const SEER: &str = "628111@s.whatsapp.net";
    const TARGET: &str = "628999@s.whatsapp.net";

    fn game(target_role: &str) -> Value {
        game_fixture(
            "night",
            serde_json::json!([
                player(SEER, 1, "seer", true, false),
                player(TARGET, 2, target_role, true, false),
            ]),
        )
    }

    fn run(arg: &str, g: Option<Value>) -> Reply {
        let mut games = Map::new();
        if let Some(g) = g {
            games.insert("120363@g.us".into(), g);
        }
        let ctx = night_ctx(SEER, "wwsee", arg, games);
        (oktz_plugin_api::find("wwsee").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("wwsee").unwrap();
        assert_eq!(p.meta.name, "wwsee");
        assert_eq!(p.meta.alias, ["seer", "vision", "wse"]);
        assert_eq!(p.meta.category, "game");
        assert_eq!(p.meta.description, "Seer night action - See target role");
        assert_eq!(p.meta.usage, ".wwsee <nomor>");
        assert_eq!(p.meta.example, ".wwsee 1");
        assert_eq!(p.meta.cooldown, 0);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["seer", "vision", "wse"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "wwsee");
        }
    }

    #[test]
    fn a_village_target_is_revealed_as_village_and_a_wolf_as_wolf() {
        let r = run("2", Some(game("villager")));
        let t = text_of(&r);
        assert!(t.contains("🔮 *HASIL PENGLIHATAN*"), "{t}");
        assert!(t.contains("@628999 adalah:\n👨‍🌾 *Villager*"), "{t}");
        assert!(t.contains("> Team: 👨‍🌾 Village"), "{t}");

        let r = run("2", Some(game("werewolf")));
        let t = text_of(&r);
        assert!(t.contains("🐺 *Werewolf*"), "{t}");
        assert!(t.contains("> Team: 🐺 Wolf"), "{t}");
    }

    #[test]
    fn revealing_consumes_the_skill_without_touching_night_actions() {
        let games: Map<String, Value> = [("120363@g.us".to_string(), game("villager"))]
            .into_iter()
            .collect();
        let ctx = night_ctx(SEER, "wwsee", "2", games);
        let r = (oktz_plugin_api::find("wwsee").unwrap().handler)(ctx.clone());
        assert!(text_of(&r).contains("HASIL PENGLIHATAN"));
        let g = ctx.session_get(GAMES_KEY).unwrap();
        assert!(
            g["120363@g.us"]["nightActions"].as_object().unwrap().is_empty(),
            "wwsee tidak menulis nightActions"
        );
        assert_eq!(g["120363@g.us"]["players"][0]["skillUsed"], serde_json::json!(true));
    }

    #[test]
    fn an_unknown_target_role_falls_back_to_the_error_template() {
        // JS melempar `TypeError` di `ROLES[targetPlayer.role].emoji`, dan
        // `try/catch` di `wwsee.js` membalas template error — bukan role.
        let r = run("2", Some(game("wizard")));
        let t = text_of(&r);
        assert!(t.starts_with("☢ Kayaknya command "), "{t}");
        assert!(t.contains("command `.wwsee`"), "{t}");
    }

    #[test]
    fn without_a_number_nothing_is_written() {
        let r = run("", Some(game("villager")));
        assert_eq!(
            text_of(&r),
            "❌ Masukkan nomor target! Contoh: `.wwsee 2`"
        );
    }
}
