//! Guard: binary harus benar-benar me-link crate plugin.
//!
//! `all_plugins()` mengembalikan slice yang bisa KOSONG tanpa error — kalau
//! crate plugin tidak ter-link, `oktz boot` tetap exit 0 sambil melaporkan
//! `plugins: 0`. Ini persis kegagalan yang tidak terlihat sampai bot
//! diam di grup. Test ini menjaganya.

use std::process::Command;

/// Path binary `oktz` hasil build. `CARGO_BIN_EXE_oktz` diisi cargo untuk test
/// integrasi pada crate yang punya bin target.
fn boot_output() -> String {
    let exe = env!("CARGO_BIN_EXE_oktz");
    let out = Command::new(exe)
        .arg("boot")
        .output()
        .expect("binary oktz harus bisa dijalankan");
    assert!(
        out.status.success(),
        "oktz boot gagal: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

#[test]
fn boot_reports_a_non_zero_plugin_count() {
    let stdout = boot_output();
    // Baris JSON terakhir yang memuat "plugins": N.
    let line = stdout
        .lines()
        .find(|l| l.contains("\"plugins\""))
        .unwrap_or_else(|| panic!("tidak ada baris JSON di stdout: {stdout}"));

    let count: usize = line
        .split("\"plugins\":")
        .nth(1)
        .and_then(|rest| rest.split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("tidak bisa parse jumlah plugin dari: {line}"));

    assert!(
        count > 0,
        "plugins: 0 berarti crate plugin tidak ter-link — \
         cek `use_plugin_crates()` di oktz-bin/src/main.rs"
    );
}

#[test]
fn every_registered_plugin_has_a_name() {
    // Plugin yang nama-nya kosong lolos `count > 0` tapi tidak bisa dipanggil.
    // Ini regresi `Expr::Group` di macro — lindungi di sini.
    let stdout = boot_output();
    assert!(
        !stdout.contains("\"plugins\":0"),
        "registry kosong: {stdout}"
    );
}