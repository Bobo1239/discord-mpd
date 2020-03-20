use shared::config::Config;
use shared::dotenv::dotenv;

fn main() {
    dotenv().ok();
    let config = Config::from_env();
    web::launch(config);
}
