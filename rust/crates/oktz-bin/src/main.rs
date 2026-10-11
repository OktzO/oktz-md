use std::path::PathBuf;

use oktz_core::config::Config;
use oktz_store::json_store::JsonStore;
use oktz_wa::MessageBuilderExt;

/// Referensi eksplisit ke crate plugin.
///
/// Rust membuang crate yang tidak dirujuk, sehingga statik `linkme`-nya ikut
/// hilang dan registry kosong. Fungsi ini yang menjaga tautan itu hidup.
fn use_plugin_crates() {
    // PENTANGI: tambah crate plugin baru di sini juga.
    let _ = oktz_plugins::cek::cekbaik;
}

/// Env yang mengarahkan SEMUA default db runtime (pairing, smoke, send).
///
/// Dulu semua default-nya di-hardcode ke `/tmp/opencode/...`, yaitu direktori
/// scratch yang bisa dihapus kapan saja: bot produksi yang kehilangan db
/// berarti kehilangan device row dan sesi ratchet. Env ini jadi tempat
/// memindai lokasi data untuk CI atau install yang tidak memakai default.
const DB_DIR_ENV: &str = "OKTZ_DB_DIR";

/// Root db yang harus bertahan antar-restart (pairing).
///
/// Default-nya ada di `database/`, folder data proyek yang juga dipakai
/// `Config::database.path` dan sudah masuk .gitignore — bukan temp dir.
fn data_dir(env: Option<&str>) -> PathBuf {
    match env.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => PathBuf::from(v),
        None => PathBuf::from("./database/wa"),
    }
}

/// Root db sekali pakai untuk `smoke` / `send`.
///
/// Bedanya sengaja: dua subcommand itu selalu membangun db dari nol dari
/// creds + fixture (`prepare_store` menghapus file lama sebelum menulis),
/// jadi isinya disposable dan tidak pernah dibaca lagi. Temp dir +
/// nama per-pid (lihat [`dev_db`]) supaya dua run paralel tidak saling
/// menghapus. Tetap bisa dipindah ke direktori nyata lewat [`DB_DIR_ENV`].
fn dev_dir(env: Option<&str>) -> PathBuf {
    match env.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => PathBuf::from(v),
        None => std::env::temp_dir(),
    }
}

/// File db dev. `pid` masuk nama supaya dua proses smoke/send bersamaan
/// tidak berebut file yang sama.
fn dev_db(dir: &std::path::Path, kind: &str, pid: u32) -> PathBuf {
    dir.join(format!("oktz-{kind}-{pid}.db"))
}

/// Path final satu db: argumen eksplisit di CLI selalu menang atas default.
fn resolve_db(explicit: Option<&str>, fallback: PathBuf) -> PathBuf {
    explicit
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or(fallback)
}

