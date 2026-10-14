//! `plugins/fun/siapa.js` — acak satu member grup untuk kategori tertentu.
//!
//! `pluginConfig.name` di JS adalah array 52 kategori; `#[plugin]` hanya
//! menerima satu string, jadi kategori pertama jadi `name` dan sisanya jadi
//! alias (`husbualbum.rs` memakai aturan yang sama).
//!
//! `ponytail:` dua langkah JS TIDAK di-port: penyaringan
//! `id !== sock.user?.id?.split(':')[0] + '@s.whatsapp.net'` (butuh `sock`,
//! bot sendiri bisa ikut terundi) dan cabang `catch` yang memanggil
//! `te(m.prefix, m.command, m.pushName)` — handler Rust tidak bisa melempar,
//! jadi tidak ada error yang perlu ditangkap.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{jid_number, random_index};

/// `name` (kategori pertama) + sisa array `name` sebagai alias.
/// Dipakai tes untuk membandingkan attribute `#[plugin]` dengan array JS.
#[cfg(test)]
pub(crate) const COMMANDS: [&str; 52] = [
    "bego", "goblok", "janda", "perawan", "babi", "tolol", "pekok", "jancok", "pinter", "pintar",
    "asu", "bodoh", "gay", "lesby", "bajingan", "anjing", "anjg", "anjj", "anj", "ngentod", "ngentot",
    "monyet", "mastah", "newbie", "bangsat", "bangke", "sange", "sangean", "dakjal", "horny", "wibu",
    "puki", "puqi", "peak", "pantex", "pantek", "setan", "iblis", "cacat", "yatim", "piatu", "ganteng",
    "cantik", "jelek", "keren", "cupu", "noob", "pro", "sultan", "miskin", "kaya", "siapa",
];

/// `positiveWords.includes(command)` — Only these get the "Yang paling" label.
const POSITIVE: [&str; 9] = [
    "ganteng", "cantik", "keren", "pro", "sultan", "kaya", "pinter", "pintar", "mastah",
];

const NO_MEMBER: &str = "❌ Tidak ada member di grup!";

fn label(command: &str) -> &'static str {
    if POSITIVE.contains(&command) {
        "Yang paling"
    } else {
        "Anak"
    }
}

