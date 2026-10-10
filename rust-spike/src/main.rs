// oktz-spike — THROWAWAY spike untuk kelayakan rewrite Rust.
// Lihat docs/superpowers/plans/2026-10-10-rust-rewrite-spike-s1-s4.md
// Modul rss dipakai ulang oleh semua probe.

pub mod creds_map;
pub mod onigis_session;
pub mod rss;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("baseline");
    println!(
        "{}",
        serde_json::json!({"phase":"startup","cmd":cmd,"rss_kb":rss::current_kb()})
    );

    match cmd {
        "baseline" => {
            // Startup baseline sebelum load apa pun.
            println!("baseline ok");
        }
        "import" => {
            // Baca dump Turso fixture dan coba impor seluruh sesi.
            let path = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "fixtures/turso_session_keys.json".into());
            let raw =
                std::fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("baca {path}: {e}"))?;
            let dump: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(&raw).map_err(|e| anyhow::anyhow!("parse dump: {e}"))?;

            let sessions = dump
                .get("session")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let mut agg = onigis_session::ImportStats::default();
            let mut ok_records = 0usize;
            let mut failed_records = 0usize;
            let mut first_error: Option<String> = None;

            for row in &sessions {
                let Some(data) = row.get("data").and_then(|d| d.as_str()) else {
                    continue;
                };
                let mut stats = onigis_session::ImportStats::default();
                match onigis_session::import_session_record(data, &mut stats) {
                    Ok(_) => ok_records += 1,
                    Err(e) => {
                        failed_records += 1;
                        if first_error.is_none() {
                            first_error = Some(format!("{e:#}"));
                        }
                    }
                }
                agg.total_sessions += stats.total_sessions;
                agg.imported_sessions += stats.imported_sessions;
                agg.skipped_prekey_chains += stats.skipped_prekey_chains;
                agg.skipped_unknown_basekeyrole += stats.skipped_unknown_basekeyrole;
                agg.skipped_other += stats.skipped_other;
                for e in stats.errors.iter() {
                    if agg.errors.len() < 20 {
                        agg.errors.push(e.clone());
                    }
                }
            }

            println!(
                "{}",
                serde_json::json!({
                    "phase": "import",
                    "rows_total": sessions.len(),
                    "records_ok": ok_records,
                    "records_failed": failed_records,
                    "sessions_total": agg.total_sessions,
                    "sessions_imported": agg.imported_sessions,
                    "skipped_prekey_chains": agg.skipped_prekey_chains,
                    "skipped_unknown_basekeyrole": agg.skipped_unknown_basekeyrole,
                    "skipped_other": agg.skipped_other,
                    "rss_kb": rss::current_kb(),
                    "first_error": first_error,
                })
            );
            for e in &agg.errors {
                eprintln!("  ! {e}");
            }
        }
        "creds" => {
            // Pemetaan creds.json onigis -> Device. Jalan tanpa jaringan.
            let path = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| "../storage/session/creds.json".into());
            let raw =
                std::fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("baca {path}: {e}"))?;
            let (m, _c) = creds_map::inspect_creds(&raw)?;
            println!(
                "{}",
                serde_json::json!({
                    "phase": "creds",
                    "registration_id": m.registration_id,
                    "pn": m.pn,
                    "lid": m.lid,
                    "push_name": m.push_name,
                    "platform": m.platform,
                    "signed_pre_key_id": m.signed_pre_key_id,
                    "next_pre_key_id": m.next_pre_key_id,
                    "adv_secret_key_len": m.adv_secret_key_len,
                    "has_routing_info": m.has_routing_info,
                    "must_default": m.defaulted_fields,
                    "rss_kb": rss::current_kb(),
                })
            );
        }
        "rss" => {
            let hold: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(60);
            let mut t = tokio::time::interval(std::time::Duration::from_secs(5));
            let end = tokio::time::Instant::now() + std::time::Duration::from_secs(hold);
            while tokio::time::Instant::now() < end {
                t.tick().await;
                println!(
                    "{}",
                    serde_json::json!({"phase":"rss","rss_kb":rss::current_kb()})
                );
            }
        }
        other => {
            println!("unknown cmd: {other}");
        }
    }
    Ok(())
}
