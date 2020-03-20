use shared::config::Config;
use shared::dotenv::dotenv;

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();
    let config = Config::from_env();
    discord::launch(config).await;
}
