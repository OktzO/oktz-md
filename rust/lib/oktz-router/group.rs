//! Port rantai group-protection dari `src/handler.js:770-871`.
//!
//! Rantai itu 16 sub-gate yang jalan HANYA kalau `m.isGroup && !m.isNewsletter`,
//! dan tiap gate yang `return true` langsung menghentikan pipeline. Gate PERTAMA
//! yang match yang menang — urutan itu load-bearing, bukan detail.
//!
//! Modul ini fournir dua hal terpisah:
//!
//! 1. **Framework** — [`GroupGuard`] + [`Decision`]. Gate =
//!    `fn(&GateInput) -> GateOutcome`; guard menjalankan daftar berurutan dan
//!    berhenti di gate pertama yang _match_.
//! 2. **Gate yang benar-benar ported** — lihat [`default_gate_chain`] untuk
//!    urutan dan daftar gate yang TIDAK ikut dipindah beserta alasannya.
//!
//! Data yang dipakai gate (daftar admin grup, `selfMode`, pesan) sudah
//! diteruskan caller sebagai nilai biasa, jadi tidak ada I/O dan tidak ada
//! ketergantungan wacore di sini.

/// Caller yang dipakai gate. Cerminan flag `m.*` yang dibaca rantai JS.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupCaller {
    pub sender: String,
    /// `m.key.participant` — bisa beda dari `sender` di grup addressing LID.
    pub participant: Option<String>,
    pub chat: String,
    pub is_group: bool,
    pub is_newsletter: bool,
    pub is_owner: bool,
    pub is_admin: bool,
    pub is_partner: bool,
    pub is_bot_admin: bool,
    pub from_me: bool,
    pub is_command: bool,
    pub is_document: bool,
    /// `m.type` — dipakai `checkAntidocument` (`documentMessage` /
    /// `documentWithCaptionMessage`).
    pub message_type: String,
    /// `m.body || ""`
    pub body: String,
    /// `m.mentionedJid`
    pub mentioned_jid: Vec<String>,
    /// Jumlah member grup (`groupMetadata.participants.length`).
    pub participant_count: usize,
}

/// Participant grup, seperlunya untuk cek admin.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Participant {
    /// `p.jid || p.id`
    pub id: String,
    pub lid: String,
    pub phone_number: String,
    pub admin: bool,
}

/// Setelan grup — semua field yang dibaca gate. String `Option` dipakai untuk
/// toggle yang di JS dibandingkan dengan `!== "on"` (bukan `!group.x`), jadi
/// nilai boolean `true` tidak menyalakan gate. Mode `None` = `"remove"`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupSettings {
    pub antilink: Option<String>,
    pub antilink_mode: Option<String>,
    pub antilinkgc: Option<String>,
    pub antilinkgc_mode: Option<String>,
    pub antilinkall: Option<String>,
    pub antilinkall_mode: Option<String>,
    pub antihidetag: Option<String>,
    /// Gate ini pakai `!groupData.antidocument` (truthiness), bukan `"on"`.
    pub antidocument: bool,
    pub mute: bool,
    pub muted_members: Vec<String>,
    pub mutegc: bool,
    pub slowmode: Option<Slowmode>,
}

/// `groupData.slowmode`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Slowmode {
    pub enabled: bool,
    /// Detik; `None`/0 → default 30 (`sm.delay || 30`).
    pub delay_secs: Option<u64>,
    /// `None`/kosong → `"all"`. `"onlycommand"` punya cabang sendiri.
    pub mode: Option<String>,
}

/// Semua yang dibutuhkan gate besides caller + setelan grup.
#[derive(Debug, Clone, Default)]
pub struct GateEnv {
    /// `sock.user.id.split(":")[0] + "@s.whatsapp.net"` — BENTUK BUGGY yang
    /// dipakai `handleAntilink*` (group-protection.js:1327/1898/1953).
    /// Gate yang pakai `botJidOf()` (yang benar) sengaja TIDAK diport.
    pub bot_number: String,
    /// `sock.user.lid` — kandidat kedua untuk cek "bot admin".
    pub bot_lid: Option<String>,
    /// `db.setting("selfMode") === true`.
    pub self_mode: bool,
    pub participants: Vec<Participant>,
    /// `Date.now()` untuk gate slowmode.
    pub now_ms: i64,
}

/// Hasil satu gate: `None` = tidak trigger,_chain lanjut_.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    /// Gate tidak trigger — lanjut ke gate berikutnya.
    Pass,
    /// Gate memicu aksi, chain BERHENTI.
    Decide(Decision),
    /// Gate melakukan aksi tapi JS TIDAK `return` — chain LANJUT.
    /// Cuma `checkSpam` yang begini (handler.js:833-842).
    Continue,
}

/// Keputusan router yang harus dieksekusi caller.
///
/// `DeleteAndReply` dan `KickAndReply` dipisah dari `Reply`/`DeleteAndStop`
/// karena majority gate menghapus pesan DAN membalas; kalau dipipihkan jadi
/// satu varian, salah satu efeknya hilang diam-diam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Lolos semua gate — pesan lanjut ke pipeline router.
    Allow,
    /// Berhenti; balas teks. Pesan TIDAK dihapus.
    Reply(String),
    /// Berhenti; hapus pesan; tanpa balasan.
    DeleteAndStop,
    /// Berhenti tanpa aksi sama sekali.
    Stop,
    /// Hapus pesan lalu balas. Urutan eksekusi mengikuti JS: delete → reply.
    DeleteAndReply(String),
    /// Hapus pesan, keluarkan pengirim, lalu balas.
    ///
    /// Fallback saat kick gagal TIDAK ditangani di sini (butuh I/O) — caller
    /// yang harus balas pesan non-kick. Lihat `KICK_FALLBACK`.
    KickAndReply(String),
}

/// Teks fallback ketika `groupParticipantsUpdate` gagal — pollah yang sama
/// seperti cabang non-kick. `handleAntilinkGc/All` (group-protection.js:1921
/// / 1976) memanggilnya di `catch`; `handleAntilink` (:1349) tidak, jadi
/// caller tidak boleh memakai ini untuk gate `antilink`.
pub const KICK_FALLBACK: &str = "kick_failed";

/// Varian `Reply` yang perlu konten `KickAndReply`. Helper supaya caller
/// tidak perlu menyimpan string kedua di luar.
impl Decision {
    /// Teks balasan, kalau decision ini punya balasan.
    pub fn reply_text(&self) -> Option<&str> {
        match self {
            Self::Reply(t) | Self::DeleteAndReply(t) | Self::KickAndReply(t) => Some(t),
            Self::Allow | Self::DeleteAndStop | Self::Stop => None,
        }
    }
}

/// Input yang diteruskan ke setiap gate.
pub struct GateInput<'a> {
    pub caller: &'a GroupCaller,
    pub settings: &'a GroupSettings,
    pub env: &'a GateEnv,
    /// `lastMessageTime.get("chat_sender") || 0` (slowmode.js:135).
    pub last_message_ms: i64,
}

type GateFn = fn(&GateInput<'_>) -> GateOutcome;

/// Satu gate + nama untuk debug/log. Nama ada supaya urutan rantai bisa
/// diuji tanpa membandingkan alamat fungsi (yang bisa di-dedup optimizer).
#[derive(Clone)]
pub struct Gate {
    pub name: &'static str,
    pub run: GateFn,
}

impl Gate {
    pub const fn new(name: &'static str, run: GateFn) -> Self {
        Self { name, run }
    }
}

/// Rantai gate. Urutan daftar = urutan eksekusi.
pub struct GroupGuard {
    gates: Vec<Gate>,
}

impl GroupGuard {
    pub fn new(gates: Vec<Gate>) -> Self {
        Self { gates }
    }

    /// Nama gate berurutan — berguna untuk log saat gate pertama match.
    pub fn gate_names(&self) -> Vec<&'static str> {
        self.gates.iter().map(|g| g.name).collect()
    }

    /// Jalankan rantai. Gate pertama yang `Decide` menang.
    pub fn run(&self, input: &GateInput<'_>) -> Decision {
        // handler.js:770 — seluruh blok dilewati kalau bukan grup / newsletter.
        if !input.caller.is_group || input.caller.is_newsletter {
            return Decision::Allow;
        }
        for gate in &self.gates {
            if let GateOutcome::Decide(d) = (gate.run)(input) {
                return d;
            }
        }
        Decision::Allow
    }
}

/// Rantai default — urutan persis `handler.js:773-870`, gate yang tidak
/// diport di-lewati (komentarnya menyebut alasannya; urutan relatif gate yang
/// ada tetap sama).
pub fn default_gate_chain() -> GroupGuard {
    GroupGuard::new(vec![
        Gate::new("antilink", gate_antilink),        // handler.js:782
        Gate::new("antilinkgc", gate_antilink_gc),   // handler.js:785
        Gate::new("antilinkall", gate_antilink_all), // handler.js:788
        Gate::new("antihidetag", gate_hidetag),      // handler.js:791
        Gate::new("antidocument", gate_antidocument),// handler.js:797
        Gate::new("mute", gate_mute),                // handler.js:809
        Gate::new("mutedmember", gate_muted_member), // handler.js:818
        Gate::new("mutegc", gate_mutegc),            // handler.js:827
        Gate::new("slowmode", gate_slowmode),        // handler.js:844
        // Dilewati: AntiJudol/AntiPhising/AntiCustom (butuh daftar kata
        //   JUDOL_PATTERNS/PHISING + regex custom dari DB), AntiSwGc (butuh
        //   tree protobuf `findSwGcType`), detectBot (butuh `analyzeBotMessage`
        //   + `groupMetadata`), checkSpam (butuh `spamTracker` lintas pesan +
        //   `handleSpamAction`), isToxic (butuh `toxicWords` + counter
        //   `toxicWarns` per member di DB).
    ])
}

// ---------------------------------------------------------------------------
// Gate
// ---------------------------------------------------------------------------

