// oktz-spike — THROWAWAY spike untuk kelayakan rewrite Rust.
// Lihat docs/superpowers/plans/2026-10-10-rust-rewrite-spike-s1-s4.md
// Modul rss dipakai ulang oleh semua probe.

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
    println!("{}", serde_json::json!({"phase":"startup","cmd":cmd,"rss_kb":rss::current_kb()}));

    match cmd {
        "baseline" => {
            // Startup baseline sebelum load apa pun.
            println!("baseline ok");
        }
        "rss" => {
            let hold: u64 = args
                .get(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(60);
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