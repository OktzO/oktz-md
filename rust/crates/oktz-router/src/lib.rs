//! Router: pipeline pesan dari parse command sampai dispatch plugin.
//!
//! Port dari `src/handler.js` (2404 baris) dan `src/lib/middleware.js`.
//! Scope saat ini HANYA logika murni yang bisa diuji tanpa koneksi WA:
//! parsing command, permission, mode. Gate yang butuh I/O (dedup 100ms,
//! rate limiter, energi, group protection) TIDAK di sini — lihat README crate
//! dan `TODO` di tiap modul. Menambahkannya tanpa bukti hanya menambah
//! kode mati.
//!
//! Yang TIDAK bisa dilemarkan: urutan. `check_mode` jalan sebelum
//! `check_permission`, dan keduanya sebelum lookup plugin — sama seperti
//! handler.js:879 → 1709 → 1810 → 1896.

pub mod command;
pub mod mode;
pub mod permission;

pub use command::{ParseOptions, ParsedCommand, parse_command};
pub use mode::{BotMode, DenyMode, GroupBotMode, GroupOverrides, ModeSettings};
pub use permission::{
    AccessGrant, Caller, DenyReason, GroupState, PermissionInput, UserState,
};

use oktz_plugin_api::{Ctx, PluginEntry, Reply};

/// Keputusan router untuk satu pesan — router yang memutuskan; ini yang
/// differentiate "diam" (drop) dari "reply".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Bukan command — biarkan pipeline lain (smart trigger, game) yang
    /// mencoba. Router tidak balas apa pun.
    NotACommand,
    /// Command tapi tidak ada plugin yang cocok — senyap, seperti JS
    /// (`getPlugin` miss → `return` tanpa reply).
    NoPlugin,
    /// Ditolak permission — router kirim `reason`.
    DeniedPermission(String),
    /// Ditolak mode — `DenyMode::Afk`/`OnlyThisGroup` reply, `Silent` drop.
    DeniedMode(DenyMode),
    /// Ditolak botMode per-grup — router kirim pesan saran mode.
    DeniedGroupMode(String),
    /// Plugin jalan, mengembalikan balasan.
    Handled(Reply),
}

/// Semua yang router butuhkan untuk memutuskan. Trait ini exist supaya test
/// bisa pakai stub tanpa DB.
pub trait RouterContext {
    fn user_state(&self, sender: &str) -> UserState;
    fn group_state(&self, chat: &str) -> Option<GroupState>;
    fn group_bot_mode(&self, chat: &str) -> String;
    fn mode_settings(&self) -> ModeSettings;
    fn group_overrides(&self, chat: &str) -> Option<GroupOverrides>;
    fn capprem(&self, name: &str) -> Option<bool>;
    fn uptime_secs(&self) -> u64;
}

/// Config parse + dispatch.
pub struct Router<'a, C: RouterContext> {
    ctx: &'a C,
    parse_opts: ParseOptions,
    now_ms: i64,
}

impl<'a, C: RouterContext> Router<'a, C> {
    pub fn new(ctx: &'a C, parse_opts: ParseOptions, now_ms: i64) -> Self {
        Self {
            ctx,
            parse_opts,
            now_ms,
        }
    }

