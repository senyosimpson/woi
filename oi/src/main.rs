use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use tracing_subscriber;
use woi;
use woi::channel::mpsc;
use woi::io::AsyncReadExt;
use woi::net::TcpStream;
use woi::time::sleep;
use woi::Runtime;

enum Mains {
    Channel,
    TcpRead,
    Sleep,
}

fn main() {
    let m = Mains::Channel;
    match m {
        Mains::Channel => channel(),
        Mains::TcpRead => tcp_read(),
        Mains::Sleep => go_sleep(),
    }
}

fn channel() {
    tracing_subscriber::fmt::init();

    let rt = Runtime::new();
    let h = rt.block_on(async {
        let (tx, rx) = mpsc::channel();
        // woi::spawn(async {
        //     let tx = tx.clone();
        //     println!("Sending message from handle 1");
        //     tx.send("hello").unwrap()
        // });

        let h1 = woi::spawn(async move {
            println!("Sending message from handle one after sleeping");
            sleep(Duration::from_secs(1)).await;
            println!("Done sleeping. Sending message from handle one");
            tx.send("hello world").unwrap();
            println!("Sent message!");
        });

        let h2 = woi::spawn(async move {
            println!("Received message: {}", rx.recv().await.unwrap());
            // println!("Received message: {}", rx.recv().await.unwrap());
        });

        h2.await;
    });

    println!("Finished")
}

fn tcp_read() {
    tracing_subscriber::fmt::init();

    let rt = Runtime::new();
    rt.block_on(async {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let mut stream = TcpStream::connect(addr).await.unwrap();

        let handle = woi::spawn(async move {
            let mut buf = vec![0; 1024];
            let n = stream
                .read(&mut buf)
                .await
                .expect("failed to read data from socket");
            println!("Received message: {}", String::from_utf8(buf).unwrap());
            n
        });

        let n = handle.await;
        println!("Read {} bytes", n)
    })
}

fn go_sleep() {
    tracing_subscriber::fmt::init();

    let rt = Runtime::new();
    rt.block_on(async {
        let now = Instant::now();
        let handle = woi::spawn(async {
            println!("Sleeping for 5 seconds!");
            sleep(Duration::from_secs(5)).await;
        });

        handle.await;

        let later = Instant::now();
        let elapsed = later - now;
        println!(
            "Waking from sleep! {}:{} elapsed",
            elapsed.as_secs(),
            elapsed.subsec_millis()
        );
    })
}
