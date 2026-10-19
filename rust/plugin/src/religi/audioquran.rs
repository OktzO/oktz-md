//! `plugins/religi/audioquran.js` — cari dan putar audio Al-Quran dari mp3quran.
//!
//! `NOTE (mod.rs):` `religi/mod.rs` sudah memuat `asmaulhusna` dan `islami`
//! (bukan milik tugas ini), jadi berkas ini sengaja tidak menambahkan
//! `pub mod`. Tambahkan baris berikut secara manual ke `src/religi/mod.rs`:
//!   pub mod audioquran;
//
//! Semua pemanggilan API lewat `ctx.http.get` dengan `terima_semua_status()`
//! (mirip `validateStatus: () => true` di axios). Header `Origin`/`Referer`
//! dibawa pada setiap permintaan.
use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

const API: &str = "https://www.mp3quran.net/api/v3";

fn mp3quran_url(path: &str, language: &str) -> String {
    if language.is_empty() {
        format!("{API}{path}")
    } else {
        format!("{API}{path}?language={language}")
    }
}

const ALIASES: &[(&str, &[&str])] = &[
    ("sudais", &["alsudaes", "sudaes", "abdulrahman alsudaes"]),
    ("mishary", &["meshary", "mishary alafasy", "alafasy", "alafasi"]),
    ("misyari", &["meshary", "alafasy"]),
    ("husary", &["hosary", "alhosary", "mahmoud khalil alhosary"]),
    ("hosary", &["alhosary", "husary"]),
    ("ghamdi", &["alghamdi", "saad alghamdi", "saad al ghamdi"]),
    ("ghamidi", &["alghamdi"]),
    ("shuraim", &["alshuraim", "saud alshuraim"]),
    ("shatri", &["shatry", "abubakr ashshatri", "abu bakr al shatri"]),
    ("ajmi", &["alajmi", "ahmed alajmi"]),
    ("minshawi", &["minshawy", "mohamed siddiq elminshawi"]),
    ("afasy", &["alafasy", "meshary alafasy"]),
    ("rifai", &["alrefaei", "hani rifai"]),
    ("muaiqly", &["maher almuaiqly", "almuaiqly"]),
    ("muaiqli", &["almuaiqly"]),
    ("juhany", &["aljohany", "abdullah aljohany"]),
    ("johany", &["aljohany"]),
    ("basfar", &["albasfar", "abdullah basfar"]),
    ("ayyub", &["mohammad ayyoub"]),
    ("ayyoub", &["mohammad ayyoub"]),
    ("budair", &["albudair", "salah albudair"]),
    ("tablawi", &["altablawi", "mohammed altablawi"]),
    ("thubaity", &["althubaity", "ibrahim althubaity"]),
];

/// Jarak Levenshtein antara dua string (mirip fungsi `lev` di JS).
fn levenshtein(a: &str, b: &str) -> usize {
    let m = a.len();
    let n = b.len();
    let mut dp: Vec<Vec<usize>> = vec![vec![0; n + 1]; m + 1];
    for (i, baris) in dp.iter_mut().enumerate() {
        baris[0] = i;
    }
    for (j, sel) in dp[0].iter_mut().enumerate() {
        *sel = j;
    }
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            dp[i][j] = std::cmp::min(
                dp[i - 1][j] + 1,
                std::cmp::min(dp[i][j - 1] + 1, dp[i - 1][j - 1] + cost),
            );
        }
    }
    dp[m][n]
}

