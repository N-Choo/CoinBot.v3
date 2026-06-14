pub mod contracts;
pub mod transaction;
pub mod user;

use crate::handlers::user::auth::SessionCache;

pub async fn authenticate(header: &actix_web::HttpRequest, cache: &SessionCache) -> Option<String> {
    let cookie = header.cookie("session_token")?;
    cache.get(cookie.value()).await
}

#[cfg(test)]
mod tests;