/// `group-protection.js:1311` `handleAntilink`.
///
/// PENTING: gate ini TIDAK mengecek `selfMode` sebelum skip pesan bot sendiri
/// (bandingkan `handleAntilinkGc` :1899-1900). Dan `botNumber` di sini bentuk
/// buggy `id.split(":")[0] + "@s.whatsapp.net"` — sama seperti JS.
fn gate_antilink(input: &GateInput<'_>) -> GateOutcome {
    let c = input.caller;
    if input.settings.antilink.as_deref() != Some("on") {
        return GateOutcome::Pass;
    }
    if !contains_link(&c.body) {
        return GateOutcome::Pass;
    }

    let sender_tag = sender_tag(&c.sender);
    if is_same_participant(c.participant.as_deref().unwrap_or(&c.sender), &input.env.bot_number)
    {
        return GateOutcome::Pass;
    }

    if is_admin_check(&input.env.participants, c.participant.as_deref(), &c.sender) {
        return GateOutcome::Pass;
    }
    if !is_bot_admin_check(input.env) {
        // Gate ini replying "bot bukan admin" — berbeda dari Gc/All yang diam.
        return GateOutcome::Decide(Decision::Reply(NOT_ADMIN.to_string()));
    }

    let kicked = input.settings.antilink_mode.as_deref() == Some("kick");
    let text = if kicked {
        msg(ANTILINK_KICK, &sender_tag)
    } else {
        msg(ANTILINK, &sender_tag)
    };
    GateOutcome::Decide(if kicked {
        Decision::KickAndReply(text)
    } else {
        Decision::DeleteAndReply(text)
    })
}

/// `group-protection.js:1888` `handleAntilinkGc` — hanya link WA spesifik.
///
/// Berbeda dari `handleAntilink` di dua hal yang mengubah perilaku:
/// - self-sender hanya di-skip kalau `selfMode` MATI (:1900);
/// - bot bukan admin → `return false`, SENYAP (:1909), bukan reply.
fn gate_antilink_gc(input: &GateInput<'_>) -> GateOutcome {
    let c = input.caller;
    if input.settings.antilinkgc.as_deref() != Some("on") {
        return GateOutcome::Pass;
    }
    if !contains_wa_specific_link(&c.body) {
        return GateOutcome::Pass;
    }

    let sender_tag = sender_tag(&c.sender);
    if !input.env.self_mode
        && is_same_participant(c.participant.as_deref().unwrap_or(&c.sender), &input.env.bot_number)
    {
        return GateOutcome::Pass;
    }

    if is_admin_check(&input.env.participants, c.participant.as_deref(), &c.sender) {
        return GateOutcome::Pass;
    }
    if !is_bot_admin_check(input.env) {
        return GateOutcome::Pass;
    }

    let kicked = input.settings.antilinkgc_mode.as_deref() == Some("kick");
    let text = if kicked {
        msg(ANTILINK_GC_KICK, &sender_tag)
    } else {
        msg(ANTILINK_GC, &sender_tag)
    };
    GateOutcome::Decide(if kicked {
        Decision::KickAndReply(text)
    } else {
        Decision::DeleteAndReply(text)
    })
}

/// `group-protection.js:1943` `handleAntilinkAll` — link apa pun (TLD daftar).
///
/// Strukturnya identik dengan `handleAntilinkGc` (termasuk senyap saat bot
/// bukan admin, :1964) — hanya deteksi link dan teks pesannya yang beda.
fn gate_antilink_all(input: &GateInput<'_>) -> GateOutcome {
    let c = input.caller;
    if input.settings.antilinkall.as_deref() != Some("on") {
        return GateOutcome::Pass;
    }
    if !contains_any_link(&c.body) {
        return GateOutcome::Pass;
    }

    let sender_tag = sender_tag(&c.sender);
    if !input.env.self_mode
        && is_same_participant(c.participant.as_deref().unwrap_or(&c.sender), &input.env.bot_number)
    {
        return GateOutcome::Pass;
    }

    if is_admin_check(&input.env.participants, c.participant.as_deref(), &c.sender) {
        return GateOutcome::Pass;
    }
    if !is_bot_admin_check(input.env) {
        return GateOutcome::Pass;
    }

    let kicked = input.settings.antilinkall_mode.as_deref() == Some("kick");
    let text = if kicked {
        msg(ANTILINK_ALL_KICK, &sender_tag)
    } else {
        msg(ANTILINK_ALL, &sender_tag)
    };
    GateOutcome::Decide(if kicked {
        Decision::KickAndReply(text)
    } else {
        Decision::DeleteAndReply(text)
    })
}

/// `group-protection.js:2145` `handleAntiHidetag`.
///
/// Threshold-nya `mentionedJid.length < participants.length` → lolos, jadi
/// yang dihapus hanya hidetag yang menyasar SEMUA member. Bot bukan admin
/// → senyap (`return false`), bukan reply.
fn gate_hidetag(input: &GateInput<'_>) -> GateOutcome {
    let c = input.caller;
    if input.settings.antihidetag.as_deref() != Some("on") {
        return GateOutcome::Pass;
    }
    if c.mentioned_jid.is_empty() {
        return GateOutcome::Pass;
    }
    if c.mentioned_jid.len() < c.participant_count {
        return GateOutcome::Pass;
    }
    if c.is_admin || c.is_owner || c.from_me {
        return GateOutcome::Pass;
    }
    if !c.is_bot_admin {
        return GateOutcome::Pass;
    }

    let text = msg(ANTIHIDETAG, &sender_tag(&c.sender));
    GateOutcome::Decide(Decision::DeleteAndReply(text))
}

/// `plugins/group/antidocument.js:32` `checkAntidocument`.
///
/// Punya exemption `isAdmin || isOwner || fromMe` di baris PERTAMA — sudah
/// ada sebelum toggle dicek. Toggle-nya truthiness (`!groupData.antidocument`),
/// bukan `"on"` seperti gate antilink.
fn gate_antidocument(input: &GateInput<'_>) -> GateOutcome {
    let c = input.caller;
    if c.is_admin || c.is_owner || c.from_me {
        return GateOutcome::Pass;
    }
    if !input.settings.antidocument {
        return GateOutcome::Pass;
    }
    let is_document = c.is_document
        || c.message_type == "documentMessage"
        || c.message_type == "documentWithCaptionMessage";
    if !is_document {
        return GateOutcome::Pass;
    }

    let text = msg(ANTIDOCUMENT, &sender_tag(&c.sender));
    GateOutcome::Decide(Decision::DeleteAndReply(text))
}

/// `handler.js:809-816` + `plugins/group/mute.js:31` `isMuted`.
///
/// Exemption `!isAdmin && !isOwner` ada di handler, jadi bot bukan admin TIDAK
/// exempt: dia tetap delete kalau punya hak, lalu stop. Reply? Tidak ada.
fn gate_mute(input: &GateInput<'_>) -> GateOutcome {
    let c = input.caller;
    if c.is_admin || c.is_owner {
        return GateOutcome::Pass;
    }
    if !input.settings.mute {
        return GateOutcome::Pass;
    }
    GateOutcome::Decide(if c.is_bot_admin {
        Decision::DeleteAndStop
    } else {
        Decision::Stop
    })
}

/// `handler.js:818-825` + `plugins/group/mutemember.js:102` `isMutedMember`.
fn gate_muted_member(input: &GateInput<'_>) -> GateOutcome {
    let c = input.caller;
    if c.is_admin || c.is_owner {
        return GateOutcome::Pass;
    }
    if input.settings.muted_members.is_empty() {
        return GateOutcome::Pass;
    }
    if !is_muted_member(&input.settings.muted_members, &c.sender) {
        return GateOutcome::Pass;
    }
    GateOutcome::Decide(if c.is_bot_admin {
        Decision::DeleteAndStop
    } else {
        Decision::Stop
    })
}

/// `handler.js:827-831` + `plugins/group/mutegc.js:49` `isMutegc`.
///
/// Satu-satunya gate dengan exemption tigaflag `!isAdmin && !isOwner &&
/// !isPartner` (handler.js:827). Tidak ada delete — `return` polos.
fn gate_mutegc(input: &GateInput<'_>) -> GateOutcome {
    let c = input.caller;
    if c.is_admin || c.is_owner || c.is_partner {
        return GateOutcome::Pass;
    }
    if !input.settings.mutegc {
        return GateOutcome::Pass;
    }
    GateOutcome::Decide(Decision::Stop)
}

/// `handler.js:844-856` + `plugins/group/slowmode.js:120` `checkSlowmode`.
///
/// Dua cabang yang saling berbeda dan keduanya penting:
/// - `onlycommand` + pesan command + terlalu cepat → `return` SENYAP, pesan
///   TIDAK dihapus (:849).
/// - mode lain + terlalu cepat → delete lalu `return` (:851-852).
///
/// Sisi caller: kalau mode `all`/`onlycommand` dan waktu sudah cukup, JS
/// `lastMessageTime.set(key, now)` (slowmode.js:142). Itu efek samping dan
/// gate ini pure, jadi [`GroupGuard::run_with_record`] melaporkannya lewat
/// return value kedua — bukan dit-ulangkan di dalam gate.
fn gate_slowmode(input: &GateInput<'_>) -> GateOutcome {
    match eval_slowmode(input) {
        SlowVerdict::Pass => GateOutcome::Pass,
        SlowVerdict::Stop => GateOutcome::Decide(Decision::Stop),
        SlowVerdict::Delete => GateOutcome::Decide(Decision::DeleteAndStop),
    }
}

/// Hasil evaluasi slowmode — dipakai gate DAN penentu efek samping
/// `lastMessageTime`, supaya logikanya tidak ditulis dua kali.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlowVerdict {
    Pass,
    Stop,
    Delete,
}

fn eval_slowmode(input: &GateInput<'_>) -> SlowVerdict {
    let c = input.caller;
    if c.is_admin || c.is_owner {
        return SlowVerdict::Pass;
    }
    let Some(sm) = input.settings.slowmode.as_ref() else {
        return SlowVerdict::Pass;
    };
    if !sm.enabled {
        return SlowVerdict::Pass;
    }
    let raw = sm.mode.as_deref().unwrap_or("all");
    let mode = if raw.is_empty() { "all" } else { raw };
    if mode == "onlycommand" && !c.is_command {
        return SlowVerdict::Pass;
    }

    let delay = sm.delay_secs.filter(|d| *d > 0).unwrap_or(30) as f64;
    let elapsed_secs = (input.env.now_ms - input.last_message_ms) as f64 / 1000.0;
    if elapsed_secs >= delay {
        return SlowVerdict::Pass;
    }
    if mode == "onlycommand" {
        // handler.js:849 — `return` tanpa delete.
        SlowVerdict::Stop
    } else {
        SlowVerdict::Delete
    }
}

