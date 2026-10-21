//! Model kartu interaktif WhatsApp (native flow) untuk plugin.
//!
//! Padanan JS-nya:
//! - `src/lib/builder.js:544` — kelas `Button` (tombol + header + footer).
//! - `src/lib/socket.js:358` — `sendInteractiveCard` (kirim kartu).
//! - `src/lib/rich-messages.js:454` — `wrapInteractive` (pembungkus kartu ke
//!   `viewOnceMessage` + `deviceListMetadata`).
//!
//! Bentuk yang WA benar-benar render: `interactiveMessage` dibungkus
//! `viewOnceMessage` (tipe `FutureProofMessage`) + `messageContextInfo`
//! berisi `deviceListMetadata` versi 2 + extra stanza node `biz`. Bentuk
//! legacy (`ListMessage`, `interactiveButtons` telanjang) relay-nya sukses
//! tapi WA membuangnya diam-diam — jangan dipakai.
//!
//! `ponytail:` yang belum ada di sini: `AIRich` (kartu unifiedResponse),
//! `carouselMessage`, dan `templateMessage`. Semuanya butuh proto yang lebih
//! dari native-flow dan belum dipakai plugin produksi.

use serde::Serialize;
use serde_json::Value;

use crate::registry::MediaSource;

/// Parameter tombol native-flow — JSON mentah yang jadi `buttonParamsJson`.
///
/// `serde_json::Value` biasa tidak bisa `Eq` (angka bisa NaN), padahal
/// `Reply` harus `Eq` supaya router bisa memakainya sebagai nilai enum
/// biasa. Wrapper ini tetap bisa diindeks seperti JSON biasa
/// (`params["display_text"]`) dan dibandingkan lewat isi JSON-nya.
#[derive(Debug, Clone, Default)]
pub struct Params(Value);

impl Params {
    pub fn baru(nilai: Value) -> Self {
        Self(nilai)
    }

    /// Isi JSON-nya.
    pub fn nilai(&self) -> &Value {
        &self.0
    }

    /// Bentuk teks yang dikirim ke WA (`buttonParamsJson`).
    pub fn json(&self) -> String {
        self.0.to_string()
    }
}

impl PartialEq for Params {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Params {}

impl From<Value> for Params {
    fn from(nilai: Value) -> Self {
        Self(nilai)
    }
}

impl std::ops::Index<&str> for Params {
    type Output = Value;

    fn index(&self, key: &str) -> &Value {
        &self.0[key]
    }
}

/// Nama aksi tombol native-flow. WA membaca `name` sebagai string literal;
/// nama yang salah membuat tombol hilang tanpa error sama sekali, jadi
/// daftar ini enum dan bukan string bebas.
///
/// Daftarnya diambil dari `src/lib/builder.js` dan pemakaian nyata di
/// `plugins/` (quick_reply ~105, single_select ~32, cta_url ~15, cta_copy ~8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AksiTombol {
    /// `quick_reply` — balasan cepat: isi `id` = command yang dijalankan.
    BalasanCepat,
    /// `single_select` — daftar pilihan; isinya `buttonParamsJson` berisi
    /// `title`, `sections[]`, dan `icon`.
    PilihSatuan,
    /// `cta_url` — tombol buka tautan.
    BukaUrl,
    /// `cta_copy` — tombol salin teks.
    Salin,
    /// `cta_call` — tombol telepon.
    Telepon,
    /// `cta_reminder` — tombol buat pengingat.
    Pengingat,
    /// `cta_cancel_reminder` — tombol batalkan pengingat.
    BatalPengingat,
    /// `address_message` — kirim alamat.
    Alamat,
    /// `send_location` — kirim lokasi.
    Lokasi,
}

impl AksiTombol {
    /// String yang dibaca WA (`name` di `NativeFlowButton`).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BalasanCepat => "quick_reply",
            Self::PilihSatuan => "single_select",
            Self::BukaUrl => "cta_url",
            Self::Salin => "cta_copy",
            Self::Telepon => "cta_call",
            Self::Pengingat => "cta_reminder",
            Self::BatalPengingat => "cta_cancel_reminder",
            Self::Alamat => "address_message",
            Self::Lokasi => "send_location",
        }
    }

    /// `name` string dari `interactiveButtons[]` JS. `None` = nama di luar
    /// daftar yang dikenal WA.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "quick_reply" => Self::BalasanCepat,
            "single_select" => Self::PilihSatuan,
            "cta_url" => Self::BukaUrl,
            "cta_copy" => Self::Salin,
            "cta_call" => Self::Telepon,
            "cta_reminder" => Self::Pengingat,
            "cta_cancel_reminder" => Self::BatalPengingat,
            "address_message" => Self::Alamat,
            "send_location" => Self::Lokasi,
            _ => return None,
        })
    }
}

