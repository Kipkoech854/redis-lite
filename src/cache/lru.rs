//use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::rc::{Rc, Weak};
use std::sync::{Arc, RwLock};

//[TODO] figure out a way to store only one key so its not duplicated in the hasmap key and node key
#[derive(Debug, Clone)]
pub struct Node {
    key: String,
    value: String,
    prev: Option<Weak<RefCell<Node>>>,
    next: Option<Rc<RefCell<Node>>>,
}

pub struct RetNode {
    pub value: String,
}

impl Node {
    pub fn new(key: String, value: String) -> Self {
        Node {
            key: key,
            value: value,
            prev: None,
            next: None,
        }
    }
}

#[derive(Debug)]
pub struct Cache {
    head: Option<Rc<RefCell<Node>>>,
    tail: Option<Rc<RefCell<Node>>>,
    map: HashMap<String, Rc<RefCell<Node>>>,
    capacity: usize,
    size: usize,
}

impl Cache {
    pub fn new(capacity: usize) -> Self {
        Cache {
            head: None,
            tail: None,
            map: HashMap::new(),
            capacity: capacity,
            size: 0,
        }
    }

    pub fn set(cache: Arc<RwLock<Cache>>, node: Rc<RefCell<Node>>) -> Result<(), Box<dyn Error>> {
        let mut set_grd = cache.write().unwrap();

        // Read the key early (This is read-only and safe)
        let node_key = node.borrow().key.clone();
        //  Clone the node pointer for the map BEFORE moving the original into add_to_front later
        let map_node_pointer = Rc::clone(&node);

        // Check if it exists
        if set_grd.map.contains_key(&node_key) {
            drop(set_grd);
            //[TODO] use a signal for move to front
            Self::move_to_front(Arc::clone(&cache), map_node_pointer)?;
            return Ok(());
        }

        // [TODO] Check capacity and evict_tail here before adding new items

        // Run the list manipulation. If this panics, the cache state below isn't mutated.
        Self::add_to_front(node, &mut *set_grd)?;

        // safely commit changes to the cache state
        set_grd.map.insert(node_key, map_node_pointer);
        set_grd.size += 1;

        Ok(())
    }

    pub fn add_to_front(node: Rc<RefCell<Node>>, cache: &mut Cache) -> Result<(), Box<dyn Error>> {
        // 1. Take the current head out of the cache cleanly.
        // .take() leaves `None` in the cache temporarily and gives us ownership of the old head.
        let exhead = cache.head.take();

        // 2. Configure the new head node's pointers
        {
            let mut adding_node = node.borrow_mut();

            // Next points to the old head (if it existed)
            adding_node.next = exhead.clone();

            // Head node's PREV must ALWAYS be None!
            adding_node.prev = None;
        } // Borrow drops here safely

        // 3. If there was an old head, update its `prev` pointer to point back to our new node
        if let Some(old_head_rc) = exhead {
            old_head_rc.borrow_mut().prev = Some(Rc::downgrade(&node));
        } else {
            // If there was NO old head, it means the cache was empty!
            // Therefore, this new node is also the TAIL.
            cache.tail = Some(Rc::clone(&node));
        }

        // Set the cache head to our new node
        cache.head = Some(node);

        Ok(())
    }

