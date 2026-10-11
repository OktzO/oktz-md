//! Gate yang butuh state & waktu: rate limiter, kuota auto-work, cooldown, energi.
//!
//! `permission` dan `mode` sengaja hanya berisi fungsi murni. Empat gate di
//! sini memang butuh state (counter per sender) dan jam, tapi TIDAK butuh I/O:
//! waktu masuk sebagai `now_ms: i64`, saldo keluar lewat trait yang caller
//! isi (DB, cache, atau stub test). Konsekuensinya semua test di file ini
//! sinkron, tanpa async dan tanpa DB.
//!
//! Sumber JS:
//! - rate limiter global — `handler.js:358-362` + `handler.js:643-652`,
//!   algoritma `rate-limiter-flexible@9.1.1` (`RateLimiterMemory.consume` +
//!   `MemoryStorage.incrby`).
//! - kuota auto-work — `handler.js:373-422`.
//! - cooldown — `handler.js:1837-1849` + `handler.js:1922-1924`,
//!   `database.js:682-709`.
//! - energi — `handler.js:1851-1890` + `database.js:627-644`.
//!
//! Yang TIDAK sengaja direplikasi: `setTimeout` untuk meng-expire entri
//! limiter. JS memakai timer, di sini entri basi dibuang secara lazy saat
//! diakses (`expires_at <= now_ms`). Perilaku yang bisa diamati identik; yang
//! hilang hanya timer yang jalan di background.

use std::cell::OnceCell;
use std::collections::HashMap;

use oktz_plugin_api::PluginMeta;

use crate::permission::Caller;

/// Sentinel "energi tak terbatas" (`-1`) di JS.
pub const UNLIMITED: i64 = -1;

// ================= Rate limiter =================

#[derive(Debug, Clone, Copy)]
struct Entry {
    value: i64,
    expires_at_ms: i64,
}

/// Hasil `consume` yang LOLOS — cerminan `RateLimiterRes` (`remainingPoints`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsumeOk {
    pub remaining_points: u32,
}

/// Hasil `consume` yang DITOLAK — `rate-limiter-flexible` menolak dengan
/// `reject(res)`, jadi `msBeforeNext` masih berguna untuk pesan "tunggu".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsumeErr {
    /// Titik yang sudah terpakai dalam jendela berjalan (dapat > `points`).
    pub consumed_points: u32,
    pub ms_before_next: i64,
}

/// Leaky window counter per key — port `RateLimiterMemory` + `MemoryStorage`.
///
/// Semantik yang harus dijaga persis (sumber: `node_modules/rate-limiter-flexible@9.1.1`):
/// 1. `consume` menambah DULU laluMLETAK (`handler.js:647`: `await
///    globalRateLimiter.consume(jid)` yang di-`try/catch` → gagal = sedang
///    spam). Jadi command ke-9 tetap TERKONSUMSI walau ditolak.
/// 2. `MemoryStorage.incrby` menambah ke entri yang masih hidup; kalau sudah
///    kedaluwarsa ia mulai jendela baru dengan nilai 1, bukan menambah.
/// 3. Blokir hanya terjadi pada consumption pertama yang melewati `points`:
///    `consumedPoints <= points + pointsToConsume` (satu operasi = 1). Stroke
///    berikutnya saat blokir masih hidup menambah angka tapi TIDAK memperpanjang
///    blokir — jadi blokir benar-benar lasts `blockDuration`.
/// 4. Saat blokir dipasang, entri ditimpa: nilai = `consumedPoints`,-deadline
///    baru `now + blockDuration`. Jendela durasi awal dibuang.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    points: u32,
    duration_ms: i64,
    block_ms: i64,
    entries: HashMap<String, Entry>,
}

impl RateLimiter {
    /// `block_ms == 0` = tanpa blokir, hanya jendela bergulir (default
    /// `rate-limiter-flexible`).
    pub fn new(points: u32, duration_ms: i64, block_ms: i64) -> Self {
        Self {
            points,
            duration_ms,
            block_ms,
            entries: HashMap::new(),
        }
    }

    /// `handler.js:358-362` — limiter command global: 8 titik / 3 detik,
    /// blokir 2 detik.
    pub fn command_global() -> Self {
        Self::new(8, 3_000, 2_000)
    }

    /// `handler.js:373-376` + `config.js:170-175` — kuota auto-work: 10 titik
    /// per 60 detik, tanpa `blockDuration` (JS tidak mengaturnya → default 0).
    pub fn auto_work() -> Self {
        Self::new(10, 60_000, 0)
    }

    fn set(&mut self, key: &str, value: i64, duration_ms: i64, now_ms: i64) -> i64 {
        self.entries.insert(
            key.to_string(),
            Entry {
                value,
                expires_at_ms: now_ms.saturating_add(duration_ms),
            },
        );
        duration_ms
    }

    /// Port `MemoryStorage.incrby(key, 1, durationSec)` → `(value, msBeforeNext)`.
    fn incr(&mut self, key: &str, now_ms: i64) -> (i64, i64) {
        let live = self
            .entries
            .get(key)
            .is_some_and(|e| e.expires_at_ms.saturating_sub(now_ms) > 0);
        if live {
            let entry = self.entries.get_mut(key).expect("dicek hidup");
            entry.value += 1;
            (entry.value, entry.expires_at_ms.saturating_sub(now_ms))
        } else {
            // Entri hilang atau basi → jendela baru mulai dari 1.
            let ms = self.set(key, 1, self.duration_ms, now_ms);
            (1, ms)
        }
    }

    fn ms_before_next(&self, key: &str, now_ms: i64) -> i64 {
        self.entries
            .get(key)
            .map_or(0, |e| e.expires_at_ms.saturating_sub(now_ms))
    }

