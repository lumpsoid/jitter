//! Labels and error messages the GUI shows as sent, formatted once here so
//! every page that shows the same value shows it the same way.

use jitter_core::models::Seconds;
use jitter_twitch::TwitchError;

/// What the user reads when a Twitch call failed: one sentence per
/// [`TwitchError`] variant, in the viewer's terms. twitch's own `Display`
/// stays the diagnostic text (the CLI, logs); its detail (a decode error,
/// GQL's messages) means nothing to a viewer.
pub fn twitch_error(error: &TwitchError) -> String {
    match error {
        TwitchError::Network(_) => "Couldn't reach Twitch. Check your connection.".to_owned(),
        TwitchError::Http { status } => {
            format!("Twitch answered with an error (HTTP {status}). Try again later.")
        }
        TwitchError::Decode(_) => {
            "Twitch sent a response jitter doesn't understand. jitter may need an update."
                .to_owned()
        }
        TwitchError::Gql(_) => "Twitch reported an error. Try again later.".to_owned(),
        TwitchError::IntegrityRequired => {
            "Twitch asked for an integrity check, which jitter can't pass without signing in."
                .to_owned()
        }
        TwitchError::QueryRetired { operation } => {
            format!(
                "Twitch no longer answers jitter's {operation} request. jitter needs an update."
            )
        }
        TwitchError::PlaybackForbidden => "Twitch does not allow playing this without signing in \
             (subscriber-only or region-blocked)."
            .to_owned(),
        TwitchError::Offline => "This channel is offline.".to_owned(),
        TwitchError::NotFound => "No such channel or video.".to_owned(),
    }
}

/// A live viewer count, compact like Twitch's own: `1 viewer`,
/// `950 viewers`, `12.3K viewers`, `1.2M viewers`. Truncated, never
/// rounded up, so 999 999 reads `999.9K`, not `1000K`.
pub fn viewers_label(viewers: u32) -> String {
    let noun = if viewers == 1 { "viewer" } else { "viewers" };
    format!("{} {noun}", compact(viewers))
}

/// A follower count, compact like [`viewers_label`]: `1 follower`,
/// `2.9M followers`.
pub fn followers_label(followers: u32) -> String {
    let noun = if followers == 1 {
        "follower"
    } else {
        "followers"
    };
    format!("{} {noun}", compact(followers))
}

/// `950`, `12.3K`, `1.2M`: truncated to one decimal, never rounded up.
fn compact(count: u32) -> String {
    match count {
        0..1_000 => count.to_string(),
        1_000..1_000_000 => tenths(count / 100, "K"),
        _ => tenths(count / 100_000, "M"),
    }
}

/// A video's length like a player shows it: `1:02:03`, or `4:05` under an
/// hour.
pub fn duration_label(duration: Seconds) -> String {
    let seconds = duration.get();
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    match hours {
        0 => format!("{minutes}:{seconds:02}"),
        _ => format!("{hours}:{minutes:02}:{seconds:02}"),
    }
}

/// How long a stream has been live, to the minute: `3h 12m`, or `12m`
/// under an hour (`0m` in its first minute). Hours do not roll into days.
pub fn uptime_label(uptime: Seconds) -> String {
    let minutes = uptime.get() / 60;
    match minutes / 60 {
        0 => format!("{minutes}m"),
        hours => format!("{hours}h {}m", minutes % 60),
    }
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
    use jitter_core::models::Seconds;
    use jitter_twitch::TwitchError;

    use super::{duration_label, followers_label, twitch_error, uptime_label, viewers_label};

    fn reads(error: TwitchError, message: &str) {
        assert_eq!(twitch_error(&error), message, "{error:?}");
    }

    #[test]
    fn a_network_failure_points_at_the_connection() {
        reads(
            TwitchError::Network("dns error: no such host".to_owned()),
            "Couldn't reach Twitch. Check your connection.",
        );
    }

    #[test]
    fn an_http_failure_names_the_status() {
        reads(
            TwitchError::Http { status: 503 },
            "Twitch answered with an error (HTTP 503). Try again later.",
        );
    }

    #[test]
    fn a_decode_failure_hides_the_parser_detail() {
        reads(
            TwitchError::Decode("missing field `data`".to_owned()),
            "Twitch sent a response jitter doesn't understand. jitter may need an update.",
        );
    }

    #[test]
    fn gql_errors_hide_twitchs_messages() {
        reads(
            TwitchError::Gql(vec!["service timeout".to_owned()]),
            "Twitch reported an error. Try again later.",
        );
    }

    #[test]
    fn an_integrity_check_says_why_it_cannot_pass() {
        reads(
            TwitchError::IntegrityRequired,
            "Twitch asked for an integrity check, which jitter can't pass without signing in.",
        );
    }

    #[test]
    fn a_retired_query_names_the_operation() {
        reads(
            TwitchError::QueryRetired {
                operation: "ChannelByLogin",
            },
            "Twitch no longer answers jitter's ChannelByLogin request. jitter needs an update.",
        );
    }

    #[test]
    fn forbidden_playback_names_the_likely_causes() {
        reads(
            TwitchError::PlaybackForbidden,
            "Twitch does not allow playing this without signing in \
             (subscriber-only or region-blocked).",
        );
    }

    #[test]
    fn offline_and_not_found_are_plain() {
        reads(TwitchError::Offline, "This channel is offline.");
        reads(TwitchError::NotFound, "No such channel or video.");
    }

    #[test]
    fn durations_show_hours_only_from_an_hour() {
        assert_eq!(duration_label(Seconds::new(0)), "0:00");
        assert_eq!(duration_label(Seconds::new(245)), "4:05");
        assert_eq!(duration_label(Seconds::new(3599)), "59:59");
        assert_eq!(duration_label(Seconds::new(3723)), "1:02:03");
        assert_eq!(duration_label(Seconds::new(36_000)), "10:00:00");
    }

    #[test]
    fn uptimes_read_in_hours_and_minutes() {
        assert_eq!(uptime_label(Seconds::new(0)), "0m");
        assert_eq!(uptime_label(Seconds::new(59)), "0m");
        assert_eq!(uptime_label(Seconds::new(60)), "1m");
        assert_eq!(uptime_label(Seconds::new(3599)), "59m");
        assert_eq!(uptime_label(Seconds::new(3600)), "1h 0m");
        assert_eq!(uptime_label(Seconds::new(11_520)), "3h 12m");
        assert_eq!(uptime_label(Seconds::new(100_000)), "27h 46m");
    }

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

    #[test]
    fn followers_read_like_viewers() {
        assert_eq!(followers_label(1), "1 follower");
        assert_eq!(followers_label(704), "704 followers");
        assert_eq!(followers_label(39_950), "39.9K followers");
        assert_eq!(followers_label(2_998_637), "2.9M followers");
    }
}
