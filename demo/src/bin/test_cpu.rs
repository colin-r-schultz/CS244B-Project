use std::env::args;
use std::thread;
use std::time::{Duration, Instant};

use demo::make_config;
use dfut::{dfut_procs, Node};

dfut_procs! {
#![resources(dfut::resource::CpuResources)]

#[requires(cpus(1))]
async fn slow() -> () {
    cpus.run(|| {
    thread::sleep(Duration::from_secs(1))
    }).await.await;
}

async fn fast() -> () {

}

async fn dfut_main() -> () {
    let start = Instant::now();
    let ss = [dfut::spawn(slow()), dfut::spawn(slow()), dfut::spawn(slow())];
    let f = dfut::spawn(fast());
    f.await;
    println!("fast took {:?}", start.elapsed());
    for s in ss {
        s.await
    }
    println!("slow took {:?}", start.elapsed());
}

}

fn main() {
    let config = make_config! {
        0: {cpus: 3}
    };
    let id = args().nth(1).unwrap().parse().unwrap();
    let node = Node::new(id, config).unwrap();
    node.start((id == 0).then(dfut_main));
}
