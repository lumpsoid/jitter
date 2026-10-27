//! Labels the pages show, worded like the Flutter head's.

/// A viewer count, compact: `999 viewers`, `12.3K viewers`, `1 viewer`.
pub fn viewers_label(viewers: u32) -> String {
    let noun = if viewers == 1 { "viewer" } else { "viewers" };
    format!("{} {noun}", compact(viewers))
}

fn compact(count: u32) -> String {
    match count {
        0..1_000 => count.to_string(),
        1_000..1_000_000 => tenths(count / 100, "K"),
        _ => tenths(count / 100_000, "M"),
    }
}

fn tenths(tenths: u32, suffix: &str) -> String {
    match tenths % 10 {
        0 => format!("{}{suffix}", tenths / 10),
        rest => format!("{}.{rest}{suffix}", tenths / 10),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewers_are_compact() {
        assert_eq!(viewers_label(1), "1 viewer");
        assert_eq!(viewers_label(999), "999 viewers");
        assert_eq!(viewers_label(12_345), "12.3K viewers");
        assert_eq!(viewers_label(2_000_000), "2M viewers");
    }
}
