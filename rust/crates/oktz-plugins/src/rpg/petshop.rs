//! `plugins/rpg/petshop.js` — beli pet dari toko.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::common::int_field;
use crate::rpg::jualan::{rpg_map, thousands_en};

/// `PETS_FOR_SALE` (`petshop.js:19-25`) — `(kunci, nama, harga, sifat)`.
pub(crate) const PETS: [(&str, &str, i64, &str); 5] = [
    (
        "cat",
        "🐱 Kucing",
        5000,
        "Bawa hoki (Luck tinggi, Attack sedang)",
    ),
    (
        "dog",
        "🐕 Anjing",
        6000,
        "Penjaga setia (Attack tinggi, Defense bagus)",
    ),
    ("bird", "🐦 Burung", 4500, "Lincah & Hoki (Luck sangat tinggi)"),
    ("fish", "🐟 Ikan", 3000, "Murah meriah (Bawa keberuntungan)"),
    (
        "rabbit",
        "🐰 Kelinci",
        5500,
        "Mungil & gesit (Balance semua stats)",
    ),
];

pub(crate) fn find_pet(key: &str) -> Option<&'static (&'static str, &'static str, i64, &'static str)> {
    PETS.iter().find(|p| p.0 == key)
}

/// `petToBuy.name.split(" ")[1] || "My Pet"` — namaFamili tanpa emote.
pub(crate) fn pet_name(name: &str) -> String {
    name.split(' ')
        .nth(1)
        .filter(|s| !s.is_empty())
        .unwrap_or("My Pet")
        .to_string()
}

