# Design: GachaHusbu Upgrade + Complexity Pass (Waifu & Husbu) — OURIN-MD

Tanggal: 2026-08-25
Status: Draft (menunggu review user)

## 1. Tujuan

1. **GachaHusbu setara GachaWaifu** — pool 300+ husbu di folder data per-franchise,
   personality, mood, event, pity, album, pool browse, jealousy, dowry per tier.
2. **Sistem kesulitan & emosi baru** (berlaku dua gender): diminishing affection gain,
   fatigue interaksi harian, neglect decay, sistem marah (anger meter).
3. **Konten diperluas**: +12 aksi kencan, +1 intim, +2 married, +8 event acak per gender.
4. **Femboy** sebagai personality ke-11 untuk husbu.
5. Test waifu existing tetap hijau.

## 2. Arsitektur File

```
data/husbu/                 ← BARU, mirror data/waifu/
├── index.js                ← loader: merge franchise, validasi, dedupe, bobot tier
├── _template.js
├── naruto.js, one-piece.js, bleach.js, jujutsu-kaisen.js, kimetsu.js,
├── my-hero-academia.js, attack-on-titan.js, dragon-ball.js, hunter-x-hunter.js,
├── black-clover.js, haikyuu.js, kuroko.js, blue-lock.js, fairy-tail.js,
├── sword-art-online.js, rezero.js, fate.js, genshin-impact.js, honkai-star-rail.js,
├── bungo-stray-dogs.js, spy-x-family.js, chainsaw-man.js, tokyo-revengers.js,
├── one-punch-man.js, death-note.js, code-geass.js, fullmetal-alchemist.js,
├── tokyo-ghoul.js, seven-deadly-sins.js, kaguya.js, cote.js,
├── assassination-classroom.js, oregairu.js, baka-to-test.js
└── (total ±34 file franchise → target ≥300 entry; femboy tersebar di file asalnya)

src/lib/ourin-romance.js    ← BARU: sistem kesulitan/emosi shared, konstanta (DOWRY,
│                             TIER_WEIGHTS, TIER_ORDER) + jealousyCheckAll
src/lib/ourin-waifu.js      ← +12 aksi, +8 event, result {like,dislike}; signature applyAction tetap
src/lib/ourin-husbu.js      ← BARU: mirror waifu, konten maskulin + femboy, import DOWRY shared

plugins/fun/gachawaifu.js   ← upgrade: menu baru + difficulty + panel anger
plugins/fun/gachahusbu.js   ← rewrite total: mirror gachawaifu, konten husbu
plugins/fun/husbualbum.js   ← BARU: mirror waifualbum
plugins/fun/husbupool.js    ← BARU: mirror waifupool
src/handler.js              ← jealousyCheck → jealousyCheckAll (romance.js)

tests/husbu-data.test.mjs   ← BARU, mirror waifu-data.test.mjs
tests/husbu-lib.test.mjs    ← BARU, mirror waifu-lib.test.mjs
tests/romance-lib.test.mjs  ← BARU: diminish, fatigue, neglect, anger
```

## 3. Schema Entry Husbu

Sama dengan waifu: `{ name, series, age, height, weight, tier, personality, keyword? }`.
`keyword` default `${name} ${series} anime`.

## 4. Personality

**Waifu (tetap, 10):** tsundere, deredere, kuudere, dandere, yandere, onee-san, genki,
himedere, ojou-sama, amayadori.

**Husbu (baru, 11):**
| Personality | Arketipe | Contoh |
|---|---|---|
| tsundere | Pahit dingin, lembut dalam | Bakugo, Todoroki, Sasuke |
| kuudere | Dingin & kalem | Gojo, Giyuu, Zoro, Kageyama |
| genki | Energik ceria | Tanjiro, Asta, Denji |
| yandere | Posesif obsesif | Yuno, Sukuna |
| dandere | Pendiam pemalu | Inumaki, Midoriya |
| oji-san | Dewasa protektif | Loid, Levi, Zhongli |
| playboy | Genit menggoda | Sanji, Dazai, Tartaglia |
| prince | Anggun berwibawa | Gilgamesh, Ayato, Neuvillette |
| badboy | Berbahaya misterius | Toji, Aventurine, Dabi |
| sunao | Jujur lembut | Hinata (Haikyuu), Ichigo, Oikawa |
| femboy | Lembut feminin | Astolfo, Felix, Venti, Nagisa, Saika, Hideyoshi |

