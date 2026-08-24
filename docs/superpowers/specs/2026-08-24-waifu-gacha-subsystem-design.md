# Design: Waifu Gacha Subsystem v2 — OURIN-MD

Tanggal: 2026-08-24
Status: Approved (desain di-chat, user setuju + 2 tambahan: integrasi coin/level/EXP & kompatibilitas Turso)

## 1. Tujuan

Mengganti plugin `plugins/fun/gachawaifu.js` (40 waifu hardcoded, satu file) menjadi
subsystem modular dengan:

1. **300+ waifu** di folder data terpisah per-franchise — mudah ditambah tanpa sentuh logika.
2. **5 tier** gacha (tambah Mythic) + sistem pity.
3. **Random Event System** — event acak setelah aksi waifu.
4. **Jealousy/Cemburu Trigger** — waifu bereaksi saat pemain memakai command fun/game lain.
5. **Album & Statistik Luck** — riwayat pull + agregat luck score.
6. **Mood Harian + Anniversary** — mood mengubah multiplier affection; hadiah harian pasca-nikah.
7. **Aksi diperluas** di semua fase + sub-pilihan lebih banyak.
8. **Integrasi economy**: koin (`db.updateKoin`), EXP (`addExpWithLevelCheck` → level-up card otomatis), energi/limit.
9. **100% kompatibel Turso**: semua state hidup di objek user (`getUser`/`setUser`) yang sudah
   dual-write lowdb+Turso oleh `ourin-database.js`. Tidak ada tabel/skema baru.

## 2. Arsitektur File

```
data/waifu/
├── index.js              ← loader: merge franchise files, validasi entry, dedupe nama,
│                           hitung bobot tier agregat, export getPool()/searchPool()
├── _template.js          ← contoh format entry (dokumentasi hidup untuk pengisi data)
├── naruto.js             ← file per-franchise: export const waifus = [ {...}, ... ]
├── one-piece.js
├── bleach.js
├── kimetsu.js
├── jujutsu-kaisen.js
├── ... (±30 file franchise; total target ≥300 entry)
src/lib/ourin-waifu.js    ← logika inti (pure-ish, tanpa dependensi WA):
                            rollWaifu(pity), applyAction(), rollEvent(),
                            getDailyMood(), jealousyCheck(), albumStats()
plugins/fun/gachawaifu.js ← refactor tipis: UI gacha + routing tombol aksi
plugins/fun/waifualbum.js ← BARU: .waifualbum — riwayat + statistik luck
plugins/fun/waifupool.js  ← BARU: .waifupool [nama|tier] — browse/search pool
```

Loader `index.js` menggunakan `import.meta.url` + scan direktori (atau daftar eksplisit
franchise modules — dipilih eksplisit agar aman terhadap hot-reload & bundling).
Entry rusak/tanpa field wajib di-skip + `logger.warn`.

## 3. Schema Entry Waifu

```js
{
  name: "Miku Nakano",            // required, unik lintas franchise
  series: "Gotoubun no Hanayome", // required
  age: 17, height: "165 cm", weight: "49 kg",   // required
  tier: "Rare",                   // Common | Rare | Epic | Legendary | Mythic
  personality: "dandere",         // wajib, lihat tabel personality
  keyword: "..."                  // optional override keyword Pinterest;
                                  // default auto: `${name} ${series} anime`
}
```

### Personality types (10) — memengaruhi multiplier hasil aksi

| type | ciri | suka (+20%) | netral | benci (−30%) |
|------|------|-------------|--------|--------------|
| tsundere | malu-melu, kasar | jalanjalan sederhana | intim | pujian berlebihan |
| deredere | manja terbuka | semua mesra | — | diabaikan |
| kuudere | dingin tenang | bioskop, piknik | umum | arcade riuh |
| dandere | pendiam | kafe, masak | umum | keramaian mall |
| yandere | posesif | peluk/cium intens | umum | tinggal sendirian |
| onee-san | dewasa penyayang | masak, tidur bareng | umum | — |
| genki | energik riang | karaoke, arcade, piknik | umum | aktivitas membosankan |
| himedere | ingin dimanjakan | belanja mahal, hadiah | umum | hadiah murahan |
| ojou-sama | bangsawan elegan | restoran/perhiasan | umum | tempat murahan |
| amayadori* | santai ngantuk | tidur bareng, piknik | umum | olahraga berat |

