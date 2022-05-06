use core::time::Duration;

use woi::channel::mpsc;
use woi::time::sleep;
use woi::Runtime;

fn main() {
    tracing_subscriber::fmt::init();

    let rt = Runtime::new();
    rt.block_on(async {
        let (tx, rx) = mpsc::unbounded::channel();
        woi::spawn(async {
            let tx = tx.clone();
            println!("Sending message from task 1");
            tx.send("task 1: fly.io").unwrap()
        });

        woi::spawn(async move {
            println!("Sending message from task 2 after sleeping");
            sleep(Duration::from_secs(1)).await;
            println!("Done sleeping. Sending message from task 2");
            tx.send("handle 2: hello world").unwrap();
        });

        let h2 = woi::spawn(async move {
            println!("Received message: {}", rx.recv().await.unwrap());
            println!("Received message: {}", rx.recv().await.unwrap());
        });

        h2.await.unwrap();
    });

    println!("Finished")
}
