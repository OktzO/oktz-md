//! `plugins/group/botmodegrup.js` — sakelar mode bot per grup.
//!
//! Nama file JS adalah `botmode.js`, tapi `pluginConfig.name` di dalamnya
//! `botmodegrup`; yang didaftarkan ke registry adalah nama itu.
//!
//! `ponytail:` hanya `handler` yang dipindah. `getGroupMode`,
//! `getModeCategories`, dan `filterCategoriesByMode` (`botmode.js:117-137`)
//! dipakai middleware untuk menyembunyikan command, bukan oleh handler ini,
//! jadi tidak ikut — lihat laporan batch.

use serde_json::Value;
use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// `(key, nama, deskripsi)` — urutan `Object.entries(MODES)` di JS, yang
/// menentukan urutan baris pada balasan tanpa argumen.
pub(crate) const MODES: [(&str, &str, &str); 6] = [
    ("md", "Multi-Device", "Mode default dengan semua fitur standar"),
    ("all", "All Features", "Semua fitur dari semua mode bisa diakses"),
    ("cpanel", "CPanel Pterodactyl", "Mode khusus untuk panel server"),
    ("pushkontak", "Push Kontak", "Mode khusus untuk push kontak ke member"),
    ("store", "Store/Toko", "Mode khusus untuk toko manual"),
    ("otp", "OTP Service", "Mode layanan OTP otomatis"),
];

pub(crate) const BAD_MODE: &str =
    "❌ Mode tidak valid. Pilihan: `md, all, cpanel, pushkontak, store, otp`";

/// `MODES[currentMode]?.name || 'Unknown'` — mode yang tersimpan tapi tidak
/// ada di tabel (mis. dihapus manual) tidak boleh membuat plugin gagal.
fn mode_name(key: &str) -> &str {
    MODES
        .iter()
        .find(|(k, ..)| *k == key)
        .map_or("Unknown", |(_, name, _)| *name)
}

fn mode_label(key: &str) -> String {
    key.to_uppercase()
}

