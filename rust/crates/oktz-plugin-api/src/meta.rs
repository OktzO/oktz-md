//! Kontrak plugin — cerminan `pluginConfig` JS.
//!
//! Field di sini persis yang BENAR-BENAR muncul di 797 file plugin
//! (lihat `docs/rust-rewrite/AUDIT.md` §1). Field `limit` ada di default JS
//! tapi 0 plugin override — sengaja tidak diclear karena tidak ada yang memakainya.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub is_premium: bool,
    pub is_group: bool,
    pub is_private: bool,
    pub is_admin: bool,
    pub is_bot_admin: bool,
    pub skip_registration: bool,
}

impl Default for PluginMeta {
    fn default() -> Self {
        Self {
            name: String::new(),
            alias: Vec::new(),
            category: String::new(),
            description: String::new(),
            usage: String::new(),
            example: String::new(),
            cooldown: 0,
            energi: 0,
            is_enabled: true,
            is_owner: false,
            is_premium: false,
            is_group: false,
            is_private: true,
            is_admin: false,
            is_bot_admin: false,
            skip_registration: false,
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
    fn defaults_match_js_dataset() {
        let m = PluginMeta::default();
        assert_eq!(m.cooldown, 0);
        assert_eq!(m.energi, 0);
        assert!(m.is_enabled);
        assert!(m.is_private, "default JS isPrivate true di most");
        assert!(!m.is_owner);
        assert!(m.alias.is_empty());
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
    fn serializes_to_snake_case_keys() {
        let m = PluginMeta::default();
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["is_owner"], false);
        assert_eq!(v["is_enabled"], true);
        assert!(v.get("limit").is_none(), "limit tidak boleh ada");
    }
}