#[plugin(
    name = "audioquran",
    alias = ["audio-quran", "murottal"],
    category = "religi",
    description = "Cari dan putar audio Al-Quran dari berbagai Qari (mp3quran)",
    usage = ".audio-quran [mode] [args]",
    example = ".audio-quran audio Sudais 1",
    cooldown = 15,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn audioquran(ctx: Ctx) -> Reply {
    if ctx.args.is_empty() {
        return Reply::text(
            "🕌 *AUDIO QURAN*\n\n\
             > Mode yang tersedia:\n\
             - `.audio-quran reciters` (List qari)\n\
             - `.audio-quran suwar` (List surah 1-114)\n\
             - `.audio-quran radios` (List radio live)\n\
             - `.audio-quran riwayat` (List bacaan)\n\
             - `.audio-quran audio <nama_qari> <nomor_surah>`\n\n\
             *Contoh:* `.audio-quran audio sudais 1`"
        );
    }

    let mode = ctx.args[0].to_lowercase();

    if mode == "audio" {
        if ctx.args.len() < 3 {
            return Reply::text("❌ Format salah! Contoh: `.audio-quran audio sudais 1`");
        }
        let surah_id_str = ctx.args.last().unwrap();
        let Ok(surah_id) = surah_id_str.parse::<i64>() else {
            return Reply::text("❌ Format salah! Contoh: `.audio-quran audio sudais 1`");
        };
        if !(1..=114).contains(&surah_id) {
            return Reply::text("❌ Surah harus antara 1-114.");
        }
        let reciter_query = ctx.args[1..ctx.args.len() - 1].join(" ").trim().to_lowercase();

        let input = serde_json::json!({ "mode": "audio", "reciter": reciter_query, "surah": surah_id, "language": "id" });
        let res = match mp3quran(&ctx, &input) {
            Ok(r) => r,
            Err(e) => return Reply::text(format!("❌ {e}")),
        };

        if !res.get("Status").and_then(Value::as_bool).unwrap_or(false) {
            let error = res.get("Error").and_then(Value::as_str).unwrap_or("kesalahan");
            return Reply::text(format!("❌ {error}"));
        }

        let r = res.get("Result").unwrap();
        let first_audio = r.get("firstAudio").and_then(Value::as_str).unwrap_or("");
        let reciter_name = r.get("audios")
            .and_then(|a| a.get(0).and_then(|v| v.get("reciterName").and_then(Value::as_str)))
            .unwrap_or("-");
        let moshaf_name = r.get("audios")
            .and_then(|a| a.get(0).and_then(|v| v.get("moshafName").and_then(Value::as_str)))
            .unwrap_or("-");

        let caption = format!(
            "🕌 *AUDIO QURAN*\n\n\
             *Qari:* {reciter_name}\n\
             *Moshaf:* {moshaf_name}\n\
             *Surah:* ke-{surah_id}\n\n\
             > Sedang mengirim audio...",
        );

        Reply::sequence(vec![
            Reply::text(caption),
            Reply::audio(
                oktz_plugin_api::MediaSource::url(first_audio.to_string()),
            ),
        ])
    } else {
        let input = serde_json::json!({ "mode": mode, "language": "id" });
        let res = match mp3quran(&ctx, &input) {
            Ok(r) => r,
            Err(e) => return Reply::text(format!("❌ {e}")),
        };

        if !res.get("Status").and_then(Value::as_bool).unwrap_or(false) {
            let error = res.get("Error").and_then(Value::as_str).unwrap_or("kesalahan");
            return Reply::text(format!("❌ {error}"));
        }

        let r = res.get("Result").unwrap();
        let message = r.get("message").and_then(Value::as_str).unwrap_or("");

        let mut txt = format!("🕌 *{message}*\n\n");

        if mode == "reciters" {
            let reciters = r.get("reciters").and_then(Value::as_array).cloned().unwrap_or_default();
            for q in reciters.iter().take(50) {
                let id = q.get("id").and_then(Value::as_i64).unwrap_or(0);
                let name = q.get("name").and_then(Value::as_str).unwrap_or("-");
                let moshaf_count = q.get("moshafCount").and_then(Value::as_i64).unwrap_or(0);
                txt.push_str(&format!("- #{id} *{name}* ({moshaf_count} riwayat)\n"));
            }
            let total = r.get("count").and_then(Value::as_i64).unwrap_or(0);
            if total > 50 {
                txt.push_str(&format!("\n> ... +{} qari lainnya.", total - 50));
            }
        } else if mode == "suwar" {
            let suwar = r.get("suwar").and_then(Value::as_array).cloned().unwrap_or_default();
            for s in suwar {
                let id = s.get("id").and_then(Value::as_i64).unwrap_or(0);
                let name = s.get("name").and_then(Value::as_str).unwrap_or("-");
                let typ = s.get("type").and_then(Value::as_str).unwrap_or("-");
                txt.push_str(&format!("- #{id} *{name}* ({typ})\n"));
            }
        } else if mode == "riwayat" {
            let riwayat = r.get("riwayat").and_then(Value::as_array).cloned().unwrap_or_default();
            for rw in riwayat {
                let id = rw.get("id").and_then(Value::as_i64).unwrap_or(0);
                let name = rw.get("name").and_then(Value::as_str).unwrap_or("-");
                txt.push_str(&format!("- #{id} *{name}*\n"));
            }
        } else if mode == "radios" {
            let radios = r.get("radios").and_then(Value::as_array).cloned().unwrap_or_default();
            for rd in radios.iter().take(50) {
                let id = rd.get("id").and_then(Value::as_i64).unwrap_or(0);
                let name = rd.get("name").and_then(Value::as_str).unwrap_or("-");
                let url = rd.get("url").and_then(Value::as_str).unwrap_or("-");
                txt.push_str(&format!("- #{id} *{name}*\n  Link: {url}\n"));
            }
        } else {
            return Reply::text(format!(
                "❌ Mode \"{mode}\" tidak dikenal. Pakai: reciters | suwar | riwayat | radios | audio"
            ));
        }

        Reply::text(txt)
    }
}

