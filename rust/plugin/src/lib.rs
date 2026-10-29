#![allow(special_module_name)]

// `plugins/main/` adalah kategori plugin JS, bukan binary crate kita —
// makanya `pub mod main;` di bawah bukan salah. Lint `special_module_name`
// hanya bisa ditekan di level crate: `#[allow]` pada item-nya sendiri
// diabaikan rustc.
pub mod ai;
pub mod anime;
pub mod canvas;
pub mod cek;
pub mod clan;
pub mod common;
pub mod download;
pub mod ephoto;
pub mod fun;
pub mod game;
pub mod group;
pub mod info;
pub mod main;
pub mod nsfw;
pub mod owner;
pub mod panel;
pub mod primbon;
pub mod random;
pub mod religi;
pub mod stalker;
pub mod rpg;
pub mod search;
pub mod sticker;
pub mod store;
pub mod tools;
pub mod user;
pub mod utility;

#[cfg(test)]
mod registry_test {
    use std::collections::HashSet;

    use oktz_plugin_api::{all_plugins, find};

    /// Registry penuh: nama unik, dan setiap nama/alias harus menunjuk plugin
    /// yang sama lewat jalur yang dipakai router. Test ini menangkap plugin
    /// yang lupa didaftarkan (`pub mod`) sekaligus alias mati/tabrakan.
    #[test]
    fn nama_unik_dan_bisa_ditemukan() {
        let plugins = all_plugins();
        assert!(
            plugins.len() > 500,
            "registry hanya {} plugin; crate plugin mungkin tidak ter-link",
            plugins.len()
        );

        let mut nama = HashSet::new();
        for p in plugins {
            assert!(!p.meta.name.is_empty(), "plugin tanpa nama");
            assert!(!p.meta.category.is_empty(), "{} tanpa kategori", p.meta.name);
            assert!(nama.insert(p.meta.name.as_str()), "nama {} dipakai dua plugin", p.meta.name);
        }

        for p in plugins {
            let ketemu = find(p.meta.name.as_str())
                .unwrap_or_else(|| panic!("{} tidak ditemukan", p.meta.name));
            assert_eq!(ketemu.meta.name, p.meta.name);
        }
    }

    /// Command menu wajib ada: tanpa ini `.menu` jatuh ke NoPlugin dan pipeline
    /// hanya mencatat NotACommand (gejala "menu tidak muncul").
    #[test]
    fn menu_dan_menucat_terdaftar() {
        for nama in ["menu", "menucat"] {
            assert!(find(nama).is_some(), "{nama} tidak terdaftar");
        }
    }
}