    /// Satu operasi = satu titik. `Ok` = boleh jalan, `Err` = kena limit.
    ///
    /// `now_ms` masuk sebagai parameter, bukan dibaca dari jam — itu yang
    /// membuat test deterministik.
    pub fn consume(&mut self, key: &str, now_ms: i64) -> Result<ConsumeOk, ConsumeErr> {
        let (consumed, _ms_before_next) = self.incr(key, now_ms);
        if consumed > i64::from(self.points) {
            // `blockDuration > 0 && consumedPoints <= points + pointsToConsume`
            if self.block_ms > 0 && consumed <= i64::from(self.points) + 1 {
                self.set(key, consumed, self.block_ms, now_ms);
            }
            Err(ConsumeErr {
                consumed_points: consumed as u32,
                ms_before_next: self.ms_before_next(key, now_ms),
            })
        } else {
            Ok(ConsumeOk {
                remaining_points: (i64::from(self.points) - consumed).max(0) as u32,
            })
        }
    }

    /// Nilai entri tanpa menaikkan — untuk observasi/test.
    pub fn peek(&self, key: &str, now_ms: i64) -> Option<u32> {
        self.entries
            .get(key)
            .filter(|e| e.expires_at_ms.saturating_sub(now_ms) > 0)
            .map(|e| e.value as u32)
    }

    /// Buang state satu key (mis. reset manual admin).
    pub fn forget(&mut self, key: &str) {
        self.entries.remove(key);
    }

    /// Jumlah key yang masih hidup — hook metrics/debug.
    pub fn live_keys(&self, now_ms: i64) -> usize {
        self.entries
            .values()
            .filter(|e| e.expires_at_ms.saturating_sub(now_ms) > 0)
            .count()
    }
}

/// `handler.js:643-652` + exemption di call site `handler.js:1508`:
/// `if (!m.isOwner && !m.isPremium && (await isSpamming(spamKey))) return;`
///
/// `true` berarti pesan dibuang (sedang spam). `anti_spam_enabled` =
/// `config.features.antiSpam` (`config.js:158`) — kalau mati, JS `return false`
/// tanpa menyentuh limiter.
pub fn is_spamming(
    limiter: &mut RateLimiter,
    caller: &Caller,
    jid: &str,
    now_ms: i64,
    anti_spam_enabled: bool,
) -> bool {
    if !anti_spam_enabled || is_spam_exempt(caller) {
        return false;
    }
    limiter.consume(jid, now_ms).is_err()
}

/// Owner dan premium bebas limit command (`handler.js:1508`). Bandingkan dengan
/// cooldown: yang-exempt di situ owner + partner, premium tidak.
pub fn is_spam_exempt(caller: &Caller) -> bool {
    caller.is_owner || caller.is_premium
}

// ================= Kuota auto-work =================

/// Setting yang dibaca `consumeAutoWorkQuota` (`handler.js:382-408`):
/// `config.autoWork.*` plus bagian `config.energi` yang dipakai untuk cek saldo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoWorkSettings {
    /// `config.autoWork.enabled` — kill switch (`config.js:171`).
    pub enabled: bool,
    /// Energi per pekerjaan auto (`config.autoWork.energi`, default 1).
    /// `0` = tanpa potong energi.
    pub cost: i64,
    /// `config.energi.enabled !== false` setelah resolusi setting DB.
    pub energi_enabled: bool,
    /// `config.energi.default` — saldo kalau user tidak punya record.
    pub default_energi: i64,
}

impl Default for AutoWorkSettings {
    /// `config.js:170-175` + `config.js:112-117`.
    fn default() -> Self {
        Self {
            enabled: true,
            cost: 1,
            energi_enabled: true,
            default_energi: 99_999,
        }
    }
}

/// Hasil satu percobaan auto-work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoWorkDecision {
    /// `true` = boleh jalan.
    pub allowed: bool,
    /// Energi yang HARUS dipotong caller (`update_energi`, `database.js:627`).
    /// `0` = tidak ada potongan. Catatan: pada saldo `-1` nilai ini tetap
    /// `cost` — `updateEnergi` sendiri yang short-circuit, bukan gate ini.
    pub deduct: i64,
}

impl AutoWorkDecision {
    const ALLOWED: Self = Self {
        allowed: true,
        deduct: 0,
    };
    const DENIED: Self = Self {
        allowed: false,
        deduct: 0,
    };
}

/// Port `consumeAutoWorkQuota` (`handler.js:382-408`).
///
/// Urutan WAJIB dipertahankan — ia yang menentukan apakah sebuah pesan
/// membakar kuota:
///
/// 1. `autoWork.enabled === false` → lolos tanpa efek.
/// 2. `m.isOwner || m.isPremium` → lolos tanpa efek. Keduanya skip
///    limiter DAN cek energi; `is_partner` TIDAK ada di sini (asimetri dengan
///    gate cooldown — itu disengaja di JS).
/// 3. Cek saldo (`currentEnergi !== -1 && currentEnergi < cost` → tolak).
///    `-1` berarti tak terbatas: cek dilewati, TAPI langkah 4 tetap jalan.
/// 4. `autoWorkLimiter.consume(key)` → kalau melempar, tolak.
/// 5. Potong energi.
pub fn consume_auto_work(
    limiter: &mut RateLimiter,
    caller: &Caller,
    key: &str,
    settings: &AutoWorkSettings,
    user_energi: Option<i64>,
    now_ms: i64,
) -> AutoWorkDecision {
    if !settings.enabled || caller.is_owner || caller.is_premium {
        return AutoWorkDecision::ALLOWED;
    }

    let charge_energi = settings.cost > 0 && settings.energi_enabled;
    if charge_energi {
        let current = user_energi.unwrap_or(settings.default_energi);
        // `-1` = tak terbatas: hanya melompati cek SALDO, bukan limiter.
        if current != UNLIMITED && current < settings.cost {
            return AutoWorkDecision::DENIED;
        }
    }

    if limiter.consume(key, now_ms).is_err() {
        return AutoWorkDecision::DENIED;
    }

    AutoWorkDecision {
        allowed: true,
        deduct: if charge_energi { settings.cost } else { 0 },
    }
}

/// Memoized decision per pesan — port `createAutoWorkGuard`
/// (`handler.js:416-422`).
///
/// Auto-AI, CAI chat, auto-download, auto-join, transkripsi CMD-VN, dan smart
/// trigger semuanya memanggil `allowAutoWork()` pada pesan yang sama
/// (`handler.js:984, 1129, 1136, 1143, 1149, 1422`). Tanpa memoization satu
/// pesan bisa burned 6 kuota; JS karena itu menyimpan satu `decided` per pesan.
///
/// `OnceCell` memberi jaminan yang sama: closure `charge` dijalankan paling
/// banyak sekali, hasilnya dikembalikan ulang apa adanya pada panggilan
/// berikutnya — termasuk ketika hasilnya `false` (JS memakai sentinel `null`,
/// bukan `falsy`, jadi penolakan juga di-memo).
#[derive(Debug)]
pub struct AutoWorkGuard {
    decided: OnceCell<AutoWorkDecision>,
}

