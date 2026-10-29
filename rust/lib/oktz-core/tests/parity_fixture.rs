//! Fixture parity: nilai yang dilihat user harus sama dengan `config.js`
//! yang sedang berjalan, bukan dengan diligent transkripsi di file Rust.
//!
//! Fixture dibuat ulang oleh `scripts/gen-parity-fixtures.mjs`. Kalau test
//! ini gagal setelah `config.js` diubah, jalankan ulang script itu — jangan
//! mengoreksi nilai di sisi Rust.

use std::path::PathBuf;

fn fixture() -> serde_json::Value {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parity/config_text.json");
    let raw = std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("fixture parity tidak terbaca: {p:?}: {e}"));
    serde_json::from_str(&raw).expect("fixture parity bukan JSON")
}

#[test]
fn config_text_matches_the_js_source() {
    let f = fixture();
    assert_eq!(
        f["messages"]["ownerOnly"],
        "*Akses Ditolak!* Fitur ini khusus untuk Owner bot."
    );
}

#[test]
fn every_message_key_from_config_js_is_present() {
    let f = fixture();
    for k in [
        "success",
        "error",
        "ownerOnly",
        "premiumOnly",
        "groupOnly",
        "privateOnly",
        "adminOnly",
        "botAdminOnly",
        "cooldown",
        "energiExceeded",
        "limitDeducted",
        "banned",
        "rejectCall",
    ] {
        assert!(
            f["messages"].get(k).and_then(|v| v.as_str()).is_some(),
            "config.messages.{k} hilang dari fixture"
        );
    }
}

#[test]
fn bot_identity_matches_config_js() {
    let f = fixture();
    assert_eq!(f["info"]["website"], "https://oktzz.xyz");
    assert_eq!(f["info"]["botName"], "Oktz Bot");
    assert_eq!(f["info"]["version"], "3.3.1");
    assert_eq!(f["info"]["developer"], "Zann");
    assert_eq!(f["saluran"]["id"], "120363411617217441@newsletter");
}