#[plugin(
    name = "petshop",
    alias = ["tokopet", "buypet", "belipet"],
    category = "rpg",
    description = "Beli pet dari toko",
    usage = ".petshop <buy> <pet>",
    example = ".petshop buy cat",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn petshop(ctx: Ctx) -> Reply {
    let koin = int_field(&ctx, "koin");
    let action = ctx.args.first().map(|a| a.to_lowercase());
    let pet_key = ctx.args.get(1).map(|a| a.to_lowercase());

    if action.as_deref() != Some("buy") {
        let mut txt = String::from(
            "Halo Petualang! Selamat datang di Toko Hewan Peliharaan 🐾🏪\n\
             Pilih teman petualanganmu yang lucu-lucu ini!\n\n*Daftar Peliharaan:*\n",
        );
        for (key, name, price, desc) in PETS {
            txt += &format!(
                "\n*{name}*\n💰 Harga: Rp {}\n📝 Sifat: {desc}\n👉 Adopsi: `.petshop buy {key}`\n",
                thousands_en(price)
            );
        }
        txt += &format!("\n\n💰 *Uang Kamu:* Rp {}", thousands_en(koin));
        return Reply::text(txt);
    }

    let Some(key) = pet_key else {
        return Reply::text(format!(
            "Hayo, mau adopsi hewan apa nih? Sebutin jenisnya dong! 😂\n\
             Contoh: `{}petshop buy cat`",
            ctx.prefix
        ));
    };

    if ctx
        .user_field("rpg.pet")
        .is_some_and(|p| !p.is_null())
    {
        return Reply::text(
            "Waduh kak, kamu kan udah punya peliharaan! 😭\n\
             Kasihan nanti dia cemburu. Lepas dulu peliharaan lamamu atau coba sistem kawin silang (`.breeding`).",
        );
    }

    let Some(&(_, name, price, _)) = find_pet(&key) else {
        return Reply::text(format!(
            "Maaf kak, hewan jenis itu lagi kosong atau emang nggak dijual di sini! ❌\n\
             Cek daftarnya lagi pake `{}petshop`",
            ctx.prefix
        ));
    };

    if koin < price {
        return Reply::text(format!(
            "Aduh uangnya kurang nih buat biaya adopsi kak! 😭\n\
             Total biayanya Rp {} tapi uang kakak cuma Rp {}",
            thousands_en(price),
            thousands_en(koin)
        ));
    }

    let mut rpg = rpg_map(&ctx);
    rpg.insert(
        "pet".into(),
        json!({
            "type": key,
            "name": pet_name(name),
            "level": 1,
            "exp": 0,
            "hunger": 80,
            "stats": Value::Null,
        }),
    );
    // `if (!user.inventory) user.inventory = {}` ada di JS tapi tak pernah
    // dibaca atau ditulis setelahnya, jadi tidak ikut dibawa ke mutasi.
    Reply::text(format!(
        "SELAMAT! 🎉🎉\n\n\
         Kamu resmi mengadopsi *{name}*!\n\
         💰 Biaya Adopsi: *Rp -{}*\n\n\
         Dia udah gak sabar pengen jalan-jalan sama kamu. \
         Jangan lupa kasih makan dan cek statusnya pakai `{}pet` ya! 🐾✨",
        thousands_en(price),
        ctx.prefix
    ))
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({ "koin": koin - price, "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(args: &[&str], user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "petshop".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("petshop").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    /// Versi yang memiliki reply-nya — untuk `text(run(..))` di dalam assert.
    fn text(r: Reply) -> String {
        text_of(&r).to_string()
    }

    fn written(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, SENDER);
                data
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("petshop").unwrap();
        assert_eq!(p.meta.name, "petshop");
        assert_eq!(p.meta.alias, ["tokopet", "buypet", "belipet"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Beli pet dari toko");
        assert_eq!(p.meta.usage, ".petshop <buy> <pet>");
        assert_eq!(p.meta.example, ".petshop buy cat");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["tokopet", "buypet", "belipet"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "petshop");
        }
    }

    #[test]
    fn the_shop_front_lists_every_animal_in_table_order_with_the_purse() {
        for args in [vec![], vec!["sell"], vec!["adopt"]] {
            let t = text(run(&args, json!({"koin": 12_345})));
            assert!(t.starts_with("Halo Petualang!"), "{args:?} -> {t}");
            let pos = |s: &str| t.find(s).expect(s);
            assert!(pos("🐱 Kucing") < pos("🐰 Kelinci"), "{args:?} -> {t}");
            assert!(t.contains("💰 Harga: Rp 5,000"), "{args:?} -> {t}");
            assert!(t.contains("👉 Adopsi: `.petshop buy fish`"), "{args:?} -> {t}");
            assert!(t.ends_with("💰 *Uang Kamu:* Rp 12,345"), "{args:?} -> {t}");
        }
    }

    #[test]
    fn buying_without_a_pet_name_asks_for_one() {
        let t = text(run(&["buy"], json!({"koin": 9999})));
        assert!(t.contains("Sebutin jenisnya dong!"), "{t}");
        assert!(t.contains("`.petshop buy cat`"), "{t}");
    }

    #[test]
    fn one_pet_per_adopter() {
        let t = text(run(
            &["buy", "cat"],
            json!({"koin": 9999, "rpg": {"pet": {"type": "dog"}}}),
        ));
        assert!(t.starts_with("Waduh kak, kamu kan udah punya peliharaan!"), "{t}");
        assert!(t.contains("`.breeding`"), "{t}");
    }

    #[test]
    fn a_null_pet_still_counts_as_the_default_empty_bag() {
        // `if (user.rpg.pet)` — `null` falsy, jadi adopsi boleh jalan.
        let r = run(&["buy", "cat"], json!({"koin": 9999, "rpg": {"pet": null}}));
        assert!(text_of(&r).contains("SELAMAT!"));
    }

    #[test]
    fn an_unknown_species_is_refused() {
        let t = text(run(&["buy", "unicorn"], json!({"koin": 9999})));
        assert!(t.contains("hewan jenis itu lagi kosong"), "{t}");
        assert!(t.contains("`.petshop`"), "{t}");
    }

    #[test]
    fn the_adoption_record_starts_at_the_hard_coded_defaults() {
        let r = run(&["buy", "cat"], json!({"koin": 9999}));
        let d = written(&r);
        assert_eq!(d["koin"], json!(4999), "9999 - 5000");
        assert_eq!(
            d["rpg"]["pet"],
            json!({"type": "cat", "name": "Kucing", "level": 1, "exp": 0, "hunger": 80, "stats": null})
        );
    }

    #[test]
    fn a_broke_adopter_keeps_their_purse() {
        let r = run(&["buy", "dog"], json!({"koin": 5999}));
        let t = text_of(&r);
        assert!(t.contains("Total biayanya Rp 6,000"), "{t}");
        assert!(t.contains("uang kakak cuma Rp 5,999"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_display_name_drops_the_leading_emoji() {
        assert_eq!(pet_name("🐱 Kucing"), "Kucing");
        assert_eq!(pet_name("Kucing"), "My Pet", "`|| \"My Pet\"` di JS");
        assert_eq!(pet_name("🐱 "), "My Pet", "spesies kosong");
    }
}
