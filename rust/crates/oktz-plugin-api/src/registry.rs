use std::sync::{Arc, OnceLock};

use crate::meta::PluginMeta;

pub type HandlerFn = Arc<dyn Fn(Ctx) -> Reply + Send + Sync>;

pub enum Reply {
    Text(String),
    None,
}

pub struct Ctx {
    pub text: String,
    pub args: Vec<String>,
    pub sender: String,
    pub is_owner: bool,
    pub uptime_secs: u64,
}

pub struct PluginEntry {
    pub meta: PluginMeta,
    pub handler: HandlerFn,
}

#[doc(hidden)]
pub type PluginInit = fn() -> &'static PluginEntry;

#[linkme::distributed_slice]
pub static PLUGINS: [PluginInit];

static CACHE: OnceLock<Vec<&'static PluginEntry>> = OnceLock::new();

pub fn all_plugins() -> &'static [&'static PluginEntry] {
    CACHE.get_or_init(|| PLUGINS.iter().map(|f| f()).collect())
}

pub fn count() -> usize {
    all_plugins().len()
}

pub fn find(command: &str) -> Option<&'static PluginEntry> {
    all_plugins()
        .iter()
        .copied()
        .find(|e| e.meta.matches(command))
}
