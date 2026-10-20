//! `plugins/download/githubdl.js` — unduh repository GitHub sebagai ZIP.
//!
//! `ponytail:` pemeriksaan kedua di JS memakai `fetch(..., { method: 'HEAD' })`,
//! tapi `HttpGet` (`oktz-plugin-api`) tidak punya HEAD — hanya GET. Konsekuensinya
//! ZIP yang dicek ikut terunduh utuh, dan `maxContentLength` yang di JS membatasi
//! sisanya tidak ada di sini. Tambahkan metode HEAD ke trait `HttpGet` kalau
//! repo besar mulai jadi masalah.
//!
//! `ponytail:` `Reply` tidak punya variant document, jadi hasil `sendMedia` hanya
//! terkirim sebagai nama file teks.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;


/// `githubdl.js:36-42` — kartu pakai, delimiter markdown-nya backtick.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
         > `{prefix}githubdl <user> <repo> <branch>`\n\n\
         > Contoh:\n\
         > `{prefix}githubdl niceplugin NiceBot main`\n\
         > `{prefix}githubdl https://github.com/user/repo`"
    )
}

/// `githubdl.js:46` — username ada, repository tidak.
pub(crate) const NO_REPO: &str = "❌ *ʀᴇᴘᴏ ᴅɪʙᴜᴛᴜʜᴋᴀɴ*\n\n> Masukkan nama repository";

/// `githubdl.js:56` — API GitHub menjawab tidak `ok`.
pub(crate) fn not_found(username: &str, repo: &str) -> String {
    format!("❌ *ʀᴇᴘᴏ ᴛɪᴅᴀᴋ ᴅɪᴛᴇᴍᴜᴋᴀɴ*\n\n> `{username}/{repo}` tidak ada")
}

/// `githubdl.js:68` — HEAD ke ZIP tidak `ok`.
pub(crate) fn bad_branch(branch: &str, default_branch: &str) -> String {
    format!(
        "❌ *ʙʀᴀɴᴄʜ ᴛɪᴅᴀᴋ ᴀᴅᴀ*\n\n\
         > Branch `{branch}` tidak ditemukan\n\
         > Default: `{default_branch}`"
    )
}

/// Hasil pemecahan argumen `githubdl.js:19-33`. `username`/`repo` kosong
/// berarti input malformed — hanya `username` yang diperiksa plugin.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Target {
    pub username: Option<String>,
    pub repo: Option<String>,
    pub branch: String,
}

/// `githubdl.js:23` — `/github\.com\/([^/]+)\/([^/]+)/i` tanpa regex crate.
/// `user` dan `repo` wajib punya minimal satu karakter (`[^/]+`).
fn match_repo_url(url: &str) -> Option<(String, String)> {
    const NEEDLE: &[u8] = b"github.com/";
    let bytes = url.as_bytes();
    let mut from = 0usize;
    while from + NEEDLE.len() <= bytes.len() {
        if bytes[from..from + NEEDLE.len()].eq_ignore_ascii_case(NEEDLE) {
            let rest = &url[from + NEEDLE.len()..];
            // `\/([^/]+)\/([^/]+)` — slash kedua wajib ada.
            let user_end = rest.find('/')?;
            let username = &rest[..user_end];
            let after = &rest[user_end + 1..];
            let repo_end = after.find('/').unwrap_or(after.len());
            let repo = &after[..repo_end];
            if !username.is_empty() && !repo.is_empty() {
                return Some((username.to_string(), repo.to_string()));
            }
        }
        from += 1;
    }
    None
}

/// `githubdl.js:19-33`. `args[0]` kosong membuat `branch` tetap kosong, padahal
/// JS isi `'main'` — bedanya tidak terlihat karena `username` kosong sudah lebih
/// dulu mengembalikan kartu pakai.
pub(crate) fn parse_target<S: AsRef<str>>(args: &[S]) -> Target {
    let Some(first) = args.first().map(AsRef::as_ref) else {
        return Target::default();
    };
    // `includes('github.com')` di JS TIDAK case-insensitive; regex-nya iya.
    if first.contains("github.com") {
        let Some((username, repo)) = match_repo_url(first) else {
            return Target::default();
        };
        return Target {
            username: Some(username),
            repo: Some(repo.strip_suffix(".git").unwrap_or(&repo).to_string()),
            branch: args
                .get(1)
                .map(|s| s.as_ref().to_string())
                .unwrap_or_else(|| "main".to_string()),
        };
    }
    Target {
        username: Some(first.to_string()),
        repo: args.get(1).map(|s| s.as_ref().to_string()),
        branch: args
            .get(2)
            .map(|s| s.as_ref().to_string())
            .unwrap_or_else(|| "main".to_string()),
    }
}

