//! `plugins/fun/tod.rs` — putar botol: pilih satu member grup lalu undi
//! TRUTH atau DARE.
//!
//! `ponytail:` JS memanggil `sock.groupMetadata(m.chat)` dan, kalau roster
//! kosong, `randomMember.jid` melempar `TypeError` yang ditangkap `catch`.
//! Di sini roster kosong langsung menghasilkan balasan template error yang
//! sama — tidak ada jalur yang bisa "diam-diam" membalas tanpa target.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{jid_number, random_index};

/// `src/lib/error.js` memakai `config.errorTemplate` (`config.js:153`).
/// `m.pushName` tidak ada di `Ctx`, jadi fallback `'User'` yang dipakai.
#[allow(dead_code)]
pub(crate) fn error_template(prefix: &str, command: &str) -> String {
    format!(
        "☢ Kayaknya command `{prefix}{command}` lagi ada kendala\n\
         Silahkan coba lagi nanti, User\n\n\
         _Jika masalah berlanjut, silahkan hubungi owner bot_"
    )
}

const TRUTHS: [&str; 15] = [
    "Siapa orang terakhir yang kamu chat sebelum pesan ini?",
    "Apa rahasia terbesar yang belum pernah kamu ceritakan pada siapapun di grup ini?",
    "Jika kamu diwajibkan menikah dengan salah satu member grup ini, siapa yang akan kamu pilih? Tag orangnya!",
    "Pernahkah kamu menyukai seseorang di grup ini tapi tidak berani mengungkapkannya?",
    "Apa hal paling memalukan yang pernah terjadi padamu di depan umum?",
    "Sebutkan 3 hal yang paling kamu benci dari dirimu sendiri.",
    "Jika kamu bisa kembali ke masa lalu, momen memalukan apa yang ingin kamu ubah?",
    "Siapa mantan terindahmu? Kenapa putus?",
    "Pernahkah kamu berbohong ke orang tua demi bolos sekolah/kuliah/kerja? Ceritakan!",
    "Apa hal paling konyol yang pernah kamu cari di history Google pencarianmu?",
    "Siapa member di grup ini yang menurutmu paling asik, dan siapa yang paling ngeselin?",
    "Kapan terakhir kali kamu menangis dan apa alasannya?",
    "Berapa jumlah saldo di rekening/e-wallet kamu sekarang? Jujur!",
    "Apakah kamu pernah ketahuan stalking sosmed orang? Siapa?",
    "Apa kebohongan terbesar yang pernah kamu ucapkan ke sahabatmu?",
];

const DARES: [&str; 16] = [
    "Kirim foto selfie kamu dengan gaya paling jelek sekarang juga ke grup!",
    "Kirim VN (Voice Note) nyanyi reff lagu 'Balonku Ada Lima' tapi semua huruf vokalnya diganti jadi 'O'.",
    "Chat mantan kamu sekarang, bilang 'Aku masih sayang', lalu screenshot dan kirim ke grup ini!",
    "Pakai foto profil wajah jelekmu selama 1 jam penuh!",
    "Tulis status WhatsApp 'Aku sayang banget sama [Tag 1 Member Grup]' dan biarkan selama 30 menit. SS ke grup!",
    "Kirim VN bilang 'Mewing mewing sigma skibidi' dengan nada serius ke grup.",
    "Spam 5 stiker paling absurd/nggak jelas yang kamu punya ke grup ini sekarang.",
    "Tag salah satu admin grup dan bilang 'Kamu jelek banget hari ini'.",
    "Ubah bio WhatsApp kamu menjadi 'Saya badut ancol' selama 1 hari.",
    "Kirim gombalan paling maut ke member yang berlawanan jenis di grup ini!",
    "Buat 1 puisi alay tentang ayam goreng, bacakan lewat VN!",
    "Ketik 'Aku ini sebenernya alien' dan tag 3 orang secara acak di grup ini.",
    "Sebutkan nama lengkapmu dengan suara berteriak di VN!",
    "Pilih salah satu nomor secara acak dari kontakmu, chat 'P' sebanyak 10 kali, screenshot ke sini!",
    "Jadikan foto profil bot ini sebagai foto profil WhatsApp kamu selama 30 menit.",
    "Kirim VN menyanyikan lagu Indonesia paling viral dengan serapih mungkin!",
];

/// `Math.random() > 0.5` — undian 50/50 antara TRUTH dan DARE.
fn draw_challenge() -> (bool, String) {
    if rand_bool() {
        (
            true,
            TRUTHS[random_index(TRUTHS.len())].to_string(),
        )
    } else {
        (
            false,
            DARES[random_index(DARES.len())].to_string(),
        )
    }
}

/// `Math.random() > 0.5`. Bool dari RNG yang sama dengan sisa plugin.
fn rand_bool() -> bool {
    use rand::Rng as _;
    rand::rng().random_bool(0.5)
}

/// Badan balasan, terpisah supaya bisa diuji tanpa `Ctx`.
pub(crate) fn render(target_jid: &str) -> String {
    let (is_truth, challenge) = draw_challenge();
    let type_label = if is_truth {
        "🗣️ *TRUTH* 🗣️\n_(Harus dijawab dengan sangat jujur!)_"
    } else {
        "🔥 *DARE* 🔥\n_(Harus dilakukan, buktikan ke grup!)_"
    };
    format!(
        "🍾 *BOTOL DIPUTAR!* 🍾\n\n\
         Botol berputar kencang di tengah-tengah kalian...\n\
         Perlahan putarannya melambat, dan berhenti menunjuk kepada...\n\n\
         👉 @{n} 👈\n\n\
         {type_label}\n\n\
         *Tantangan/Pertanyaan:*\n👉 {challenge}",
        n = jid_number(target_jid)
    )
}

