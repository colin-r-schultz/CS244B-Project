use std::collections::HashMap;
use std::env::args;
use std::time::Duration;

use dfut::{dfut_procs, Node};
use serde::{Deserialize, Serialize};

dfut_procs! {


async fn fib(n: i32) -> i32 {
    if n <= 1 {
        1
    } else {
        dfut::spawn(add(dfut::spawn(fib(n-1)), dfut::spawn(fib(n-2)))).await
    }
}

async fn add(a: i32, b: i32) -> i32 {
    println!("{} + {}", a, b);
    a + b
}

async fn dfut_main() -> () {
    use dfut::macros::support::DFutCall;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let dfut1 = dfut::spawn(add(1, 2));
    let dfut2 = dfut::spawn(add(3, 4));
    let call = add(dfut1, dfut2);
    for x in call.get_dfut_deps() {
        println!("dep: {:?}", x);
    }
    println!("result = {}", call.await);
}

}

fn main() {
    let addrs = HashMap::from([
        (0, "127.0.0.1:8000".parse().unwrap()),
        (1, "127.0.0.1:8001".parse().unwrap()),
        (2, "127.0.0.1:8002".parse().unwrap()),
        // (3, "127.0.0.1:8003".parse().unwrap()),
        // (4, "127.0.0.1:8004".parse().unwrap()),
    ]);
    let id = args().nth(1).unwrap().parse().unwrap();
    let node = Node::new(id, addrs).unwrap();
    node.start((id == 0).then(dfut_main));
}
