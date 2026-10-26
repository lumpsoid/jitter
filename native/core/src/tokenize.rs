//! Splits a chat message into renderable fragments. Pure: the message text,
//! its Twitch emote spans and the third-party emote set go in, fragments
//! come out.

use crate::models::{Emote, EmoteSet, EmoteSpan, Fragment};

/// Twitch's spans are code-point indices (not bytes, not UTF-16 units), end
/// inclusive. Spans that are out of range or overlap an earlier one are
/// ignored. Outside spans, whole whitespace-separated words are looked up in
/// `set`.
pub fn tokenize(text: &str, spans: &[EmoteSpan], set: &EmoteSet) -> Vec<Fragment> {
    let chars: Vec<char> = text.chars().collect();
    let mut sorted: Vec<&EmoteSpan> = spans.iter().collect();
    sorted.sort_by_key(|s| s.start);

    let mut out = Vec::new();
    let mut cursor = 0;
    for span in sorted {
        if span.start < cursor || span.start > span.end || span.end >= chars.len() {
            continue;
        }
        tokenize_words(&chars[cursor..span.start], set, &mut out);
        push_emote(&mut out, span.emote.clone());
        cursor = span.end + 1;
    }
    tokenize_words(&chars[cursor..], set, &mut out);
    out
}

fn tokenize_words(chars: &[char], set: &EmoteSet, out: &mut Vec<Fragment>) {
    let mut rest = chars;
    while let Some(first) = rest.first() {
        let space = first.is_whitespace();
        let len = rest
            .iter()
            .position(|c| c.is_whitespace() != space)
            .unwrap_or(rest.len());
        let (token, tail) = rest.split_at(len);
        rest = tail;
        let word: String = token.iter().collect();
        if space {
            push_text(out, word);
        } else if let Some(emote) = set.get(&word) {
            push_emote(out, emote.clone());
        } else if is_link(&word) {
            out.push(Fragment::Link(word));
        } else {
            push_text(out, word);
        }
    }
}

fn is_link(word: &str) -> bool {
    ["http://", "https://", "www."]
        .iter()
        .any(|prefix| word.len() > prefix.len() && word.starts_with(prefix))
}

fn push_text(out: &mut Vec<Fragment>, text: String) {
    if let Some(Fragment::Text(last)) = out.last_mut() {
        last.push_str(&text);
    } else {
        out.push(Fragment::Text(text));
    }
}

/// A zero-width emote attaches to the previous emote, swallowing the
/// whitespace between them; with no emote before it, it stays text.
fn push_emote(out: &mut Vec<Fragment>, emote: Emote) {
    if !emote.zero_width {
        out.push(Fragment::Emote(emote));
        return;
    }
    let gap = matches!(out.last(), Some(Fragment::Text(t)) if t.trim().is_empty());
    let base_index = out.len().checked_sub(if gap { 2 } else { 1 });
    let base = base_index.and_then(|i| out.get(i));
    if !matches!(base, Some(Fragment::Emote(_) | Fragment::Overlay { .. })) {
        push_text(out, emote.name);
        return;
    }
    if gap {
        out.pop();
    }
    match out.pop() {
        Some(Fragment::Emote(base)) => out.push(Fragment::Overlay {
            base,
            overlays: vec![emote],
        }),
        Some(Fragment::Overlay { base, mut overlays }) => {
            overlays.push(emote);
            out.push(Fragment::Overlay { base, overlays });
        }
        Some(other) => out.push(other),
        None => {}
    }
}
