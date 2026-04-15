use std::{
    sync::Mutex,
    sync::Arc,
};

use crate::{
    aid::AID, 
};

pub struct Inventory {
    max: usize,
    items: (String, usize), // Resources are stored as (String, usize) where String is name of resource and usize is count
}

#[derive(Clone)]
pub enum InventoryMessage {
    // The following are sent by owner (entity)
    Increase,
    Decrease,
    TakeFrom(AID<InventoryMessage>), 
    Kill,

    // The following are sent by another inventory
    MoveTo(Arc<Mutex<Inventory>>),  
}

pub mod inventory {
    use std::{
        sync::{Arc, Mutex, MutexGuard}
    };

    use crate::{
        aid::AID, 
        inventory::{Inventory, InventoryMessage}
    };


    /// Initializes a new inventory and returns its AID
    pub fn init() -> AID<InventoryMessage> {
        return AID::new(inventory_loop);
    }

    /// Constructs the inventory
    /// 
    /// NOTE: Could be moved to be a method of inventory if that is desired
    fn construct_inventory() -> Arc<Mutex<Inventory>> {
        return Arc::new(Mutex::new(Inventory { 
                                            max: 10,
                                            items: (String::from("Mutexium"), 0),
                                        }));
    }

    fn inventory_loop(
            _aid: AID<InventoryMessage>, 
            mailbox: std::sync::mpsc::Receiver<InventoryMessage>
        ){
        let inventory: Arc<Mutex<Inventory>> = construct_inventory();

        loop {
            match mailbox.recv().unwrap() { // Recv blocks the thread until a message is recieved which seems right for this
                InventoryMessage::Increase => increase(inventory.clone()),
                InventoryMessage::Decrease => decrease(inventory.clone()),
                InventoryMessage::TakeFrom(other) => 
                    take_from(inventory.clone(), other),
                InventoryMessage::Kill => return,

                InventoryMessage::MoveTo(other_inventory) => 
                    move_to(inventory.clone(), other_inventory),
            };

            print_inv(&inventory, String::from("Inventory"));
        }
    }

    /// Increase this inventory by one, will not do anything if inventory is full.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory' - Atomic Reference Counter to the inventory to increase
    fn increase(inventory: Arc<Mutex<Inventory>>) {
        if is_full(inventory.clone()) {
            return; // Error or something
        }

        let mut contents: MutexGuard<'_, Inventory> = inventory.lock().unwrap(); 
        (*contents).items.1 += 1;

        return; // Success or something
    }

    /// Decrease this inventory by one, will not do anything if inventory is empty.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory' - Atomic Reference Counter to the inventory to increase
    fn decrease(inventory: Arc<Mutex<Inventory>>) {
        if is_empty(inventory.clone()) {
            return; // Error or something
        }

        let mut contents: MutexGuard<'_, Inventory> = inventory.lock().unwrap(); 
        (*contents).items.1 -= 1;

        return; // Success or something
    }

    /// Moves an item from 'from to 'to'. 
    /// 
    /// # Arguments
    /// 
    /// * 'from' - Atomic Reference Counter to inventory to move the item from
    /// * 'to' - Atomic Reference Counter to inventory to move item to
    fn move_to(from: Arc<Mutex<Inventory>>, to: Arc<Mutex<Inventory>>) {
        if is_empty(from.clone()) || is_full(to.clone()) {
            return; // ERROR or something
        }

        decrease(from.clone());
        increase(to.clone());

        return; // Success or something
    }

    /// Asks the other inventory to perform the move_to function with 
    /// this inventory as 'to' parameter
    /// 
    /// # Arguments
    /// 
    /// * 'inventory' - Atomic Reference Counter to calling inventory 
    /// * 'aid' - AID of the inventory to take from
    fn take_from(inventory: Arc<Mutex<Inventory>>, aid: AID<InventoryMessage>) {
        _ = aid.send(InventoryMessage::MoveTo(inventory)); // Should handle Result in some way
    }


    // Non message functions
    fn is_empty(inventory: Arc<Mutex<Inventory>>) -> bool { // Could be a method of Inventory
        return (*inventory.lock().unwrap()).items.1 == 0;
    }

    fn is_full(inventory: Arc<Mutex<Inventory>>) -> bool { // Could be a method of Inventory
        let unlocked_inv: &Inventory = &(*inventory.lock().unwrap());
        return unlocked_inv.items.1 == unlocked_inv.max;
    }

    // For debugging as of now
    fn print_inv(this_items: &Arc<Mutex<Inventory>>, name: String) {
        let contents: MutexGuard<'_, Inventory> = this_items.lock().unwrap();

        println!("{0}: {1} - {2}/{3}", name, (*contents).items.0, (*contents).items.1, (*contents).max);
    }
}