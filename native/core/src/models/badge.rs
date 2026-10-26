/// A badge as parsed from the `badges` tag. Images are not resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Badge {
    pub set_id: String,
    pub version: String,
}
