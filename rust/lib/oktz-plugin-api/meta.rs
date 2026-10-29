//! Kontrak plugin — cerminan `pluginConfig` JS.
//!
//! Field di sini persis yang BENAR-BENAR muncul di 797 file plugin
//! (lihat `docs/rust-rewrite/AUDIT.md` §1). Field `limit` ada di default JS
//! tapi 0 plugin override — sengaja tidak diclear karena tidak ada yang memakainya.
use serde::{Deserialize, Serialize};

// camelCase wajib: nama field di JS pluginConfig (isOwner, isEnabled, ...).
// Kalau tidak, kontrak serialisasi different dengan plugin-manifest.json.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PluginMeta {
    pub name: String,
    pub alias: Vec<String>,
    pub category: String,
    pub description: String,
    pub usage: String,
    pub example: String,
    pub cooldown: u32,
    pub energi: i64,
    pub is_enabled: bool,
    pub is_owner: bool,
    /// `checkPermission` aturan 2 (`middleware.js:57`) — partner tier antara
    /// owner dan premium. Hilang dari struct awal, padahal router butuh.
    pub is_partner: bool,
    pub is_premium: bool,
    pub is_group: bool,
    pub is_private: bool,
    pub is_admin: bool,
    pub is_bot_admin: bool,
    pub skip_registration: bool,
    /// Non-empty = command tidak pakai prefix (mis. alias trigger kata).
    /// `plugins/owner/eval.js` memakainya; `isBanned`/`noPrefix` juga ada di
    /// manifest tapi hanya 1 file — belum dipakai router.
    pub custom_trigger: Option<String>,
}

impl Default for PluginMeta {
    fn default() -> Self {
        Self {
            name: String::new(),
            alias: Vec::new(),
            category: String::from("uncategorized"),
            description: String::from("No description"),
            usage: String::new(),
            example: String::new(),
            cooldown: 3,
            energi: 0,
            is_enabled: true,
            is_owner: false,
            is_partner: false,
            is_premium: false,
            is_group: false,
            is_private: false,
            is_admin: false,
            is_bot_admin: false,
            skip_registration: false,
            custom_trigger: None,
        }
    }
}

impl PluginMeta {
    pub fn matches(&self, command: &str) -> bool {
        self.name == command || self.alias.iter().any(|a| a == command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_plugin_js() {
        // plugins.js:61-75 defaultConfig
        let m = PluginMeta::default();
        assert_eq!(m.category, "uncategorized");
        assert_eq!(m.cooldown, 3);
        assert_eq!(m.description, "No description");
        assert!(m.is_enabled);
        assert!(!m.is_private, "default JS isPrivate=false");
        assert!(!m.is_owner);
        assert!(m.alias.is_empty());
        assert_eq!(m.energi, 0, "energi tidak ada di defaultConfig JS");
    }

    #[test]
    fn matches_name_and_alias() {
        let m = PluginMeta {
            name: "menu".into(),
            alias: vec!["m".into(), "help".into()],
            ..PluginMeta::default()
        };
        assert!(m.matches("menu"));
        assert!(m.matches("m"));
        assert!(m.matches("help"));
        assert!(!m.matches("nope"));
    }

    #[test]
    fn serializes_to_camel_case_keys_like_js() {
        let m = PluginMeta::default();
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["isOwner"], false);
        assert_eq!(v["isEnabled"], true);
        assert_eq!(v["isGroup"], false);
        assert!(v.get("is_owner").is_none(), "tidak boleh snake_case");
        assert!(
            v.get("limit").is_none(),
            "limit tidak dipakai plugin mana pun"
        );
    }
}
