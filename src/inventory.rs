use std::{
    sync::Mutex,
    sync::Arc,
};

use crate::{
    aid::AID, 
};

struct Inventory { // Think about moving the Arc to be the entire inventory
    max: usize,
    items: Arc<Mutex<(String, usize)>>, // Resources are stored as (String, usize) where String is name of resource and usize is count
}

pub enum InventoryMessage {
    // The following are sent by owner (entity)
    Increase,
    Decrease,
    TakeFrom(AID<InventoryMessage>), 
    Kill,

    // The following are sent by another inventory
    MoveTo(Inventory),  
}

pub mod inventory {
    use std::{
        sync::{Arc, Mutex, MutexGuard}
    };

    use crate::{
        aid::AID, 
        inventory::{Inventory, InventoryMessage}
    };

    pub fn init() -> AID<InventoryMessage> {
        return AID::new(main_loop);
    }

    fn construct_inventory() -> Inventory {
        let inv_arc: Arc<Mutex<(String, usize)>> = Arc::new(
                                                Mutex::new((String::from("Mutexium"), 0))
                                                    );
        return Inventory { max: 10, items: inv_arc };
    }

    fn main_loop(_aid: AID<InventoryMessage>, mailbox: std::sync::mpsc::Receiver<InventoryMessage>) {
        let inventory: Inventory = construct_inventory();

        loop {
            for msg in &mailbox {
                match msg {
                    InventoryMessage::Increase => increase(&inventory),
                    InventoryMessage::Decrease => decrease(&inventory),
                    InventoryMessage::TakeFrom(other) => take_from(&inventory, other),
                    InventoryMessage::Kill => return,

                    InventoryMessage::MoveTo(other_inventory) => move_to(&inventory, &other_inventory), // This is a placeholder
                };
            }
        }
    }

    // Increase this inventory by one, will not do anything if inventory is full.
    fn increase(inventory: &Inventory) {
        if is_full(inventory) {
            return; // Error or something
        }

        let mut contents: MutexGuard<'_, (String, usize)> = inventory.items.lock().unwrap(); 
        (*contents).1 += 1;

        return; // Success or something
    }

    // Decrease this inventory by one, will not do anything if inventory is empty.
    fn decrease(inventory: &Inventory) {
        if is_empty(inventory) {
            return; // Error or something
        }

        let mut contents: MutexGuard<'_, (String, usize)> = inventory.items.lock().unwrap(); 
        (*contents).1 -= 1;

        return; // Success or something
    }

    // Moves an item from 'from to 'to'. 
    fn move_to(from: &Inventory, to: &Inventory) {
        if is_empty(from) || is_full(to) {
            return; // ERROR or something
        }

        decrease(from);
        increase(to);

        return; // Success or something
    }

    fn take_from(inventory: &Inventory, aid: AID<InventoryMessage>) {
        aid.send(InventoryMessage::MoveTo(inventory));
    }

    fn is_empty(inventory: &Inventory) -> bool { // Could be a method of Inventory
        return (*inventory.items.lock().unwrap()).1 == 0;
    }

    fn is_full(inventory: &Inventory) -> bool { // Could be a method of Inventory
        return (*inventory.items.lock().unwrap()).1 == inventory.max;
    }

    // For debugging as of now
    fn print_inv(this_items: &Arc<Mutex<(String, usize)>>, name: String) {
        let contents: MutexGuard<'_, (String, usize)> = this_items.lock().unwrap();

        println!("{0} - {1}", name, (*contents).1);
    }
}