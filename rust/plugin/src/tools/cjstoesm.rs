//! `plugins/tools/cjstoesm.js` — ubah `require()`/`module.exports` jadi ESM.
//!
//! `convertCjsToEsm` di JS adalah 15 `String.replace` berurutan dengan regex.
//! Regex tidak boleh dipakai di crate ini (tidak ada dependency `regex`), jadi
//! tiap aturan ditulis ulang sebagai pemindai byte — DAN urutan, sifat
//! serakah, serta bug JS-nya dipakai apa adanya. Dua di antaranya penting:
//!
//! - Aturan `.default` (`cjstoesm.js:36`) tidak pernah jalan: aturan `.prop`
//!   (`:31`) lebih dulu cocok dan menelan `.default` sebagai properti biasa.
//! - Aturan `exports.X` (`:70-86`) tidak punya batas kata, jadi ia juga cocok
//!   di dalam `module.exports.X` dan menghasilkan `module.export { … }`.
//!
//! `ponytail:` `sock.sendCodeBlock` tidak ada di API port, jadi hasil konversi
//! dikirim sebagai teks biasa. Cabang `catch` JS tidak diport — konversi string
//! di Rust tidak bisa gagal di tengah jalan seperti `replace` JS.

use oktz_plugin_api::{Ctx, Reply, plugin};

// ---------- primitif pemindai (pengganti regex) ----------

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn next_boundary(s: &str, i: usize) -> usize {
    match s[i..].chars().next() {
        Some(c) => i + c.len_utf8(),
        None => i + 1,
    }
}

/// `\w+` — JS `\w` = `[A-Za-z0-9_]`, tanpa `$`.
fn ident_at(s: &str, i: usize) -> Option<usize> {
    let mut j = i;
    while let Some(c) = s[j..].chars().next() {
        if !is_word(c) {
            break;
        }
        j += c.len_utf8();
    }
    (j > i).then_some(j)
}

/// `\b` di depan posisi `i`: regex JS di sini tidak pakainya, tapi pemindai
/// `const|let|var` dan `require` butuh batas agar `constant`/`myrequire` tidak
/// ikut cocok.
fn word_start_before(s: &str, i: usize) -> bool {
    s[..i].chars().next_back().is_none_or(|c| !is_word(c))
}

