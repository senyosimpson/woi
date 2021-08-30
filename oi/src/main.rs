use woi;
// use woi::net::TcpListener;
// use woi::io::{AsyncReadExt, AsyncWriteExt};

fn main() {
    // woi::Runtime::block_on(async {
    //     println!("Hello Senyo");
    // });

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