#[plugin(
    name = "botmodegrup",
    alias = ["setmodegrup", "modegrup"],
    category = "group",
    description = "Atur mode bot untuk grup ini",
    usage = ".botmodegrup <md/cpanel/pushkontak/store/otp/all>",
    example = ".botmodegrup store",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn botmodegrup(ctx: Ctx) -> Reply {
    let mode = ctx
        .args
        .first()
        .map(|a| a.to_lowercase())
        .unwrap_or_default();
    let current = ctx
        .group_field("botMode")
        .and_then(Value::as_str)
        .unwrap_or("all");

    if mode.is_empty() {
        let mut mode_list = String::new();
        for (key, _, desc) in MODES {
            let marker = if key == current { " ⬅️" } else { "" };
            mode_list.push_str(&format!("┃ `{}botmode {key}`{marker}\n", ctx.prefix));
            mode_list.push_str(&format!("┃ └ {desc}\n"));
        }
        return Reply::text(format!(
            "🔧 *ʙᴏᴛ ᴍᴏᴅᴇ*\n\n\
             > Mode saat ini: *{}* ({})\n\
             \n╭─「 📋 *ᴘɪʟɪʜᴀɴ* 」\n\
             {mode_list}╰───────────────\n\n\
             *ꜰʟᴀɢ sᴛᴏʀᴇ:*\n\
             > `{}botmode store` - Manual order\n\n\
             > _Pengaturan per-grup_",
            mode_label(current),
            mode_name(current),
            ctx.prefix
        ));
    }

    if !MODES.iter().any(|(k, ..)| *k == mode) {
        return Reply::text(BAD_MODE);
    }

    let mut data = serde_json::json!({ "botMode": mode });
    // `mode === 'store'` juga ensured `storeConfig.products` ada, supaya mode
    // toko tidak setengah jadi. `Mutation::SetGroup` adalah patch dangkal,
    // jadi `storeConfig` ikut ditulis utuh — field lain di dalamnya harus
    // ikut disalin seperti `{ ...groupData.storeConfig }` di JS.
    let product_count = if mode == "store" {
        let mut config = ctx
            .group_field("storeConfig")
            .filter(|v| v.is_object())
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let products = config
            .get("products")
            .filter(|v| v.is_array())
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new()));
        let count = products.as_array().map_or(0, Vec::len);
        config["products"] = products;
        data["storeConfig"] = config;
        count
    } else {
        0
    };

    let extra = if mode == "store" {
        format!(
            "\n\n📋 *Manual mode*\n\
             > Admin perlu confirm order manual\n\
             > Product: `{}` item\n\n\
             *ᴘᴀɴᴅᴜᴀɴ:*\n\
             > `{}addprod <kode> <harga> <nama>`\n\
             > `{}listprod` - Lihat produk",
            product_count, ctx.prefix, ctx.prefix
        )
    } else {
        String::new()
    };

    Reply::text(format!(
        "✅ *ᴍᴏᴅᴇ ᴅɪᴜʙᴀʜ*\n\n\
         > Mode: *{}* ({})\n\
         > Grup: *{}*\n\
         {extra}\n\n\
         > Ketik `{}menu` untuk melihat menu.",
        mode_label(&mode),
        mode_name(&mode),
        crate::common::jid_number(&ctx.chat),
        ctx.prefix
    ))
    .with_reaction("✅")
    .with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "botmodegrup".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("botmodegrup").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1, "harus tepat satu mutasi");
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, "120363@g.us");
                data
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config_not_the_file_name() {
        let p = oktz_plugin_api::find("botmodegrup").unwrap();
        assert_eq!(p.meta.name, "botmodegrup");
        assert_eq!(p.meta.alias, ["setmodegrup", "modegrup"]);
        assert_eq!(p.meta.description, "Atur mode bot untuk grup ini");
        assert_eq!(p.meta.usage, ".botmodegrup <md/cpanel/pushkontak/store/otp/all>");
        assert_eq!(p.meta.example, ".botmodegrup store");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin);
        assert!(!p.meta.is_bot_admin, "tidak ada di pluginConfig");
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["setmodegrup", "modegrup"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "botmodegrup"
            );
        }
    }

    #[test]
    fn no_argument_lists_every_mode_and_marks_the_current_one() {
        let r = run(serde_json::json!({}), "");
        assert!(
            text_of(&r).contains("> Mode saat ini: *ALL* (All Features)"),
            "{}",
            text_of(&r)
        );
        for (key, ..) in MODES {
            assert!(
                text_of(&r).contains(&format!("┃ `.botmode {key}`")),
                "{key} harus tampil: {}",
                text_of(&r)
            );
        }
        assert!(text_of(&r).contains("┃ `.botmode all` ⬅️"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_stored_mode_marks_the_right_row() {
        let r = run(serde_json::json!({"botMode": "otp"}), "");
        assert!(
            text_of(&r).contains("> Mode saat ini: *OTP* (OTP Service)"),
            "{}",
            text_of(&r)
        );
        assert!(text_of(&r).contains("┃ `.botmode otp` ⬅️"), "{}", text_of(&r));
        assert!(text_of(&r).contains("┃ `.botmode all`\n"), "{}", text_of(&r));
    }

    #[test]
    fn an_unknown_stored_mode_falls_back_to_the_unknown_label() {
        let r = run(serde_json::json!({"botMode": "kustom"}), "");
        assert!(
            text_of(&r).contains("> Mode saat ini: *KUSTOM* (Unknown)"),
            "{}",
            text_of(&r)
        );
        assert!(text_of(&r).contains("┃ `.botmode md`\n"), "tidak ada yang ditandai");
    }

    #[test]
    fn a_valid_mode_is_written_and_confirmed() {
        let r = run(serde_json::json!({}), "md");
        assert_eq!(written(&r), &serde_json::json!({ "botMode": "md" }));
        assert!(text_of(&r).contains("> Mode: *MD* (Multi-Device)"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Grup: *120363*"), "{}", text_of(&r));
        assert!(!text_of(&r).contains("Manual mode"), "bukan mode store");
    }

    #[test]
    fn store_mode_also_seeds_an_empty_product_list() {
        let r = run(serde_json::json!({}), "store");
        let data = written(&r);
        assert_eq!(data["botMode"], serde_json::json!("store"));
        assert_eq!(data["storeConfig"]["products"], serde_json::json!([]));
        assert!(text_of(&r).contains("> Product: `0` item"), "{}", text_of(&r));
        assert!(
            text_of(&r).contains("`.addprod <kode> <harga> <nama>`"),
            "{}",
            text_of(&r)
        );
    }

    #[test]
    fn an_existing_product_list_survives_switching_to_store() {
        let r = run(
            serde_json::json!({"storeConfig": {"products": [{"kode": "a"}, {"kode": "b"}]}}),
            "store",
        );
        let data = written(&r);
        assert_eq!(data["storeConfig"]["products"].as_array().unwrap().len(), 2);
        assert!(text_of(&r).contains("> Product: `2` item"), "{}", text_of(&r));
    }

    #[test]
    fn an_invalid_mode_is_refused_and_nothing_is_written() {
        let r = run(serde_json::json!({}), "ngawur");
        assert_eq!(text_of(&r), BAD_MODE);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_mode_is_lowercased_before_validating() {
        assert_eq!(written(&run(serde_json::json!({}), "STORE"))["botMode"], serde_json::json!("store"));
        assert_eq!(written(&run(serde_json::json!({}), "  Md "))["botMode"], serde_json::json!("md"));
    }

    #[test]
    fn a_successful_switch_always_reacts() {
        match run(serde_json::json!({}), "otp").without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }
}