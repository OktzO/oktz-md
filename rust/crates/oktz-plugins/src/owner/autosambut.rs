//! `plugins/owner/autosambut.js` — `groups[chat].autoSambut`
//! (on/off/delay/add/del/list).
//!
//! `ponytail:` flag `--global` tidak bisa diport: cabangnya menulis ke SETIAP
//! grup hasil `sock.groupFetchAllParticipating()`, sedangkan `Ctx` tidak punya
//! daftar grup dan satu `Mutation::SetGroup` hanya bisa satu JID. Perintah yang
//! membawa `--global` di sini berlaku untuk grup ini saja. Tambahkan
//! `all_group_jids: Vec<String>` ke `Ctx` kalau router mau keduanya.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::common::{js_truthy, js_truthy_value, parse_int, string_list};

const DEFAULT_PESAN: &str = "Halo {user}! Selamat datang kembali 🙇‍♂️";
const DEFAULT_DELAY_MS: i64 = 2 * 60 * 60 * 1000;

/// `groupData.autoSambut` setelah blockade di JS di-normalisasi: object default
/// kalau belum ada, `pesan` lama dimigrasi ke `pesanList`, dan `pesanList`
/// dijamin tidak kosong.
struct Auto {
    enabled: bool,
    delay_ms: i64,
    pesan: Vec<String>,
}

/// `parseTime` (`autosambut.js:19-30`) — `^([\d.]+)([a-zA-Z]+)$`, satuan dari
/// huruf pertama saja (`minutes` tidak ada, `menit` juga ditolak).
pub(crate) fn parse_time(raw: &str) -> Option<i64> {
    let split = raw
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit() && *c != '.')?;
    let (num, unit) = raw.split_at(split.0);
    if num.is_empty() || !unit.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    // `parseFloat` berhenti di titik kedua: "1.2.3" -> 1.2.
    let value = match num.split_once('.') {
        Some((whole, frac)) => {
            let frac: String = frac.chars().take_while(char::is_ascii_digit).collect();
            format!("{whole}.{frac}").parse::<f64>().ok()
        }
        None => num.parse::<f64>().ok(),
    }?;
    let ms = match unit.to_lowercase().chars().next()? {
        's' => value * 1000.0,
        'm' => value * 60.0 * 1000.0,
        'h' => value * 60.0 * 60.0 * 1000.0,
        'd' => value * 24.0 * 60.0 * 60.0 * 1000.0,
        'w' => value * 7.0 * 24.0 * 60.0 * 60.0 * 1000.0,
        'y' => value * 365.0 * 24.0 * 60.0 * 60.0 * 1000.0,
        _ => return None,
    };
    Some(ms as i64)
}

/// `formatTime` (`autosambut.js:32-38`) — cabang terakhir tetap "hari", bukan
/// minggu: dua jalur `return` di JS sama-sama begitu.
pub(crate) fn format_time(ms: i64) -> String {
    let ms = ms as f64;
    if ms < 60_000.0 {
        format!("{} detik", ms / 1000.0)
    } else if ms < 3_600_000.0 {
        format!("{} menit", ms / 60_000.0)
    } else if ms < 86_400_000.0 {
        format!("{} jam", ms / 3_600_000.0)
    } else {
        format!("{} hari", ms / 86_400_000.0)
    }
}

/// Normalisasi `autoSambasut`. Nilai balik kedua adalah payload yang harus
/// ditulis sebagai efek samping pemuatan (seed default atau migrasi `pesan`),
/// persis dua `db.setGroup` di awal handler JS.
fn load(ctx: &Ctx) -> (Auto, Option<Value>) {
    let mut write: Option<Value> = None;
    let mut root = match ctx.group_field("autoSambut") {
        Some(v) if js_truthy_value(v) => v.clone(),
        _ => {
            let seed = json!({
                "enabled": false,
                "delayMs": DEFAULT_DELAY_MS,
                "pesanList": [DEFAULT_PESAN],
                "lastChats": {}
            });
            write = Some(seed.clone());
            seed
        }
    };

    // `pesan` (versi lama) -> `pesanList`, lalu key `pesan` dihapus.
    if let Some(pesan) = root.get("pesan").cloned()
        && let Some(obj) = root.as_object_mut()
    {
        if !obj.get("pesanList").is_some_and(|v| js_truthy(Some(v))) {
            obj.insert("pesanList".into(), Value::Array(vec![pesan]));
        }
        obj.remove("pesan");
        write = Some(root.clone());
    }

    let list = root.get("pesanList").and_then(Value::as_array);
    let pesan = match list {
        Some(a) if !a.is_empty() => string_list(Some(&Value::Array(a.clone()))),
        _ => vec![DEFAULT_PESAN.to_string()],
    };

    let enabled = js_truthy(root.get("enabled"));
    let delay_ms = match root.get("delayMs") {
        Some(v) if js_truthy(Some(v)) => v.as_i64().unwrap_or(DEFAULT_DELAY_MS),
        _ => DEFAULT_DELAY_MS,
    };

    (
        Auto {
            enabled,
            delay_ms,
            pesan,
        },
        write,
    )
}