fn mp3quran(ctx: &Ctx, input: &Value) -> Result<Value, String> {
    let mode = input.get("mode").and_then(Value::as_str).unwrap_or("");

    if mode == "reciters" || mode == "suwar" || mode == "riwayat" || mode == "radios" {
        let language = input.get("language").and_then(Value::as_str).unwrap_or("eng");
        let url = mp3quran_url(&format!("/{mode}"), language);
        let Ok(res) = ctx.http
            .get(&url)
            .map_err(|e| format!("HTTP error: {}", e.message)) else {
            return Err("Gagal terhubung ke mp3quran".to_string());
        };
        let body = res.json().unwrap_or(Value::Null);
        let list = body.get("reciters").or_else(|| body.get("suwar"))
            .or_else(|| body.get("riwayat")).or_else(|| body.get("radios"));
        let mut result = serde_json::Map::new();
        result.insert("message".to_string(), Value::String("OK".to_string()));
        result.insert("count".to_string(), Value::from(0));
        result.insert(mode.to_string(), list.cloned().unwrap_or(Value::Null));
        Ok(serde_json::json!({
            "Status": true,
            "Code": 200,
            "Result": serde_json::Value::Object(result)
        }))
    } else if mode == "audio" {
        let reciter_query = input.get("reciter").and_then(Value::as_str).unwrap_or("").to_lowercase();
        let surah_id = input.get("surah").and_then(Value::as_i64).unwrap_or(0);

        let language = "id";
        let url = mp3quran_url("/reciters", language);
        let Ok(res) = ctx.http
            .get(&url)
            .map_err(|e| format!("HTTP error: {}", e.message)) else {
            return Err("Gagal terhubung ke mp3quran".to_string());
        };
        let body = res.json().unwrap_or(Value::Null);
        let all = body.get("reciters").and_then(Value::as_array).cloned().unwrap_or_default();

        let mut matches: Vec<&Value> = all.iter()
            .filter(|r| r.get("name").and_then(Value::as_str).unwrap_or("").to_lowercase().contains(&reciter_query))
            .collect();

        if matches.is_empty() {
            let mut expanded: Vec<String> = vec![reciter_query.clone()];
            if let Some(alias_list) = ALIASES.iter().find(|(k, _)| *k == reciter_query).map(|(_, v)| *v) {
                for a in alias_list {
                    expanded.push(a.to_string());
                }
            }
            for word in reciter_query.split_whitespace() {
                if let Some(alias_list) = ALIASES.iter().find(|(k, _)| *k == word).map(|(_, v)| *v) {
                    for a in alias_list {
                        expanded.push(a.to_string());
                    }
                }
            }
            for variant in &expanded {
                let found: Vec<&Value> = all.iter()
                    .filter(|r| r.get("name").and_then(Value::as_str).unwrap_or("").to_lowercase().contains(variant))
                    .collect();
                if !found.is_empty() {
                    matches = found;
                    break;
                }
            }
        }

        if matches.is_empty() {
            let threshold = if reciter_query.len() <= 4 { 1 } else { 2 };
            let mut fuzzy_hits: Vec<(&Value, usize)> = Vec::new();
            for r in &all {
                let name = r.get("name").and_then(Value::as_str).unwrap_or("");
                let tokens: Vec<&str> = name.split([' ', '-', '\'']).filter(|s| !s.is_empty()).collect();
                for tok in tokens {
                    let d = levenshtein(&reciter_query, tok);
                    if d <= threshold {
                        fuzzy_hits.push((r, d));
                        break;
                    }
                }
            }
            fuzzy_hits.sort_by_key(|(_, d)| *d);
            matches = fuzzy_hits.into_iter().take(10).map(|(r, _)| r).collect();
        }

        if matches.is_empty() {
            let mut ranked: Vec<(String, usize)> = all.iter().map(|r| {
                let name = r.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                let tokens: Vec<&str> = name.split([' ', '-', '\'']).filter(|s| !s.is_empty()).collect();
                let best = tokens.iter().map(|tok| levenshtein(&reciter_query, tok)).min().unwrap_or(usize::MAX);
                (name, best)
            }).collect();
            ranked.sort_by_key(|(_, d)| *d);
            ranked.truncate(5);
            let suggestions: Vec<String> = ranked.iter().map(|(n, _)| n.clone()).collect();
            return Ok(serde_json::json!({
                "Status": false,
                "Code": 404,
                "Error": format!("Qari \"{}\" gak ketemu. Suggestion terdekat: {}. Atau mode \"reciters\" buat list lengkap.", reciter_query, suggestions.join(", "))
            }));
        }

        let mut audios: Vec<Value> = Vec::new();
        for reciter in &matches {
            let moshaf_list = reciter.get("moshaf").and_then(Value::as_array).cloned().unwrap_or_default();
            for moshaf in moshaf_list {
                let surah_list_str = moshaf.get("surah_list").and_then(Value::as_str).unwrap_or("");
                let surah_list: Vec<i64> = surah_list_str.split(',')
                    .filter_map(|n| n.trim().parse::<i64>().ok())
                    .collect();
                if !surah_list.contains(&surah_id) {
                    continue;
                }
                let server = moshaf.get("server").and_then(Value::as_str).unwrap_or("");
                let audio_url = build_audio_url(server, surah_id);
                audios.push(serde_json::json!({
                    "reciterId": reciter.get("id"),
                    "reciterName": reciter.get("name"),
                    "moshafId": moshaf.get("id"),
                    "moshafName": moshaf.get("name"),
                    "surahId": surah_id,
                    "audioUrl": audio_url,
                }));
            }
        }

        if audios.is_empty() {
            let match_count = matches.len();
            return Ok(serde_json::json!({
                "Status": false,
                "Code": 404,
                "Error": format!("Qari ketemu ({match_count}× match) tapi surah {surah_id} gak tersedia di moshaf-nya.")
            }));
        }

        let first_audio = audios[0].get("audioUrl").and_then(Value::as_str).unwrap_or("").to_string();
        Ok(serde_json::json!({
            "Status": true,
            "Code": 200,
            "Result": {
                "message": format!("🎵 {} audio surah {surah_id} dari {} qari match \"{reciter_query}\"", audios.len(), matches.len()),
                "matchCount": matches.len(),
                "audioCount": audios.len(),
                "firstAudio": first_audio,
                "audios": audios,
            }
        }))
    } else {
        Ok(serde_json::json!({
            "Status": false,
            "Code": 400,
            "Error": format!("Unknown mode \"{mode}\". Pakai: reciters | suwar | riwayat | radios | audio")
        }))
    }
}

fn build_audio_url(server_base: &str, surah_id: i64) -> String {
    let padded = format!("{surah_id:03}");
    let base = if server_base.ends_with('/') { server_base.to_string() } else { format!("{server_base}/") };
    format!("{base}{padded}.mp3")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_mirip_js() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("abc", "abc"), 0);
        // `plugins/religi/audioquran.js:167-184`: cuma i→e di posisi kelima → 1.
        assert_eq!(levenshtein("sudais", "sudaes"), 1);
    }

    #[test]
    fn alias_ditemukan() {
        assert!(ALIASES.iter().any(|(k, _)| *k == "sudais"));
        assert!(ALIASES.iter().any(|(k, _)| *k == "mishary"));
    }

    #[test]
    fn build_audio_url_padding() {
        assert_eq!(
            build_audio_url("https://server.com", 1),
            "https://server.com/001.mp3"
        );
        assert_eq!(
            build_audio_url("https://server.com/", 114),
            "https://server.com/114.mp3"
        );
    }
}
