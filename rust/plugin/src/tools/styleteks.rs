//! `plugins/tools/styleteks.js` — ubah teks biasa jadi berbagai gaya font.
//!
//! Hasil dikirim sebagai kartu native-flow: body berisi teks hasil gaya, dan
//! tombol `cta_copy` dibangun dengan `Tombol::salin` (params `display_text`,
//! `id`, `copy_code` — persis bentuk yang dibaca WA). Kartu tidak mengutip
//! pesan pemicu (`Kartu::quote` bawaan `false`), jadi `quoted: m` di JS belum
//! ikut.
//! Satu `Reply` juga hanya membawa satu reaksi; reaksi penutup `✅`/`❌` yang
//! dipertahankan, reaksi pembuka `🕕` dilewatkan.

use oktz_plugin_api::{Ctx, Reply, Tombol, plugin};

const CONTOH: &str = "Zann";
const GAYA_EKSTRA: [&str; 3] = ["Strike Through", "Underline", "Slash Through"];

/// Peta `FONTS` dari JS: urutan gaya dan pasangan huruf sama persis.
static FONTS: [(&str, &[(&str, &str)]); 14] = [
    ("Cursive", &[
        ("a", "𝓪"),
        ("b", "𝓫"),
        ("c", "𝓬"),
        ("d", "𝓭"),
        ("e", "𝓮"),
        ("f", "𝓯"),
        ("g", "𝓰"),
        ("h", "𝓱"),
        ("i", "𝓲"),
        ("j", "𝓳"),
        ("k", "𝓴"),
        ("l", "𝓵"),
        ("m", "𝓶"),
        ("n", "𝓷"),
        ("o", "𝓸"),
        ("p", "𝓹"),
        ("q", "𝓺"),
        ("r", "𝓻"),
        ("s", "𝓼"),
        ("t", "𝓽"),
        ("u", "𝓾"),
        ("v", "𝓿"),
        ("w", "𝔀"),
        ("x", "𝔁"),
        ("y", "𝔂"),
        ("z", "𝔃"),
        ("A", "𝓐"),
        ("B", "𝓑"),
        ("C", "𝓒"),
        ("D", ""),
        ("E", "𝓔"),
        ("F", "𝓕"),
        ("G", "𝓖"),
        ("H", "𝓗"),
        ("I", "𝓘"),
        ("J", "𝓙"),
        ("K", "𝓚"),
        ("L", "𝓛"),
        ("M", "𝓜"),
        ("N", "𝓝"),
        ("O", "𝓞"),
        ("P", "𝓟"),
        ("Q", "𝓠"),
        ("R", "𝓡"),
        ("S", "𝓢"),
        ("T", "𝓣"),
        ("U", "𝓤"),
        ("V", "𝓥"),
        ("W", "𝓦"),
        ("X", "𝓧"),
        ("Y", "𝓨"),
        ("Z", "𝓩"),
    ]),
    ("Double Struck", &[
        ("a", "𝕒"),
        ("b", "𝕓"),
        ("c", "𝕔"),
        ("d", "𝕕"),
        ("e", "𝕖"),
        ("f", "𝕗"),
        ("g", "𝕘"),
        ("h", "𝕙"),
        ("i", "𝕚"),
        ("j", "𝕛"),
        ("k", "𝕜"),
        ("l", "𝕝"),
        ("m", "𝕞"),
        ("n", "𝕟"),
        ("o", "𝕠"),
        ("p", "𝕡"),
        ("q", "𝕢"),
        ("r", "𝕣"),
        ("s", "𝕤"),
        ("t", "𝕥"),
        ("u", "𝕦"),
        ("v", "𝕧"),
        ("w", "𝕨"),
        ("x", "𝕩"),
        ("y", "𝕪"),
        ("z", "𝕫"),
        ("A", "𝔸"),
        ("B", "𝔹"),
        ("C", "ℂ"),
        ("D", "𝔻"),
        ("E", "𝔼"),
        ("F", "𝔽"),
        ("G", "𝔾"),
        ("H", "ℍ"),
        ("I", "𝕀"),
        ("J", "𝕁"),
        ("K", "𝕂"),
        ("L", "𝕃"),
        ("M", "𝕄"),
        ("N", "ℕ"),
        ("O", "𝕆"),
        ("P", "ℙ"),
        ("Q", "ℚ"),
        ("R", "ℝ"),
        ("S", "𝕊"),
        ("T", "𝕋"),
        ("U", "𝕌"),
        ("V", "𝕍"),
        ("W", "𝕎"),
        ("X", "𝕏"),
        ("Y", "𝕐"),
        ("Z", "ℤ"),
    ]),
    ("Old English", &[
        ("a", "𝔞"),
        ("b", "𝔟"),
        ("c", "𝔠"),
        ("d", "𝔡"),
        ("e", "𝔢"),
        ("f", "𝔣"),
        ("g", "𝔤"),
        ("h", "𝔥"),
        ("i", "𝔦"),
        ("j", "𝔧"),
        ("k", "𝔨"),
        ("l", "𝔩"),
        ("m", "𝔪"),
        ("n", "𝔫"),
        ("o", "𝔬"),
        ("p", "𝔭"),
        ("q", "𝔮"),
        ("r", "𝔯"),
        ("s", "𝔰"),
        ("t", "𝔱"),
        ("u", "𝔲"),
        ("v", "𝔳"),
        ("w", "𝔴"),
        ("x", "𝔵"),
        ("y", "𝔶"),
        ("z", "𝔷"),
        ("A", "𝔄"),
        ("B", "𝔅"),
        ("C", "ℭ"),
        ("D", "𝔇"),
        ("E", "𝔈"),
        ("F", "𝔉"),
        ("G", "𝔊"),
        ("H", "ℌ"),
        ("I", "ℑ"),
        ("J", "𝔍"),
        ("K", "𝔎"),
        ("L", "𝔏"),
        ("M", "𝔐"),
        ("N", "𝔑"),
        ("O", "𝔒"),
        ("P", "𝔓"),
        ("Q", "𝔔"),
        ("R", "ℜ"),
        ("S", "𝔖"),
        ("T", "𝔗"),
        ("U", "𝔘"),
        ("V", "𝔙"),
        ("W", "𝔚"),
        ("X", "𝔛"),
        ("Y", "𝔜"),
        ("Z", "ℨ"),
    ]),
    ("Monospace", &[
        ("a", "𝚊"),
        ("b", "𝚋"),
        ("c", "𝚌"),
        ("d", "𝚍"),
        ("e", "𝚎"),
        ("f", "𝚏"),
        ("g", "𝚐"),
        ("h", "𝚑"),
        ("i", "𝚒"),
        ("j", "𝚓"),
        ("k", "𝚔"),
        ("l", "𝚕"),
        ("m", "𝚖"),
        ("n", "𝚗"),
        ("o", "𝚘"),
        ("p", "𝚙"),
        ("q", "𝚚"),
        ("r", "𝚛"),
        ("s", "𝚜"),
        ("t", "𝚝"),
        ("u", "𝚞"),
        ("v", "𝚟"),
        ("w", "𝚠"),
        ("x", "𝚡"),
        ("y", "𝚢"),
        ("z", "𝚣"),
        ("A", "𝙰"),
        ("B", "𝙱"),
        ("C", "𝙲"),
        ("D", "𝙳"),
        ("E", "𝙴"),
        ("F", "𝙵"),
        ("G", "𝙶"),
        ("H", "𝙷"),
        ("I", "𝙸"),
        ("J", "𝙹"),
        ("K", "𝙺"),
        ("L", "𝙻"),
        ("M", "𝙼"),
        ("N", "𝙽"),
        ("O", "𝙾"),
        ("P", "𝙿"),
        ("Q", "𝚀"),
        ("R", "𝚁"),
        ("S", "𝚂"),
        ("T", "𝚃"),
        ("U", "𝚄"),
        ("V", "𝚅"),
        ("W", "𝚆"),
        ("X", "𝚇"),
        ("Y", "𝚈"),
        ("Z", "𝚉"),
    ]),
    ("Bold", &[
        ("a", "𝐚"),
        ("b", "𝐛"),
        ("c", "𝐜"),
        ("d", "𝐝"),
        ("e", "𝐞"),
        ("f", "𝐟"),
        ("g", "𝐠"),
        ("h", "𝐡"),
        ("i", "𝐢"),
        ("j", "𝐣"),
        ("k", "𝐤"),
        ("l", "𝐥"),
        ("m", "𝐦"),
        ("n", "𝐧"),
        ("o", "𝐨"),
        ("p", "𝐩"),
        ("q", "𝐪"),
        ("r", "𝐫"),
        ("s", "𝐬"),
        ("t", "𝐭"),
        ("u", "𝐮"),
        ("v", "𝐯"),
        ("w", "𝐰"),
        ("x", "𝐱"),
        ("y", "𝐲"),
        ("z", "𝐳"),
        ("A", "𝐀"),
        ("B", "𝐁"),
        ("C", "𝐂"),
        ("D", "𝐃"),
        ("E", "𝐄"),
        ("F", "𝐅"),
        ("G", "𝐆"),
        ("H", "𝐇"),
        ("I", "𝐈"),
        ("J", "𝐉"),
        ("K", "𝐊"),
        ("L", "𝐋"),
        ("M", "𝐌"),
        ("N", "𝐍"),
        ("O", "𝐎"),
        ("P", "𝐏"),
        ("Q", "𝐐"),
        ("R", "𝐑"),
        ("S", "𝐒"),
        ("T", "𝐓"),
        ("U", "𝐔"),
        ("V", "𝐕"),
        ("W", "𝐖"),
        ("X", "𝐗"),
        ("Y", "𝐘"),
        ("Z", "𝐙"),
    ]),
    ("Bold Sans", &[
        ("a", "𝗮"),
        ("b", "𝗯"),
        ("c", "𝗰"),
        ("d", "𝗱"),
        ("e", "𝗲"),
        ("f", "𝗳"),
        ("g", "𝗴"),
        ("h", "𝗵"),
        ("i", "𝗶"),
        ("j", "𝗷"),
        ("k", "𝗸"),
        ("l", "𝗹"),
        ("m", "𝗺"),
        ("n", "𝗻"),
        ("o", "𝗼"),
        ("p", "𝗽"),
        ("q", "𝗾"),
        ("r", "𝗿"),
        ("s", "𝘀"),
        ("t", "𝘁"),
        ("u", "𝘂"),
        ("v", "𝘃"),
        ("w", "𝘄"),
        ("x", "𝘅"),
        ("y", "𝘆"),
        ("z", "𝘇"),
        ("A", "𝗔"),
        ("B", "𝗕"),
        ("C", "𝗖"),
        ("D", "𝗗"),
        ("E", "𝗘"),
        ("F", "𝗙"),
        ("G", "𝗚"),
        ("H", "𝗛"),
        ("I", "𝗜"),
        ("J", "𝗝"),
        ("K", "𝗞"),
        ("L", "𝗟"),
        ("M", "𝗠"),
        ("N", "𝗡"),
        ("O", "𝗢"),
        ("P", "𝗣"),
        ("Q", "𝗤"),
        ("R", "𝗥"),
        ("S", "𝗦"),
        ("T", "𝗧"),
        ("U", "𝗨"),
        ("V", "𝗩"),
        ("W", "𝗪"),
        ("X", "𝗫"),
        ("Y", "𝗬"),
        ("Z", "𝗭"),
    ]),
    ("Italic", &[
        ("a", "𝘢"),
        ("b", "𝘣"),
        ("c", "𝘤"),
        ("d", "𝘥"),
        ("e", "𝘦"),
        ("f", "𝘧"),
        ("g", "𝘨"),
        ("h", "𝘩"),
        ("i", "𝘪"),
        ("j", "𝘫"),
        ("k", "𝘬"),
        ("l", "𝘭"),
        ("m", "𝘮"),
        ("n", "𝘯"),
        ("o", "𝘰"),
        ("p", "𝘱"),
        ("q", "𝘲"),
        ("r", "𝘳"),
        ("s", "𝘴"),
        ("t", "𝘵"),
        ("u", "𝘶"),
        ("v", "𝘷"),
        ("w", "𝘸"),
        ("x", "𝘹"),
        ("y", "𝘺"),
        ("z", "𝘻"),
        ("A", "𝘈"),
        ("B", "𝘉"),
        ("C", "𝘊"),
        ("D", "𝘋"),
        ("E", "𝘌"),
        ("F", "𝘍"),
        ("G", "𝘎"),
        ("H", "𝘏"),
        ("I", "𝘐"),
        ("J", "𝘑"),
        ("K", "𝘒"),
        ("L", "𝘓"),
        ("M", "𝘔"),
        ("N", "𝘕"),
        ("O", "𝘖"),
        ("P", "𝘗"),
        ("Q", "𝘘"),
        ("R", "𝘙"),
        ("S", "𝘚"),
        ("T", "𝘛"),
        ("U", "𝘜"),
        ("V", "𝘝"),
        ("W", "𝘞"),
        ("X", "𝘟"),
        ("Y", "𝘠"),
        ("Z", "𝘡"),
    ]),
    ("Wide", &[
        ("a", "\u{ff41}"),
        ("b", "\u{ff42}"),
        ("c", "\u{ff43}"),
        ("d", "\u{ff44}"),
        ("e", "\u{ff45}"),
        ("f", "\u{ff46}"),
        ("g", "\u{ff47}"),
        ("h", "\u{ff48}"),
        ("i", "\u{ff49}"),
        ("j", "\u{ff4a}"),
        ("k", "\u{ff4b}"),
        ("l", "\u{ff4c}"),
        ("m", "\u{ff4d}"),
        ("n", "\u{ff4e}"),
        ("o", "\u{ff4f}"),
        ("p", "\u{ff50}"),
        ("q", "\u{ff51}"),
        ("r", "\u{ff52}"),
        ("s", "\u{ff53}"),
        ("t", "\u{ff54}"),
        ("u", "\u{ff55}"),
        ("v", "\u{ff56}"),
        ("w", "\u{ff57}"),
        ("x", "\u{ff58}"),
        ("y", "\u{ff59}"),
        ("z", "\u{ff5a}"),
        ("A", "\u{ff21}"),
        ("B", "\u{ff22}"),
        ("C", "\u{ff23}"),
        ("D", "\u{ff24}"),
        ("E", "\u{ff25}"),
        ("F", "\u{ff26}"),
        ("G", "\u{ff27}"),
        ("H", "\u{ff28}"),
        ("I", "\u{ff29}"),
        ("J", "\u{ff2a}"),
        ("K", "\u{ff2b}"),
        ("L", "\u{ff2c}"),
        ("M", "\u{ff2d}"),
        ("N", "\u{ff2e}"),
        ("O", "\u{ff2f}"),
        ("P", "\u{ff30}"),
        ("Q", "\u{ff31}"),
        ("R", "\u{ff32}"),
        ("S", "\u{ff33}"),
        ("T", "\u{ff34}"),
        ("U", "\u{ff35}"),
        ("V", "\u{ff36}"),
        ("W", "\u{ff37}"),
        ("X", "\u{ff38}"),
        ("Y", "\u{ff39}"),
        ("Z", "\u{ff3a}"),
    ]),
    ("Squares", &[
        ("a", "🄰"),
        ("b", "🄱"),
        ("c", "🄲"),
        ("d", "🄳"),
        ("e", "🄴"),
        ("f", "🄵"),
        ("g", "🄶"),
        ("h", "🄷"),
        ("i", "🄸"),
        ("j", "🄹"),
        ("k", "🄺"),
        ("l", "🄻"),
        ("m", "🄼"),
        ("n", "🄽"),
        ("o", "🄾"),
        ("p", "🄿"),
        ("q", "🅀"),
        ("r", "🅁"),
        ("s", "🅂"),
        ("t", "🅃"),
        ("u", "🅄"),
        ("v", "🅅"),
        ("w", "🅆"),
        ("x", "🅇"),
        ("y", "🅈"),
        ("z", "🅉"),
        ("A", "🄰"),
        ("B", "🄱"),
        ("C", "🄲"),
        ("D", "🄳"),
        ("E", "🄴"),
        ("F", "🄵"),
        ("G", "🄶"),
        ("H", "🄷"),
        ("I", "🄸"),
        ("J", "🄹"),
        ("K", "🄺"),
        ("L", "🄻"),
        ("M", "🄼"),
        ("N", "🄽"),
        ("O", "🄾"),
        ("P", "🄿"),
        ("Q", "🅀"),
        ("R", "🅁"),
        ("S", "🅂"),
        ("T", "🅃"),
        ("U", "🅄"),
        ("V", "🅅"),
        ("W", "🅆"),
        ("X", "🅇"),
        ("Y", "🅈"),
        ("Z", "🅉"),
    ]),
    ("Bubbles", &[
        ("a", "ⓐ"),
        ("b", "ⓑ"),
        ("c", "ⓒ"),
        ("d", "ⓓ"),
        ("e", "ⓔ"),
        ("f", "ⓕ"),
        ("g", "ⓖ"),
        ("h", "ⓗ"),
        ("i", "ⓘ"),
        ("j", "ⓙ"),
        ("k", "ⓚ"),
        ("l", "ⓛ"),
        ("m", "ⓜ"),
        ("n", "ⓝ"),
        ("o", "ⓞ"),
        ("p", "ⓟ"),
        ("q", "ⓠ"),
        ("r", "ⓡ"),
        ("s", "ⓢ"),
        ("t", "ⓣ"),
        ("u", "ⓤ"),
        ("v", "ⓥ"),
        ("w", "ⓦ"),
        ("x", "ⓧ"),
        ("y", "ⓨ"),
        ("z", "ⓩ"),
        ("A", "Ⓐ"),
        ("B", "Ⓑ"),
        ("C", "Ⓒ"),
        ("D", "Ⓓ"),
        ("E", "Ⓔ"),
        ("F", "Ⓕ"),
        ("G", "Ⓖ"),
        ("H", "Ⓗ"),
        ("I", "Ⓘ"),
        ("J", "Ⓙ"),
        ("K", "Ⓚ"),
        ("L", "Ⓛ"),
        ("M", "Ⓜ"),
        ("N", "Ⓝ"),
        ("O", "Ⓞ"),
        ("P", "Ⓟ"),
        ("Q", "Ⓠ"),
        ("R", "Ⓡ"),
        ("S", "Ⓢ"),
        ("T", "Ⓣ"),
        ("U", "Ⓤ"),
        ("V", "Ⓥ"),
        ("W", "Ⓦ"),
        ("X", "Ⓧ"),
        ("Y", "Ⓨ"),
        ("Z", "Ⓩ"),
    ]),
    ("Future Alien", &[
        ("a", "ᗩ"),
        ("b", "ᗷ"),
        ("c", "ᑢ"),
        ("d", "ᕲ"),
        ("e", "ᘿ"),
        ("f", "ᖴ"),
        ("g", "ᘜ"),
        ("h", "ᕼ"),
        ("i", "ᓰ"),
        ("j", "ᒚ"),
        ("k", "ᖽᐸ"),
        ("l", "ᒪ"),
        ("m", "ᘻ"),
        ("n", "ᘉ"),
        ("o", "ᓍ"),
        ("p", "ᕵ"),
        ("q", "ᕴ"),
        ("r", "ᖇ"),
        ("s", "S"),
        ("t", "ᖶ"),
        ("u", "ᑘ"),
        ("v", "ᐺ"),
        ("w", "ᘺ"),
        ("x", "᙭"),
        ("y", "ᖻ"),
        ("z", "ᗱ"),
        ("A", "ᗩ"),
        ("B", "ᗷ"),
        ("C", "ᑢ"),
        ("D", "ᕲ"),
        ("E", "ᘿ"),
        ("F", "ᖴ"),
        ("G", "ᘜ"),
        ("H", "ᕼ"),
        ("I", "ᓰ"),
        ("J", "ᒚ"),
        ("K", "ᖽᐸ"),
        ("L", "ᒪ"),
        ("M", "ᘻ"),
        ("N", "ᘉ"),
        ("O", "ᓍ"),
        ("P", "ᕵ"),
        ("Q", "ᕴ"),
        ("R", "ᖇ"),
        ("S", "S"),
        ("T", "ᖶ"),
        ("U", "ᑘ"),
        ("V", "ᐺ"),
        ("W", "ᘺ"),
        ("X", "᙭"),
        ("Y", "ᖻ"),
        ("Z", "ᗱ"),
    ]),
    ("Subscript", &[
        ("a", "ₐ"),
        ("b", "b"),
        ("c", "c"),
        ("d", "d"),
        ("e", "ₑ"),
        ("f", "f"),
        ("g", "g"),
        ("h", "ₕ"),
        ("i", "ᵢ"),
        ("j", "ⱼ"),
        ("k", "ₖ"),
        ("l", "ₗ"),
        ("m", "ₘ"),
        ("n", "ₙ"),
        ("o", "ₒ"),
        ("p", "ₚ"),
        ("q", "q"),
        ("r", "ᵣ"),
        ("s", "ₛ"),
        ("t", "ₜ"),
        ("u", "ᵤ"),
        ("v", "ᵥ"),
        ("w", "w"),
        ("x", "ₓ"),
        ("y", "y"),
        ("z", "z"),
        ("A", "ₐ"),
        ("B", "B"),
        ("C", "C"),
        ("D", "D"),
        ("E", "ₑ"),
        ("F", "F"),
        ("G", "G"),
        ("H", "ₕ"),
        ("I", "ᵢ"),
        ("J", "ⱼ"),
        ("K", "ₖ"),
        ("L", "ₗ"),
        ("M", "ₘ"),
        ("N", "ₙ"),
        ("O", "ₒ"),
        ("P", "ₚ"),
        ("Q", "Q"),
        ("R", "ᵣ"),
        ("S", "ₛ"),
        ("T", "ₜ"),
        ("U", "ᵤ"),
        ("V", "ᵥ"),
        ("W", "W"),
        ("X", "ₓ"),
        ("Y", "Y"),
        ("Z", "Z"),
    ]),
    ("Superscript", &[
        ("a", "ᵃ"),
        ("b", "ᵇ"),
        ("c", "ᶜ"),
        ("d", "ᵈ"),
        ("e", "ᵉ"),
        ("f", "ᶠ"),
        ("g", "ᵍ"),
        ("h", "ʰ"),
        ("i", "ⁱ"),
        ("j", "ʲ"),
        ("k", "ᵏ"),
        ("l", "ˡ"),
        ("m", "ᵐ"),
        ("n", "ⁿ"),
        ("o", "ᵒ"),
        ("p", "ᵖ"),
        ("q", "q"),
        ("r", "ʳ"),
        ("s", "ˢ"),
        ("t", "ᵗ"),
        ("u", "ᵘ"),
        ("v", "ᵛ"),
        ("w", "ʷ"),
        ("x", "ˣ"),
        ("y", "ʸ"),
        ("z", "ᶻ"),
        ("A", "ᴬ"),
        ("B", "ᴮ"),
        ("C", "ᶜ"),
        ("D", "ᴰ"),
        ("E", "ᴱ"),
        ("F", "ᶠ"),
        ("G", "ᴳ"),
        ("H", "ᴴ"),
        ("I", "ᴵ"),
        ("J", "ᴶ"),
        ("K", "ᴷ"),
        ("L", "ᴸ"),
        ("M", "ᴹ"),
        ("N", "ᴺ"),
        ("O", "ᴼ"),
        ("P", "ᴾ"),
        ("Q", "Q"),
        ("R", "ᴿ"),
        ("S", "ˢ"),
        ("T", "ᵀ"),
        ("U", "ᵁ"),
        ("V", "ⱽ"),
        ("W", "ᵂ"),
        ("X", "ˣ"),
        ("Y", "ʸ"),
        ("Z", "ᶻ"),
    ]),
    ("Tiny", &[
        ("A", "ᴀ"),
        ("B", "ʙ"),
        ("C", "ᴄ"),
        ("D", "ᴅ"),
        ("E", "ᴇ"),
        ("F", "ꜰ"),
        ("G", "ɢ"),
        ("H", "ʜ"),
        ("I", "ɪ"),
        ("J", "ᴊ"),
        ("K", "ᴋ"),
        ("L", "ʟ"),
        ("M", "ᴍ"),
        ("N", "ɴ"),
        ("O", "ᴏ"),
        ("P", "ᴘ"),
        ("Q", "Q"),
        ("R", "ʀ"),
        ("S", "ꜱ"),
        ("T", "ᴛ"),
        ("U", "ᴜ"),
        ("V", "ᴠ"),
        ("W", "ᴡ"),
        ("X", "x"),
        ("Y", "ʏ"),
        ("Z", "ᴢ"),
        ("a", "ᴀ"),
        ("b", "ʙ"),
        ("c", "ᴄ"),
        ("d", "ᴅ"),
        ("e", "ᴇ"),
        ("f", "ꜰ"),
        ("g", "ɢ"),
        ("h", "ʜ"),
        ("i", "ɪ"),
        ("j", "ᴊ"),
        ("k", "ᴋ"),
        ("l", "ʟ"),
        ("m", "ᴍ"),
        ("n", "ɴ"),
        ("o", "ᴏ"),
        ("p", "ᴘ"),
        ("q", "q"),
        ("r", "ʀ"),
        ("s", "ꜱ"),
        ("t", "ᴛ"),
        ("u", "ᴜ"),
        ("v", "ᴠ"),
        ("w", "ᴡ"),
        ("x", "x"),
        ("y", "ʏ"),
        ("z", "ᴢ"),
    ]),
];

