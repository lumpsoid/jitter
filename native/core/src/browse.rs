//! The top-streams list. Paging, the in-flight guard and stale-result
//! protection are the shared [`PagedList`]; top streams add nothing to it.

use crate::models::Stream;
use crate::paged_list::PagedList;

pub type BrowseController = PagedList<Stream>;
