//! `plugins/panel/rolemanager.js` — kelola owner/ceo/reseller per server.
//!
//! `pluginConfig.name` di JS adalah 45 command hasil loop
//! `ROLES × VALID_SERVERS × (add|del|list)`; `#[plugin]` hanya menerima satu
//! string, jadi command pertama jadi `name` dan 44 sisanya jadi alias — aturan
//! yang sama dipakai `fun/siapa.rs` untuk array `name` miliknya.
//!
//! `ponytail:` `src/lib/roles-cpanel.js` menyimpan tiap pasangan
//! role/server di file sendiri (`database/cpanel/<role>_<server>.json`).
//! `Mutation` hanya punya `SetSetting`, jadi semua daftar dipindah ke satu
//! setting `cpanelRoles` dengan kunci `"<role>_<server>"` → array nomor.
//! Pemetaan 1:1, hanya tempatnya yang berbeda.
//!
//! `ponytail:` `isLid(jid)` → `lidToJid(jid)` tidak di-port; `src/lib/lid.js`
//! butuh cache file `database/lid-cache.json` plus `jidDecode` dari `onigis`.
//! JID `@lid` karena itu diproses apa adanya: angka device-nya yang dipakai.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// `VALID_SERVERS` (`roles-cpanel.js:4`).
pub(crate) const SERVERS: [&str; 5] = ["v1", "v2", "v3", "v4", "v5"];
/// `VALID_ROLES` (`roles-cpanel.js:5`) — urutan ini menentukan role pertama
/// yang cocok di `getUserRole`.
pub(crate) const ROLES: [&str; 3] = ["owner", "ceo", "reseller"];

/// Bentuk `setting("cpanelRoles")`.
const STORE: &str = "cpanelRoles";

fn key(role: &str, server: &str) -> String {
    format!("{role}_{server}")
}

/// `(action, role, server)` dari `/^(add|del|list)(owner|ceo|reseller)(v[1-5])$/i`.
///
/// Ditulis sebagai parser, bukan `split`: `addownerv1x` harus gagal, bukan
/// diam-diam jadi prefix yang cocok. Nilai balik diambil dari konstanta,
/// bukan dari `c` lokal — `strip_prefix` meminjam `c`, jadi mengembalikannya
/// membuat nilai hidup-nya ikut menggantung.
fn parse_command(cmd: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let c = cmd.to_lowercase();
    for action in ["add", "del", "list"] {
        let Some(rest) = c.strip_prefix(action) else {
            continue;
        };
        for role in ROLES {
            // Dicari lewat `position` supaya yang dikembalikan `&'static str`
            // dari `SERVERS`, bukan `&str` yang dipinjam dari `c` lokal.
            let Some(pos) = rest
                .strip_prefix(role)
                .and_then(|server| SERVERS.iter().position(|s| *s == server))
            else {
                continue;
            };
            return Some((action, role, SERVERS[pos]));
        }
    }
    None
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// `cleanJid` + `getNumber` digabung: JS membentuk JID lengkap lalu memotong
/// bagian sebelum `@`. Hasilnya sama persis dengan memotong langsung.
fn number_of(jid: &str) -> Option<String> {
    let trimmed = jid.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(jid_number(trimmed).to_string())
}

fn jid_number(jid: &str) -> &str {
    crate::common::jid_number(jid)
}

fn list_for(ctx: &Ctx, role: &str, server: &str) -> Vec<String> {
    crate::common::string_list(ctx.setting(STORE).and_then(|v| v.get(key(role, server))))
}

fn role_of(ctx: &Ctx, number: &str, server: &str) -> Option<&'static str> {
    ROLES
        .into_iter()
        .find(|r| list_for(ctx, r, server).iter().any(|n| n == number))
}

fn can_manage(ctx: &Ctx, target_role: &str, server: &str) -> bool {
    if ctx.is_owner {
        return true;
    }
    let Some(user_role) = role_of(ctx, &number_of(&ctx.sender).unwrap_or_default(), server) else {
        return false;
    };
    if user_role == "reseller" || target_role == "owner" {
        return false;
    }
    true
}

