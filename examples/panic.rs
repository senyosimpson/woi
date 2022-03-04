use std::panic;

use woi::Runtime;

#[allow(unreachable_code)]
fn main() {
    tracing_subscriber::fmt::init();

    // Set the panic to do nothing
    panic::set_hook(Box::new(|_| {}));

    let rt = Runtime::new();
    rt.block_on(async {
        let jh = woi::spawn(async move {
            println!("We are about to panic");
            panic!("Panicking!");
            5
        });

        let output = jh.await;
        let err = output.unwrap_err();
        println!("However, we recovered :)");
        println!("Encountered error: {:#?}", err);
    })
}