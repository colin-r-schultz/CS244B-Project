use std::collections::HashMap;
use std::env::args;
use std::time::Duration;

use dfut::{dfut_procs, Node};

dfut_procs! {
    async fn add(a: i32, b: i32) -> i32 {
        a + b
    }

    async fn dfut_main() -> () {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let fut = dfut::spawn(add(2, 3));
        let res = dfut::spawn(add(2, fut)).await;
        println!("2 + 3 = {}", res);
    }
}

fn main() {
    let addrs = HashMap::from([
        (0, "127.0.0.1:8000".parse().unwrap()),
        (1, "127.0.0.1:8001".parse().unwrap()),
    ]);
    let id = args().nth(1).unwrap().parse().unwrap();
    let node = Node::new(id, addrs).unwrap();
    node.start((id == 0).then(dfut_main));
}
