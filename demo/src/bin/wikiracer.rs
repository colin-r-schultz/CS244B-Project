use dfut::resource::CpuResources;
use dfut::{dfut_procs, Node};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env::args;
use tokio::task::JoinSet;

#[derive(Serialize, Deserialize, Clone)]
pub struct Html(String);

fn print_wikiladder(start: String, end: String, mut parent_map: HashMap<String, String>) {
    let mut reverse_path = Vec::new();
    let mut curr = end;
    while curr != start {
        let next = parent_map.remove(&curr).unwrap();
        reverse_path.push(curr);
        curr = next;
    }
    print!("Wikiladder: {start}, ");
    for s in reverse_path.iter().rev() {
        print!("{s}, ");
    }
    println!();
}

dfut_procs! {
#![resources(CpuResources)]

#[requires(cpus(1))]
async fn find_links(html: Html) -> Vec<String> {
    const PATTERN: &'static str = "<a href=\"/wiki/";
    let res = cpus.run(move || {
        let mut res = Vec::new();
        let mut s = &html.0[..];
        while let Some(idx) = s.find(PATTERN) {
            s = &s[idx+PATTERN.len()..];
            if let Some(idx) = s.find('"') {
                res.push(s[..idx].to_owned());
                s = &s[idx..];
            } else {
                break;
            }
        }
        res
    }).await.await;
    println!("Parsed {} links", res.len());
    res
}

async fn get_html(article: String) -> Html {
    println!("Requesting {article}");
    let text = reqwest::get(format!("https://en.wikipedia.org/wiki/{article}"))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    Html(text)
}

async fn dfut_main(start: String, end: String) -> () {
    let mut parent_map = HashMap::from([(start.clone(), String::new())]);
    let mut open_set = vec![start.clone()];
    loop {
        let mut set = JoinSet::new();
        for s in open_set.drain(..) {
            set.spawn(async move {
                let html = dfut::spawn(get_html(s.clone()));
                let children: Vec<String> = dfut::spawn(find_links(html)).await;
                (s, children)
            });
        }
        while let Some(res) = set.join_next().await {
            let (s, mut children) = res.unwrap();
            for child in children
                .drain(..)
            {
                if parent_map.contains_key(&child) || child.starts_with("Special:") {
                    continue;
                }
                parent_map.insert(child.clone(), s.clone());
                if child == end {
                    return print_wikiladder(start, end, parent_map);
                }
                open_set.push(child);
            }
        }
    }
}
}

fn main() {
    let config = demo::make_config! {
        0: {}, 1: {}, 2: {cpus: 1}, 3: {cpus: 1}
    };
    let mut args = args();
    let id = args.nth(1).unwrap().parse().unwrap();
    let main = if id == 0 {
        let start = args.next().unwrap();
        let end = args.next().unwrap();
        Some(dfut_main(start, end))
    } else {
        None
    };

    let node = Node::new(id, config).unwrap();
    node.start(main);
}
