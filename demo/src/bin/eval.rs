use std::collections::HashMap;
use std::env::args;
use std::future::IntoFuture;
use std::time::Instant;

use dfut::resource::{ResourceConfig, Resources};
use dfut::{dfut_procs, Node};

pub struct DummyResources;

impl Resources for DummyResources {
    fn from_config(_config: &ResourceConfig) -> Self {
        Self
    }
}

impl DummyResources {
    pub fn dummy<const _N: usize>(&self) {}
}

dfut_procs! {
#![resources(DummyResources)]

#[requires(dummy(1) as _dummy)]
async fn noop() -> () {}

async fn dfut_main(n_tasks: usize) -> () {
    let mut tasks = Vec::with_capacity(n_tasks);
    let start = Instant::now();
    for _ in 0..n_tasks {
        tasks.push(IntoFuture::into_future(dfut::spawn(noop())))
    }
    for fut in tasks.drain(..) {
        fut.await;
    }
    let dur = start.elapsed();
    println!("{}", dur.as_secs_f64());
}

}

fn main() {
    let mut args = args();

    let n_nodes = args.nth(1).unwrap().parse().unwrap();
    let id = args.next().unwrap().parse().unwrap();
    let n_tasks: usize = args.next().unwrap().parse().unwrap();
    let mut config = HashMap::new();
    for i in 0..n_nodes {
        config.insert(
            i,
            (
                format!("127.0.0.1:800{i}").parse().unwrap(),
                if i == 0 {
                    HashMap::new()
                } else {
                    HashMap::from([("dummy".to_owned(), 1)])
                },
            ),
        );
    }
    let node = Node::new(id, config).unwrap();
    node.start((id == 0).then(|| dfut_main(n_tasks)));
}
