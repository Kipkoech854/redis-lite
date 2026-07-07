use crate::{
    cache::{
        lru::{self, Cache, Node, RetNode},
        store,
    },
    parser::{
        self,
        ParseResult::{Failure, Success},
    },
};
use std::{
    cell::RefCell,
    error::Error,
    fs,
    hash::{Hash, Hasher},
    io::{BufReader, prelude::*},
    net::{TcpListener, TcpStream},
    rc::Rc,
    thread,
};
use std::{
    hash::DefaultHasher,
    sync::{Arc, Mutex, RwLock, mpsc},
};

pub struct HttpResponse {
    pub status_line: String,
    pub content_length: usize,
    pub content: String,
}

impl HttpResponse {
    // This is the "new" constructor method
    pub fn new(status: &str, content: String) -> Self {
        Self {
            status_line: status.to_string(),
            content_length: content.len(),
            content,
        }
    }
}
struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}
//struct Job;
type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    fn new(size: usize) -> ThreadPool {
        //panic if size is less than 0
        assert!(size > 0);
        //create the channel to send and receive connections
        let (sender, receiver) = mpsc::channel();
        //create a new atomic reference of a mutex to receiver since it will be used by the multiple workers
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }
        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }
    fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}
impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
        for worker in self.workers.drain(..) {
            worker.thread.join().unwrap();
        }
    }
}

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}
impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let job = receiver.lock().unwrap().recv().unwrap();
                job();
            }
        });
        Worker { id, thread }
    }
}

pub fn start_server() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let threads = 4;
    // 1. Initialize your thread pools
    let network_pool = ThreadPool::new(threads);
    let cache_store = Arc::new(store::start_cache_store(threads, 1000)); // Wrap in Arc so HTTP threads can share handles to it

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let cache_store_clone = Arc::clone(&cache_store);

        // 2. Dispatch the network handling to your connection thread pool
        network_pool.execute(move || {
            handle_connection(stream, cache_store_clone, threads);
        });
    }
}

fn handle_connection(
    mut stream: TcpStream,
    cache_store: Arc<store::StoreThreadPool>,
    threads: usize,
) {
    let mut buf_reader = BufReader::new(&stream);
    println!("{:?}", buf_reader);
    let request = match parser::parse_request(&mut buf_reader) {
        // We bind the Success data to the variable 'req'
        Success(req) => req,
        Failure(resp) => {
            // Send the response immediately
            let _ = write!(
                stream,
                "{}\r\nContent-Length: {}\r\n\r\n{}",
                resp.status_line, resp.content_length, resp.content
            );
            return;
        }
    };
    if request.method == "POST" {
        //[TODO] make sure post requests are properly routed
        return;
    };
    //[TODO] check request header permissions
    // [TODO] ensure we check what the request type is for proper routing
    // get key from parsed request
    let key = request.path.clone();

    //figure out what cache thread could be holding this key
    let thread_id = get_shard_id(&key, threads);

    //create a temporary cross thread reply channel
    let (reply_sender, reply_receiver) = mpsc::channel::<Option<RetNode>>();

    cache_store.execute_on(thread_id, move |cache_arc| {
        //[TODO] ensure the get method trigers rearrangement of  the cache ....move to front
        // Also ensure that we dont hit the db if it was something like a search that would probably need to return related content
        let result = Cache::get(cache_arc, key);
        //send request back to the waiting network thread
        let _ = reply_sender.send(result);
    });
    //The network thread is blocked until the shard responds
    let cache_result = reply_receiver.recv().unwrap();

    //do the next thing based on the result from the cache
    if let Some(node) = cache_result {
        //if the resource was found send it back
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            node.value.len(),
            node.value
        );
    } else {
        //route them to db or return the response was not found
        // if we hit the db make sure we parse the response to ensure what was being asked for should be cached
        // This will be based on if it was a search result
    }
    //stream.write_all(response.as_bytes()).unwrap();
}
pub fn get_shard_id(key: &str, num_shards: usize) -> usize {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    (hasher.finish() % num_shards as u64) as usize
}
/*if request.method == "POST" {
    let body = match request.body {
        Some(body) => body,
        None => {
            let response = HttpResponse::new(
                "400",
                "Error :request body not found for a POST request".to_string(),
            );
            let _ = write!(
                stream,
                "HTTP/1.1 {} \r\nContent-Length: {}\r\n\r\n{}",
                response.status_line, response.content_length, response.content
            );
            return;
        }
    };
    cache_store.execute_on(thread_id, move |cache_arc| {
        let node = Rc::new(RefCell::new(Node::new(request.path, body)));
        let result = Cache::set(cache_arc, node);
        match result {
            Ok(()) => {
                let _ = reply_sender.send(Some(RetNode {
                    value: "Successfully added with post request".to_string(),
                }));
            }
            Err(e) =>{}
        }
    })
}; */
