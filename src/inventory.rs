use std::{
    sync::Mutex,
    sync::Arc,
};

struct Inventory {
    items: Arc<Mutex<(String, usize)>>, // Resources are stored as (String, usize) where String is name of resource and usize is count
}

pub mod inventory {
    use std::{
        sync::{Arc, Mutex, MutexGuard}, thread::{self, JoinHandle}
    };

    use crate::{inventory::Inventory};

    pub fn init() -> (JoinHandle<()>, Arc<Mutex<(String, usize)>>) {
        let inv_mut: Arc<Mutex<(String, usize)>> = Arc::new(Mutex::new((String::from("Mutexium"), 0)));
        let inventory: Inventory = Inventory { items: inv_mut.clone() };

        let handle: thread::JoinHandle<()> = thread::spawn(|| main_loop(inventory));

        return (handle, inv_mut);
    }

    fn main_loop(inventory: Inventory) {
        loop {
            // // TODO: Do something here, this is just a placeholder
            // println!("In {}...", name);

            // let mut contents = inventory.items.lock().unwrap();

            // (*contents).1 += 1;

            // println!("{0} - {1}", (*contents).0, (*contents).1);
        }
    }

    /*
        These functions should be in the entities (probably) and not public
        nvm i think this works since this is a module that can just be imported
     */
    pub fn increase(/*inventory: Inventory or*/ items: &Arc<Mutex<(String, usize)>>) {
        let mut contents: MutexGuard<'_, (String, usize)> = items.lock().unwrap(); 
        (*contents).1 += 1;
    }

    pub fn decrease(/*inventory: Inventory or*/ items: &Arc<Mutex<(String, usize)>>) {
        let mut contents: MutexGuard<'_, (String, usize)> = items.lock().unwrap(); 
        (*contents).1 -= 1;
    }

    /**
     * Moves from from to to
     */
    pub fn r#move(from: &Arc<Mutex<(String, usize)>>, to: &Arc<Mutex<(String, usize)>>) {
        decrease(from);
        increase(to);
    }

    // For debugging as of now
    pub fn print_inv(this_items: &Arc<Mutex<(String, usize)>>, name: String) {
        let contents: MutexGuard<'_, (String, usize)> = this_items.lock().unwrap();

        println!("{0} - {1}", name, (*contents).1);
    }
}