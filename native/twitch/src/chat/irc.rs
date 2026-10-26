//! A pure IRCv3 line parser. No I/O, no Twitch semantics.

/// One parsed IRC line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrcMessage {
    /// `(key, unescaped value)` in line order. A tag without `=` has an
    /// empty value, like `key=`.
    pub tags: Vec<(String, String)>,
    /// What follows the leading `:`, without it (`nick!nick@host`).
    pub prefix: Option<String>,
    pub command: String,
    /// Middle parameters, without the trailing one.
    pub params: Vec<String>,
    /// The parameter after ` :`, which may contain spaces or be empty.
    pub trailing: Option<String>,
}

impl IrcMessage {
    pub fn tag(&self, key: &str) -> Option<&str> {
        self.tags
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The nick part of the prefix (`nick` of `nick!user@host`).
    pub fn nick(&self) -> Option<&str> {
        let prefix = self.prefix.as_deref()?;
        let nick = prefix.split(['!', '@']).next()?;
        (!nick.is_empty()).then_some(nick)
    }
}

/// Splits one WebSocket frame into its lines. A frame can carry several
/// lines separated by `\r\n`; empty lines are dropped.
pub fn split_frame(frame: &str) -> impl Iterator<Item = &str> {
    frame.split("\r\n").filter(|line| !line.is_empty())
}

/// Parses one line (a trailing `\r\n` is tolerated). Malformed input, with
/// no command, gives `None`.
pub fn parse(line: &str) -> Option<IrcMessage> {
    let mut rest = line.trim_end_matches(['\r', '\n']);

    let tags = match rest.strip_prefix('@') {
        Some(after) => {
            let (block, tail) = after.split_once(' ')?;
            rest = tail.trim_start_matches(' ');
            parse_tags(block)
        }
        None => Vec::new(),
    };

    let prefix = match rest.strip_prefix(':') {
        Some(after) => {
            let (prefix, tail) = after.split_once(' ')?;
            rest = tail.trim_start_matches(' ');
            Some(prefix.to_owned())
        }
        None => None,
    };

    let (head, trailing) = match rest.split_once(" :") {
        Some((head, trailing)) => (head, Some(trailing.to_owned())),
        None => (rest, None),
    };

    let mut words = head.split(' ').filter(|w| !w.is_empty());
    let command = words.next()?.to_owned();
    let params = words.map(str::to_owned).collect();

    Some(IrcMessage {
        tags,
        prefix,
        command,
        params,
        trailing,
    })
}

fn parse_tags(block: &str) -> Vec<(String, String)> {
    block
        .split(';')
        .filter(|tag| !tag.is_empty())
        .map(|tag| match tag.split_once('=') {
            Some((key, value)) => (key.to_owned(), unescape(value)),
            None => (tag.to_owned(), String::new()),
        })
        .collect()
}

/// IRCv3 tag value unescape: `\:` `\s` `\\` `\r` `\n`; any other escaped
/// character stands for itself and a lone trailing backslash is dropped.
fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some(':') => out.push(';'),
            Some('s') => out.push(' '),
            Some('r') => out.push('\r'),
            Some('n') => out.push('\n'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}
