use shared::config::Config;
use shared::dotenv::dotenv;

// TODO: error handling...

#[tokio::main]
async fn main() {
    dotenv().ok();
    let config = Config::from_env();

    // TODO: Make launch `Send` and spawn it normally (and disable the `rt-utils` tokio feature)
    let local = tokio::task::LocalSet::new();
    local.spawn_local(discord::launch(config.clone()));

    std::thread::spawn(|| {
        web::launch(config);
    });
    local.await;
}