/// `true` kalau slowmode aktif untuk pesan ini DAN waktunya sudah cukup —
/// artinya caller harus menyimpan `now_ms` untuk `chat_sender`.
fn slowmode_records_timestamp(input: &GateInput<'_>) -> bool {
    let c = input.caller;
    let Some(sm) = input.settings.slowmode.as_ref() else {
        return false;
    };
    if !sm.enabled {
        return false;
    }
    let raw = sm.mode.as_deref().unwrap_or("all");
    let mode = if raw.is_empty() { "all" } else { raw };
    if mode == "onlycommand" && !c.is_command {
        return false;
    }
    // `eval_slowmode` sudah menolak admin/owner; di sini cukup kasus tidak
    // exempt agar hasilnya sama persis dengan "lolos + bukan exempt".
    if c.is_admin || c.is_owner {
        return false;
    }
    let delay = sm.delay_secs.filter(|d| *d > 0).unwrap_or(30) as f64;
    (input.env.now_ms - input.last_message_ms) as f64 / 1000.0 >= delay
}

impl GroupGuard {
    /// Sama seperti [`Self::run`], tapi melaporkan apakah caller harus
    /// menyimpan `now_ms` untuk key `chat_sender` (efek samping
    /// `lastMessageTime.set()` di slowmode.js:142).
    pub fn run_with_record(&self, input: &GateInput<'_>) -> (Decision, bool) {
        let d = self.run(input);
        let record = d == Decision::Allow && slowmode_records_timestamp(input);
        (d, record)
    }
}

// ---------------------------------------------------------------------------
// Teks default `gpMsg` (group-protection.js:25-53)
// ---------------------------------------------------------------------------

pub const NOT_ADMIN: &str = "⚠ Bot bukan admin, tidak bisa menghapus pesan.";
pub const ANTILINK: &str = "⚠ *Antilink* — @%user% mengirim link.\nPesan dihapus.";
pub const ANTILINK_KICK: &str = "⚠ *Antilink* — @%user% di-kick karena mengirim link.";
pub const ANTILINK_GC: &str = "⚠ *Antilink WA* — @%user% mengirim link WA.\nPesan dihapus.";
pub const ANTILINK_GC_KICK: &str = "⚠ *Antilink WA* — @%user% di-kick karena mengirim link WA.";
pub const ANTILINK_ALL: &str = "⚠ *Antilink* — @%user% mengirim link.\nPesan dihapus.";
pub const ANTILINK_ALL_KICK: &str = "⚠ *Antilink* — @%user% di-kick karena mengirim link.";
pub const ANTIHIDETAG: &str = "⚠ *AntiHidetag* — Hidetag dari @%user% dihapus.";
pub const ANTIDOCUMENT: &str = "⚠ *AntiDocument* — Dokumen dari @%user% dihapus.";

/// `gpMsg(key, { user })` — ganti `%user%` dengan nomor tanpa domain.
fn msg(template: &str, sender_tag: &str) -> String {
    template.replace("%user%", sender_tag)
}

/// `m.sender.split("@")[0]`
fn sender_tag(sender: &str) -> String {
    sender.split('@').next().unwrap_or(sender).to_string()
}

// ---------------------------------------------------------------------------
// Cek admin (port `adminFlagsFor` serialize.js:140, `isSameParticipant` lid.js:454)
// ---------------------------------------------------------------------------

/// Digit saja dari sebuah JID — `replace(/[^0-9]/g, "")`.
fn digits(value: &str) -> String {
    value.chars().filter(char::is_ascii_digit).collect()
}

/// `_numHit` (serialize.js:118): exact dulu, lalu fuzzy suffix hanya kalau
/// kedua sisi panjangnya >= 8. Tanpa guard itu, nomor pendek ("1") akan cocok
/// dengan semua participant dan semua orang jadi admin.
fn num_hit(candidate_nums: &[String], num: &str) -> bool {
    if num.is_empty() {
        return false;
    }
    if candidate_nums.iter().any(|n| n == num) {
        return true;
    }
    if num.len() < 8 {
        return false;
    }
    candidate_nums
        .iter()
        .filter(|n| n.len() >= 8)
        .any(|n| n.ends_with(num) || num.ends_with(n.as_str()))
}

/// `_participantNums` (serialize.js:101) tanpa cabang LID→PN yang butuh cache.
fn participant_nums(p: &Participant) -> Vec<String> {
    let mut nums: Vec<String> = Vec::new();
    let mut add = |v: &str| {
        let d = digits(v);
        if !d.is_empty() && !nums.contains(&d) {
            nums.push(d);
        }
    };
    add(&p.id);
    add(&p.lid);
    add(&p.phone_number);
    nums
}

/// `adminFlagsFor(...).isAdmin` — sender cukup salah satu kandidat yang
/// match participant admin.
fn is_admin_check(participants: &[Participant], participant: Option<&str>, sender: &str) -> bool {
    let candidates: Vec<&str> = participant.into_iter().chain(std::iter::once(sender)).collect();
    let sender_nums: Vec<String> = candidates
        .iter()
        .map(|c| digits(c))
        .filter(|d| !d.is_empty())
        .collect();
    if sender_nums.is_empty() {
        return false;
    }
    participants.iter().any(|p| {
        p.admin && {
            let nums = participant_nums(p);
            sender_nums.iter().any(|sn| num_hit(&nums, sn))
        }
    })
}

/// `adminFlagsFor(...).isBotAdmin` — bot harus cocok ke `bot_number` atau
/// `bot_lid`, supaya grup addressing LID tetap terdeteksi (lihat komentar
/// serialize.js:259-268).
fn is_bot_admin_check(env: &GateEnv) -> bool {
    let mut bot_nums: Vec<String> = vec![digits(&env.bot_number)];
    if let Some(lid) = &env.bot_lid {
        bot_nums.push(digits(lid));
    }
    bot_nums.retain(|d| !d.is_empty());
    if bot_nums.is_empty() {
        return false;
    }
    env.participants.iter().any(|p| {
        p.admin && {
            let nums = participant_nums(p);
            bot_nums.iter().any(|bn| num_hit(&nums, bn))
        }
    })
}

/// `normalizeComparableJid` (lid.js:438) TANPA konversi LID→PN (butuh cache).
/// Yang performed: buang suffix device `:12` pada JID `@s.whatsapp.net`.
fn normalize_comparable_jid(jid: &str) -> String {
    let value = jid.trim();
    if value.is_empty() {
        return String::new();
    }
    if value.contains(':') && value.ends_with("@s.whatsapp.net") {
        return format!("{}@s.whatsapp.net", value.split(':').next().unwrap_or(value));
    }
    value.to_string()
}

/// `isSameParticipant` (lid.js:454) tanpa cabang LID→PN.
fn is_same_participant(left: &str, right: &str) -> bool {
    let l = normalize_comparable_jid(left);
    let r = normalize_comparable_jid(right);
    if l.is_empty() || r.is_empty() {
        return false;
    }
    if l == r {
        return true;
    }
    let (ln, rn) = (digits(&l), digits(&r));
    if ln.is_empty() || rn.is_empty() {
        return false;
    }
    ln == rn || ln.ends_with(&rn) || rn.ends_with(&ln)
}

/// `mutemember.js:102` `isMutedMember` — cocok longgar via `endsWith`, sama
/// seperti JS (memakai JID tersimpan apa adanya, termasuk yang sudah ke-LID).
fn is_muted_member(muted: &[String], sender: &str) -> bool {
    let strip = |v: &str| v.split('@').next().unwrap_or(v).to_string();
    let sender_number = strip(sender);
    muted.iter().any(|jid| {
        let c = strip(jid);
        c == sender_number || c.ends_with(&sender_number) || sender_number.ends_with(&c)
    })
}

// ---------------------------------------------------------------------------
// Deteksi link — padanan 3 regex di group-protection.js:91-94, 1857-1866
// ---------------------------------------------------------------------------

// `ponytail:` crate `regex` tidak ada di dependency oktz-router dan
// Cargo.toml di luar file yang boleh diubah, jadi ketiga regex ditulis ulang
// sebagai scanner. Sudah dicocokkan dengan regex aslinya di node lewat test
// `link_detection_matches_js_regex`; kalau regex crate masuk workspace,
// ganti ketiganya dengan `Regex::new` persis seperti JS dan hapus `TLDS`.

/// TLD untuk `ALL_LINK_PATTERN` (group-protection.js:1866).
const TLDS: &[&str] = &[
    "com", "net", "org", "io", "id", "co", "cc", "me", "ly", "gg", "gl", "it", "tv", "ru", "de",
    "fr", "uk", "us", "info", "biz", "xyz", "top", "site", "online", "store", "app", "dev", "ai",
    "link", "click", "fun", "space", "live", "world", "tech", "digital", "cloud", "pro", "vip",
    "pw", "tk", "ml", "ga", "cf", "gq", "club", "mobi", "name", "asia", "tel", "wang", "win",
    "bid", "loan", "ren", "ink", "art", "shop", "life", "game", "news", "blog", "design", "studio",
    "photo", "press", "media", "social", "group", "team", "community", "domain",
];

/// Karakter sebelum `.` pada `LINK_REGEX`: `[-a-zA-Z0-9@:%._+~#=]`.
fn is_link_lead(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '@' | ':' | '%' | '.' | '_' | '+' | '~' | '#' | '=')
}

/// Karakter TLD pada `LINK_REGEX`: `[a-zA-Z0-9()]`.
fn is_link_tld(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '(' || c == ')'
}

