//! `plugins/tools/lookup.js` — DNS lookup untuk domain.
//!
//! `pub mod lookup;` perlu ditambah manual ke `src/tools/mod.rs` (mod.rs
//! berisi plugin lain, bukan milik saya — lihat PORT-BRIEF §6).
//!
//! Pakai `ctx.http.get` ke api.hackertarget.com (DNS + whois), lalu render
//! teks. Tidak ada `sendToolsPreview` (I/O WhatsApp di `src/lib/context.js`),
//! jadi balasan adalah `Reply::text` polos — deviasi dari JS yang memanggil
//! `sendToolsPreview`, yang butuh `sock`.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `^(https?:\/\/)?(www\.)?` dihilangkan, lalu ambil segmen sebelum `/`.
fn bersihkan_domain(input: &str) -> String {
    let tanpa_scheme = input
        .strip_prefix("http://")
        .or_else(|| input.strip_prefix("https://"))
        .unwrap_or(input);
    let tanpa_www = tanpa_scheme.strip_prefix("www.").unwrap_or(tanpa_scheme);
    tanpa_www.split('/').next().unwrap_or(tanpa_www).to_string()
}

/// Padanan regex JS
/// `/^[a-zA-Z0-9][a-zA-Z0-9-]{0,61}[a-zA-Z0-9]?(\.[a-zA-Z]{2,})+$/`
/// tanpa dependensi `regex`.
fn valid_domain(domain: &str) -> bool {
    if domain.is_empty() {
        return false;
    }
    let bagian: Vec<&str> = domain.split('.').collect();
    if bagian.len() < 2 {
        return false;
    }
    for part in &bagian {
        if part.is_empty() || part.len() > 63 {
            return false;
        }
        if part.starts_with('-') || part.ends_with('-') {
            return false;
        }
        for c in part.chars() {
            if !c.is_ascii_alphanumeric() && c != '-' {
                return false;
            }
        }
    }
    let tld = bagian.last().unwrap();
    if tld.len() < 2 {
        return false;
    }
    tld.chars().all(|c| c.is_ascii_alphanumeric())
}

