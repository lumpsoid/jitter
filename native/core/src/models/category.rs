use super::CategoryId;

/// A category (game) as the directory lists it, with its live viewers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    /// Either Twitch's `{width}x{height}` template or a fixed-size URL
    /// ending in `-<width>x<height>.<ext>`.
    pub box_art_url: String,
    pub viewers: u32,
}

impl Category {
    /// The box art at `width`×`height`: the template filled in, or the
    /// fixed size swapped for this one (Twitch serves any size at the same
    /// path). A URL with neither is returned unchanged.
    pub fn box_art(&self, width: u32, height: u32) -> String {
        let url = &self.box_art_url;
        if url.contains("{width}") || url.contains("{height}") {
            return url
                .replace("{width}", &width.to_string())
                .replace("{height}", &height.to_string());
        }
        resized(url, width, height).unwrap_or_else(|| url.clone())
    }
}

/// `…-40x56.jpg` as `…-<width>x<height>.jpg`; `None` without that suffix.
fn resized(url: &str, width: u32, height: u32) -> Option<String> {
    let (stem, extension) = url.rsplit_once('.')?;
    let (base, size) = stem.rsplit_once('-')?;
    let (w, h) = size.split_once('x')?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    (digits(w) && digits(h)).then(|| format!("{base}-{width}x{height}.{extension}"))
}
