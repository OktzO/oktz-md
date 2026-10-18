//! `plugins/tools/esmtocjs.js` — ubah `import`/`export` ESM jadi CommonJS.
//!
//! `convertEsmToCjs` di JS adalah 12 `String.replace` berurutan yang juga
//! menulis daftar export ke `exportedItems`. Regex tidak tersedia di crate ini,
//! jadi tiap aturan ditulis ulang sebagai pemindai byte dan efek sampingnya
//! (pengumpulan nama export) dikumpulkan lewat closure.
//!
//! Satu bug JS menentukan hasil dan WAJIB dipertahankan: aturan
//! `export default (\w+)` (`esmtocjs.js:51`) jalan lebih dulu dari aturan
//! `export default (function|class|async function) …` (`:57`) dan `\w+` bisa
//! makan kata `function`. Jadi `export default function foo() {}` berakhir
//! sebagai `foo() {}` + `module.exports = function`.
//!
//! `ponytail:` `sock.sendCodeBlock` tidak ada di API port, jadi hasil konversi
//! dikirim sebagai teks biasa. Cabang `catch` JS tidak diport — konversi string
//! di Rust tidak bisa gagal di tengah jalan.

use oktz_plugin_api::{Ctx, Reply, plugin};

// ---------- primitif pemindai (pengganti regex) ----------

