//! GQL client tests against the Phase 2 recorded fixtures. No network.
//!
//! Bodies Twitch did not produce during recording are hand-written here and
//! marked as such: the `failed integrity check` and generic error bodies,
//! the browse edges with a null broadcaster and a null game, and the
//! channel lookup's `description` and `followers` (added to the recorded
//! user, and the empty and missing ones).

use jitter_core::models::{
    CategoryId, ChannelId, ChannelLogin, Cursor, SearchQuery, Seconds, Timestamp, VideoId,
    VideoKind,
};
use jitter_twitch::TwitchError;
use jitter_twitch::client::{CLIENT_ID, headers};
use jitter_twitch::gql::users_stream::{LIVE_STATUS_BATCH, batch_variables};
use jitter_twitch::gql::{
    browse_directories, browse_popular, channel_by_login, channel_videos, decode, game_streams,
    persisted_body, playback_details, query_body, search_channels, search_games, users_stream,
};

const BROWSE: &[u8] = include_bytes!("fixtures/gql_browse_popular.json");
// Hand-written after the `BrowsePage_AllDirectories` response shape and the
// raw `GameStreams` document: not recorded.
const DIRECTORIES: &[u8] = include_bytes!("fixtures/gql_browse_directories.json");
const GAME_STREAMS: &[u8] = include_bytes!("fixtures/gql_game_streams.json");
// Hand-written after the raw `SearchChannels` and `SearchGames` documents'
// live answers (checked with `jitter search` on 2026-10-27), with a dropped
// user, a null node and a category without box art added: not recorded.
const SEARCH_CHANNELS: &[u8] = include_bytes!("fixtures/gql_search_channels.json");
const SEARCH_GAMES: &[u8] = include_bytes!("fixtures/gql_search_games.json");
const VIDEOS: &[u8] = include_bytes!("fixtures/gql_channel_videos.json");
const USERS_STREAM: &[u8] = include_bytes!("fixtures/gql_users_stream.json");
const CHANNEL: &[u8] = include_bytes!("fixtures/gql_channel_by_login.json");
const CHANNEL_UNKNOWN: &[u8] = include_bytes!("fixtures/gql_channel_by_login_unknown.json");
// Hand-written after the raw `PlaybackDetails*` documents' live answers
// (checked with `jitter play` on 2026-10-27; VOD 2909822254 is in Just
// Chatting and was streamed 2026-10-24 20:50 UTC, seconds not shown), with
// `users_stream`'s recorded game: not recorded.
const DETAILS_LIVE: &[u8] = include_bytes!("fixtures/gql_playback_details_live.json");
const DETAILS_VOD: &[u8] = include_bytes!("fixtures/gql_playback_details_vod.json");
const BOGUS_HASH: &[u8] = include_bytes!("fixtures/gql_persisted_bogus_hash.json");

fn ok<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(e) => unreachable!("expected Ok, got {e:?}"),
    }
}

fn login(raw: &str) -> ChannelLogin {
    ok(ChannelLogin::parse(raw))
}

// --- headers ---------------------------------------------------------------

#[test]
fn headers_without_device_id_carry_only_client_id() {
    let map = ok(headers(None));
    assert_eq!(map.len(), 1);
    assert_eq!(
        map.get("client-id").and_then(|v| v.to_str().ok()),
        Some(CLIENT_ID)
    );
    assert!(map.get("authorization").is_none());
}

#[test]
fn headers_with_device_id_add_x_device_id() {
    let map = ok(headers(Some("0123456789abcdef0123456789abcdef")));
    assert_eq!(map.len(), 2);
    assert_eq!(
        map.get("client-id").and_then(|v| v.to_str().ok()),
        Some(CLIENT_ID)
    );
    assert_eq!(
        map.get("x-device-id").and_then(|v| v.to_str().ok()),
        Some("0123456789abcdef0123456789abcdef")
    );
    assert!(map.get("authorization").is_none());
}

// --- request bodies --------------------------------------------------------