/// `prepare_store` menerima `&str`, jadi path non-UTF8 harus jadi error.
/// `unwrap_or_default()` di sini menghasilkan `""` — sqlite lalu mencoba
/// membuka path relatif kosong, yang errornya jauh lebih membingungkan
/// daripada "path db bukan utf-8".
fn db_arg(path: PathBuf) -> Result<String, std::io::Error> {
    match path.into_os_string().into_string() {
        Ok(s) => Ok(s),
        Err(os) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("path db bukan utf-8: {os:?}"),
        )),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    oktz_core::logging::init("info");

    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("boot");

    let cfg = Config::from_env();
    let db_dir_env = oktz_core::env::Env::get(DB_DIR_ENV);

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
        let db = db_arg(dev_db(&dev_dir(db_dir_env.as_deref()), "send", std::process::id()))?;
        return send(&creds, &to, hold, &kind, &db).await;
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
        let db = db_arg(dev_db(&dev_dir(db_dir_env.as_deref()), "smoke", std::process::id()))?;
        return smoke(&creds, &fixture, hold, &db).await;
    }

    // `oktz run [creds] [settings.json] [hold_secs]` — jalur produksi: connect
    // dengan pipeline router terpasang, jadi command masuk benar-benar mencapai
    // plugin. smoke/send sengaja lewat pairing_run langsung dan tidak memakai
    // pipeline.
    if mode == "run" {
        let creds = args
            .get(2)
            .cloned()
            .unwrap_or_else(|| "../storage/session/creds.json".into());
        let settings = args.get(3).cloned();
        let hold: u64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(3600);
        return run(&creds, settings.as_deref(), hold).await;
    }

    // `oktz pair-code <nomor> [db] [hold]` / `oktz pair-qr [db] [hold]`
    if mode == "pair-code" || mode == "pair-qr" {
        // Device row di db pairing selalu dibuat baru, jadi path default-nya
        // tidak boleh menimpa db bot yang sedang dipakai — makanya nama file
        // terpisah dari db smoke/send.
        let db = db_arg(resolve_db(
            args.get(if mode == "pair-code" { 3 } else { 2 }).map(|s| s.as_str()),
            data_dir(db_dir_env.as_deref()).join("oktz-pair.db"),
        ))?;
        let hold: u64 = args
            .get(if mode == "pair-code" { 4 } else { 3 })
            .and_then(|s| s.parse().ok())
            .unwrap_or(300);

        let out = if mode == "pair-code" {
            let Some(nomor) = args.get(2) else {
                eprintln!("pemakaian: oktz pair-code <nomor> [db] [hold_secs]");
                std::process::exit(2);
            };
            oktz_wa::pairing_run::pair_code_flow(&db, nomor, hold).await
        } else {
            oktz_wa::pairing_run::pair_qr_flow(&db, hold).await
        };

        // Hasil akhir dicetak sebagai satu baris JSON; kegagalan tidak pernah
        // keluar dengan exit 0 supaya tidak dipakai script sebagai "berhasil".
        return match out {
            Ok(v) => {
                println!("{v}");
                Ok(())
            }
            Err(e) => {
                let line = serde_json::json!({
                    "phase": "FAILED",
                    "mode": mode,
                    "db": db,
                    "error": format!("{e:#}"),
                });
                eprintln!("{line}");
                std::process::exit(1);
            }
        };
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

    // PENTING: crate plugin hanya ter-link kalau dirujuk di sini. Rust
    // membuang dependency yang tak terpakai, dan `linkme` mendaftarkan lewat
    // statik — kalau crate-nya hilang, `all_plugins()` melaporkan 0 tanpa error.
    // Setiap crate plugin baru wajib ditambah ke daftar ini.
    use_plugin_crates();

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

/// Creds untuk `FalseLogoutGuard` — dipakai bersama oleh smoke dan send.
///
/// `load_guard_creds` sudah aman dipanggil dengan path eksplisit: file yang
/// tidak terbaca menghasilkan `Value::Null`, bukan error.
fn creds_for_guard(path: &str) -> serde_json::Value {
    oktz_wa::bot::load_guard_creds(Some(path))
}

/// Jalur produksi: connect lewat `connect_device` sehingga pipeline router
/// terpasang dan command masuk benar-benar mencapai plugin.
///
/// `settings.json` dibaca pipeline sebagai store. Kalau tidak ada, pipeline
/// jalan dengan default (semua mode public, semua group `all`) — bot tetap
/// merespons, hanya gating per-grup yang tidak berlaku.
async fn run(
    creds: &str,
    settings: Option<&str>,
    hold_secs: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = db_arg(resolve_db(
        None,
        data_dir(Some("/var/lib/oktz")).join("oktz-run.db"),
    ))?;
    let store = oktz_wa::prepare_store(creds, &db, None).await?;

    // Pipeline membaca lokasi settings lewat env saat connect. Argumen harus
    // dipasang SEBELUM connect_device, makanya di sini bukan di dalam closure.
    if let Some(p) = settings {
        // Aman: masih single-threaded startup, sebelum task apa pun dibuat.
        unsafe { std::env::set_var(oktz_wa::bot::SETTINGS_ENV, p) };
        tracing::info!(path = %p, "settings dari argumen");
    }

    let handle = oktz_wa::connect_device(store, |client| async move {
        println!(
            "{}",
            serde_json::json!({"phase":"CONNECTED","pipeline":true,"rss_kb":oktz_wa::rss::current_kb()})
        );
        let _ = &client;
    })
    .await?;

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(hold_secs);
    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
    println!(
        "{}",
        serde_json::json!({"phase":"done","rss_kb":oktz_wa::rss::current_kb()})
    );
    handle.shutdown().await;
    Ok(())
}

/// Smoke produksi: import device + sesi ratchet, connect, print pesan masuk.
async fn smoke(
    creds: &str,
    fixture: &str,
    hold_secs: u64,
    db: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = oktz_wa::prepare_store(creds, db, Some(fixture)).await?;

    let (handle, _guard) = oktz_wa::pairing_run::connect_device_guarded(
        store,
        creds_for_guard(creds),
        move |_client| async move {
            println!(
                "{}",
                serde_json::json!({"phase":"CONNECTED","rss_kb":oktz_wa::rss::current_kb()})
            );
        },
    )
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
    db: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = oktz_wa::prepare_store(creds, db, None).await?;
    let to = to.to_string();
    let kind = kind.to_string();

    let (handle, _guard) = oktz_wa::pairing_run::connect_device_guarded(
        store,
        creds_for_guard(creds),
        move |client| {
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
                let opts = oktz_wa::SendOptions::default()
                    .with_extra_stanza_nodes(vec![oktz_wa::biz_node()]);

                let msg = match kind.as_str() {
                    "card" => {
                        oktz_wa::build_card("Kartu dari produksi Rust", Some("footer produksi"))
                    }
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
        },
    )
    .await?;

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(hold_secs.max(20));
    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    handle.shutdown().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairing_db_default_is_a_persistent_dir_not_scratch() {
        // Regresi M7: default lama menunjuk `/tmp/opencode/oktz-pair.db`.
        // Path itu hilang begitu scratch dir di-reset, dan bot yang kehilangan
        // db kehilangan device row + sesi ratchet.
        let db = resolve_db(None, data_dir(None).join("oktz-pair.db"));
        assert_eq!(db, PathBuf::from("./database/wa/oktz-pair.db"));
        assert!(
            !db.to_string_lossy().contains("/tmp/opencode"),
            "default pairing db tidak boleh menunjuk scratch dir: {}",
            db.display()
        );
    }

    #[test]
    fn db_dir_env_moves_the_persistent_default() {
        assert_eq!(
            resolve_db(None, data_dir(Some("/var/lib/oktz")).join("oktz-pair.db")),
            PathBuf::from("/var/lib/oktz/oktz-pair.db")
        );
    }

    #[test]
    fn blank_env_falls_back_to_the_built_in_default() {
        // `OKTZ_DB_DIR=""` harus berarti "tidak di-override", bukan path ""
        // yang jadi db relatif bernama kosong.
        assert_eq!(data_dir(Some("")), PathBuf::from("./database/wa"));
        assert_eq!(data_dir(Some("   ")), PathBuf::from("./database/wa"));
        assert_eq!(data_dir(None), PathBuf::from("./database/wa"));
    }

    #[test]
    fn explicit_positional_path_still_wins() {
        // Kontrak CLI lama: `oktz pair-code <nomor> [db] [hold]` — argumen
        // posisi kalau ada harus menang atas default, apa pun isi env.
        assert_eq!(
            resolve_db(Some("/tmp/one-off.db"), data_dir(None).join("oktz-pair.db")),
            PathBuf::from("/tmp/one-off.db")
        );
        assert_eq!(
            resolve_db(
                Some("/tmp/one-off.db"),
                data_dir(Some("/var/lib/oktz")).join("oktz-pair.db")
            ),
            PathBuf::from("/tmp/one-off.db")
        );
        assert_eq!(
            resolve_db(Some("  "), data_dir(None).join("oktz-pair.db")),
            PathBuf::from("./database/wa/oktz-pair.db"),
            "argumen kosong harus jatuh ke default, bukan jadi path kosong"
        );
    }

    #[test]
    fn dev_db_is_disposable_and_unique_per_process() {
        let a = dev_db(&dev_dir(None), "smoke", 111);
        let b = dev_db(&dev_dir(None), "smoke", 222);
        assert_eq!(a.parent(), Some(std::env::temp_dir().as_path()));
        assert_ne!(
            a, b,
            "dua proses smoke paralel harus tidak berebut file db yang sama \
             — `prepare_store` menghapus file lama sebelum menulis"
        );
        assert_eq!(
            dev_db(&dev_dir(None), "send", 111).file_name().unwrap(),
            "oktz-send-111.db"
        );
        assert_eq!(a.file_name().unwrap(), "oktz-smoke-111.db");
    }

    #[test]
    fn dev_db_follows_db_dir_env_out_of_temp() {
        // smoke/send memang alat dev dan boleh temp, tapi CI atau lokal yang
        // ingin datanya bertahan bisa memindahkannya lewat env yang sama.
        let db = dev_db(&dev_dir(Some("/tmp/oktz-ci")), "smoke", 7);
        assert_eq!(db, PathBuf::from("/tmp/oktz-ci/oktz-smoke-7.db"));
    }
}
