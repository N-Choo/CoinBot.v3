//! API gateway for CoinBot.v3.
//!
//! An Actix-Web HTTP service exposing wallet authentication, contract signing,
//! and USDT deposit intake. Requests are authenticated with Redis-backed
//! sessions, state is persisted in PostgreSQL, and deposit tickets are delegated
//! to the deposit worker over gRPC.
//!
//! - [`config`] — environment-driven [`config::AppConfig`].
//! - [`constants`] — platform wallet and USDT contract addresses.
//! - [`routes`] — the `/api` route table.
//! - [`state`] — shared [`state::AppState`] (DB pool, caches, gRPC channel).
//! - [`handlers`] — HTTP handlers grouped by domain.
//! - [`models`] — request/response DTOs and error types.
pub mod config;
pub mod constants;
pub mod routes;
pub mod state;

pub mod handlers;
pub mod models;
