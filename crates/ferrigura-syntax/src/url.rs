//! `sanitizeUrl` of `@braintree/sanitize-url` 7.1.2 and `formatUrl` of the reference (`utils.ts`).
//!
//! Stub: replaced by the port. See `docs-port/class/PLAN.md`.

/// Placeholder: returns the input unchanged until the port lands.
pub(crate) fn sanitize_url(url: &str) -> String {
    url.to_owned()
}

/// Placeholder until the port lands: `formatUrl` with the trimming and the empty check only.
pub(crate) fn format_url(link: &str, loose: bool) -> Option<String> {
    let url = link.trim();
    if url.is_empty() {
        None
    } else if loose {
        Some(url.to_owned())
    } else {
        Some(sanitize_url(url))
    }
}
