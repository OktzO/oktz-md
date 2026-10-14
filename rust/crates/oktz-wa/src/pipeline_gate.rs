//! Gate yang butuh state & I/O, di atas primitif bebas-I/O di
//! `oktz_router::gates` / `oktz_router::group`.
//!
//! Dipisah dari [`crate::pipeline`] supaya `pipeline.rs` tetap soal routing dan
//! pengiriman, sementara file ini squarely soal "data mana yang hidup di mana".
//!
//! ## Kenapa state-nya in-memory, bukan ke disk
//!
//! Di JS, `db.setCooldown` menulis ke `user.cooldowns` lalu `setUser` → file
//! JSON (`database.js:701-709`), dan `updateEnergi` melakukan hal sama pada
//! `user.energi` (`database.js:627-644`). Jadi **JS memang persist ke disk**.
//!
//! Menirunya di sini akan salah. `StoreContext` memegang `Arc<JsonStore>` yang
//! dibaca lewat `&self`, dan `JsonStore::write(&self)` menulis SELURUH
//! dokumen. Dipanggil per command, itu berarti tiap pesan men-write seluruh
//! `database.json` — hammering disk di hot path, dan lebih buruk lagi,
//! menimpa perubahan yang ditulis jalur lain (pairing/registrasi) karena
//! salinan di memori bisa basi.
//!
//! Yang benar untuk satu proses: **state gate hidup di dalam `Pipeline`**,
//! di bawah `Mutex`, seperti `RateLimiter` yang sudah ada di sana.
//! Konsekuensinya jujur dan dibatasi: restart bot menghapus cooldown yang
//! belum kedaluwarsa dan mengembalikan saldo energi ke default tier. Tidak ada
//! user yang kehilangan kredit, dan charge tetap terjadi sebelum dispatch —
//! jadi tidak ada command gratis akibat restart.
//!
//! Trade-off itu dibalik begitu ada jalur tulis store yang bisa dipakai dari
//! sini: ganti dua store di bawah dengan implementasi `CooldownStore` /
//! `EnergiBalance` yang menulis `user.cooldowns` / `user.energi`. Titik
//! sambungnya sudah sempit karena keduanya cuma bicara lewat trait.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use oktz_plugin_api::PluginMeta;
use oktz_router::gates::{
    EnergiBalance, EnergiDecision, EnergiSettings, InMemoryCooldowns, InMemoryEnergi, charge_energi,
    check_cooldown, set_cooldown,
};
use serde_json::Value;

use oktz_router::Caller;

// =============================================================================
// State gate lintas-pesan
// =============================================================================

/// State yang harus bertahan antar-pesan dalam satu proses.
///
/// Dua store sudah tersedia di `oktz_router::gates` — [`InMemoryCooldowns`] dan
/// [`InMemoryEnergi`] — dengan trait yang persis dibutuhkan, jadi tidak ada
/// tipe baru di sini. Yang ditambahkan cuma pembungkus yang menjaga keduanya
/// di bawah satu `Mutex` supaya tidak bisa ter-update separuh jalan.
#[derive(Default)]
pub struct GateState {
    data: Mutex<GateData>,
}

#[derive(Default)]
struct GateData {
    cooldowns: InMemoryCooldowns,
    energi: InMemoryEnergi,
    /// `lastMessageTime` (`slowmode.js:135`) — `chat_sender` → ms terakhir.
    /// Toda del chain, bukan store: JS juga menyimpannya di memori.
    last_message_ms: HashMap<String, i64>,
}

impl GateState {
    pub fn new() -> Self {
        Self::default()
    }

    /// `PoisonError::into_inner`: state gate yang sudah stalk_selector tidak
    /// Tricky untuk dipulihkan, dan membiarkan panic berbulan-bulan lebih
    /// buruk daripada saturala yang korup.
    fn lock(&self) -> MutexGuard<'_, GateData> {
        self.data.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// `db.checkCooldown` (`handler.js:1839-1849`). `Some(sisa_detik)` = masih
    /// aktif → caller harus bereaksi dan membatalkan dispatch.
    pub fn cooldown_remaining(
        &self,
        caller: &Caller,
        cmd_key: &str,
        cooldown_secs: u32,
        now_ms: i64,
    ) -> Option<u64> {
        check_cooldown(&self.lock().cooldowns, caller, cmd_key, cooldown_secs, now_ms)
    }

