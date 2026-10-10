use std::path::PathBuf;

use oktz_core::config::Config;
use oktz_store::json_store::JsonStore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    oktz_core::logging::init("info");

    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("boot");

    let cfg = Config::from_env();

    if mode == "verify-db" {
        return verify_db(&cfg.database.path);
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
    let mut entries: Vec<_> = std::fs::read_dir(&path)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&p) else {
            continue;
        };
        let out = oktz_store::lowdb::roundtrip_bytes(&raw);
        let (identical, detail) = match out {
            Ok(s) => {
                let ok = s == raw;
                (
                    ok,
                    if ok {
                        String::new()
                    } else {
                        let first_diff = raw
                            .bytes()
                            .zip(s.bytes())
                            .position(|(a, b)| a != b)
                            .unwrap_or(raw.len().min(s.len()));
                        format!(
                            "beda mulai byte {first_diff}; len {} vs {}",
                            raw.len(),
                            s.len()
                        )
                    },
                )
            }
            Err(e) => (false, format!("parse gagal: {e}")),
        };
        results.push(serde_json::json!({
            "file": p.file_name().unwrap_or_default().to_string_lossy(),
            "bytes": raw.len(),
            "identical": identical,
            "detail": detail,
        }));
    }
    let all_ok = results.iter().all(|r| r["identical"] == true);
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