/// Bentuk yang ditulis setiap kali status berubah: objek utuh dengan
/// `lastChats` kosong, sama seperti `db.setGroup` di JS.
fn write_payload(auto: &Auto) -> Value {
    json!({
        "enabled": auto.enabled,
        "delayMs": auto.delay_ms,
        "pesanList": auto.pesan,
        "lastChats": {}
    })
}

fn mutasi(ctx: &Ctx, auto: &Auto) -> Mutation {
    Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: json!({ "autoSambut": write_payload(auto) }),
    }
}

fn usage(ctx: &Ctx, auto: &Auto) -> String {
    let status = if auto.enabled { "Aktif ✅" } else { "Nonaktif ❌" };
    format!(
        "⚠️ *SISTEM AUTO SAMBUT*\n\n\
         Sistem otomatis menyambut owner di grup secara acak ketika owner muncul setelah lama idle.\n\
         Status: *{status}*\n\
         Batas Waktu Idle: *{}*\n\
         Jumlah Pesan Acak: *{} Sapaan*\n\n\
         *PENGGUNAAN UTAMA:*\n\
         • *{prefix}autosambut on/off* — Menghidupkan/mematikan fitur di grup ini\n\
         • *{prefix}autosambut delay <waktu>* — Mengubah batas waktu idle\n\n\
         *PENGATURAN PESAN ACAK (LIST):*\n\
         • *{prefix}autosambut list* — Melihat semua sapaan yang telah didaftarkan\n\
         • *{prefix}autosambut add <teks>* — Menambah teks sambutan baru ke daftar\n\
         • *{prefix}autosambut del <angka>* — Menghapus pesan pada nomor urutan tertentu\n\n\
         *PENJELASAN KHUSUS:*\n\
         1. Gunakan format waktu: *s* (detik), *m* (menit), *h* (jam), *d* (hari). Contoh: *{prefix}autosambut delay 30m*\n\
         2. Gunakan *{{name}}* untuk menyebut pushname owner, dan *{{user}}* untuk me-mention owner.\n\
         3. Jika kamu menambahkan atribut *--global* di akhir setiap perintah, maka pengaturan di grup *ini* akan langsung dicopy ke SEMUA grup yang bot singgahi!",
        format_time(auto.delay_ms),
        auto.pesan.len(),
        prefix = ctx.prefix,
    )
}

