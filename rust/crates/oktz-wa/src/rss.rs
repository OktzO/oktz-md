// Baca RSS proses dari /proc/self/status (VmRSS).

pub fn current_kb() -> u64 {
    let Ok(s) = std::fs::read_to_string("/proc/self/status") else {
        return 0;
    };
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb: String = rest
                .trim()
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            return kb.parse().unwrap_or(0);
        }
    }
    0
}
