use oktz_core::error::{Error, Result};

/// Koneksi Turso/LibSQL. Fase 5 mengimplementasikan penuh (read+write+mirror);
/// Fase 1 hanya menjaga bentuk API supaya pemanggil tidak berubah.
pub struct TursoStore {
    pub url: String,
    pub auth_token: String,
}

impl TursoStore {
    pub fn from_env() -> Result<Self> {
        let url = oktz_core::env::Env::require("TURSO_URL")?;
        let auth_token = oktz_core::env::Env::require("TURSO_AUTH_TOKEN")?;
        Ok(Self { url, auth_token })
    }

    pub fn is_configured(&self) -> bool {
        !self.url.is_empty() && !self.auth_token.is_empty()
    }

    pub fn validate(&self) -> Result<()> {
        if self.url.is_empty() {
            return Err(Error::Config("TURSO_URL kosong".into()));
        }
        if !self.url.starts_with("libsql://") && !self.url.starts_with("https://") {
            return Err(Error::Config(format!(
                "TURSO_URL skema tak dikenal: {}",
                self.url
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_empty_url() {
        let s = TursoStore {
            url: String::new(),
            auth_token: "t".into(),
        };
        assert!(s.validate().is_err());
        assert!(!s.is_configured());
    }

    #[test]
    fn validate_rejects_unknown_scheme() {
        let s = TursoStore {
            url: "postgres://x".into(),
            auth_token: "t".into(),
        };
        assert!(s.validate().is_err());
    }

    #[test]
    fn validate_accepts_libsql() {
        let s = TursoStore {
            url: "libsql://db.example.turso.io".into(),
            auth_token: "t".into(),
        };
        assert!(s.validate().is_ok());
        assert!(s.is_configured());
    }
}
