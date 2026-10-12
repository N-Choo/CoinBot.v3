use crate::auth::cache::SessionCache;

/// Resolve the authenticated wallet for a request.
///
/// Reads the `session_token` cookie and looks it up in the session cache.
/// Returns `None` when the cookie is absent or the session has expired, which
/// callers translate into `401 Unauthorized`.
pub async fn authenticate(header: &actix_web::HttpRequest, cache: &SessionCache) -> Option<String> {
    let cookie = header.cookie("session_token")?;
    cache.get(cookie.value()).await
}
