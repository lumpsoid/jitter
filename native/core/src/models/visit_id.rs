/// A visit's key, as the store assigned it. Never reused: an id from a
/// removed visit cannot name a later one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VisitId(i64);

impl VisitId {
    pub fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    pub fn get(self) -> i64 {
        self.0
    }
}