fn set_list(ctx: &Ctx, role: &str, server: &str, list: Vec<String>) -> Mutation {
    let mut next = ctx.setting(STORE).cloned().unwrap_or_default();
    if !next.is_object() {
        next = serde_json::json!({});
    }
    next[key(role, server)] = serde_json::json!(list);
    Mutation::SetSetting {
        key: STORE.into(),
        value: next,
    }
}

fn list_reply(role: &str, server: &str, list: &[String]) -> String {
    let role_label = capitalize(role).to_uppercase();
    let server_label = server.to_uppercase();
    if list.is_empty() {
        return format!(
            "📋 *ᴅᴀꜰᴛᴀʀ {role_label} {server_label}*\n\n> Belum ada {role} terdaftar."
        );
    }
    let mut txt = format!(
        "📋 *ᴅᴀꜰᴛᴀʀ {role_label} {server_label}*\n\n> Total: *{}* {role}\n\n",
        list.len()
    );
    for (i, num) in list.iter().enumerate() {
        txt.push_str(&format!("{}. `{num}`\n", i + 1));
    }
    txt.push_str(&format!(
        "\n> _Role: {} | Server: {server_label}_",
        capitalize(role)
    ));
    txt
}

#[plugin(
    name = "addownerv1",
    alias = [
        "delownerv1", "listownerv1",
        "addownerv2", "delownerv2", "listownerv2",
        "addownerv3", "delownerv3", "listownerv3",
        "addownerv4", "delownerv4", "listownerv4",
        "addownerv5", "delownerv5", "listownerv5",
        "addceov1", "delceov1", "listceov1",
        "addceov2", "delceov2", "listceov2",
        "addceov3", "delceov3", "listceov3",
        "addceov4", "delceov4", "listceov4",
        "addceov5", "delceov5", "listceov5",
        "addresellerv1", "delresellerv1", "listresellerv1",
        "addresellerv2", "delresellerv2", "listresellerv2",
        "addresellerv3", "delresellerv3", "listresellerv3",
        "addresellerv4", "delresellerv4", "listresellerv4",
        "addresellerv5", "delresellerv5", "listresellerv5"
    ],
    category = "panel",
    description = "Kelola owner/ceo/reseller per server",
    usage = ".addownerv1 @user atau .listceov2",
    example = ".addresellerv1 @user",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn rolemanager(ctx: Ctx) -> Reply {
    let Some((action, role, server)) = parse_command(&ctx.command) else {
        return Reply::text("❌ Command tidak valid.");
    };

    let role_label = capitalize(role);
    let server_label = server.to_uppercase();

    if action == "list" {
        return Reply::text(list_reply(role, server, &list_for(&ctx, role, server)));
    }

    if !can_manage(&ctx, role, server) {
        let user_role = number_of(&ctx.sender)
            .and_then(|n| role_of(&ctx, &n, server))
            .map(capitalize);
        return Reply::text(format!(
            "❌ *ᴀᴋsᴇs ᴅɪᴛᴏʟᴀᴋ*\n\n\
             > Kamu tidak bisa mengelola *{role_label}* di *{server_label}*\n\
             > Role kamu: *{}*\n\n\
             > Hirarki: Owner > CEO > Reseller",
            user_role.unwrap_or_else(|| "Tidak ada".into())
        ));
    }

    // Urutan target: reply, lalu mention, lalu angka di teks — sama seperti JS.
    let target = ctx
        .quoted()
        .and_then(|q| number_of(&q.sender))
        .or_else(|| ctx.mentioned.first().and_then(|j| number_of(j)))
        .or_else(|| {
            let digits: String = ctx.text.trim().chars().filter(char::is_ascii_digit).collect();
            (!digits.is_empty()).then_some(digits)
        });

    let Some(target) = target else {
        return Reply::text(format!(
            "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
             > `{p}{c} @user`\n\
             > `{p}{c} 628xxx`\n\
             > Reply pesan user",
            p = ctx.prefix,
            c = ctx.command
        ));
    };

    let mut list = list_for(&ctx, role, server);

    if action == "add" {
        // `addRole(targetUser, server, role)` — `getUserRole` di JS (roles-cpanel.js:83)
        // dipanggil atas TARGET, bukan atas pengirim.
        if let Some(existing) = role_of(&ctx, &target, server) {
            return Reply::text(format!(
                "❌ *ɢᴀɢᴀʟ*\n\n> Sudah memiliki role {existing} di {server_label}"
            ));
        }
        if list.contains(&target) {
            return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Sudah terdaftar");
        }
        list.push(target.clone());
        let total = list.len();
        return Reply::text(format!(
            "✅ *{} ᴅɪᴛᴀᴍʙᴀʜᴋᴀɴ*\n\n\
             ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
             ┃ 📱 ɴᴏᴍᴏʀ: `{target}`\n\
             ┃ 🏷️ ʀᴏʟᴇ: `{role_label}`\n\
             ┃ 🖥️ sᴇʀᴠᴇʀ: `{server_label}`\n\
             ┃ 📊 ᴛᴏᴛᴀʟ: `{total}` {role}\n\
             ╰┈┈⬡",
            role_label.to_uppercase()
        ))
        .with_reaction("✅")
        .with_mutation(set_list(&ctx, role, server, list));
    }

    // `del`
    let Some(pos) = list.iter().position(|n| *n == target) else {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Tidak terdaftar");
    };
    list.remove(pos);
    let total = list.len();
    Reply::text(format!(
        "✅ *{} ᴅɪʜᴀᴘᴜs*\n\n\
         > Nomor: `{target}`\n\
         > Server: *{server_label}*\n\
         > Total: *{total}* {role}",
        role_label.to_uppercase()
    ))
    .with_reaction("✅")
    .with_mutation(set_list(&ctx, role, server, list))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn ctx_of(command: &str, settings: Value, sender: &str, is_owner: bool) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: command.into(),
            sender: sender.into(),
            text: String::new(),
            settings,
            is_owner,
            ..Default::default()
        }
    }

    fn run(command: &str, settings: Value, sender: &str, is_owner: bool) -> Reply {
        (oktz_plugin_api::find(command).unwrap().handler)(ctx_of(
            command, settings, sender, is_owner,
        ))
    }

    fn body(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap().to_string()
    }

    fn store_after(r: &Reply) -> Vec<String> {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, STORE);
                crate::common::string_list(value.get("ceo_v2"))
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("addownerv1").unwrap();
        assert_eq!(p.meta.name, "addownerv1");
        assert_eq!(p.meta.category, "panel");
        assert_eq!(p.meta.description, "Kelola owner/ceo/reseller per server");
        assert_eq!(p.meta.usage, ".addownerv1 @user atau .listceov2");
        assert_eq!(p.meta.example, ".addresellerv1 @user");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn all_forty_five_generated_commands_resolve() {
        // 3 role x 5 server x 3 aksi — angka ini berasal dari loop di JS.
        assert_eq!(ROLES.len() * SERVERS.len() * 3, 45);
        let mut seen = 0;
        for role in ROLES {
            for server in SERVERS {
                for action in ["add", "del", "list"] {
                    let cmd = format!("{action}{role}{server}");
                    assert_eq!(
                        parse_command(&cmd),
                        Some((action, role, server)),
                        "{cmd} harus bisa diparse"
                    );
                    assert!(
                        oktz_plugin_api::find(&cmd).is_some(),
                        "{cmd} tidak terdaftar"
                    );
                    seen += 1;
                }
            }
        }
        assert_eq!(seen, 45);
    }

    #[test]
    fn a_command_outside_the_pattern_is_refused() {
        // Prefix yang cocok tidak cukup: sisanya harus persis.
        for cmd in ["addownerv6", "addbosserv1", "addownerv", "listownerv10"] {
            assert_eq!(parse_command(cmd), None, "{cmd}");
        }
    }

    #[test]
    fn an_empty_role_list_says_nothing_is_registered() {
        let r = run("listceov2", serde_json::json!({}), "628111@s.whatsapp.net", false);
        assert!(
            body(&r).starts_with("📋 *ᴅᴀꜰᴛᴀʀ CEO V2*\n\n> Belum ada ceo terdaftar."),
            "{}",
            body(&r)
        );
        assert!(r.mutations().is_empty(), "list tidak menulis apa pun");
    }

    #[test]
    fn a_populated_role_list_is_numbered_with_the_total() {
        let r = run(
            "listceov2",
            serde_json::json!({"cpanelRoles": {"ceo_v2": ["628111", "628222"]}}),
            "628999@s.whatsapp.net",
            false,
        );
        let t = body(&r);
        assert!(t.starts_with("📋 *ᴅᴀꜰᴛᴀʀ CEO V2*"), "{t}");
        assert!(t.contains("> Total: *2* ceo"), "{t}");
        assert!(t.contains("1. `628111`\n2. `628222`"), "{t}");
        assert!(t.contains("_Role: Ceo | Server: V2_"), "{t}");
    }

    #[test]
    fn an_owner_without_a_target_gets_the_usage_card() {
        let r = run("addownerv1", serde_json::json!({}), "628111@s.whatsapp.net", true);
        // Owner tetap butuh target: guard `!targetUser` di JS jalan untuk semua.
        let t = body(&r);
        assert!(t.starts_with("⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*"), "{t}");
        assert!(t.contains("> `.addownerv1 @user`"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_plain_number_argument_is_accepted_as_the_target() {
        let mut ctx = ctx_of("addownerv1", serde_json::json!({}), "628111@s.whatsapp.net", true);
        ctx.text = "628555".into();
        let r = (oktz_plugin_api::find("addownerv1").unwrap().handler)(ctx);
        let t = body(&r);
        assert!(t.starts_with("✅ *OWNER ᴅɪᴛᴀᴍʙᴀʜᴋᴀɴ*"), "{t}");
        assert!(t.contains("┃ 📱 ɴᴏᴍᴏʀ: `628555`"), "{t}");
        assert!(t.contains("┃ 🏷️ ʀᴏʟᴇ: `Owner`"), "{t}");
        assert!(t.contains("┃ 🖥️ sᴇʀᴠᴇʀ: `V1`"), "{t}");
        assert!(t.contains("┃ 📊 ᴛᴏᴛᴀʟ: `1` owner"), "{t}");
        match &r.mutations()[0] {
            Mutation::SetSetting { value, .. } => assert_eq!(
                crate::common::string_list(value.get("owner_v1")),
                ["628555"]
            ),
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn a_mention_is_the_target_when_nothing_is_quoted() {
        let mut ctx = ctx_of("addceov2", serde_json::json!({}), "628111@s.whatsapp.net", true);
        ctx.mentioned = vec!["628777@s.whatsapp.net".into()];
        let r = (oktz_plugin_api::find("addceov2").unwrap().handler)(ctx);
        assert!(body(&r).contains("ɴᴏᴍᴏʀ: `628777`"), "{}", body(&r));
    }

    #[test]
    fn a_manager_cannot_touch_owner_role() {
        let r = run(
            "addownerv1",
            serde_json::json!({"cpanelRoles": {"ceo_v1": ["628111"]}}),
            "628111@s.whatsapp.net",
            false,
        );
        let t = body(&r);
        assert!(t.starts_with("❌ *ᴀᴋsᴇs ᴅɪᴛᴏʟᴀᴋ*"), "{t}");
        assert!(t.contains("Role kamu: *Ceo*"), "{t}");
        assert!(t.contains("> Hirarki: Owner > CEO > Reseller"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_reseller_and_a_stranger_both_have_no_managing_power() {
        for (settings, sender, expected) in [
            (serde_json::json!({"cpanelRoles": {"reseller_v1": ["628111"]}}), "628111@s.whatsapp.net", "*Reseller*"),
            (serde_json::json!({}), "628111@s.whatsapp.net", "*Tidak ada*"),
        ] {
            let r = run("addceov1", settings, sender, false);
            let t = body(&r);
            assert!(t.starts_with("❌ *ᴀᴋsᴇs ᴅɪᴛᴏʟᴀᴋ*"), "{t}");
            assert!(t.contains(&format!("Role kamu: {expected}")), "{t}");
            assert!(r.mutations().is_empty(), "{t}");
        }
    }

    // `canManageRole` (roles-cpanel.js:149) menolak HANYA role `owner` sebagai
    // target; `ceo` boleh menambah `ceo`. Yang ditolak cuma reseller/tanpa role
    // (lihat test sebelumnya) dan target `owner` (lihat `a_manager_cannot_touch_owner_role`).
    #[test]
    fn a_ceo_can_manage_any_non_owner_role_on_its_own_server() {
        let settings = serde_json::json!({"cpanelRoles": {"ceo_v1": ["628111"], "ceo_v2": ["628111"]}});
        for cmd in ["addresellerv1", "addceov1"] {
            let mut ctx = ctx_of(cmd, settings.clone(), "628111@s.whatsapp.net", false);
            ctx.text = "628555".into();
            let r = (oktz_plugin_api::find(cmd).unwrap().handler)(ctx);
            assert!(body(&r).starts_with("✅"), "{cmd}: {}", body(&r));
            assert!(body(&r).contains("ɴᴏᴍᴏʀ: `628555`"), "{cmd}: {}", body(&r));
        }
    }

    #[test]
    fn adding_a_number_that_already_has_a_role_on_that_server_is_refused() {
        let mut ctx = ctx_of(
            "addceov2",
            serde_json::json!({"cpanelRoles": {"owner_v2": ["628111"]}}),
            "628999@s.whatsapp.net",
            true,
        );
        ctx.text = "628111".into();
        let r = (oktz_plugin_api::find("addceov2").unwrap().handler)(ctx);
        let t = body(&r);
        assert!(t.starts_with("❌ *ɢᴀɢᴀʟ*"), "{t}");
        assert!(t.contains("Sudah memiliki role owner di V2"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn deleting_a_registered_number_removes_it_from_the_list() {
        let settings = serde_json::json!({"cpanelRoles": {"ceo_v2": ["628111", "628222"]}});
        let mut ctx = ctx_of("delceov2", settings, "628111@s.whatsapp.net", true);
        ctx.text = "628111".into();
        let r = (oktz_plugin_api::find("delceov2").unwrap().handler)(ctx);
        let t = body(&r);
        assert!(t.starts_with("✅ *CEO ᴅɪʜᴀᴘᴜs*"), "{t}");
        assert!(t.contains("> Nomor: `628111`"), "{t}");
        assert!(t.contains("> Total: *1* ceo"), "{t}");
        assert_eq!(store_after(&r), ["628222"]);
    }

    #[test]
    fn deleting_an_unregistered_number_says_so() {
        let mut ctx = ctx_of(
            "delceov2",
            serde_json::json!({"cpanelRoles": {"ceo_v2": ["628111"]}}),
            "628111@s.whatsapp.net",
            true,
        );
        ctx.text = "628999".into();
        let r = (oktz_plugin_api::find("delceov2").unwrap().handler)(ctx);
        assert_eq!(body(&r), "❌ *ɢᴀɢᴀʟ*\n\n> Tidak terdaftar");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn roles_on_other_servers_do_not_leak_into_the_check() {
        // CEO di V1 tidak berarti boleh menambah reseller di V2.
        let r = run(
            "addresellerv2",
            serde_json::json!({"cpanelRoles": {"ceo_v1": ["628111"]}}),
            "628111@s.whatsapp.net",
            false,
        );
        assert!(body(&r).starts_with("❌ *ᴀᴋsᴇs ᴅɪᴛᴏʟᴀᴋ*"), "{}", body(&r));
    }
}