(*nama fleksibel saat implementasi)

## 4. Tier, Bobot & Pity

| Tier | % chance | target jumlah | dowry nikah |
|------|----------|---------------|-------------|
| Common | 55% | ~120 | 1.000 limit, 20.000 koin, 500 EXP |
| Rare | 25% | ~90 | 3.000 limit, 60.000 koin, 1.500 EXP |
| Epic | 13% | ~55 | 8.000 limit, 200.000 koin, 5.000 EXP |
| Legendary | 5.5% | ~25 | 15.000 limit, 500.000 koin, 12.000 EXP |
| Mythic | 1.5% | ~12 | 30.000 limit, 1.000.000 koin, 30.000 EXP |

Pity (state `user.waifuStats.pityCounter`, lifetime): tiap 20 roll tanpa Epic+ → guaranteed Epic+
(dan reset). Legendary/Mythic murni RNG. Roll hanya bisa saat single (aturan existing tetap).

## 5. Random Event System

Setelah aksi selesai (bukan aksi navigasi/pemilih sub-menu): **18% chance** event.
State tidak perlu baru; efek langsung ditampilkan sebagai blok teks tambahan.

Event pool (contoh, final ±12, beberapa gate per fase):

- 🌧️ Hujan deras, berdua satu payung → +aff kecil
- 💸 Nemu dompet & balikkan → +koin random 1k–25k
- 😠 Ketemu rival dekat waifu → −aff kecil (kecuali yandere: +5)
- 🎤 Idol favoritnya ketemu di jalanan → mood naik, +aff
- 🐱 Kucing lucu diselamatkan → +aff besar jika deredere/genki
- 🎰 Tiket lotre jatuh dari langit → +koin besar atau 0 (50/50)
- 📉 Waktu hampir telat janji → −mood, aksi berikutnya ×0.8 (satu hari)
- 💍 (married only) Kenangan pernikahan teringang → +aff
- 🔥 (fase intim only) Momen berlanjut → +aff besar
- dst.

Event memodifikasi affection/koin/mood via helper yang sama dengan aksi biasa.

## 6. Mood Harian + Anniversary

- State: `user.waifu.mood = { type: "ceria|romantis|biasa|sedih|marah", since: ISO }`
  dan `user.waifu.moodUntil` untuk mood sementara akibat event.
- Mood dasar harian: seeded-random dari `(tanggal Asia/Jakarta + senderJid)` → konsisten sepanjang hari,
  berbeda antar user.
- Multiplier affection: ceria ×1.3, romantis ×1.2, biasa ×1.0, sedih ×0.7, marah ×0.5.
- Anniversary: `user.waifu.marriedDate` dicatat saat nikah. `.waifuku` menampilkan
  `💍 Hari ke-N`. Istri boleh klaim hadiah harian sekali/hari:
  koin 5k–20k + EXP 200–800 (diskalakan tier). Bonus milestone day 7/30/100.

## 7. Economy Integration (coin / EXP / level / energi)

Semua reward lewat API existing:

```js
import { addExpWithLevelCheck } from "../../src/lib/ourin-level.js";
db.updateKoin(m.sender, amount);                       // koin
await addExpWithLevelCheck(sock, m, db, user, exp);    // EXP + level-up card otomatis
db.updateEnergi(m.sender, amount);                     // limit/energi (dowry, hadiah)
```

EXP juga didapat kecil dari tiap aksi positif (25–75) supaya main waifu = progres level.
Level-up notification memakai mekanisme existing (canvas card).

## 8. Jealousy Trigger

Satu titik integrasi di `src/handler.js` tepat setelah `await plugin.handler(m, context)` (~line 1802):

