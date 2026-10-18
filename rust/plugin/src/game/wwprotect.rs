//! `plugins/game/wwprotect.js` — aksi malam Werewolf: lindungi target.
//!
//! Seluruh logika ada di [`crate::game::wwkill`]; file ini hanya mendaftarkan
//! config plugin-nya, persis seperti `wwprotect.js` yang hanya meneruskan ke
//! `nightActionHandler`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::game::wwkill::night_action;

#[plugin(
    name = "wwprotect",
    alias = ["protect", "guardian", "wpr"],
    category = "game",
    description = "Guardian night action - Protect target",
    usage = ".wwprotect <nomor>",
    example = ".wwprotect 3",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = true,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn wwprotect(ctx: Ctx) -> Reply {
    night_action(&ctx, "wwprotect")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::wwkill::{
        NOT_IN_GAME, Refusal, game_fixture, in_night, night_ctx, plan, player,
    };

    const GUARD: &str = "628111@s.whatsapp.net";
    const OTHER: &str = "628999@s.whatsapp.net";

    fn game() -> serde_json::Value {
        game_fixture(
            "night",
            serde_json::json!([
                player(GUARD, 1, "guardian", true, false),
                player(OTHER, 3, "villager", true, false),
            ]),
        )
    }

    fn run(arg: &str, g: Option<serde_json::Value>) -> Reply {
        let mut games = serde_json::Map::new();
        if let Some(g) = g {
            games.insert("120363@g.us".into(), g);
        }
        let ctx = night_ctx(GUARD, "wwprotect", arg, games);
        (oktz_plugin_api::find("wwprotect").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("wwprotect").unwrap();
        assert_eq!(p.meta.name, "wwprotect");
        assert_eq!(p.meta.alias, ["protect", "guardian", "wpr"]);
        assert_eq!(p.meta.category, "game");
        assert_eq!(p.meta.description, "Guardian night action - Protect target");
        assert_eq!(p.meta.usage, ".wwprotect <nomor>");
        assert_eq!(p.meta.example, ".wwprotect 3");
        assert_eq!(p.meta.cooldown, 0);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["protect", "guardian", "wpr"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "wwprotect");
        }
    }

    #[test]
    fn a_guardian_chooses_a_protection_target_and_marks_the_skill_used() {
        let ctx = night_ctx(
            GUARD,
            "wwprotect",
            "3",
            [("120363@g.us".to_string(), game())].into_iter().collect(),
        );
        let r = (oktz_plugin_api::find("wwprotect").unwrap().handler)(ctx.clone());
        let t = r.without_mutations().text_body().unwrap();
        assert!(t.contains("🛡️ *TARGET DILINDUNGI*"), "{t}");
        assert!(t.contains("Melindungi: @628999"), "{t}");
        let g = ctx.session_get(crate::game::wwkill::GAMES_KEY).unwrap();
        assert_eq!(g["120363@g.us"]["nightActions"]["protect"], serde_json::json!(OTHER));
        assert!(g["120363@g.us"]["nightActions"].get("kill").is_none());
        assert_eq!(g["120363@g.us"]["players"][0]["skillUsed"], serde_json::json!(true));
        match r.without_mutations() {
            Reply::Text { mentions, .. } => assert_eq!(mentions, &[OTHER.to_string()]),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn without_a_number_the_usage_hint_appears_and_nothing_is_written() {
        let r = run("", Some(game()));
        assert_eq!(
            text_of(&r),
            "❌ Masukkan nomor target! Contoh: `.wwprotect 2`"
        );
        assert_eq!(
            plan(&game(), GUARD, "wwprotect", None),
            Err(Refusal::NoTarget)
        );
    }

    #[test]
    fn outside_a_night_game_the_entry_guard_wins_over_the_role_guard() {
        assert_eq!(owned(run("3", None)), NOT_IN_GAME);
        assert!(in_night(&game(), GUARD), "fixture-nya fase malam");
    }

    #[test]
    fn a_non_guardian_role_falls_through_to_the_no_ability_message() {
        let g = game_fixture(
            "night",
            serde_json::json!([
                player(GUARD, 1, "werewolf", true, false),
                player(OTHER, 3, "villager", true, false),
            ]),
        );
        let r = run("3", Some(g));
        assert!(text_of(&r).contains("Role kamu: Werewolf"), "{}", text_of(&r));
    }
}
