//! Emote provider tests against the Phase 2 recorded fixtures. No network.
//!
//! Hand-written (not recorded): the 7TV `flags = 3` emote, a BTTV `IceCold`
//! entry (the recorded BTTV lists have none), emotes missing required
//! fields, and a 7TV user without an emote set.

use jitter_core::models::{ChannelId, Emote};
use jitter_twitch::TwitchError;
use jitter_twitch::client::{headers, third_party_headers};
use jitter_twitch::emotes::{
    TwitchEmoteScale, bttv, ffz, seventv, twitch_emote_url, twitch_still_emote_url,
};

const SEVENTV_GLOBAL: &[u8] = include_bytes!("fixtures/seventv_global.json");
const SEVENTV_CHANNEL: &[u8] = include_bytes!("fixtures/seventv_channel.json");
const BTTV_GLOBAL: &[u8] = include_bytes!("fixtures/bttv_global.json");
const BTTV_CHANNEL: &[u8] = include_bytes!("fixtures/bttv_channel.json");
const FFZ_GLOBAL: &[u8] = include_bytes!("fixtures/ffz_global.json");
const FFZ_ROOM: &[u8] = include_bytes!("fixtures/ffz_room.json");

fn ok<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(e) => unreachable!("expected Ok, got {e:?}"),
    }
}

fn find<'a>(emotes: &'a [Emote], name: &str) -> &'a Emote {
    match emotes.iter().find(|e| e.name == name) {
        Some(e) => e,
        None => unreachable!("no emote named {name}"),
    }
}

#[test]
fn third_party_requests_carry_no_twitch_headers() {
    let map = third_party_headers();
    assert!(map.is_empty());
    assert!(!map.contains_key("client-id"));
    assert!(!map.contains_key("x-device-id"));
    assert!(!map.contains_key("authorization"));
    // Twitch's own builder does carry the id, so the test is not vacuous.
    assert!(ok(headers(None)).contains_key("client-id"));
}

#[test]
fn seventv_global_maps_ids_names_urls_and_flags() {
    let emotes = ok(seventv::parse_global(SEVENTV_GLOBAL));
    assert_eq!(emotes.len(), 4);
    let petpet = find(&emotes, "PETPET");
    assert_eq!(petpet.id, "01FSNK0GP000038GDBZ91HT6SQ");
    assert_eq!(
        petpet.url,
        "https://cdn.7tv.app/emote/01FSNK0GP000038GDBZ91HT6SQ/2x.webp"
    );
    assert!(petpet.zero_width);
    assert!(!find(&emotes, "reckH").zero_width);
}

#[test]
fn seventv_channel_reads_the_emote_set() {
    let emotes = ok(seventv::parse_channel(SEVENTV_CHANNEL));
    assert_eq!(emotes.len(), 4);
    assert!(find(&emotes, "IceCold").zero_width);
    assert!(!find(&emotes, "GAMBA").zero_width);
}

#[test]
fn seventv_flags_are_tested_by_bit() {
    let body = br#"{"emotes":[
        {"id":"a","name":"Both","flags":3,"data":{"host":{"url":"//cdn.7tv.app/emote/a"}}},
        {"id":"b","name":"Other","flags":2,"data":{"host":{"url":"//cdn.7tv.app/emote/b"}}},
        {"id":"c","name":"NoFlags","data":{"host":{"url":"//cdn.7tv.app/emote/c"}}}
    ]}"#;
    let emotes = ok(seventv::parse_global(body));
    assert!(find(&emotes, "Both").zero_width);
    assert!(!find(&emotes, "Other").zero_width);
    assert!(!find(&emotes, "NoFlags").zero_width);
}

#[test]
fn seventv_drops_emotes_missing_required_data() {
    let body = br#"{"emotes":[
        {"id":"a","name":"NoData","flags":0},
        {"id":"b","name":"NoHost","flags":0,"data":{}},
        {"name":"NoId","flags":0,"data":{"host":{"url":"//h/x"}}},
        {"id":"d","name":"Good","flags":0,"data":{"host":{"url":"//h/d"}}}
    ]}"#;
    let emotes = ok(seventv::parse_global(body));
    assert_eq!(emotes.len(), 1);
    assert_eq!(emotes[0].name, "Good");
}

#[test]
fn seventv_user_without_an_emote_set_has_no_emotes() {
    assert!(ok(seventv::parse_channel(br#"{"emote_set":null}"#)).is_empty());
}

#[test]
fn bttv_global_flags_the_zero_width_names() {
    let emotes = ok(bttv::parse_global(BTTV_GLOBAL));
    let hazmat = find(&emotes, "cvHazmat");
    assert!(hazmat.zero_width);
    assert!(find(&emotes, "cvMask").zero_width);
    assert!(!find(&emotes, "KKona").zero_width);
    assert_eq!(
        hazmat.url,
        format!("https://cdn.betterttv.net/emote/{}/2x.webp", hazmat.id)
    );
}

#[test]
fn bttv_ice_cold_is_zero_width() {
    let body = br#"[{"id":"x1","code":"IceCold","imageType":"png"},
                     {"id":"x2","code":"IceWarm","imageType":"png"},
                     {"code":"NoId"}]"#;
    let emotes = ok(bttv::parse_global(body));
    assert_eq!(emotes.len(), 2);
    assert!(find(&emotes, "IceCold").zero_width);
    assert!(!find(&emotes, "IceWarm").zero_width);
}

#[test]
fn bttv_channel_joins_channel_and_shared_emotes() {
    let emotes = ok(bttv::parse_channel(BTTV_CHANNEL));
    let names: Vec<_> = emotes.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "forsenBlunder",
            "forsenBucket",
            "forsenClown",
            "RebeccaBlack",
            "EZ",
            "gachiGASM"
        ]
    );
    assert!(emotes.iter().all(|e| !e.zero_width));
}

