//! Labels the GUI shows as sent, formatted once here so every page that
//! shows the same value shows it the same way.

/// A live viewer count, compact like Twitch's own: `1 viewer`,
/// `950 viewers`, `12.3K viewers`, `1.2M viewers`. Truncated, never
/// rounded up, so 999 999 reads `999.9K`, not `1000K`.
pub fn viewers_label(viewers: u32) -> String {
    let count = match viewers {
        0..1_000 => viewers.to_string(),
        1_000..1_000_000 => tenths(viewers / 100, "K"),
        _ => tenths(viewers / 100_000, "M"),
    };
    let noun = if viewers == 1 { "viewer" } else { "viewers" };
    format!("{count} {noun}")
}

/// `123` tenths with suffix `K` is `12.3K`; a whole number drops the `.0`.
fn tenths(tenths: u32, suffix: &str) -> String {
    match tenths % 10 {
        0 => format!("{}{suffix}", tenths / 10),
        rest => format!("{}.{rest}{suffix}", tenths / 10),
    }
}

#[cfg(test)]
mod tests {
    use super::viewers_label;

    #[test]
    fn counts_below_a_thousand_are_exact() {
        assert_eq!(viewers_label(0), "0 viewers");
        assert_eq!(viewers_label(1), "1 viewer");
        assert_eq!(viewers_label(999), "999 viewers");
    }

    #[test]
    fn thousands_show_one_decimal() {
        assert_eq!(viewers_label(1_000), "1K viewers");
        assert_eq!(viewers_label(12_345), "12.3K viewers");
        assert_eq!(viewers_label(999_999), "999.9K viewers");
    }

    #[test]
    fn millions_show_one_decimal() {
        assert_eq!(viewers_label(1_000_000), "1M viewers");
        assert_eq!(viewers_label(1_250_000), "1.2M viewers");
        assert_eq!(viewers_label(u32::MAX), "4294.9M viewers");
    }
}
