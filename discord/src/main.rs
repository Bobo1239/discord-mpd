use shared::dotenv::dotenv;

fn main() {
    dotenv().ok();
    let config = Config::from_env();
    discord::lainch(&config);
}
