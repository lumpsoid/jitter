//! The few fMP4 boxes a VOD download reads and rewrites. Twitch's fMP4
//! segments carry each fragment's start time (`tfdt`) on the broadcast's
//! clock, so a VOD's file starts at, say, 62 s. A download moves every
//! `tfdt` back by one time shared by all tracks, in place and at the same
//! width, so no box changes size and no other byte moves.

use jitter_core::download::TrackShift;

/// Each track's id and its ticks per second, from an init segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timescales(Vec<(u32, u32)>);

/// Reads each track's timescale from an init segment (`moov`, each `trak`'s
/// `tkhd` and `mdhd`). None when it does not have that shape.
pub fn timescales(init: &[u8]) -> Option<Timescales> {
    let moov = boxes(init, 0, init.len()).find(|b| &b.kind == b"moov")?;
    let mut tracks = Vec::new();
    for trak in boxes(init, moov.payload, moov.end).filter(|b| &b.kind == b"trak") {
        let tkhd = child(init, &trak, b"tkhd")?;
        let track = read_u32(
            init,
            tkhd.payload + if version(init, &tkhd)? == 1 { 20 } else { 12 },
        )?;
        let mdhd = child(init, &child(init, &trak, b"mdia")?, b"mdhd")?;
        let timescale = read_u32(
            init,
            mdhd.payload + if version(init, &mdhd)? == 1 { 20 } else { 12 },
        )?;
        if timescale == 0 {
            return None;
        }
        tracks.push((track, timescale));
    }
    (!tracks.is_empty()).then_some(Timescales(tracks))
}

/// How far back each track moves so that `first`, the first media segment,
/// starts at zero: the earliest track's start, in seconds, in every track's
/// ticks. One time for all, so the tracks keep their lead on each other.
/// None when `first` has no fragment time for a track the init lists.
pub fn shifts(timescales: &Timescales, first: &[u8]) -> Option<Vec<TrackShift>> {
    let times = decode_times(first);
    let mut earliest: Option<(u64, u32)> = None;
    for &(track, timescale) in &timescales.0 {
        let start = times
            .iter()
            .find(|time| time.track == track)
            .and_then(|time| time.read(first))?;
        // start / timescale < ticks / scale, without dividing.
        let sooner = earliest.is_none_or(|(ticks, scale)| {
            u128::from(start) * u128::from(scale) < u128::from(ticks) * u128::from(timescale)
        });
        if sooner {
            earliest = Some((start, timescale));
        }
    }
    let (ticks, scale) = earliest?;
    timescales
        .0
        .iter()
        .map(|&(track, timescale)| {
            let shift = u128::from(ticks) * u128::from(timescale) / u128::from(scale);
            Some(TrackShift {
                track,
                ticks: u64::try_from(shift).ok()?,
            })
        })
        .collect()
}

/// Moves every fragment time in `segment` back by its track's shift. A
/// track with no shift stays as it is. A time earlier than its shift
/// becomes zero; returns how many did, which Twitch's VODs never had.
pub fn shift(segment: &mut [u8], shifts: &[TrackShift]) -> usize {
    let mut clamped = 0;
    for time in decode_times(segment) {
        let Some(shift) = shifts.iter().find(|shift| shift.track == time.track) else {
            continue;
        };
        let Some(start) = time.read(segment) else {
            continue;
        };
        if start < shift.ticks {
            clamped += 1;
        }
        time.write(segment, start.saturating_sub(shift.ticks));
    }
    clamped
}

/// One box: its type, and where its payload starts and it ends.
struct Mp4Box {
    kind: [u8; 4],
    payload: usize,
    end: usize,
}

/// A fragment's start time: its track, and its `tfdt`'s value's place and
/// width.
struct DecodeTime {
    track: u32,
    at: usize,
    wide: bool,
}

impl DecodeTime {
    fn read(&self, buf: &[u8]) -> Option<u64> {
        if self.wide {
            read_u64(buf, self.at)
        } else {
            read_u32(buf, self.at).map(u64::from)
        }
    }

    /// `value` is never more than the one read, so it fits the width.
    fn write(&self, buf: &mut [u8], value: u64) {
        if self.wide {
            put(buf, self.at, &value.to_be_bytes());
        } else if let Ok(value) = u32::try_from(value) {
            put(buf, self.at, &value.to_be_bytes());
        }
    }
}

/// Every fragment time in `segment`: each `moof`'s `traf`s, with the track
/// from its `tfhd`.
fn decode_times(segment: &[u8]) -> Vec<DecodeTime> {
    let mut times = Vec::new();
    for moof in boxes(segment, 0, segment.len()).filter(|b| &b.kind == b"moof") {
        for traf in boxes(segment, moof.payload, moof.end).filter(|b| &b.kind == b"traf") {
            let track =
                child(segment, &traf, b"tfhd").and_then(|tfhd| read_u32(segment, tfhd.payload + 4));
            let tfdt = child(segment, &traf, b"tfdt");
            if let (Some(track), Some(tfdt)) = (track, tfdt) {
                let Some(v) = version(segment, &tfdt) else {
                    continue;
                };
                times.push(DecodeTime {
                    track,
                    at: tfdt.payload + 4,
                    wide: v == 1,
                });
            }
        }
    }
    times
}

/// The boxes in `buf[start..end]`, in order, up to the first whose size
/// does not fit.
fn boxes(buf: &[u8], start: usize, end: usize) -> impl Iterator<Item = Mp4Box> + '_ {
    let mut at = start;
    std::iter::from_fn(move || {
        if at.checked_add(8)? > end {
            return None;
        }
        let size = read_u32(buf, at)?;
        let kind: [u8; 4] = buf.get(at + 4..at + 8)?.try_into().ok()?;
        let (size, header) = match size {
            0 => (end - at, 8),
            1 => (usize::try_from(read_u64(buf, at + 8)?).ok()?, 16),
            size => (usize::try_from(size).ok()?, 8),
        };
        let box_end = at.checked_add(size)?;
        if size < header || box_end > end {
            return None;
        }
        let found = Mp4Box {
            kind,
            payload: at + header,
            end: box_end,
        };
        at = box_end;
        Some(found)
    })
}

/// The first box of `kind` inside `parent`.
fn child(buf: &[u8], parent: &Mp4Box, kind: &[u8; 4]) -> Option<Mp4Box> {
    boxes(buf, parent.payload, parent.end).find(|b| &b.kind == kind)
}

/// A full box's version: the first byte of its payload.
fn version(buf: &[u8], full_box: &Mp4Box) -> Option<u8> {
    buf.get(full_box.payload).copied()
}

fn read_u32(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        buf.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn read_u64(buf: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(
        buf.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
}

fn put(buf: &mut [u8], at: usize, bytes: &[u8]) {
    if let Some(slot) = buf.get_mut(at..at + bytes.len()) {
        slot.copy_from_slice(bytes);
    }
}