/// `applyStyle(text, map)` — nilai peta kosong jatuh ke karakter asli,
/// sama seperti cek truthy di JS.
fn terapkan(teks: &str, peta: &[(&str, &str)]) -> String {
    let mut hasil = String::with_capacity(teks.len());
    for c in teks.chars() {
        let mut buf = [0u8; 4];
        let kunci = c.encode_utf8(&mut buf);
        match peta.iter().find(|(k, _)| *k == kunci) {
            Some((_, ganti)) if !ganti.is_empty() => hasil.push_str(ganti),
            _ => hasil.push(c),
        }
    }
    hasil
}

/// `strikeThrough`/`underline`/`slashThrough` — sisipkan tanda setelah tiap
/// karakter, lalu satu tanda lagi di akhir.
///
/// `ponytail:` JS memecah per code unit UTF-16 (emoji jadi surrogate tunggal
/// yang hanya bisa dikirim sebagai CESU-8); di sini per `char` supaya hasilnya
/// tetap UTF-8 valid.
fn gores(teks: &str, tanda: char) -> String {
    let mut hasil = String::with_capacity(teks.len() * 2);
    for c in teks.chars() {
        hasil.push(c);
        hasil.push(tanda);
    }
    hasil.push(tanda);
    hasil
}

fn pratinjau(nama: &str) -> String {
    match nama {
        "Strike Through" => gores(CONTOH, '\u{0336}'),
        "Underline" => gores(CONTOH, '\u{0332}'),
        "Slash Through" => gores(CONTOH, '\u{0337}'),
        _ => match FONTS.iter().find(|(n, _)| *n == nama) {
            Some((_, peta)) => terapkan(CONTOH, peta),
            None => String::new(),
        },
    }
}

