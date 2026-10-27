//! One channel's videos of one kind, paged. Changing the channel or the kind
//! starts a new list: a page still in flight for the old one is ignored.

use crate::models::{ChannelLogin, ListState, Page, PageRequest, Video, VideoKind};
use crate::paged_list::PagedList;

#[derive(Debug)]
pub struct VodsController {
    channel: ChannelLogin,
    kind: VideoKind,
    list: PagedList<Video>,
}

impl VodsController {
    /// Nothing is loaded until the first `begin_refresh`.
    pub fn new(channel: ChannelLogin, kind: VideoKind) -> Self {
        Self {
            channel,
            kind,
            list: PagedList::new(),
        }
    }

    /// The channel a request from this controller is for.
    pub fn channel(&self) -> &ChannelLogin {
        &self.channel
    }

    /// The kind a request from this controller is for.
    pub fn kind(&self) -> VideoKind {
        self.kind
    }

    pub fn state(&self) -> &ListState<Video> {
        self.list.state()
    }

    /// Switches to another channel and starts over from its first page.
    pub fn show_channel(&mut self, channel: ChannelLogin) -> PageRequest {
        self.channel = channel;
        self.list.begin_refresh()
    }

    /// Switches to another kind and starts over from its first page.
    pub fn show_kind(&mut self, kind: VideoKind) -> PageRequest {
        self.kind = kind;
        self.list.begin_refresh()
    }

    pub fn begin_refresh(&mut self) -> PageRequest {
        self.list.begin_refresh()
    }

    pub fn begin_next(&mut self) -> Option<PageRequest> {
        self.list.begin_next()
    }

    pub fn apply_page(&mut self, request: PageRequest, result: Result<Page<Video>, String>) {
        self.list.apply_page(request, result);
    }
}
