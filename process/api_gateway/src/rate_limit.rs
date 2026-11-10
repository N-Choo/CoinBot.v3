//! Rate-limit key extraction for deployments behind a reverse proxy.

use actix_governor::{KeyExtractor, SimpleKeyExtractionError};
use actix_web::dev::ServiceRequest;

/// Key requests by the real client IP, trusting the first `X-Forwarded-For`
/// hop (set by the bundled nginx) and falling back to the peer address.
#[derive(Clone)]
pub struct ClientIpKeyExtractor;

impl KeyExtractor for ClientIpKeyExtractor {
    type Key = String;
    type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

    fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
        if let Some(ip) = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(',').next())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Ok(ip.to_string());
        }

        Ok(req
            .peer_addr()
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    #[test]
    fn prefers_the_first_forwarded_ip() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "203.0.113.7, 10.0.0.2"))
            .to_srv_request();
        assert_eq!(ClientIpKeyExtractor.extract(&req).unwrap(), "203.0.113.7");
    }
}
