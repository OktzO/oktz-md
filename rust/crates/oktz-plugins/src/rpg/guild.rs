//! `plugins/rpg/guild.js` — buat/gabung/keluar guild, donasi kas.
//!
//! `ponytail:` tabel guild ada di `db.db.data.guilds` — key TOP-LEVEL store,
//! bukan `settings` dan bukan milik user. `Mutation` hanya punya SetUser /
//! SetGroup / SetSetting, jadi di sini guild disimpan sebagai setting
//! `guilds` (`ctx.setting("guilds")` untuk baca). Perilaku plugin tetap sama
//! karena hanya satu writer.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value, json};

use crate::common::jid_number;
use crate::rpg::heal::rpg_map;
use crate::tools::kalkulatormbg::thousands_id;

const CREATE_COST: i64 = 10_000;
const MAX_MEMBERS: usize = 50;
const GUILDS_KEY: &str = "guilds";

fn guilds(ctx: &Ctx) -> Map<String, Value> {
    ctx.setting(GUILDS_KEY)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn guild_id(rpg: &Map<String, Value>) -> Option<String> {
    rpg.get("guildId")
        .filter(|v| crate::common::js_truthy(Some(v)))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn member_list(guild: &Value) -> Vec<String> {
    guild
        .get("members")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default()
}

fn treasury(guild: &Value) -> i64 {
    guild.get("treasury").and_then(Value::as_i64).unwrap_or(0)
}

fn level_of(guild: &Value) -> i64 {
    or(guild.get("level").and_then(Value::as_i64), 1)
}

fn or(v: Option<i64>, d: i64) -> i64 {
    match v {
        Some(x) if x != 0 => x,
        _ => d,
    }
}

/// `Object.values(guilds).find(g => g.name.toLowerCase() === name.toLowerCase())`.
fn find_named(guilds: &Map<String, Value>, name: &str) -> Option<(String, Value)> {
    guilds
        .iter()
        .find(|(_, g)| {
            g.get("name")
                .and_then(Value::as_str)
                .is_some_and(|n| n.to_lowercase() == name.to_lowercase())
        })
        .map(|(k, v)| (k.clone(), v.clone()))
}

/// `args.slice(1).join(" ")` — nama guild boleh berisi spasi.
fn guild_name(ctx: &Ctx) -> String {
    ctx.args.iter().skip(1).cloned().collect::<Vec<_>>().join(" ")
}

fn save(guilds: Map<String, Value>) -> Mutation {
    Mutation::SetSetting {
        key: GUILDS_KEY.to_string(),
        value: Value::Object(guilds),
    }
}

#[plugin(
    name = "guild",
    alias = ["clan", "team", "kelompok"],
    category = "rpg",
    description = "Sistem guild/clan",
    usage = ".guild <create/join/leave/info>",
    example = ".guild create DragonSlayers",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn guild(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let mut all = guilds(&ctx);
    let action = crate::common::first_keyword(&ctx);
    let name = guild_name(&ctx);
    let my_id = guild_id(&rpg);

    let known = matches!(
        action.as_deref(),
        Some("create" | "join" | "leave" | "info" | "list" | "members" | "deposit")
    );
    if !known {
        let p = &ctx.prefix;
        let status = match &my_id {
            Some(id) => format!(
                "📌 Status: Tergabung di *{}*",
                all.get(id)
                    .and_then(|g| g.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("Unknown")
            ),
            None => "📌 Status: *Jomblo Guild (Kagak Punya Temen)*".to_string(),
        };
        return Reply::text(format!(
            "🏰 *SERIKAT GUILD RPG* 🏰\n\n\
             Bangun atau gabung ke serikat buat dapet *benefit* bareng temen-temen lu!\n\n\
             *Daftar Perintah:*\n\
             🗡️ `{p}guild create <nama>` (Bikin Guild)\n\
             🛡️ `{p}guild join <nama>` (Gabung Guild)\n\
             🏃 `{p}guild leave` (Keluar Guild)\n\
             📜 `{p}guild info` (Cek Stats Guild)\n\
             👥 `{p}guild members` (Cek Anggota)\n\
             💰 `{p}guild deposit <amount>` (Donasi Kas)\n\
             🏆 `{p}guild list` (Top Guilds)\n\n\
             {status}"
        ));
    }

    match action.as_deref().unwrap_or_default() {
        "list" => {
            let list: Vec<&Value> = all.values().collect();
            if list.is_empty() {
                return Reply::text(format!(
                    "Belum ada guild di server ini! Bikin dong pake `{}guild create <nama>`",
                    ctx.prefix
                ));
            }
            let mut txt = String::from("🏆 *DAFTAR TOP GUILD* 🏆\n\n");
            for g in list.iter().take(10) {
                txt.push_str(&format!(
                    "🏰 *{}* (Lv. {})\n👥 Member: {}/50\n💰 Kas Kasir: Rp {}\n──────────────\n",
                    g.get("name").and_then(Value::as_str).unwrap_or("?"),
                    level_of(g),
                    member_list(g).len(),
                    thousands_id(treasury(g) as f64)
                ));
            }
            Reply::text(txt)
        }

        "create" => {
            if my_id.is_some() {
                return Reply::text("Rakus lu! Kan udah punya guild. Leave dulu sana kalau mau bikin baru!");
            }
            // `guildName.length` di JS = jumlah UTF-16 unit, bukan byte.
            let len = name.chars().count() as i64;
            if name.is_empty() || len < 3 {
                return Reply::text("Nama guild minimal *3 huruf* bos!");
            }
            if len > 20 {
                return Reply::text("Nama guild kepanjangan, maksimal *20 huruf* aja!");
            }
            if find_named(&all, &name).is_some() {
                return Reply::text(format!(
                    "Yahh, nama *{name}* udah dipake kelompok lain! Cari nama yang lebih keren!"
                ));
            }
            let koin = crate::common::int_field(&ctx, "koin");
            if koin < CREATE_COST {
                return Reply::text(
                    "Miskin amat mau jadi ketua? Butuh *Rp 10.000* buat biaya administrasi pendaftaran Guild!",
                );
            }

            let id = format!("guild_{}", crate::common::now_ms());
            all.insert(
                id.clone(),
                json!({ "id": id, "name": name, "leader": ctx.sender,
                       "members": [ctx.sender.clone()], "treasury": 0, "level": 1,
                       "exp": 0, "createdAt": crate::common::now_ms() }),
            );
            rpg.insert("guildId".into(), json!(id));

            Reply::Text {
                text: format!(
                    "🎉 *GUILD RESMI BERDIRI!* 🎉\n\n\
                     Papan nama *{name}* telah dipasang di markas baru!\n\n\
                     👑 Ketua: @{}\n\
                     💸 Biaya Bangunan: *-Rp {}*\n\n\
                     > _Ajak temen-temen lu buat gabung pake `.guild join {name}`!_",
                    jid_number(&ctx.sender),
                    thousands_id(CREATE_COST as f64)
                ),
                mentions: vec![ctx.sender.clone()],
            }
            .mutating(vec![
                save(all),
                Mutation::SetUser {
                    jid: ctx.sender.clone(),
                    data: json!({ "koin": koin - CREATE_COST, "rpg": rpg }),
                },
            ])
        }

        "join" => {
            if my_id.is_some() {
                return Reply::text("Lu udah punya kelompok bro! Nggak bisa *double agent* di sini.");
            }
            if name.is_empty() {
                return Reply::text(format!(
                    "Tulis nama guild yang mau dimasukin!\nContoh: `{}guild join DragonSlayers`",
                    ctx.prefix
                ));
            }
            let Some((id, mut target)) = find_named(&all, &name) else {
                return Reply::text(format!("Guild *{name}* nggak ketemu! Typo kali lu?"));
            };
            let mut members = member_list(&target);
            if members.len() >= MAX_MEMBERS {
                let target_name = target
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(name.as_str())
                    .to_string();
                return Reply::text(format!(
                    "Maaf bang, kapasitas markas guild *{target_name}* udah full (50/50)!"
                ));
            }
            members.push(ctx.sender.clone());
            if let Some(obj) = target.as_object_mut() {
                obj.insert("members".into(), json!(members));
            }
            all.insert(id.clone(), target);
            // JS: `user.rpg.guildId = targetGuild.id`, bukan nama yang diketik.
            rpg.insert("guildId".into(), json!(id));

            let target_name = all
                .values()
                .find(|g| g.get("name").and_then(Value::as_str) == Some(name.as_str()))
                .and_then(|g| g.get("name"))
                .and_then(Value::as_str)
                .unwrap_or(&name)
                .to_string();
            Reply::text(format!(
                "✅ Selamat datang di barak! Lu sekarang resmi jadi anggota guild *{target_name}*! ⚔️"
            ))
            .mutating(vec![save(all), Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: json!({ "rpg": rpg }),
            }])
        }

        "leave" => {
            let Some(id) = my_id else {
                return Reply::text("Lu aja belum masuk guild mana-mana, mau leave darimana coba? 😂");
            };
            let Some(mut current) = all.get(&id).cloned() else {
                rpg.insert("guildId".into(), Value::Null);
                return Reply::text(
                    "Guild lu kayaknya udah bubar atau di-*delete*. Data udah di-reset.",
                )
                .with_mutation(Mutation::SetUser {
                    jid: ctx.sender.clone(),
                    data: json!({ "rpg": rpg }),
                });
            };

            let mut members = member_list(&current);
            let leader = current.get("leader").and_then(Value::as_str);
            if leader == Some(ctx.sender.as_str()) && members.len() > 1 {
                return Reply::text(
                    "Woy ketua! Masa mau ninggalin anggota gitu aja? Transfer kepemimpinan dulu ke member lain atau kick semua anggotanya! 😡",
                );
            }
            members.retain(|m| m != &ctx.sender);
            let guild_name = current
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("Unknown")
                .to_string();
            if let Some(obj) = current.as_object_mut() {
                obj.insert("members".into(), json!(members));
            }
            if member_list(&current).is_empty() {
                all.remove(&id);
            } else {
                all.insert(id, current);
            }
            rpg.insert("guildId".into(), Value::Null);

            Reply::text(format!(
                "🏃 Lu cabut dari markas *{guild_name}* dan kembali jadi ronin tanpa tuan!"
            ))
            .mutating(vec![save(all), Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: json!({ "rpg": rpg }),
            }])
        }

        "info" => {
            let Some(id) = my_id else {
                return Reply::text("Lu nggak punya guild bos! Cari temen sana!");
            };
            let Some(current) = all.get(&id) else {
                return Reply::text("Guild tidak ditemukan!");
            };
            let leader = current.get("leader").and_then(Value::as_str).unwrap_or("");
            Reply::Text {
                text: format!(
                    "🏰 *PAPAN INFO GUILD* 🏰\n\n\
                     👑 Nama: *{}*\n\
                     👤 Leader: @{}\n\
                     📊 Level: *{}*\n\
                     👥 Anggota: *{}/50*\n\
                     💰 Uang Kas: *Rp {}*",
                    current.get("name").and_then(Value::as_str).unwrap_or("?"),
                    jid_number(leader),
                    level_of(current),
                    member_list(current).len(),
                    thousands_id(treasury(current) as f64)
                ),
                mentions: if leader.is_empty() { vec![] } else { vec![leader.to_string()] },
            }
        }

        "members" => {
            let Some(id) = my_id else {
                return Reply::text("Cieee nggak punya guild...");
            };
            let Some(current) = all.get(&id) else {
                return Reply::text("Guild tidak ditemukan!");
            };
            let leader = current.get("leader").and_then(Value::as_str).unwrap_or("");
            let guild_name = current
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("?")
                .to_string();
            let members = member_list(current);
            let rows: Vec<String> = members
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    format!(
                        "{}. @{}{}",
                        i + 1,
                        jid_number(m),
                        if m == leader { " 👑" } else { " 🗡️" }
                    )
                })
                .collect();
            Reply::Text {
                text: format!("👥 *DAFTAR ANGGOTA {guild_name}*\n\n{}", rows.join("\n")),
                mentions: members,
            }
        }

        _ => {
            let Some(id) = my_id else {
                return Reply::text("Lu mau donasi ke panti asuhan mana? Lu aja nggak punya guild!");
            };
            let Some(mut current) = all.get(&id).cloned() else {
                return Reply::text("Guild tidak ditemukan!");
            };
            let amount = ctx.args.get(1).and_then(|a| crate::common::parse_int(a)).unwrap_or(0);
            if amount < 100 {
                return Reply::text("Pelit amat! Minimal donasi kas *Rp 100* lah!");
            }
            let koin = crate::common::int_field(&ctx, "koin");
            if koin < amount {
                return Reply::text("Duit lu kurang bro buat donasi segitu!");
            }

            let new_treasury = treasury(&current) + amount;
            if let Some(obj) = current.as_object_mut() {
                obj.insert("treasury".into(), json!(new_treasury));
            }
            all.insert(id, current);

            Reply::text(format!(
                "✅ *DONASI KAS BERHASIL!*\n\n\
                 Lu barusan masukin *Rp {}* ke brankas Guild!\n\
                 Total Kas Sekarang: *Rp {}* 🏰💰",
                thousands_id(amount as f64),
                thousands_id(new_treasury as f64)
            ))
            .mutating(vec![save(all), Mutation::SetUser {
                jid: ctx.sender.clone(),
                data: json!({ "koin": koin - amount }),
            }])
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(user: Value, settings: Value, args: &[&str]) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "guild".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.join(" "),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("guild").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn existing_guild() -> Value {
        json!({ "guilds": { "guild_1": {
            "id": "guild_1", "name": "DragonSlayers", "leader": "628222@s.whatsapp.net",
            "members": ["628222@s.whatsapp.net"], "treasury": 500, "level": 3, "exp": 0 } } })
    }

    fn member() -> Value {
        json!({ "rpg": { "guildId": "guild_1" } })
    }

    fn setting_of(r: &Reply) -> Value {
        let mut out = Value::Null;
        for m in r.mutations() {
            if let Mutation::SetSetting { key, value } = m {
                assert_eq!(key, "guilds");
                out = value.clone();
            }
        }
        out
    }

    fn user_of(r: &Reply, jid: &str) -> Value {
        r.mutations()
            .iter()
            .find_map(|m| match m {
                Mutation::SetUser { jid: j, data } if j == jid => Some(data.clone()),
                _ => None,
            })
            .unwrap_or(Value::Null)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("guild").unwrap();
        assert_eq!(p.meta.name, "guild");
        assert_eq!(p.meta.alias, ["clan", "team", "kelompok"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Sistem guild/clan");
        assert_eq!(p.meta.usage, ".guild <create/join/leave/info>");
        assert_eq!(p.meta.example, ".guild create DragonSlayers");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["clan", "team", "kelompok"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "guild");
        }
    }

    #[test]
    fn the_menu_names_every_command_and_the_membership_state() {
        let lonely_reply = run(json!({}), json!({}), &[]);
        let lonely = text_of(&lonely_reply);
        assert!(lonely.starts_with("🏰 *SERIKAT GUILD RPG*"), "{lonely}");
        for cmd in ["create", "join", "leave", "info", "members", "deposit", "list"] {
            assert!(lonely.contains(&format!("guild {cmd}")), "{cmd} hilang");
        }
        assert!(lonely.contains("*Jomblo Guild (Kagak Punya Temen)*"), "{lonely}");

        let joined_reply = run(member(), existing_guild(), &["help"]);
        let joined = text_of(&joined_reply);
        assert!(joined.contains("Tergabung di *DragonSlayers*"), "{joined}");
        assert!(!joined.contains("Jomblo"), "sudah punya guild: {joined}");
    }


    #[test]
    fn an_unknown_action_falls_through_to_the_menu() {
        let r = run(json!({}), json!({}), &["destroy"]);
        assert!(text_of(&r).starts_with("🏰 *SERIKAT GUILD RPG*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_top_list_is_empty_until_someone_creates_a_guild() {
        let r = run(json!({}), json!({}), &["list"]);
        assert!(text_of(&r).starts_with("Belum ada guild di server ini!"));
        assert!(r.mutations().is_empty());

        let r = run(json!({}), existing_guild(), &["list"]);
        let t = text_of(&r);
        assert!(t.starts_with("🏆 *DAFTAR TOP GUILD*"), "{t}");
        assert!(t.contains("🏰 *DragonSlayers* (Lv. 3)"), "{t}");
        assert!(t.contains("👥 Member: 1/50"), "{t}");
        assert!(t.contains("💰 Kas Kasir: Rp 500"), "{t}");
    }

    #[test]
    fn founding_a_guild_costs_ten_thousand_and_registers_the_founder() {
        let r = run(json!({ "koin": 50_000 }), json!({}), &["create", "Night", "Riders"]);
        let t = text_of(&r);
        assert!(t.starts_with("🎉 *GUILD RESMI BERDIRI!*"), "{t}");
        assert!(t.contains("👑 Ketua: @628111"), "{t}");
        assert!(t.contains("💸 Biaya Bangunan: *-Rp 10.000*"), "{t}");
        let me = user_of(&r, "628111@s.whatsapp.net");
        assert_eq!(me["koin"], json!(40_000));
        let guilds = setting_of(&r);
        let created = guilds.as_object().unwrap().values().next().unwrap().clone();
        assert_eq!(created["name"], json!("Night Riders"));
        assert_eq!(created["leader"], json!("628111@s.whatsapp.net"));
        assert_eq!(created["members"], json!(["628111@s.whatsapp.net"]));
        assert_eq!(created["level"], json!(1));
        assert_eq!(me["rpg"]["guildId"], created["id"]);
    }

    #[test]
    fn the_founder_name_is_checked_before_the_wallet() {
        for (name, expect) in [
            (vec!["create", "ab"], "minimal *3 huruf*"),
            (vec!["create", "abcdefghijklmnopqrstu"], "maksimal *20 huruf*"),
        ] {
            let r = run(json!({ "koin": 0 }), json!({}), &name);
            assert!(text_of(&r).contains(expect), "args {name:?}: {}", text_of(&r));
            assert!(r.mutations().is_empty(), "args {name:?}");
        }
    }

    #[test]
    fn a_taken_name_and_an_empty_wallet_each_stop_a_new_guild() {
        let r = run(json!({ "koin": 99_999 }), existing_guild(), &["create", "dragonslayers"]);
        assert!(text_of(&r).starts_with("Yahh, nama *dragonslayers* udah dipake kelompok lain!"));
        assert!(r.mutations().is_empty());

        let r = run(json!({ "koin": 9_999 }), json!({}), &["create", "Bocah"]);
        assert!(text_of(&r).starts_with("Miskin amat mau jadi ketua?"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_member_cannot_found_a_second_guild() {
        let r = run(member(), existing_guild(), &["create", "Lain"]);
        assert!(text_of(&r).starts_with("Rakus lu! Kan udah punya guild."));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn joining_is_case_insensitive_and_appends_the_member() {
        let r = run(json!({}), existing_guild(), &["join", "dragonslayers"]);
        assert!(
            text_of(&r).starts_with("✅ Selamat datang di barak!"),
            "{}",
            text_of(&r)
        );
        let guilds = setting_of(&r);
        let guild = guilds["guild_1"].clone();
        assert_eq!(
            guild["members"],
            json!(["628222@s.whatsapp.net", "628111@s.whatsapp.net"])
        );
        // `user.rpg.guildId = targetGuild.id` — bukan nama yang diketik.
        assert_eq!(user_of(&r, "628111@s.whatsapp.net")["rpg"]["guildId"], json!("guild_1"));
    }

    #[test]
    fn joining_needs_a_name_that_exists_and_room_to_sit() {
        let r = run(json!({}), existing_guild(), &["join"]);
        assert!(text_of(&r).starts_with("Tulis nama guild yang mau dimasukin!"));
        assert!(text_of(&r).contains("`.guild join DragonSlayers`"));

        let r = run(json!({ }), existing_guild(), &["join", "Nope"]);
        assert!(text_of(&r).starts_with("Guild *Nope* nggak ketemu!"));

        let full = json!({ "guilds": { "g": { "id": "g", "name": "Penuh",
            "leader": "628222@s.whatsapp.net",
            "members": (0..50).map(|i| format!("{i}@s.whatsapp.net")).collect::<Vec<_>>(),
            "treasury": 0, "level": 1 } } });
        let r = run(json!({}), full, &["join", "Penuh"]);
        assert!(text_of(&r).starts_with("Maaf bang, kapasitas markas guild *Penuh* udah full (50/50)!"));
    }

    #[test]
    fn a_member_cannot_join_a_second_guild() {
        let r = run(member(), existing_guild(), &["join", "DragonSlayers"]);
        assert!(text_of(&r).starts_with("Lu udah punya kelompok bro!"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn leaving_removes_only_the_caller_and_keeps_the_guild_alive() {
        let two = json!({ "rpg": { "guildId": "guild_1" } });
        let settings = json!({ "guilds": { "guild_1": {
            "id": "guild_1", "name": "DragonSlayers", "leader": "628222@s.whatsapp.net",
            "members": ["628222@s.whatsapp.net", "628111@s.whatsapp.net"],
            "treasury": 0, "level": 1 } } });
        let _ = two;
        let r = run(json!({ "rpg": { "guildId": "guild_1" } }), settings, &["leave"]);
        assert!(text_of(&r).starts_with("🏃 Lu cabut dari markas *DragonSlayers*"));
        let guilds = setting_of(&r);
        assert_eq!(
            guilds["guild_1"]["members"],
            json!(["628222@s.whatsapp.net"]),
            "ketua tetap di dalam"
        );
        assert_eq!(user_of(&r, "628111@s.whatsapp.net")["rpg"]["guildId"], Value::Null);
    }

    #[test]
    fn the_last_member_out_deletes_the_guild() {
        let settings = json!({ "guilds": { "guild_1": {
            "id": "guild_1", "name": "LastOne", "leader": "628222@s.whatsapp.net",
            "members": ["628111@s.whatsapp.net"], "treasury": 0, "level": 1 } } });
        let r = run(json!({ "rpg": { "guildId": "guild_1" } }), settings, &["leave"]);
        assert!(setting_of(&r).as_object().unwrap().is_empty(), "guild dihapus");
    }

    #[test]
    fn a_leader_with_members_may_not_walk_out() {
        let settings = json!({ "guilds": { "guild_1": {
            "id": "guild_1", "name": "Kepemimpinan", "leader": "628111@s.whatsapp.net",
            "members": ["628111@s.whatsapp.net", "628222@s.whatsapp.net"],
            "treasury": 0, "level": 1 } } });
        let r = run(json!({ "rpg": { "guildId": "guild_1" } }), settings, &["leave"]);
        assert!(text_of(&r).starts_with("Woy ketua! Masa mau ninggalin anggota gitu aja?"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn leaving_without_a_guild_or_after_a_dissolution_resets_the_pointer() {
        let r = run(json!({}), json!({}), &["leave"]);
        assert!(text_of(&r).starts_with("Lu aja belum masuk guild mana-mana"));
        assert!(r.mutations().is_empty());

        let r = run(member(), json!({}), &["leave"]);
        assert!(text_of(&r).starts_with("Guild lu kayaknya udah bubar"));
        assert_eq!(user_of(&r, "628111@s.whatsapp.net")["rpg"]["guildId"], Value::Null);
    }

    #[test]
    fn the_info_board_shows_the_stored_figures() {
        let r = run(member(), existing_guild(), &["info"]);
        let t = text_of(&r);
        assert!(t.starts_with("🏰 *PAPAN INFO GUILD*"), "{t}");
        assert!(t.contains("👑 Nama: *DragonSlayers*"), "{t}");
        assert!(t.contains("👤 Leader: @628222"), "{t}");
        assert!(t.contains("📊 Level: *3*"), "{t}");
        assert!(t.contains("👥 Anggota: *1/50*"), "{t}");
        assert!(t.contains("💰 Uang Kas: *Rp 500*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_member_roll_crowns_the_leader_and_arms_the_rest() {
        let settings = json!({ "guilds": { "guild_1": {
            "id": "guild_1", "name": "Antrean", "leader": "628222@s.whatsapp.net",
            "members": ["628222@s.whatsapp.net", "628111@s.whatsapp.net"],
            "treasury": 0, "level": 1 } } });
        let r = run(member(), settings, &["members"]);
        let t = text_of(&r);
        assert!(t.starts_with("👥 *DAFTAR ANGGOTA Antrean*"), "{t}");
        assert!(t.contains("1. @628222 👑"), "{t}");
        assert!(t.contains("2. @628111 🗡️"), "{t}");
        match r.without_mutations() {
            Reply::Text { mentions, .. } => assert_eq!(mentions.len(), 2),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn guildless_info_and_members_are_refused() {
        assert!(text_of(&run(json!({ }), json!({}), &["info"])).starts_with("Lu nggak punya guild bos!"));
        assert!(text_of(&run(json!({}), json!({}), &["members"])).starts_with("Cieee nggak punya guild..."));
        assert!(text_of(&run(json!({}), json!({}), &["deposit", "500"]))
            .starts_with("Lu mau donasi ke panti asuhan mana?"));
    }

    #[test]
    fn a_donation_moves_money_into_the_treasury() {
        let r = run(
            json!({ "koin": 10_000, "rpg": { "guildId": "guild_1" } }),
            existing_guild(),
            &["deposit", "1000"],
        );
        let t = text_of(&r);
        assert!(t.starts_with("✅ *DONASI KAS BERHASIL!*"), "{t}");
        assert!(t.contains("*Rp 1.000*"), "{t}");
        assert!(t.contains("Total Kas Sekarang: *Rp 1.500*"), "{t}");
        assert_eq!(setting_of(&r)["guild_1"]["treasury"], json!(1_500));
        assert_eq!(user_of(&r, "628111@s.whatsapp.net")["koin"], json!(9_000));
    }

    #[test]
    fn a_small_or_unaffordable_donation_is_refused() {
        let base = json!({ "koin": 10_000, "rpg": { "guildId": "guild_1" } });
        for amount in ["0", "99", "abc"] {
            let r = run(base.clone(), existing_guild(), &["deposit", amount]);
            assert!(text_of(&r).starts_with("Pelit amat! Minimal donasi kas *Rp 100* lah!"), "amount {amount}");
            assert!(r.mutations().is_empty(), "amount {amount}");
        }
        let r = run(
            json!({ "koin": 99, "rpg": { "guildId": "guild_1" } }),
            existing_guild(),
            &["deposit", "5000"],
        );
        assert!(text_of(&r).starts_with("Duit lu kurang bro buat donasi segitu!"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_missing_guild_row_blocks_every_guild_scoped_command() {
        for args in [vec!["info"], vec!["members"], vec!["deposit", "1000"]] {
            let r = run(json!({ "rpg": { "guildId": "ghost" } }), json!({}), &args);
            assert!(text_of(&r).starts_with("Guild tidak ditemukan!"), "args {args:?}");
            assert!(r.mutations().is_empty(), "args {args:?}");
        }
    }
}