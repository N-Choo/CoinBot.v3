use actix_web::{
    App, HttpServer,
    middleware::{Logger, NormalizePath},
    web,
};
use dotenvy::dotenv;
use api_gateway::{config::AppConfig, logger::init_logger, routes::api_routes};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    init_logger();

    let config = AppConfig::from_env()?;
    let app_state = api_gateway::state::AppState::new(&config).await?;
    let config_data = web::Data::new(config.clone());

    // Initial HTTP workers to handle inncomming TCP connections.
    HttpServer::new(move || {
        App::new()
            .wrap(config_data.get_cors())
            .wrap(NormalizePath::trim())
            .wrap(Logger::new("%a\t | %s\t | %Dms\t | %r\t"))
            .app_data(web::Data::new(app_state.nonce_cache.clone()))
            .app_data(web::Data::new(app_state.session_cache.clone()))
            .configure(api_routes)
    })
    .workers(config.n_worker)
    .backlog(config.n_queue)
    .bind((config.ip, config.port))?
    .run()
    .await?;

    Ok(())
}
