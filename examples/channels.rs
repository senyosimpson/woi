use std::time::Duration;

use woi::channel::mpsc;
use woi::time::sleep;
use woi::Runtime;

fn main() {
    tracing_subscriber::fmt::init();

    let rt = Runtime::new();
    rt.block_on(async {
        let (tx, rx) = mpsc::channel();
        woi::spawn(async {
            let tx = tx.clone();
            println!("Sending message from handle 1");
            tx.send("fly.io").unwrap()
        });

        woi::spawn(async move {
            println!("Sending message from handle one after sleeping");
            sleep(Duration::from_secs(1)).await;
            println!("Done sleeping. Sending message from handle one");
            tx.send("hello world").unwrap();
            println!("Sent message!");
        });

        let h2 = woi::spawn(async move {
            println!("Received message: {}", rx.recv().await.unwrap());
            println!("Received message: {}", rx.recv().await.unwrap());
        });

        h2.await.unwrap();
    });

    println!("Finished")
}