    /// Pipeline lengkap: parse → mode → lookup → permission → group mode → dispatch.
    pub fn route(
        &self,
        caller: &mut Caller,
        body: &str,
        plugin_lookup: impl Fn(&str) -> Option<&'static PluginEntry>,
    ) -> Decision {
        let parsed = parse_command(body, &self.parse_opts);
        if !parsed.is_command {
            return Decision::NotACommand;
        }
        caller.command = parsed.command.clone();

        // 1. Mode global (handler.js:879) — SEBELUM permission.
        let overrides = if caller.is_group {
            self.ctx.group_overrides(&caller.chat)
        } else {
            None
        };
        if let Err(reason) = mode::check_mode(caller, &self.ctx.mode_settings(), overrides) {
            return Decision::DeniedMode(reason);
        }

        // 2. Lookup plugin.
        let Some(entry) = plugin_lookup(&parsed.command) else {
            return Decision::NoPlugin;
        };

        let user = self.ctx.user_state(&caller.sender);

        // 3. Permission (handler.js:1810).
        let input = permission::PermissionInput {
            meta: &entry.meta,
            now_ms: self.now_ms,
            capprem: self.ctx.capprem(&entry.meta.name),
            group: if caller.is_group {
                self.ctx.group_state(&caller.chat)
            } else {
                None
            },
        };
        if let Err(reason) = permission::check(caller, &user, &input) {
            return Decision::DeniedPermission(reason.message().to_string());
        }

        // 4. botMode per-grup (handler.js:1709). Non-group: skip.
        if caller.is_group && !caller.is_owner && !caller.is_admin {
            let bot_mode = self.ctx.group_bot_mode(&caller.chat);
            if mode::group_feature_mode(caller, &entry.meta, None, &bot_mode, false).is_err() {
                let gm = mode::GroupBotMode::parse(&bot_mode);
                let msg = mode::group_mode_denied_message(
                    &gm,
                    &entry.meta.category.to_lowercase(),
                    &parsed.prefix,
                    &parsed.command,
                );
                return Decision::DeniedGroupMode(msg);
            }
        }

        // 5. Dispatch.
        let c = Ctx {
            text: parsed.text.clone(),
            args: parsed.args.clone(),
            sender: caller.sender.clone(),
            chat: caller.chat.clone(),
            mentioned: Vec::new(),
            command: parsed.command.clone(),
            prefix: parsed.prefix.clone(),
            is_group: caller.is_group,
            is_owner: caller.is_owner,
            uptime_secs: self.ctx.uptime_secs(),
        };
        Decision::Handled((entry.handler)(c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::PluginMeta;
    use std::sync::Arc;

    struct Stub {
        user: UserState,
        group: Option<GroupState>,
        mode: ModeSettings,
    }

    impl RouterContext for Stub {
        fn user_state(&self, _s: &str) -> UserState {
            self.user.clone()
        }
        fn group_state(&self, _c: &str) -> Option<GroupState> {
            self.group
        }
        fn group_bot_mode(&self, _c: &str) -> String {
            "all".into()
        }
        fn mode_settings(&self) -> ModeSettings {
            self.mode.clone()
        }
        fn group_overrides(&self, _c: &str) -> Option<GroupOverrides> {
            None
        }
        fn capprem(&self, _n: &str) -> Option<bool> {
            None
        }
        fn uptime_secs(&self) -> u64 {
            42
        }
    }

    fn stub() -> Stub {
        Stub {
            user: UserState::default(),
            group: None,
            mode: ModeSettings {
                mode: "public".into(),
                ..Default::default()
            },
        }
    }

    fn entry(name: &str, category: &str, reply: Reply) -> PluginEntry {
        PluginEntry {
            meta: PluginMeta {
                name: name.into(),
                category: category.into(),
                ..Default::default()
            },
            handler: Arc::new(move |_| reply.clone()),
        }
    }

    fn caller() -> Caller {
        Caller {
            sender: "628123@s.whatsapp.net".into(),
            chat: "628123@s.whatsapp.net".into(),
            ..Default::default()
        }
    }

    #[test]
    fn non_command_returns_not_a_command() {
        let s = stub();
        let r = Router::new(&s, ParseOptions::default(), 1000);
        let mut c = caller();
        let d = r.route(&mut c, "halo", |_| None);
        assert_eq!(d, Decision::NotACommand);
    }

    #[test]
    fn command_without_plugin_is_silent() {
        let s = stub();
        let r = Router::new(&s, ParseOptions::default(), 1000);
        let mut c = caller();
        let d = r.route(&mut c, ".tidakada", |_| None);
        assert_eq!(d, Decision::NoPlugin);
    }

    #[test]
    fn plugin_runs_and_returns_reply() {
        let s = stub();
        let r = Router::new(&s, ParseOptions::default(), 1000);
        let mut c = caller();
        let e = Box::leak(Box::new(entry("ping", "main", Reply::text("pong"))));
        let d = r.route(&mut c, ".ping", |_| Some(e));
        assert_eq!(d, Decision::Handled(Reply::text("pong")));
    }

    #[test]
    fn mode_denied_before_plugin_lookup() {
        // Mode self, caller bukan owner: embora plugin ada, mode menolak dulu.
        let s = Stub {
            mode: ModeSettings {
                mode: "self".into(),
                ..Default::default()
            },
            ..stub()
        };
        let r = Router::new(&s, ParseOptions::default(), 1000);
        let mut c = caller();
        let d = r.route(&mut c, ".ping", |_| None);
        assert_eq!(d, Decision::DeniedMode(DenyMode::Silent));
    }

    #[test]
    fn permission_denied_returns_message() {
        let s = stub();
        let r = Router::new(&s, ParseOptions::default(), 1000);
        let mut c = caller();
        let owner_only = {
            let m = PluginMeta {
                name: "secret".into(),
                is_owner: true,
                ..Default::default()
            };
            Box::leak(Box::new(PluginEntry {
                meta: m,
                handler: Arc::new(|_| Reply::text("x")),
            }))
        };
        let d = r.route(&mut c, ".secret", |_| Some(owner_only));
        assert!(matches!(d, Decision::DeniedPermission(msg) if msg.contains("Owner only")));
    }

    #[test]
    fn ctx_carries_parsed_fields_to_handler() {
        let s = stub();
        let r = Router::new(&s, ParseOptions::default(), 1000);
        let mut c = caller();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(None));
        let sink = seen.clone();
        let e = Box::leak(Box::new(PluginEntry {
            meta: PluginMeta {
                name: "echo".into(),
                category: "tools".into(),
                ..Default::default()
            },
            handler: Arc::new(move |c: Ctx| {
                *sink.lock().unwrap() = Some(c);
                Reply::None
            }),
        }));
        r.route(&mut c, ".echo satu dua", |_| Some(e));
        let got = seen.lock().unwrap().clone().expect("handler harus jalan");
        assert_eq!(got.command, "echo");
        assert_eq!(got.args, vec!["satu", "dua"]);
        assert_eq!(got.text, "satu dua");
        assert_eq!(got.uptime_secs, 42);
    }
}