fn bantuan(prefix: &str) -> String {
    let mut teks = String::from("✨ *FANCY TEXT GENERATOR* ✨\n\n");
    teks.push_str("Ubah teks biasamu menjadi gaya tulisan yang unik dan keren!\n\n");
    teks.push_str("🔮 *DAFTAR GAYA FONT:*\n");
    for (nama, peta) in FONTS.iter() {
        teks.push_str(&format!("🔹 *{nama}* ➔ {}\n", terapkan(CONTOH, peta)));
    }
    for nama in GAYA_EKSTRA {
        teks.push_str(&format!("🔹 *{nama}* ➔ {}\n", pratinjau(nama)));
    }
    teks.push_str("\n*CARA PENGGUNAAN:*\n");
    teks.push_str(&format!("- Ketik `{prefix}styleteks <Nama Font> | <Teks>`\n"));
    teks.push_str(&format!(
        "- Contoh: `{prefix}styleteks Cursive | Ini tulisan keren`\n\n"
    ));
    teks.push_str("_Pastikan mengetikkan nama font sesuai daftar di atas (boleh huruf kecil/besar) dan gunakan garis lurus (|) sebagai pemisah!_");
    teks
}

/// Cari gaya berdasarkan nama (sudah lowercase), termasuk alias garis.
fn cari_gaya(nama: &str, teks: &str) -> Option<String> {
    for (n, peta) in FONTS.iter() {
        if n.to_lowercase() == nama {
            return Some(terapkan(teks, peta));
        }
    }
    match nama {
        "strike through" | "strike" => Some(gores(teks, '\u{0336}')),
        "underline" => Some(gores(teks, '\u{0332}')),
        "slash through" | "slash" => Some(gores(teks, '\u{0337}')),
        _ => None,
    }
}