#[test]
fn ffz_global_reads_only_the_default_sets() {
    let emotes = ok(ffz::parse_global(FFZ_GLOBAL));
    assert!(!emotes.is_empty());
    let zrehplar = find(&emotes, "ZrehplaR");
    assert_eq!(zrehplar.id, "9");
    assert_eq!(zrehplar.url, "https://cdn.frankerfacez.com/emote/9/2");
    assert!(emotes.iter().all(|e| !e.zero_width));
}

#[test]
fn ffz_global_ignores_sets_that_are_not_default() {
    let body = br#"{"default_sets":[1],"sets":{
        "1":{"emoticons":[{"id":1,"name":"In","urls":{"1":"//cdn/1/1"}}]},
        "2":{"emoticons":[{"id":2,"name":"Out","urls":{"1":"//cdn/2/1"}}]}}}"#;
    let emotes = ok(ffz::parse_global(body));
    assert_eq!(emotes.len(), 1);
    // protocol-relative URLs are made absolute; scale 1 is the fallback.
    assert_eq!(emotes[0].url, "https://cdn/1/1");
}

#[test]
fn ffz_room_reads_the_room_set() {
    let emotes = ok(ffz::parse_room(FFZ_ROOM));
    assert_eq!(emotes.len(), 1);
    assert_eq!(emotes[0].name, "WideHard");
    assert_eq!(emotes[0].id, "246878");
    assert!(!emotes[0].zero_width);
}

#[test]
fn a_malformed_body_is_a_decode_error() {
    assert!(matches!(
        seventv::parse_global(b"<html>"),
        Err(TwitchError::Decode(_))
    ));
    assert!(matches!(
        bttv::parse_channel(b"\"text\""),
        Err(TwitchError::Decode(_))
    ));
    assert!(matches!(
        ffz::parse_room(b"{}"),
        Err(TwitchError::Decode(_))
    ));
}

#[test]
fn channel_urls_use_the_numeric_id() {
    let id = ok(ChannelId::parse("71092938"));
    assert_eq!(
        seventv::channel_url(&id),
        "https://7tv.io/v3/users/twitch/71092938"
    );
    assert_eq!(
        bttv::channel_url(&id),
        "https://api.betterttv.net/3/cached/users/twitch/71092938"
    );
    assert_eq!(
        ffz::channel_url(&id),
        "https://api.frankerfacez.com/v1/room/id/71092938"
    );
}

#[test]
fn twitch_emote_url_per_scale() {
    assert_eq!(
        twitch_emote_url("25", TwitchEmoteScale::One),
        "https://static-cdn.jtvnw.net/emoticons/v2/25/default/dark/1.0"
    );
    assert!(twitch_emote_url("25", TwitchEmoteScale::Three).ends_with("/dark/3.0"));
}

#[test]
fn twitch_still_emote_url_is_the_static_format() {
    assert_eq!(
        twitch_still_emote_url("25", TwitchEmoteScale::Two),
        "https://static-cdn.jtvnw.net/emoticons/v2/25/static/dark/2.0"
    );
}

#[test]
fn seventv_animated_emotes_carry_the_hosts_still_and_static_ones_none() {
    let emotes = ok(seventv::parse_global(SEVENTV_GLOBAL));
    assert_eq!(
        find(&emotes, "PETPET").still_url.as_deref(),
        Some("https://cdn.7tv.app/emote/01FSNK0GP000038GDBZ91HT6SQ/2x_static.webp")
    );
    assert_eq!(find(&emotes, "reckH").still_url, None);
}

#[test]
fn seventv_without_a_file_list_has_no_still() {
    let body = br#"{"emotes":[
        {"id":"a","name":"Bare","data":{"host":{"url":"//cdn.7tv.app/emote/a"}}}
    ]}"#;
    let emotes = ok(seventv::parse_global(body));
    assert_eq!(find(&emotes, "Bare").still_url, None);
}

#[test]
fn bttv_animated_emotes_have_a_static_still() {
    let emotes = ok(bttv::parse_global(BTTV_GLOBAL));
    assert_eq!(
        find(&emotes, "SourPls").still_url.as_deref(),
        Some("https://cdn.betterttv.net/emote/566ca38765dbbdab32ec0560/static/2x.webp")
    );
    assert_eq!(find(&emotes, "CiGrip").still_url, None);
}

#[test]
fn ffz_emotes_are_already_still() {
    let emotes = ok(ffz::parse_global(FFZ_GLOBAL));
    assert!(emotes.iter().all(|e| e.still_url.is_none()));
}
