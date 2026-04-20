use std::{collections::HashMap};

use crate::{
    aid::AID, 
};

#[derive(Clone)]
pub enum Item {
    Mutexium,
}

impl Item {
    fn to_string(&self) -> &str {
        match self {
            Item::Mutexium => return "Mutexium",
        };
    }
}

#[derive(Clone)]
pub enum InventoryMessage {
    // The following are sent by owner (entity)
    Add((Item, usize)),
    Remove((Item, usize)),
    TakeFrom(AID<InventoryMessage>, (Item, usize)), 
    GiveTo(AID<InventoryMessage>, (Item, usize)),
    Kill,

    // The following are sent by another inventory
    GiveMeItem(AID<InventoryMessage>),                       // From TakeFrom 
    GiveMeItemResult(Result<(Item, usize), &'static str>), // From GiveMeItem
    TakeMyItem(AID<InventoryMessage>, (Item, usize)),        // From GiveTo
    TakeMyItemResult(Result<(Item, usize), &'static str>),   // From TakeMyItem
}


struct Inventory {
    aid: AID<InventoryMessage>,
    max: usize,
    waiting: Vec<AID<InventoryMessage>>,
    // items: (String, usize), // Resources are stored as (String, usize) where String is name of resource and usize is count
    items: HashMap<Item, (usize, usize)>, // Key: Name of item -> Value: (count, max).
}

impl Inventory {
    fn construct(aid: AID<InventoryMessage>) -> Self {
        // return Inventory { aid, max: 10, waiting: Vec::new(), items: (String::from("Mutexium"), 0) };
        return Inventory { aid, max: 10, waiting: Vec::new(), items: HashMap::new() };
    }

    fn is_empty(&self) -> bool {
        return self.items.1 == 0; // FIX
    }

    fn is_full(&self) -> bool {
        return self.items.1 == self.max; // FIX
    }

    // For debugging as of now
    fn print_inv(&self, name: String) {
        println!("{0}:\n", name);
        
        for (key, value) in &self.items {
            println!("      {0} - {1}/{2}\n", key.to_string(), value.0, value.1);
        }
    }
}

pub mod inventory {
    use crate::{
        aid::AID, 
        inventory::{
            Inventory, 
            InventoryMessage, 
            Item
        }
    };


    /// Initializes a new inventory and returns its AID
    pub fn init() -> AID<InventoryMessage> {
        return AID::new(inventory_loop);
    }

    fn inventory_loop(
            aid: AID<InventoryMessage>, 
            mailbox: std::sync::mpsc::Receiver<InventoryMessage>
        ){
        let mut inventory: Inventory = Inventory::construct(aid);
        let mut transfer_in_process: bool = false; 

        loop {
            match mailbox.recv().unwrap() {
                InventoryMessage::Add(item) => add(&mut inventory, item),
                InventoryMessage::Remove(item) => remove(&mut inventory, item),
                InventoryMessage::TakeFrom(other, items) => 
                    take_from(&inventory, other, items, &mut transfer_in_process),
                InventoryMessage::GiveTo(other, items) => 
                    give_to(&inventory, other, items, &mut transfer_in_process),
                InventoryMessage::Kill => return, 

                InventoryMessage::GiveMeItem(sender) => 
                    give_me_item(&mut inventory, sender, &mut transfer_in_process),
                InventoryMessage::GiveMeItemResult(result) => {
                    give_me_item_result(&mut inventory, result);
                    transfer_in_process = false;
                }
                InventoryMessage::TakeMyItem(sender, item) => 
                    take_my_item(&inventory, sender, item, &mut transfer_in_process),
                InventoryMessage::TakeMyItemResult(result) => 
                    take_my_item_result(&mut inventory, result),
            };

            inventory.print_inv(String::from("Inventory"));
        }
    }

    /// Increase this inventory by one, will not do anything if inventory is full.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory' - Mutable reference to the inventory to increase
    fn add(inventory: &mut Inventory, item: (Item, usize)) {
        if inventory.is_full() {
            return; // Error or something
        }

        inventory.items.1 += 1;

        return; // Success or something
    }

    /// Decrease this inventory by one, will not do anything if inventory is empty.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory' - Mutable reference to the inventory to decrease
    fn remove(inventory: &mut Inventory, item: (Item, usize)) {
        if inventory.is_empty() {
            return; // Error or something
        }

        inventory.items.1 -= 1;

        return; // Success or something
    }

    /// Asks the other inventory to perform the give_me_item function with 
    /// this inventorys AID as 'sender' parameter
    /// 
    /// # Arguments
    /// 
    /// * 'inventory'            - Reference to the inventory to move item to 
    /// * 'aid'                  - AID of the inventory to take from
    /// * 'transfer_in_progress' - Bool that is true if there is already a transfer in progress, else false
    fn take_from(
        inventory: &Inventory, 
        aid: AID<InventoryMessage>, 
        items: (Item, usize),
        transfer_in_progress: &mut bool
    ) {
        if *transfer_in_progress || inventory.is_full() {
            return; // Should send an error or whatever
        }

        *transfer_in_progress = true;
        _ = aid.send(InventoryMessage::GiveMeItem(inventory.aid.clone())); // Should handle Result in some way
    }

    fn give_to(
        inventory: &Inventory, 
        aid: AID<InventoryMessage>, 
        items: (Item, usize),
        transfer_in_progress: &mut bool
    ) {

    }

    /// Checks if this inventory can give item and sends a result containing either a tuple containing
    /// what item and quantity, or an error containing a string explaining what went wrong.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory'            - Mutable reference to this inventory
    /// * 'sender'               - AID of the requesting inventory
    /// * 'transfer_in_progress' - Bool that is true if there is already a transfer in progress, else false
    fn give_me_item(
        inventory: &mut Inventory, 
        sender: AID<InventoryMessage>, 
        transfer_in_progress: &mut bool
    ) {
        if *transfer_in_progress {
            _ = sender.send(InventoryMessage::GiveMeItemResult(Result::Err("I'm busy!"))); // Sends an error; TODO: Should handle Result in some way
            return;
        }

        if inventory.is_empty() {
            _ = sender.send(InventoryMessage::GiveMeItemResult(Result::Err("I'm empty!"))); // Sends an error; TODO: Should handle Result in some way
            return;
        }


        inventory.items.1 -= 1;
        _ = sender.send(InventoryMessage::GiveMeItemResult(Result::Ok((inventory.items.0.clone(), 1)))); // Sends a tuple containing what item it is and how many it moved; TODO: Should handle Result in some way
    }

    /// Gets the result from a GiveMeItem message and add the item to this inventory, or prints the
    /// error message if GiveMeItem failed.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory' - Mutable reference to this inventory
    /// * 'result'    - A result containing either a tuple of what item was moved and the quantity, 
    ///                 or the error message as a str
    fn give_me_item_result(inventory: &mut Inventory, result: Result<(Item, usize), &'static str>) {
        match result {
            Ok(item) => { inventory.items.1 += item.1; }, // This is a placeholder
            Err(msg) => { println!("{}", msg) } // should probably do something else
        };
    }

    fn take_my_item(
        inventory: &Inventory, 
        sender: AID<InventoryMessage>, 
        items: (Item, usize), 
        transfer_in_progress: &mut bool
    ) {

    }

    fn take_my_item_result(
        inventory: &mut Inventory, 
        result: Result<(Item, usize), &'static str>
    ) {

    }
}