use std::time::{Duration, Instant};

use woi::time::sleep;
use woi::Runtime;

fn main() {
    tracing_subscriber::fmt::init();

    let rt = Runtime::new();
    rt.block_on(async {
        let now = Instant::now();
        let handle = woi::spawn(async {
            println!("Sleeping for 5 seconds!");
            sleep(Duration::from_secs(5)).await;
        });

        let _ = handle.await;

        let later = Instant::now();
        let elapsed = later - now;
        println!(
            "Waking from sleep! {}:{} elapsed",
            elapsed.as_secs(),
            elapsed.subsec_millis()
        );
    })
}
