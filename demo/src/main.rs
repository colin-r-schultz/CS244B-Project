use std::collections::HashMap;
use std::env::args;
use std::time::Duration;

use dfut::{dfut_procs, Node};
use serde::{Deserialize, Serialize};

dfut_procs! {
#![resources(dfut::resource::CpuResources)]

async fn fib(n: i32) -> i32 {
    if n <= 1 {
        println!("1");
        1
    } else {
        dfut::spawn(add(dfut::spawn(fib(n-1)), dfut::spawn(fib(n-2)))).await
    }
}

#[requires(cpus(1))]
async fn add(a: i32, b: i32) -> i32 {
    println!("{} + {}", a, b);
    a + b
}

async fn dfut_main() -> () {
    println!("result = {}", dfut::spawn(fib(16)).await);
}

}

fn main() {
    let addrs = HashMap::from([
        (0, ("127.0.0.1:8000".parse().unwrap(), HashMap::new())),
        (1, ("127.0.0.1:8001".parse().unwrap(), HashMap::from([]))),
        (
            2,
            (
                "127.0.0.1:8002".parse().unwrap(),
                HashMap::from([("cpus".to_owned(), 1)]),
            ),
        ),
        // (3, "127.0.0.1:8003".parse().unwrap()),
        // (4, "127.0.0.1:8004".parse().unwrap()),
    ]);
    let id = args().nth(1).unwrap().parse().unwrap();
    let node = Node::new(id, addrs).unwrap();
    node.start((id == 0).then(dfut_main));
}
