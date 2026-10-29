//! Port `parseCommand` (`src/lib/serialize.js:380-435`) + `getMessageBody`
//! (`serialize.js:327-372`).
//!
//! Catatan yang perlu dijaga saat port:
//! - Parameter `prefix` di JS adalah dead code — fungsi membaca daftar prefix
//!   dari cache global. Di sini prefix jadi input eksplisit (lebih jujur), dan
//!   caller yang controlling.`getCachedPrefixes` (serialize.js:45-63).
//! - `config.command.caseSensitive` default false → command di-lowercase.

/// Hasil `parseCommand` JS.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedCommand {
    pub is_command: bool,
    pub command: String,
    pub prefix: String,
    pub args: Vec<String>,
    pub text: String,
    pub full_args: String,
}

/// Opsi parse yang memengaruhi perilaku.
#[derive(Debug, Clone)]
pub struct ParseOptions {
    /// Prefix yang dicoba, berurutan. `config.command.prefix` selalu pertama.
    pub prefixes: Vec<String>,
    /// Aktifkan branch noprefix (`database/prefix.json` `noprefix: true`).
    pub no_prefix: bool,
    /// Default false → command di-lowercase.
    pub case_sensitive: bool,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            prefixes: vec![".".into()],
            no_prefix: false,
            case_sensitive: false,
        }
    }
}

/// Prefix kosong akan match SEMUA pesan (JS punya bug ini juga karena
/// `body.startsWith("")` selalu true). Guard di sini: prefix kosong dilewati.
fn usable_prefix(p: &str) -> bool {
    !p.is_empty()
}

/// `parse.split(/\s+/)` setelah `.trim()` — hasilnya tidak pernah berisi
/// string kosong, sama seperti JS.
fn split_ws(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_string).collect()
}

