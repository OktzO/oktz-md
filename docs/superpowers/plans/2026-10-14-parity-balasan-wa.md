# Parity Balasan WhatsApp — Kutipan, Mention, Teks Config, Varian Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Setiap pesan yang keluar dari bot Rust punya bentuk yang sama persis dengan bot JS `ourin-md`: kutipan pesan pemicu, mention pengirim, teks penolakan, dan teks konfigurasi.

**Architecture:** `config.js` dipindahkan ke `oktz-core::config` sebagai sumber tunggal nilai tampilan. `render_reply` di `oktz-wa` receiving `ReplyTarget` yang dibangun pipeline per-pesan, sehingga default JS (kutip + mention sender) berlaku tanpa mengubah 884 call site plugin. Teks yang dinormalkan Node disimpan sebagai fixture dan dipakai test sebagai acuan.

**Tech Stack:** Rust 1.94.0, `serde_json`, `tracing`, Node 22 (hanya untuk membuat fixture, tidak ikut produksi).

**Spec:** `docs/superpowers/specs/2026-10-14-parity-tampilan-design.md` §3, §4.1, §4.5, §5

## Sub-sistem lain yang belum jadi plan ini

Spec ini mencakup empat sub-sistem. Plan ini hanya yang pertama; tiga
lainnya butuh plan tersendiri karena tidak saling bergantung:

| Sub-sistem | Isi spec | Kenapa dipisah |
|---|---|---|
| **Plan ini** | §4.1 kutipan+mention, §4.5 teks config | Menempel pada `Reply`, `render_reply`, `permission.rs` — inti setiap pesan |
| Plan berikutnya | §4.2 `replyVariant` 1-11 | Butuh 11 encoder pesan berbeda, tidak ada hubungannya dengan pipeline plugin |
| Plan berikutnya | §4.3 kartu (3 jalur JS) | `cards.rs` + encoder `waproto`; berdiri sendiri |
| Plan berikutnya | §4.6 logger terminal | Modul `jslog` baru, tidak menyentuh jalur pesan |

## Global Constraints

- **Nilai taken verbatim dari JS.** Setiap string yang ditulis ke user disalin persis dari `config.js` / `serialize.js` / `middleware.js`, termasuk spasi ganda, tanda baca, dan emoji. Tidak ada penyederhanaan.
- **Rust kalah, JS menang.** Kalau test Rust lama mengunci perilaku berbeda dari JS, test itu ditulis ulang — bukan dibiarkan.
- **Jangan buat teks baru.** Kalau nilai tidak ada di JS, nilai itu tidak ada di Rust. Bukan placeholder, bukan `TODO`.
- **Comment dan commit message bahasa Indonesia.** Tidak ada huruf CJK atau non-Latin yang bocor ke kode maupun commit.
- **Selalu `--release`.** `cargo test` debug mengompilasi ulang wacore dari nol (~15 menit sia-sia).
- Toolchain: `source /tmp/opencode/build_env.sh`, lalu `NS "cargo test --release"`. **Jangan** `$NS "..."` — di harness itu menyatukan seluruh argumen jadi satu kata.
- **Crate plugin wajib dirujuk di `use_plugin_crates()`** di `oktz-bin/src/main.rs`; kalau tidak, `linkme` hilang dan `all_plugins()` melaporkan 0 tanpa error.

## Review Focus

Lima kelas input yang paling mungkin menggigit orang yang memakai bot ini, dan
tugas mana yang/testing mana yang memegangnya:

1. **Pengirim di grup yang balasannya menyebut orang lain** — `mentionedJid`
   harus berisi persis yang plugin minta, dan sender otomatis hanyaadded kalau
   plugin tidak menyebut apa pun (Task 4).
2. **Balasan ke pesan yang punya stiker/gambar** — `quotedMessage` harus
   membawa pesan asli apa adanya, bukan teks kosong (Task 4).
3. **Nomor owner dipanggil tanpa permission** — user harus melihat
   `*Akses Ditolak!* Fitur ini khusus untuk Owner bot.`, bukan
   `🚫 Owner only!` (Task 3).
4. **Setting `replyVariant` hilang dari store** — harus jatuh ke 1, bukan 0
   atau error (Task 5).
5. **Pesan tanpa pemicu** (balasan plugin ke reaccion) — tidak boleh
   crash saat tries membangun kutipan dari pesan yang tidak ada (Task 4).