    pub fn move_to_front(
        cache: Arc<RwLock<Cache>>,
        node: Rc<RefCell<Node>>,
    ) -> Result<(), Box<dyn Error>> {
        //the thread tries to acquire the lock  write() is blocking and will cause the thread to wait for the lock
        // [TODO]  implement try_write to avoid blocking in case of many request
        // 1. Acquire write lock
        let mut set_grd = cache.write().unwrap();

        // 2. Check if it's already the head (Using the Option comparison we discussed!)
        /*if set_grd.head.as_ref() == Some(&node) {
            println!("The node is already head");
            return Ok(());
        }*/

        // 3. SAFELY extract neighbor pointers without holding borrows open
        // We clone the Options (cheap 8-byte pointer increments) so we can drop the borrow instantly.
        let (prev_opt, next_opt) = {
            let borrowed = node.borrow();
            (borrowed.prev.clone(), borrowed.next.clone())
        }; // The borrow on `node` drops right here! Safe from deadlocks.

        // 4. Stitch the neighbors together (Bypass the moving node)
        match (prev_opt, next_opt) {
            (None, Some(next)) => {
                // Case C: Node is the HEAD (Handled at the top, but keeping logic sound)
                next.borrow_mut().prev = None;
                set_grd.head = Some(next);
            }
            (Some(prev), Some(next)) => {
                // Case A: Node is in the middle of the list
                if let Some(prev_rc) = prev.upgrade() {
                    prev_rc.borrow_mut().next = Some(Rc::clone(&next));
                    next.borrow_mut().prev = Some(Rc::downgrade(&prev_rc));
                }
            }
            (Some(prev), None) => {
                // Case B: Node is the TAIL (No next node)
                if let Some(prev_rc) = prev.upgrade() {
                    prev_rc.borrow_mut().next = None;
                    set_grd.tail = Some(prev_rc); // The previous node becomes the new tail
                }
            }

            (None, None) => {
                // Case D: It's the only item in the list, do nothing
            }
        }

        // 5. Now that the node is completely detached from its old position,
        // clear its old pointers before sending it to the front.
        {
            let mut borrowed = node.borrow_mut();
            borrowed.prev = None;
            borrowed.next = None;
        }

        // 6. Push to front
        // Ensure your add_to_front accepts a &mut Cache reference or uses set_grd
        let result = Self::add_to_front(node, &mut *set_grd).unwrap();

        Ok(result)
    }
    pub fn evict_tail(cache: Arc<RwLock<Cache>>) -> Result<(), Box<dyn Error>> {
        // Attempt to get the write lock non-blocking
        let mut set_grd = match cache.try_write() {
            Ok(guard) => guard,
            Err(_) => return Ok(()), // Lock is busy, skip this eviction attempt safely
        };

        // Take the current tail out of the cache struct
        // We use .take() to pull the Rc out of Option, leaving None in its place temporarily
        if let Some(old_tail_rc) = set_grd.tail.take() {
            //  Borrow the old tail to read its key and its previous node pointer
            let old_tail_borrow = old_tail_rc.borrow();
            let key_to_remove = old_tail_borrow.key.clone();
            let prev_node_option = old_tail_borrow.prev.clone();

            // Drop this borrow immediately so we don't cause a runtime RefCell panic
            // when modifying pointers next!
            drop(old_tail_borrow);

            if let Some(prev_weak) = prev_node_option {
                // Upgrade the weak pointer to an Rc to use it
                if let Some(prev_rc) = prev_weak.upgrade() {
                    prev_rc.borrow_mut().next = None;
                    set_grd.tail = Some(prev_rc);
                }
            }

            // This drops the HashMap's strong Rc reference to the node
            set_grd.map.remove(&key_to_remove);

            // Explicitly dropping old_tail_rc here guarantees the reference count hits 0.
            // Because it's removed from the list and the map, it is fully deallocated!
            drop(old_tail_rc);

            set_grd.size -= 1;
        }

        Ok(())
    }

    pub fn get(cache: Arc<RwLock<Cache>>, key: String) -> Option<RetNode> {
        // Acquire a read lock from the RWLock, converting Result to Option
        let read_grd = cache.read().ok()?;

        // look if the key is in the hasmap
        if let Some(node_rc) = read_grd.map.get(&key) {
            // Ask refcell for read-only access to the node
            let borrowed_node = node_rc.borrow();

            // return a clone of the value
            // cant return a reference to something inside a local refcell
            return Some(RetNode {
                value: borrowed_node.value.clone(),
            });
        }

        None
    }
}