## 5. Sistem Kesulitan & Emosi (`src/lib/ourin-romance.js`)

Semua pure function, diterapkan di lapisan plugin setelah `applyAction`.

1. **Diminishing gain**
   ```
   f = max(0.15, 1.05 - currentAffection/100)   // 50→0.55, 80→0.25, 100→0.15
   change = max(1, round(change * f))
   ```
2. **Fatigue harian**
   ```
   actionsToday <= 3 → ×1
   actionsToday > 3  → ×max(0.25, 1 - (actionsToday-3)*0.12)
   reset saat lastActionDate != hari ini
   ```
3. **Neglect decay** (diterapkan saat buka panel & sebelum aksi)
   ```
   hours < 24          → decay 0
   else decay = min(20, floor(hours/24) * 3)   // −3 per hari penuh, cap 20
   ```
4. **Sistem marah** — state `partner.anger` (0..100)
   - aksi dislike / event buruk → `anger += 12`
   - aksi like → `anger -= 8`
   - decay natural `-10`/hari (bersamaan neglect tick)
   - `anger >= 50` → mood dipaksa `marah` (gain ×0.5) + drain aff `-floor(anger/25)` per aksi
   - meter anger ditampilkan di panel status
   - `anger` tak pernah < 0 / > 100
5. **jealousyCheckAll** — gabung cek waifu + husbu; set command exclusions kedua gender.

## 6. Aksi & Event

**Aksi approach baru (+12, key sama dua gender):**
menu_kuliner: `restoran_makan, restoran_dimsum, restoran_bbq`
menu_olahraga: `olahraga_hiking, olahraga_lari, olahraga_panjat`
menu_alam: `alam_camping, alam_mancing, alam_perahu`
menu_seni: `seni_museum, seni_melukis, seni_konser`

**Intim baru:** `pijat_bahu`
**Married baru:** `nontonrumah, jalanpagi`

**Event baru (+8 per gender):** foto, lomba, salahpaham (anger up), pujian, sunset,
kenangan (married), badai (mood marah), rezeki (koin). Event `salahpaham` menaikkan `anger`.

`applyAction`/`applyHusbuAction` return tambahan `{ like, dislike }` (additive, tak
mengubah field existing → test lama aman).

## 7. Perilaku Plugin

**gachahusbu.js (rewrite)** — mirror gachawaifu:
- `husbuku` panel: personality, mood harian, anger meter, warning neglect.
- `husbuaction` menu berlapis; backfill personality hash (seed nama) untuk user lama.
- `nikah`: dowry via tabel `DOWRY` per tier (ganti hardcode 5000/100k), set `fun.pasangan`.
- `klaim` harian pasca-nikah + milestone hari ke-7/30/100.
- `hadiah`, `mesra`, `rayu`, reject intim (<80), leave saat aff 0.
- Pity 20 → dijamin Epic+; stats `husbuStats`, riwayat `husbuHistory` (cap 100).

**gachawaifu.js (upgrade)** — menu baru (kuliner/olahraga/alam/seni), panel anger,
neglect tick, fatigue & diminishing di aplikasikan.

**husbualbum.js / husbupool.js** — mirror persis waifualbum/waifupool, key `husbu*`.

**handler.js** — panggil `jealousyCheckAll` dari `ourin-romance.js`.

## 8. Migration

User lama punya `user.husbu` tanpa `personality/mood/anger` → backfill hash dari nama
(pilih dari 11 personality husbu). Field baru (anger, actionsToday, lastActionDate,
lastInteractionAt) default 0/null — aman terhadap data existing.

## 9. Testing

- `npm test` — waifu-data + waifu-lib lama tetap hijau (applyAction signature tak berubah).
- Baru: husbu-data (≥300 entry, no dup, tier valid, personality valid, distribusi), husbu-lib
  (pity, distribusi roll, applyAction personality+mood, rollEvent gate, text render, DOWRY,
  femboy render), romance-lib (diminish monotonic, fatigue, neglect calc, anger threshold).
- `npm run lint` (eslint plugins/) bersih.

## 10. Non-Goals

- Tidak ubah UI (tetap quick_reply buttons).
- Tidak tambah schema DB baru (semua state di objek user).
- Tidak ubah ekonomi existing (koin/limit/EXP) kecuali husbu nikah pakai DOWRY.
