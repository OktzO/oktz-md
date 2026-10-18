//! `plugins/group/antispam.js` — sakelar proteksi spam + pengaturan jeda/delay.
//!
//! Hanya handler `.antispam` yang dipindah. Dua export lain tidak:
//! - `checkSpam` (:93) menghitung `Date.now()` per pesan dan membaca
//!   `m.fromMe`/`m.isOwner`, yang tidak ada di `Ctx`.
//! - `handleSpamAction` (:133) butuh `sock.groupParticipantsUpdate` (:171)
//!   dan `sock.sendMessage({delete})` (:184).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

const KNOWN_ACTIONS: [&str; 5] = ["on", "off", "warning", "kick", "delete"];

fn help(prefix: &str) -> String {
    format!(
        "🛡️ *ANTI SPAM GROUP*\n\n\
         Fitur ini melindungi grup dari member yang mengirim pesan berulang-ulang dengan sangat cepat dan brutal sehingga mengganggu kenyamanan member lain\n\n\
         *Cara pakai:*\n\
         > `{prefix}antispam on` (Aktifkan fitur antispam)\n\
         > `{prefix}antispam off` (Matikan fitur antispam)\n\n\
         *Pilih Metode Hukuman:*\n\
         > `{prefix}antispam warning` (Beri teguran keras hingga 3 kali peringatan)\n\
         > `{prefix}antispam kick` (Otomatis tendang spammer langsung tanpa ampun)\n\
         > `{prefix}antispam delete` (Hapus seluruh pesan spam yang dikirimkan)\n\n\
         *Atur Sensitivitas Jeda (Delay):*\n\
         > `{prefix}antispam 2` (Set jarak antar pesan maksimal 2 detik)\n\
         > `{prefix}antispam 1500` (Set jarak ke 1500 milidetik)"
    )
}

/// `/^(\d+)(s|ms)?$/` — kembalikan `(angka, satuan)`, `None` kalau tidak cocok.
/// Group kedua hanya menerima `"s"` dan `"ms"`; `"m"` atau `"x"` tidak match.
fn split_delay(action: &str) -> Option<(&str, &str)> {
    let end = action
        .bytes()
        .position(|b| !b.is_ascii_digit())
        .unwrap_or(action.len());
    let (digits, unit) = action.split_at(end);
    let unit = match unit {
        "" => "",
        "s" => "s",
        "ms" => "ms",
        _ => return None,
    };
    // `(\d+)` minimal satu digit: "s" sendiri tidak boleh jadi delay.
    (!digits.is_empty()).then_some((digits, unit))
}

/// `parseInt(digits)` dengan saturasi, supaya digit raksasa tidak `None`
/// (JS menghasilkan angka besar yang lalu di-clamp, bukan `NaN`).
fn parse_delay_digits(digits: &str) -> i64 {
    digits.bytes().fold(0i64, |acc, b| {
        acc.saturating_mul(10).saturating_add(i64::from(b - b'0'))
    })
}

/// `(delayMs / 1000).toFixed(1)`.
///
/// `toFixed` JS memilih n terbesar saat seri, jadi 1.25 jadi "1.3". Karena
/// `delayMs` sudah di-clamp ke 500..=10000, pembagiannya cukup dihitung bulat:
/// desimal pertama = `delayMs / 100`, naik satu saat sisa >= 50.
fn one_decimal_second(delay_ms: i64) -> String {
    let tenths = if delay_ms % 100 >= 50 {
        delay_ms / 100 + 1
    } else {
        delay_ms / 100
    };
    format!("{}.{}", tenths / 10, tenths % 10)
}

