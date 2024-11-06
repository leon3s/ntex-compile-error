use ntex::web;
use ntex_cors::Cors;

use nanocl_utils::ntex::middlewares;

pub async fn gen() -> std::io::Result<ntex::server::Server> {
    let mut server = web::HttpServer::new(move || {
      web::App::new()
        // bind config state
        .state(
          web::types::PayloadConfig::new(20_000_000_000), // <- limit size of the payload
        )
        .wrap(Cors::new().finish())
        .wrap(middlewares::Versioning::new("1.0").finish())
        .wrap(middlewares::SerializeError)
        // Default logger middleware
        .wrap(web::middleware::Logger::default())
        // Set Json body max size
        .state(web::types::JsonConfig::default().limit(20_000_000))
    });
    server = server.bind("0.0.0.0:5555")?;
    Ok(server.run())
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    println!("Hello, world!");
    let server = gen().await?;
    server.await?;
    Ok(())
}