```js
if (["fun","game"].includes(plugin.config.category)) {
  const { jealousyCheck } = await import("./lib/ourin-waifu.js");
  await jealousyCheck({ m, sock, db, command: m.command }).catch(() => {});
}
```

Aturan `jealousyCheck`:
- Skip jika command termasuk command waifu sendiri (`gachawaifu|waifuaction|waifuku|...|waifualbum|waifupool`).
- Skip jika user tidak punya waifu.
- Cooldown internal 45 menit per user (disimpan `user.waifu.lastJealousAt`) anti-spam.
- Belum menikah: 15% chance bereaksi → aff −3..−8, kadang mood turun; pesan singkat m.reply().
- Sudah menikah (istri): 8% chance, −1..−3, nada pengertian.

## 9. Album & Statistik Luck (`.waifualbum`)

State baru pada user:
```js
user.waifuHistory = [ { name, series, tier, at } ]      // max 100 terakhir (shift)
user.waifuStats = { totalGacha, byTier: {...}, pityCounter,
                    rarest: { name, tier }, marriedCount }
```
Output: total gacha, distribusi tier, rarest pull, streak pity aktif, **luck score**
= rasio nilai tier didapat vs nilai ekspektasi bobot (≥1 = hoki). Riwayat 10 pull terakhir ditampilkan.

`.waifupool [query]`: cari nama/franchise/tier, tampil paginasi tombol quick_reply
(⬅️➡️), maks ±10 entri/halaman, menampilkan tier & personality.

## 10. Aksi Diperluas

- Pendekatan (<80): jalan-jalan(4 tujuan), kafe(4 menu), bioskop(3 genre), belanja(3),
  **karaoke**(2), **arcade**(2), **piknik**(2), **masak bareng**(2)
- Intim (80–99): peluk/cium/tidur/mandi sub-pilihan ditambah + **gendong**, **tepuk kepala**
- Married: mesra, rayu, hadiah, **bulan madu**(2 sub-pilihan)
- Semua hasil aksi melewati: personality multiplier × mood multiplier → clamp 0–100.
- Aturan anti-mesum <80 tetap (reject + tampar), pesan variasi per personality.

## 11. Kompatibilitas Turso

- Tidak ada skema/tabel baru. Seluruh state (`user.waifu`, `user.waifuHistory`,
  `user.waifuStats`) tersimpan via `db.getUser/setUser` → otomatis ikut
  `flushAllToTurso()` / dirty-flush interval existing di `ourin-database.js`.
- Data file `data/waifu/*.js` adalah kode statis (bukan state) — tidak masuk DB.
- Migrasi cloud Turso user: nol pekerjaan tambahan dari fitur ini.

## 12. Error Handling

- Loader: entry invalid di-skip + warning (nama file + alasan).
- Gambar: urutan fallback sama seperti existing (imageUrl cache → Pinterest API → URL default).
- Jealousy hook dibungkus try/catch — gagal hook TIDAK boleh menggagalkan command utama.
- Semua path plugin tetap punya try-catch global handler existing.

## 13. Testing

`npm test` (node --test):

- `tests/waifu-data.test.mjs`: load semua franchise file → ≥300 entry; tidak ada nama duplikat;
  tier/personality valid; field wajib lengkap; distribusi jumlah per tier mendekati target.
- `tests/waifu-lib.test.mjs`: distribusi roll sanity (χ² longgar / proporsi kasar),
  pity guarantee, mood seeded deterministik (sama hari+jid → sama mood), personality×mood
  multiplier math, jealousy cooldown logic, album stats & luck score.

Plugin handler (WA I/O) tidak di-unit-test sesuai pola repo.

## 14. Non-Tujuan (YAGNI)

- Tidak mengubah fitur lain (rpg/store/cekpacar) selain titik integrasi yang disebut.
- Tidak membuat tabel SQL baru / API eksternal karakter baru.
- Tidak menambah NSFW content — konten intim tetap seputar level existing (hint, fade-to-black).