#[plugin(
    name = "antispam",
    alias = ["antispamgc"],
    category = "group",
    description = "Mengatur fitur perlindungan grup dari pesan spam secara brutal",
    usage = ".antispam <on/off/action/delay>",
    example = ".antispam on\n.antispam warning\n.antispam 2",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn antispam(ctx: Ctx) -> Reply {
    let Some(action) = ctx.args.first().map(|a| a.to_lowercase()) else {
        return Reply::text(help(&ctx.prefix));
    };

    let delay = split_delay(&action);
    if !KNOWN_ACTIONS.contains(&action.as_str()) && delay.is_none() {
        return Reply::text(help(&ctx.prefix));
    }

    if let Some((digits, unit)) = delay {
        let mut delay_ms = parse_delay_digits(digits);
        // Syarat dievaluasi terhadap nilai SEBELUM dikali, sama seperti JS.
        if unit == "s" || (1..=10).contains(&delay_ms) {
            delay_ms = delay_ms.saturating_mul(1000);
        }
        delay_ms = delay_ms.clamp(500, 10_000);
        let seconds = one_decimal_second(delay_ms);
        return Reply::text(format!(
            "🛡️ *SENSITIVITAS ANTI SPAM DIPERBARUI*\n\n\
             > Jeda Maksimal: *{delay_ms} ms* ({seconds} detik)\n\n\
             Sistem kini akan menganggap pesan sebagai spam jika anggota mengirim beberapa pesan dengan jeda di bawah *{seconds} detik* antar pesannya"
        ))
        .with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "antispamDelay": delay_ms }),
        });
    }

    let write = |data: serde_json::Value, text: String| {
        Reply::text(text).with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data,
        })
    };

    if action == "on" || action == "off" {
        let is_enable = action == "on";
        // `groupData.antispam === isEnable` — perbandingan ketat dengan bool,
        // jadi nilai yang bukan `true`/`false` (mis. angka) tidak pernah sama.
        let stored = ctx
            .group_field("antispam")
            .and_then(serde_json::Value::as_bool);
        if stored == Some(is_enable) {
            return Reply::text(format!(
                "✅ Fitur antispam sudah {} di grup ini, tidak ada perubahan yang dibuat",
                if is_enable { "aktif" } else { "nonaktif" }
            ));
        }
        let status = if is_enable { "AKTIF ✅" } else { "NONAKTIF ❌" };
        let tighten = if is_enable {
            "mengawasi secara ketat"
        } else {
            "berhenti mengawasi"
        };
        return write(
            serde_json::json!({ "antispam": is_enable }),
            format!(
                "🛡️ *ANTI SPAM DIPERBARUI*\n\n\
                 > Status: *{status}*\n\n\
                 Sistem bot kini akan {tighten} setiap aktivitas spam atau flood pesan yang dilakukan oleh member di dalam grup ini"
            ),
        );
    }

    let text_action = match action.as_str() {
        "warning" => "Memberikan peringatan keras secara bertahap",
        "kick" => "Menendang member yang membandel secara otomatis",
        "delete" => "Menghapus pesan spam yang mengganggu",
        _ => "",
    };
    write(
        serde_json::json!({ "antispamAction": &action }),
        format!(
            "🛡️ *AKSI ANTI SPAM DIPERBARUI*\n\n\
             > Metode Hukuman: *{}*\n\n\
             Sistem bot akan langsung mengambil tindakan berupa *{text_action}* apabila ada member yang terdeteksi melakukan pelanggaran berupa tindakan spam brutal",
            action.to_uppercase()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "antispam".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("antispam").unwrap().handler)(ctx)
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

    fn delay_ms_of(r: &Reply) -> i64 {
        written(r)["antispamDelay"]
            .as_i64()
            .expect("harus angka")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("antispam").unwrap();
        assert_eq!(p.meta.name, "antispam");
        assert_eq!(p.meta.alias, ["antispamgc"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Mengatur fitur perlindungan grup dari pesan spam secara brutal"
        );
        assert_eq!(p.meta.usage, ".antispam <on/off/action/delay>");
        assert_eq!(
            p.meta.example,
            ".antispam on\n.antispam warning\n.antispam 2"
        );
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin);
        // `isPrivate` dan `isBotAdmin` tidak ada di pluginConfig JS.
        assert!(!p.meta.is_bot_admin);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn the_alias_resolves_to_this_plugin() {
        assert_eq!(
            oktz_plugin_api::find("antispamgc").unwrap().meta.name,
            "antispam"
        );
    }

    #[test]
    fn missing_and_unknown_actions_show_the_help_without_writing() {
        for text in ["", "mungkin", "on2", "s", "-5", "5m", "5x"] {
            let r = run(serde_json::json!({}), text);
            assert_eq!(text_of(&r), help("."), "input {text:?}");
            assert!(r.mutations().is_empty(), "input {text:?} tidak boleh menulis");
        }
    }

    #[test]
    fn the_delay_regex_accepts_only_digits_with_s_or_ms() {
        assert_eq!(split_delay("2"), Some(("2", "")));
        assert_eq!(split_delay("1500"), Some(("1500", "")));
        assert_eq!(split_delay("7s"), Some(("7", "s")));
        assert_eq!(split_delay("7ms"), Some(("7", "ms")));
        assert_eq!(split_delay("007"), Some(("007", "")));
        assert_eq!(split_delay(""), None);
        assert_eq!(split_delay("s"), None, "digitnya wajib minimal satu");
        assert_eq!(split_delay("5m"), None, "hanya s dan ms yang sah");
    }

    #[test]
    fn a_bare_one_to_ten_is_read_as_seconds() {
        assert_eq!(delay_ms_of(&run(serde_json::json!({}), "2")), 2000);
        assert_eq!(delay_ms_of(&run(serde_json::json!({}), "10")), 10_000);
        // Di atas 10 tidak dikali, lalu dijepit ke batas bawah 500.
        assert_eq!(delay_ms_of(&run(serde_json::json!({}), "11")), 500);
        assert_eq!(delay_ms_of(&run(serde_json::json!({}), "1500")), 1500);
    }

    #[test]
    fn the_s_suffix_always_means_seconds() {
        assert_eq!(delay_ms_of(&run(serde_json::json!({}), "20s")), 10_000);
        // 1s -> 1000, di atas batas bawah; 0s -> 0, dijepit ke 500.
        assert_eq!(delay_ms_of(&run(serde_json::json!({}), "1s")), 1000);
        assert_eq!(delay_ms_of(&run(serde_json::json!({}), "0s")), 500);
    }

    #[test]
    fn a_small_bare_number_is_multiplied_even_with_the_ms_suffix() {
        // `delayMatch[2] === "s" || (delayMs >= 1 && delayMs <= 10)` — syarat
        // angka tidak pernah melihat satuan, jadi "9ms" jadi 9000 juga.
        assert_eq!(delay_ms_of(&run(serde_json::json!({}), "9ms")), 9000);
        assert_eq!(delay_ms_of(&run(serde_json::json!({}), "10ms")), 10_000);
        // Di luar 1..10 satuan `ms` baru benar-benar milidetik.
        assert_eq!(delay_ms_of(&run(serde_json::json!({}), "11ms")), 500);
        assert_eq!(delay_ms_of(&run(serde_json::json!({}), "750ms")), 750);
    }

    #[test]
    fn the_delay_is_clamped_into_500_to_10000() {
        assert_eq!(delay_ms_of(&run(serde_json::json!({}), "0")), 500);
        assert_eq!(delay_ms_of(&run(serde_json::json!({}), "499")), 500);
        assert_eq!(delay_ms_of(&run(serde_json::json!({}), "10001")), 10_000);
        assert_eq!(
            delay_ms_of(&run(serde_json::json!({}), "99999999999999999999999999")),
            10_000,
            "digit raksasa harus di-clamp, bukan jadi NaN"
        );
    }

    #[test]
    fn tofixed_one_rounds_half_up_like_js() {
        // JS: (1.25).toFixed(1) === "1.3" — seri memilih n terbesar.
        assert_eq!(one_decimal_second(500), "0.5");
        assert_eq!(one_decimal_second(750), "0.8");
        assert_eq!(one_decimal_second(1250), "1.3");
        assert_eq!(one_decimal_second(1500), "1.5");
        assert_eq!(one_decimal_second(2500), "2.5");
        assert_eq!(one_decimal_second(995), "1.0");
        assert_eq!(one_decimal_second(10_000), "10.0");
    }

    #[test]
    fn the_delay_reply_quotes_the_same_seconds_twice() {
        let r = run(serde_json::json!({}), "1500");
        assert!(text_of(&r).contains("> Jeda Maksimal: *1500 ms* (1.5 detik)"), "{}", text_of(&r));
        assert!(
            text_of(&r).contains("di bawah *1.5 detik* antar pesannya"),
            "{}",
            text_of(&r)
        );
    }

    #[test]
    fn on_and_off_write_the_flag() {
        let on = run(serde_json::json!({}), "on");
        assert_eq!(written(&on), &serde_json::json!({ "antispam": true }));
        assert!(
            text_of(&on).contains("> Status: *AKTIF ✅*")
                && text_of(&on).contains("kini akan mengawasi secara ketat"),
            "{}",
            text_of(&on)
        );

        let off = run(serde_json::json!({ "antispam": true }), "off");
        assert_eq!(written(&off), &serde_json::json!({ "antispam": false }));
        assert!(
            text_of(&off).contains("> Status: *NONAKTIF ❌*")
                && text_of(&off).contains("kini akan berhenti mengawasi"),
            "{}",
            text_of(&off)
        );
    }

    #[test]
    fn repeating_the_current_state_writes_nothing() {
        let already_on = run(serde_json::json!({ "antispam": true }), "on");
        assert_eq!(
            text_of(&already_on),
            "✅ Fitur antispam sudah aktif di grup ini, tidak ada perubahan yang dibuat"
        );
        assert!(already_on.mutations().is_empty());

        let already_off = run(serde_json::json!({ "antispam": false }), "off");
        assert_eq!(
            text_of(&already_off),
            "✅ Fitur antispam sudah nonaktif di grup ini, tidak ada perubahan yang dibuat"
        );
        assert!(already_off.mutations().is_empty());
    }

    #[test]
    fn a_non_boolean_stored_flag_is_not_equal_to_the_new_value() {
        // `1 === true` false di JS, jadi angka tidak boleh dianggap "sudah on".
        let r = run(serde_json::json!({ "antispam": 1 }), "on");
        assert_eq!(written(&r), &serde_json::json!({ "antispam": true }));

        let missing = run(serde_json::json!({}), "off");
        assert_eq!(written(&missing), &serde_json::json!({ "antispam": false }));
    }

    #[test]
    fn punishment_actions_write_the_action_and_describe_it() {
        for (input, label, desc) in [
            (
                "warning",
                "WARNING",
                "Memberikan peringatan keras secara bertahap",
            ),
            (
                "kick",
                "KICK",
                "Menendang member yang membandel secara otomatis",
            ),
            (
                "delete",
                "DELETE",
                "Menghapus pesan spam yang mengganggu",
            ),
        ] {
            let r = run(serde_json::json!({}), input);
            // `antispam.js:20` mentohkan `args[0]` lebih dulu, jadi yang
            // ditulis ke store adalah bentuk huruf kecil.
            assert_eq!(written(&r), &serde_json::json!({ "antispamAction": input.to_lowercase() }));
            // JS `antispam.js:88` membungkus deskripsi di dalam asterisk,
            // jadi yang dibandingkan termasuk tandanya.
            assert!(
                text_of(&r).contains(&format!("> Metode Hukuman: *{label}*")),
                "{}",
                text_of(&r)
            );
            assert!(
                text_of(&r).contains(&format!("*{desc}*")),
                "{}",
                text_of(&r)
            );
        }
    }

    #[test]
    fn the_action_is_lowercased_before_matching() {
        assert_eq!(
            written(&run(serde_json::json!({}), "WARNING")),
            &serde_json::json!({ "antispamAction": "warning" })
        );
        // Tampilannya tetap huruf besar walau inputnya huruf kecil.
        assert!(text_of(&run(serde_json::json!({}), "KICK")).contains("*KICK*"));
    }
}