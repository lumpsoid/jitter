use jitter_core::chat::{ChatController, Connection, MESSAGE_LIMIT};
use jitter_core::models::{
    ChannelId, ChatChange, ChatEvent, ChatMessage, Chatter, Emote, EmoteProvider, EmoteScope,
    Fragment, RoomStateUpdate, Seconds, Timestamp,
};

fn uid(raw: &str) -> ChannelId {
    let Ok(id) = ChannelId::parse(raw) else {
        unreachable!("valid id");
    };
    id
}

fn message(id: &str, user: &str, text: &str) -> ChatMessage {
    ChatMessage {
        id: id.to_owned(),
        sender: Chatter {
            user_id: uid(user),
            display_name: "n".to_owned(),
            color: None,
            badges: vec![],
        },
        text: text.to_owned(),
        action: false,
        emotes: vec![],
        sent_at: Timestamp::from_unix(0),
        deleted: false,
    }
}

fn emote(name: &str) -> Emote {
    Emote {
        id: name.to_owned(),
        name: name.to_owned(),
        url: format!("https://e/{name}"),
        still_url: None,
        zero_width: false,
    }
}

fn deleted(chat: &ChatController) -> Vec<(String, bool)> {
    chat.messages().map(|m| (m.id.clone(), m.deleted)).collect()
}

#[test]
fn the_601st_message_evicts_the_oldest() {
    let mut chat = ChatController::new();
    for i in 0..MESSAGE_LIMIT {
        let m = message(&i.to_string(), "1", "hi");
        assert_eq!(
            chat.apply(ChatEvent::Message(m.clone())),
            vec![ChatChange::Appended { message: m }]
        );
    }
    let last = message("last", "1", "hi");
    assert_eq!(
        chat.apply(ChatEvent::Message(last.clone())),
        vec![
            ChatChange::Appended { message: last },
            ChatChange::Evicted { count: 1 }
        ]
    );
    assert_eq!(chat.messages().count(), MESSAGE_LIMIT);
    assert_eq!(chat.messages().next().map(|m| m.id.as_str()), Some("1"));
}

#[test]
fn clear_message_reports_the_marked_id() {
    let mut chat = ChatController::new();
    chat.apply(ChatEvent::Message(message("a", "1", "x")));
    assert_eq!(
        chat.apply(ChatEvent::ClearMessage { id: "a".into() }),
        vec![ChatChange::Marked {
            ids: vec!["a".to_owned()]
        }]
    );
}

#[test]
fn clearing_nothing_new_changes_nothing() {
    let mut chat = ChatController::new();
    chat.apply(ChatEvent::Message(message("a", "1", "x")));
    assert_eq!(
        chat.apply(ChatEvent::ClearMessage { id: "gone".into() }),
        vec![]
    );
    chat.apply(ChatEvent::ClearMessage { id: "a".into() });
    assert_eq!(
        chat.apply(ChatEvent::ClearMessage { id: "a".into() }),
        vec![]
    );
    assert_eq!(
        chat.apply(ChatEvent::ClearUser { user_id: uid("1") }),
        vec![]
    );
}

#[test]
fn clear_user_reports_that_users_ids() {
    let mut chat = ChatController::new();
    chat.apply(ChatEvent::Message(message("a", "1", "x")));
    chat.apply(ChatEvent::Message(message("b", "2", "y")));
    chat.apply(ChatEvent::Message(message("c", "1", "z")));
    assert_eq!(
        chat.apply(ChatEvent::ClearUser { user_id: uid("1") }),
        vec![ChatChange::Marked {
            ids: vec!["a".to_owned(), "c".to_owned()]
        }]
    );
}

#[test]
fn clear_all_room_connection_and_notice_report_their_change() {
    let mut chat = ChatController::new();
    chat.apply(ChatEvent::Message(message("a", "1", "x")));
    assert_eq!(chat.apply(ChatEvent::ClearAll), vec![ChatChange::Cleared]);
    assert_eq!(
        chat.apply(ChatEvent::RoomState(RoomStateUpdate::default())),
        vec![ChatChange::RoomChanged]
    );
    assert_eq!(
        chat.apply(ChatEvent::Connected),
        vec![ChatChange::ConnectionChanged]
    );
    assert_eq!(
        chat.apply(ChatEvent::Disconnected),
        vec![ChatChange::ConnectionChanged]
    );
    assert_eq!(
        chat.apply(ChatEvent::Notice("n".into())),
        vec![ChatChange::NoticeChanged]
    );
}