fn next_boundary(s: &str, i: usize) -> usize {
    match s[i..].chars().next() {
        Some(c) => i + c.len_utf8(),
        None => i + 1,
    }
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `s[i..].starts_with(..)` yang aman untuk `i > len` — semua aturan melompat
/// `i + 6` (`import`/`export`) tanpa tahu sisa panjangnya.
fn has(s: &str, i: usize, lit: &str) -> bool {
    s.get(i..).is_some_and(|t| t.starts_with(lit))
}

/// `\w+` — JS `\w` = `[A-Za-z0-9_]`, tanpa `$`.
fn ident_at(s: &str, i: usize) -> Option<usize> {
    let mut j = i.min(s.len());
    while let Some(c) = s[j..].chars().next() {
        if !is_word(c) {
            break;
        }
        j += c.len_utf8();
    }
    (j > i).then_some(j)
}

fn ws_fwd(s: &str, mut i: usize) -> usize {
    i = i.min(s.len());
    while let Some(c) = s[i..].chars().next() {
        if !c.is_whitespace() {
            break;
        }
        i += c.len_utf8();
    }
    i
}

fn semi(s: &str, i: usize) -> usize {
    if s[i..].starts_with(';') { i + 1 } else { i }
}

/// `\s*=` setelah `i`; posisi tepat SETELAH `=`. Spasi setelah `=` bukan bagian
/// regex JS, jadi harus tetap ikut di hasil.
fn eq_after(s: &str, i: usize) -> Option<usize> {
    let j = ws_fwd(s, i);
    has(s, j, "=").then_some(j + 1)
}

/// `from\s+['"]([^'"]+)['"]\s*;?` — path tanpa kutip, posisi setelah `;?`.
fn from_path(s: &str, i: usize) -> Option<(String, usize)> {
    let j = ws_fwd(s, i);
    if !has(s, j, "from") {
        return None;
    }
    let k = ws_fwd(s, j + 4);
    let q = s[k..].chars().next()?;
    if q != '\'' && q != '"' {
        return None;
    }
    let start = k + 1;
    let end = start + s[start..].find(q)?;
    let path = s[start..end].to_string();
    if path.is_empty() {
        return None; // `[^'"]+`
    }
    let m = ws_fwd(s, end + q.len_utf8());
    Some((path, semi(s, m)))
}

/// `\s+['"]([^'"]+)['"]\s*;?` — path tanpa kutip, posisi sebelum `;?`.
fn bare_path(s: &str, i: usize) -> Option<(String, usize)> {
    let k = ws_fwd(s, i);
    let q = s[k..].chars().next()?;
    if q != '\'' && q != '"' {
        return None;
    }
    let start = k + 1;
    let end = start + s[start..].find(q)?;
    let path = s[start..end].to_string();
    if path.is_empty() {
        return None;
    }
    let m = ws_fwd(s, end + q.len_utf8());
    Some((path, semi(s, m)))
}

/// Cocok pertama mulai `from`; kembalikan `(start, end, pengganti)`.
fn scan<F>(s: &str, from: usize, mut at: F) -> Option<(usize, usize, String)>
where
    F: FnMut(&str, usize) -> Option<(usize, String)>,
{
    let mut i = from;
    while i <= s.len() {
        if let Some((end, rep)) = at(s, i) {
            return Some((i, end, rep));
        }
        if i == s.len() {
            return None;
        }
        i = next_boundary(s, i);
    }
    None
}

fn replace_pass<F>(src: &str, mut at: F) -> String
where
    F: FnMut(&str, usize) -> Option<(usize, String)>,
{
    let mut out = String::with_capacity(src.len());
    let mut copied = 0usize;
    let mut from = 0usize;
    while let Some((start, end, rep)) = scan(src, from, &mut at) {
        out.push_str(&src[copied..start]);
        out.push_str(&rep);
        copied = end;
        from = if end > start {
            end
        } else {
            next_boundary(src, start)
        };
    }
    out.push_str(&src[copied..]);
    out
}

// ---------- aturan 1..13, urutannya sama dengan JS ----------

/// Satu nama yang diekspor: `a` atau `b as c` pada `export { … }`.
pub(crate) struct Exported {
    pub(crate) name: String,
    pub(crate) alias: Option<String>,
}

impl Exported {
    /// `e.alias ? `${e.alias}: ${e.name}` : e.name`
    fn pair(&self) -> String {
        match &self.alias {
            Some(a) => format!("{a}: {}", self.name),
            None => self.name.clone(),
        }
    }
}

/// `i.trim().split(/\s+as\s+/)` — `parts.length === 2 ? `${parts[0]}: ${parts[1]}`
/// : `parts[0]`. Regex itu memakan spasi di kiri-kanan `as`, jadi hanya bentuk
/// tiga kata `x as y` yang terpecah dua.
fn split_as(item: &str) -> String {
    let p: Vec<&str> = item.split_whitespace().collect();
    if p.len() == 3 && p[1] == "as" {
        format!("{}: {}", p[0], p[2])
    } else {
        p.first().copied().unwrap_or_default().to_string()
    }
}

/// `import\s+(\w+)\s*,\s*\{\s*([^}]+)\s*\}\s*from\s+['"]([^'"]+)['"]\s*;?`
fn e1(s: &str, i: usize) -> Option<(usize, String)> {
    if !has(s, i, "import") {
        return None;
    }
    let j = ws_fwd(s, i + 6);
    if j == i + 6 {
        return None; // `\s+`
    }
    let def_end = ident_at(s, j)?;
    let k = ws_fwd(s, def_end);
    if !has(s, k, ",") {
        return None;
    }
    let b = ws_fwd(s, k + 1);
    if !has(s, b, "{") {
        return None;
    }
    let rel = s[b + 1..].find('}')?;
    if rel == 0 {
        return None; // `([^}]+)`
    }
    let inner = &s[b + 1..b + 1 + rel];
    let (path, end) = from_path(s, b + 2 + rel)?;
    let items: Vec<&str> = inner.split(',').map(str::trim).collect();
    Some((
        end,
        format!(
            "const {} = require('{path}')\nconst {{ {} }} = require('{path}')",
            &s[j..def_end],
            items.join(", ")
        ),
    ))
}

/// `import\s+(\w+)\s+from\s+['"]([^'"]+)['"]\s*;?`
fn e2(s: &str, i: usize) -> Option<(usize, String)> {
    if !has(s, i, "import") {
        return None;
    }
    let j = ws_fwd(s, i + 6);
    if j == i + 6 {
        return None;
    }
    let ke = ident_at(s, j)?;
    let k = ws_fwd(s, ke);
    if k == ke {
        return None; // `\s+` sebelum `from`
    }
    let (path, end) = from_path(s, k)?;
    Some((end, format!("const {} = require('{path}')", &s[j..ke])))
}

/// `import\s*\{\s*([^}]+)\s*\}\s*from\s+['"]([^'"]+)['"]\s*;?`
fn e3(s: &str, i: usize) -> Option<(usize, String)> {
    if !has(s, i, "import") {
        return None;
    }
    let b = ws_fwd(s, i + 6); // `import\s*\{`
    if !has(s, b, "{") {
        return None;
    }
    let rel = s[b + 1..].find('}')?;
    if rel == 0 {
        return None;
    }
    let inner = &s[b + 1..b + 1 + rel];
    let (path, end) = from_path(s, b + 2 + rel)?;
    let items: Vec<String> = inner.split(',').map(split_as).collect();
    Some((
        end,
        format!("const {{ {} }} = require('{path}')", items.join(", ")),
    ))
}

/// `import\s*\*\s*as\s+(\w+)\s+from\s+['"]([^'"]+)['"]\s*;?`
fn e4(s: &str, i: usize) -> Option<(usize, String)> {
    if !has(s, i, "import") {
        return None;
    }
    let mut j = ws_fwd(s, i + 6);
    if !has(s, j, "*") {
        return None;
    }
    j = ws_fwd(s, j + 1);
    if !has(s, j, "as") {
        return None;
    }
    let k = ws_fwd(s, j + 2);
    let ke = ident_at(s, k)?;
    let m = ws_fwd(s, ke);
    if m == ke {
        return None;
    }
    let (path, end) = from_path(s, m)?;
    Some((end, format!("const {} = require('{path}')", &s[k..ke])))
}

/// `import\s+['"]([^'"]+)['"]\s*;?`
fn e5(s: &str, i: usize) -> Option<(usize, String)> {
    if !has(s, i, "import") {
        return None;
    }
    let j = ws_fwd(s, i + 6);
    if j == i + 6 {
        return None; // `\s+`
    }
    let (path, end) = bare_path(s, j)?;
    Some((end, format!("require('{path}')")))
}

/// `export\s+default\s+(\w+)\s*;?` — juga menelan `export default function …`
/// (`\w+` bisa makan `function`), jadi aturan `function|class` di JS mati.
fn e6(s: &str, i: usize) -> Option<(usize, String)> {
    if !has(s, i, "export") {
        return None;
    }
    let j = ws_fwd(s, i + 6); // `export`
    let k = ws_fwd(s, j + 7); // `default`
    if j == i + 6 || !has(s, j, "default") || k == j + 7 {
        return None;
    }
    let n = ws_fwd(s, k);
    let ke = ident_at(s, n)?;
    let end = semi(s, ws_fwd(s, ke));
    CURRENT.with(|c| {
        let mut c = c.borrow_mut();
        c.has_default = true;
        c.default_value = s[n..ke].to_string();
    });
    Some((end, String::new()))
}

/// `export\s+default\s+(\{[\s\S]*?\})\s*;?` — `[\s\S]*?` malas, jadi berhenti di
/// `}` pertama dan tidak bisa nesting.
fn e8(s: &str, i: usize) -> Option<(usize, String)> {
    if !has(s, i, "export") {
        return None;
    }
    let j = ws_fwd(s, i + 6);
    if j == i + 6 || !has(s, j, "default") {
        return None;
    }
    let b = ws_fwd(s, j + 7);
    if b == j + 7 {
        return None; // `\s+` setelah `default`
    }
    if !has(s, b, "{") {
        return None;
    }
    let rel = s[b + 1..].find('}')?;
    let close = b + 1 + rel;
    let end = semi(s, ws_fwd(s, close + 1));
    CURRENT.with(|c| {
        let mut c = c.borrow_mut();
        c.has_default = true;
        c.default_value = s[b..close + 1].to_string();
    });
    Some((end, String::new()))
}

/// `export\s*\{\s*([^}]+)\s*\}\s*;?`
fn e9(s: &str, i: usize, items: &mut Vec<Exported>) -> Option<(usize, String)> {
    if !has(s, i, "export") {
        return None;
    }
    let b = ws_fwd(s, i + 6);
    if !has(s, b, "{") {
        return None;
    }
    let rel = s[b + 1..].find('}')?;
    if rel == 0 {
        return None;
    }
    let inner = &s[b + 1..b + 1 + rel];
    let end = semi(s, ws_fwd(s, b + 2 + rel));
    for item in inner.split(',') {
        let p: Vec<&str> = item.split_whitespace().collect();
        match (p.len(), p.get(1)) {
            (3, Some(&"as")) => items.push(Exported {
                name: p[0].to_string(),
                alias: Some(p[2].to_string()),
            }),
            _ => items.push(Exported {
                name: p.first().copied().unwrap_or_default().to_string(),
                alias: None,
            }),
        }
    }
    Some((end, String::new()))
}

/// `export\s+(const|let|var)\s+(\w+)\s*=`
fn e10(s: &str, i: usize, items: &mut Vec<Exported>) -> Option<(usize, String)> {
    if !has(s, i, "export") {
        return None;
    }
    let j = ws_fwd(s, i + 6);
    if j == i + 6 {
        return None;
    }
    let ty = if has(s, j, "const") {
        "const"
    } else if has(s, j, "let") {
        "let"
    } else if has(s, j, "var") {
        "var"
    } else {
        return None;
    };
    let k = ws_fwd(s, j + ty.len());
    if k == j + ty.len() {
        return None; // `\s+` setelah tipe
    }
    let ke = ident_at(s, k)?;
    let end = eq_after(s, ke)?;
    items.push(Exported {
        name: s[k..ke].to_string(),
        alias: None,
    });
    Some((end, format!("{ty} {} =", &s[k..ke])))
}

/// `export\s+(async\s+)?function\s+(\w+)`
fn e11(s: &str, i: usize, items: &mut Vec<Exported>) -> Option<(usize, String)> {
    if !has(s, i, "export") {
        return None;
    }
    let j = ws_fwd(s, i + 6);
    if j == i + 6 {
        return None; // `\s+`
    }
    let (aw, k) = if has(s, j, "async") {
        let m = ws_fwd(s, j + 5);
        if m > j + 5 {
            ("async ", m)
        } else {
            ("", j)
        }
    } else {
        ("", j)
    };
    if !has(s, k, "function") {
        return None;
    }
    let n = ws_fwd(s, k + 8);
    let ke = ident_at(s, n)?;
    items.push(Exported {
        name: s[n..ke].to_string(),
        alias: None,
    });
    Some((ke, format!("{aw}function {}", &s[n..ke])))
}

/// `export\s+class\s+(\w+)`
fn e12(s: &str, i: usize, items: &mut Vec<Exported>) -> Option<(usize, String)> {
    if !has(s, i, "export") {
        return None;
    }
    let j = ws_fwd(s, i + 6);
    if j == i + 6 || !has(s, j, "class") {
        return None;
    }
    let n = ws_fwd(s, j + 5);
    if n == j + 5 {
        return None; // `\s+` setelah `class`
    }
    let ke = ident_at(s, n)?;
    items.push(Exported {
        name: s[n..ke].to_string(),
        alias: None,
    });
    Some((ke, format!("class {}", &s[n..ke])))
}

/// `export\s*\*\s*from\s+['"]([^'"]+)['"]\s*;?`
fn e13(s: &str, i: usize) -> Option<(usize, String)> {
    if !has(s, i, "export") {
        return None;
    }
    let j = ws_fwd(s, i + 6);
    if !has(s, j, "*") {
        return None;
    }
    let m = ws_fwd(s, j + 1);
    let (path, end) = from_path(s, m)?;
    Some((
        end,
        format!("Object.assign(module.exports, require('{path}'))"),
    ))
}

thread_local! {
    /// `hasDefaultExport` + `defaultExportValue` — dua closure aturan yang
    /// sama-sama menuliskannya, jadi disimpan sekali per konversi.
    static CURRENT: std::cell::RefCell<DefaultState> =
        const { std::cell::RefCell::new(DefaultState { has_default: false, default_value: String::new() }) };
}

#[derive(Clone)]
struct DefaultState {
    has_default: bool,
    default_value: String,
}

pub(crate) fn convert_esm_to_cjs(code: &str) -> String {
    CURRENT.with(|c| {
        *c.borrow_mut() = DefaultState {
            has_default: false,
            default_value: String::new(),
        }
    });

    let mut items: Vec<Exported> = Vec::new();
    let mut r = code.to_string();
    r = replace_pass(&r, e1);
    r = replace_pass(&r, e2);
    r = replace_pass(&r, e3);
    r = replace_pass(&r, e4);
    r = replace_pass(&r, e5);
    r = replace_pass(&r, e6);
    r = replace_pass(&r, e8);
    r = replace_pass(&r, |s, i| e9(s, i, &mut items));
    r = replace_pass(&r, |s, i| e10(s, i, &mut items));
    r = replace_pass(&r, |s, i| e11(s, i, &mut items));
    r = replace_pass(&r, |s, i| e12(s, i, &mut items));
    r = replace_pass(&r, e13);

    let state = CURRENT.with(|c| c.borrow().clone());
    let mut export_code = String::new();
    if state.has_default {
        export_code.push_str(&format!("\nmodule.exports = {}", state.default_value));
    }
    if !items.is_empty() {
        let pairs: Vec<String> = items.iter().map(Exported::pair).collect();
        export_code.push_str(&if state.has_default {
            format!("\nmodule.exports = {{ ...module.exports, {} }}", pairs.join(", "))
        } else {
            format!("\nmodule.exports = {{ {} }}", pairs.join(", "))
        });
    }

    collapse_blank_lines(&format!("{}{export_code}", r.trim()))
}

/// `\n{3,}` → `\n\n`.
fn collapse_blank_lines(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut run = 0usize;
    for c in s.chars() {
        if c == '\n' {
            run += 1;
            if run <= 2 {
                out.push('\n');
            }
        } else {
            run = 0;
            out.push(c);
        }
    }
    out
}

#[plugin(
    name = "esmtocjs",
    alias = ["esm2cjs", "esmconvert"],
    category = "tools",
    description = "Convert ESM (ES Modules) ke CommonJS",
    usage = ".esmtocjs <reply kode>",
    example = ".esmtocjs",
    cooldown = 5,
    energi = 1,
    is_private = false
)]
pub fn esmtocjs(ctx: Ctx) -> Reply {
    let code = ctx
        .quoted()
        .map(|q| q.text.as_str())
        .filter(|t| !t.is_empty())
        .or_else(|| {
            let t = ctx.text.trim();
            if t.is_empty() { None } else { Some(t) }
        });

    let Some(code) = code else {
        return Reply::text(format!(
            "🔄 *ᴇsᴍ ᴛᴏ ᴄᴊs ᴄᴏɴᴠᴇʀᴛᴇʀ*\n\n\
             > Convert ES Modules ke CommonJS\n\n\
             > *Cara pakai:*\n\
             > Reply kode ESM dengan {}esmtocjs\n\n\
             > *Contoh ESM:*\n\
             > `import axios from 'axios'`\n\
             > `export default function() {{}}`",
            ctx.prefix
        ));
    };

    Reply::text(convert_esm_to_cjs(code))
}