/// Port persis `parseCommand`.
pub fn parse_command(body: &str, opts: &ParseOptions) -> ParsedCommand {
    let mut result = ParsedCommand::default();
    if body.is_empty() {
        return result;
    }

    // Loop prefix. Panjang lebih panjang dulu? JS memakai urutan cache —
    // jadi JANGAN reorder, "." dan "," bisa tumpang tindih.
    for p in &opts.prefixes {
        if !usable_prefix(p) || !body.starts_with(p.as_str()) {
            continue;
        }
        let without_prefix = body[p.len()..].trim();
        let parts = split_ws(without_prefix);

        result.is_command = true;
        result.prefix = p.clone();
        // `parts[0]` bisa "" kalau body persis sama dengan prefix.
        let first = parts.first().cloned().unwrap_or_default();
        result.command = if opts.case_sensitive {
            first
        } else {
            first.to_lowercase()
        };
        result.args = parts.into_iter().skip(1).collect();
        result.text = without_prefix[result.command.len().min(without_prefix.len())..].trim().to_string();
        result.full_args = result.text.clone();
        return result;
    }

    // Branch noprefix.
    if opts.no_prefix {
        let parts = split_ws(body.trim());
        if let Some(potential) = parts.first() {
            let candidate = if opts.case_sensitive {
                potential.clone()
            } else {
                potential.to_lowercase()
            };
            // Guard JS: `/^[a-z0-9_-]+$/i` dan `length <= 20`.
            let valid_chars = !candidate.is_empty()
                && candidate
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            if valid_chars && candidate.len() <= 20 {
                result.is_command = true;
                result.prefix = String::new();
                result.command = candidate;
                result.args = parts.into_iter().skip(1).collect();
                // JS: `text = args.join(" ")` (re-normalized).
                result.text = result.args.join(" ");
                // JS: `fullArgs = body.slice(cmd.length).trim()` (raw slice) —
                // BEDA dari text. Pertahankan perbedaan ini.
                result.full_args = body[result.command.len().min(body.len())..].trim().to_string();
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> ParseOptions {
        ParseOptions::default()
    }

    #[test]
    fn empty_body_is_not_a_command() {
        assert_eq!(parse_command("", &opts()), ParsedCommand::default());
    }

    #[test]
    fn plain_text_is_not_a_command() {
        let r = parse_command("halo dunia", &opts());
        assert!(!r.is_command);
        assert!(r.command.is_empty());
    }

    #[test]
    fn simple_command_with_arg() {
        let r = parse_command(".ping", &opts());
        assert!(r.is_command);
        assert_eq!(r.command, "ping");
        assert_eq!(r.prefix, ".");
        assert!(r.args.is_empty());
        assert_eq!(r.text, "");
    }

    #[test]
    fn command_with_args_splits_and_lowercases() {
        let r = parse_command(".menu PING Extra", &opts());
        assert!(r.is_command);
        assert_eq!(r.command, "menu");
        assert_eq!(r.args, vec!["PING", "Extra"]);
        assert_eq!(r.text, "PING Extra");
        assert_eq!(r.full_args, "PING Extra");
    }

    #[test]
    fn command_itself_lowercased_but_args_preserved() {
        let r = parse_command(".PiNg Foo", &opts());
        assert_eq!(r.command, "ping");
        assert_eq!(r.args, vec!["Foo"]);
    }

    #[test]
    fn case_sensitive_keeps_command_as_is() {
        let o = ParseOptions {
            case_sensitive: true,
            ..opts()
        };
        let r = parse_command(".PiNg", &o);
        assert_eq!(r.command, "PiNg");
    }

    #[test]
    fn whitespace_collapsed_so_args_never_empty() {
        let r = parse_command(".cmd    a     b", &opts());
        assert_eq!(r.args, vec!["a", "b"]);
    }

    #[test]
    fn secondary_prefix_supported() {
        let o = ParseOptions {
            prefixes: vec![".".into(), "!".into()],
            ..opts()
        };
        let r = parse_command("!ping", &o);
        assert!(r.is_command);
        assert_eq!(r.command, "ping");
        assert_eq!(r.prefix, "!");
    }

    #[test]
    fn empty_prefix_does_not_match_everything() {
        // Guard: prefix "" harus di-skip, bukan match semua pesan.
        let o = ParseOptions {
            prefixes: vec!["".into(), ".".into()],
            ..opts()
        };
        let r = parse_command("apapun", &o);
        assert!(!r.is_command, "prefix kosong tidak boleh match semua");
    }

    #[test]
    fn noprefix_branch_accepts_bare_word() {
        let o = ParseOptions {
            no_prefix: true,
            ..opts()
        };
        let r = parse_command("ping arg1", &o);
        assert!(r.is_command);
        assert_eq!(r.command, "ping");
        assert!(r.prefix.is_empty());
        assert_eq!(r.args, vec!["arg1"]);
        assert_eq!(r.text, "arg1");
    }

    #[test]
    fn noprefix_rejects_long_or_invalid_words() {
        let o = ParseOptions {
            no_prefix: true,
            ..opts()
        };
        // Terlalu panjang (>20).
        assert!(!parse_command("abcdefghijklmnopqrstuvwxyz", &o).is_command);
        // Karakter tak valid.
        assert!(!parse_command("ping!x", &o).is_command);
    }

    #[test]
    fn noprefix_text_and_full_args_differ_preserved() {
        // JS: text=args.join(" ") (re-normalized), fullArgs=body.slice(cmd).trim()
        // (raw slice). Untuk input dengan spasi berlebih, keduanya berbeda.
        let o = ParseOptions {
            no_prefix: true,
            ..opts()
        };
        let r = parse_command("cmd   spaced   out", &o);
        assert_eq!(r.command, "cmd");
        assert_eq!(r.text, "spaced out");
        assert_eq!(r.full_args, "spaced   out");
    }

    #[test]
    fn body_equal_to_prefix_yields_empty_command() {
        // JS: parts[0] = "" → command="", isCommand=true. Menuju not-found path.
        let r = parse_command(".", &opts());
        assert!(r.is_command);
        assert_eq!(r.command, "");
    }
}