/// `githubdl.js:73` — `fileName` untuk `sendMedia`.
pub(crate) fn zip_filename(repo: &str, branch: &str) -> String {
    format!("{repo} - Branch: {branch}.zip")
}

/// `res.ok` di JS = status 2xx. `HttpGet` sudah mengembalikan `Err` di atas
/// 399, tapi 3xx juga tidak `ok`.
fn is_ok(status: u16) -> bool {
    (200..300).contains(&status)
}

#[plugin(
    name = "githubdl",
    alias = ["gitdl", "gitclone", "repodownload"],
    category = "download",
    description = "Download repository GitHub sebagai ZIP",
    usage = ".githubdl <user> <repo> <branch>",
    example = ".githubdl niceplugin NiceBot main",
    cooldown = 15,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn githubdl(ctx: Ctx) -> Reply {
    let target = parse_target(&ctx.args);
    // `githubdl.js:35` hanya memeriksa `username` lebih dulu; `repo` kosong
    // ditangani setelahnya supaya urutannya sama.
    let Some(username) = target.username else {
        return Reply::text(help(&ctx.prefix));
    };
    let Some(repo) = target.repo else {
        return Reply::text(NO_REPO);
    };

    let repo_url = format!("https://api.github.com/repos/{username}/{repo}");
    let Some(response) = ctx.http.get(&repo_url).ok().filter(|r| is_ok(r.status)) else {
        return Reply::text(not_found(&username, &repo)).with_reaction("❌");
    };
    // `await res.json()` melempar kalau body bukan JSON → `catch` plugin.
    let Some(repo_data) = response.json() else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };
    let default_branch = repo_data
        .get("default_branch")
        .map(nullable_branch)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "main".to_string());

    // `branch = branch || defaultBranch`; `branch` sudah `'main'` dari
    // `parse_target`, jadi `defaultBranch` praktis hanya untuk pesan error.
    let branch = if target.branch.is_empty() {
        default_branch.clone()
    } else {
        target.branch.clone()
    };
    let zip_url =
        format!("https://github.com/{username}/{repo}/archive/refs/heads/{branch}.zip");

    // `ponytail:` GET, bukan HEAD — trait `HttpGet` belum punya HEAD.
    match ctx.http.get(&zip_url) {
        Ok(r) if is_ok(r.status) => {
            Reply::text(zip_filename(&repo, &branch)).with_reaction("✅")
        }
        Ok(_) => Reply::text(bad_branch(&branch, &default_branch)).with_reaction("❌"),
        Err(_) => Reply::text(crate::common::error_template(&ctx)).with_reaction("☢"),
    }
}

