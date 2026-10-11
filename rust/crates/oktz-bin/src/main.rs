use std::path::PathBuf;

use oktz_core::config::Config;
use oktz_store::json_store::JsonStore;
use oktz_wa::MessageBuilderExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    oktz_core::logging::init("info");

    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("boot");

    let cfg = Config::from_env();

    if mode == "verify-db" {
        let dir = args
            .get(2)
            .cloned()
            .unwrap_or_else(|| cfg.database.path.clone());
        return verify_db(&dir);
    }

    if mode.starts_with("send-") {
        let creds = args
            .get(2)
            .cloned()
            .unwrap_or_else(|| "../storage/session/creds.json".into());
        let to = args.get(3).cloned().unwrap_or_default();
        let hold: u64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(60);
        let kind = mode.trim_start_matches("send-").to_string();
        return send(&creds, &to, hold, &kind).await;
    }

    if mode == "smoke" {
        let creds = args
            .get(2)
            .cloned()
            .unwrap_or_else(|| "../storage/session/creds.json".into());
        let fixture = args
            .get(3)
            .cloned()
            .unwrap_or_else(|| "fixtures/turso_session_keys.json".into());
        let hold: u64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(90);
        return smoke(&creds, &fixture, hold).await;
    }

    let store_path = PathBuf::from(&cfg.database.path).join("settings.json");
    let store = JsonStore::open(&store_path)?;
    tracing::info!(
        path = %store.path().display(),
        keys = store.all().as_object().map(serde_json::Map::len).unwrap_or(0),
        turso = cfg.turso.enabled,
        owner = cfg.owner_numbers.len(),
        "store dibuka"
    );

    let plugins = oktz_plugin_api::all_plugins();
    tracing::info!(count = plugins.len(), "plugin dimuat");
    for p in plugins {
        tracing::debug!(name = %p.meta.name, category = %p.meta.category, "plugin");
    }

    println!(
        "{}",
        serde_json::json!({
            "mode": mode,
            "database": store.path().display().to_string(),
            "plugins": plugins.len(),
            "owner": cfg.owner_numbers.len(),
            "turso": cfg.turso.enabled,
        })
    );
    Ok(())
}

/// Baca tiap file JSON di folder DB, parse lalu serialize ulang, bandingkan
/// byte. differing byte = risiko kehilangan data user.
fn verify_db(dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(dir);
    if !path.is_dir() {
        println!("{}", serde_json::json!({"db": dir, "exists": false}));
        return Ok(());
    }
    let mut results = Vec::new();
    fn walk(dir: &std::path::Path, out: &mut Vec<serde_json::Value>) -> std::io::Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out)?;
                continue;
            }
            if p.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let Ok(raw) = std::fs::read_to_string(&p) else {
                continue;
            };
            let (identical, detail) = match oktz_store::lowdb::roundtrip_bytes(&raw) {
                Ok(s) if s == raw => (true, String::new()),
                Ok(s) => {
                    let at = raw
                        .bytes()
                        .zip(s.bytes())
                        .position(|(a, b)| a != b)
                        .unwrap_or(raw.len().min(s.len()));
                    (
                        false,
                        format!("beda mulai byte {at}; len {} vs {}", raw.len(), s.len()),
                    )
                }
                Err(e) => (false, format!("parse gagal: {e}")),
            };
            out.push(serde_json::json!({
                "file": p.display().to_string(),
                "bytes": raw.len(),
                "identical": identical,
                "detail": detail,
            }));
        }
        Ok(())
    }
    walk(&path, &mut results)?;
    // Path yang bukan direktori menghasilkan vektor kosong, dan `all()` pada
    // vektor kosong = true (false-green). Pastikan ada filenya.
    let all_ok = !results.is_empty() && results.iter().all(|r| r["identical"] == true);
    println!(
        "{}",
        serde_json::json!({
            "db": dir,
            "files": results.len(),
            "all_identical": all_ok,
            "results": results,
        })
    );
    if !all_ok {
        std::process::exit(1);
    }
    Ok(())
}

/// Smoke produksi: import device + sesi ratchet, connect, print pesan masuk.
async fn smoke(
    creds: &str,
    fixture: &str,
    hold_secs: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = oktz_wa::prepare_store(creds, "/tmp/opencode/oktz-smoke.db", Some(fixture)).await?;

    let handle = oktz_wa::connect_device(store, move |_client| async move {
        println!(
            "{}",
            serde_json::json!({"phase":"CONNECTED","rss_kb":oktz_wa::rss::current_kb()})
        );
    })
    .await?;

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(hold_secs);
    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
    println!(
        "{}",
        serde_json::json!({"phase":"done","rss_kb":oktz_wa::rss::current_kb()})
    );
    handle.shutdown().await;
    Ok(())
}

/// Kirim satu pesan sesuai jenis: text | card | list.
async fn send(
    creds: &str,
    to: &str,
    hold_secs: u64,
    kind: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = oktz_wa::prepare_store(creds, "/tmp/opencode/oktz-send.db", None).await?;
    let to = to.to_string();
    let kind = kind.to_string();

    let handle = oktz_wa::connect_device(store, move |client| {
        let to = to.clone();
        let kind = kind.clone();
        async move {
            let jid: oktz_wa::Jid = match to.parse() {
                Ok(j) => j,
                Err(e) => {
                    println!(
                        "{}",
                        serde_json::json!({"phase":"ERR","msg":format!("JID: {e}")})
                    );
                    return;
                }
            };
            let opts =
                oktz_wa::SendOptions::default().with_extra_stanza_nodes(vec![oktz_wa::biz_node()]);

            let msg = match kind.as_str() {
                "card" => oktz_wa::build_card("Kartu dari produksi Rust", Some("footer produksi")),
                "list" => oktz_wa::build_list(),
                _ => oktz_wa::Message::text("Halo dari produksi Rust"),
            };

            let res = if kind == "text" {
                client
                    .send_message(jid.clone(), msg)
                    .await
                    .map(|r| r.message_id)
            } else {
                client
                    .send_message_with_options(jid.clone(), msg, opts)
                    .await
                    .map(|r| r.message_id)
            };

            println!(
                "{}",
                serde_json::json!({
                    "phase": if res.is_ok() { "SENT" } else { "ERR" },
                    "kind": kind,
                    "to": jid.to_string(),
                    "message_id": res.as_ref().ok().map(|s| s.to_string()),
                    "error": res.as_ref().err().map(|e| e.to_string()),
                    "rss_kb": oktz_wa::rss::current_kb(),
                })
            );
        }
    })
    .await?;

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(hold_secs.max(20));
    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    handle.shutdown().await;
    Ok(())
}
