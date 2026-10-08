# Design: Pre-Key Session Pruning

## Goal

1. Hentikan pertumbuhan `session_keys` tanpa batas di Turso — sekarang +30 key/hari.
2. Pangkas `pre-key` yang sudah tidak mungkin dipakai lagi.
3. Session harus tetap bertahan jangka panjang. Tidak ada yang perlu di-pairing ulang.

## Non-Goal

- Tidak menyentuh `session`, `sender-key`, `identity-key`, `device-list`, `lid-mapping`, `tctoken`. Semuanya adalah ratchet state atau trust anchor. Menghapus = session rusak.
- Tidak mengubah format penyimpanan `creds`.
- Tidak menyentuh `database/main/*.json` — lihat "Temuan lain" di bawah.

## Constraints

- Jadwal sweep: **3 hari sekali**, sesuai permintaan.
- Hentikan pertumbuhan di akar, bukan hanya menyapu gejalanya. Prune tanpa pasif akan terisi ulang 30 key/hari sebelum sweep berikutnya.
- Kalau Turso mati, sweep dilewati dan dicatat. Tidak boleh ganggu hot path.
- Nol dependency baru.

---

## Baseline — diukur di Turso produksi, 2026-10-08

`session_keys` = **11.736 keys / 3,0 MB**. Ini **18× lebih besar** dari seluruh DB bot (474.203 byte di `database/main/`).

| Kategori | Keys | Bytes | Bisa dihapus? |
|---|---|---|---|
| `pre-key` | 10.813 | 1.773.332 | ✅ ya |
| `sender-key` | 174 | 644.290 | ❌ ratchet percakapan |
| `session` | 490 | 505.197 | ❌ ratchet |
| `sender-key-memory` | 10 | 12.580 | ❌ |
| `lid-mapping` | 290 | 4.591 | ❌ |
| `identity-key` | 38 | 2.698 | ❌ trust anchor |
| `device-list` | 400 | 2.289 | ❌ emang (lihat Scope) |
| `tctoken` | 11 | 1.105 | ❌ |

`pre-key` = **59% dari seluruh `session_keys`** dan 30 dari 10.813 (94%) berasal dari satu batch terakhir.

Untuk perbandingan, storage lokal `storage/session/` cuma **12 file / 4.184 byte** — lokal sudah berputar, Turso tidak. Perbedaan itu sendiri bukti masalahnya: `trimLocalCache` memotong RAM tapi tidak pernah menyentuh Turso.

## Success criteria

| # | Kriteria | Ambang |
|---|---|---|
| 1 | `session_keys` total setelah prune | < 1,3 MB (turun ≥ 56%) |
| 2 | `pre-key` keys | ≤ 900 (dari 10.813) |
| 3 | Key baru per hari, kondisi steady state | tidak ada pertumbuhan bersih — jumlah pre-key tidak naik setelah sweep |
| 4 | `session`, `sender-key`, `identity-key`, `creds` | **row count dan byte identik** sebelum/sesudah |
| 5 | Bot tetap terhubung | tidak ada 401 setelah sweep, tidak ada reconnect loop |
| 6 | Sweep tidakucces | tidak boleh menghentikan bot; hanya log |

Kriteria 4 dijaga test — kalau sweep salah dan menyentuh kategori lain, byte-nya berubah dan test gagal.

### Hasil ukur aktual — 2026-10-08

Sweep produksi pertama dijalankan dan mengembalikan `deleted: 0`. **Itu benar, bukan kegagalan:** seluruh pre-key yang ada berumur 4,1 hari, jadi tidak ada yang melewati ambang 30 hari.

Kriteria 4 **terbukti terpenuhi** — `session` 493 key / 510.086 byte, `sender-key` 177 / 660.091, `identity-key` 40 / 2.840: semua utuh sebelum dan sesudah sweep.

Kriteria 1, 2, dan 3 **belum bisa dinilai.** Baseline spec mengasumsikan pre-key bertambah 30/hari; hasil ukur menunjukkan pola sebenarnya adalah burst — 10.828 key datang dalam ~4 burst (23, 364, 5.573, 3.829, 284, 15) mengikuti `uploadPreKeys` di setiap reconnect, bukan upload terjadwal. Angka "30/hari" dan proyeksi "~1,27 MB" karena itu tidak dapat dipertahankan sebagai fakta.

