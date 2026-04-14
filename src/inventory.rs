use std::{
    sync::Mutex,
    sync::Arc,
};

struct Inventory {
    items: Arc<Mutex<(String, usize)>>, // Resources are stored as (String, usize) where String is name of resource and usize is count
}

pub mod inventory {
    use std::{
        sync::{Arc, Mutex}, thread::{self, JoinHandle}
    };

    use crate::{inventory::Inventory};

    pub fn init(name: String) -> (JoinHandle<()>, Arc<Mutex<(String, usize)>>) {
        let inv_mut: Arc<Mutex<(String, usize)>> = Arc::new(Mutex::new((String::from("Mutexium"), 0)));
        let inventory: Inventory = Inventory { items: inv_mut.clone() };

        let handle: thread::JoinHandle<()> = thread::spawn(|| main_loop(inventory, name));

        return (handle, inv_mut);
    }

    fn main_loop(inventory: Inventory, name: String) {
        loop {
            // TODO: Do something here, this is just a placeholder
            println!("In {}...", name);

            let mut contents = inventory.items.lock().unwrap();

            (*contents).1 += 1;

            println!("{0} - {1}", (*contents).0, (*contents).1);
        }
    }
}