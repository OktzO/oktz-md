//! Operasi ekonomi murni yang digunakan oleh game (koin / exp).
//!
//! `ponytail:` `db.updateKoin` / `db.updateEnergi` / `addExpWithLevelCheck`
//! di `games.js` butuh `sock` + `db` (I/O), yang belum ada di `Ctx`. Hanya
//! fungsi hitungan polos yang dipindah ke sini.

/// `addCoins` — tambah koin ke saldo. Angka boleh negatif (pengurangan).
///
/// Padanan `db.updateKoin(jid, saldoLama + amount)` di JS.
pub fn add_coins(saldo: i64, amount: i64) -> i64 {
    saldo.saturating_add(amount)
}

/// `addExp` — tambah exp. Angka boleh negatif (pengurangan).
///
/// Padanan penambahan `exp` sebelum `addExpWithLevelCheck` di JS.
pub fn add_exp(exp: i64, amount: i64) -> i64 {
    exp.saturating_add(amount)
}

/// `expForLevel` = `(level - 1) * EXP_PER_LEVEL` (`level.js`).
pub const EXP_PER_LEVEL: i64 = 10_000;

/// Level dari exp: `Math.floor(exp / EXP_PER_LEVEL) + 1`.
pub fn calculate_level(exp: i64) -> i64 {
    exp.div_euclid(EXP_PER_LEVEL) + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------- add_coins

    #[test]
    fn add_coins_positif_tambah_saldo() {
        assert_eq!(add_coins(100, 50), 150);
    }

    #[test]
    fn add_coins_negatif_kurang_saldo() {
        assert_eq!(add_coins(100, -30), 70);
    }

    #[test]
    fn add_coins_negatif_lebih_dari_saldo_tetap_nol() {
        // `saturating_add` tidak akan underflow di i64, tapi JS `+` bisa
        // jadi negatif. Sengaja pakai saturating agar saldo tidak minus.
        assert_eq!(add_coins(5, -10), 0);
    }

    #[test]
    fn add_coins_nol_tidak_mengubah() {
        assert_eq!(add_coins(42, 0), 42);
    }

    // ------------------------------------------------- add_exp

    #[test]
    fn add_exp_positif_tambah_exp() {
        assert_eq!(add_exp(1000, 500), 1500);
    }

    #[test]
    fn add_exp_negatif_kurang_exp() {
        assert_eq!(add_exp(1000, -300), 700);
    }

    #[test]
    fn add_exp_negatif_lebih_dari_exp_tetap_nol() {
        assert_eq!(add_exp(5, -10), 0);
    }

    // ------------------------------------------------- calculate_level

    #[test]
    fn calculate_level_nilai_awal_user_baru() {
        // User baru exp = 0 -> level 1.
        assert_eq!(calculate_level(0), 1);
    }

    #[test]
    fn calculate_level_batas_level_exp() {
        assert_eq!(calculate_level(9_999), 1);
        assert_eq!(calculate_level(10_000), 2);
        assert_eq!(calculate_level(99_999), 10);
        assert_eq!(calculate_level(100_000), 11);
    }

    #[test]
    fn calculate_level_negatif() {
        // `div_euclid` untuk pembagi positif = floor sama seperti JS.
        assert_eq!(calculate_level(-1), 0);
        assert_eq!(calculate_level(-10_000), 0);
        assert_eq!(calculate_level(-10_001), -1);
    }
}