Ambang 30 hari sendiri tetap benar secara protokol, tapi steady-state hanya bisa ditentukan setelah satu siklus sweep berjalan di atas data nyata. Menilai ulang kriteria 1–3 setelah 30 hari data produksi.

---

## 1. Akar masalah

Baileys (fork `onigis` v10.1.0-rc.8) punya API delete resmi untuk pre-key di `lib/Signal/libsignal.js:517`:

```js
removePreKey: (id) => keys.set({ 'pre-key': { [id]: null } }),
```

`removePreKey` **tidak pernah dipanggil di mana pun** — hanya ada di baris definisinya.

 plumbing delete di sisi kita sudah benar. `turso-session.js:408-412`:

```js
if (value === null) {
  await withRemoteRetry(() => client.execute({
    sql: 'DELETE FROM session_keys WHERE scope = ? AND category = ? AND id = ?',
    args: [scope, type, id],
  }));
```

Jadi jalur delete sudah ada end-to-end. Yang hilang cuma pemanggilnya.

## 2. Mengapa menghapus pre-key aman

Ini dibuktikan dari source, bukan dari asumsi:

1. **Pre-key bersifat one-shot.** `lib/Socket/messages-recv.js:509-512` mengambil **tepat 1** pre-key per pesan masuk (`getNextPreKeys(authState, 1)`). Setelah dipakai, ID itu tidak pernah diminta lagi.
2. **Ada API delete resmi** di library — `libsignal.js:517` — yang dirancang persis untuk consumed pre-key.
3. **Kemampuan upload tidak hilang.** `lib/Socket/socket.js:382-419` sudah otomatis mengupload ulang pre-key baru. Server tidak menolak key yang sudah tidak ada.
4. **Yang dihapus hanya row-nya, bukan signed pre-key.** `creds.preKey` (ID signed pre-key yang aktif) ada di `session_creds` dan tidak tersentuh. Pre-key yang masih aktif juga aman karena ID-nya masih ada di cache.
5. **Turun ke backup.** `messages-recv.js:576` membaca stanza `pre-key count` dan memicu upload ulang bila stok kurang.

Kalau karena satu dan lain reasons pre-key aktif hilang, bot upload ulang sendiri dan tidak ada pengguna yang perlu pairing ulang.

## 3. Perubahan

### 3.1 Pasifikan `removePreKey`

Sumber pertumbuhan harus dimatikan, bukan hanya disapu.

Pilihan: patch `node_modules/onigis` (fragile, hilang saat reinstall) atau hook di `turso-session.js` saat `keys.set` menerima patch `pre-key` dengan nilai `null`.

**Keputusan: hook di `turso-session.js`.** Tidak menyentuh `node_modules`, tidak perlu postinstall patch, dan tetap berlaku kalau fork-nya di-upgrade. Plumbing `value === null` sudah ada di sana — tinggal memastikan delete lokal dan delete Turso keduanya terjadi untuk pre-key.

Setelah ini: setiap pre-key yang terpakai langsung hilang dari Turso. Pertumbuhan harian → ~0 tanpa perlu sweep.

### 3.2 Sweep 3 hari

Stub kecil untuk pre-key yang tidak pernah terpakai (mis. peer yang upload lalu hilang sebelum pakai, atau era sebelumnya sebelum pasif aktif).

Aturan:
- `DELETE FROM session_keys WHERE scope = ? AND category = 'pre-key' AND updated_at < ?`
- Ambang: **30 hari**
- Satu statement, satu round-trip. debeBasis data — bukan load semua key ke RAM.

Yang swept: **hanya `pre-key`**. Kategori lain di-query untuk laporan saja, tidak pernah ditulis.

### 3.3 Pelaporan

Setiap sweep mencatat: jumlah row per kategori sebelum/sesudah + byte yang dihapus. Kalau byte yang dihapus 0 selama 3 sweep berturut-turut, log info — artinya pasif sudah bekerja dan sweep jadi no-op.

Tanpa laporan ini, kita tidak bisa tahu apakah sweep masih berguna atau pasif sudah cukup.

### 3.4 Owner command

`sessionstats` — menampilkan bytes per kategori + jumlah key + umur pre-key tertua. Dipakai memverifikasi kriteria sukses tanpa menunggu 3 hari.

