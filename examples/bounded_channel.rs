use core::time::Duration;

use woi::channel::mpsc;
use woi::time::sleep;
use woi::Runtime;

fn main() {
    tracing_subscriber::fmt::init();

    let rt = Runtime::new();
    rt.block_on(async {
        let (tx, rx) = mpsc::bounded::channel(2);

        woi::spawn(async {
            let tx = tx.clone();
            tx.send("task 1").await.unwrap();
            println!("Sent message from task 1");
        });

        woi::spawn(async {
            let tx = tx.clone();
            tx.send("task 2").await.unwrap();
            println!("Sent message from task 2");
        });

        woi::spawn(async {
            let tx = tx.clone();
            tx.send("task 3").await.unwrap();
            println!("Sent message from task 3");
        });

        let h1 = woi::spawn(async move {
            println!("Received message: {}", rx.recv().await.unwrap());
            println!("Received message: {}", rx.recv().await.unwrap());
            println!("Received message: {}", rx.recv().await.unwrap());
        });

        h1.await.unwrap();
    });

    println!("Finished")
}
