use std::env::args;

use dfut::{dfut_procs, Node};

dfut_procs! {

async fn fib(n: u64) -> u64 {
    if n <= 1 {
        1
    } else {
        dfut::spawn(add(dfut::spawn(fib(n-1)), dfut::spawn(fib(n-2)))).await
    }
}

async fn add(a: u64, b: u64) -> u64 {
    println!("{a} + {b}");
    a + b
}

async fn dfut_main(n: u64) -> () {
    println!("result = {}", dfut::spawn(fib(n)).await);
}

}

fn main() {
    let config = demo::make_config! {
        0: {},
        1: {},
        2: {},
        3: {}
    };
    let mut args = args();
    let id = args.nth(1).unwrap().parse().unwrap();
    let main = if id == 0 {
        let n = args.next().map_or(20, |s| s.parse().unwrap());
        println!("Computing fib({n})");
        Some(dfut_main(n))
    } else {
        None
    };
    let node = Node::new(id, config).unwrap();
    node.start(main);
}