#[test]
fn live_status_batches_250_ids_into_100_100_50() {
    let ids: Vec<ChannelId> = (1..=250)
        .map(|n| ok(ChannelId::parse(&n.to_string())))
        .collect();
    let batches = batch_variables(&ids);
    let sizes: Vec<usize> = batches
        .iter()
        .map(|v| v["ids"].as_array().map_or(0, Vec::len))
        .collect();
    assert_eq!(LIVE_STATUS_BATCH, 100);
    assert_eq!(sizes, vec![100, 100, 50]);
    assert_eq!(batches[2]["ids"][0], "201");
    assert!(batch_variables(&[]).is_empty());
}

#[test]
fn persisted_body_has_the_web_client_shape() {
    let body = persisted_body(&browse_popular::QUERY, browse_popular::variables(None));
    assert_eq!(body["operationName"], "BrowsePage_Popular");
    assert_eq!(body["extensions"]["persistedQuery"]["version"], 1);
    assert_eq!(
        body["extensions"]["persistedQuery"]["sha256Hash"],
        browse_popular::QUERY.hash
    );
    let vars = &body["variables"];
    assert!(vars["cursor"].is_null());
    assert_eq!(vars["options"]["sort"], "VIEWER_COUNT");
    assert_eq!(vars["sortTypeIsRecency"], false);
}

#[test]
fn channel_videos_variables_name_the_broadcast_type() {
    let cursor = Cursor::new("c1");
    let vars = channel_videos::variables(&login("xqc"), VideoKind::Highlight, Some(&cursor));
    assert_eq!(vars["broadcastType"], "HIGHLIGHT");
    assert_eq!(vars["channelOwnerLogin"], "xqc");
    assert_eq!(vars["cursor"], "c1");
    assert_eq!(vars["videoSort"], "TIME");
    assert_eq!(vars["includePreviewBlur"], false);
}

// --- error classification --------------------------------------------------

#[test]
fn bogus_hash_is_query_retired_for_the_requested_operation() {
    assert_eq!(
        browse_popular::parse(BOGUS_HASH),
        Err(TwitchError::QueryRetired {
            operation: "BrowsePage_Popular"
        })
    );
    assert_eq!(
        channel_videos::parse(BOGUS_HASH, VideoKind::Archive),
        Err(TwitchError::QueryRetired {
            operation: "FilterableVideoTower_Videos"
        })
    );
}

#[test]
fn failed_integrity_check_is_integrity_required() {
    // Hand-written: not observed anonymously in Phase 2.
    let body = br#"{"errors":[{"message":"failed integrity check","path":[]}],"data":null}"#;
    assert_eq!(
        browse_popular::parse(body),
        Err(TwitchError::IntegrityRequired)
    );
}

#[test]
fn other_errors_are_gql_even_with_data() {
    // Hand-written: errors alongside data must not be read as a result.
    let body = br#"{"errors":[{"message":"service timeout"}],"data":{"users":[]}}"#;
    assert_eq!(
        users_stream::parse(body),
        Err(TwitchError::Gql(vec!["service timeout".into()]))
    );
}

#[test]
fn no_data_and_no_errors_is_decode() {
    let result = decode::<serde_json::Value>(br#"{"extensions":{}}"#, "X");
    assert!(matches!(result, Err(TwitchError::Decode(_))));
}

// --- BrowsePage_Popular ----------------------------------------------------

#[test]
fn browse_maps_streams_without_start_time() {
    let page = ok(browse_popular::parse(BROWSE));
    assert_eq!(page.items.len(), 5);
    let first = &page.items[0];
    assert_eq!(first.id, "318143733475");
    assert_eq!(first.channel.login.as_str(), "caedrel");
    assert_eq!(first.channel.display_name, "Caedrel");
    let game = ok(first.game.as_ref().ok_or("no game"));
    assert_eq!(game.id.as_str(), "21779");
    assert_eq!(game.name, "League of Legends");
    assert_eq!(first.viewers, 248965);
    assert_eq!(first.started_at, None);
    // A fixed-size URL: no template to fill.
    assert_eq!(
        first.thumbnail(320, 180),
        "https://static-cdn.jtvnw.net/previews-ttv/live_user_caedrel-440x248.jpg"
    );
    assert_eq!(
        page.next.as_ref().map(Cursor::as_str),
        Some("eyJzIjo0NzU0NC41NDk1MjcxODU2MzQsImQiOmZhbHNlLCJ0Ijp0cnVlfQ==")
    );
}

#[test]
fn browse_drops_null_broadcaster_and_keeps_null_game() {
    // Hand-written: the recorded page has neither a null broadcaster nor a
    // null game.
    let body = br#"{"data":{"streams":{"edges":[
        {"cursor":"a","node":{"id":"1","title":"t","viewersCount":5,
          "previewImageURL":"https://p/1.jpg","broadcaster":null,"game":null}},
        {"cursor":"b","node":{"id":"2","title":"t","viewersCount":4,
          "previewImageURL":"https://p/2.jpg",
          "broadcaster":{"id":"7","login":"abc","displayName":"Abc","profileImageURL":"https://a"},
          "game":null}}
      ],"pageInfo":{"hasNextPage":false}}}}"#;
    let page = ok(browse_popular::parse(body));
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, "2");
    assert_eq!(page.items[0].game, None);
    assert_eq!(page.next, None);
}