impl Default for AutoWorkGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoWorkGuard {
    pub fn new() -> Self {
        Self {
            decided: OnceCell::new(),
        }
    }

    /// Setara `allowAutoWork()`. `charge` hanya dijalankan pada panggilan
    /// pertama; sisanya membaca hasil yang tersimpan.
    pub fn allow(&mut self, charge: impl FnOnce() -> AutoWorkDecision) -> AutoWorkDecision {
        *self.decided.get_or_init(charge)
    }

    /// Sudah pernah diputuskan? (untuk assert di test)
    pub fn is_decided(&self) -> bool {
        self.decided.get().is_some()
    }
}

// ================= Cooldown =================

/// Sumber deadline cooldown per `(user, command)`. Di JS ini field
/// `user.cooldowns` di DB (`database.js:701-709`) — trait supaya caller bisa
/// isi dengan DB-nya sendiri tanpa gate ini melakukan I/O.
pub trait CooldownStore {
    /// `user.cooldowns[command]` sebagai epoch ms; `None` = belum pernah.
    fn cooldown_end(&self, user: &str, cmd_key: &str) -> Option<i64>;
    fn set_cooldown_end(&mut self, user: &str, cmd_key: &str, end_ms: i64);
}

/// Implementasi in-memory untuk test dan mode single-process.
#[derive(Debug, Default)]
pub struct InMemoryCooldowns {
    map: HashMap<(String, String), i64>,
}

impl CooldownStore for InMemoryCooldowns {
    fn cooldown_end(&self, user: &str, cmd_key: &str) -> Option<i64> {
        self.map
            .get(&(user.to_string(), cmd_key.to_string()))
            .copied()
    }

    fn set_cooldown_end(&mut self, user: &str, cmd_key: &str, end_ms: i64) {
        self.map
            .insert((user.to_string(), cmd_key.to_string()), end_ms);
    }
}

/// Owner dan partner bebas cooldown (`handler.js:1839` dan `handler.js:1922`).
///
/// Perhatikan: premium TIDAK ada di exempt ini, padahal premium bebas dari
/// rate limiter (`handler.js:1508`) dan bebas dari gate auto-work
/// (`handler.js:384`). Asimetri ini berasal dari JS dan sengaja dipertahankan;
/// jangan "menyamakan" tanpa alasan.
pub fn is_cooldown_exempt(caller: &Caller) -> bool {
    caller.is_owner || caller.is_partner
}

/// Port `db.checkCooldown` + siftanya (`handler.js:1839-1849`).
///
/// `Some(sisa_detik)` = cooldown masih aktif dan `sisa_detik > 0` → penolakan.
/// Penolakan di JS adalah REAKSI (`m.react("⏱️")`), bukan reply, jadi caller
/// tidak perlu menulis pesan apa pun.
///
/// Pembulatan `Math.ceil((cooldownEnd - now) / 1000)` (`database.js:696`)
/// dipertahankan — sisa 1 ms tetap dibulat jadi 1 detik.
pub fn check_cooldown<S: CooldownStore + ?Sized>(
    store: &S,
    caller: &Caller,
    cmd_key: &str,
    cooldown_secs: u32,
    now_ms: i64,
) -> Option<u64> {
    if cooldown_secs == 0 || is_cooldown_exempt(caller) {
        return None;
    }
    let remaining_ms = store.cooldown_end(&caller.sender, cmd_key)? - now_ms;
    if remaining_ms <= 0 {
        return None;
    }
    // `remaining_ms > 0` sudah dipastikan di atas; `u64::div_ceil` stabil
    // sedangkan padanannya untuk `i64` masih unstable (`int_roundings`).
    Some((remaining_ms as u64).div_ceil(1000))
}

/// Port `db.setCooldown` (`database.js:701-709`), dipanggil SETELAH handler
/// selesai (`handler.js:1922-1924`) — jadi command yang gagal di tengah tidak
/// membekukan cooldown.
pub fn set_cooldown<S: CooldownStore + ?Sized>(
    store: &mut S,
    caller: &Caller,
    cmd_key: &str,
    cooldown_secs: u32,
    now_ms: i64,
) {
    if cooldown_secs == 0 || is_cooldown_exempt(caller) {
        return;
    }
    let end = now_ms.saturating_add(i64::from(cooldown_secs) * 1000);
    store.set_cooldown_end(&caller.sender, cmd_key, end);
}

// ================= Energi =================

/// Sumber/penerima saldo energi. `None` = user tidak ada di DB.
pub trait EnergiBalance {
    /// `db.getUser(jid)?.energi`.
    fn energy(&self, user: &str) -> Option<i64>;
    /// Tulis saldo baru.
    fn set_energy(&mut self, user: &str, value: i64);
}

/// Implementasi in-memory untuk test dan mode single-process.
#[derive(Debug, Default)]
pub struct InMemoryEnergi {
    map: HashMap<String, i64>,
}

impl EnergiBalance for InMemoryEnergi {
    fn energy(&self, user: &str) -> Option<i64> {
        self.map.get(user).copied()
    }

    fn set_energy(&mut self, user: &str, value: i64) {
        self.map.insert(user.to_string(), value);
    }
}

/// Resolve `energiEnabled` (`handler.js:1851-1854`):
/// `db.setting("energi") !== undefined ? db.setting("energi") : config.energi.enabled !== false`.
///
/// Sisi config harus sudah di-resolve `!== false` oleh caller (`Option::None`
/// → `true`, sama dengan `undefined !== false`).
pub fn resolve_energi_enabled(db_setting: Option<bool>, config_enabled: bool) -> bool {
    db_setting.unwrap_or(config_enabled)
}