/// Satu tombol native-flow. `params` objek yang jadi `buttonParamsJson` —
/// bentuknya sudah ditentukan WA, jadi plugin yang menyusunnya, pipeline
/// hanya memetakan `aksi`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tombol {
    pub aksi: AksiTombol,
    pub params: Params,
}

impl Tombol {
    pub fn baru(aksi: AksiTombol, params: Value) -> Self {
        Self {
            aksi,
            params: Params::baru(params),
        }
    }

    /// Tombol dari `name` string (JS `interactiveButtons[].name`).
    /// `None` = nama di luar daftar yang dikenal WA.
    pub fn dari_nama(name: &str, params: Value) -> Option<Self> {
        Some(Self::baru(AksiTombol::from_name(name)?, params))
    }

    /// `quick_reply` — `{"display_text", "id"}`. `id` = command yang
    /// dijalankan saat tombol ditekan (JS `menu.js:520`).
    pub fn balas_cepat(display_text: impl Into<String>, id: impl Into<String>) -> Self {
        Self::baru(
            AksiTombol::BalasanCepat,
            serde_json::json!({
                "display_text": display_text.into(),
                "id": id.into(),
            }),
        )
    }

    /// `cta_copy` — `{"display_text", "id", "copy_code"}` (JS
    /// `styleteks.js`, `tools/`).
    pub fn salin(
        display_text: impl Into<String>,
        id: impl Into<String>,
        copy_code: impl Into<String>,
    ) -> Self {
        Self::baru(
            AksiTombol::Salin,
            serde_json::json!({
                "display_text": display_text.into(),
                "id": id.into(),
                "copy_code": copy_code.into(),
            }),
        )
    }

    /// `cta_url` — `{"display_text", "url", "merchant_url"}` (JS
    /// `plugins/main/sc.js:44`).
    pub fn buka_url(display_text: impl Into<String>, url: impl Into<String>) -> Self {
        let url = url.into();
        Self::baru(
            AksiTombol::BukaUrl,
            serde_json::json!({
                "display_text": display_text.into(),
                "url": url,
                "merchant_url": url,
            }),
        )
    }

    /// `cta_call` — `{"display_text", "phone_number"}`.
    pub fn telepon(display_text: impl Into<String>, nomor: impl Into<String>) -> Self {
        Self::baru(
            AksiTombol::Telepon,
            serde_json::json!({
                "display_text": display_text.into(),
                "phone_number": nomor.into(),
            }),
        )
    }

    /// `single_select` — `{"title", "sections", "icon"}` (JS `menu.js:598`).
    pub fn pilih(daftar: &Daftar) -> Self {
        Self::baru(AksiTombol::PilihSatuan, serde_json::to_value(daftar).unwrap_or(Value::Null))
    }
}

/// Satu baris di dalam `single_select`. Field yang tidak diisi dilewati dari
/// `buttonParamsJson` — bentuk akhir sudah ditentukan WA, bukan kita.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Baris {
    pub title: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Baris {
    pub fn baru(title: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            id: id.into(),
            header: None,
            description: None,
        }
    }

    pub fn header(mut self, h: impl Into<String>) -> Self {
        self.header = Some(h.into());
        self
    }

    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
}

/// Satu section `single_select` (JS `builder.js:617` `makeSection`).
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct Bagian {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight_label: Option<String>,
    pub rows: Vec<Baris>,
}

impl Bagian {
    pub fn baru(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            highlight_label: None,
            rows: Vec::new(),
        }
    }

    pub fn highlight_label(mut self, h: impl Into<String>) -> Self {
        self.highlight_label = Some(h.into());
        self
    }

    pub fn baris(mut self, row: Baris) -> Self {
        self.rows.push(row);
        self
    }
}