## 4. Scope: kenapa `device-list` dikeluarkan

`device-list` = 400 keys / 2.289 byte. Secara protokol masih bisa di-prune, tapi:
- hanya 0,08% dari `session_keys`
- peer mungkin perlu kirim ulang device list kalau dihapus
- risk/reward-nya tidak sepadan

Kalau nanti benar-benar butuh, TTL 90 hari masuk sebagai pekerjaan terpisah. Tidak sekarang.

## 5. Temuan lain (di luar scope)

Diukur, tidak dikerjakan di spec ini. Dicatat supaya tidak hilang:

**`users.json` = 343.423 byte, 604 user.** 331 user (55%, 169.677 byte) inactive >14 hari dan tidak pernah dibersihkan karena 3 bug mati:

| # | Lokasi | Bug |
|---|---|---|
| 1 | `data-pruner.js:48` | Baca `group.lastActivity` yang tidak pernah ditulis → prune grup mati selamanya |
| 2 | `data-pruner.js:89-96` | Sweep cooldown kedaluwarsa tidak pernah `markDirty("users")` → expired tertinggal 6 jam+ |
| 3 | `plugins/owner/clearsessions.js:21` | Nunjuk `storage/sessions/` (jamak); path asli `storage/session/` → command no-op |

Perhatikan: `data-pruner.js:29-42` **sudah** menghapus user yang `lastSeen` >14 hari. Itu hard delete, dan di antara 331 user itu ada yang punya `koin: 100000` dan `exp: 2355`. Kalau bot dimatikan >14 hari, progres user hilang permanen — bertentangan dengan tujuan "bisa ditinggal jangka panjang tanpa hapus session".

Perbaikannya: ganti hard delete jadi arsip (pindah ke tier cold, restore transparan). Butuh desain tersendiri — tidak dicampur ke spec pruning pre-key.

**`sender-key` 217.903 byte di satu baris** (`120363428677398852@g.us::188412430651393_1::54`) — grup 1.883 member. Ratchet state, tidak boleh dihapus, tapi di-stringify ulang tiap flush.ipeluang optimasi terpisah.

**`backup_snapshots` 0 baris** — tabel ada tapi tidak pernah dipakai. Dead schema.

---

## 6. Testing

`node:test`, Turso via `file::memory:`, `now` di-inject.

| Test | Yang dijaga |
|---|---|
| `prekey-delete.test.mjs` | `keys.set({'pre-key': {id: null}})` menghapus row lokal **dan** Turso |
| `prekey-nontarget.test.mjs` | patch null di kategori selain `pre-key` **tidak** dihapus di luar jalur resmi |
| `prekey-sweep.test.mjs` | hanya `pre-key` >30 hari yang terhapus; `session`/`sender-key`/`identity-key` row-count identik |
| `prekey-degraded.test.mjs` | Turso mati saat sweep → bot tetap jalan, hanya log |
| `prekey-boundary.test.mjs` | tepat 30 hari belum terhapus, 30 hari + 1ms sudah |

Lima file. Nol dependency baru.

Kriteria 4 dijaga `prekey-sweep.test.mjs`: test memastikan byte per kategori lain identik sebelum/sesudah sweep. Kalau regresi membuat sweep menyasar kategori lain, test gagal.

## 7. Risiko

| Risiko | Mitigasi |
|---|---|
| Pre-key aktif terhapus | Sweep pakai umur 30 hari; pre-key aktif selalu <30 hari. Amankan oleh `prekey-boundary` test. |
| Bot kena 401 setelah sweep | Tidak mungkin — `session`/`sender-key`/`identity-key` tidak tersentuh. Kriteria 4 + 5 mengunci ini. |
| Sweep berjalan saat trafik tinggi | Statement DELETE satu round-trip, tanpa lock panjang. Jadwal 3 hari membuat frekuensi sangat rendah. |
| Pasif damaging ratchet state | Hook hanya meng cares `pre-key` dan hanya saat nilai `null` — persis kontrak `removePreKey`. |
| `node_modules` di-upgrade dan behavior berubah | Kita tidak patch `node_modules`. Kalau `onigis` nanti memanggil `removePreKey` sendiri, hook Idempoten — delete dua kali aman. |