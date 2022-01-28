use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use tracing_subscriber;
use woi;
use woi::io::AsyncReadExt;
use woi::net::TcpStream;

fn main() {
    tracing_subscriber::fmt::init();

    let rt = woi::Runtime::new();
    rt.block_on(async {

        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let mut stream = TcpStream::connect(addr).await.unwrap();

        let handle = rt.spawn(async move {
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
        // println!("Received message: {}", String::from_utf8(buf).unwrap());
    })
    // at this point, the io resource gets dropped as well as the handle
    



    // rt.block_on(async {
    //     let handle = rt.spawn(async {
    //         println!("Hello Senyo");
    //         5
    //     });

    //     let value = handle.await;
    //     println!("Value: {}", value);
    // });

    // println!("Got {}", out);

    // woi::Runtime::block_on(async move {
    //     let listener = TcpListener::bind("127.0.0.1:8080").await?;

    //     loop {
    //         let (mut socket, _) = listener.accept().await?;

    //         woi::spawn(async move {
    //             let mut buf = [0; 1024];

    //             // In a loop, read data from the socket and write the data back.
    //             loop {
    //                 let n = match socket.read(&mut buf).await {
    //                     // socket closed
    //                     Ok(n) if n == 0 => return,
    //                     Ok(n) => n,
    //                     Err(e) => {
    //                         eprintln!("failed to read from socket; err = {:?}", e);
    //                         return;
    //                     }
    //                 };

    //                 // Write the data back
    //                 if let Err(e) = socket.write_all(&buf[0..n]).await {
    //                     eprintln!("failed to write to socket; err = {:?}", e);
    //                     return;
    //                 }
    //             }
    //         });
    //     }
    // })
}