---

### Task 1: Harness Fixture dari Node

Nilai acuan harus berasal dari JS yang berjalan, bukan dari diligent
transkripsi. Tanpa ini, "sama persis" cuma klaim.

**Files:**
- Create: `scripts/gen-parity-fixtures.mjs`
- Create: `rust/crates/oktz-core/tests/fixtures/parity/config_text.json`
- Test: `rust/crates/oktz-core/tests/parity_fixture.rs`

**Interfaces:**
- Consumes: `ourin-md/config.js` (default export, sudah ESM), `ourin-md/src/lib/serialize.js`.
- Produces: `rust/crates/oktz-core/tests/fixtures/parity/config_text.json` dengan bentuk:

```json
{ "messages": { "ownerOnly": "*Akses Ditolak!* Fitur ini khusus untuk Owner bot." } }
```

- [ ] **Step 1: Tulis test yang gagal**

`rust/crates/oktz-core/tests/parity_fixture.rs`:

```rust
use std::path::PathBuf;

fn fixture() -> serde_json::Value {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/parity/config_text.json");
    serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap()
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
    let keys = ["success", "error", "ownerOnly", "premiumOnly", "groupOnly",
                "privateOnly", "adminOnly", "botAdminOnly", "cooldown",
                "energiExceeded", "limitDeducted", "banned", "rejectCall"];
    for k in keys {
        assert!(f["messages"].get(k).and_then(|v| v.as_str()).is_some(),
                "config.messages.{k} hilang dari fixture");
    }
}
```

- [ ] **Step 2: Jalankan test, harus gagal**

Run: `NS "cargo test --release -p oktz-core --test parity_fixture"`

Expected: FAIL — `No such file or directory ... parity/config_text.json`.

- [ ] **Step 3: Tulis generator**

`scripts/gen-parity-fixtures.mjs`:

```javascript
// Fixture parity dari JS produksi. Jalankan ulang setiap config.js berubah:
//   node scripts/gen-parity-fixtures.mjs
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import config from "../config.js";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const OUT = path.join(ROOT, "rust/crates/oktz-core/tests/fixtures/parity");
fs.mkdirSync(OUT, { recursive: true });

const out = {
  messages: config.messages,
  info: {
    website: config.info.website,
    botName: config.bot.name,
    version: config.bot.version,
    developer: config.bot.developer,
  },
  saluran: { id: config.saluran.id, name: config.saluran.name },
};

const file = path.join(OUT, "config_text.json");
fs.writeFileSync(file, JSON.stringify(out, null, 2));
console.log(`${file} (${Object.keys(config.messages).length} pesan)`);
```

- [ ] **Step 4: Jalankan generator dan test**

Run: `node scripts/gen-parity-fixtures.mjs && NS "cargo test --release -p oktz-core --test parity_fixture"`

Expected: script mencetak path; test PASS 2/2.

- [ ] **Step 5: Commit**

```bash
git add scripts/gen-parity-fixtures.mjs rust/crates/oktz-core/tests/
git commit -m "Fixture parity: nilai config.js diambil dari JS yang berjalan"
```

---

### Task 2: `oktz-core::config` Menyimpan Teks Tampilan

Nilai dari fixture masuk ke satu tempat. Plugin berhenti hardcode
`BOT_NAME` di Task 3.

**Files:**
- Modify: `rust/crates/oktz-core/src/config.rs` (tambah struct `Messages`, `BotInfo`, `Saluran`, field baru di `Config`)
- Test: `rust/crates/oktz-core/src/config.rs` (mod tests yang sudah ada)

**Interfaces:**
- Consumes: `Config::from_env()` yang sudah ada; fixture Task 1.
- Produces:
  - `oktz_core::config::Messages` dengan `pub` field: `success`, `error`, `owner_only`, `premium_only`, `group_only`, `private_only`, `admin_only`, `bot_admin_only`, `cooldown`, `energi_exceeded`, `limit_deducted`, `banned`, `reject_call` — semua `String`, semua `Default` berisi nilai JS.
  - `oktz_core::config::BotInfo { pub name: String, pub version: String, pub developer: String, pub website: String }`.
  - `oktz_core::config::Saluran { pub id: String, pub name: String }`.
  - `Config` dapat field baru `pub messages: Messages`, `pub bot: BotInfo`, `pub saluran: Saluran`.
  - `Messages::cooldown_text(&self, secs: u64) -> String` mengganti `%time%` dengan angka detik.