/// Isi `buttonParamsJson` untuk `single_select` (JS `menu.js:598`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Daftar {
    pub title: String,
    pub sections: Vec<Bagian>,
    /// `icon` — WA cuma kenal `"DEFAULT"` dan `"REVIEW"` (JS `menu.js:700`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl Daftar {
    pub fn baru(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            sections: Vec::new(),
            icon: None,
        }
    }

    pub fn bagian(mut self, bagian: Bagian) -> Self {
        self.sections.push(bagian);
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

/// Header kartu: judul, subjudul, dan media (gambar) yang jadi sampul.
///
/// JS: `interactiveMessage.header` (`menu.js:598` — `title: ""`,
/// `subtitle: config.bot.name`, `hasMediaAttachment: true`, `imageMessage`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KartuHeader {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    /// Media header. `None` = header tanpa gambar (WA tetap render judulnya).
    pub media: Option<MediaSource>,
}

impl KartuHeader {
    pub fn baru() -> Self {
        Self::default()
    }

    pub fn judul(mut self, judul: impl Into<String>) -> Self {
        self.title = Some(judul.into());
        self
    }

    pub fn subjudul(mut self, sub: impl Into<String>) -> Self {
        self.subtitle = Some(sub.into());
        self
    }

    pub fn media(mut self, media: MediaSource) -> Self {
        self.media = Some(media);
        self
    }
}

/// Kartu native-flow: body, footer, header, tombol, `messageParamsJson`, dan
/// mention (JS `interactiveMessage`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Kartu {
    /// `body.text` — teks utama kartu.
    pub body: String,
    /// `footer.text`. `None` = footer tidak dikirim.
    pub footer: Option<String>,
    /// `header`. `None` = tanpa header.
    pub header: Option<KartuHeader>,
    /// `nativeFlowMessage.buttons`.
    pub buttons: Vec<Tombol>,
    /// `nativeFlowMessage.messageParamsJson` — `bottom_sheet`,
    /// `limited_time_offer`, `tap_target_configuration` (JS `menu.js:755`).
    pub params: Option<Params>,
    /// `contextInfo.mentionedJid` — mention di dalam kartu.
    pub mentions: Vec<String>,
    /// `true` = kartu mengutip pesan pemicu (JS `quoted: m`).
    ///
    /// Default `false`: sebagian plugin JS mengutip pesan lain (atau tidak
    /// mengutip apa pun), jadi mengutip pemicu selalu bukan perilaku yang
    /// benar.
    pub quote: bool,
    /// `contextInfo.isForwarded` (JS `menucat.js:283`).
    pub forwarded: bool,
    /// `contextInfo.forwardingScore` (JS `menucat.js:283`).
    pub forwarding_score: Option<u32>,
    /// `contextInfo.participant` (JS: `"0@s.whatsapp.net"`).
    pub participant: Option<String>,
    /// `contextInfo.quotedMessage` — teks yang dikutip. JS `menucat.js:290`
    /// mengutip `conversation` berisi nama bot, bukan pesan pemicu.
    pub quote_teks: Option<String>,
    /// Teks cadangan kalau kartu gagal terkirim.
    ///
    /// JS melakukan hal yang sama di `menu.js:1379` (catch → `m.reply(text)`):
    /// user tetap dapat isi menu walau WA menolak merender kartunya.
    pub fallback_teks: Option<String>,
}

