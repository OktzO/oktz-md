//! `plugins/user/bataldaftar.js` — batalkan sesi pendaftaran yang aktif.
//!
//! `clearRegistrationSession` menghapus `global.registrationSessions[key]`,
//! jadi padanannya persis satu `ctx.session_take(key)` yang mengembalikan
//! `Some` kalau memang ada sesinya.
//!
//! `ponytail:` `getRegistrationSessionKey` di JS menormalkan `@lid` lebih dulu
//! lewat `lidToJid` (`src/lib/lid.js` — butuh cache file + `onigis`), jadi
//! kunci di sini cukup digit dari `m.sender` penuh (suffix `:device` ikut
//! terhitung, sama seperti `replace(/[^0-9]/g,"")`). Plugin `daftar` yang
//! menulis sesi harus memakai kunci yang sama; lihat `ponytail:` di modul itu.

use oktz_plugin_api::{Ctx, Reply, plugin};

const NONE: &str = "❌ Kamu tidak punya sesi pendaftaran aktif.";

/// `getRegistrationSessionKey` (daftar.js:90) + kunci legacy yang dicoba
/// `getRegistrationSessionEntry` (daftar.js:108).
fn keys_for(ctx: &Ctx) -> Vec<String> {
    let raw = ctx.sender.trim();
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    let mut keys = vec![if digits.is_empty() {
        raw.to_lowercase()
    } else {
        digits
    }];
    if !keys[0].is_empty() && keys[0] != raw {
        keys.push(raw.to_string());
    }
    keys
}