- [ ] **Step 1: Tulis test yang gagal**

Tambahkan ke `mod tests` di `config.rs`:

```rust
#[test]
fn denial_texts_match_config_js_not_the_middleware_fallback() {
    let m = Messages::default();
    // `middleware.js:57` memakai fallback hanya kalau `config.messages`
    // kosong; di produksi nilainya selalu ada, jadi yang tampil ke user
    // adalah string dari config.js.
    assert_eq!(m.owner_only, "*Akses Ditolak!* Fitur ini khusus untuk Owner bot.");
    assert_ne!(m.owner_only, "🚫 Owner only!");
    assert_eq!(
        m.energi_exceeded,
        "⚡ *Energi Habis!* Energi kamu sudah habis. Tunggu reset besok atau beli Premium."
    );
}

#[test]
fn cooldown_replaces_the_time_placeholder() {
    let m = Messages::default();
    assert!(m.cooldown_text(30).contains("30"));
    assert!(!m.cooldown_text(30).contains("%time%"));
}

#[test]
fn bot_info_website_matches_config_js() {
    assert_eq!(BotInfo::default().website, "https://oktzz.xyz");
}
```

- [ ] **Step 2: Jalankan test, harus gagal**

Run: `NS "cargo test --release -p oktz-core config"`

Expected: FAIL — `cannot find value Messages in this scope`.

- [ ] **Step 3: Implementasikan**

Tambahkan ke `config.rs`, sebelum `pub struct Config`:

```rust
/// `config.js` `messages`. Field `snake_case`, nilai `verbatim` dari JS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Messages {
    pub success: String,
    pub error: String,
    pub owner_only: String,
    pub premium_only: String,
    pub group_only: String,
    pub private_only: String,
    pub admin_only: String,
    pub bot_admin_only: String,
    pub cooldown: String,
    pub energi_exceeded: String,
    pub limit_deducted: String,
    pub banned: String,
    pub reject_call: String,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            success: d("✅ *Berhasil!* Permintaan kamu sudah selesai."),
            error: d("❌ *Error!* Ada masalah pada sistem, coba lagi nanti."),
            owner_only: d("*Akses Ditolak!* Fitur ini khusus untuk Owner bot."),
            premium_only: d("💎 *Premium Only!* Fitur ini khusus member Premium. Ketik *.benefitpremium* untuk info upgrade."),
            group_only: d("👥 *Group Only!* Fitur ini hanya bisa digunakan di dalam grup."),
            // `config.js:204` begins with U+FFFD, bukan emoji: disalin apa
            // adanya supaya byte yang tampil sama.
            private_only: d("\u{fffd} *Private Only!* Fitur ini hanya bisa digunakan di chat pribadi bot."),
            admin_only: d("\u{fffd}️ *Admin Only!* Kamu harus jadi Admin grup untuk pakai fitur ini."),
            bot_admin_only: d("🤖 *Bot Bukan Admin!* Jadikan bot sebagai Admin grup dulu biar bisa kerja."),
            cooldown: d("🕕 *Tunggu Dulu!* Kamu masih dalam cooldown. Tunggu %time% detik lagi ya."),
            energi_exceeded: d("⚡ *Energi Habis!* Energi kamu sudah habis. Tunggu reset besok atau beli Premium."),
            limit_deducted: d("🔋 Limit kau berkurang sebanyak {amount}. Sisa limit: {sisa}"),
            banned: d("🚫 *Kamu Dibanned!* Kamu tidak bisa menggunakan bot ini karena telah melanggar aturan."),
            reject_call: d("🚫 JANGAN TELPON NOMOR INI WEH"),
        }
    }
}

impl Messages {
    pub fn cooldown_text(&self, secs: u64) -> String {
        self.cooldown.replace("%time%", &secs.to_string())
    }
}
```

Lalu `BotInfo`, `Saluran`, dan field `Default` di `Config`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotInfo {
    pub name: String,
    pub version: String,
    pub developer: String,
    pub website: String,
}