#[plugin(
    name = "styleteks",
    alias = ["fancytext", "font", "gaya", "tulisan", "fontstyle"],
    category = "tools",
    description = "Mengubah teks biasa menjadi berbagai gaya tulisan keren (fancy text).",
    usage = ".styleteks <teks>",
    example = ".styleteks Halo Dunia",
    cooldown = 5,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn styleteks(ctx: Ctx) -> Reply {
    if ctx.text.is_empty() || !ctx.text.contains('|') {
        return Reply::text(bantuan(&ctx.prefix));
    }

    let (font, isi) = ctx
        .text
        .split_once('|')
        .expect("sudah dicek mengandung '|'");
    let font_terpilih = font.trim().to_lowercase();
    let teks_asli = isi.trim();

    if teks_asli.is_empty() {
        return Reply::text(format!(
            "❌ *TEKS KOSONG*\n\nJangan lupa masukkan teks yang ingin diubah setelah garis lurus.\nContoh: `{}styleteks Cursive | Halo Dunia`",
            ctx.prefix
        ))
        .with_reaction("❌");
    }

    let Some(hasil) = cari_gaya(&font_terpilih, teks_asli) else {
        return Reply::text(format!(
            "❌ *FONT TIDAK DITEMUKAN*\n\nGaya font *{}* tidak terdaftar.\nSilakan ketik `{}styleteks` untuk melihat daftar font yang tersedia.",
            font.trim(),
            ctx.prefix
        ))
        .with_reaction("❌");
    };

    // `sock.sendMessage(m.chat, { text: hasil, interactiveButtons: [{cta_copy}] })`
    // — teks masuk body kartu, tombol salin ikut.
    Reply::interactive(hasil.clone())
        .with_button(Tombol::salin("📋 Copy Hasil Teks", "copy", hasil))
        .with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    fn jalankan(teks: &str) -> Reply {
        styleteks(Ctx {
            prefix: ".".into(),
            command: "styleteks".into(),
            text: teks.into(),
            args: teks.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        })
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    fn ganti(font: &str, huruf: &str) -> &'static str {
        FONTS
            .iter()
            .find(|(n, _)| *n == font)
            .unwrap()
            .1
            .iter()
            .find(|(k, _)| *k == huruf)
            .unwrap()
            .1
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = find("styleteks").unwrap();
        assert_eq!(p.meta.name, "styleteks");
        assert_eq!(
            p.meta.alias,
            ["fancytext", "font", "gaya", "tulisan", "fontstyle"]
        );
        assert_eq!(p.meta.category, "tools");
        assert_eq!(
            p.meta.description,
            "Mengubah teks biasa menjadi berbagai gaya tulisan keren (fancy text)."
        );
        assert_eq!(p.meta.usage, ".styleteks <teks>");
        assert_eq!(p.meta.example, ".styleteks Halo Dunia");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn menu_berisi_semua_gaya_dan_pratinjau() {
        let teks = bantuan("!");
        assert!(teks.starts_with("✨ *FANCY TEXT GENERATOR* ✨\n\n"));
        assert_eq!(teks.matches("🔹 *").count(), 17);
        for baris in [
            "🔹 *Cursive* ➔ 𝓩𝓪𝓷𝓷\n",
            "🔹 *Double Struck* ➔ ℤ𝕒𝕟𝕟\n",
            "🔹 *Tiny* ➔ ᴢᴀɴɴ\n",
            // JS menyisipkan tanda di antara huruf DAN satu di akhir.
            "🔹 *Strike Through* ➔ Z\u{0336}a\u{0336}n\u{0336}n\u{0336}\u{0336}\n",
            "🔹 *Underline* ➔ Z\u{0332}a\u{0332}n\u{0332}n\u{0332}\u{0332}\n",
            "🔹 *Slash Through* ➔ Z\u{0337}a\u{0337}n\u{0337}n\u{0337}\u{0337}\n",
        ] {
            assert!(teks.contains(baris), "baris hilang: {baris:?}");
        }
        assert!(teks.ends_with(
            "_Pastikan mengetikkan nama font sesuai daftar di atas (boleh huruf kecil/besar) dan gunakan garis lurus (|) sebagai pemisah!_"
        ));
    }

    #[test]
    fn tanpa_pipa_atau_kosong_memberi_bantuan_tanpa_reaksi() {
        for teks in ["", "Cursive", "   "] {
            let r = jalankan(teks);
            assert_eq!(r.text_body().unwrap(), bantuan("."));
            assert!(reaksi(&r).is_none(), "input {teks:?} tidak bereaksi");
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn cursive_memakai_huruf_asli_untuk_peta_kosong() {
        let r = jalankan("Cursive | Halo Dunia");
        // JS: `map["D"]` kosong → falsy → huruf asli "D" tetap dipakai.
        assert_eq!(r.text_body().unwrap(), "𝓗𝓪𝓵𝓸 D𝓾𝓷𝓲𝓪");
        assert_eq!(reaksi(&r), Some("✅"));
        assert!(matches!(
            r.without_mutations().tanpa_reaksi(),
            Reply::Interactive { .. }
        ));
        assert_eq!(
            r.mutations(),
            [],
            "hasil sukses tidak menyimpan apa pun"
        );
    }

    #[test]
    fn nama_font_tidak_peka_huruf_dan_pipa_ikut_di_teks() {
        let r = jalankan("  CuRsIvE  |  halo | dunia  ");
        assert_eq!(r.text_body().unwrap(), "𝓱𝓪𝓵𝓸 | 𝓭𝓾𝓷𝓲𝓪");
    }

    #[test]
    fn alias_garis_strike_underline_slash() {
        // JS `text.split("").join(tanda) + tanda` → tanda di antara huruf DAN
        // satu ekstra di akhir.
        assert_eq!(
            jalankan("strike | Halo").text_body().unwrap(),
            "H\u{0336}a\u{0336}l\u{0336}o\u{0336}\u{0336}"
        );
        assert_eq!(
            jalankan("strike through | Halo").text_body().unwrap(),
            "H\u{0336}a\u{0336}l\u{0336}o\u{0336}\u{0336}"
        );
        assert_eq!(
            jalankan("underline | Halo").text_body().unwrap(),
            "H\u{0332}a\u{0332}l\u{0332}o\u{0332}\u{0332}"
        );
        assert_eq!(
            jalankan("slash | Halo").text_body().unwrap(),
            "H\u{0337}a\u{0337}l\u{0337}o\u{0337}\u{0337}"
        );
        assert_eq!(
            jalankan("slash through | Halo").text_body().unwrap(),
            "H\u{0337}a\u{0337}l\u{0337}o\u{0337}\u{0337}"
        );
    }

    #[test]
    fn teks_kosong_setelah_pipa_ditolak() {
        let r = jalankan("Cursive |   ");
        assert_eq!(
            r.text_body().unwrap(),
            "❌ *TEKS KOSONG*\n\nJangan lupa masukkan teks yang ingin diubah setelah garis lurus.\nContoh: `.styleteks Cursive | Halo Dunia`"
        );
        assert_eq!(reaksi(&r), Some("❌"));
    }

    #[test]
    fn font_tidak_dikenal_memakai_nama_asli_yang_di_trim() {
        let r = jalankan("  Nope  | Halo");
        assert_eq!(
            r.text_body().unwrap(),
            "❌ *FONT TIDAK DITEMUKAN*\n\nGaya font *Nope* tidak terdaftar.\nSilakan ketik `.styleteks` untuk melihat daftar font yang tersedia."
        );
        assert_eq!(reaksi(&r), Some("❌"));
    }

    #[test]
    fn peta_font_lengkap_dan_nilai_khusus_sesuai_js() {
        assert_eq!(FONTS.len(), 14);
        let nama: Vec<&str> = FONTS.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            nama,
            [
                "Cursive",
                "Double Struck",
                "Old English",
                "Monospace",
                "Bold",
                "Bold Sans",
                "Italic",
                "Wide",
                "Squares",
                "Bubbles",
                "Future Alien",
                "Subscript",
                "Superscript",
                "Tiny"
            ]
        );
        for (nama, peta) in FONTS.iter() {
            assert_eq!(peta.len(), 52, "peta {nama} harus 52 huruf");
        }
        assert_eq!(ganti("Cursive", "D"), "");
        assert_eq!(ganti("Future Alien", "k"), "ᖽᐸ");
        assert_eq!(ganti("Tiny", "X"), "x");
        assert_eq!(ganti("Bold", "Z"), "𝐙");
    }

    #[test]
    fn hasil_sukses_membawa_tombol_salin_cta_copy() {
        let tanpa = jalankan("Cursive | Halo");
        let tanpa = tanpa.without_mutations();
        let Reply::Interactive { kartu } = tanpa.tanpa_reaksi() else {
            panic!("harus kartu interaktif");
        };
        assert_eq!(kartu.body, "𝓗𝓪𝓵𝓸");
        assert_eq!(kartu.footer, None::<String>);
        assert_eq!(kartu.buttons.len(), 1);
        assert_eq!(kartu.buttons[0].aksi.as_str(), "cta_copy");
        assert_eq!(kartu.buttons[0].params["display_text"], "📋 Copy Hasil Teks");
        assert_eq!(kartu.buttons[0].params["id"], "copy");
        assert_eq!(kartu.buttons[0].params["copy_code"], "𝓗𝓪𝓵𝓸");
    }

    #[test]
    fn karakter_tak_dikenal_dibiarkan() {
        let r = jalankan("Bold | 123 😀");
        assert_eq!(r.text_body().unwrap(), "123 😀");
    }
}
