use crossbeam_utils::thread;

use shared::config::Config;
use shared::dotenv::dotenv;

// TODO: error handling...

fn main() {
    dotenv().ok();
    let config = Config::from_env();

    thread::scope(|scope| {
        scope.spawn(|_| {
            web::launch(&config);
        });
        scope.spawn(|_| {
            discord::launch(&config);
        });
    })
    .unwrap();
}
