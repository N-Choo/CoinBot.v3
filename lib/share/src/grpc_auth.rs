#![allow(clippy::result_large_err)]
//! Shared service-token authentication for internal gRPC calls.
//!
//! The services run on a private network, but a token keeps a compromised
//! sidecar/container from driving the trading or deposit pipeline.

use tonic::{Request, Status};

/// Metadata header carrying the service token.
pub const TOKEN_HEADER: &str = "x-service-token";

/// Verify the token on an incoming request.
pub fn check<T>(request: &Request<T>, expected: &str) -> Result<(), Status> {
    if expected.is_empty() {
        return Err(Status::internal("service token is not configured"));
    }

    let supplied = request
        .metadata()
        .get(TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if supplied == expected {
        Ok(())
    } else {
        Err(Status::unauthenticated("invalid service token"))
    }
}

/// Build a request carrying the service token.
pub fn with_token<T>(message: T, token: &str) -> Result<Request<T>, Status> {
    let mut request = Request::new(message);
    let value = token
        .parse()
        .map_err(|_| Status::internal("invalid service token"))?;
    request.metadata_mut().insert(TOKEN_HEADER, value);
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_expected_token() {
        let request = with_token((), "secret").unwrap();
        assert!(check(&request, "secret").is_ok());
    }

    #[test]
    fn rejects_a_missing_or_wrong_token() {
        assert!(check(&Request::new(()), "secret").is_err());
        assert!(check(&with_token((), "wrong").unwrap(), "secret").is_err());
    }
}