/// Char `\w` untuk uji word-boundary `\b`.
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `LINK_REGEX` (group-protection.js:91) — bentuk aslinya
/// `(?:https?:\/\/)?(?:www\.)?[-a-zA-Z0-9@:%._+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b[-a-zA-Z0-9()@:%_+.~#?&//=]*`
///
/// Yang menentukan: ada `.` yang didahului char lead, diikuti 1–6 char TLD,
/// lalu `\b`. Run TLD di-backtrack dari terpanjang ke terpendek karena `\b`
/// bisa gagal untuk run terpanjang tapi lolos untuk yang lebih pendek.
fn contains_link(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    for i in 1..chars.len() {
        if chars[i] != '.' || !is_link_lead(chars[i - 1]) {
            continue;
        }
        let mut run = 0;
        while run < 6 && i + 1 + run < chars.len() && is_link_tld(chars[i + 1 + run]) {
            run += 1;
        }
        // Backtrack dari run terpanjang ke terpendek; `\b` harus berlaku.
        // `\b` = tepat satu sisi yang `\w`. Ujung string dianggap NON-word,
        // jadi "com" di akhir teks tetap boundary.
        for take in (1..=run).rev() {
            let j = i + 1 + take;
            let before_is_word = is_word(chars[j - 1]);
            let after_is_word = chars.get(j).is_some_and(|c| is_word(*c));
            if before_is_word != after_is_word {
                return true;
            }
        }
    }
    false
}

/// `containsWaSpecificLink` (group-protection.js:1868) — 5 pola, case-insensitive.
fn contains_wa_specific_link(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    // (prefix, suffix harus alphanumeric kecuali api.whatsapp.com/send)
    const PREFIXES: &[(&str, bool)] = &[
        ("chat.whatsapp.com/", true),
        ("wa.me/", true),
        ("whatsapp.com/channel/", true),
        ("whatsapp.com/c/", true),
        ("api.whatsapp.com/send", false),
    ];
    for (prefix, need_tail) in PREFIXES {
        let mut from = 0;
        while let Some(rel) = lower[from..].find(prefix) {
            let tail = &lower[from + rel + prefix.len()..];
            if !need_tail || tail.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) {
                return true;
            }
            from += rel + 1;
        }
    }
    false
}