impl Kartu {
    pub fn baru(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            ..Default::default()
        }
    }

    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    pub fn header(mut self, header: KartuHeader) -> Self {
        self.header = Some(header);
        self
    }

    pub fn tombol(mut self, tombol: Tombol) -> Self {
        self.buttons.push(tombol);
        self
    }

    pub fn params(mut self, params: Value) -> Self {
        self.params = Some(Params::baru(params));
        self
    }

    pub fn mention(mut self, jid: impl Into<String>) -> Self {
        self.mentions.push(jid.into());
        self
    }

    /// Kartu mengutip pesan pemicu (JS `quoted: m`).
    pub fn with_quote(mut self) -> Self {
        self.quote = true;
        self
    }

    /// Kartu ditandai sebagai pesan ter-forward (JS `isForwarded: true`).
    pub fn forwarded(mut self) -> Self {
        self.forwarded = true;
        self
    }

    /// `contextInfo.forwardingScore` (JS `menucat.js:283`).
    pub fn forwarding_score(mut self, skor: u32) -> Self {
        self.forwarding_score = Some(skor);
        self
    }

    /// `contextInfo.participant` (JS: `"0@s.whatsapp.net"`).
    pub fn participant(mut self, participant: impl Into<String>) -> Self {
        self.participant = Some(participant.into());
        self
    }

    /// Kartu mengutip teks tertentu (JS `quotedMessage: { conversation }`).
    pub fn quote_teks(mut self, teks: impl Into<String>) -> Self {
        self.quote_teks = Some(teks.into());
        self
    }

    /// Teks cadangan kalau kartu gagal terkirim (JS `catch` → `m.reply`).
    pub fn fallback_teks(mut self, teks: impl Into<String>) -> Self {
        self.fallback_teks = Some(teks.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nama_aksi_sesuai_daftar_wa() {
        assert_eq!(AksiTombol::BalasanCepat.as_str(), "quick_reply");
        assert_eq!(AksiTombol::PilihSatuan.as_str(), "single_select");
        assert_eq!(AksiTombol::BukaUrl.as_str(), "cta_url");
        assert_eq!(AksiTombol::Salin.as_str(), "cta_copy");
        assert_eq!(AksiTombol::from_name("quick_reply"), Some(AksiTombol::BalasanCepat));
        assert_eq!(AksiTombol::from_name("cta_misterius"), None);
    }

    #[test]
    fn tombol_balas_cepat_memakai_display_text_dan_id() {
        let t = Tombol::balas_cepat("🧀 Owner", ".owner");
        assert_eq!(t.aksi, AksiTombol::BalasanCepat);
        assert_eq!(t.params["display_text"], "🧀 Owner");
        assert_eq!(t.params["id"], ".owner");
    }

    #[test]
    fn tombol_salin_memuat_copy_code() {
        let t = Tombol::salin("📋 Copy", "copy", "halo");
        assert_eq!(t.params["display_text"], "📋 Copy");
        assert_eq!(t.params["id"], "copy");
        assert_eq!(t.params["copy_code"], "halo");
    }

    #[test]
    fn tombol_url_menyalin_ke_merchant_url() {
        let t = Tombol::buka_url("🥐 GitHub", "https://github.com/OktzO");
        assert_eq!(t.params["url"], "https://github.com/OktzO");
        assert_eq!(t.params["merchant_url"], "https://github.com/OktzO");
    }

    #[test]
    fn single_select_menghasilkan_params_ala_js() {
        let daftar = Daftar::baru("🍃 List Menu")
            .bagian(
                Bagian::baru("Berikut adalah pilihan nya (1 kategori)")
                    .baris(Baris::baru("🏠 main", ".menucat main").description("12 command")),
            )
            .icon("DEFAULT");
        let t = Tombol::pilih(&daftar);
        assert_eq!(t.aksi, AksiTombol::PilihSatuan);
        assert_eq!(t.params["title"], "🍃 List Menu");
        assert_eq!(t.params["icon"], "DEFAULT");
        assert_eq!(t.params["sections"][0]["title"], "Berikut adalah pilihan nya (1 kategori)");
        assert_eq!(t.params["sections"][0]["rows"][0]["title"], "🏠 main");
        assert_eq!(t.params["sections"][0]["rows"][0]["description"], "12 command");
        assert_eq!(t.params["sections"][0]["rows"][0]["id"], ".menucat main");
    }

    #[test]
    fn bagian_tanpa_judul_tidak_muncul_di_json() {
        let bagian = Bagian::default().baris(Baris::baru("a", ".a"));
        let json = serde_json::to_value(&bagian).unwrap();
        assert!(json.get("title").is_none(), "{json}");
        assert!(json.get("highlight_label").is_none(), "{json}");
        assert_eq!(json["rows"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn kartu_menyimpan_semua_bagian() {
        let kartu = Kartu::baru("body")
            .footer("footer")
            .header(KartuHeader::baru().judul("").subjudul("Bot"))
            .tombol(Tombol::balas_cepat("x", ".x"))
            .params(serde_json::json!({"bottom_sheet": {}}))
            .mention("628@s.whatsapp.net");
        assert_eq!(kartu.body, "body");
        assert_eq!(kartu.footer.as_deref(), Some("footer"));
        assert_eq!(kartu.header.unwrap().subtitle.as_deref(), Some("Bot"));
        assert_eq!(kartu.buttons.len(), 1);
        assert!(kartu.params.is_some());
        assert_eq!(kartu.mentions, vec!["628@s.whatsapp.net"]);
    }
}
