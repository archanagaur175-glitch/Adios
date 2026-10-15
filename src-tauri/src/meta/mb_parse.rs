//! Search body decoding (P5.1): `{created, count, offset, "<list>": [...]}`.
//! Split from `mb_types` to keep every source file ≤ 200 lines.

use super::mb_types::SearchResults;

/// Decode a full MB search body. A missing list key decodes as an empty
/// page (zero-result responses).
pub fn parse_list<T: serde::de::DeserializeOwned>(
    list_key: &str,
    body: &str,
) -> Result<SearchResults<T>, String> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("mb: invalid json: {e}"))?;
    let count = v.get("count").and_then(|c| c.as_i64()).unwrap_or(0);
    let offset = v.get("offset").and_then(|c| c.as_i64()).unwrap_or(0);
    let items = match v.get(list_key) {
        Some(arr) if arr.is_array() => serde_json::from_value(arr.clone())
            .map_err(|e| format!("mb: invalid {list_key} items: {e}"))?,
        _ => Vec::new(),
    };
    Ok(SearchResults {
        count,
        offset,
        items,
    })
}