/// `repoData.default_branch || 'main'` — `null`/tidak ada jadi `main`.
fn nullable_branch(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use oktz_plugin_api::HttpGet;
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        repo: Result<(u16, String), ()>,
        zip: Result<u16, ()>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn ok(zip_status: u16) -> Arc<Stub> {
            Arc::new(Stub {
                repo: Ok((
                    200,
                    serde_json::json!({ "default_branch": "develop" }).to_string(),
                )),
                zip: Ok(zip_status),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn repo_status(status: u16) -> Arc<Stub> {
            Arc::new(Stub {
                repo: Ok((status, String::new())),
                zip: Ok(200),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn repo_not_json() -> Arc<Stub> {
            Arc::new(Stub {
                repo: Ok((200, "<html>bukan json</html>".to_string())),
                zip: Ok(200),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn zip_offline() -> Arc<Stub> {
            Arc::new(Stub {
                repo: Ok((200, serde_json::json!({ "default_branch": "develop" }).to_string())),
                zip: Err(()),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let (status, text) = if url.starts_with("https://api.github.com/repos/") {
                self.repo
                    .clone()
                    .map_err(|()| oktz_plugin_api::HttpError { message: "offline".into() })?
            } else {
                (self.zip.map_err(|()| oktz_plugin_api::HttpError {
                    message: "offline".into(),
                })?, String::new())
            };
            Ok(HttpResponse {
                status,
                body: HttpBody::Bytes {
                    data: Arc::from(text.into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
    }

    fn run(http: Arc<Stub>, args: &[&str]) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "githubdl".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.join(" "),
            args: args.iter().map(|a| (*a).to_string()).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("githubdl").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("githubdl").unwrap();
        assert_eq!(p.meta.name, "githubdl");
        assert_eq!(p.meta.alias, ["gitdl", "gitclone", "repodownload"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(
            p.meta.description,
            "Download repository GitHub sebagai ZIP"
        );
        assert_eq!(p.meta.usage, ".githubdl <user> <repo> <branch>");
        assert_eq!(p.meta.example, ".githubdl niceplugin NiceBot main");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["gitdl", "gitclone", "repodownload"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "githubdl");
        }
    }

    #[test]
    fn three_bare_words_are_read_as_user_repo_branch() {
        assert_eq!(
            parse_target::<String>(&["niceplugin".into(), "NiceBot".into(), "dev".into()]),
            Target {
                username: Some("niceplugin".into()),
                repo: Some("NiceBot".into()),
                branch: "dev".into(),
            }
        );
    }

    #[test]
    fn a_url_first_argument_wins_and_the_git_suffix_is_stripped() {
        assert_eq!(
            parse_target::<String>(&["https://github.com/user/repo.git".into(), "dev".into()]),
            Target {
                username: Some("user".into()),
                repo: Some("repo".into()),
                branch: "dev".into(),
            }
        );
        // Tanpa branch, `args[1] || 'main'`.
        assert_eq!(parse_target::<String>(&["https://github.com/u/r".into()]).branch, "main");
        // `.git` hanya dilepas di akhir, `repo.git.git` jadi `repo.git`.
        assert_eq!(
            parse_target::<String>(&["github.com/u/r.git.git".into()]).repo.unwrap(),
            "r.git"
        );
    }

    #[test]
    fn a_broken_url_leaves_the_target_empty_so_the_usage_card_shows() {
        // `urlMatch` null → `username` tetap `undefined` → kartu pakai, bukan
        // pesan repository kosong.
        for args in [
            vec!["github.com/tosatu"],
            vec!["https://github.com/u"],
            vec!["github.com//r"],
            vec!["github.com/u/"],
            vec![],
        ] {
            let t = parse_target(&args);
            assert!(t.username.is_none(), "{args:?} -> {t:?}");
        }
    }

    #[test]
    fn an_empty_argument_shows_the_usage_card_without_any_request() {
        let http = Stub::ok(200);
        assert_eq!(text_of(&run(http.clone(), &[])), help("."));
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_username_without_a_repo_asks_for_the_repository() {
        let http = Stub::ok(200);
        let r = run(http.clone(), &["niceplugin"]);
        assert_eq!(text_of(&r), NO_REPO);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_resolved_branch_replies_with_the_zip_filename() {
        let http = Stub::ok(200);
        let r = run(http.clone(), &["niceplugin", "NiceBot", "main"]);
        assert_eq!(text_of(&r), "NiceBot - Branch: main.zip");
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "✅"),
            "sukses harus bereaksi ✅"
        );
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(
            seen,
            vec![
                "https://api.github.com/repos/niceplugin/NiceBot".to_string(),
                "https://github.com/niceplugin/NiceBot/archive/refs/heads/main.zip".to_string(),
            ]
        );
    }

    #[test]
    fn a_missing_repository_is_reported_with_the_user_and_repo_names() {
        for status in [404u16, 403, 500] {
            let r = run(Stub::repo_status(status), &["niceplugin", "NiceBot"]);
            assert_eq!(text_of(&r), not_found("niceplugin", "NiceBot"), "status {status}");
            assert!(
                matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "❌"),
                "repo tidak ada harus bereaksi ❌"
            );
        }
    }

    #[test]
    fn a_missing_branch_names_the_default_branch_in_the_message() {
        let r = run(Stub::ok(404), &["niceplugin", "NiceBot", "tidak-ada"]);
        assert_eq!(text_of(&r), bad_branch("tidak-ada", "develop"));
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "❌"),
            "branch tidak ada harus bereaksi ❌"
        );
    }

    #[test]
    fn a_body_that_is_not_json_reports_the_system_error() {
        let r = run(Stub::repo_not_json(), &["niceplugin", "NiceBot"]);
        assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "githubdl"));
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "☢"),
            "harus bereaksi ☢"
        );
    }

    #[test]
    fn a_transport_failure_on_the_zip_check_reports_the_system_error() {
        let r = run(Stub::zip_offline(), &["niceplugin", "NiceBot"]);
        assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "githubdl"));
    }
}