// --- FilterableVideoTower_Videos -------------------------------------------

#[test]
fn videos_map_duration_published_at_and_requested_kind() {
    let page = ok(channel_videos::parse(VIDEOS, VideoKind::Highlight));
    assert_eq!(page.items.len(), 5);
    let second = &page.items[1];
    assert_eq!(second.id.as_str(), "2909822254");
    assert_eq!(second.duration, Seconds::new(42700));
    assert_eq!(second.published_at, Timestamp::from_unix(1792875046));
    assert_eq!(second.channel.login.as_str(), "xqc");
    assert!(page.items.iter().all(|v| v.kind == VideoKind::Highlight));
    assert!(page.next.is_some());
}

#[test]
fn videos_for_unknown_login_are_not_found() {
    // Same `"user": null` shape as the recorded unknown-login lookup.
    assert_eq!(
        channel_videos::parse(br#"{"data":{"user":null}}"#, VideoKind::Archive),
        Err(TwitchError::NotFound)
    );
}

// --- users stream (live status) --------------------------------------------

#[test]
fn users_stream_yields_only_the_live_id() {
    let streams = ok(users_stream::parse(USERS_STREAM));
    assert_eq!(streams.len(), 1);
    let live = &streams[0];
    assert_eq!(live.channel.id.as_str(), "71092938");
    assert_eq!(live.viewers, 22239);
    assert_eq!(live.started_at, Some(Timestamp::from_unix(1793048367)));
    let game = ok(live.game.as_ref().ok_or("no game"));
    assert_eq!(game.id.as_str(), "32982");
    assert_eq!(game.name, "Grand Theft Auto V");
    assert_eq!(
        live.thumbnail(320, 180),
        "https://static-cdn.jtvnw.net/previews-ttv/live_user_xqc-320x180.jpg"
    );
}

// --- playback details ------------------------------------------------------

#[test]
fn playback_details_ask_by_login_and_by_id() {
    assert_eq!(
        playback_details::live_variables(&login("xqc"))["login"],
        "xqc"
    );
    let Ok(id) = VideoId::parse("2909822254") else {
        unreachable!("valid id");
    };
    assert_eq!(playback_details::vod_variables(&id)["id"], "2909822254");
}

#[test]
fn live_details_carry_the_stream_game() {
    let details = ok(playback_details::parse_live(DETAILS_LIVE));
    let game = ok(details.game.ok_or("no game"));
    assert_eq!(game.id.as_str(), "32982");
    assert_eq!(game.name, "Grand Theft Auto V");
}

#[test]
fn vod_details_carry_the_video_game() {
    let details = ok(playback_details::parse_vod(DETAILS_VOD));
    let game = ok(details.game.ok_or("no game"));
    assert_eq!(game.id.as_str(), "509658");
    assert_eq!(game.name, "Just Chatting");
}

#[test]
fn vod_details_carry_when_it_was_streamed() {
    let details = ok(playback_details::parse_vod(DETAILS_VOD));
    // 2026-10-24T20:50:00Z
    assert_eq!(
        details.streamed_at,
        Some(Timestamp::from_unix(1_792_875_000))
    );
}

#[test]
fn live_details_and_undated_vods_have_no_streamed_date() {
    assert_eq!(
        ok(playback_details::parse_live(DETAILS_LIVE)).streamed_at,
        None
    );
    let undated = br#"{"data":{"video":{"game":null}}}"#;
    assert_eq!(ok(playback_details::parse_vod(undated)).streamed_at, None);
    let unreadable = br#"{"data":{"video":{"createdAt":"yesterday","game":null}}}"#;
    assert_eq!(
        ok(playback_details::parse_vod(unreadable)).streamed_at,
        None
    );
}

#[test]
fn offline_channel_and_gameless_vod_have_no_game() {
    let offline = br#"{"data":{"user":{"stream":null}}}"#;
    assert_eq!(ok(playback_details::parse_live(offline)).game, None);
    let no_game = br#"{"data":{"video":{"game":null}}}"#;
    assert_eq!(ok(playback_details::parse_vod(no_game)).game, None);
}

#[test]
fn details_of_an_unknown_login_or_vod_are_not_found() {
    let unknown = br#"{"data":{"user":null}}"#;
    assert!(matches!(
        playback_details::parse_live(unknown),
        Err(TwitchError::NotFound)
    ));
    let unknown = br#"{"data":{"video":null}}"#;
    assert!(matches!(
        playback_details::parse_vod(unknown),
        Err(TwitchError::NotFound)
    ));
}

// --- channel lookup --------------------------------------------------------

#[test]
fn channel_lookup_maps_the_user() {
    let profile = ok(channel_by_login::parse(CHANNEL));
    let channel = profile.channel;
    assert_eq!(channel.id.as_str(), "71092938");
    assert_eq!(channel.login.as_str(), "xqc");
    assert_eq!(channel.display_name, "xQc");
    assert_eq!(
        profile.description.as_deref(),
        Some("THE BEST AT ABSOLUTELY EVERYTHING. THE JUICER. LEADER OF THE JUICER SQUAD.")
    );
    assert_eq!(profile.followers, Some(12_345_678));
}

#[test]
fn channel_lookup_asks_for_the_description_and_followers() {
    let document = channel_by_login::QUERY.document;
    assert!(document.contains(" description "));
    assert!(document.contains(" followers { totalCount } "));
}

#[test]
fn a_missing_follower_count_is_none() {
    let missing = channel_body("");
    assert_eq!(
        ok(channel_by_login::parse(missing.as_bytes())).followers,
        None
    );
}

/// Hand-written: the recorded user with the empty `description` Twitch
/// sends for a channel without one, and with none at all.
fn channel_body(description: &str) -> String {
    format!(
        r#"{{"data":{{"user":{{"id":"71092938","login":"xqc","displayName":"xQc",
            "profileImageURL":"https://cdn/xqc.jpeg"{description}}}}}}}"#
    )
}

