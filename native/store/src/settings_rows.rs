use jitter_core::models::{QualityPreference, Setting, Settings, ThemeChoice};
use jitter_core::ports::{SettingsStore, StoreError};
use rusqlite::types::Value;

use crate::sqlite_store::{SqliteStore, bad_row, db_error};

/// The `settings` row as read: theme, animated emotes, auto-load chat,
/// start quality, third-party emotes, background audio. `NULL` is a
/// setting never stored.
type Row = (
    Option<String>,
    Option<i64>,
    Option<i64>,
    Option<String>,
    Option<i64>,
    Option<i64>,
);

/// One row, one column per setting. `put_setting` updates only its own
/// column, so it never overwrites a setting another process stored
/// meanwhile; the migration inserted the row, so nothing here inserts.
impl SettingsStore for SqliteStore {
    fn load_settings(&self) -> Result<Settings, StoreError> {
        let (
            theme,
            animated_emotes,
            auto_load_chat,
            start_quality,
            third_party_emotes,
            background_audio,
        ): Row = self
            .conn()?
            .query_row(
                "SELECT theme, animated_emotes, auto_load_chat, start_quality, third_party_emotes,
                        background_audio
                 FROM settings WHERE id = 1",
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .map_err(db_error)?;
        let defaults = Settings::default();
        Ok(Settings {
            theme: named("theme", theme, ThemeChoice::from_name)?.unwrap_or(defaults.theme),
            animated_emotes: flag("animated_emotes", animated_emotes)?
                .unwrap_or(defaults.animated_emotes),
            auto_load_chat: flag("auto_load_chat", auto_load_chat)?
                .unwrap_or(defaults.auto_load_chat),
            start_quality: named("start_quality", start_quality, QualityPreference::from_name)?
                .unwrap_or(defaults.start_quality),
            third_party_emotes: flag("third_party_emotes", third_party_emotes)?
                .unwrap_or(defaults.third_party_emotes),
            background_audio: flag("background_audio", background_audio)?
                .unwrap_or(defaults.background_audio),
        })
    }

    fn put_setting(&mut self, setting: Setting) -> Result<(), StoreError> {
        // Column names come from this match only, never from input.
        let (column, value) = match setting {
            Setting::Theme(theme) => ("theme", Value::from(theme.name().to_owned())),
            Setting::AnimatedEmotes(on) => ("animated_emotes", Value::from(on)),
            Setting::AutoLoadChat(on) => ("auto_load_chat", Value::from(on)),
            Setting::StartQuality(quality) => {
                ("start_quality", Value::from(quality.name().to_owned()))
            }
            Setting::ThirdPartyEmotes(on) => ("third_party_emotes", Value::from(on)),
            Setting::BackgroundAudio(on) => ("background_audio", Value::from(on)),
        };
        self.conn()?
            .execute(
                &format!("UPDATE settings SET {column} = ?1 WHERE id = 1"),
                [value],
            )
            .map(drop)
            .map_err(db_error)
    }
}

/// A stored enum by its core name; `None` when never stored.
fn named<T>(
    column: &str,
    value: Option<String>,
    parse: fn(&str) -> Option<T>,
) -> Result<Option<T>, StoreError> {
    value
        .map(|name| parse(&name).ok_or_else(|| bad_row("settings", column, &name)))
        .transpose()
}

/// A stored bool, 0 or 1; `None` when never stored.
fn flag(column: &str, value: Option<i64>) -> Result<Option<bool>, StoreError> {
    value
        .map(|v| match v {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(bad_row("settings", column, other)),
        })
        .transpose()
}