/// Setting energi global (`config.js:112-117`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnergiSettings {
    pub enabled: bool,
    /// Dipakai owner DAN partner — JS tidak punya tier partner terpisah
    /// (`handler.js:1873`: `m.isOwner || m.isPartner ? ownerEnergi : ...`).
    pub owner: i64,
    pub premium: i64,
    pub default: i64,
}

impl Default for EnergiSettings {
    /// `config.js:112-117`: owner `-1` (tak terbatas), premium `99999999`,
    /// default `99999`.
    fn default() -> Self {
        Self {
            enabled: true,
            owner: UNLIMITED,
            premium: 99_999_999,
            default: 99_999,
        }
    }
}

impl EnergiSettings {
    /// Tier yang berlaku untuk caller — `handler.js:1871-1877`.
    fn tier_for(&self, caller: &Caller) -> i64 {
        if caller.is_owner || caller.is_partner {
            self.owner
        } else if caller.is_premium {
            self.premium
        } else {
            self.default
        }
    }
}

/// Cost satu command. Override per-plugin menang atas `meta.energi`
/// (`handler.js:1856-1857`):
/// `capEnergiOverrides[name] !== undefined ? overrides[name] : (plugin.energi || 0)`.
///
/// Perhatikan operatornya `!== undefined`, bukan `||`: override bernilai `0`
///Tetap menang dan mematikan biaya command itu.
pub fn energi_cost(meta: &PluginMeta, overrides: &HashMap<String, i64>) -> i64 {
    match overrides.get(&meta.name) {
        Some(v) => *v,
        None => meta.energi.max(0),
    }
}

/// Hasil gate energi untuk satu dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnergiDecision {
    /// `false` = saldo kurang, caller membalas pesan "energi habis".
    pub allowed: bool,
    /// Saldo tak terbatas (`-1` di tier atau di user) — cek dan potongan
    /// dilewati seluruhnya.
    pub unlimited: bool,
    /// Potongan yang sudah diterapakan oleh `charge_energi` ke `balance`.
    pub deducted: i64,
    /// Saldo SETELAH potongan — caller yang memutuskan boleh tidak mengirim
    /// notifikasi (`db.setting("notiflimit")` di `handler.js:1884`).
    pub remaining: Option<i64>,
}

impl EnergiDecision {
    const NOOP: Self = Self {
        allowed: true,
        unlimited: false,
        deducted: 0,
        remaining: None,
    };
    const UNLIMITED: Self = Self {
        allowed: true,
        unlimited: true,
        deducted: 0,
        remaining: None,
    };
    const INSUFFICIENT: Self = Self {
        allowed: false,
        unlimited: false,
        deducted: 0,
        remaining: None,
    };
}

/// Port `updateEnergi` (`database.js:627-644`).
///
/// `-1` selalu berarti tak terbatas dan TIDAK pernah ditulis ulang — entah di
/// record user, entah karena tier owner/premium di-`-1`. Nilai diklem ke `0`
/// (`Math.max(0, ...)`).
pub fn update_energi<B: EnergiBalance + ?Sized>(
    balance: &mut B,
    caller: &Caller,
    settings: &EnergiSettings,
    delta: i64,
) -> i64 {
    let stored = balance.energy(&caller.sender);
    if stored == Some(UNLIMITED)
        || ((caller.is_owner || caller.is_partner) && settings.owner == UNLIMITED)
        || (caller.is_premium && settings.premium == UNLIMITED)
    {
        return UNLIMITED;
    }
    let current = stored.unwrap_or(settings.tier_for(caller));
    let next = (current + delta).max(0);
    balance.set_energy(&caller.sender, next);
    next
}

/// Port gate energi (`handler.js:1851-1890`), termasuk potongannya.
///
/// Pemotongan terjadi DI DALAM fungsi ini, sebelum handler plugin jalan, dan
/// tidak ada refund kalau handler gagal — sama seperti JS. Dipisah dari
/// "cek" justru karena itu: `&mut balance` DIPAKAI tepat sekali per call,
/// jadi `deducted` tak mungkin terpotong dua kali.
pub fn charge_energi<B: EnergiBalance + ?Sized>(
    balance: &mut B,
    caller: &Caller,
    meta: &PluginMeta,
    settings: &EnergiSettings,
    overrides: &HashMap<String, i64>,
) -> EnergiDecision {
    let cost = energi_cost(meta, overrides);
    if !settings.enabled || cost <= 0 {
        return EnergiDecision::NOOP;
    }

    let stored = balance.energy(&caller.sender);
    let tier = settings.tier_for(caller);

    // `handler.js:1865-1870`: dua cabang ini keluar tanpa cek dan tanpa potong.
    // Salah satunya triggered `user?.energi === -1` — saldo tak terbatas
    // user mana pun, bukan cuma owner/premium.
    if tier == UNLIMITED || stored == Some(UNLIMITED) {
        return EnergiDecision::UNLIMITED;
    }

    let current = stored.unwrap_or(tier);
    if current < cost {
        return EnergiDecision::INSUFFICIENT;
    }

    let next = update_energi(balance, caller, settings, -cost);
    EnergiDecision {
        allowed: true,
        unlimited: false,
        deducted: cost,
        remaining: Some(next),
    }
}

