use core::time::Duration;

use woi::channel::mpsc;
use woi::time::sleep;
use woi::Runtime;

fn main() {
    tracing_subscriber::fmt::init();

    let rt = Runtime::new();
    rt.block_on(async {
        let (tx, rx) = mpsc::unbounded::channel();

        let h1 = woi::spawn(async {
            let tx = tx.clone();
            println!("Sending message from handle 1");
            tx.send("hello").unwrap()
        });

        let h2 = woi::spawn(async move {
            println!("Sending message from handle one after sleeping");
            sleep(Duration::from_secs(1)).await;
            println!("Done sleeping. Sending message from handle one");
            tx.send("hello world").unwrap();
            println!("Sent message!");
        });

        let h3 = woi::spawn(async move {
            println!("Received message: {}", rx.recv().await.unwrap());
            println!("Received message: {}", rx.recv().await.unwrap());
        });

        let _ = woi::join!(h1, h2, h3);
    });

    println!("Finished")
}