fn ws_fwd(s: &str, mut i: usize) -> usize {
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

/// `\s*=\s*` setelah posisi `i`; `None` kalau tidak ada `=` di titik itu.
fn eq_after(s: &str, i: usize) -> Option<usize> {
    let j = ws_fwd(s, i);
    if !s[j..].starts_with('=') {
        return None;
    }
    Some(ws_fwd(s, j + 1))
}

/// `(async\s+)?` — `\s+` wajib ada, jadi `asynchronous` tidak ikut.
fn async_prefix(s: &str, i: usize) -> (String, usize) {
    if s[i..].starts_with("async") {
        let j = ws_fwd(s, i + 5);
        if j > i + 5 {
            return ("async ".to_string(), j);
        }
    }
    (String::new(), i)
}

/// `\(([^)]*)\)` — regex JS tidak bisa nesting, ini juga tidak.
fn paren(s: &str, i: usize) -> Option<(String, usize)> {
    if !s[i..].starts_with('(') {
        return None;
    }
    let r = s[i + 1..].find(')')?;
    Some((s[i + 1..i + 1 + r].to_string(), i + 1 + r + 1))
}

/// `require\s*\(\s*['"]([^'"]+)['"]\s*\)` mulai di `i`.
fn require_call(s: &str, i: usize) -> Option<(String, usize)> {
    if !s[i..].starts_with("require") || !word_start_before(s, i) {
        return None;
    }
    let j = ws_fwd(s, i + 7);
    if !s[j..].starts_with('(') {
        return None;
    }
    let k = ws_fwd(s, j + 1);
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
    if !s[m..].starts_with(')') {
        return None;
    }
    Some((path, m + 1))
}

fn decl_kw(s: &str, i: usize) -> Option<usize> {
    for kw in ["const", "let", "var"] {
        if s[i..].starts_with(kw) && word_start_before(s, i) {
            return Some(i + kw.len());
        }
    }
    None
}

/// `\s*=\s*require(...)` + ekor `\s*;?` — dipakai aturan `const/let/var`.
fn require_tail(s: &str, i: usize) -> Option<(String, usize)> {
    let m = eq_after(s, i)?;
    let (path, end) = require_call(s, m)?;
    Some((path, semi(s, ws_fwd(s, end))))
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

/// `{ a, b: c }` → `a, b as c` (`cjstoesm.js:23-27`).
fn named_to_import(list: &str) -> String {
    list.split(',')
        .map(|item| {
            let p: Vec<&str> = item.trim().split(':').map(str::trim).collect();
            if p.len() == 2 {
                format!("{} as {}", p[0], p[1])
            } else {
                p[0].to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `{ a, b: c }` → `a, c as b` (`cjstoesm.js:49-54`).
fn obj_to_export(list: &str) -> String {
    list.split(',')
        .map(|item| {
            let p: Vec<&str> = item.trim().split(':').map(str::trim).collect();
            if p.len() == 2 {
                format!("{} as {}", p[1], p[0])
            } else {
                p[0].to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

// ---------- aturan 1..15, urutannya sama dengan JS ----------

/// `(?:const|let|var)\s*\{\s*([^}]+)\s*\}\s*=\s*require\(...\)\s*;?`
fn r1(s: &str, i: usize) -> Option<(usize, String)> {
    let b = ws_fwd(s, decl_kw(s, i)?);
    if !s[b..].starts_with('{') {
        return None;
    }
    let rel = s[b + 1..].find('}')?;
    if rel == 0 {
        return None; // `([^}]+)` minimal satu karakter
    }
    let inner = &s[b + 1..b + 1 + rel];
    let (path, end) = require_tail(s, b + 2 + rel)?;
    Some((end, format!("import {{ {} }} from '{path}';", named_to_import(inner))))
}

/// `(?:const|let|var)\s+(\w+)\s*=\s*require\(...\)\.(\w+)\s*;?` — aturan ini
/// yang menangkap `… .default` (aturan `.default` di JS tidak pernah jalan).
fn r2(s: &str, i: usize) -> Option<(usize, String)> {
    let k = decl_kw(s, i)?;
    let j = ws_fwd(s, k);
    if j == k {
        return None; // `\s+`
    }
    let a = ident_at(s, j)?;
    let name = &s[j..a];
    let m = eq_after(s, a)?;
    let (path, e) = require_call(s, m)?;
    if !s[e..].starts_with('.') {
        return None;
    }
    let p = ident_at(s, e + 1)?;
    let prop = &s[e + 1..p];
    let end = semi(s, ws_fwd(s, p));
    let imp = if name == prop {
        format!("import {{ {prop} }} from '{path}';")
    } else {
        format!("import {{ {prop} as {name} }} from '{path}';")
    };
    Some((end, imp))
}

/// `(?:const|let|var)\s+(\w+)\s*=\s*require\(...\)\s*;?`
fn r4(s: &str, i: usize) -> Option<(usize, String)> {
    let k = decl_kw(s, i)?;
    let j = ws_fwd(s, k);
    if j == k {
        return None;
    }
    let a = ident_at(s, j)?;
    let (path, end) = require_tail(s, a)?;
    Some((end, format!("import {} from '{path}';", &s[j..a])))
}

/// `^require\(...\)\s*;?$` (flag `m`) — `\s*` serakah sampai baris kosong,
/// jadi `;` bisa muncul setelah spasi dan `$` diuji dengan mundur.
fn r5(s: &str, i: usize) -> Option<(usize, String)> {
    if i != 0 && !s[..i].ends_with('\n') {
        return None;
    }
    let (path, e) = require_call(s, i)?;
    let line_end = |p: usize| p == s.len() || s[p..].starts_with('\n') || s[p..].starts_with('\r');
    let mut end = ws_fwd(s, e);
    loop {
        if line_end(end) || (s[end..].starts_with(';') && line_end(end + 1)) {
            let end = if line_end(end) { end } else { end + 1 };
            return Some((end, format!("import '{path}';")));
        }
        if end == e {
            return None;
        }
        end -= 1;
        while end > e && !s.is_char_boundary(end) {
            end -= 1;
        }
    }
}

fn mod_exports(s: &str, i: usize) -> Option<usize> {
    s[i..]
        .starts_with("module.exports")
        .then_some(i + "module.exports".len())
}

fn exports_prop(s: &str, i: usize) -> Option<usize> {
    s[i..].starts_with("exports.").then_some(i + "exports.".len())
}

/// `module.exports\s*=\s*\{([^}]+)\}\s*;?`
fn r6(s: &str, i: usize) -> Option<(usize, String)> {
    let j = eq_after(s, mod_exports(s, i)?)?;
    if !s[j..].starts_with('{') {
        return None;
    }
    let rel = s[j + 1..].find('}')?;
    if rel == 0 {
        return None; // `([^}]+)` minimal satu karakter, jadi `{}` jatuh ke r9
    }
    let inner = &s[j + 1..j + 1 + rel];
    let end = semi(s, ws_fwd(s, j + 2 + rel));
    Some((end, format!("export {{ {} }};", obj_to_export(inner))))
}

/// `(async\s+)?(function|class)\s*(\w*)\s*(\([^)]*\))?\s*\{`
///
/// `params` dikembalikan INCLUDING tanda kurung — `(\([^)]*\))` di JS ikut
/// ditulis apa adanya ke hasil. `classes` menolak `class`: hanya aturan
/// `module.exports` (r7) yang memperbolehkannya, `exports.*` (r10) tidak.
fn decl_tail(s: &str, i: usize, classes: bool) -> Option<(&'static str, String, String, String, usize)> {
    let (aw, j) = async_prefix(s, i);
    let ty = if s[j..].starts_with("function") {
        "function"
    } else if classes && s[j..].starts_with("class") {
        "class"
    } else {
        return None;
    };
    let k = ws_fwd(s, j + ty.len());
    let n = ident_at(s, k).unwrap_or(k);
    let m = ws_fwd(s, n);
    let (params, p) = if s[m..].starts_with('(') {
        let (inner, e) = paren(s, m)?;
        (format!("({inner})"), e)
    } else {
        (String::new(), m)
    };
    let q = ws_fwd(s, p);
    if !s[q..].starts_with('{') {
        return None;
    }
    Some((ty, aw, s[k..n].to_string(), params, q + 1))
}

/// `module.exports\s*=\s*(async\s+)?(function|class)\s*(\w*)\s*(\([^)]*\))?\s*\{`
fn r7(s: &str, i: usize) -> Option<(usize, String)> {
    let j = eq_after(s, mod_exports(s, i)?)?;
    let (ty, aw, name, params, end) = decl_tail(s, j, true)?;
    let body = if name.is_empty() {
        format!("export default {aw}{ty}{params} {{")
    } else {
        format!("export default {aw}{ty} {name}{params} {{")
    };
    Some((end, body))
}

/// `module.exports\s*=\s*(async\s+)?\(([^)]*)\)\s*=>`
fn r8(s: &str, i: usize) -> Option<(usize, String)> {
    let j = eq_after(s, mod_exports(s, i)?)?;
    let (aw, k) = async_prefix(s, j);
    if !s[k..].starts_with('(') {
        return None;
    }
    let (params, e) = paren(s, k)?;
    let e = ws_fwd(s, e);
    if !s[e..].starts_with("=>") {
        return None;
    }
    Some((e + 2, format!("export default {aw}({params}) =>")))
}

/// `module.exports\s*=\s*([^;\n]+)\s*;?` — nilai serakah sampai `;`/newline,
/// jadi spasi akhir ikut terbawa ke hasil.
fn r9(s: &str, i: usize) -> Option<(usize, String)> {
    let e = eq_after(s, mod_exports(s, i)?)?;
    let mut k = e;
    while k < s.len() {
        let c = s[k..].chars().next().unwrap();
        if c == ';' || c == '\n' {
            break;
        }
        k += c.len_utf8();
    }
    if k == e {
        return None;
    }
    let value = &s[e..k];
    let end = semi(s, ws_fwd(s, k));
    Some((end, format!("export default {value};")))
}

/// `exports\.(\w+)\s*=\s*(async\s+)?function\s*(\w*)\s*(\([^)]*\))?\s*\{`
fn r10(s: &str, i: usize) -> Option<(usize, String)> {
    let p = exports_prop(s, i)?;
    let ke = ident_at(s, p)?;
    let (ty, aw, _name, params, end) = decl_tail(s, eq_after(s, ke)?, false)?;
    let body = format!("export {aw}{ty} {}{params} {{", &s[p..ke]);
    Some((end, body))
}

/// `exports\.(\w+)\s*=\s*(async\s+)?\(([^)]*)\)\s*=>\s*\{`
fn r11(s: &str, i: usize) -> Option<(usize, String)> {
    let p = exports_prop(s, i)?;
    let ke = ident_at(s, p)?;
    let (aw, k) = async_prefix(s, eq_after(s, ke)?);
    if !s[k..].starts_with('(') {
        return None;
    }
    let (params, e) = paren(s, k)?;
    let e = ws_fwd(s, e);
    if !s[e..].starts_with("=>") {
        return None;
    }
    let e = ws_fwd(s, e + 2);
    if !s[e..].starts_with('{') {
        return None;
    }
    Some((e + 1, format!("export const {} = {aw}({params}) => {{", &s[p..ke])))
}

/// `exports\.(\w+)\s*=\s*(\w+)\s*;?`
fn r12(s: &str, i: usize) -> Option<(usize, String)> {
    let p = exports_prop(s, i)?;
    let ke = ident_at(s, p)?;
    let j = eq_after(s, ke)?;
    let ve = ident_at(s, j)?;
    let end = semi(s, ws_fwd(s, ve));
    Some((end, export_pair(&s[p..ke], &s[j..ve])))
}

/// `exports\.(\w+)\s*=\s*([^;\n]+)\s*;?` — nilai yang diawali `async`,
/// `function`, `(` atau `class` dikembalikan apa adanya (`cjstoesm.js:84`).
fn r13(s: &str, i: usize) -> Option<(usize, String)> {
    let p = exports_prop(s, i)?;
    let ke = ident_at(s, p)?;
    let j = eq_after(s, ke)?;
    let mut k = j;
    while k < s.len() {
        let c = s[k..].chars().next().unwrap();
        if c == ';' || c == '\n' {
            break;
        }
        k += c.len_utf8();
    }
    if k == j {
        return None;
    }
    let value = &s[j..k];
    let end = semi(s, ws_fwd(s, k));
    let trimmed = value.trim();
    if ["async", "function", "(", "class"].iter().any(|p| trimmed.starts_with(p)) {
        return Some((end, s[i..end].to_string()));
    }
    Some((end, format!("export const {} = {value};", &s[p..ke])))
}

/// `module\.exports\.(\w+)\s*=\s*(\w+)\s*;?`
fn r14(s: &str, i: usize) -> Option<(usize, String)> {
    let p = mod_exports(s, i)?;
    if !s[p..].starts_with('.') {
        return None;
    }
    let ke = ident_at(s, p + 1)?;
    let j = eq_after(s, ke)?;
    let ve = ident_at(s, j)?;
    let end = semi(s, ws_fwd(s, ve));
    Some((end, export_pair(&s[p + 1..ke], &s[j..ve])))
}

fn export_pair(key: &str, value: &str) -> String {
    if key == value {
        format!("export {{ {key} }};")
    } else {
        format!("export {{ {value} as {key} }};")
    }
}

/// `Object\.assign\s*\(\s*module\.exports\s*,\s*require\(...\)\s*\)\s*;?`
fn r15(s: &str, i: usize) -> Option<(usize, String)> {
    let head = "Object.assign";
    if !s[i..].starts_with(head) {
        return None;
    }
    let j = ws_fwd(s, i + head.len());
    if !s[j..].starts_with('(') {
        return None;
    }
    let k = ws_fwd(s, j + 1);
    let m = mod_exports(s, k)?;
    let m = ws_fwd(s, m);
    if !s[m..].starts_with(',') {
        return None;
    }
    let n = ws_fwd(s, m + 1);
    let (path, e) = require_call(s, n)?;
    let e = ws_fwd(s, e);
    if !s[e..].starts_with(')') {
        return None;
    }
    let end = semi(s, ws_fwd(s, e + 1));
    Some((end, format!("export * from '{path}';")))
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

pub(crate) const DIRNAME_HELPER: &str = "import { fileURLToPath } from 'url';\nimport { dirname } from 'path';\nconst __filename = fileURLToPath(import.meta.url);\nconst __dirname = dirname(__filename);\n\n";

pub(crate) fn convert_cjs_to_esm(code: &str) -> String {
    let mut r = code.to_string();
    r = replace_pass(&r, r1);
    r = replace_pass(&r, r2);
    r = replace_pass(&r, r4);
    r = replace_pass(&r, r5);
    r = replace_pass(&r, r6);
    r = replace_pass(&r, r7);
    r = replace_pass(&r, r8);
    r = replace_pass(&r, r9);
    r = replace_pass(&r, r10);
    r = replace_pass(&r, r11);
    r = replace_pass(&r, r12);
    r = replace_pass(&r, r13);
    r = replace_pass(&r, r14);
    r = replace_pass(&r, r15);

    if (r.contains("__dirname") || r.contains("__filename")) && !r.contains("fileURLToPath") {
        r = format!("{DIRNAME_HELPER}{r}");
    }
    collapse_blank_lines(&r).trim().to_string()
}

#[plugin(
    name = "cjstoesm",
    alias = ["cjs2esm", "cjsconvert"],
    category = "tools",
    description = "Convert CommonJS ke ESM (ES Modules)",
    usage = ".cjstoesm <reply kode>",
    example = ".cjstoesm",
    cooldown = 5,
    energi = 1,
    is_private = false
)]
pub fn cjstoesm(ctx: Ctx) -> Reply {
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
            "🔄 *ᴄᴊs ᴛᴏ ᴇsᴍ ᴄᴏɴᴠᴇʀᴛᴇʀ*\n\n\
             > Convert CommonJS ke ES Modules\n\n\
             > *Cara pakai:*\n\
             > Reply kode CJS dengan {}cjstoesm\n\n\
             > *Contoh CJS:*\n\
             > `const axios = require('axios')`\n\
             > `module.exports = handler`",
            ctx.prefix
        ));
    };

    Reply::text(convert_cjs_to_esm(code))
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
            timestamp: 0,
        }
    }

    fn run(text: &str, quoted: Option<QuotedMessage>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "cjstoesm".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: vec!["const".into(), "axios".into()],
            quoted,
            ..Default::default()
        };
        (oktz_plugin_api::find("cjstoesm").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("cjstoesm").unwrap();
        assert_eq!(p.meta.name, "cjstoesm");
        assert_eq!(p.meta.alias, ["cjs2esm", "cjsconvert"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.description, "Convert CommonJS ke ESM (ES Modules)");
        assert_eq!(p.meta.usage, ".cjstoesm <reply kode>");
        assert_eq!(p.meta.example, ".cjstoesm");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_private && !p.meta.is_group && !p.meta.is_owner);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["cjs2esm", "cjsconvert"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "cjstoesm");
        }
    }

    #[test]
    fn without_code_it_shows_the_manual() {
        let r = run("   ", None);
        let t = text_of(&r);
        assert!(t.starts_with("🔄 *ᴄᴊs ᴛᴏ ᴇsᴍ ᴄᴏɴᴠᴇʀᴛᴇʀ*"), "{t}");
        assert!(t.contains("> Reply kode CJS dengan .cjstoesm"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_quoted_body_wins_over_the_command_text() {
        let r = run(
            "diabaikan",
            Some(quoted("const axios = require('axios')")),
        );
        assert_eq!(text_of(&r), "import axios from 'axios';");
    }

    // Semua hasil di bawah dicocokkan keluar dari `convertCjsToEsm` Node,
    // bukan dikarang.
    #[test]
    fn conversions_match_the_js_regex_pipeline() {
        for (src, want) in [
            ("const axios = require('axios')", "import axios from 'axios';"),
            ("const { a, b } = require('x')", "import { a, b } from 'x';"),
            ("const { a, b: c } = require('x')", "import { a, b as c } from 'x';"),
            // `d: e: f` punya tiga bagian, jadi aturan JS jatuh ke `parts[0]`.
            (
                "var { a, b: c, d: e: f } = require('z')",
                "import { a, b as c, d } from 'z';",
            ),
            ("let x = require('y').default", "import { default as x } from 'y';"),
            ("const f = require('y').join", "import { join as f } from 'y';"),
            ("require('side-effect')", "import 'side-effect';"),
            ("module.exports = { a, b: c }", "export { a, c as b };"),
            (
                "module.exports = async function handler(a, b) {\n  return a\n}",
                "export default async function handler(a, b) {\n  return a\n}",
            ),
            (
                "module.exports = (a, b) => a + b",
                "export default (a, b) => a + b",
            ),
            ("module.exports = someValue", "export default someValue;"),
            // Spasi akhir ikut nilai `[^;\n]+` yang serakah.
            ("module.exports = p ", "export default p ;"),
            ("module.exports = class Foo {\n}", "export default class Foo {\n}"),
            ("module.exports = {\n}", "export {  };"),
            // `{}` tidak punya isi, jadi `([^}]+)` gagal dan aturannya jatuh
            // ke `module.exports\s*=\s*([^;\n]+)`.
            ("module.exports = {}", "export default {};"),
            ("const {} = require('empty')", "const {} = require('empty')"),
            // `exports.*` hanya menerima `function`, bukan `class`.
            (
                "exports.cls = class Foo {}",
                "export { class as cls };Foo {}",
            ),
            (
                "exports.hello = function () {\n  return 1\n}",
                "export function hello() {\n  return 1\n}",
            ),
            (
                "exports.fn = async function named(a) {\n  return a\n}",
                "export async function fn(a) {\n  return a\n}",
            ),
            (
                "exports.fn = (a) => {\n  return a\n}",
                "export const fn = (a) => {\n  return a\n}",
            ),
            ("exports.name = value", "export { value as name };"),
            ("exports.a = asyncThing", "export { asyncThing as a };"),
            // Aturan `exports.` tidak punya batas kata: dia ikut memotong
            // `module.exports.ok = value` jadi `module.export { … }`.
            ("module.exports.ok = value", "module.export { value as ok };"),
            (
                "Object.assign(module.exports, require('./lib'))",
                "export * from './lib';",
            ),
        ] {
            assert_eq!(convert_cjs_to_esm(src), want, "input: {src:?}");
        }
    }

    #[test]
    fn trailing_whitespace_after_require_is_swallowed_like_the_regex() {
        assert_eq!(
            convert_cjs_to_esm("const {a} = require('x');\nconst b = require('y')"),
            "import { a } from 'x';\nimport b from 'y';"
        );
        assert_eq!(
            convert_cjs_to_esm("const a = require('b')\n\n\n\nconst c = require('d')"),
            "import a from 'b';import c from 'd';"
        );
    }

    #[test]
    fn side_effect_require_is_converted_per_line() {
        assert_eq!(
            convert_cjs_to_esm("require('side')\nrequire('side2')"),
            "import 'side';\nimport 'side2';"
        );
    }

    #[test]
    fn dirname_helper_is_prepended_once() {
        let out = convert_cjs_to_esm("const p = require('path')\nconst __dirname = '/x'\nmodule.exports = p");
        assert!(
            out.starts_with("import { fileURLToPath } from 'url';\nimport { dirname } from 'path';\nconst __filename = fileURLToPath(import.meta.url);\nconst __dirname = dirname(__filename);\n\n"),
            "{out}"
        );
        assert_eq!(out.matches("fileURLToPath").count(), 2, "helper +_import_: {out}");
        assert!(
            convert_cjs_to_esm("const __dirname = '/x'\nconst u = fileURLToPath(1)\nmodule.exports = u")
                .starts_with("const __dirname"),
            "sudah punya fileURLToPath -> helper tidak ditambah"
        );
    }

    #[test]
    fn three_blank_lines_collapse_to_two() {
        assert_eq!(convert_cjs_to_esm("\n\n\n\na = 1\n\n\n\nb = 2\n"), "a = 1\n\nb = 2");
    }
}
