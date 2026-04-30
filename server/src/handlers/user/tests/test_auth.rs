#[cfg(test)]
mod tests {
    use crate::{
        handlers::user::auth::{AuthController, CacheType},
        models::auth::VerifySignaturRequest,
    };

    use actix_web::{
        Responder,
        cookie::Cookie,
        http::{StatusCode, header::SET_COOKIE},
        test, web,
    };
    use ethers::{
        core::rand::{SeedableRng, rngs::StdRng},
        signers::{LocalWallet, Signer},
    };
    use moka::future::Cache;

    fn setup_caches() -> (CacheType, CacheType) {
        let nonce_cache = Cache::new(100);
        let session_cache = Cache::new(100);
        (nonce_cache, session_cache)
    }

    #[actix_web::test]
    async fn test_post_auth_success() {
        let mut seeded_rng = StdRng::seed_from_u64(84);
        let wallet = LocalWallet::new(&mut seeded_rng);
        let wallet_address = format!("0x{:x}", wallet.address()).to_lowercase();
        let nonce = "test_nonce";
        let signature = wallet.sign_message(nonce).await.unwrap();

        let (nonce_cache, session_cache) = setup_caches();
        let nonce_cache_data = web::Data::new(nonce_cache.clone());
        let session_cache_data = web::Data::new(session_cache.clone());

        nonce_cache
            .insert(wallet_address.clone(), nonce.to_string())
            .await;

        let payload = VerifySignaturRequest {
            signature: format!("0x{}", signature),
            msg: nonce.to_string(),
        };

        let resp = AuthController::login(
            web::Json(payload),
            nonce_cache_data.clone(),
            session_cache_data.clone(),
        )
        .await;

        // Convert Responder to HttpResponse
        let req = test::TestRequest::default().to_http_request();
        let http_resp = resp.respond_to(&req);
        assert_eq!(http_resp.status(), StatusCode::OK);

        let cookie_header = http_resp
            .headers()
            .get(SET_COOKIE)
            .expect("No Set-Cookie header")
            .to_str()
            .unwrap();

        let cookie = Cookie::parse(cookie_header).unwrap();
        let token = cookie.value();

        // Verify session was actually created in cache
        let cached_wallet = session_cache.get(token).await;
        assert_eq!(cached_wallet.unwrap(), wallet_address);
    }

    #[actix_web::test]
    async fn test_post_auth_nonce_mismatch() {
        let mut seeded_rng = StdRng::seed_from_u64(84);
        let wallet = LocalWallet::new(&mut seeded_rng);
        let wallet_address = format!("0x{:x}", wallet.address()).to_lowercase();

        let (nonce_cache, session_cache) = setup_caches();

        // Put the CORRECT nonce in the cache
        nonce_cache
            .insert(wallet_address.clone(), "correct_nonce".to_string())
            .await;

        // This allows get_wallet to succeed, so we can reach the 400 check
        let wrong_nonce = "evil_nonce";
        let signature = wallet.sign_message(wrong_nonce).await.unwrap();

        let payload = VerifySignaturRequest {
            signature: format!("0x{}", signature),
            msg: wrong_nonce.to_string(),
        };

        let resp = AuthController::login(
            web::Json(payload),
            web::Data::new(nonce_cache),
            web::Data::new(session_cache),
        )
        .await;

        let req = test::TestRequest::default().to_http_request();
        let http_resp = resp.respond_to(&req);

        assert_eq!(http_resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_post_auth_invalid_signature() {
        let (nonce_cache, session_cache) = setup_caches();

        let payload = VerifySignaturRequest {
            signature: "not-a-hex-string".to_string(),
            msg: "some_nonce".to_string(),
        };

        let resp = AuthController::login(
            web::Json(payload),
            web::Data::new(nonce_cache),
            web::Data::new(session_cache),
        )
        .await;

        let req = test::TestRequest::default().to_http_request();
        let http_resp = resp.respond_to(&req);

        assert_eq!(http_resp.status(), StatusCode::UNAUTHORIZED);
    }


    #[actix_web::test]
    async fn test_verify_session_ok() {
        let (_, session_cache) = setup_caches();

        // Simulate a logged-in user by inserting a session token
        let token = "test_token";
        let wallet_address = "0x1234567890abcdef".to_string();
        session_cache
            .insert(token.to_string(), wallet_address.clone())
            .await;

        let req = test::TestRequest::default()
            .cookie(Cookie::build("session_token", token).finish())
            .to_http_request();

        let resp =
            AuthController::verify_session(req.clone(), web::Data::new(session_cache.clone()))
                .await;

        let http_resp = resp.respond_to(&req);
        assert_eq!(http_resp.status(), StatusCode::OK);

        // Verify the session token is still valid
        let cached_wallet = session_cache.get(token).await;
        assert_eq!(cached_wallet.unwrap(), wallet_address);
    }

    #[actix_web::test]
    async fn test_verify_session_invalid() {
        let (_, session_cache) = setup_caches();

        let req = test::TestRequest::default()
            .cookie(Cookie::build("session_token", "invalid_token").finish())
            .to_http_request();

        let resp =
            AuthController::verify_session(req.clone(), web::Data::new(session_cache.clone()))
                .await;

        let http_resp = resp.respond_to(&req);
        assert_eq!(http_resp.status(), StatusCode::UNAUTHORIZED);

        // Verify the session token is not valid
        let cached_wallet = session_cache.get("invalid_token").await;
        assert!(cached_wallet.is_none());
    }
}