#[test]
fn an_empty_description_is_none() {
    let empty = channel_body(r#","description":"""#);
    assert_eq!(
        ok(channel_by_login::parse(empty.as_bytes())).description,
        None
    );
    let blank = channel_body(r#","description":"  ""#);
    assert_eq!(
        ok(channel_by_login::parse(blank.as_bytes())).description,
        None
    );
    let missing = channel_body("");
    assert_eq!(
        ok(channel_by_login::parse(missing.as_bytes())).description,
        None
    );
}

#[test]
fn channel_lookup_of_unknown_login_is_not_found() {
    assert_eq!(
        channel_by_login::parse(CHANNEL_UNKNOWN),
        Err(TwitchError::NotFound)
    );
}

// --- BrowsePage_AllDirectories (top categories) -----------------------------

#[test]
fn directories_body_has_the_persisted_shape() {
    let body = persisted_body(
        &browse_directories::QUERY,
        browse_directories::variables(Some(&Cursor::new("MQ=="))),
    );
    assert_eq!(body["operationName"], "BrowsePage_AllDirectories");
    assert_eq!(
        body["extensions"]["persistedQuery"]["sha256Hash"],
        "2f67f71ba89f3c0ed26a141ec00da1defecb2303595f5cda4298169549783d9e"
    );
    assert_eq!(body["variables"]["cursor"], "MQ==");
    assert_eq!(body["variables"]["options"]["sort"], "VIEWER_COUNT");
}

#[test]
fn directories_map_categories_and_drop_ones_without_box_art() {
    let page = ok(browse_directories::parse(DIRECTORIES));
    let names: Vec<&str> = page.items.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        ["Just Chatting", "League of Legends", "Grand Theft Auto V"]
    );
    let first = &page.items[0];
    assert_eq!(first.id.as_str(), "509658");
    assert_eq!(first.viewers, 412345);
    assert_eq!(
        first.box_art(285, 380),
        "https://static-cdn.jtvnw.net/ttv-boxart/509658-285x380.jpg"
    );
    // The dropped Elden Ring edge is not the last, so the cursor is NA==.
    assert_eq!(page.next.as_ref().map(Cursor::as_str), Some("NA=="));
}

#[test]
fn directories_end_when_twitch_says_so() {
    let body = br#"{"data":{"directoriesWithTags":{"edges":[
        {"cursor":"a","node":{"id":"1","displayName":"A","avatarURL":"https://b/1-1x1.jpg","viewersCount":1}}
      ],"pageInfo":{"hasNextPage":false}}}}"#;
    let page = ok(browse_directories::parse(body));
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.next, None);
}

