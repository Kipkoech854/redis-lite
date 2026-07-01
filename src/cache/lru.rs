use std::collections::HashMap;
use std::process::exit;
use std::error::Error;


#[derive(Debug, Clone)]
pub struct Node{
    key:Box<String>,
    value:String,
    prev: Option< *mut Node>,
    next: Option< *mut Node>,
}

impl Node{
    pub fn new(key: String, value:String)-> Self{
        Node{ key:Box::new(key), value: value, prev:None, next: None}
    }
}



pub struct Cache{
    head: *mut Node,
    tail: *mut Node,
    map: HashMap< String, *mut Node>,
    capacity:usize,
    size: usize,
}

impl Cache{
  pub unsafe  fn new(mut node: Node, capacity: usize) ->Self{
      let ptr: *mut Node = & raw mut node;
      let mut hash = HashMap::new();
      hash.insert(*node.key, ptr);
      Cache{
          head: ptr,
          tail: ptr,
          map:hash,
          capacity: capacity,
          size:1,
      }
   }
   pub unsafe fn add_to_front(cache: *mut Cache, node: *mut Node ) -> Cache{
       let (exhead, tail, capacity) = ((*cache).head, (*cache).tail, (*cache).capacity);
       (*node).next = Some(exhead);
       (*node).next = None;
       let mut hash = HashMap::new();
       hash.insert(*(*node).key.clone(), node);

       Cache{
           head: node,
           tail:tail,
           map:hash,
           capacity:capacity,
           size: (*cache).size + 1,
       }
  }
   pub unsafe fn move_to_front(cache: *mut Cache, node: *mut Node) -> Cache{
       //check if the node we want to move is the head and if it is return the cache unchanged
       if  (*cache).head == node{
           println!("The node is already head");
           exit (1);
       }
      // check if the node being moved is the tail to avoid seting an invalid next node
      if  (*cache).tail == node{
          //set the prev's next node to None
          (*node).prev = None;
          //set prev node to be the new tail
          (*cache).tail = (*node).prev.expect("Error setting new tail while moving to the front");
          //call the function to move it to the front
          let moved = Self::add_to_front(cache, node);
          return moved
          
      }
       //This applies to when the node being moved is not the head nor the tail
       //Look for the previous node to th node being moved
       //change the pointer of its next node to point to the next of the node being moved
       if let Some(prev_node) = (*node).prev{
           (*prev_node).next = (*node).next;
       } 
       //look for the next node to the node being moved
       //Change the pointer of its previous node to the previous of the node being moved
       if let Some(next_node) = (*node).next{
           (*next_node).prev = (*node).prev;
       }
       //call the function add_to_front to move the node to the front as new head
       let moved = Self::add_to_front(cache, node);
       //return the new cache with moved values
       moved
   }
   pub unsafe fn evict_tail(cache: *mut Cache) -> Result<(),dyn Error>{
       let tail = (*cache).tail;
       //set the previous nodes next node to None
       if let Some(prev) = (*tail).prev{
           let mut tail_prev = (*prev).next.unwrap();
           Some(tail_prev) = None;
           //set the tails prev node as the new tail
           (*cache).tail = prev;
       };

       //drop the original tail
       drop(tail);
       //return a success Result
       Ok(())
   }
  pub fn set(cache: *mut Cache, node: *mut Node) -> Cache {
       
  } 
}



