//! Model tabel AI (WA `richResponseMessage`) untuk plugin.
//!
//! Padanan JS: `src/lib/rich-messages.js:608` (`generateTableContent`) dan
//! `src/lib/socket.js:1298` (`sock.sendTable`).
//!
//! Bentuknya: `botForwardedMessage` → `richResponseMessage` dengan
//! submessages: teks opsional (judul), `tableMetadata` (judul + baris), teks
//! opsional (footer). Baris pertama adalah heading (`isHeading: true`).
//!
//! `ponytail:` `unifiedResponse` (base64 JSON) dan `botMetadata` belum
//! dibuat — JS `generateTableContent` (v1) juga tidak memakainya, jadi
//! tabel v1 sudah sama persis dengan produksi.

/// Tabel AI: judul tabel, baris heading, dan isi baris.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tabel {
    /// `tableMetadata.title`.
    pub judul: String,
    /// Baris heading (diletakkan pertama, `isHeading: true`).
    pub header: Vec<String>,
    /// Isi tabel. Tiap baris = sel per kolom.
    pub baris: Vec<Vec<String>>,
    /// Teks sebelum tabel (`messageType: 2`).
    pub header_teks: Option<String>,
    /// Teks setelah tabel (`messageType: 2`).
    pub footer: Option<String>,
}

impl Tabel {
    pub fn baru(judul: impl Into<String>) -> Self {
        Self {
            judul: judul.into(),
            ..Default::default()
        }
    }

    /// Baris heading tabel.
    pub fn header(mut self, header: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.header = header.into_iter().map(Into::into).collect();
        self
    }

    /// Tambah satu baris isi.
    pub fn baris(mut self, baris: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.baris.push(baris.into_iter().map(Into::into).collect());
        self
    }

    /// Teks sebelum tabel (JS `options.headerText`).
    pub fn header_teks(mut self, teks: impl Into<String>) -> Self {
        self.header_teks = Some(teks.into());
        self
    }

    /// Teks setelah tabel (JS `options.footer`).
    pub fn footer(mut self, teks: impl Into<String>) -> Self {
        self.footer = Some(teks.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabel_menyimpan_judul_header_dan_baris() {
        let t = Tabel::baru("⚡ System Performance")
            .header(["Metric", "Value"])
            .baris(["Status", "Online"])
            .baris(["Cores", "4"])
            .header_teks("🎄 Dibawah ini adalah statistik bot kita")
            .footer("🍃 Realtime Monitoring");
        assert_eq!(t.judul, "⚡ System Performance");
        assert_eq!(t.header, vec!["Metric", "Value"]);
        assert_eq!(t.baris.len(), 2);
        assert_eq!(t.baris[1], vec!["Cores", "4"]);
        assert_eq!(
            t.header_teks.as_deref(),
            Some("🎄 Dibawah ini adalah statistik bot kita")
        );
        assert_eq!(t.footer.as_deref(), Some("🍃 Realtime Monitoring"));
    }
}