#[test]
fn a_retired_directories_hash_names_its_operation() {
    assert_eq!(
        browse_directories::parse(BOGUS_HASH),
        Err(TwitchError::QueryRetired {
            operation: "BrowsePage_AllDirectories"
        })
    );
}

// --- GameStreams (a category's streams) -------------------------------------

#[test]
fn game_streams_body_asks_by_id_and_viewer_count() {
    let id = ok(CategoryId::parse("512953"));
    let body = query_body(&game_streams::QUERY, game_streams::variables(&id, None));
    assert_eq!(body["variables"]["id"], "512953");
    assert_eq!(body["variables"]["sort"], "VIEWER_COUNT");
    assert_eq!(body["variables"]["after"], serde_json::Value::Null);
}

#[test]
fn game_streams_place_each_stream_in_the_game() {
    let page = ok(game_streams::parse(GAME_STREAMS));
    let logins: Vec<&str> = page
        .items
        .iter()
        .map(|s| s.channel.login.as_str())
        .collect();
    assert_eq!(logins, ["vinesauce", "lirik"]);
    let first = &page.items[0];
    let game = ok(first.game.as_ref().ok_or("no game"));
    assert_eq!(game.id.as_str(), "512953");
    assert_eq!(game.name, "Elden Ring");
    assert_eq!(first.viewers, 9000);
    assert_eq!(first.started_at, Some(Timestamp::from_unix(1793037600)));
    assert_eq!(
        first.thumbnail(440, 248),
        "https://static-cdn.jtvnw.net/previews-ttv/live_user_vinesauce-440x248.jpg"
    );
    assert_eq!(
        page.next.as_ref().map(Cursor::as_str),
        Some("eyJzIjozMDAwfQ==")
    );
}

