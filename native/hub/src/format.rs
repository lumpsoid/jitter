//! Labels and error messages the GUI shows as sent, formatted once here so
//! every page that shows the same value shows it the same way.

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, TimeZone};
use jitter_core::models::{ClockError, Seconds, Timestamp};
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

/// Why a typed time is not one to go to: `Type a time, like 1:23:45 or
/// 1h23m.`, or `The VOD ends at 5:12:03.`
pub fn clock_error(error: ClockError) -> String {
    match error {
        ClockError::Empty | ClockError::Malformed => {
            "Type a time, like 1:23:45 or 1h23m.".to_owned()
        }
        ClockError::PastEnd { duration } => {
            format!("The VOD ends at {}.", duration_label(duration))
        }
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

/// A sleep timer the player offers: `15 minutes`.
pub fn sleep_preset_label(minutes: u32) -> String {
    format!("{minutes} minutes")
}

/// An armed sleep timer's time left, in minutes rounded up: `23 min left`.
pub fn sleep_left_label(minutes: u32) -> String {
    format!("{minutes} min left")
}

/// The day a VOD was streamed, in the viewer's time zone: `Oct 24`, with
/// the year when it is not this one: `Dec 31, 2025`. `None` outside the
/// dates chrono can hold.
pub fn streamed_label(at: Timestamp) -> Option<String> {
    let local = Local.timestamp_opt(at.unix(), 0).single()?;
    Some(date_label(local.date_naive(), Local::now().year()))
}

/// [`streamed_label`] once the day is known: `Oct 24` in `this_year`,
/// `Dec 31, 2025` otherwise.
fn date_label(date: NaiveDate, this_year: i32) -> String {
    let pattern = if date.year() == this_year {
        "%b %-d"
    } else {
        "%b %-d, %Y"
    };
    date.format(pattern).to_string()
}

/// `at` on the viewer's clock; `None` outside the dates chrono can hold.
pub fn local_time(at: Timestamp) -> Option<NaiveDateTime> {
    Some(Local.timestamp_opt(at.unix(), 0).single()?.naive_local())
}

/// The viewer's today.
pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

/// A day of the history: `Today`, `Yesterday`, or the weekday and date,
/// `Mon, Oct 24`, with the year when it is not this one:
/// `Wed, Dec 31, 2025`.
pub fn day_label(date: NaiveDate, today: NaiveDate) -> String {
    if date == today {
        return "Today".to_owned();
    }
    if today.pred_opt() == Some(date) {
        return "Yesterday".to_owned();
    }
    format!("{}, {}", date.format("%a"), date_label(date, today.year()))
}

/// A time of day on a 24-hour clock: `14:02`.
pub fn clock_label(at: NaiveDateTime) -> String {
    at.format("%H:%M").to_string()
}

/// A day and its time: `Today, 14:02`, `Mon, Oct 24, 09:30`.
pub fn day_time_label(at: NaiveDateTime, today: NaiveDate) -> String {
    format!("{}, {}", day_label(at.date(), today), clock_label(at))
}

/// How long a visit was watched, to the minute: `<1m` in its first
/// minute, then as [`uptime_label`]: `40m`, `1h 5m`.
pub fn watched_label(watched: Seconds) -> String {
    match watched.get() {
        0..60 => "<1m".to_owned(),
        _ => uptime_label(watched),
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
    use chrono::NaiveDate;
    use jitter_core::models::{ClockError, Seconds};
    use jitter_twitch::TwitchError;

    use super::{
        clock_error, clock_label, date_label, day_label, day_time_label, duration_label,
        followers_label, twitch_error, uptime_label, viewers_label, watched_label,
    };

    #[test]
    fn a_typed_time_says_what_to_type_or_where_the_vod_ends() {
        assert_eq!(
            clock_error(ClockError::Malformed),
            "Type a time, like 1:23:45 or 1h23m."
        );
        assert_eq!(
            clock_error(ClockError::PastEnd {
                duration: Seconds::new(18723)
            }),
            "The VOD ends at 5:12:03."
        );
    }

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

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        let Some(date) = NaiveDate::from_ymd_opt(year, month, day) else {
            unreachable!("valid date");
        };
        date
    }

    #[test]
    fn history_days_read_today_yesterday_then_the_date() {
        let today = day(2026, 10, 28);
        assert_eq!(day_label(today, today), "Today");
        assert_eq!(day_label(day(2026, 10, 27), today), "Yesterday");
        assert_eq!(day_label(day(2026, 10, 24), today), "Sat, Oct 24");
        assert_eq!(day_label(day(2025, 12, 31), today), "Wed, Dec 31, 2025");
        assert_eq!(day_label(day(2026, 1, 1), day(2026, 1, 1)), "Today");
        assert_eq!(day_label(day(2025, 12, 31), day(2026, 1, 1)), "Yesterday");
    }

    #[test]
    fn history_times_read_on_a_24_hour_clock() {
        let Some(at) = day(2026, 10, 27).and_hms_opt(9, 5, 59) else {
            unreachable!("valid time");
        };
        assert_eq!(clock_label(at), "09:05");
        assert_eq!(day_time_label(at, day(2026, 10, 28)), "Yesterday, 09:05");
    }

    #[test]
    fn watched_time_reads_to_the_minute() {
        assert_eq!(watched_label(Seconds::new(10)), "<1m");
        assert_eq!(watched_label(Seconds::new(59)), "<1m");
        assert_eq!(watched_label(Seconds::new(60)), "1m");
        assert_eq!(watched_label(Seconds::new(3_900)), "1h 5m");
    }

    #[test]
    fn dates_this_year_drop_the_year() {
        assert_eq!(date_label(day(2026, 10, 24), 2026), "Oct 24");
        assert_eq!(date_label(day(2026, 1, 5), 2026), "Jan 5");
    }

    #[test]
    fn dates_of_other_years_show_it() {
        assert_eq!(date_label(day(2025, 12, 31), 2026), "Dec 31, 2025");
        assert_eq!(date_label(day(2027, 3, 1), 2026), "Mar 1, 2027");
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