#[plugin(
    name = "bego",
    alias = [
        "goblok", "janda", "perawan", "babi", "tolol", "pekok", "jancok", "pinter", "pintar",
        "asu", "bodoh", "gay", "lesby", "bajingan", "anjing", "anjg", "anjj", "anj", "ngentod",
        "ngentot", "monyet", "mastah", "newbie", "bangsat", "bangke", "sange", "sangean",
        "dakjal", "horny", "wibu", "puki", "puqi", "peak", "pantex", "pantek", "setan", "iblis",
        "cacat", "yatim", "piatu", "ganteng", "cantik", "jelek", "keren", "cupu", "noob", "pro",
        "sultan", "miskin", "kaya", "siapa"
    ],
    category = "fun",
    description = "Random pilih member untuk kategori tertentu",
    usage = ".<kategori>",
    example = ".ganteng",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn siapa(ctx: Ctx) -> Reply {
    let command = ctx.command.to_lowercase();
    let members = ctx.roster.member_jids();
    if members.is_empty() {
        return Reply::text(NO_MEMBER);
    }
    let picked = &members[random_index(members.len())];
    Reply::Text {
        text: format!("*{} {} di sini adalah* @{}", label(&command), command, jid_number(picked)),
        mentions: vec![picked.clone()],
    }
    .with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Participant;

    fn ctx_of(command: &str, jids: &[&str]) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: command.into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            roster: oktz_plugin_api::registry::GroupRoster {
                participants: jids
                    .iter()
                    .map(|j| Participant {
                        jid: (*j).into(),
                        ..Default::default()
                    })
                    .collect(),
                is_bot_admin: false,
            },
            ..Default::default()
        }
    }

    /// `find(kategori)` tidak selalu mengembalikan plugin ini: `pintar`,
    /// `ganteng`, `cantik`, `cupu`, `kaya`, `noob`, `pro` juga alias plugin
    /// `cek*`, dan tabrakan itu memang ada di JS. Jadi test memanggil
    /// handler lewat `name` + `category`, bukan lewat `find`.
    fn entry() -> &'static oktz_plugin_api::PluginEntry {
        oktz_plugin_api::all_plugins()
            .iter()
            .copied()
            .find(|e| e.meta.name == "bego" && e.meta.category == "fun")
            .expect("plugin fun/siapa harus terdaftar")
    }

    fn run(command: &str, jids: &[&str]) -> Reply {
        (entry().handler)(ctx_of(command, jids))
    }

    fn body(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap().to_string()
    }

    /// Handler membungkus balasan dengan `.with_reaction("✅")`, jadi
    /// `without_mutations()` berhenti di `Reaction` — mentions ada satu
    /// level di dalam, bukan di `Text` langsung.
    fn mentions(r: &Reply) -> Vec<String> {
        match r.without_mutations() {
            Reply::Text { mentions, .. } => mentions.clone(),
            Reply::Reaction { reply, .. } => mentions(reply),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    fn emoji(r: &Reply) -> &str {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => emoji,
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = entry();
        assert_eq!(p.meta.name, "bego");
        assert_eq!(p.meta.category, "fun");
        assert_eq!(
            p.meta.description,
            "Random pilih member untuk kategori tertentu"
        );
        assert_eq!(p.meta.usage, ".<kategori>");
        assert_eq!(p.meta.example, ".ganteng");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_category_from_the_js_name_array_resolves() {
        // `find(c)` tidak boleh jadi kriteria: `pintar`/`ganteng`/`kaya`/dst
        // juga alias plugin `cek*` dan menang di registry — persis seperti di
        // JS. Yang diuji di sini: seluruh 52 kategori terdaftar sebagai
        // command plugin ini.
        let meta = &entry().meta;
        for c in COMMANDS {
            assert!(
                meta.name == c || meta.alias.iter().any(|a| a == c),
                "kategori {c} tidak terdaftar"
            );
        }
        assert_eq!(
            COMMANDS.len(),
            1 + meta.alias.len(),
            "sisa array name harus jadi alias"
        );
        assert_eq!(COMMANDS[0], "bego", "kategori pertama jadi name");
        assert_eq!(meta.name, COMMANDS[0]);
        // `bego` tidak bentrok dengan plugin lain, jadi registry bisa
        // membuktikan registrasinya lewat `find`.
        assert_eq!(
            oktz_plugin_api::find("bego").map(|p| p.meta.name.as_str()),
            Some("bego")
        );
    }

    #[test]
    fn a_chosen_member_is_mentioned_by_its_bare_number() {
        let r = run("ganteng", &["628111@s.whatsapp.net", "628222@s.whatsapp.net"]);
        let t = body(&r);
        assert!(t.starts_with("*Yang paling ganteng di sini adalah* @"), "{t}");
        let m = mentions(&r);
        assert_eq!(m.len(), 1, "tepat satu mention: {m:?}");
        let num = t.rsplit('@').next().unwrap();
        assert_eq!(m[0], format!("{num}@s.whatsapp.net"), "mention harus JID penuh");
        assert!(!num.contains('@'), "teks hanya nomor: {num}");
    }

    #[test]
    fn only_the_nine_positive_words_get_the_praise_label() {
        for c in POSITIVE {
            assert!(body(&run(c.clone(), &["628111@s.whatsapp.net"])).contains("*Yang paling "));
        }
        for c in ["bego", "ngentod", "jelek", "siapa"] {
            let __balasan = run(c.clone(), &["628111@s.whatsapp.net"]);
            let t = body(&__balasan);
            assert!(t.contains(&format!("*Anak {c} di sini adalah*")), "{c}: {t}");
        }
    }

    #[test]
    fn an_empty_roster_is_reported_and_nobody_is_mentioned() {
        let r = run("ganteng", &[]);
        assert_eq!(body(&r), NO_MEMBER);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_single_member_roster_always_picks_that_member() {
        let r = run("keren", &["628999@s.whatsapp.net"]);
        assert!(body(&r).ends_with("@628999"), "{}", body(&r));
        assert_eq!(mentions(&r), ["628999@s.whatsapp.net"]);
        assert_eq!(emoji(&r), "✅");
    }

    #[test]
    fn the_command_is_lowercased_before_matching_positive_words() {
        let mut ctx = ctx_of("GANTENG", &["628111@s.whatsapp.net"]);
        ctx.command = "GANTENG".into();
        let r = (entry().handler)(ctx);
        assert!(body(&r).starts_with("*Yang paling ganteng di sini adalah*"), "{}", body(&r));
    }
}