#[cfg(test)]
mod tests {
    use super::*;

    use oktz_plugin_api::QuotedMessage;

    fn quoted(text: &str) -> QuotedMessage {
        QuotedMessage {
            id: "q1".into(),
            sender: "628999@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            kind: oktz_plugin_api::QuotedKind::Text,
            mimetype: None,
            file_name: None,
            from_me: false,
        }
    }

    fn run(text: &str, quoted: Option<QuotedMessage>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "esmtocjs".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: vec!["import".into(), "axios".into()],
            quoted,
            ..Default::default()
        };
        (oktz_plugin_api::find("esmtocjs").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("esmtocjs").unwrap();
        assert_eq!(p.meta.name, "esmtocjs");
        assert_eq!(p.meta.alias, ["esm2cjs", "esmconvert"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.description, "Convert ESM (ES Modules) ke CommonJS");
        assert_eq!(p.meta.usage, ".esmtocjs <reply kode>");
        assert_eq!(p.meta.example, ".esmtocjs");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_private && !p.meta.is_group && !p.meta.is_owner);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["esm2cjs", "esmconvert"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "esmtocjs");
        }
    }

    #[test]
    fn without_code_it_shows_the_manual() {
        let r = run("", None);
        let t = text_of(&r);
        assert!(t.starts_with("🔄 *ᴇsᴍ ᴛᴏ ᴄᴊs ᴄᴏɴᴠᴇʀᴛᴇʀ*"), "{t}");
        assert!(t.contains("> Reply kode ESM dengan .esmtocjs"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_quoted_body_wins_over_the_command_text() {
        let r = run("diabaikan", Some(quoted("import axios from 'axios'")));
        assert_eq!(text_of(&r), "const axios = require('axios')");
    }

    // Semua hasil di bawah dicocokkan keluar dari `convertEsmToCjs` Node.
    #[test]
    fn conversions_match_the_js_regex_pipeline() {
        for (src, want) in [
            ("import axios from 'axios'", "const axios = require('axios')"),
            (
                "import axios, { a, b } from 'axios'",
                "const axios = require('axios')\nconst { a, b } = require('axios')",
            ),
            ("import { a, b as c } from 'm'", "const { a, b: c } = require('m')"),
            ("import * as ns from 'm'", "const ns = require('m')"),
            ("import 'side-effect'", "require('side-effect')"),
            ("import {a,b,c} from 'm'", "const { a, b, c } = require('m')"),
            ("export default handler", "\nmodule.exports = handler"),
            ("export default { a: 1 }", "\nmodule.exports = { a: 1 }"),
            ("export { a, b as c }", "\nmodule.exports = { a, c: b }"),
            ("export{ a }", "\nmodule.exports = { a }"),
            ("export const x = 1", "const x = 1\nmodule.exports = { x }"),
            ("export let n = 1", "let n = 1\nmodule.exports = { n }"),
            ("export var n = 1", "var n = 1\nmodule.exports = { n }"),
            ("export function run() {}", "function run() {}\nmodule.exports = { run }"),
            ("export async function run() {}", "async function run() {}\nmodule.exports = { run }"),
            ("export class K {}", "class K {}\nmodule.exports = { K }"),
            ("export * from './lib'", "Object.assign(module.exports, require('./lib'))"),
            ("export *from 'm'", "Object.assign(module.exports, require('m'))"),
            ("let z = 3", "let z = 3"),
            (
                "export const x = 1\nexport const y = 2",
                "const x = 1\nconst y = 2\nmodule.exports = { x, y }",
            ),
            (
                "import a from 'a'\nexport default a\nexport const y = 2",
                "const a = require('a')const y = 2\nmodule.exports = a\nmodule.exports = { ...module.exports, y }",
            ),
        ] {
            assert_eq!(convert_esm_to_cjs(src), want, "input: {src:?}");
        }
    }

    // `export default (\w+)` jalan lebih dulu dan `\w+` makan kata `function`,
    // jadi badan `export default function …` ikut hilang (bug JS yang diport).
    #[test]
    fn the_default_function_branch_is_eaten_by_the_word_rule() {
        assert_eq!(
            convert_esm_to_cjs("export default function foo() {}"),
            "foo() {}\nmodule.exports = function"
        );
    }

    #[test]
    fn the_state_does_not_leak_between_conversions() {
        assert_eq!(convert_esm_to_cjs("export default a"), "\nmodule.exports = a");
        assert_eq!(
            convert_esm_to_cjs("export const b = 1"),
            "const b = 1\nmodule.exports = { b }",
            "default dari panggilan sebelumnya tidak boleh ikut"
        );
    }
}
