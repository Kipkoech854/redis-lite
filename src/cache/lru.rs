use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;

use std::rc::Rc;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct Node {
    key: Box<String>,
    value: String,
    prev: Option<Rc<RefCell<Node>>>,
    next: Option<Rc<RefCell<Node>>>,
}

impl Node {
    pub fn new(key: String, value: String) -> Self {
        Node {
            key: Box::new(key),
            value: value,
            prev: None,
            next: None,
        }
    }
}

pub struct Cache {
    head: Rc<RefCell<Node>>,
    tail: Rc<RefCell<Node>>,
    map: HashMap<String, Rc<RefCell<Node>>>,
    capacity: usize,
    size: usize,
}

impl Cache {
    pub fn new(node: &Node, capacity: usize) -> Self {
        let ptr = Rc::new(RefCell::new(node.clone()));
        let mut hash = HashMap::new();
        hash.insert(*node.key.clone(), Rc::clone(&ptr));
        Cache {
            head: Rc::clone(&ptr),
            tail: Rc::clone(&ptr),
            map: hash,
            capacity: capacity,
            size: 1,
        }
    }

    pub fn set(cache: Arc<RwLock<Cache>>, node: Rc<RefCell<Node>>) -> Result<(), dyn Error> {
        //the thread tries to acquire the lock  write() is blocking and will cause the thread to wait for the lock
        // [TODO]  implement try_write to avoid blocking in case of many request
        let mut set_grd = cache.write().unwrap();
        //[TODO] call move to front instead of  a new set if it exists
        if (*set_grd).map.contains_key(&node.key) {
            return Ok(());
        }
        //[TODO] check the size of cache against capacity to alert evict tail if capacity is close to full
        //if the key doesnt exist add to front
        Self::add_to_front(node, set_grd).unwrap();
        Ok(())
    }

    pub fn add_to_front(
        node: Rc<RefCell<Node>>,
        write_grd: RwLockWriteGuard<'_, Cache>,
    ) -> Result<(), dyn Error> {
        //extract details like the cache's head, tail and capaity that will be used in adding ro front
        let (exhead, capacity) = ((write_grd).head, (write_grd).capacity);
        //access the new node being added to front to mutate the next pointer to exhead and prev pointer to the node being added
        let adding_node = node.try_borrow_mut().unwrap();
        (adding_node.next, adding_node.prev) = (exhead, Some(node));

        //change the head ptr
        write_grd.head = node;
        // insert the new nodes key and the node the hashmap already in cache
        write_grd.map.insert(*adding_node.key, node);
        //add 1 to the size
        // [TODO] update size to actually use bytes
        write_grd.size += 1;

        Ok(())
    }
    pub fn move_to_front(
        cache: Arc<RwLock<Cache>>,
        node: Rc<RefCell<Node>>,
    ) -> Result<(), dyn Error> {
        //the thread tries to acquire the lock  write() is blocking and will cause the thread to wait for the lock
        // [TODO]  implement try_write to avoid blocking in case of many request
        let mut set_grd = cache.write().unwrap();
        //check if the node we want to move is the head and if it is return the cache unchanged
        if set_grd.head == node {
            println!("The node is already head");
            return Ok(());
        }
        ////access the new node being added to front to mutate the next pointer to exhead and prev pointer to the node being added
        let moving_node = node.try_borrow_mut().unwrap();
        //get the previos node to the moving node && the next node
        let moving_prev_node = moving_node.prev.try_borrow_mut().unwrap();
        let moving_next_node = moving_node.next.try_borrow_mut().unwrap();
        // check if the node being moved is the tail to avoid seting an invalid next node
        if set_grd.tail == node {
            //set the prev's next node to None
            moving_prev_node.next = None;
            //set prev node to be the new tail
            set_grd.tail =
                moving_prev_node.expect("Error setting new tail while moving to the front");
            //call the function to move it to the front
            let moved = Self::add_to_front(node, set_grd).unwrap();
            return Ok(());
        }
        //This applies to when the node being moved is not the head nor the tail
        //Look for the previous node to th node being moved
        //change the pointer of its next node to point to the next of the node being moved
        if let Some(prev_node) = moving_prev_node {
            (prev_node).next = moving_node.next;
        }
        //look for the next node to the node being moved
        //Change the pointer of its previous node to the previous of the node being moved
        if let Some(next_node) = moving_next_node {
            next_node.prev = moving_node.prev;
        }
        //call the function add_to_front to move the node to the front as new head
        let moved = Self::add_to_front(node, set_grd);
        //return the new cache with moved values
        Ok(())
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

            if let Some(prev_rc) = prev_node_option {
                // Cut the link: Set the previous node's 'next' pointer to None
                prev_rc.borrow_mut().next = None;

                // Update the cache's tail to be this previous node
                set_grd.tail = prev_rc;
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
}
