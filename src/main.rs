mod cache;

fn main(){
    println!("Hello world");
    let new_node = cache::lru::Node::new(String::from("This is te key"), String::from("This is the value"));
    println!("made a new node:{:?}", new_node);
}