/// Render pesan potong limit (`handler.js:1885-1887`):
/// `limitMsg.replace("{amount}", ...).replace("{sisa}", ...)`.
pub fn render_limit_deducted(template: &str, amount: i64, remaining: i64) -> String {
    template
        .replace("{amount}", &amount.to_string())
        .replace("{sisa}", &remaining.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const USER: &str = "628123@s.whatsapp.net";
    const T0: i64 = 1_000_000;

    fn user() -> Caller {
        Caller {
            sender: USER.into(),
            chat: USER.into(),
            ..Default::default()
        }
    }

    fn owner() -> Caller {
        Caller {
            is_owner: true,
            ..user()
        }
    }

    fn premium() -> Caller {
        Caller {
            is_premium: true,
            ..user()
        }
    }

    fn partner() -> Caller {
        Caller {
            is_partner: true,
            ..user()
        }
    }

    fn meta(name: &str, energi: i64) -> PluginMeta {
        PluginMeta {
            name: name.into(),
            energi,
            ..Default::default()
        }
    }

    // ---------- Rate limiter ----------

    #[test]
    fn rate_limiter_allows_eight_then_blocks_the_ninth() {
        let mut rl = RateLimiter::command_global();
        for i in 0..8 {
            assert!(
                rl.consume(USER, T0).is_ok(),
                "command ke-{} harus lolos",
                i + 1
            );
        }
        let blocked = rl.consume(USER, T0).unwrap_err();
        assert_eq!(blocked.consumed_points, 9);
    }

    #[test]
    fn rate_limiter_block_expires_after_block_duration() {
        let mut rl = RateLimiter::command_global();
        for _ in 0..9 {
            let _ = rl.consume(USER, T0);
        }
        assert!(rl.consume(USER, T0 + 1999).is_err(), "masih diblokir");
        // `MemoryStorage.set` menimpa entri dengan TTL blokir 2 detik, jadi
        // tepat di 2000 ms jendela baru mulai.
        assert!(
            rl.consume(USER, T0 + 2000).is_ok(),
            "setelah blockDuration harus boleh lagi"
        );
    }

    #[test]
    fn rate_limiter_rejected_calls_still_consume_points() {
        // `handler.js:647` consume-then-check: command yang DITOLAK ikut
        // membakar kuota, jadi spam lanjut bisa memperpanjang tekanan.
        let mut rl = RateLimiter::command_global();
        for _ in 0..9 {
            let _ = rl.consume(USER, T0);
        }
        let err = rl.consume(USER, T0).unwrap_err();
        assert_eq!(err.consumed_points, 10);
        assert_eq!(rl.peek(USER, T0), Some(10));
    }

    #[test]
    fn rate_limiter_does_not_extend_block_on_extra_calls() {
        // `consumedPoints <= points + pointsToConsume` — stroke 2 saat blokir
        // hidup tidak memasang blokir baru.
        let mut rl = RateLimiter::command_global();
        for _ in 0..9 {
            let _ = rl.consume(USER, T0);
        }
        let _ = rl.consume(USER, T0 + 1000); // masih dalam blokir
        assert!(
            rl.consume(USER, T0 + 1999).is_err(),
            "blokir tidak boleh diperpanjang oleh panggilan tambahan"
        );
        assert!(rl.consume(USER, T0 + 2000).is_ok());
    }

    #[test]
    fn rate_limiter_owner_is_exempt() {
        let mut rl = RateLimiter::command_global();
        for _ in 0..20 {
            assert!(!is_spamming(&mut rl, &owner(), USER, T0, true));
        }
        assert_eq!(rl.peek(USER, T0), None, "exempt tak boleh menyentuh limiter");
    }

    #[test]
    fn rate_limiter_premium_is_exempt() {
        let mut rl = RateLimiter::command_global();
        for _ in 0..20 {
            assert!(!is_spamming(&mut rl, &premium(), USER, T0, true));
        }
        assert_eq!(rl.peek(USER, T0), None);
    }

    #[test]
    fn rate_limiter_partner_is_not_exempt() {
        // Partner bebas cooldown tapi TIDAK bebas spam — asimetri yang sama
        // seperti auto-work. Cegah "menyamar" partner jadi premium.
        let mut rl = RateLimiter::command_global();
        for _ in 0..8 {
            assert!(!is_spamming(&mut rl, &partner(), USER, T0, true));
        }
        assert!(is_spamming(&mut rl, &partner(), USER, T0, true));
    }

    #[test]
    fn rate_limiter_disabled_never_blocks() {
        let mut rl = RateLimiter::command_global();
        for _ in 0..50 {
            assert!(!is_spamming(&mut rl, &user(), USER, T0, false));
        }
        assert_eq!(rl.peek(USER, T0), None);
    }

    #[test]
    fn rate_limiter_is_keyed_per_sender() {
        let mut rl = RateLimiter::command_global();
        for _ in 0..8 {
            assert!(rl.consume(USER, T0).is_ok());
        }
        assert!(rl.consume(USER, T0).is_err());
        assert!(
            rl.consume("628999@s.whatsapp.net", T0).is_ok(),
            "sender lain punya ember sendiri"
        );
    }

    // ---------- Kuota auto-work ----------

    #[test]
    fn auto_work_limit_rejects_eleventh_message() {
        let mut rl = RateLimiter::auto_work();
        let s = AutoWorkSettings {
            cost: 0, // fokus ke limiter
            ..Default::default()
        };
        for i in 0..10 {
            assert!(
                consume_auto_work(&mut rl, &user(), "k", &s, Some(100), T0).allowed,
                "pesan ke-{} harus lolos",
                i + 1
            );
        }
        assert!(
            !consume_auto_work(&mut rl, &user(), "k", &s, Some(100), T0).allowed,
            "pesan ke-11 harus kena limit"
        );
    }

    #[test]
    fn auto_work_limiter_still_blocks_while_energy_is_charged() {
        // Urutan itu load-bearing: cek saldo harus TIDAK menggantikan limiter.
        // Kalau `cost > 0` dan saldo masih cukup, kuota habis harus tetap menolak
        // (dan tidak boleh memotong energi).
        let mut rl = RateLimiter::auto_work();
        let s = AutoWorkSettings {
            cost: 2,
            ..Default::default()
        };
        for i in 0..10 {
            assert!(consume_auto_work(&mut rl, &user(), "k", &s, Some(10_000), T0).allowed);
            assert_eq!(rl.peek("k", T0), Some(i as u32 + 1));
        }
        let d = consume_auto_work(&mut rl, &user(), "k", &s, Some(10_000), T0);
        assert!(!d.allowed, "limit auto-work tetap berlaku saat energi dipotong");
        assert_eq!(d.deduct, 0, "pesan yang ditolak limiter tak boleh dipotong");
    }

    #[test]
    fn auto_work_resets_after_window() {
        let mut rl = RateLimiter::auto_work();
        let s = AutoWorkSettings {
            cost: 0,
            ..Default::default()
        };
        for _ in 0..11 {
            let _ = consume_auto_work(&mut rl, &user(), "k", &s, Some(100), T0);
        }
        assert!(consume_auto_work(&mut rl, &user(), "k", &s, Some(100), T0 + 60_000).allowed);
    }

    #[test]
    fn auto_work_owner_skips_limiter_entirely() {
        let mut rl = RateLimiter::auto_work();
        for _ in 0..50 {
            let d = consume_auto_work(&mut rl, &owner(), "k", &AutoWorkSettings::default(), Some(0), T0);
            assert!(d.allowed, "owner selalu boleh");
            assert_eq!(d.deduct, 0, "owner tidak dipotong energi");
        }
        assert_eq!(rl.peek("k", T0), None, "limiter tak boleh tersentuh");
    }

    #[test]
    fn auto_work_premium_skips_limiter_entirely() {
        let mut rl = RateLimiter::auto_work();
        for _ in 0..50 {
            let d =
                consume_auto_work(&mut rl, &premium(), "k", &AutoWorkSettings::default(), Some(0), T0);
            assert!(d.allowed);
            assert_eq!(d.deduct, 0);
        }
        assert_eq!(rl.peek("k", T0), None);
    }

    #[test]
    fn auto_work_unlimited_energy_skips_balance_check_but_consumes_limiter_point() {
        let mut rl = RateLimiter::auto_work();
        let s = AutoWorkSettings::default(); // cost 1
        // Saldo -1: cek SALDO dilewati...
        let d = consume_auto_work(&mut rl, &user(), "k", &s, Some(UNLIMITED), T0);
        assert!(d.allowed);
        // ...tapi titik limiter tetap terpakai.
        assert_eq!(rl.peek("k", T0), Some(1));
        // Dan `updateEnergi` sendiri yang.skip menulis ulang saldo -1.
        let mut bal = InMemoryEnergi::default();
        bal.set_energy(USER, UNLIMITED);
        assert_eq!(update_energi(&mut bal, &user(), &EnergiSettings::default(), -1), UNLIMITED);
        assert_eq!(bal.energy(USER), Some(UNLIMITED), "saldo -1 tak boleh berubah");
    }

    #[test]
    fn auto_work_insufficient_energy_denies_without_touching_limiter() {
        let mut rl = RateLimiter::auto_work();
        let s = AutoWorkSettings {
            cost: 5,
            ..Default::default()
        };
        let d = consume_auto_work(&mut rl, &user(), "k", &s, Some(3), T0);
        assert!(!d.allowed);
        assert_eq!(rl.peek("k", T0), None, "cek saldo gagal BEFORE limiter");
    }

    #[test]
    fn auto_work_charges_energy_when_sufficient() {
        let mut rl = RateLimiter::auto_work();
        let s = AutoWorkSettings {
            cost: 3,
            ..Default::default()
        };
        let d = consume_auto_work(&mut rl, &user(), "k", &s, Some(10), T0);
        assert!(d.allowed);
        assert_eq!(d.deduct, 3);
    }

    #[test]
    fn auto_work_disabled_skips_everything() {
        let mut rl = RateLimiter::auto_work();
        let s = AutoWorkSettings {
            enabled: false,
            cost: 10,
            ..Default::default()
        };
        for _ in 0..50 {
            let d = consume_auto_work(&mut rl, &user(), "k", &s, Some(0), T0);
            assert!(d.allowed);
            assert_eq!(d.deduct, 0);
        }
        assert_eq!(rl.peek("k", T0), None);
    }

    #[test]
    fn auto_work_guard_charges_once_per_message() {
        let mut rl = RateLimiter::auto_work();
        let s = AutoWorkSettings::default();
        let mut guard = AutoWorkGuard::new();
        let mut charges = 0;

        // Enam titik kerja mahal pada pesan yang sama (handler.js:984, 1129,
        // 1136, 1143, 1149, 1422) — semuanya harus memakai SATU kuota.
        for _ in 0..6 {
            let d = guard.allow(|| {
                charges += 1;
                consume_auto_work(&mut rl, &user(), "k", &s, Some(100), T0)
            });
            assert!(d.allowed);
            assert_eq!(d.deduct, 1);
        }
        assert_eq!(charges, 1, "guard hanya boleh charge sekali");
        assert!(guard.is_decided());
        assert_eq!(rl.peek("k", T0), Some(1), "satu titik limiter terpakai");
    }

    #[test]
    fn auto_work_guard_memoizes_denial_too() {
        // JS memakai sentinel `null`, bukan `falsy` — keputusan `false` juga
        // disimpan, jadi 5 titik kerja gagal tetap membakar nol kuota tambahan.
        let mut rl = RateLimiter::auto_work();
        let s = AutoWorkSettings {
            cost: 0,
            ..Default::default()
        };
        for _ in 0..10 {
            let _ = rl.consume("k", T0);
        }
        let mut guard = AutoWorkGuard::new();
        let mut charges = 0;
        for _ in 0..5 {
            assert!(!guard.allow(|| {
                charges += 1;
                consume_auto_work(&mut rl, &user(), "k", &s, Some(100), T0)
            })
            .allowed);
        }
        assert_eq!(charges, 1);
    }

    #[test]
    fn auto_work_guard_memoizes_per_instance() {
        let mut rl = RateLimiter::auto_work();
        let s = AutoWorkSettings {
            cost: 0,
            ..Default::default()
        };
        let mut charges = 0;
        for _ in 0..3 {
            let mut guard = AutoWorkGuard::new();
            guard.allow(|| {
                charges += 1;
                consume_auto_work(&mut rl, &user(), "k", &s, Some(100), T0)
            });
        }
        assert_eq!(charges, 3, "guard baru = pesan baru = kuota baru");
    }

    // ---------- Cooldown ----------

    #[test]
    fn cooldown_first_call_allowed() {
        let mut store = InMemoryCooldowns::default();
        assert_eq!(check_cooldown(&store, &user(), "ping", 5, T0), None);
        set_cooldown(&mut store, &user(), "ping", 5, T0);
    }

    #[test]
    fn cooldown_second_call_returns_remaining_seconds() {
        let mut store = InMemoryCooldowns::default();
        set_cooldown(&mut store, &user(), "ping", 5, T0);
        let remaining = check_cooldown(&store, &user(), "ping", 5, T0 + 1000).expect("harus ditolak");
        assert_eq!(remaining, 4, "ceil((5000-1000)/1000)");
    }

    #[test]
    fn cooldown_rounds_up_partial_seconds() {
        let mut store = InMemoryCooldowns::default();
        set_cooldown(&mut store, &user(), "ping", 5, T0);
        // `Math.ceil` — sisa 1 ms tetap 1 detik, bukan 0.
        assert_eq!(
            check_cooldown(&store, &user(), "ping", 5, T0 + 4999),
            Some(1)
        );
        assert_eq!(
            check_cooldown(&store, &user(), "ping", 5, T0 + 1000),
            Some(4)
        );
    }

    #[test]
    fn cooldown_expires_after_window() {
        let mut store = InMemoryCooldowns::default();
        set_cooldown(&mut store, &user(), "ping", 5, T0);
        assert!(check_cooldown(&store, &user(), "ping", 5, T0 + 4999).is_some());
        assert_eq!(
            check_cooldown(&store, &user(), "ping", 5, T0 + 5000),
            None,
            "tepat di deadline harus lolos"
        );
    }

    #[test]
    fn cooldown_is_keyed_per_command() {
        let mut store = InMemoryCooldowns::default();
        set_cooldown(&mut store, &user(), "ping", 5, T0);
        assert!(check_cooldown(&store, &user(), "ping", 5, T0).is_some());
        assert_eq!(
            check_cooldown(&store, &user(), "pong", 5, T0),
            None,
            "command lain tidak ikut beku"
        );
    }

    #[test]
    fn cooldown_zero_duration_is_disabled() {
        let mut store = InMemoryCooldowns::default();
        set_cooldown(&mut store, &user(), "ping", 0, T0);
        assert_eq!(check_cooldown(&store, &user(), "ping", 0, T0), None);
        assert_eq!(store.cooldown_end(USER, "ping"), None, "tak ada yang ditulis");
    }

    #[test]
    fn cooldown_owner_is_exempt() {
        let mut store = InMemoryCooldowns::default();
        let mut o = owner();
        o.sender = USER.into();
        set_cooldown(&mut store, &user(), "ping", 5, T0); // user biasa beku dulu
        assert_eq!(check_cooldown(&store, &o, "ping", 5, T0 + 10), None);
        set_cooldown(&mut store, &o, "ping", 5, T0);
        assert_eq!(
            store.cooldown_end(USER, "ping"),
            Some(T0 + 5000),
            "exempt juga berarti tidak pernah SET cooldown"
        );
    }

    #[test]
    fn cooldown_partner_is_exempt() {
        let mut store = InMemoryCooldowns::default();
        set_cooldown(&mut store, &partner(), "ping", 5, T0);
        assert_eq!(check_cooldown(&store, &partner(), "ping", 5, T0), None);
    }

    #[test]
    fn cooldown_premium_is_not_exempt() {
        // ASIMETRI dari JS: premium bebas rate limiter (handler.js:1508) dan
        // bebas auto-work (handler.js:384), tapi TIDAK bebas cooldown
        // (handler.js:1839 hanya mengecek isOwner/isPartner).
        let mut store = InMemoryCooldowns::default();
        let p = premium();
        set_cooldown(&mut store, &p, "ping", 5, T0);
        assert!(
            check_cooldown(&store, &p, "ping", 5, T0).is_some(),
            "premium tetap kena cooldown"
        );
    }

    // ---------- Energi ----------

    #[test]
    fn energi_sufficient_balance_deducts_exact_cost_and_reports_remainder() {
        let mut bal = InMemoryEnergi::default();
        bal.set_energy(USER, 50);
        let d = charge_energi(
            &mut bal,
            &user(),
            &meta("heavy", 7),
            &EnergiSettings::default(),
            &HashMap::new(),
        );
        assert!(d.allowed);
        assert!(!d.unlimited);
        assert_eq!(d.deducted, 7);
        assert_eq!(d.remaining, Some(43));
        assert_eq!(bal.energy(USER), Some(43), "saldo harus benar-benar turun");
    }

    #[test]
    fn energi_insufficient_balance_denies() {
        let mut bal = InMemoryEnergi::default();
        bal.set_energy(USER, 3);
        let d = charge_energi(
            &mut bal,
            &user(),
            &meta("heavy", 7),
            &EnergiSettings::default(),
            &HashMap::new(),
        );
        assert!(!d.allowed);
        assert_eq!(d.deducted, 0);
        assert_eq!(bal.energy(USER), Some(3), "saldo tak boleh tersentuh");
    }

    #[test]
    fn energi_exact_balance_is_allowed_and_drops_to_zero() {
        let mut bal = InMemoryEnergi::default();
        bal.set_energy(USER, 7);
        let d = charge_energi(
            &mut bal,
            &user(),
            &meta("heavy", 7),
            &EnergiSettings::default(),
            &HashMap::new(),
        );
        assert!(d.allowed, "saldo pas-pasan cukup");
        assert_eq!(d.remaining, Some(0));
        assert_eq!(bal.energy(USER), Some(0));
    }

    #[test]
    fn energi_override_map_beats_meta() {
        let mut ov = HashMap::new();
        ov.insert("heavy".to_string(), 20);
        assert_eq!(energi_cost(&meta("heavy", 7), &ov), 20);

        let mut bal = InMemoryEnergi::default();
        bal.set_energy(USER, 50);
        let d = charge_energi(
            &mut bal,
            &user(),
            &meta("heavy", 7),
            &EnergiSettings::default(),
            &ov,
        );
        assert_eq!(d.deducted, 20, "override menang atas meta");
        assert_eq!(bal.energy(USER), Some(30));
    }

    #[test]
    fn energi_zero_override_disables_the_cost() {
        // `capEnergiOverrides[name] !== undefined` — 0 di override tetap
        // menang dan mematikan biaya, bukan jatuh ke meta.
        let mut ov = HashMap::new();
        ov.insert("heavy".to_string(), 0);
        assert_eq!(energi_cost(&meta("heavy", 7), &ov), 0);

        let mut bal = InMemoryEnergi::default();
        bal.set_energy(USER, 1);
        let d = charge_energi(
            &mut bal,
            &user(),
            &meta("heavy", 7),
            &EnergiSettings::default(),
            &ov,
        );
        assert!(d.allowed);
        assert_eq!(d.deducted, 0);
        assert_eq!(bal.energy(USER), Some(1));
    }

    #[test]
    fn energi_missing_override_falls_back_to_meta_then_zero() {
        let ov = HashMap::new();
        assert_eq!(energi_cost(&meta("a", 4), &ov), 4);
        assert_eq!(energi_cost(&meta("b", 0), &ov), 0, "meta 0 = gratis");
    }

    #[test]
    fn energi_minus_one_means_unlimited_no_deduction() {
        let mut bal = InMemoryEnergi::default();
        bal.set_energy(USER, UNLIMITED);
        let d = charge_energi(
            &mut bal,
            &user(),
            &meta("heavy", 7),
            &EnergiSettings::default(),
            &HashMap::new(),
        );
        assert!(d.allowed);
        assert!(d.unlimited);
        assert_eq!(d.deducted, 0);
        assert_eq!(bal.energy(USER), Some(UNLIMITED), "tidak ada penulisan");
    }

    #[test]
    fn energi_minus_one_tier_default_means_unlimited() {
        // `config.energi.owner = -1` → owner/partner tak terbatas tanpa pernah
        // menyentuh record user.
        let mut bal = InMemoryEnergi::default();
        let d = charge_energi(
            &mut bal,
            &owner(),
            &meta("heavy", 7),
            &EnergiSettings::default(),
            &HashMap::new(),
        );
        assert!(d.allowed && d.unlimited);
        assert_eq!(bal.energy(USER), None, "record user tak pernah dibuat");
    }

    #[test]
    fn energi_premium_tier_limit_applies_when_configured() {
        // Kalau `config.energi.premium` diubah jadi angka, premium ikut
        // dipotong seperti user biasa.
        let mut bal = InMemoryEnergi::default();
        bal.set_energy(USER, 100);
        let settings = EnergiSettings {
            premium: 1_000,
            ..Default::default()
        };
        let d = charge_energi(
            &mut bal,
            &premium(),
            &meta("heavy", 7),
            &settings,
            &HashMap::new(),
        );
        assert_eq!(d.deducted, 7);
        assert_eq!(bal.energy(USER), Some(93));
    }

    #[test]
    fn energi_falls_back_to_tier_default_when_user_has_no_record() {
        let settings = EnergiSettings {
            default: 10,
            owner: UNLIMITED,
            premium: 1_000,
            enabled: true,
        };
        let mut bal = InMemoryEnergi::default();
        let d = charge_energi(
            &mut bal,
            &user(),
            &meta("heavy", 4),
            &settings,
            &HashMap::new(),
        );
        assert!(d.allowed);
        assert_eq!(d.remaining, Some(6));
        assert_eq!(bal.energy(USER), Some(6));
    }

    #[test]
    fn energi_zero_cost_is_a_noop() {
        let mut bal = InMemoryEnergi::default();
        bal.set_energy(USER, 0);
        let d = charge_energi(
            &mut bal,
            &user(),
            &meta("free", 0),
            &EnergiSettings::default(),
            &HashMap::new(),
        );
        assert!(d.allowed);
        assert!(!d.unlimited);
        assert_eq!(d.deducted, 0);
    }

    #[test]
    fn energi_disabled_is_a_noop() {
        let mut bal = InMemoryEnergi::default();
        bal.set_energy(USER, 1);
        let settings = EnergiSettings {
            enabled: false,
            ..Default::default()
        };
        let d = charge_energi(
            &mut bal,
            &user(),
            &meta("heavy", 7),
            &settings,
            &HashMap::new(),
        );
        assert!(d.allowed);
        assert_eq!(d.deducted, 0);
        assert_eq!(bal.energy(USER), Some(1));
    }

    /// `deducted` dilaporkan sekali dan saldo ditulis tepat sekali per
    /// dispatch — tidak ada jalan untuk memotong dua kali.
    #[test]
    fn energi_deducts_exactly_once_per_dispatch() {
        struct Counting {
            inner: InMemoryEnergi,
            writes: usize,
        }
        impl EnergiBalance for Counting {
            fn energy(&self, user: &str) -> Option<i64> {
                self.inner.energy(user)
            }
            fn set_energy(&mut self, user: &str, value: i64) {
                self.writes += 1;
                self.inner.set_energy(user, value);
            }
        }

        let mut bal = Counting {
            inner: InMemoryEnergi::default(),
            writes: 0,
        };
        bal.inner.set_energy(USER, 100);

        let d = charge_energi(
            &mut bal,
            &user(),
            &meta("heavy", 7),
            &EnergiSettings::default(),
            &HashMap::new(),
        );
        assert_eq!(d.deducted, 7);
        assert_eq!(bal.writes, 1, "satu dispatch = satu penulisan saldo");
        assert_eq!(bal.energy(USER), Some(93));
    }

    #[test]
    fn update_energi_clamps_at_zero() {
        // `Math.max(0, ...)` (database.js:641) — pengembalian (refund) lebih
        // besar dari saldo tidak membuat saldo negatif.
        let mut bal = InMemoryEnergi::default();
        bal.set_energy(USER, 5);
        assert_eq!(update_energi(&mut bal, &user(), &EnergiSettings::default(), -50), 0);
        assert_eq!(bal.energy(USER), Some(0));
    }

    #[test]
    fn update_energi_supports_refund_for_games() {
        // `src/lib/games.js:249` memakai `updateEnergi` untuk memberi kembali
        // limit — jadi delta positif harus tetap jalan.
        let mut bal = InMemoryEnergi::default();
        bal.set_energy(USER, 5);
        assert_eq!(update_energi(&mut bal, &user(), &EnergiSettings::default(), 10), 15);
    }

    #[test]
    fn resolve_energi_enabled_prefers_db_setting() {
        assert!(resolve_energi_enabled(Some(true), false));
        assert!(!resolve_energi_enabled(Some(false), true));
        assert!(resolve_energi_enabled(None, true));
        assert!(!resolve_energi_enabled(None, false));
    }

    #[test]
    fn render_limit_deducted_fills_placeholders() {
        let msg = render_limit_deducted(
            "🔋 Limit kau berkurang sebanyak {amount}. Sisa limit: {sisa}",
            7,
            43,
        );
        assert_eq!(
            msg,
            "🔋 Limit kau berkurang sebanyak 7. Sisa limit: 43",
            "handler.js:1886"
        );
    }
}