#[test]
fn set_emotes_asks_for_a_rerender() {
    let mut chat = ChatController::new();
    chat.apply(ChatEvent::Message(message("a", "1", "LUL")));
    assert_eq!(
        chat.set_emotes(EmoteScope::Global, EmoteProvider::Bttv, vec![emote("LUL")]),
        ChatChange::Rerender
    );
}

#[test]
fn clear_message_marks_only_that_id() {
    let mut chat = ChatController::new();
    chat.apply(ChatEvent::Message(message("a", "1", "x")));
    chat.apply(ChatEvent::Message(message("b", "1", "y")));
    chat.apply(ChatEvent::ClearMessage { id: "b".into() });
    assert_eq!(
        deleted(&chat),
        vec![("a".to_owned(), false), ("b".to_owned(), true)]
    );
}

#[test]
fn clear_user_marks_that_users_messages() {
    let mut chat = ChatController::new();
    chat.apply(ChatEvent::Message(message("a", "1", "x")));
    chat.apply(ChatEvent::Message(message("b", "2", "y")));
    chat.apply(ChatEvent::Message(message("c", "1", "z")));
    chat.apply(ChatEvent::ClearUser { user_id: uid("1") });
    assert_eq!(
        deleted(&chat),
        vec![
            ("a".to_owned(), true),
            ("b".to_owned(), false),
            ("c".to_owned(), true)
        ]
    );
}

#[test]
fn clear_all_empties_the_buffer() {
    let mut chat = ChatController::new();
    chat.apply(ChatEvent::Message(message("a", "1", "x")));
    chat.apply(ChatEvent::ClearAll);
    assert_eq!(chat.messages().count(), 0);
}

#[test]
fn partial_room_state_changes_only_its_fields() {
    let mut chat = ChatController::new();
    chat.apply(ChatEvent::RoomState(RoomStateUpdate {
        subs_only: Some(true),
        ..Default::default()
    }));
    chat.apply(ChatEvent::RoomState(RoomStateUpdate {
        slow: Some(Seconds::new(30)),
        ..Default::default()
    }));
    assert!(chat.room().subs_only);
    assert_eq!(chat.room().slow, Seconds::new(30));
    chat.apply(ChatEvent::RoomState(RoomStateUpdate {
        subs_only: Some(false),
        ..Default::default()
    }));
    assert!(!chat.room().subs_only);
    assert_eq!(chat.room().slow, Seconds::new(30));
}

#[test]
fn connection_and_notice_are_state() {
    let mut chat = ChatController::new();
    assert_eq!(chat.connection(), Connection::Disconnected);
    chat.apply(ChatEvent::Connected);
    assert_eq!(chat.connection(), Connection::Connected);
    chat.apply(ChatEvent::Notice("slow mode".into()));
    assert_eq!(chat.notice(), Some("slow mode"));
    chat.apply(ChatEvent::Disconnected);
    assert_eq!(chat.connection(), Connection::Disconnected);
}

#[test]
fn emotes_set_after_a_message_apply_on_the_next_fragments_call() {
    let mut chat = ChatController::new();
    let m = message("a", "1", "hi LUL");
    chat.apply(ChatEvent::Message(m.clone()));
    assert_eq!(chat.fragments(&m), vec![Fragment::Text("hi LUL".into())]);
    chat.set_emotes(EmoteScope::Global, EmoteProvider::Bttv, vec![emote("LUL")]);
    assert_eq!(
        chat.fragments(&m),
        vec![Fragment::Text("hi ".into()), Fragment::Emote(emote("LUL"))]
    );
}

#[test]
fn a_failed_provider_is_visible_until_it_loads() {
    let mut chat = ChatController::new();
    chat.provider_failed(EmoteProvider::Bttv);
    chat.provider_failed(EmoteProvider::Bttv);
    assert_eq!(chat.failed_providers(), &[EmoteProvider::Bttv]);
    chat.set_emotes(EmoteScope::Global, EmoteProvider::Bttv, vec![]);
    assert!(chat.failed_providers().is_empty());
}
