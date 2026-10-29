use std::sync::Once;

static INIT: Once = Once::new();

pub fn init(level: &str) {
    INIT.call_once(|| {
        let filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));
        let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_idempotent() {
        init("info");
        init("debug");
        init("trace");
    }
}
