// data/waifu/_template.js
// Template untuk mengisi file franchise. Salin blok waifus ini ke file franchise baru,
// lalu isi sesuai data. Wajib: name, series, age, height, weight, tier, personality.
// keyword opsional; default auto: `${name} ${series} anime`
export const waifus = [
  {
    name: "Miku Nakano",            // required, unik lintas franchise
    series: "Gotoubun no Hanayome", // required
    age: 17, height: "165 cm", weight: "49 kg",   // required
    tier: "Rare",                   // Common|Rare|Epic|Legendary|Mythic
    personality: "dandere",         // wajib, lihat tabel personality di spec
    keyword: "",                    // optional override keyword Pinterest
  },
];