#[plugin(
    name = "tod",
    alias = ["spin", "putarbotol", "todgame"],
    category = "fun",
    description = "Bermain Truth or Dare bersama teman-teman grup secara instan!",
    usage = ".tod",
    example = ".tod",
    cooldown = 5,
    energi = 1,
    is_group = true,
    is_private = false
)]
pub fn tod(ctx: Ctx) -> Reply {
    if ctx.roster.is_empty() {
        return Reply::text(crate::common::error_template(&ctx));
    }
    let jid = ctx.roster.participants[random_index(ctx.roster.participants.len())].jid.clone();
    Reply::Text {
        text: render(&jid),
        mentions: vec![jid],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::registry::{GroupRoster, Participant};

    fn roster_of(jids: &[&str]) -> GroupRoster {
        GroupRoster {
            participants: jids
                .iter()
                .map(|j| Participant {
                    jid: (*j).to_string(),
                    is_admin: false,
                    is_owner: false,
                    data: Default::default(),
                })
                .collect(),
            is_bot_admin: false,
        }
    }

    fn run(roster: GroupRoster, command: &str) -> Reply {
        (oktz_plugin_api::find("tod").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: command.into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            roster,
            ..Default::default()
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("tod").unwrap();
        assert_eq!(p.meta.name, "tod");
        // `name` di JS berupa array; tiga elemen pertama jadi name + alias.
        assert_eq!(p.meta.alias, ["spin", "putarbotol", "todgame"]);
        assert_eq!(p.meta.category, "fun");
        assert_eq!(
            p.meta.description,
            "Bermain Truth or Dare bersama teman-teman grup secara instan!"
        );
        assert_eq!(p.meta.usage, ".tod");
        assert_eq!(p.meta.example, ".tod");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for key in ["tod", "spin", "putarbotol", "todgame"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("tod"),
                "key {key} salah resolve"
            );
        }
    }

    #[test]
    fn the_normal_branch_mentions_a_real_member_of_the_roster() {
        let roster = roster_of(&["628111@s.whatsapp.net", "628222@s.whatsapp.net"]);
        for _ in 0..50 {
            let r = run(roster.clone(), "tod");
            match r {
                Reply::Text { text, mentions } => {
                    assert_eq!(mentions.len(), 1);
                    assert!(
                        roster.participants.iter().any(|p| p.jid == mentions[0]),
                        "mention di luar roster: {:?}",
                        mentions[0]
                    );
                    assert!(text.contains(&format!("👉 @{} 👈", jid_number(&mentions[0]))));
                    assert!(text.starts_with("🍾 *BOTOL DIPUTAR!* 🍾\n\n"));
                }
                other => panic!("harus Text, dapat {other:?}"),
            }
        }
    }

    #[test]
    fn the_challenge_always_comes_from_the_matching_table() {
        for _ in 0..300 {
            let r = render("628111@s.whatsapp.net");
            if r.contains("🗣️ *TRUTH* 🗣️") {
                assert!(r.contains("_(Harus dijawab dengan sangat jujur!)_"));
            } else {
                assert!(r.contains("🔥 *DARE* 🔥"));
                assert!(r.contains("_(Harus dilakukan, buktikan ke grup!)_"));
            }
            let challenge = r.rsplit("👉 ").next().expect("baris tantangan");
            assert!(
                TRUTHS.contains(&challenge) || DARES.contains(&challenge),
                "tantangan di luar tabel: {challenge}"
            );
        }
    }

    #[test]
    fn both_branches_and_both_tables_are_reachable() {
        let mut truth = 0;
        let mut dare = 0;
        let mut seen_t = vec![false; TRUTHS.len()];
        let mut seen_d = vec![false; DARES.len()];
        for _ in 0..600 {
            let r = render("1@s.whatsapp.net");
            let challenge = r.rsplit("👉 ").next().expect("ada baris tantangan");
            if let Some(i) = TRUTHS.iter().position(|c| *c == challenge) {
                truth += 1;
                seen_t[i] = true;
            } else if let Some(i) = DARES.iter().position(|c| *c == challenge) {
                dare += 1;
                seen_d[i] = true;
            } else {
                panic!("tantangan di luar kedua tabel: {challenge}");
            }
        }
        assert!(truth > 0 && dare > 0, "salah satu cabang tidak terjangkau");
        assert!(
            seen_t.iter().all(|s| *s),
            "ada pertanyaan yang tidak pernah terpilih"
        );
        assert!(
            seen_d.iter().all(|s| *s),
            "ada dare yang tidak pernah terpilih"
        );
    }

    #[test]
    fn an_empty_roster_takes_the_error_branch() {
        // `sock.groupMetadata` + `randomMember.jid` di JS melempar TypeError,
        // jadi catch memanggil `te(...)`.
        let out = run(GroupRoster::default(), "tod");
        assert_eq!(
            out.text_body(),
            Some(crate::common::error_template_bawaan(".", "tod").as_str()),
            "harus template error"
        );
    }

    #[test]
    fn the_error_template_carries_the_live_prefix_and_command() {
        let r = (oktz_plugin_api::find("tod").unwrap().handler)(Ctx {
            prefix: "!".into(),
            command: "spin".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            ..Default::default()
        });
        let text = r.text_body().unwrap();
        assert!(text.contains("`!spin`"), "{text}");
        assert!(text.ends_with("silahkan hubungi owner bot_"), "{text}");
    }

    #[test]
    fn nothing_is_written_by_a_game_that_only_reads_the_roster() {
        let r = run(roster_of(&["628111@s.whatsapp.net"]), "tod");
        assert!(r.mutations().is_empty());
    }
}
