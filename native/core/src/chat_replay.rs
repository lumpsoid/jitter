//! A VOD's chat, replayed in step with the player. The head reports where
//! the player is and fetches the pages asked for; the replay decides which
//! messages are on screen at that position, when the next page is needed,
//! and when a jump in position is a seek that starts the chat over.
//!
//! What is on screen is a [`ChatController`] fed the replayed messages, so
//! a head renders a replay exactly like live chat. Pure state.

use std::collections::VecDeque;

use crate::chat::ChatController;
use crate::models::{
    ChatChange, ChatEvent, Cursor, Emote, EmoteProvider, EmoteScope, Page, ReplayComment,
    ReplayFrom, Seconds, VideoId,
};
use crate::playback::PlaybackTarget;

/// A position more than this far ahead of the last one is a seek, not
/// playback: reports come about every second, so ordinary play (even fast)
/// moves far less. Any step back is a seek.
pub const SEEK_JUMP: Seconds = Seconds::new(20);

/// After a seek the chat starts this far before the new position, so the
/// panel shows what was being said rather than an empty list.
pub const BACKFILL: Seconds = Seconds::new(30);

/// The next page is asked for once this few fetched messages are still
/// waiting for their time, so it arrives before they run out.
pub const LOW_WATER: usize = 25;

/// A page for the head to fetch; handed back unchanged with its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayRequest {
    pub video: VideoId,
    pub from: ReplayFrom,
}

/// What one action changed, in order, and the page to fetch next, if any.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReplayStep {
    /// As live chat reports them. The replay's status (waiting, failure)
    /// changing is a [`ChatChange::ConnectionChanged`].
    pub changes: Vec<ChatChange>,
    pub fetch: Option<ReplayRequest>,
}

#[derive(Debug)]
pub struct ChatReplay {
    video: VideoId,
    /// The messages on screen, and the emotes they render with.
    chat: ChatController,
    /// The last position reported; `None` until the player has one.
    position: Option<Seconds>,
    /// Fetched messages whose time has not come yet, oldest first.
    pending: VecDeque<ReplayComment>,
    /// Where the page after the last one fetched starts; `None` at the end.
    next: Option<Cursor>,
    /// The page in flight; only its result applies.
    loading: Option<ReplayRequest>,
    /// Why the last page failed; nothing more is fetched until a seek.
    failure: Option<String>,
}

impl ChatReplay {
    /// Fetches nothing until the player reports where it is.
    pub fn new(video: VideoId) -> Self {
        Self {
            video,
            chat: ChatController::new(),
            position: None,
            pending: VecDeque::new(),
            next: None,
            loading: None,
            failure: None,
        }
    }

    /// The messages on screen, oldest first, and how to render them.
    pub fn chat(&self) -> &ChatController {
        &self.chat
    }

    /// Nothing to show yet: the player has no position, or the page for
    /// the position it jumped to is still loading.
    pub fn waiting(&self) -> bool {
        self.position.is_none()
            || matches!(
                self.loading,
                Some(ReplayRequest {
                    from: ReplayFrom::Offset(_),
                    ..
                })
            )
    }

    /// Why the chat stopped: a page failed to load.
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    /// See [`ChatController::set_emotes`].
    pub fn set_emotes(
        &mut self,
        scope: EmoteScope,
        provider: EmoteProvider,
        emotes: Vec<Emote>,
    ) -> ChatChange {
        self.chat.set_emotes(scope, provider, emotes)
    }

    /// See [`ChatController::provider_failed`].
    pub fn provider_failed(&mut self, provider: EmoteProvider) {
        self.chat.provider_failed(provider);
    }

    /// The player showing `target` is at `position` of `duration`. Ignored
    /// unless `target` is this VOD and the player has loaded (a zero
    /// `duration` means its position means nothing yet). Shows the
    /// messages whose time has come; a seek (the first position, a step
    /// back, or a jump past [`SEEK_JUMP`]) clears the chat and loads it
    /// again from [`BACKFILL`] before the new position.
    pub fn position(
        &mut self,
        target: &PlaybackTarget,
        position: Seconds,
        duration: Seconds,
    ) -> ReplayStep {
        let PlaybackTarget::Vod(video) = target else {
            return ReplayStep::default();
        };
        if *video != self.video || duration == Seconds::ZERO {
            return ReplayStep::default();
        }
        let seek = match self.position {
            None => true,
            Some(last) => position < last || position > last + SEEK_JUMP,
        };
        let status = self.status();
        self.position = Some(position);
        let mut step = match seek {
            true => self.seek(position),
            false => ReplayStep {
                changes: self.release(),
                fetch: self.refill(),
            },
        };
        self.status_changed(status, &mut step.changes);
        step
    }

    /// A page asked for arrived. Ignored unless it is the one in flight, so
    /// a page from before a seek never lands after it. A failure stops the
    /// chat until the next seek.
    pub fn apply(
        &mut self,
        request: ReplayRequest,
        result: Result<Page<ReplayComment>, String>,
    ) -> ReplayStep {
        if self.loading.as_ref() != Some(&request) {
            return ReplayStep::default();
        }
        let status = self.status();
        self.loading = None;
        let mut step = match result {
            Err(message) => {
                self.failure = Some(message);
                ReplayStep::default()
            }
            Ok(page) => {
                self.pending.extend(page.items);
                self.next = page.next;
                ReplayStep {
                    changes: self.release(),
                    fetch: self.refill(),
                }
            }
        };
        self.status_changed(status, &mut step.changes);
        step
    }

    /// Drops everything fetched and shown, and asks for the chat from a
    /// little before `position`.
    fn seek(&mut self, position: Seconds) -> ReplayStep {
        self.pending.clear();
        self.next = None;
        self.failure = None;
        let shown = self.chat.messages().next().is_some();
        let changes = match shown {
            true => self.chat.apply(ChatEvent::ClearAll),
            false => Vec::new(),
        };
        let request = ReplayRequest {
            video: self.video.clone(),
            from: ReplayFrom::Offset(position - BACKFILL),
        };
        self.loading = Some(request.clone());
        ReplayStep {
            changes,
            fetch: Some(request),
        }
    }

    /// Moves the fetched messages whose time has come onto the screen.
    fn release(&mut self) -> Vec<ChatChange> {
        let Some(position) = self.position else {
            return Vec::new();
        };
        let mut changes = Vec::new();
        while let Some(comment) = self.pending.pop_front() {
            if comment.offset > position {
                self.pending.push_front(comment);
                break;
            }
            changes.extend(self.chat.apply(ChatEvent::Message(comment.message)));
        }
        changes
    }

    /// The next page, when few fetched messages are left, there is one,
    /// and nothing is in flight or has failed.
    fn refill(&mut self) -> Option<ReplayRequest> {
        if self.loading.is_some() || self.failure.is_some() || self.pending.len() > LOW_WATER {
            return None;
        }
        let request = ReplayRequest {
            video: self.video.clone(),
            from: ReplayFrom::After(self.next.clone()?),
        };
        self.loading = Some(request.clone());
        Some(request)
    }

    fn status(&self) -> (bool, Option<String>) {
        (self.waiting(), self.failure.clone())
    }

    fn status_changed(&self, before: (bool, Option<String>), changes: &mut Vec<ChatChange>) {
        if self.status() != before {
            changes.push(ChatChange::ConnectionChanged);
        }
    }
}
