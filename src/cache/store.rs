use crate::cache::lru::Cache;
use std::sync::{Arc, RwLock, mpsc};
use std::thread;

pub struct StoreThreadPool {
    workers: Vec<Worker>,
    senders: Vec<mpsc::Sender<Job>>,
}
//struct Job;
//[TODO] make it so it takes only cache
// no need of atomic reference since a thread owns a cache instance
type Job = Box<dyn FnOnce(Arc<RwLock<Cache>>) + Send + 'static>;

impl StoreThreadPool {
    fn new(size: usize, cache_capacity: usize) -> StoreThreadPool {
        //panic if size is less than 0
        assert!(size > 0);
        let mut workers = Vec::with_capacity(size);
        let mut senders = Vec::with_capacity(size);
        for id in 0..size {
            let worker = Worker::new(id, size, cache_capacity);
            senders.push(worker.sender.clone());
            workers.push(worker);
        }
        StoreThreadPool { workers, senders }
    }
    pub fn execute_on<F>(&self, thread_id: usize, f: F)
    where
        F: FnOnce(Arc<RwLock<Cache>>) + Send + 'static,
    {
        // Wrap the user's closure 'f' inside our Job box
        let job: Job = Box::new(|cache_arc| {
            // When the worker runs this job, we pass the thread's cache into 'f'
            f(cache_arc);
        });

        self.senders[thread_id].send(job).unwrap();
    }
}
impl Drop for StoreThreadPool {
    fn drop(&mut self) {
        // Phase 1: Drop all senders to signal workers to stop.
        // Clearing the vector drops all the Sender instances inside it.
        self.senders.clear();

        // Phase 2: Drain the workers vector and join each thread sequentially.
        // .drain(..) removes the workers from the Vec so we can take ownership of the join handles.
        for worker in self.workers.drain(..) {
            worker.thread.join().unwrap();
        }
    }
}

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
    sender: mpsc::Sender<Job>,
}
impl Worker {
    fn new(id: usize, size: usize, cache_capactity: usize) -> Worker {
        let (sender, receiver) = mpsc::channel::<Job>();
        let capacity = cache_capactity / size;
        let thread = thread::spawn(move || {
            // Create the Arc/RwLock right here inside the thread!
            // It is completely owned by this thread, keeping Rc/RefCell happy.
            let local_cache = Arc::new(RwLock::new(Cache::new(capacity)));
            //loop only through the workers messages
            //This is because the workers own the cache and are responsible for what happens in them
            // Because of this sharding it means the requests will need to be routed to specific workers
            // This is implemented using hashing
            for job in receiver {
                job(Arc::clone(&local_cache));
            }
        });

        Worker { id, thread, sender }
    }
}

pub fn start_cache_store(shards: usize, capacity: usize) -> StoreThreadPool {
    StoreThreadPool::new(shards, capacity)
}