    /// `db.setCooldown` (`handler.js:1922-1924`) — dipanggil SETELAH handler
    /// selesai, jadi command yang gagal di tengah tidak membekukan cooldown.
    pub fn start_cooldown(
        &self,
        caller: &Caller,
        cmd_key: &str,
        cooldown_secs: u32,
        now_ms: i64,
    ) {
        set_cooldown(
            &mut self.lock().cooldowns,
            caller,
            cmd_key,
            cooldown_secs,
            now_ms,
        );
    }

    /// Gate energi (`handler.js:1851-1890`) termasuk potongannya.
    ///
    /// Saldo yang belum pernah tercatat ikut ditulis di sini dengan tier
    /// caller, persis `user?.energi ?? (isOwner ? ownerEnergi : ...)`
    /// (`handler.js:1871-1877`) — kalau tidak, charge kedua akan membaca
    /// `None` lagi dan membebankan biaya dua kali ke saldo default.
    pub fn charge_energi(
        &self,
        caller: &Caller,
        meta: &PluginMeta,
        settings: &EnergiSettings,
        overrides: &HashMap<String, i64>,
    ) -> EnergiDecision {
        let mut data = self.lock();
        seed_energi(&mut data.energi, caller, settings);
        charge_energi(&mut data.energi, caller, meta, settings, overrides)
    }

    /// Saldo energi user seperti yang ada di gate state — untuk assert test
    /// bahwa charge terjadi tepat sekali.
    pub fn energy(&self, user: &str) -> Option<i64> {
        self.lock().energi.energy(user)
    }

    /// `lastMessageTime.get(key) || 0` — `0` berarti "tidak pernah", yang
    /// membuat gate slowmode selalu lolos untuk pesan pertama.
    pub fn last_message_ms(&self, key: &str) -> i64 {
        self.lock().last_message_ms.get(key).copied().unwrap_or(0)
    }

    /// `lastMessageTime.set(key, now)` — dipanggil HANYA saat rantai grup
    /// lolos, supaya pesan yang dibuang tidak memperlambat pesan berikutnya.
    pub fn record_message(&self, key: &str, now_ms: i64) {
        self.lock().last_message_ms.insert(key.to_string(), now_ms);
    }
}

/// Tulis saldo tier ke store kalau user belum punya — see
/// [`GateState::charge_energi`] untuk alasan.
fn seed_energi(store: &mut InMemoryEnergi, caller: &Caller, settings: &EnergiSettings) {
    if store.energy(&caller.sender).is_some() {
        return;
    }
    let tier = if caller.is_owner || caller.is_partner {
        settings.owner
    } else if caller.is_premium {
        settings.premium
    } else {
        settings.default
    };
    // Tier `-1` (tak terbatas) juga ditulis apa adanya: `charge_energi`
    // hubung pendek di `-1` dan tidak akan pernah mengarang angka.
    store.set_energy(&caller.sender, tier);
}

// =============================================================================
// Group settings — `groups[chat]` → `GroupSettings`
// =============================================================================

/// `groups[chat]` → [`oktz_router::group::GroupSettings`] untuk rantai
/// group-protection.
///
/// Peta persis ke bentuk yang dibaca `group-protection.js`:
/// - field `"on"` → `Some("on")`, `null`/hilang → `None`. Gate membandingkan
///   dengan `!== "on"` (`group-protection.js:1315`), jadi `None` = mati dan
///   `Some("true")` juga mati — boolean `true` tidak menyalakan apa pun.
/// - `*Mode` string mentah; `None` = default `"remove"` di sisi gate.
///
/// `Value::Null` (grup tidak ada di store) menghasilkan semua `None`, sama
/// seperti `db.getGroup()` yang `undefined` lalu `group.antilink !== "on"`
/// benar di JS.
pub fn group_settings_from(entry: &Value) -> oktz_router::group::GroupSettings {
    use oktz_router::group::{GroupSettings, Slowmode};
    let g = |k: &str| entry.get(k).and_then(Value::as_str).map(str::to_string);
    GroupSettings {
        antilink: g("antilink"),
        antilink_mode: g("antilinkMode"),
        antilinkgc: g("antilinkgc"),
        antilinkgc_mode: g("antilinkgcMode"),
        antilinkall: g("antilinkall"),
        antilinkall_mode: g("antilinkallMode"),
        antihidetag: g("antihidetag"),
        antidocument: entry
            .get("antidocument")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        mute: entry.get("mute").and_then(Value::as_bool).unwrap_or(false),
        muted_members: entry
            .get("mutedMembers")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        mutegc: entry.get("mutegc").and_then(Value::as_bool).unwrap_or(false),
        slowmode: entry.get("slowmode").map(|sm| Slowmode {
            enabled: sm.get("enabled").and_then(Value::as_bool).unwrap_or(false),
            delay_secs: sm.get("delay").and_then(Value::as_u64),
            mode: sm
                .get("mode")
                .and_then(Value::as_str)
                .map(str::to_string),
        }),
    }
}