impl Default for BotInfo {
    fn default() -> Self {
        Self {
            name: d("Oktz Bot"),
            version: d("3.3.1"),
            developer: d("Zann"),
            website: d("https://oktzz.xyz"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Saluran {
    pub id: String,
    pub name: String,
}
```

`Config` dapat `#[derive(Default)]` sudah ada; tambahkan field `messages`,
`bot`, `saluran` ke struct dan ke `Config::from_env()` sebagai
`Messages::default()`, `BotInfo::default()`, `Saluran::default()`.

- [ ] **Step 4: Jalankan test, harus hijau**

Run: `NS "cargo test --release -p oktz-core config"`

Expected: PASS, tidak ada kegagalan.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/oktz-core/src/config.rs
git commit -m "oktz-core: config.js messages/bot/saluran sebagai sumber teks"
```

---

### Task 3: Router dan Pipeline Pakai Teks Config

Sekarang user melihat string fallback; setelah ini user melihat string
`config.js`.

**Files:**
- Modify: `rust/crates/oktz-router/src/permission.rs:81-95` (`DenyReason::message`)
- Modify: `rust/crates/oktz-wa/src/pipeline.rs:1447-1458` (`decision_to_reply`)
- Test: `rust/crates/oktz-router/src/permission.rs`, `rust/crates/oktz-wa/src/pipeline.rs`

**Interfaces:**
- Consumes: `oktz_core::config::Messages` (Task 2).
- Produces:
  - `oktz_router::permission::DenyReason::message_with(&self, m: &Messages) -> String`
    — `message()` yang ada tetap ada sebagai pembungkus yang memakai
    `Messages::default()`, supaya test lama tidak rusak seketika.
  - `DenyMode::Afk` dan `DenyMode::OnlyThisGroup` tetap tanpa teks di enum;
    teksnya datang dari pipeline (lihat di bawah) karena JS menyisipkan
    `botAfk.reason` / `onlyThisGroup.link`.

- [ ] **Step 1: Tulis test yang gagal**

Di `permission.rs` `mod tests`:

```rust
#[test]
fn deny_reason_uses_config_text_when_given_it() {
    let m = Messages::default();
    assert_eq!(DenyReason::OwnerOnly.message_with(&m),
               "*Akses Ditolak!* Fitur ini khusus untuk Owner bot.");
    assert_eq!(DenyReason::GameDisabled.message_with(&m), m.game_disabled);
}
```

`GameDisabled` dan `RpgDisabled` tidak ada di `config.js` — nilainya tetap
konstanta seperti sekarang. Tambahkan dua field itu ke `Messages` dengan
nilai yang sama dengan `permission.rs` sekarang, supaya struct tetap punya
satu sumber.

Di `pipeline.rs` `mod tests`:

```rust
#[test]
fn afk_denial_text_comes_from_config_not_a_hardcoded_string() {
    let r = decision_to_reply(&Decision::DeniedMode(DenyMode::Afk), &Messages::default());
    assert!(r.text_body().unwrap().contains("sᴇᴅᴀɴɢ ᴀꜰᴋ"));
}
```

- [ ] **Step 2: Jalankan test, harus gagal**

Run: `NS "cargo test --release -p oktz-router permission && NS "cargo test --release -p oktz-wa afk_denial"`

Expected: FAIL — `no method named message_with`.

- [ ] **Step 3: Implementasikan**

Di `permission.rs`, ganti `message()` dengan:

```rust
impl DenyReason {
    /// Balasan dari `config.js`. `GameDisabled`/`RpgDisabled` tidak ada di
    /// sana, jadi nilainya tetap seperti sekarang — ini bukan penolakan
    /// yang dibaca user dari config.
    pub fn message_with(&self, m: &Messages) -> String {
        match self {
            Self::OwnerOnly => m.owner_only.clone(),
            Self::PartnerOnly => d(PARTNER_ONLY).into(),
            Self::PremiumOnly => m.premium_only.clone(),
            Self::GroupOnly => m.group_only.clone(),
            Self::PrivateOnly => m.private_only.clone(),
            Self::AdminOnly => m.admin_only.clone(),
            Self::BotAdminOnly => m.bot_admin_only.clone(),
            Self::GameDisabled => m.game_disabled.clone(),
            Self::RpgDisabled => m.rpg_disabled.clone(),
        }
    }

    pub fn message(&self) -> String {
        self.message_with(&Messages::default())
    }
}
```

Di `pipeline.rs`, `decision_to_reply` jadi menerima `&Messages`:

```rust
pub fn decision_to_reply(d: &Decision, m: &Messages) -> Reply {
    match d {
        Decision::Handled(r) => r.clone(),
        Decision::DeniedPermission(msg) | Decision::DeniedGroupMode(msg) => Reply::text(msg),
        Decision::DeniedMode(DenyMode::Afk) => Reply::text(afk_text(m)),
        Decision::DeniedMode(DenyMode::OnlyThisGroup) => Reply::text(only_group_text(m)),
        Decision::DeniedMode(DenyMode::Silent | DenyMode::SelfGroup) => Reply::None,
        Decision::NotACommand | Decision::NoPlugin => Reply::None,
    }
}
```

`afk_text` dan `only_group_text` membaca `botAfk.reason` dan
`onlyThisGroup.link` dari store; kalau kosong, teksnya sama persis dengan
yang sekarang supaya tidak ada yang hilang:

```rust
fn afk_text(m: &Messages) -> String {
    // `handler.js` menyisipkan `botAfk.reason` ke dalam kalimat; tanpa itu
    // teksnya persis yang sekarang.
    format!(
        "💤 *ʙᴏᴛ sᴇᴅᴀɴɢ ᴀꜰᴋ*\n\n> Bot tidak bisa menerima perintah saat ini\n> Mohon tunggu sampai owner mengaktifkan kembali"
    )
}
```

Hapus test lama yang mengunci string fallback; ganti dengan assertion ke
nilai `config.js`.

- [ ] **Step 4: Jalankan test**

Run: `NS "cargo test --release -p oktz-router -p oktz-wa"`

Expected: semua PASS. Kalau ada test yang gagal karena string fallback,
ubah assertion-nya ke nilai `config.js` — bukan nilai Rust.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/oktz-router/src/permission.rs rust/crates/oktz-wa/src/pipeline.rs
git commit -m "Router dan pipeline pakai teks config.js, bukan string fallback"
```

---

### Task 4: Default Balasan Mengutip dan Menyebut Pengirim

Ini perubahan yang paling terlihat: hari ini hampir setiap balasan terkirim
sebagai pesan polos tanpa kutipan, padahal JS mengutip pesan pemicu setiap kali.

**Files:**
- Modify: `rust/crates/oktz-plugin-api/src/registry.rs` (enum `Reply`, `text`, `text_mention`)
- Modify: `rust/crates/oktz-wa/src/pipeline.rs` (`render_reply`, `text_message`, `Pipeline::process`)
- Test: kedua file

**Interfaces:**
- Consumes: `QuoteSource::from_inbound` yang sudah ada.
- Produces:
  - `pub struct ReplyTarget { pub quote: Option<QuoteSource>, pub sender: String }`
    — satu-satunya sumber "ke siapa dan apa yang harus dikutip" untuk
    `render_reply`.
  - `pub fn render_reply(reply: &Reply, target: &ReplyTarget) -> Result<Option<Rendered>, PipelineError>`
    — tanda tangan berubah; semua call site dan test diperbarui.
  - `Reply::text` dan `Reply::text_mention` **tidak berubah tandatiganya**.
  - `Reply::text_quoted(text)` sebagai opt-out eksplisit (`quoted: false`),
    dan `Reply::text_no_mention(text)` untuk yang ingin mengutip tapi tidak
    menyebut pengirim (`mentions: []`).

- [ ] **Step 1: Tulis test yang gagal**

Di `pipeline.rs` `mod tests`:

```rust
fn target(sender: &str) -> ReplyTarget {
    ReplyTarget { quote: None, sender: sender.into() }
}

#[test]
fn a_plain_reply_quotes_the_triggering_message() {
    // `serialize.js:1000`: `quotedMsg = options.quoted !== false ? msg : undefined`
    let r = render_reply(&Reply::text("pong"), &target("628@s.whatsapp.net")).unwrap().unwrap();
    let ci = context_info_of(&r.message).unwrap();
    assert_eq!(ci.participant.as_deref(), Some("628@s.whatsapp.net"));
}

#[test]
fn a_plain_reply_mentions_the_sender_by_default() {
    // `serialize.js:994`: `mentionedJid: options?.mentions || [m?.sender] || []`
    let r = render_reply(&Reply::text("pong"), &target("628@s.whatsapp.net")).unwrap().unwrap();
    let ci = context_info_of(&r.message).unwrap();
    assert_eq!(ci.mentioned_jid.as_ref().unwrap()[0].user.as_str(), "628");
}

#[test]
fn text_mention_replaces_the_default_mention() {
    let r = render_reply(
        &Reply::text_mention("hai", "628999@s.whatsapp.net"),
        &target("628@s.whatsapp.net"),
    ).unwrap().unwrap();
    let ci = context_info_of(&r.message).unwrap();
    assert_eq!(ci.mentioned_jid.as_ref().unwrap()[0].user.as_str(), "628999");
}

#[test]
fn text_no_mention_quotes_but_does_not_mention() {
    let r = render_reply(&Reply::text_no_mention("diam"), &target("628@s.whatsapp.net"))
        .unwrap().unwrap();
    let ci = context_info_of(&r.message).unwrap();
    assert!(ci.mentioned_jid.as_ref().unwrap().is_empty());
    assert!(ci.participant.is_some());
}

#[test]
fn text_quoted_opts_out_of_quoting_entirely() {
    let r = render_reply(&Reply::text_quoted(" polos"), &target("628@s.whatsapp.net"))
        .unwrap().unwrap();
    // Tanpa kutipan dan tanpa mention = `conversation` polos, bukan
    // `extendedTextMessage`.
    assert!(context_info_of(&r.message).is_none());
}

#[test]
fn missing_trigger_message_does_not_panic() {
    // Review Focus #5: balasan ke pesan yang tidak punya pemicu harus tetap
    // jalan, hanya tidak punya kutipan.
    let r = render_reply(&Reply::text("halo"), &ReplyTarget { quote: None, sender: "628@s.whatsapp.net".into() })
        .unwrap().unwrap();
    assert!(context_info_of(&r.message).is_none());
}
```

- [ ] **Step 2: Jalankan test, harus gagal**

Run: `NS "cargo test --release -p oktz-wa render_reply"`

Expected: FAIL — `cannot find value ReplyTarget`.

- [ ] **Step 3: Implementasikan**

`ReplyTarget` di `pipeline.rs`:

```rust
/// Ke mana balasan pergi dan apa yang harus dikutip. JS punya ini di
/// `serialize.js` sebagai `m` — setiap balasan tahu pemicunya.
#[derive(Debug, Clone, Default)]
pub struct ReplyTarget {
    pub quote: Option<QuoteSource>,
    pub sender: String,
}
```

`render_reply`:

```rust
pub fn render_reply(
    reply: &Reply,
    target: &ReplyTarget,
) -> Result<Option<Rendered>, PipelineError> {
    let quote = target.quote.as_ref();
    let sender_mention = || vec![target.sender.clone()];
    ...
    Reply::Text { text, mentions } => {
        let mentions = if mentions.is_empty() { sender_mention() } else { mentions.clone() };
        Ok(Some(Rendered::wire(text_message(text, &mentions, quote), Vec::new())))
    }
    Reply::TextQuoted { text, mentions } => {
        let mentions = if mentions.is_empty() { sender_mention() } else { mentions.clone() };
        Ok(Some(Rendered::wire(text_message(text, &mentions, quote), Vec::new())))
    }
```

Constructor baru di `registry.rs`:

```rust
impl Reply {
    /// `quoted: false` — tidak mengutip pesan pemicu.
    pub fn text_quoted(text: impl Into<String>) -> Self {
        Reply::NoQuote { text: text.into(), mentions: vec![] }
    }

    /// `mentions: []` — mengutip tapi tidak menyebut pengirim.
    pub fn text_no_mention(text: impl Into<String>) -> Self {
        Reply::TextNoMention { text: text.into() }
    }
}
```

Dua variant baru mengikuti enum `Reply` yang sudah ada:
`NoQuote { text: String, mentions: Vec<String> }` dan
`TextNoMention { text: String }`.

Di `Pipeline::process`, ganti tiga call site `render_reply(&reply, quote.as_ref())`
menjadi `render_reply(&reply, &ReplyTarget { quote, sender: caller.sender.clone() })`.
`QuoteSource::from_inbound(m)` dipanggil sekali, bukan tiga kali.

- [ ] **Step 4: Jalankan test**

Run: `NS "cargo test --release -p oktz-wa -p oktz-plugins -p oktz-bin"`

Expected: semua PASS. 884 call site `Reply::text` tidak berubah, jadi tidak
perlu disentuh — itu tujuan desainnya.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/oktz-plugin-api rust/crates/oktz-wa
git commit -m "Balasan default mengutip pesan pemicu dan menyebut pengirim seperti serialize.js"
```

---

### Task 5: `replyVariant` Diterapkan ke Balasan

Task ini baru memindahkan kerangka: `replyVariant` aktif di deployment adalah
**9**, jadi varian 9 harus benar lebih dulu. Varian 1-8 dan 10-11 menyusul di
plan sendiri.

**Files:**
- Create: `rust/crates/oktz-wa/src/reply_variant.rs`
- Modify: `rust/crates/oktz-wa/src/lib.rs` (`pub mod reply_variant;`, re-export)
- Modify: `rust/crates/oktz-wa/src/pipeline.rs` (`Pipeline::deliver` memanggil varian)
- Test: `rust/crates/oktz-wa/src/reply_variant.rs`

**Interfaces:**
- Consumes: `oktz_core::config::BotInfo` (Task 2), `ReplyTarget` (Task 4).
- Produces:
  - `pub fn reply_variant(raw: Option<&Value>) -> u32` — `Some(9)` → 9,
    `None` → 1, angka di luar 1..=11 → 1. (`serialize.js:983-991`: default 1.)
  - `pub fn apply_variant9(text: &str, sender: &str, chat: &str) -> wa::Message`
    — `relayMessage` dengan `orderMessage`; nilai field disalin dari
    `serialize.js:1231-1258` apa adanya, termasuk `orderId: "WXX"`,
    `orderTitle: "CONTOL"`, `token: "SDsdafma"`, `totalCurrencyCode: "IDR"`.
  - `pub fn apply_variant(text: &str, target: &ReplyTarget, v: u32) -> Option<wa::Message>`
    — `Some(..)` kalau varian punya bentuk pesan sendiri; `None` berarti
    jalankan jalur `extendedTextMessage` yang sekarang.

- [ ] **Step 1: Tulis test yang gagal**

```rust
#[test]
fn missing_variant_falls_back_to_one() {
    assert_eq!(reply_variant(None), 1);
    assert_eq!(reply_variant(Some(&serde_json::json!("bukan angka"))), 1);
    assert_eq!(reply_variant(Some(&serde_json::json!(99))), 1);
}

#[test]
fn the_active_variant_is_read_as_is() {
    assert_eq!(reply_variant(Some(&serde_json::json!(9))), 9);
}

#[test]
fn variant_nine_keeps_the_constants_from_serialize_js() {
    let m = apply_variant9("hai", "628@s.whatsapp.net", "628@s.whatsapp.net");
    let json = serde_json::to_value(&m).unwrap();
    assert_eq!(json["orderMessage"]["orderId"], "WXX");
    assert_eq!(json["orderMessage"]["orderTitle"], "CONTOL");
    assert_eq!(json["orderMessage"]["token"], "SDsdafma");
    assert_eq!(json["orderMessage"]["status"], "INQUIRY");
    assert_eq!(json["orderMessage"]["surface"], "CATALOG");
    assert_eq!(json["orderMessage"]["totalCurrencyCode"], "IDR");
    assert_eq!(json["orderMessage"]["itemCount"], 1);
    assert_eq!(json["orderMessage"]["messageVersion"], 1);
}

#[test]
fn variant_nine_quotes_and_mentions_like_the_other_variants() {
    let m = apply_variant9("hai", "628@s.whatsapp.net", "628@s.whatsapp.net");
    let json = serde_json::to_value(&m).unwrap();
    assert_eq!(json["orderMessage"]["contextInfo"]["participant"], "628@s.whatsapp.net");
}
```

- [ ] **Step 2: Jalankan test, harus gagal**

Run: `NS "cargo test --release -p oktz-wa reply_variant"`

Expected: FAIL — `cannot find function reply_variant`.

- [ ] **Step 3: Implementasikan**

`reply_variant.rs`:

```rust
//! `serialize.js:983-1290`. Hanya varian 9 yang diimplementasikan di sini;
//! varian lain memakai jalur `extendedTextMessage` sampai_task-nya sendiri.

/// `db.setting("replyVariant") || 1`, diamankan dari nilai di luar rentang.
pub fn reply_variant(raw: Option<&serde_json::Value>) -> u32 {
    raw.and_then(|v| v.as_u64())
        .filter(|n| (1..=11).contains(n))
        .unwrap_or(1) as u32
}
```

`apply_variant9` memakai `whatsapp_rust::proto_helpers` atau membangun
`wa::Message` lewat `serde_json` → `from_value`, karena `waproto` sudah
punya `OrderMessage` tapi `orderTitle`/`token` mungkin tidak punya accessor
publik. Kalau ternyata tidak bisa lewat protobuf builder, bangun lewat
`serde_json::from_value` dengan kunci yang sama persis seperti test.

Di `Pipeline::deliver`, baca varian dari store:

```rust
let variant = reply_variant(self.ctx.settings_entry().get("replyVariant").as_ref());
```

dan kalau `Some(m) = apply_variant(&rendered.text_body_or_empty(), &target, variant)`
untuk varian 9, kirim `m` alih-alih hasil render biasa.

- [ ] **Step 4: Jalankan test**

Run: `NS "cargo test --release -p oktz-wa"`

Expected: PASS, tidak ada kegagalan.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/oktz-wa
git commit -m "replyVariant 9 (aktif di deployment) mulai-forming balasan orderMessage"
```

---

### Task 6: Bukti Langsung di Device Asli

Kode hijau belum berarti user melihat yang benar. Device WA asli sudah terbukti
bisa dipakai: creds + 527 sesi ratchet dari Turso, pesan masuk ter-decrypt.

**Files:**
- Create: `scripts/live-parity-check.sh`
- Modify: `docs/rust-rewrite/PROGRESS.md`

**Interfaces:**
- Consumes: `oktz run` (sudah menerima fixture sesi), binary
  `/tmp/opencode/rust/target/release/oktz`, creds dari Turso.
- Produces: skrip yang menyalakan bot 90 detik, mengirim satu pesan peruje,
  dan mencetak apa yang keluar.

- [ ] **Step 1: Tulis skrip**

```bash
#!/usr/bin/env bash
# Bukti parity langsung di device WA asli.
#   bash scripts/live-parity-check.sh [hold_secs]
# PERINGATAN: device ini dipakai bot produksi. Kalau produksi sedang
# connect, akses ini kalah dan skrip berakhir dengan conflict=replaced —
# itu bukan kegagalan parity, itu_tabrakan koneksi.
set -uo pipefail
ROOT=/home/user/noddjs/ourin-md
LIVE=/tmp/opencode/live
HOLD="${1:-90}"

cd "$ROOT"
export OKTZ_DB_DIR="$LIVE"
export OKTZ_CREDS="$LIVE/creds.json"
export RUST_LOG=info

/tmp/opencode/rust/target/release/oktz run \
  "$LIVE/creds.json" database/main/settings.json "$HOLD" \
  "$LIVE/turso_session_keys.json" 2>&1 \
  | grep -E "pesan inbound diproses|conflict=replaced|CONNECTED"
```

Buat dua file yang dibutuhkan lebih dulu bila belum ada:
`/tmp/opencode/live/creds.json` dan `/tmp/opencode/live/turso_session_keys.json`,
diambil dari Turso dengan kredensial `.env` (jangan di-commit).

- [ ] **Step 2: Jalankan dan kirim pesan uji**

Run: `bash scripts/live-parity-check.sh 120`

Lalu dari WhatsApp nomor lain, kirim ke `6285143885645`: pesan biasa, satu
grup, dan satu command owner.

Expected di log: tiga baris `pesan inbound diproses`, masing-masing
`body=` berisi plaintext yang benar dan `outcome=` yang sesuai.

- [ ] **Step 3: Catat hasil**

Perbarui `docs/rust-rewrite/PROGRESS.md`: tanggal, command yang diuji, dan
teks balasan yang benar-benar diterima. Kalau ada yang beda dari `config.js`,
catat sebagai temuan — jangan tulis "selesai".

- [ ] **Step 4: Commit**

```bash
git add scripts/live-parity-check.sh docs/rust-rewrite/PROGRESS.md
git commit -m "Bukti parity di device WA asli + catat hasil pengujian"
```