#[plugin(
    name = "autosambut",
    alias = ["sambutowner"],
    category = "group",
    description = "Mengatur fitur sambutan otomatis saat owner muncul setelah lama idle",
    usage = ".autosambut on/off/delay/add/del/list",
    example = ".autosambut on",
    cooldown = 3,
    energi = 0,
    is_group = true,
    is_private = false,
    is_owner = true
)]
pub fn autosambut(ctx: Ctx) -> Reply {
    let action = ctx.args.first().map(|a| a.to_lowercase());
    let (mut auto, seed) = load(&ctx);
    // JS menulis seed/migrasi dulu, apa pun aksi-nya; branch tanpa aksi
    // perubahan apa pun hanya bawa efek pemuatan itu.
    let mut writes: Vec<Mutation> = seed
        .map(|v| Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: json!({ "autoSambut": v }),
        })
        .into_iter()
        .collect();

    let Some(action) = action.as_deref() else {
        return Reply::text(usage(&ctx, &auto)).mutating(writes);
    };

    match action {
        "on" | "off" => {
            auto.enabled = action == "on";
            writes.push(mutasi(&ctx, &auto));
            let text = if auto.enabled {
                "✅ *Fitur Auto Sambut Diaktifkan!*"
            } else {
                "❌ *Fitur Auto Sambut Dinonaktifkan.*"
            };
            Reply::text(text).mutating(writes)
        }
        "delay" => {
            let Some(input) = ctx.args.get(1) else {
                return Reply::text(format!(
                    "Tolong berikan waktu! Contoh: `{}autosambut delay 2h`",
                    ctx.prefix
                ))
                .mutating(writes);
            };
            let Some(ms) = parse_time(input) else {
                return Reply::text(
                    "Format waktu tidak dikenali. Gunakan angka dan akhiran s, m, h, d, w, y. \
                     Contoh: `2h` (2 jam), `30m` (30 menit)."
                        .to_string(),
                )
                .mutating(writes);
            };
            auto.delay_ms = ms;
            writes.push(mutasi(&ctx, &auto));
            Reply::text(format!(
                "⏱️ *Delay Auto Sambut Diubah!*\n\n\
                 Sekarang bot akan menyambutmu setelah kamu tidak mengetik apa-apa di grup ini \
                 selama *{}* berturut-turut.",
                format_time(ms)
            ))
            .mutating(writes)
        }
        "list" => {
            let mut txt = format!(
                "📜 *DAFTAR PESAN AUTO SAMBUT*\n\n\
                 Total ada *{}* sapaan acak yang terdaftar di grup ini:\n\n",
                auto.pesan.len()
            );
            for (i, text) in auto.pesan.iter().enumerate() {
                txt.push_str(&format!("*{}.* {text}\n\n", i + 1));
            }
            txt.push_str(&format!(
                "_Gunakan `{}autosambut del <angka>` untuk menghapus salah satu._",
                ctx.prefix
            ));
            Reply::text(txt).mutating(writes)
        }
        "add" => {
            let new_msg = ctx.args[1..]
                .iter()
                .filter(|a| a.as_str() != "--global")
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            if new_msg.is_empty() {
                return Reply::text(format!(
                    "Tolong masukkan teks sambutan.\nContoh: `{}autosambut add Halo bosku {{user}}!`",
                    ctx.prefix
                ))
                .mutating(writes);
            }
            auto.pesan.push(new_msg);
            writes.push(mutasi(&ctx, &auto));
            Reply::text(format!(
                "💬 *Pesan Baru Berhasil Ditambahkan!*\n\
                 Kini ada {} sapaan acak di dalam daftar.",
                auto.pesan.len()
            ))
            .mutating(writes)
        }
        "del" => {
            let Some(index) = ctx
                .args
                .get(1)
                .and_then(|a| parse_int(a))
                .filter(|i| *i >= 1 && *i as usize <= auto.pesan.len())
            else {
                return Reply::text(format!(
                    "Tolong masukkan angka urutan pesan yang valid.\n\
                     Lihat daftar angka dengan `{}autosambut list`.",
                    ctx.prefix
                ))
                .mutating(writes);
            };
            if auto.pesan.len() <= 1 {
                return Reply::text(
                    "Gagal dihapus! Harus ada minimal 1 pesan di dalam daftar sapaan grup ini."
                        .to_string(),
                )
                .mutating(writes);
            }
            let removed = auto.pesan.remove(index as usize - 1);
            writes.push(mutasi(&ctx, &auto));
            Reply::text(format!(
                "🗑️ *Pesan Berhasil Dihapus!*\n\nTerhapus:\n\"{removed}\"\n\
                 Sisa daftar sapaan: {}",
                auto.pesan.len()
            ))
            .mutating(writes)
        }
        "pesan" => Reply::text(format!(
            "⚠️ Perintah `pesan` telah usang dan digantikan oleh sistem acak.\n\
             Silakan gunakan `{}autosambut add <teks>` untuk menambahkan sapaan, \
             atau `{}autosambut list` untuk melihat daftar sapaan.",
            ctx.prefix, ctx.prefix
        ))
        .mutating(writes),
        _ => Reply::text(format!(
            "Perintah tidak valid. Coba ketik `{}autosambut` tanpa embel-embel untuk melihat buku panduannya.",
            ctx.prefix
        ))
        .mutating(writes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(group: Value, args: &[&str]) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "autosambut".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            is_owner: true,
            args: args.iter().map(|s| s.to_string()).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("autosambut").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    /// Semua tulis yang dikirim handler, urut. `None` = tidak menulis apa pun.
    fn writes(r: &Reply) -> Vec<Value> {
        r.mutations()
            .iter()
            .map(|m| match m {
                Mutation::SetGroup { jid, data } => {
                    assert_eq!(jid, "120363@g.us");
                    data["autoSambut"].clone()
                }
                other => panic!("harus SetGroup, dapat {other:?}"),
            })
            .collect()
    }

    /// Tulisan terakhir = keadaan akhir (tulis sebelumnya sudah tercakup di
    /// dalamnya, persis seperti `db.setGroup` berurutan di JS).
    fn final_write(r: &Reply) -> Value {
        writes(r).pop().expect("harus ada tulis")
    }

    fn with_pesan(pesan: &[&str]) -> Value {
        json!({"autoSambut": {"enabled": false, "delayMs": 1000, "pesanList": pesan}})
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("autosambut").unwrap();
        assert_eq!(p.meta.name, "autosambut");
        assert_eq!(p.meta.alias, ["sambutowner"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Mengatur fitur sambutan otomatis saat owner muncul setelah lama idle"
        );
        assert_eq!(p.meta.usage, ".autosambut on/off/delay/add/del/list");
        assert_eq!(p.meta.example, ".autosambut on");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner && p.meta.is_group);
        assert!(!p.meta.is_private && !p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn alias_resolves() {
        assert_eq!(oktz_plugin_api::find("sambutowner").unwrap().meta.name, "autosambut");
    }

    #[test]
    fn a_group_without_auto_sambut_is_seeded_then_toggled() {
        let w = writes(&run(json!({}), &["on"]));
        assert_eq!(w.len(), 2, "seed lalu aksi, seperti dua setGroup di JS");
        assert_eq!(w[0]["delayMs"], json!(DEFAULT_DELAY_MS));
        assert_eq!(w[0]["pesanList"], json!([DEFAULT_PESAN]));
        assert_eq!(w[1]["enabled"], json!(true));
        assert_eq!(final_write(&run(json!({}), &["on"]))["enabled"], json!(true));
    }

    #[test]
    fn no_action_shows_the_book_and_only_seeds() {
        let r = run(json!({}), &[]);
        let t = text_of(&r);
        assert!(t.starts_with("⚠️ *SISTEM AUTO SAMBUT*"), "{t}");
        assert!(t.contains("Status: *Nonaktif ❌*"), "{t}");
        assert!(t.contains("Batas Waktu Idle: *2 jam*"), "format 2 jam: {t}");
        assert!(t.contains("Jumlah Pesan Acak: *1 Sapaan*"), "{t}");
        assert!(t.contains("Contoh: *.autosambut delay 30m*"), "{t}");
        assert_eq!(writes(&r).len(), 1, "hanya seed");
    }

    #[test]
    fn on_and_off_toggle_the_stored_flag() {
        let r = run(with_pesan(&["a"]), &["ON"]);
        assert_eq!(text_of(&r), "✅ *Fitur Auto Sambut Diaktifkan!*");
        assert_eq!(final_write(&r)["enabled"], json!(true));

        let mut base = with_pesan(&["a"]);
        base["autoSambut"]["enabled"] = json!(true);
        let r = run(base, &["off"]);
        assert_eq!(text_of(&r), "❌ *Fitur Auto Sambut Dinonaktifkan.*");
        assert_eq!(final_write(&r)["enabled"], json!(false));
    }

    #[test]
    fn an_existing_group_is_not_reseeded() {
        let r = run(with_pesan(&["a"]), &[]);
        assert!(writes(&r).is_empty(), "sudah ada = tidak ada seed");
        assert_eq!(writes(&run(with_pesan(&["a"]), &["on"])).len(), 1, "hanya status");
    }

    #[test]
    fn the_legacy_single_pesan_key_is_migrated_into_the_list() {
        let g = json!({"autoSambut": {"enabled": true, "delayMs": 1000, "pesan": "halo"}});
        let w = writes(&run(g, &[]));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0]["pesanList"], json!(["halo"]));
        assert!(w[0].get("pesan").is_none(), "key lama harus dibuang: {w:?}");
    }

    #[test]
    fn an_legacy_pesan_does_not_overwrite_an_existing_list() {
        let g = json!({"autoSambut": {"delayMs": 1, "pesan": "lama", "pesanList": ["baru"]}});
        let w = writes(&run(g, &[]));
        assert_eq!(w[0]["pesanList"], json!(["baru"]));
        assert!(w[0].get("pesan").is_none());
    }

    #[test]
    fn delay_parses_the_six_units() {
        for (input, ms) in [
            ("30s", 30_000),
            ("30m", 1_800_000),
            ("2h", 7_200_000),
            ("3d", 259_200_000),
            ("1w", 604_800_000),
            ("1y", 31_536_000_000),
        ] {
            let r = run(json!({}), &["delay", input]);
            assert_eq!(final_write(&r)["delayMs"], json!(ms), "input {input}");
            assert!(
                text_of(&r).contains(&format!("selama *{}*", format_time(ms))),
                "input {input}: {}",
                text_of(&r)
            );
        }
    }

    #[test]
    fn delay_without_an_argument_or_with_garbage_is_refused() {
        let r = run(json!({}), &["delay"]);
        assert_eq!(text_of(&r), "Tolong berikan waktu! Contoh: `.autosambut delay 2h`");
        assert_eq!(writes(&r).len(), 1, "hanya seed");

        for bad in ["2", "h", "2x", "2 jam", "abc"] {
            let r = run(json!({}), &["delay", bad]);
            assert!(
                text_of(&r).starts_with("Format waktu tidak dikenali."),
                "input {bad}: {}",
                text_of(&r)
            );
            assert_eq!(final_write(&r)["delayMs"], json!(DEFAULT_DELAY_MS), "input {bad}");
        }
    }

    #[test]
    fn format_time_follows_the_js_unit_bands() {
        assert_eq!(format_time(30_000), "30 detik");
        assert_eq!(format_time(90_000), "1.5 menit");
        assert_eq!(format_time(7_200_000), "2 jam");
        assert_eq!(format_time(172_800_000), "2 hari");
        assert_eq!(format_time(604_800_000), "7 hari", ">= 1 minggu tetap 'hari'");
    }

    #[test]
    fn parse_time_matches_the_js_regex_and_unit_prefix() {
        assert_eq!(parse_time("1.5h"), Some(5_400_000));
        assert_eq!(parse_time("30M"), Some(1_800_000), "satuan case-insensitive");
        assert_eq!(parse_time("menit"), None, "huruf sebelum angka bukan unit");
        assert_eq!(parse_time("2 jam"), None, "spasi tidak diizinkan");
        assert_eq!(parse_time("2x"), None, "satuan tak dikenal");
        assert_eq!(parse_time("h2"), None);
    }

    #[test]
    fn list_prints_every_greeting_with_its_number() {
        let r = run(with_pesan(&["satu", "dua", "tiga"]), &["list"]);
        let t = text_of(&r);
        assert!(
            t.starts_with(
                "📜 *DAFTAR PESAN AUTO SAMBUT*\n\n\
                 Total ada *3* sapaan acak yang terdaftar di grup ini:\n\n"
            ),
            "{t}"
        );
        assert!(t.contains("*1.* satu\n\n*2.* dua\n\n*3.* tiga\n\n"), "{t}");
        assert!(
            t.ends_with("_Gunakan `.autosambut del <angka>` untuk menghapus salah satu._"),
            "{t}"
        );
        assert!(writes(&r).is_empty(), "list hanya membaca");
    }

    #[test]
    fn add_appends_the_joined_text_and_ignores_the_global_flag() {
        let r = run(with_pesan(&["lama"]), &["add", "halo", "dunia", "--global"]);
        assert_eq!(
            text_of(&r),
            "💬 *Pesan Baru Berhasil Ditambahkan!*\nKini ada 2 sapaan acak di dalam daftar."
        );
        assert_eq!(final_write(&r)["pesanList"], json!(["lama", "halo dunia"]));
    }

    #[test]
    fn add_without_text_asks_for_an_example() {
        let r = run(json!({}), &["add"]);
        assert_eq!(
            text_of(&r),
            "Tolong masukkan teks sambutan.\nContoh: `.autosambut add Halo bosku {user}!`"
        );
        assert_eq!(final_write(&r)["pesanList"], json!([DEFAULT_PESAN]), "hanya seed");
    }

    #[test]
    fn del_removes_by_one_based_index() {
        let r = run(with_pesan(&["a", "b", "c"]), &["del", "2"]);
        assert_eq!(
            text_of(&r),
            "🗑️ *Pesan Berhasil Dihapus!*\n\nTerhapus:\n\"b\"\nSisa daftar sapaan: 2"
        );
        assert_eq!(final_write(&r)["pesanList"], json!(["a", "c"]));
    }

    #[test]
    fn del_refuses_an_index_outside_the_list() {
        for arg in ["0", "3", "-1", "abc", "x"] {
            let r = run(with_pesan(&["a", "b"]), &["del", arg]);
            assert!(
                text_of(&r).starts_with("Tolong masukkan angka urutan pesan yang valid."),
                "arg {arg}: {}",
                text_of(&r)
            );
            assert!(writes(&r).is_empty(), "arg {arg}");
        }
    }

    #[test]
    fn the_last_greeting_cannot_be_deleted() {
        let r = run(with_pesan(&["satu"]), &["del", "1"]);
        assert_eq!(
            text_of(&r),
            "Gagal dihapus! Harus ada minimal 1 pesan di dalam daftar sapaan grup ini."
        );
        assert!(writes(&r).is_empty());
    }

    #[test]
    fn pesan_is_the_deprecated_command_and_unknown_words_are_rejected() {
        let r = run(json!({}), &["pesan"]);
        assert!(
            text_of(&r)
                .starts_with("⚠️ Perintah `pesan` telah usang dan digantikan oleh sistem acak."),
            "{}",
            text_of(&r)
        );
        assert!(text_of(&r).contains("`.autosambut add <teks>`"), "{}", text_of(&r));

        let r = run(json!({}), &["ngapain"]);
        assert_eq!(
            text_of(&r),
            "Perintah tidak valid. Coba ketik `.autosambut` tanpa embel-embel untuk melihat buku panduannya."
        );
    }

    #[test]
    fn an_empty_or_missing_pesan_list_falls_back_to_the_default_greeting() {
        // Fallback JS hanya mengubah objek di memori — tidak ada `setGroup`,
        // jadi balasan tampilannya benar sementara store tidak tersentuh.
        for g in [json!({}), json!({"autoSambut": {"delayMs": 1}})] {
            let r = run(g.clone(), &["list"]);
            assert!(
                text_of(&r).contains("*1.* Halo {user}!"),
                "input {g}: {}",
                text_of(&r)
            );
            let w = writes(&r);
            assert!(
                w.len() <= 1,
                "input {g}: seed hanya kalau autoSambut belum ada"
            );
            if let Some(last) = w.last() {
                assert_eq!(last["pesanList"], json!([DEFAULT_PESAN]), "input {g}");
            }
        }
    }

    #[test]
    fn a_group_that_already_has_an_empty_pesan_list_is_not_written_at_all() {
        let g = json!({"autoSambut": {"delayMs": 1, "pesanList": []}});
        let r = run(g, &["list"]);
        assert!(text_of(&r).contains("*1.* Halo {user}!"), "{}", text_of(&r));
        assert!(writes(&r).is_empty(), "fallback JS tidak menulis apa pun");
    }

    #[test]
    fn a_missing_delay_falls_back_to_two_hours() {
        let g = json!({"autoSambut": {"delayMs": 0, "pesanList": ["a"]}});
        assert!(text_of(&run(g, &[])).contains("Batas Waktu Idle: *2 jam*"));
    }

    #[test]
    fn the_written_payload_always_carries_a_fresh_last_chats_map() {
        let g = json!({"autoSambut": {"delayMs": 1, "lastChats": {"x": 1}, "pesanList": ["a"]}});
        assert_eq!(final_write(&run(g, &["on"]))["lastChats"], json!({}));
    }
}