// =============================================================================
// Roster grup → `GroupEnv.participants`
// =============================================================================

/// `GroupRoster` → `Vec<oktz_router::group::Participant>`.
///
/// `Participant` di router punya tiga identitas (`id`/`lid`/`phone_number`)
/// karena `adminFlagsFor` mencocokkan nomor dengan tiga bentuk itu. Yang kita
/// punya dari wire cuma satu JID per peserta, jadi JID yang sama diisi ke
/// `id` dan `phone_number`, dan `lid` dibiarkan kosong — `participant_nums`
/// menyaring string kosong, jadi slot kosong tidak bothers untuk apa pun.
///
/// `is_admin` sudah digabung oleh upstream: `roster_from` memakai
/// `is_admin()`, yang true untuk admin maupun superadmin
/// (`features/groups.rs:246`).
pub fn participants_from(roster: &oktz_plugin_api::registry::GroupRoster) -> Vec<oktz_router::group::Participant> {
    roster
        .participants
        .iter()
        .map(|p| oktz_router::group::Participant {
            id: p.jid.clone(),
            lid: String::new(),
            phone_number: p.jid.clone(),
            admin: p.is_admin,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn user() -> Caller {
        Caller {
            sender: "628123@s.whatsapp.net".into(),
            chat: "628123@s.whatsapp.net".into(),
            ..Default::default()
        }
    }

    #[test]
    fn absent_group_leaves_every_gate_disabled() {
        let s = group_settings_from(&Value::Null);
        assert!(s.antilink.is_none());
        assert!(!s.mutegc);
        assert!(s.slowmode.is_none());
    }

    #[test]
    fn on_literal_is_required_not_truthiness() {
        // `true` boolean tidak menyalakan antilink: JS membandingkan dengan "on".
        let s = group_settings_from(&json!({"antilink": true, "antilinkMode": "kick"}));
        assert!(s.antilink.is_none(), "boolean true bukan \"on\"");
        assert_eq!(s.antilink_mode.as_deref(), Some("kick"));
        assert_eq!(
            group_settings_from(&json!({"antilink": "on"})).antilink.as_deref(),
            Some("on")
        );
    }

    #[test]
    fn slowmode_maps_delay_and_mode() {
        let s = group_settings_from(&json!({
            "slowmode": {"enabled": true, "delay": 45, "mode": "onlycommand"}
        }));
        let sm = s.slowmode.expect("slowmode ada");
        assert!(sm.enabled);
        assert_eq!(sm.delay_secs, Some(45));
        assert_eq!(sm.mode.as_deref(), Some("onlycommand"));
    }

    #[test]
    fn cooldown_is_reported_then_can_be_started_and_is_seen_again() {
        let g = GateState::new();
        let c = user();
        assert_eq!(g.cooldown_remaining(&c, "ping", 5, 1_000), None);
        g.start_cooldown(&c, "ping", 5, 1_000);
        // Sisa 5 detik dari now=1000 -> ceil(5000/1000) = 5.
        assert_eq!(g.cooldown_remaining(&c, "ping", 5, 1_000), Some(5));
        // Lewat batas -> None, command boleh jalan lagi.
        assert_eq!(g.cooldown_remaining(&c, "ping", 5, 6_000), None);
        // Command lain punya cooldown sendiri.
        assert_eq!(g.cooldown_remaining(&c, "tebak", 5, 1_000), None);
    }

    #[test]
    fn last_message_defaults_to_zero_so_first_message_never_slowed() {
        let g = GateState::new();
        assert_eq!(g.last_message_ms("chat@sender"), 0);
        g.record_message("chat@sender", 1_234);
        assert_eq!(g.last_message_ms("chat@sender"), 1_234);
    }

    #[test]
    fn owner_and_partner_are_cooldown_exempt_but_premium_is_not() {
        let g = GateState::new();
        g.start_cooldown(&user(), "ping", 5, 0);

        let mut owner = user();
        owner.is_owner = true;
        let mut partner = user();
        partner.is_partner = true;
        let mut premium = user();
        premium.is_premium = true;

        assert_eq!(g.cooldown_remaining(&owner, "ping", 5, 10), None);
        assert_eq!(g.cooldown_remaining(&partner, "ping", 5, 10), None);
        assert_eq!(g.cooldown_remaining(&premium, "ping", 5, 10), Some(5));
    }
}