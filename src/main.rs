mod cache;
mod parser;
mod server;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

fn main() {
    println!("Hello world");

    server::start_server();

    /*let new_node = cache::lru::Cache::new(
        String::from("This should be tail"),
        String::from("This is the value of tail"),
        10,
    );
    let new_node1 = cache::lru::Node::new(
        String::from("This should be head"),
        String::from("This is the value of head"),
    );
    let new_rc = Rc::new(RefCell::new(new_node1));
    let new_lock = Arc::new(RwLock::new(new_node));
    cache::lru::Cache::set(Arc::clone(&new_lock), new_rc).unwrap();
    // Instead of printing `new_lock` directly, print its read guard!
    println!("made a new node:{:?}", new_lock.read().unwrap());*/
}