#[plugin(
    name = "bataldaftar",
    alias = ["cancelreg", "canceldaftar", "regcancel"],
    category = "user",
    description = "Batalkan sesi pendaftaran yang sedang aktif",
    usage = ".bataldaftar",
    example = ".bataldaftar",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    skip_registration = true
)]
pub fn bataldaftar(ctx: Ctx) -> Reply {
    for k in keys_for(&ctx) {
        if !k.is_empty() && ctx.session_take(&k).is_some() {
            return Reply::text(format!(
                "✅ Sesi pendaftaran berhasil dibatalkan.\n\n> Mulai lagi dengan: `{}daftar`",
                ctx.prefix
            ));
        }
    }
    Reply::text(NONE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{Ctx, SessionStore, new_session_store};
    use serde_json::{Value};

    fn run(sessions: Value, sender: &str) -> (Reply, bool) {
        let store: SessionStore = new_session_store();
        for (k, v) in sessions.as_object().unwrap() {
            store.lock().unwrap().insert(k.clone(), v.clone());
        }
        let ctx = Ctx {
            prefix: ".".into(),
            command: "bataldaftar".into(),
            sender: sender.into(),
            sessions: store,
            ..Default::default()
        };
        let left = !ctx.sessions.lock().unwrap().is_empty();
        let r = (oktz_plugin_api::find("bataldaftar").unwrap().handler)(ctx);
        (r, left)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap().to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("bataldaftar").unwrap();
        assert_eq!(p.meta.name, "bataldaftar");
        assert_eq!(
            p.meta.alias,
            ["cancelreg", "canceldaftar", "regcancel"]
        );
        assert_eq!(p.meta.category, "user");
        assert_eq!(p.meta.description, "Batalkan sesi pendaftaran yang sedang aktif");
        assert_eq!(p.meta.usage, ".bataldaftar");
        assert_eq!(p.meta.example, ".bataldaftar");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(
            p.meta.skip_registration,
            "JS set skipRegistration: true — tanpa ini pluginnya tak terjangkau"
        );
    }

    #[test]
    fn all_three_aliases_resolve_back_to_this_plugin() {
        for a in ["cancelreg", "canceldaftar", "regcancel"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("bataldaftar")
            );
        }
    }

    #[test]
    fn without_a_session_it_says_there_is_none() {
        let (r, _) = run(serde_json::json!({}), "628111@s.whatsapp.net");
        assert_eq!(text_of(&r), NONE);
    }

    #[test]
    fn an_existing_session_is_cleared_and_confirmed() {
        let (r, had) = run(serde_json::json!({"628111": {"step": "name"}}), "628111@s.whatsapp.net");
        assert!(had, "sesi harus ada sebelum handler jalan");
        let t = text_of(&r);
        assert!(t.starts_with("✅ Sesi pendaftaran berhasil dibatalkan."), "{t}");
        assert!(t.contains("> Mulai lagi dengan: `.daftar`"), "{t}");
    }

    #[test]
    fn the_session_is_actually_removed_from_the_store() {
        let store = new_session_store();
        store
            .lock()
            .unwrap()
            .insert("628111".into(), serde_json::json!({"step": "name"}));
        let ctx = Ctx {
            prefix: ".".into(),
            command: "bataldaftar".into(),
            sender: "628111@s.whatsapp.net".into(),
            sessions: store.clone(),
            ..Default::default()
        };
        assert!(ctx.session_get("628111").is_some());
        (oktz_plugin_api::find("bataldaftar").unwrap().handler)(ctx);
        assert!(store.lock().unwrap().is_empty(), "sesi harus hilang");
    }

    #[test]
    fn a_session_belonging_to_someone_else_is_untouched() {
        let store = new_session_store();
        store.lock().unwrap().insert("628999".into(), serde_json::json!({"step": "name"}));
        let ctx = Ctx {
            prefix: ".".into(),
            command: "bataldaftar".into(),
            sender: "628111@s.whatsapp.net".into(),
            sessions: store.clone(),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("bataldaftar").unwrap().handler)(ctx);
        assert_eq!(text_of(&r), NONE);
        assert!(store.lock().unwrap().contains_key("628999"));
    }

    // `getRegistrationSessionKey` = `jid.replace(/[^0-9]/g, "")` Applied ke
    // JID PENUH, jadi `628111:5@s.whatsapp.net` → `6281115`, bukan `628111`.
    #[test]
    fn the_key_is_digits_only_so_a_device_suffix_shifts_it() {
        let clear = |store: SessionStore| {
            let ctx = Ctx {
                prefix: ".".into(),
                command: "bataldaftar".into(),
                sender: "628111:5@s.whatsapp.net".into(),
                sessions: store.clone(),
                ..Default::default()
            };
            text_of(&(oktz_plugin_api::find("bataldaftar").unwrap().handler)(ctx))
        };

        let store = new_session_store();
        store.lock().unwrap().insert("6281115".into(), serde_json::json!({"step": "age"}));
        assert!(
            clear(store).starts_with("✅ Sesi pendaftaran berhasil dibatalkan."),
            "kunci digit-only `6281115` harus ketemu"
        );

        let store = new_session_store();
        store
            .lock()
            .unwrap()
            .insert("628111".into(), serde_json::json!({"step": "age"}));
        assert_eq!(clear(store), NONE, "suffix device menggeser kunci, bukan diabaikan");
    }

    // `getRegistrationSessionEntry` (daftar.js:108) jatuh ke kunci legacy
    // berupa JID mentah apa adanya kalau kunci digit tidak ada.
    #[test]
    fn a_session_stored_under_the_raw_jid_is_still_found() {
        let store = new_session_store();
        store
            .lock()
            .unwrap()
            .insert("628111@s.whatsapp.net".into(), serde_json::json!({"step": "name"}));
        let ctx = Ctx {
            prefix: ".".into(),
            command: "bataldaftar".into(),
            sender: "628111@s.whatsapp.net".into(),
            sessions: store.clone(),
            ..Default::default()
        };
        assert!(
            text_of(&(oktz_plugin_api::find("bataldaftar").unwrap().handler)(ctx))
                .starts_with("✅ Sesi pendaftaran berhasil dibatalkan.")
        );
        assert!(store.lock().unwrap().is_empty());
    }
}
