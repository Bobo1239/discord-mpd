use shared::config::Config;
use shared::dotenv::dotenv;

// TODO: error handling...

#[tokio::main]
async fn main() {
    dotenv().ok();
    let config = Config::from_env();

    tokio::spawn(discord::launch(config.clone()));
    let web = tokio::spawn(web::launch(config));

    // Rocket registers a Ctrl+C handler so we just wait keep running until web finishes
    web.await.unwrap();
}
