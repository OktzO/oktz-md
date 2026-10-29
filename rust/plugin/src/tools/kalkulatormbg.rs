use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const PENGELUARAN_PER_HARI: f64 = 319_600_000_000.0;

pub(crate) const HARGA_PORSI: f64 = 15_000.0;

pub(crate) const UMR_DKI: f64 = 5_400_000.0;

pub(crate) const UMR_JATENG: f64 = 2_040_000.0;

pub(crate) const GAJI_GURU_HONORER: f64 = 300_000.0;

pub(crate) const PEMAIN: [(&str, f64); 10] = [
    ("Cristiano Ronaldo (Al Nassr)", 4_500_000_000_000.0),
    ("Lionel Messi (Inter Miami)", 2_100_000_000_000.0),
    ("Karim Benzema (Al-Ittihad)", 1_700_000_000_000.0),
    ("Kylian Mbappé (Real Madrid)", 1_500_000_000_000.0),
    ("Erling Haaland (Man City)", 1_300_000_000_000.0),
    ("Vinícius Jr. (Real Madrid)", 960_000_000_000.0),
    ("Mohamed Salah (Liverpool)", 880_000_000_000.0),
    ("Sadio Mané (Al Nassr)", 864_000_000_000.0),
    ("Jude Bellingham (Real Madrid)", 704_000_000_000.0),
    ("Lamine Yamal (Barcelona)", 688_000_000_000.0),
];

pub(crate) const HELP_KKMBG_HEAD: &str = "🧮 *KALKULATOR MBG (Makan Bergizi Gratis)* 🧮";

pub(crate) const HELP_KKMBG_BODY: &str = "Halo kak! Penasaran berapa lama uang kamu bisa nyuplai program Makan Bergizi Gratis se-Indonesia?\n\n*Cara Pakai:*\n👉 `{prefix}kkmbg <nominal uang>`\n\n*Contoh:*\n`.kkmbg 1000000000`";

pub(crate) const KKMBG_INVALID: &str =
    "❌ Kak, tolong masukin angka uang yang valid ya! (Cuma angka aja, misal 500000)";

pub(crate) struct Durasi {
    pub(crate) tahun: u64,
    pub(crate) bulan: u64,
    pub(crate) hari: u64,
    pub(crate) jam: u64,
    pub(crate) menit: u64,
    pub(crate) detik: f64,
}

/// `Number(raw.replace(/[^0-9]/g, ''))` — buang semua non-digit, lalu parse.
/// String kosong jadi `0` (`Number("") === 0`), sama seperti JS.
pub(crate) fn parse_uang(raw: &str) -> f64 {
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    digits.parse::<f64>().unwrap_or(0.0)
}

/// `Number.prototype.toLocaleString('id-ID')` untuk bilangan bulat: pemisah
/// ribuan titik (`1000000000` → `1.000.000.000`).
pub(crate) fn thousands_id(n: f64) -> String {
    let n = n.abs() as u64;
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// `angka < 1000 ? "… Perak" : "Rp …"`.
pub(crate) fn format_rupiah(angka: f64) -> String {
    let formatted = thousands_id(angka);
    if angka < 1000.0 {
        format!("{formatted} Perak")
    } else {
        format!("Rp {formatted}")
    }
}

pub(crate) fn hitung_durasi(hari_float: f64) -> Durasi {
    Durasi {
        tahun: (hari_float / 365.0).floor() as u64,
        bulan: ((hari_float % 365.0) / 30.0).floor() as u64,
        // JS: `hariFloat % 30` — BUKAN `(hariFloat % 365) % 30`.
        hari: (hari_float % 30.0).floor() as u64,
        jam: ((hari_float % 1.0) * 24.0).floor() as u64,
        menit: (((hari_float * 24.0) % 1.0) * 60.0).floor() as u64,
        detik: ((hari_float * 24.0 * 60.0) % 1.0) * 60.0,
    }
}

pub(crate) fn render_mbg(uang: f64) -> String {
    let durasi = hitung_durasi(uang / PENGELUARAN_PER_HARI);
    let porsi = (uang / HARGA_PORSI).floor() as u64;

    let mut c = String::new();
    c.push_str(&format!("💰 *Dana :* {}\n\n", format_rupiah(uang)));
    c.push_str("⏳ *Durasi MBG:*\n");
    c.push_str(&format!(
        "{} TAHUN, {} BULAN, {} HARI\n",
        durasi.tahun, durasi.bulan, durasi.hari
    ));
    c.push_str(&format!(
        "{} JAM, {} MENIT, {:.2} DETIK\n",
        durasi.jam, durasi.menit, durasi.detik
    ));
    c.push_str(&format!(
        "_(Berdasarkan pengeluaran ~Rp {:.1} Miliar/hari)_\n\n",
        PENGELUARAN_PER_HARI / 1_000_000_000.0
    ));

    c.push_str("🍱 *Setara Porsi Makan:*\n");
    c.push_str(&format!(
        "{} porsi (@ Rp 15.000/porsi)\n\n",
        thousands_id(porsi as f64)
    ));

    c.push_str("📊 *Perbandingan Gaji Indonesia:*\n");
    c.push_str(&format!(
        "🏢 UMR DKI Jakarta (Rp 5,4 Jt/bulan): {:.1}%\n",
        uang / UMR_DKI * 100.0
    ));
    c.push_str(&format!(
        "🏭 UMR Jawa Tengah (Rp 2,04 Jt/bulan): {:.1}%\n",
        uang / UMR_JATENG * 100.0
    ));
    c.push_str(&format!(
        "👨‍🏫 Gaji Guru Honorer (Rp 300rb/bulan): {:.1}x\n\n",
        uang / GAJI_GURU_HONORER
    ));

    c.push_str("⚽ *Perbandingan Gaji Pesepakbola:*\n");
    for (nama, gaji) in PEMAIN {
        let persen = uang / gaji * 100.0;
        c.push_str(&format!("🏆 {nama}\n"));
        c.push_str(&format!("💵 {}/tahun\n", format_rupiah(gaji)));
        // JS: `persen < 0.0001 ? "0%" : persen.toFixed(4) + "%"`
        c.push_str(&format!(
            "📈 Persentase: {}\n\n",
            if persen < 0.0001 {
                "0%".to_string()
            } else {
                format!("{persen:.4}%")
            }
        ));
    }

    format!("🍽️ *HASIL HITUNG KALKULATOR MBG* 🍽️\n\n{}", c.trim())
}

#[plugin(
    name = "kalkulatormbg",
    alias = ["kkmbg"],
    category = "tools",
    description = "Hitung durasi dan perbandingan dana Makan Bergizi Gratis (MBG)",
    usage = ".kkmbg <jumlah_uang>",
    example = ".kkmbg 1000000000",
    cooldown = 5,
    energi = 1,
    is_private = false
)]
pub fn kalkulatormbg(ctx: Ctx) -> Reply {
    let Some(raw) = ctx.args.first() else {
        return Reply::text(format!(
            "{HELP_KKMBG_HEAD}\n\n{}",
            HELP_KKMBG_BODY.replace("{prefix}", &ctx.prefix)
        ));
    };

    // JS: `isNaN(uang) || uang <= 0`. `parse_uang` tidak pernah menghasilkan
    // NaN (digit kosong -> 0.0), jadi cukup cek <= 0.
    let uang = parse_uang(raw);
    if uang <= 0.0 {
        return Reply::text(KKMBG_INVALID);
    }

    Reply::text(render_mbg(uang))
}