#[plugin(
    name = "lookup",
    alias = ["dnslookup", "dns", "whois"],
    category = "tools",
    description = "DNS Lookup untuk domain",
    usage = ".lookup <domain>",
    example = ".lookup google.com",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn lookup(ctx: Ctx) -> Reply {
    let domain = match ctx.args.first() {
        Some(d) => bersihkan_domain(d),
        None => return Reply::text(format_usage(&ctx.prefix)),
    };

    if !valid_domain(&domain) {
        return Reply::text(
            "❌ *ғᴏʀᴍᴀᴛ ᴛɪᴅᴀᴋ ᴠᴀʟɪᴅ*\n\n> Contoh: `google.com`",
        );
    }

    let dns_url = format!("https://api.hackertarget.com/dnslookup/?q={domain}");
    let whois_url = format!("https://api.hackertarget.com/whois/?q={domain}");

    let dns_res = ctx.http.get(&dns_url);
    let whois_res = ctx.http.get(&whois_url);

    let dns_ok = dns_res.as_ref().map_or(false, |r| r.status == 200);
    let whois_ok = whois_res.as_ref().map_or(false, |r| r.status == 200);

    let dns_text = dns_res.ok().map(|r| r.text());
    let whois_text = whois_res.ok().map(|r| r.text());

    let dns_data = dns_ok.and_then(dns_text.as_deref());
    let whois_data = whois_ok.and_then(whois_text.as_deref());

    if dns_data.is_none() && whois_data.is_none() {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Tidak dapat memproses domain");
    }

    let mut text = format!("🔍 *ᴅɴs ʟᴏᴏᴋᴜᴘ*\n\n> Domain: `{domain}`\n\n");

    if let Some(dns) = dns_data.filter(|d| !d.contains("error")) {
        let mut records: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        for line in dns.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let tipe = parts[parts.len() - 2];
                let nilai = parts[parts.len() - 1];
                records.entry(tipe.to_string()).or_default().push(nilai.to_string());
            }
        }
        text.push_str("╭┈┈⬡「 📋 *ᴅɴs ʀᴇᴄᴏʀᴅs* 」\n");
        if let Some(a) = records.get("A") {
            text.push_str(&format!(
                "┃ 🅰️ A: {}\n",
                a.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
            ));
        }
        if let Some(aaaa) = records.get("AAAA") {
            text.push_str(&format!(
                "┃ 🔢 AAAA: {}\n",
                aaaa.iter().take(2).cloned().collect::<Vec<_>>().join(", ")
            ));
        }
        if let Some(mx) = records.get("MX") {
            text.push_str(&format!(
                "┃ 📧 MX: {}\n",
                mx.iter().take(2).cloned().collect::<Vec<_>>().join(", ")
            ));
        }
        if let Some(ns) = records.get("NS") {
            text.push_str(&format!(
                "┃ 🌐 NS: {}\n",
                ns.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
            ));
        }
        if let Some(txt) = records.get("TXT") {
            text.push_str(&format!("┃ 📝 TXT: {} records\n", txt.len()));
        }
        text.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    }

    if let Some(whois) = whois_data.filter(|w| !w.contains("error") && w.len() < 2000) {
        let ambil = |awal: &str| {
            whois
                .lines()
                .find_map(|l| {
                    let l = l.trim();
                    l.strip_prefix(awal).map(|v| v.trim().to_string())
                })
                .unwrap_or_else(|| "-".to_string())
        };
        let registrar = ambil("Registrar:");
        let created = ambil("Creation Date:");
        let expires = whois
            .lines()
            .find_map(|l| {
                let l = l.trim();
                l.strip_prefix("Expir").and_then(|v| v.split(':').nth(1)).map(|v| v.trim().to_string())
            })
            .unwrap_or_else(|| "-".to_string());
        let ns: Vec<String> = whois
            .lines()
            .filter(|l| l.trim().starts_with("Name Server:"))
            .filter_map(|l| l.trim().split(':').nth(1).map(|v| v.trim().to_string()))
            .take(2)
            .collect();

        text.push_str("╭┈┈⬡「 📄 *ᴡʜᴏɪs* 」\n");
        text.push_str(&format!(
            "┃ 🏢 Registrar: {}\n",
            &registrar[..35.min(registrar.len())]
        ));
        text.push_str(&format!(
            "┃ 📅 Created: {}\n",
            &created[..20.min(created.len())]
        ));
        text.push_str(&format!(
            "┃ ⏰ Expires: {}\n",
            &expires[..20.min(expires.len())]
        ));
        if !ns.is_empty() {
            text.push_str(&format!("┃ 🌐 NS: {}\n", ns.join(", ")));
        }
        text.push_str("╰┈┈┈┈┈┈┈┈⬡");
    }

    Reply::text(text)
}

fn format_usage(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
         > `{prefix}lookup <domain>`\n\n\
         > Contoh:\n\
         > `{prefix}lookup google.com`",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bersihkan_domain_hapus_scheme_dan_www() {
        assert_eq!(bersihkan_domain("https://www.google.com"), "google.com");
        assert_eq!(bersihkan_domain("http://google.com"), "google.com");
        assert_eq!(bersihkan_domain("www.google.com"), "google.com");
        assert_eq!(bersihkan_domain("google.com"), "google.com");
        assert_eq!(
            bersihkan_domain("https://sub.domain.co.id/path"),
            "sub.domain.co.id"
        );
    }

    #[test]
    fn valid_domain_terima_domain_bener() {
        assert!(valid_domain("google.com"));
        assert!(valid_domain("sub.domain.co.id"));
        assert!(valid_domain("a-b.c"));
    }

    #[test]
    fn valid_domain_tolak_domain_salah() {
        assert!(!valid_domain(""));
        assert!(!valid_domain("google"));
        assert!(!valid_domain("-google.com"));
        assert!(!valid_domain("google-.com"));
        assert!(!valid_domain("google.com."));
        assert!(!valid_domain("google..com"));
    }
}