/// `containsAnyLink` (group-protection.js:1878) — `ALL_LINK_PATTERN`.
///
/// Char sebelum `.` harus ada dan bukan salah satu karakter yang dikecualikan
/// (`\s<>"{}|\^`[]`); setelah `.` harus salah satu TLD di [`TLDS`].
fn contains_any_link(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    for i in 1..chars.len() {
        if chars[i] != '.' {
            continue;
        }
        let prev = chars[i - 1];
        if prev.is_whitespace() || matches!(prev, '<' | '>' | '"' | '{' | '}' | '|' | '\\' | '^' | '`' | '[' | ']')
        {
            continue;
        }
        let rest: String = chars[i + 1..].iter().collect::<String>().to_ascii_lowercase();
        if TLDS.iter().any(|tld| rest.starts_with(tld)) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOT: &str = "628999@s.whatsapp.net";

    fn admin_participant(num: &str) -> Participant {
        Participant {
            id: format!("{num}@s.whatsapp.net"),
            admin: true,
            ..Default::default()
        }
    }

    fn member_participant(num: &str) -> Participant {
        Participant {
            id: format!("{num}@s.whatsapp.net"),
            ..Default::default()
        }
    }

    fn caller() -> GroupCaller {
        GroupCaller {
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            // `m.isBotAdmin` — flag terpisah dari `isBotAdminCheck(participants)`
            // yang dipakai gate antilink. `env()` di bawah Record bot sebagai
            // admin, jadi default-nya `true` supaya konsisten.
            is_bot_admin: true,
            ..Default::default()
        }
    }

    /// Env standar: bot adalah admin, sender bukan admin.
    fn env() -> GateEnv {
        GateEnv {
            bot_number: BOT.into(),
            participants: vec![
                admin_participant("628999"),
                admin_participant("628222"),
                member_participant("628111"),
                member_participant("628333"),
            ],
            now_ms: 100_000,
            ..Default::default()
        }
    }

    fn input<'a>(
        c: &'a GroupCaller,
        s: &'a GroupSettings,
        e: &'a GateEnv,
    ) -> GateInput<'a> {
        GateInput {
            caller: c,
            settings: s,
            env: e,
            last_message_ms: 0,
        }
    }

    /// Jalankan satu gate langsung, tanpa framework.
    fn run_one(gate: GateFn, c: &GroupCaller, s: &GroupSettings, e: &GateEnv) -> GateOutcome {
        gate(&input(c, s, e))
    }

    // ==================================================================
    // Framework
    // ==================================================================

    #[test]
    fn non_group_message_skips_whole_chain() {
        let mut c = caller();
        c.is_group = false;
        let s = GroupSettings {
            antilink: Some("on".into()),
            ..Default::default()
        };
        let e = env();
        assert_eq!(default_gate_chain().run(&input(&c, &s, &e)), Decision::Allow);
    }

    #[test]
    fn newsletter_skips_whole_chain() {
        let mut c = caller();
        c.is_newsletter = true;
        c.body = "cek https://spam.com".into();
        let s = GroupSettings {
            antilink: Some("on".into()),
            ..Default::default()
        };
        let e = env();
        assert_eq!(default_gate_chain().run(&input(&c, &s, &e)), Decision::Allow);
    }

    #[test]
    fn nothing_configured_allows_everything() {
        let mut c = caller();
        c.body = "halo cek https://x.com sama wa.me/12345".into();
        let s = GroupSettings::default();
        let e = env();
        assert_eq!(default_gate_chain().run(&input(&c, &s, &e)), Decision::Allow);
    }

    #[test]
    fn default_chain_order_matches_handler_js() {
        assert_eq!(
            default_gate_chain().gate_names(),
            vec![
                "antilink",      // handler.js:782
                "antilinkgc",    // :785
                "antilinkall",   // :788
                "antihidetag",   // :791
                "antidocument",  // :797
                "mute",          // :809
                "mutedmember",   // :818
                "mutegc",        // :827
                "slowmode",      // :844
            ]
        );
    }

    // ---- ORDER: dua gate yang sama-sama match → yang DULU menang ----

    #[test]
    fn order_first_gate_wins_antilink_beats_hidetag() {
        // Satu pesan berisi link DAN hidetag semua member: antilink (:782)
        // dijalankan sebelum hidetag (:791), jadi yang tampil pesan antilink.
        let mut c = caller();
        c.body = "https://spam.com".into();
        c.mentioned_jid = vec!["1@s.whatsapp.net".into(); 4];
        c.participant_count = 4;
        let s = GroupSettings {
            antilink: Some("on".into()),
            antihidetag: Some("on".into()),
            ..Default::default()
        };
        let e = env();
        let d = default_gate_chain().run(&input(&c, &s, &e));
        assert_eq!(
            d,
            Decision::DeleteAndReply(ANTILINK.replace("%user%", "628111"))
        );
        // Bukan pesan hidetag.
        assert!(!d.reply_text().unwrap().contains("AntiHidetag"));
    }

    #[test]
    fn order_hidetag_wins_when_antilink_does_not_match() {
        // Hidetag saja: antilink lolos (toggle on tapi tidak ada link), jadi
        // gate berikutnya yang	match.
        let mut c = caller();
        c.body = "halo semua".into();
        c.mentioned_jid = vec!["1@s.whatsapp.net".into(); 4];
        c.participant_count = 4;
        let s = GroupSettings {
            antilink: Some("on".into()),
            antihidetag: Some("on".into()),
            ..Default::default()
        };
        let e = env();
        let d = default_gate_chain().run(&input(&c, &s, &e));
        assert_eq!(
            d,
            Decision::DeleteAndReply(ANTIHIDETAG.replace("%user%", "628111"))
        );
    }

    #[test]
    fn order_mute_wins_over_slowmode() {
        // mute (:809) di sebelum slowmode (:844); keduanya match.
        let c = caller();
        let s = GroupSettings {
            mute: true,
            slowmode: Some(Slowmode {
                enabled: true,
                delay_secs: Some(30),
                mode: Some("all".into()),
            }),
            ..Default::default()
        };
        let e = env();
        // slowmode butuh last_message_ms baru; default 0 = sudah lama → lolos.
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms;
        assert_eq!(
            default_gate_chain().run(&i),
            Decision::DeleteAndStop,
            "gate mute lebih dulu, slowmode tidak boleh ikut dievaluasi"
        );
    }

    #[test]
    fn order_slowmode_wins_when_mute_off() {
        let c = caller();
        let s = GroupSettings {
            slowmode: Some(Slowmode {
                enabled: true,
                delay_secs: Some(30),
                mode: Some("all".into()),
            }),
            ..Default::default()
        };
        let e = env();
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms;
        assert_eq!(default_gate_chain().run(&i), Decision::DeleteAndStop);
    }

    #[test]
    fn order_antidocument_wins_over_mute() {
        // antidocument (:797) reply, mute (:809) stop senyap. Yang lebih dulu
        // menentukan apa yang dilihat user: balasan.
        let mut c = caller();
        c.is_document = true;
        let s = GroupSettings {
            antidocument: true,
            mute: true,
            ..Default::default()
        };
        let e = env();
        let d = default_gate_chain().run(&input(&c, &s, &e));
        assert!(d.reply_text().unwrap().contains("AntiDocument"));
    }

    #[test]
    fn order_mutegc_wins_over_slowmode() {
        let c = caller();
        let s = GroupSettings {
            mutegc: true,
            slowmode: Some(Slowmode {
                enabled: true,
                delay_secs: Some(30),
                mode: Some("all".into()),
            }),
            ..Default::default()
        };
        let e = env();
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms;
        assert_eq!(
            default_gate_chain().run(&i),
            Decision::Stop,
            "mutegc (:827) tidak delete, slowmode (:844) akan delete"
        );
    }

    #[test]
    fn order_muted_member_wins_over_mutegc() {
        let c = caller();
        let s = GroupSettings {
            muted_members: vec!["628111@s.whatsapp.net".into()],
            mutegc: true,
            ..Default::default()
        };
        let e = env();
        assert_eq!(default_gate_chain().run(&input(&c, &s, &e)), Decision::DeleteAndStop);
    }

    #[test]
    fn order_antilink_gc_wins_over_antilink_all() {
        // ": wa.me/12345xyz" kena KEDUA gate. Gc (:785) lebih dulu.
        let mut c = caller();
        c.body = "wa.me/12345".into();
        let s = GroupSettings {
            antilinkgc: Some("on".into()),
            antilinkall: Some("on".into()),
            ..Default::default()
        };
        let e = env();
        let d = default_gate_chain().run(&input(&c, &s, &e));
        assert!(d.reply_text().unwrap().contains("Antilink WA"));
    }

    #[test]
    fn custom_chain_order_is_honoured() {
        // Framework, bukan urutan default, yang menentukan.
        let mut c = caller();
        c.is_document = true;
        let s = GroupSettings {
            antidocument: true,
            mute: true,
            ..Default::default()
        };
        let e = env();
        let reversed = GroupGuard::new(vec![
            Gate::new("mute", gate_mute),
            Gate::new("antidocument", gate_antidocument),
        ]);
        assert_eq!(
            reversed.run(&input(&c, &s, &e)),
            Decision::DeleteAndStop,
            "mute diletakkan duluan → mute yang menang"
        );
    }

    // ==================================================================
    // gate_antilink  (group-protection.js:1311)
    // ==================================================================

    #[test]
    fn antilink_off_lets_message_through() {
        let mut c = caller();
        c.body = "https://spam.com".into();
        let s = GroupSettings {
            antilink: Some("off".into()),
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_antilink, &c, &s, &env()),
            GateOutcome::Pass,
            "`!== \"on\"` — nilai lain termasuk None tidak menyalakan"
        );
    }

    #[test]
    fn antilink_on_without_link_passes() {
        let mut c = caller();
        c.body = "halo dunia".into();
        let s = GroupSettings {
            antilink: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(run_one(gate_antilink, &c, &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn antilink_deletes_and_replies() {
        let mut c = caller();
        c.body = "spesIAL cek https://spam.com sekarang".into();
        let s = GroupSettings {
            antilink: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_antilink, &c, &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndReply(
                ANTILINK.replace("%user%", "628111")
            ))
        );
    }

    #[test]
    fn antilink_kick_mode_returns_kick_decision() {
        let mut c = caller();
        c.body = "https://spam.com".into();
        let s = GroupSettings {
            antilink: Some("on".into()),
            antilink_mode: Some("kick".into()),
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_antilink, &c, &s, &env()),
            GateOutcome::Decide(Decision::KickAndReply(
                ANTILINK_KICK.replace("%user%", "628111")
            ))
        );
    }

    #[test]
    fn antilink_non_kick_mode_value_is_remove() {
        let mut c = caller();
        c.body = "https://spam.com".into();
        let s = GroupSettings {
            antilink: Some("on".into()),
            antilink_mode: Some("nonsense".into()),
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_antilink, &c, &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndReply(
                ANTILINK.replace("%user%", "628111")
            ))
        );
    }

    #[test]
    fn antilink_admin_is_exempt() {
        let mut c = caller();
        c.body = "https://spam.com".into();
        c.sender = "628222@s.whatsapp.net".into(); // admin di participants
        let s = GroupSettings {
            antilink: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(run_one(gate_antilink, &c, &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn antilink_exemption_uses_participant_not_sender() {
        // Sender bukan admin, tapi `m.key.participant` adalah admin.
        let mut c = caller();
        c.body = "https://spam.com".into();
        c.participant = Some("628222@s.whatsapp.net".into());
        let s = GroupSettings {
            antilink: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(run_one(gate_antilink, &c, &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn antilink_owner_without_admin_record_is_not_exempt() {
        // Gate ini cek admin dari participants, BUKAN m.isOwner. Owner yang
        // tidak tercatat admin tetap kena.
        let mut c = caller();
        c.body = "https://spam.com".into();
        c.is_owner = true;
        let s = GroupSettings {
            antilink: Some("on".into()),
            ..Default::default()
        };
        assert!(matches!(
            run_one(gate_antilink, &c, &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndReply(_))
        ));
    }

    #[test]
    fn antilink_partner_is_not_exempt() {
        let mut c = caller();
        c.body = "https://spam.com".into();
        c.is_partner = true;
        let s = GroupSettings {
            antilink: Some("on".into()),
            ..Default::default()
        };
        assert!(matches!(
            run_one(gate_antilink, &c, &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndReply(_))
        ));
    }

    #[test]
    fn antilink_replies_notadmin_when_bot_is_not_admin() {
        // Asimetri penting: gate ini REPLY, Gc/All senyap.
        let mut c = caller();
        c.body = "https://spam.com".into();
        let s = GroupSettings {
            antilink: Some("on".into()),
            ..Default::default()
        };
        let mut e = env();
        e.participants.retain(|p| !p.id.starts_with("628999"));
        assert_eq!(
            run_one(gate_antilink, &c, &s, &e),
            GateOutcome::Decide(Decision::Reply(NOT_ADMIN.into()))
        );
    }

    #[test]
    fn antilink_skips_bots_own_message_even_in_self_mode() {
        // handleAntilink TIDAK mengecek selfMode (:1327-1328) — berbeda dari
        // Gc/All. Kalau di-port dengan cek selfMode, bot di selfMode akan
        // mulai merevoke pesan outputnya sendiri.
        let mut c = caller();
        c.body = "https://spam.com".into();
        c.sender = BOT.into();
        let s = GroupSettings {
            antilink: Some("on".into()),
            ..Default::default()
        };
        let mut e = env();
        e.self_mode = true;
        assert_eq!(run_one(gate_antilink, &c, &s, &e), GateOutcome::Pass);
    }

    #[test]
    fn antilink_matches_bot_via_device_suffix() {
        let mut c = caller();
        c.body = "https://spam.com".into();
        c.sender = "628999@s.whatsapp.net".into();
        c.participant = Some("628999:12@s.whatsapp.net".into());
        let s = GroupSettings {
            antilink: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(run_one(gate_antilink, &c, &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn antilink_bot_lid_also_counts_for_bot_admin_check() {
        // Grup addressing LID: bot tercatat admin cuma lewat field `lid`.
        let mut c = caller();
        c.body = "https://spam.com".into();
        let s = GroupSettings {
            antilink: Some("on".into()),
            ..Default::default()
        };
        let mut e = env();
        e.participants.retain(|p| !p.id.starts_with("628999"));
        e.participants.push(Participant {
            lid: "111222333@s.lid".into(),
            admin: true,
            ..Default::default()
        });
        e.bot_lid = Some("111222333@s.lid".into());
        assert!(matches!(
            run_one(gate_antilink, &c, &s, &e),
            GateOutcome::Decide(Decision::DeleteAndReply(_))
        ));
    }

    // ==================================================================
    // gate_antilink_gc  (group-protection.js:1888)
    // ==================================================================

    #[test]
    fn antilink_gc_triggers_on_whatsapp_link() {
        let mut c = caller();
        c.body = "cek https://chat.whatsapp.com/AbCd1234".into();
        let s = GroupSettings {
            antilinkgc: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_antilink_gc, &c, &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndReply(
                ANTILINK_GC.replace("%user%", "628111")
            ))
        );
    }

    #[test]
    fn antilink_gc_ignores_non_whatsapp_link() {
        let mut c = caller();
        c.body = "cek https://contoh.com".into();
        let s = GroupSettings {
            antilinkgc: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(run_one(gate_antilink_gc, &c, &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn antilink_gc_ignores_bare_prefix_without_payload() {
        let mut c = caller();
        c.body = "chat.whatsapp.com/".into();
        let s = GroupSettings {
            antilinkgc: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(run_one(gate_antilink_gc, &c, &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn antilink_gc_matches_api_send_without_payload() {
        let mut c = caller();
        c.body = "api.whatsapp.com/send".into();
        let s = GroupSettings {
            antilinkgc: Some("on".into()),
            ..Default::default()
        };
        assert!(matches!(
            run_one(gate_antilink_gc, &c, &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndReply(_))
        ));
    }

    #[test]
    fn antilink_gc_kick_mode() {
        let mut c = caller();
        c.body = "wa.me/62811".into();
        let s = GroupSettings {
            antilinkgc: Some("on".into()),
            antilinkgc_mode: Some("kick".into()),
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_antilink_gc, &c, &s, &env()),
            GateOutcome::Decide(Decision::KickAndReply(
                ANTILINK_GC_KICK.replace("%user%", "628111")
            ))
        );
    }

    #[test]
    fn antilink_gc_admin_exempt() {
        let mut c = caller();
        c.body = "wa.me/12345".into();
        c.sender = "628222@s.whatsapp.net".into();
        let s = GroupSettings {
            antilinkgc: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(run_one(gate_antilink_gc, &c, &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn antilink_gc_silent_when_bot_not_admin() {
        // :1909 `if (!isBotAdminCheck(...)) return false;` — SENYAP. Kalau
        // diubah jadi reply, user tetap lihat pesan bot.
        let mut c = caller();
        c.body = "wa.me/12345".into();
        let s = GroupSettings {
            antilinkgc: Some("on".into()),
            ..Default::default()
        };
        let mut e = env();
        e.participants.retain(|p| !p.id.starts_with("628999"));
        assert_eq!(
            run_one(gate_antilink_gc, &c, &s, &e),
            GateOutcome::Pass,
            "harus senyap, bukan reply notAdmin"
        );
    }

    #[test]
    fn antilink_gc_never_acts_on_bots_own_message() {
        // Cek ulang dengan rekonstruksi JS di node: cabang `selfMode`
        // (:1900) TIDAK mengubah hasil. Kalau bot admin → cek admin (:1908)
        //-exempt; kalau bot bukan admin → cek bot-admin gagal dan gate
        // senyap (:1909). Dua-duanya `return false`.
        //
        // Diuji sebagai invarian, bukan sebagai perbedaan selfMode, karena
        // hasilnya tidak terlihat dari `Decision` — tapi CABUTNYA cek
        // selfMode akan salah kalau suatu saat urutan cek berubah.
        let s = GroupSettings {
            antilinkgc: Some("on".into()),
            ..Default::default()
        };
        let mut c = caller();
        c.body = "wa.me/12345".into();
        c.sender = BOT.into();

        for self_mode in [false, true] {
            let mut e = env();
            e.self_mode = self_mode;
            assert_eq!(
                run_one(gate_antilink_gc, &c, &s, &e),
                GateOutcome::Pass,
                "self_mode={self_mode}: pesan bot sendiri tidak pernah kena"
            );
        }
    }

    #[test]
    fn antilink_gc_self_sender_skip_is_load_bearing_in_antilink() {
        // Kebalikan dari test di atas: `handleAntilink` TIDAK punya cek
        // selfMode sama sekali. Kalau cek self-sender-nya dihapus, pesan bot
        // sendiri dengan bot non-admin akan jatuh ke cabang "reply notAdmin"
        // — bottegur dirinya sendiri. Test ini mengunci perbedaan itu.
        let s = GroupSettings {
            antilink: Some("on".into()),
            ..Default::default()
        };
        let mut c = caller();
        c.body = "https://spam.com".into();
        c.sender = BOT.into();
        let mut e = env();
        e.participants.retain(|p| !p.id.starts_with("628999"));

        assert_eq!(
            run_one(gate_antilink, &c, &s, &e),
            GateOutcome::Pass,
            "tanpa cek self-sender, hasilnya akan Reply(notAdmin)"
        );
        // Bandingkan: gate GC SENYAP di kondisi yang sama.
        let s_gc = GroupSettings {
            antilinkgc: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(run_one(gate_antilink_gc, &c, &s_gc, &e), GateOutcome::Pass);
    }

    // ==================================================================
    // gate_antilink_all  (group-protection.js:1943)
    // ==================================================================

    #[test]
    fn antilink_all_triggers_on_any_tld_link() {
        let mut c = caller();
        c.body = "cek www.contoh.co.id/page".into();
        let s = GroupSettings {
            antilinkall: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_antilink_all, &c, &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndReply(
                ANTILINK_ALL.replace("%user%", "628111")
            ))
        );
    }

    #[test]
    fn antilink_all_ignores_unknown_tld() {
        let mut c = caller();
        c.body = "cek file.inventi".into();
        let s = GroupSettings {
            antilinkall: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(run_one(gate_antilink_all, &c, &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn antilink_all_kick_mode() {
        let mut c = caller();
        c.body = "x.dev".into();
        let s = GroupSettings {
            antilinkall: Some("on".into()),
            antilinkall_mode: Some("kick".into()),
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_antilink_all, &c, &s, &env()),
            GateOutcome::Decide(Decision::KickAndReply(
                ANTILINK_ALL_KICK.replace("%user%", "628111")
            ))
        );
    }

    #[test]
    fn antilink_all_admin_exempt() {
        let mut c = caller();
        c.body = "x.dev".into();
        c.sender = "628222@s.whatsapp.net".into();
        let s = GroupSettings {
            antilinkall: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(run_one(gate_antilink_all, &c, &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn antilink_all_silent_when_bot_not_admin() {
        let mut c = caller();
        c.body = "x.dev".into();
        let s = GroupSettings {
            antilinkall: Some("on".into()),
            ..Default::default()
        };
        let mut e = env();
        e.participants.retain(|p| !p.id.starts_with("628999"));
        assert_eq!(run_one(gate_antilink_all, &c, &s, &e), GateOutcome::Pass);
    }

    // ==================================================================
    // gate_hidetag  (group-protection.js:2145)
    // ==================================================================

    #[test]
    fn hidetag_full_mention_is_removed() {
        let mut c = caller();
        c.mentioned_jid = vec!["a@s.whatsapp.net".into(); 4];
        c.participant_count = 4;
        let s = GroupSettings {
            antihidetag: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_hidetag, &c, &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndReply(
                ANTIHIDETAG.replace("%user%", "628111")
            ))
        );
    }

    #[test]
    fn hidetag_partial_mention_passes() {
        let mut c = caller();
        c.mentioned_jid = vec!["a@s.whatsapp.net".into(); 3];
        c.participant_count = 4;
        let s = GroupSettings {
            antihidetag: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(run_one(gate_hidetag, &c, &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn hidetag_no_mention_passes() {
        let c = caller();
        let s = GroupSettings {
            antihidetag: Some("on".into()),
            ..Default::default()
        };
        assert_eq!(run_one(gate_hidetag, &c, &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn hidetag_off_passes() {
        let mut c = caller();
        c.mentioned_jid = vec!["a@s.whatsapp.net".into(); 4];
        c.participant_count = 4;
        let s = GroupSettings {
            antihidetag: Some("off".into()),
            ..Default::default()
        };
        assert_eq!(run_one(gate_hidetag, &c, &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn hidetag_admin_owner_fromme_all_exempt() {
        let s = GroupSettings {
            antihidetag: Some("on".into()),
            ..Default::default()
        };
        for flag in ["is_admin", "is_owner", "from_me"] {
            let mut c = caller();
            c.mentioned_jid = vec!["a@s.whatsapp.net".into(); 4];
            c.participant_count = 4;
            match flag {
                "is_admin" => c.is_admin = true,
                "is_owner" => c.is_owner = true,
                _ => c.from_me = true,
            }
            assert_eq!(
                run_one(gate_hidetag, &c, &s, &env()),
                GateOutcome::Pass,
                "{flag} harus exempt"
            );
        }
    }

    #[test]
    fn hidetag_partner_is_not_exempt() {
        let mut c = caller();
        c.mentioned_jid = vec!["a@s.whatsapp.net".into(); 4];
        c.participant_count = 4;
        c.is_partner = true;
        let s = GroupSettings {
            antihidetag: Some("on".into()),
            ..Default::default()
        };
        assert!(matches!(
            run_one(gate_hidetag, &c, &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndReply(_))
        ));
    }

    #[test]
    fn hidetag_silent_when_bot_not_admin() {
        let mut c = caller();
        c.mentioned_jid = vec!["a@s.whatsapp.net".into(); 4];
        c.participant_count = 4;
        c.is_bot_admin = false;
        let s = GroupSettings {
            antihidetag: Some("on".into()),
            ..Default::default()
        };
        let e = GateEnv {
            bot_number: BOT.into(),
            participants: vec![member_participant("628111")],
            ..Default::default()
        };
        assert_eq!(run_one(gate_hidetag, &c, &s, &e), GateOutcome::Pass);
    }

    #[test]
    fn hidetag_more_mentions_than_participants_still_triggers() {
        // `<` bukan `<=` — mention melebihi jumlah participant tetap kena.
        let mut c = caller();
        c.mentioned_jid = vec!["a@s.whatsapp.net".into(); 10];
        c.participant_count = 2;
        let s = GroupSettings {
            antihidetag: Some("on".into()),
            ..Default::default()
        };
        assert!(matches!(
            run_one(gate_hidetag, &c, &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndReply(_))
        ));
    }

    // ==================================================================
    // gate_antidocument  (plugins/group/antidocument.js:32)
    // ==================================================================

    #[test]
    fn antidocument_deletes_document_message() {
        let mut c = caller();
        c.is_document = true;
        let s = GroupSettings {
            antidocument: true,
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_antidocument, &c, &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndReply(
                ANTIDOCUMENT.replace("%user%", "628111")
            ))
        );
    }

    #[test]
    fn antidocument_detects_document_message_type() {
        for t in ["documentMessage", "documentWithCaptionMessage"] {
            let mut c = caller();
            c.message_type = t.into();
            let s = GroupSettings {
                antidocument: true,
                ..Default::default()
            };
            assert!(matches!(
                run_one(gate_antidocument, &c, &s, &env()),
                GateOutcome::Decide(Decision::DeleteAndReply(_))
            ), "{t} harus terdeteksi");
        }
    }

    #[test]
    fn antidocument_ignores_other_types() {
        let mut c = caller();
        c.message_type = "imageMessage".into();
        let s = GroupSettings {
            antidocument: true,
            ..Default::default()
        };
        assert_eq!(run_one(gate_antidocument, &c, &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn antidocument_off_passes() {
        let mut c = caller();
        c.is_document = true;
        let s = GroupSettings {
            antidocument: false,
            ..Default::default()
        };
        assert_eq!(run_one(gate_antidocument, &c, &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn antidocument_admin_owner_fromme_exempt() {
        // antidocument.js:34 — exemption ADA, dan dicek sebelum toggle.
        let s = GroupSettings {
            antidocument: true,
            ..Default::default()
        };
        for flag in ["is_admin", "is_owner", "from_me"] {
            let mut c = caller();
            c.is_document = true;
            match flag {
                "is_admin" => c.is_admin = true,
                "is_owner" => c.is_owner = true,
                _ => c.from_me = true,
            }
            assert_eq!(
                run_one(gate_antidocument, &c, &s, &env()),
                GateOutcome::Pass,
                "{flag} harus exempt"
            );
        }
    }

    #[test]
    fn antidocument_partner_is_not_exempt() {
        let mut c = caller();
        c.is_document = true;
        c.is_partner = true;
        let s = GroupSettings {
            antidocument: true,
            ..Default::default()
        };
        assert!(matches!(
            run_one(gate_antidocument, &c, &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndReply(_))
        ));
    }

    // ==================================================================
    // gate_mute  (handler.js:809 + mute.js:31)
    // ==================================================================

    #[test]
    fn mute_stops_and_deletes_when_bot_admin() {
        let s = GroupSettings {
            mute: true,
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_mute, &caller(), &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndStop)
        );
    }

    #[test]
    fn mute_stops_silently_when_bot_not_admin() {
        let s = GroupSettings {
            mute: true,
            ..Default::default()
        };
        let mut c = caller();
        c.is_bot_admin = false;
        let e = GateEnv {
            bot_number: BOT.into(),
            participants: vec![member_participant("628111")],
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_mute, &c, &s, &e),
            GateOutcome::Decide(Decision::Stop),
            "tanpa hak delete, JS tetap return tapi tidak hapus"
        );
    }

    #[test]
    fn mute_off_passes() {
        let s = GroupSettings {
            mute: false,
            ..Default::default()
        };
        assert_eq!(run_one(gate_mute, &caller(), &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn mute_admin_and_owner_exempt() {
        let s = GroupSettings {
            mute: true,
            ..Default::default()
        };
        for flag in ["is_admin", "is_owner"] {
            let mut c = caller();
            if flag == "is_admin" {
                c.is_admin = true;
            } else {
                c.is_owner = true;
            }
            assert_eq!(
                run_one(gate_mute, &c, &s, &env()),
                GateOutcome::Pass,
                "{flag} harus exempt"
            );
        }
    }

    #[test]
    fn mute_partner_is_not_exempt() {
        let mut c = caller();
        c.is_partner = true;
        let s = GroupSettings {
            mute: true,
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_mute, &c, &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndStop)
        );
    }

    // ==================================================================
    // gate_muted_member  (handler.js:818 + mutemember.js:102)
    // ==================================================================

    #[test]
    fn muted_member_blocked_and_deleted() {
        let s = GroupSettings {
            muted_members: vec!["628111@s.whatsapp.net".into()],
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_muted_member, &caller(), &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndStop)
        );
    }

    #[test]
    fn muted_member_matches_by_suffix() {
        let s = GroupSettings {
            muted_members: vec!["628111@s.whatsapp.net".into()],
            ..Default::default()
        };
        let mut c = caller();
        c.sender = "120628111@s.whatsapp.net".into();
        assert!(matches!(
            run_one(gate_muted_member, &c, &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndStop)
        ));
    }

    #[test]
    fn muted_member_other_user_passes() {
        let s = GroupSettings {
            muted_members: vec!["628999@s.whatsapp.net".into()],
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_muted_member, &caller(), &s, &env()),
            GateOutcome::Pass
        );
    }

    #[test]
    fn muted_member_empty_list_passes() {
        let s = GroupSettings::default();
        assert_eq!(
            run_one(gate_muted_member, &caller(), &s, &env()),
            GateOutcome::Pass
        );
    }

    #[test]
    fn muted_member_admin_owner_exempt() {
        let s = GroupSettings {
            muted_members: vec!["628111@s.whatsapp.net".into()],
            ..Default::default()
        };
        let mut c = caller();
        c.is_owner = true;
        assert_eq!(run_one(gate_muted_member, &c, &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn muted_member_partner_is_not_exempt() {
        let s = GroupSettings {
            muted_members: vec!["628111@s.whatsapp.net".into()],
            ..Default::default()
        };
        let mut c = caller();
        c.is_partner = true;
        assert_eq!(
            run_one(gate_muted_member, &c, &s, &env()),
            GateOutcome::Decide(Decision::DeleteAndStop)
        );
    }

    #[test]
    fn muted_member_silent_when_bot_not_admin() {
        let s = GroupSettings {
            muted_members: vec!["628111@s.whatsapp.net".into()],
            ..Default::default()
        };
        let mut c = caller();
        c.is_bot_admin = false;
        let e = GateEnv {
            bot_number: BOT.into(),
            participants: vec![member_participant("628111")],
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_muted_member, &c, &s, &e),
            GateOutcome::Decide(Decision::Stop)
        );
    }

    // ==================================================================
    // gate_mutegc  (handler.js:827 + mutegc.js:49)
    // ==================================================================

    #[test]
    fn mutegc_stops_without_deleting() {
        let s = GroupSettings {
            mutegc: true,
            ..Default::default()
        };
        assert_eq!(
            run_one(gate_mutegc, &caller(), &s, &env()),
            GateOutcome::Decide(Decision::Stop),
            "mutegc tidak pernah menghapus pesan"
        );
    }

    #[test]
    fn mutegc_off_passes() {
        let s = GroupSettings {
            mutegc: false,
            ..Default::default()
        };
        assert_eq!(run_one(gate_mutegc, &caller(), &s, &env()), GateOutcome::Pass);
    }

    #[test]
    fn mutegc_exempts_admin_owner_and_partner() {
        // Gate SATU-SATUNYA dengan tiga flag exemption (:827).
        let s = GroupSettings {
            mutegc: true,
            ..Default::default()
        };
        for flag in ["is_admin", "is_owner", "is_partner"] {
            let mut c = caller();
            match flag {
                "is_admin" => c.is_admin = true,
                "is_owner" => c.is_owner = true,
                _ => c.is_partner = true,
            }
            assert_eq!(
                run_one(gate_mutegc, &c, &s, &env()),
                GateOutcome::Pass,
                "{flag} harus exempt"
            );
        }
    }

    #[test]
    fn mutegc_ignores_bot_admin_and_from_me() {
        // Berbeda dari antidocument/hidetag: `fromMe` TIDAK exempt di sini.
        let s = GroupSettings {
            mutegc: true,
            ..Default::default()
        };
        let mut c = caller();
        c.is_bot_admin = true;
        c.from_me = true;
        assert_eq!(
            run_one(gate_mutegc, &c, &s, &env()),
            GateOutcome::Decide(Decision::Stop)
        );
    }

    // ==================================================================
    // gate_slowmode  (handler.js:844 + slowmode.js:120)
    // ==================================================================

    fn slowmode_settings(mode: &str) -> GroupSettings {
        GroupSettings {
            slowmode: Some(Slowmode {
                enabled: true,
                delay_secs: Some(30),
                mode: Some(mode.into()),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn slowmode_deletes_when_too_fast() {
        let s = slowmode_settings("all");
        let e = env();
        let c = caller();
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms - 1_000; // 1s < 30s
        assert_eq!(gate_slowmode(&i), GateOutcome::Decide(Decision::DeleteAndStop));
    }

    #[test]
    fn slowmode_onlycommand_stops_silently_without_deleting() {
        // handler.js:849 — `return` tanpa delete. Pesan TIDAK hilang.
        let s = slowmode_settings("onlycommand");
        let e = env();
        let mut c = caller();
        c.is_command = true;
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms;
        assert_eq!(gate_slowmode(&i), GateOutcome::Decide(Decision::Stop));
    }

    #[test]
    fn slowmode_onlycommand_passes_non_command_messages() {
        // slowmode.js:129 — `mode === "onlycommand" && !m.isCommand` → false.
        let s = slowmode_settings("onlycommand");
        let e = env();
        let c = caller(); // is_command = false
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms;
        assert_eq!(gate_slowmode(&i), GateOutcome::Pass);
    }

    #[test]
    fn slowmode_default_mode_is_all() {
        let s = GroupSettings {
            slowmode: Some(Slowmode {
                enabled: true,
                delay_secs: Some(30),
                mode: None,
            }),
            ..Default::default()
        };
        let e = env();
        let c = caller();
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms;
        assert_eq!(gate_slowmode(&i), GateOutcome::Decide(Decision::DeleteAndStop));
    }

    #[test]
    fn slowmode_default_delay_is_30_seconds() {
        let s = GroupSettings {
            slowmode: Some(Slowmode {
                enabled: true,
                delay_secs: None,
                mode: Some("all".into()),
            }),
            ..Default::default()
        };
        let e = env();
        // 29s → masih diblokir.
        let c = caller();
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms - 29_000;
        assert_eq!(gate_slowmode(&i), GateOutcome::Decide(Decision::DeleteAndStop));
        // 31s → lolos.
        i.last_message_ms = e.now_ms - 31_000;
        assert_eq!(gate_slowmode(&i), GateOutcome::Pass);
    }

    #[test]
    fn slowmode_boundary_is_strictly_less_than() {
        let s = slowmode_settings("all");
        let e = env();
        let c = caller();
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms - 30_000; // tepat 30s
        assert_eq!(gate_slowmode(&i), GateOutcome::Pass, "tepat = lolos");
        i.last_message_ms = e.now_ms - 29_999;
        assert_eq!(gate_slowmode(&i), GateOutcome::Decide(Decision::DeleteAndStop));
    }

    #[test]
    fn slowmode_disabled_passes() {
        let s = GroupSettings {
            slowmode: Some(Slowmode {
                enabled: false,
                delay_secs: Some(0),
                mode: Some("all".into()),
            }),
            ..Default::default()
        };
        let e = env();
        let c = caller();
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms;
        assert_eq!(gate_slowmode(&i), GateOutcome::Pass);
    }

    #[test]
    fn slowmode_missing_config_passes() {
        let s = GroupSettings::default();
        let e = env();
        let c = caller();
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms;
        assert_eq!(gate_slowmode(&i), GateOutcome::Pass);
    }

    #[test]
    fn slowmode_admin_and_owner_exempt() {
        let s = slowmode_settings("all");
        let e = env();
        for flag in ["is_admin", "is_owner"] {
            let mut c = caller();
            if flag == "is_admin" {
                c.is_admin = true;
            } else {
                c.is_owner = true;
            }
            let mut i = input(&c, &s, &e);
            i.last_message_ms = e.now_ms;
            assert_eq!(gate_slowmode(&i), GateOutcome::Pass, "{flag} harus exempt");
        }
    }

    #[test]
    fn slowmode_partner_and_fromme_are_not_exempt() {
        let s = slowmode_settings("all");
        let e = env();
        for flag in ["is_partner", "from_me"] {
            let mut c = caller();
            if flag == "is_partner" {
                c.is_partner = true;
            } else {
                c.from_me = true;
            }
            let mut i = input(&c, &s, &e);
            i.last_message_ms = e.now_ms;
            assert_eq!(
                gate_slowmode(&i),
                GateOutcome::Decide(Decision::DeleteAndStop),
                "{flag} TIDAK exempt di slowmode"
            );
        }
    }

    // ---- efek samping lastMessageTime.set() ----------------------------

    #[test]
    fn slowmode_records_timestamp_when_time_has_passed() {
        let s = slowmode_settings("all");
        let e = env();
        let c = caller();
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms - 60_000;
        let (_, record) = default_gate_chain().run_with_record(&i);
        assert!(record, "waktu cukup → caller harus set lastMessageTime");
    }

    #[test]
    fn slowmode_does_not_record_when_blocked() {
        let s = slowmode_settings("all");
        let e = env();
        let c = caller();
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms;
        let (d, record) = default_gate_chain().run_with_record(&i);
        assert_eq!(d, Decision::DeleteAndStop);
        assert!(!record, "diblokir → tidak ada pencatatan waktu");
    }

    #[test]
    fn slowmode_does_not_record_for_exempt_users() {
        let s = slowmode_settings("all");
        let e = env();
        let mut c = caller();
        c.is_owner = true;
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms - 60_000;
        let (_, record) = default_gate_chain().run_with_record(&i);
        assert!(!record, "exempt tidak masuk slowmode");
    }

    #[test]
    fn slowmode_does_not_record_onlycommand_non_command() {
        let s = slowmode_settings("onlycommand");
        let e = env();
        let c = caller();
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms - 60_000;
        let (_, record) = default_gate_chain().run_with_record(&i);
        assert!(!record, "early-return sebelum cek waktu");
    }

    #[test]
    fn no_record_when_a_prior_gate_blocked() {
        let mut c = caller();
        c.body = "https://spam.com".into();
        let s = GroupSettings {
            antilink: Some("on".into()),
            ..slowmode_settings("all")
        };
        let e = env();
        let mut i = input(&c, &s, &e);
        i.last_message_ms = e.now_ms - 60_000;
        let (d, record) = default_gate_chain().run_with_record(&i);
        assert!(matches!(d, Decision::DeleteAndReply(_)));
        assert!(!record, "chain berhenti di antilink → slowmode tidak jalan");
    }

    // ==================================================================
    // link detection
    // ==================================================================

    #[test]
    fn contains_link_positives() {
        for t in [
            "example.com",
            "https://example.com",
            "http://www.example.co.id/path?a=1",
            "coba vk.com/x",
            "halo 3.14 itu angka",
            "user@host.tld",
        ] {
            assert!(contains_link(t), "harusnya link: {t}");
        }
    }

    #[test]
    fn contains_link_negatives() {
        for t in [
            "",
            "halo dunia",
            "tapi.",
            "gimana kabar?",
            "selamat pagi",
            "3.",
        ] {
            assert!(!contains_link(t), "harusnya bukan link: {t}");
        }
    }

    #[test]
    fn contains_wa_specific_link_matches_each_pattern() {
        for t in [
            "chat.whatsapp.com/AbCd",
            "wa.me/628123",
            "whatsapp.com/channel/ABC",
            "whatsapp.com/c/ABC",
            "api.whatsapp.com/send",
        ] {
            assert!(contains_wa_specific_link(t), "harusnya cocok: {t}");
        }
    }

    #[test]
    fn contains_wa_specific_link_is_case_insensitive() {
        assert!(contains_wa_specific_link("CHAT.WHATSAPP.COM/ABCD"));
        assert!(contains_wa_specific_link("WA.ME/628"));
    }

    #[test]
    fn contains_wa_specific_link_negatives() {
        for t in [
            "",
            "https://facebook.com",
            "chat.whatsapp.com",
            "chat.whatsapp.com/",
            "wa.me/",
            "whatsapp.com/channel/",
        ] {
            assert!(!contains_wa_specific_link(t), "harusnya tidak cocok: {t}");
        }
    }

    #[test]
    fn contains_any_link_uses_tld_allowlist() {
        for t in ["x.com", "a.b.co.id", "situs.xyz", "toko.shop", "main.ink"] {
            assert!(contains_any_link(t), "harusnya link: {t}");
        }
        for t in ["halo", "file.inventi", "3.5", "a.b_unknown"] {
            assert!(!contains_any_link(t), "harusnya bukan link: {t}");
        }
    }

    #[test]
    fn contains_any_link_excluded_chars_only_stop_at_the_boundary() {
        // Dicek dengan regex aslinya di node: char terlarang sebelum `.`
        // menghentikan scan di posisi itu, tapi regex tetap bisa mulai
        // SETELAH char itu — jadi "<x.com" tetap match.
        assert!(contains_any_link("<x.com"));
        assert!(contains_any_link("\"x.com"));
        assert!(contains_any_link("normal.com"));
        // Yang benar-benar tidak match: char terlarang tepat sebelum `.`.
        assert!(!contains_any_link("a<.com"));
    }

    /// `(teks, contains_link, contains_wa_specific_link, contains_any_link)`
    /// hasil `node` atas regex JS asli (group-protection.js:91-94, 1857-1866).
    ///
    /// Tabel ini mengikat scanner tulisan-tangan ke perilaku regex asli.
    /// Scanner bisa disederhanakan selama tabel ini tetap hijau.
    const JS_LINK_TABLE: &[(&str, bool, bool, bool)] = &[
        ("example.com", true, false, true),
        ("https://example.com", true, false, true),
        ("http://www.example.co.id/path?a=1", true, false, true),
        ("coba vk.com/x", true, false, true),
        ("halo 3.14 itu angka", true, false, false),
        ("user@host.tld", true, false, false),
        ("", false, false, false),
        ("halo dunia", false, false, false),
        ("tapi.", false, false, false),
        ("gimana kabar?", false, false, false),
        ("selamat pagi", false, false, false),
        ("3.", false, false, false),
        ("cek https://spam.com", true, false, true),
        ("wa.me/12345", true, true, true),
        ("wa.me/62811", true, true, true),
        ("cek https://contoh.com", true, false, true),
        ("chat.whatsapp.com/", true, false, true),
        ("api.whatsapp.com/send", true, true, true),
        ("api.whatsapp.com/sendXYZ", true, true, true),
        ("cek https://chat.whatsapp.com/AbCd1234", true, true, true),
        ("www.contoh.co.id/page", true, false, true),
        ("file.inventi", false, false, false),
        ("x.dev", true, false, true),
        ("situs.xyz", true, false, true),
        ("toko.shop", true, false, true),
        ("main.ink", true, false, true),
        ("a.b.co.id", true, false, true),
        ("3.5", true, false, false),
        ("a.b_unknown", false, false, false),
        ("<x.com", true, false, true),
        ("\"x.com", true, false, true),
        ("normal.com", true, false, true),
        ("hi chat.whatsapp.com/", true, false, true),
        ("wa.me/", true, false, true),
        ("whatsapp.com/channel/", true, false, true),
        ("CHAT.WHATSAPP.COM/ABCD", true, true, true),
        ("WA.ME/628", true, true, true),
        ("halo semua", false, false, false),
        ("cek https://facebook.com", true, false, true),
        ("chat.whatsapp.com", true, false, true),
        ("halo cek https://x.com sama wa.me/12345", true, true, true),
        ("1.5", true, false, false),
        ("a.b", true, false, false),
        ("halo.dunia", true, false, false),
        ("www.a.co", true, false, true),
        ("x.co.uk", true, false, true),
        ("   ", false, false, false),
        (".com", false, false, false),
        (".co", false, false, false),
        ("a.", false, false, false),
        ("a..b", true, false, false),
        ("cek ini: http://bit.ly/abc", true, false, true),
        ("t.co/abcdef", true, false, true),
        ("visit foo.bar/path#frag", true, false, false),
    ];

    #[test]
    fn link_detection_matches_js_regex() {
        for (text, want_link, want_wa, want_any) in JS_LINK_TABLE {
            assert_eq!(
                contains_link(text),
                *want_link,
                "contains_link({text:?})"
            );
            assert_eq!(
                contains_wa_specific_link(text),
                *want_wa,
                "contains_wa_specific_link({text:?})"
            );
            assert_eq!(contains_any_link(text), *want_any, "contains_any_link({text:?})");
        }
    }

    // ==================================================================
    // helper
    // ==================================================================

    #[test]
    fn is_same_participant_normalizes_device_suffix() {
        assert!(is_same_participant(
            "628999:12@s.whatsapp.net",
            "628999@s.whatsapp.net"
        ));
        assert!(is_same_participant("628999@s.whatsapp.net", "628999@s.whatsapp.net"));
    }

    #[test]
    fn is_same_participant_fuzzy_by_digits() {
        assert!(is_same_participant("120628999@s.whatsapp.net", "628999@s.whatsapp.net"));
    }

    #[test]
    fn is_same_participant_rejects_empty_and_unrelated() {
        assert!(!is_same_participant("", "628999@s.whatsapp.net"));
        assert!(!is_same_participant("628999@s.whatsapp.net", ""));
        assert!(!is_same_participant("group", ""));
        assert!(!is_same_participant("111@s.whatsapp.net", "222@s.whatsapp.net"));
    }

    #[test]
    fn num_hit_requires_8_digit_fuzzy_match() {
        // Tanpa guard panjang, "1" akan cocok dengan semua participant.
        let nums = vec!["628111111".to_string()];
        assert!(!num_hit(&nums, "1"));
        assert!(num_hit(&nums, "628111111"));
    }

    #[test]
    fn admin_check_ignores_non_admin_participants() {
        let parts = vec![
            Participant {
                id: "628111@s.whatsapp.net".into(),
                admin: false,
                ..Default::default()
            },
            admin_participant("628222"),
        ];
        assert!(!is_admin_check(&parts, None, "628111@s.whatsapp.net"));
        assert!(is_admin_check(&parts, None, "628222@s.whatsapp.net"));
    }

    #[test]
    fn admin_check_with_no_candidates_is_false() {
        let parts = vec![admin_participant("628222")];
        assert!(!is_admin_check(&parts, None, ""));
    }

    #[test]
    fn bot_admin_check_false_without_bot_jid() {
        let e = GateEnv {
            participants: vec![admin_participant("628999")],
            ..Default::default()
        };
        assert!(!is_bot_admin_check(&e));
    }

    #[test]
    fn sender_tag_strips_domain() {
        assert_eq!(sender_tag("628111@s.whatsapp.net"), "628111");
        assert_eq!(sender_tag("plain"), "plain");
        assert_eq!(sender_tag(""), "");
    }

    #[test]
    fn decision_reply_text_returns_text_only_for_replying_variants() {
        assert_eq!(Decision::Reply("a".into()).reply_text(), Some("a"));
        assert_eq!(Decision::DeleteAndReply("b".into()).reply_text(), Some("b"));
        assert_eq!(Decision::KickAndReply("c".into()).reply_text(), Some("c"));
        assert_eq!(Decision::Allow.reply_text(), None);
        assert_eq!(Decision::Stop.reply_text(), None);
        assert_eq!(Decision::DeleteAndStop.reply_text(), None);
    }
}