#[test]
fn game_streams_of_an_unknown_id_are_not_found() {
    assert_eq!(
        game_streams::parse(br#"{"data":{"game":null}}"#),
        Err(TwitchError::NotFound)
    );
}

// --- SearchChannels -----------------------------------------------------------

#[test]
fn search_channels_body_is_the_raw_document_with_the_query() {
    let query = ok(SearchQuery::parse(" lirik "));
    let body = query_body(
        &search_channels::QUERY,
        search_channels::variables(&query, Some(&Cursor::new("eyJzIjoxfQ=="))),
    );
    assert!(
        body["query"]
            .as_str()
            .is_some_and(|q| q.contains("searchUsers(userQuery: $query"))
    );
    assert_eq!(body["variables"]["query"], "lirik");
    assert_eq!(body["variables"]["first"], 30);
    assert_eq!(body["variables"]["after"], "eyJzIjoxfQ==");
    assert!(body.get("extensions").is_none());
}

#[test]
fn search_channels_map_users_with_followers_and_live_viewers() {
    let page = ok(search_channels::parse(SEARCH_CHANNELS));
    let logins: Vec<&str> = page
        .items
        .iter()
        .map(|c| c.channel.login.as_str())
        .collect();
    // `lirikiwi` has no avatar and the last edge no node: both dropped.
    assert_eq!(logins, ["lirik", "lirik_247", "lirikk"]);

    let lirik = &page.items[0];
    assert_eq!(lirik.channel.id.as_str(), "23161357");
    assert_eq!(lirik.channel.display_name, "LIRIK");
    assert_eq!(lirik.followers, Some(2_998_637));
    assert_eq!(lirik.viewers, None);

    assert_eq!(page.items[1].viewers, Some(513));
    assert_eq!(page.items[2].followers, None);
    assert_eq!(page.next.as_ref().map(Cursor::as_str), Some("eyJzIjo1fQ=="));
}

#[test]
fn search_channels_end_when_twitch_says_so() {
    let body = br#"{"data":{"searchUsers":{"edges":[
        {"cursor":"a","node":{"id":"1","login":"a","displayName":"A","profileImageURL":"https://p/a.png"}}
      ],"pageInfo":{"hasNextPage":false}}}}"#;
    let page = ok(search_channels::parse(body));
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].followers, None);
    assert_eq!(page.items[0].viewers, None);
    assert_eq!(page.next, None);
}

#[test]
fn search_channels_without_results_is_an_empty_last_page() {
    let page = ok(search_channels::parse(
        br#"{"data":{"searchUsers":{"edges":[],"pageInfo":{"hasNextPage":false}}}}"#,
    ));
    assert!(page.items.is_empty());
    assert_eq!(page.next, None);
}

#[test]
fn search_channels_null_connection_is_a_decode_error() {
    assert!(matches!(
        search_channels::parse(br#"{"data":{"searchUsers":null}}"#),
        Err(TwitchError::Decode(_))
    ));
}

// --- SearchGames --------------------------------------------------------------

#[test]
fn search_games_body_is_the_raw_document_with_the_query() {
    let query = ok(SearchQuery::parse("elden ring"));
    let body = query_body(&search_games::QUERY, search_games::variables(&query, None));
    assert!(
        body["query"]
            .as_str()
            .is_some_and(|q| q.contains("searchCategories(query: $query"))
    );
    assert_eq!(body["variables"]["query"], "elden ring");
    assert_eq!(body["variables"]["after"], serde_json::Value::Null);
}

#[test]
fn search_games_map_categories_and_count_no_viewers_as_zero() {
    let page = ok(search_games::parse(SEARCH_GAMES));
    let names: Vec<&str> = page.items.iter().map(|c| c.name.as_str()).collect();
    // `Elden Ring GB` has no box art: dropped.
    assert_eq!(names, ["ELDEN RING", "Elden Ring Demake"]);

    let first = &page.items[0];
    assert_eq!(first.id.as_str(), "512953");
    assert_eq!(first.viewers, 2501);
    assert_eq!(
        first.box_art(285, 380),
        "https://static-cdn.jtvnw.net/ttv-boxart/512953-285x380.jpg"
    );
    assert_eq!(page.items[1].viewers, 0);
    assert_eq!(page.next, None);
}

#[test]
fn search_games_page_on_while_twitch_has_more() {
    let body = br#"{"data":{"searchCategories":{"edges":[
        {"cursor":"a","node":{"id":"1","displayName":"A","boxArtURL":"https://b/1-52x72.jpg","viewersCount":3}}
      ],"pageInfo":{"hasNextPage":true}}}}"#;
    let page = ok(search_games::parse(body));
    assert_eq!(page.next.as_ref().map(Cursor::as_str), Some("a"));
}

#[test]
fn search_games_errors_are_classified() {
    let body = br#"{"errors":[{"message":"failed integrity check"}],"data":null}"#;
    assert_eq!(
        search_games::parse(body),
        Err(TwitchError::IntegrityRequired)
    );
}
