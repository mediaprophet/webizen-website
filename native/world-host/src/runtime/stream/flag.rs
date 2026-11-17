//! Production helper for EWM streaming flag. PR6a reads the query inline and
//! must not depend on this file for first paint.

use web_sys::HtmlCanvasElement;

pub fn streaming_requested(query: &str, canvas: &HtmlCanvasElement) -> bool {
    super::ewm_query_enabled(query, canvas)
}

pub const ROLLBACK: &str = "Leave streaming.default false and omit ?ewm=1";
