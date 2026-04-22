use std::{collections::HashMap};

use crate::{
    aid::AID, 
    item::Item
};

#[derive(Clone)]
pub enum InventoryMessage {
    // The following are sent by owner (entity) (public)
    Add((Item, usize)),
    Remove((Item, usize)),
    TakeFrom(AID<InventoryMessage>, (Item, usize)), 
    GiveTo(AID<InventoryMessage>, (Item, usize)),
    PrintInventory(String),
    Kill,

    // The following are sent by another inventory (private)
    GiveMeItems(AID<InventoryMessage>, (Item, usize)),      // From TakeFrom 
    GiveMeItemResult(Result<(Item, usize), &'static str>),  // From GiveMeItems
    TakeMyItems(AID<InventoryMessage>, (Item, usize)),      // From GiveTo
    TakeMyItemsResult(Result<(Item, usize), &'static str>), // From TakeMyItems
}


struct Inventory {
    aid: AID<InventoryMessage>,
    max: usize,                           // ???
    waiting: Vec<AID<InventoryMessage>>,  // TODO: Not needed for minimum viable product
    items: HashMap<Item, (usize, usize)>, // Key: Name of item -> Value: (count, max).
}

impl Inventory {
    fn construct(aid: AID<InventoryMessage>) -> Self {
        let mut inventory = Inventory { 
            aid, 
            max: 10, 
            waiting: Vec::new(), 
            items: HashMap::new() 
        };

        inventory.items.insert(Item::Mutexium, (0, 0));
        
        return inventory;
    }
    
    fn give(&mut self, (item, count): (Item, usize)) {
        // if is_full() or whatever return false

        self.items.entry(item).or_insert((0, 0)).0 += count;

        return  /* true */;
    }

    fn take(&mut self, (item, count): (Item, usize)) -> bool {
        if self.has_too_few_items((item, count)) {
            return false;
        }

        let value = self.items.get_mut(&item).unwrap();
        value.0 -= count;
        return true;
    }

    fn has_too_few_items(&self, (item, count): (Item, usize)) -> bool {
        return self.items.get(&item).unwrap().0 < count;
    }

    fn is_full(&self) -> bool {
        return false; // FIX: not needed for minimal viable product
    }

    // For debugging as of now
    fn print_inv(&self, name: String) {
        println!("{0}:", name);
        
        for (key, value) in &self.items {
            println!("      {0} - {1}/{2}", key.to_str(), value.0, value.1);
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
        // let mut transfer_in_process: bool = false; 

        loop {
            match mailbox.recv().unwrap() {
                InventoryMessage::Add(item) => add(&mut inventory, item),
                InventoryMessage::Remove(item) => remove(&mut inventory, item),
                InventoryMessage::TakeFrom(other, items) => 
                    take_from(&inventory, other, items, /*&mut transfer_in_process*/),
                InventoryMessage::GiveTo(other, items) => 
                    give_to(&inventory, other, items, /*&mut transfer_in_process*/),
                InventoryMessage::PrintInventory(name   ) => inventory.print_inv(name),
                InventoryMessage::Kill => return, 

                InventoryMessage::GiveMeItems(sender, item) => 
                    give_me_items(&mut inventory, sender, item, /*&mut transfer_in_process*/),
                InventoryMessage::GiveMeItemResult(result) => {
                    give_me_items_result(&mut inventory, result);
                    // transfer_in_process = false;
                }
                InventoryMessage::TakeMyItems(sender, item) => 
                    take_my_items(&mut inventory, sender, item, /*&mut transfer_in_process*/),
                InventoryMessage::TakeMyItemsResult(result) => 
                    take_my_items_result(&mut inventory, result),
            };            
        }
    }

    /// Gives some count of Item to inventory.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory' - Mutable reference to the inventory to increase
    /// * 'item'      - Tuple of Item and amount to take
    fn add(inventory: &mut Inventory, item: (Item, usize)) {
        // FIXME: INVENTORIES ARE INFINITE FOR FIRST DEMO
        inventory.give(item);

        return; // Success or something
    }

    /// Takes some count of Items from inventory, will not do anything if inventory is empty.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory' - Mutable reference to the inventory to take from
    /// * 'item'      - Tuple of Item and amount to take
    fn remove(inventory: &mut Inventory, item: (Item, usize)) {
        inventory.take(item);
        
        return; // Success or something
    }

    /// Asks the other inventory to perform the give_me_items function with 
    /// this inventorys AID as 'sender' parameter
    /// 
    /// # Arguments
    /// 
    /// * 'inventory'            - Reference to the inventory to move item to 
    /// * 'aid'                  - AID of the inventory to take from
    /// * 'items'                - Tuple of Item and amount to take
    /// * 'transfer_in_progress' - Bool that is true if there is already a transfer in progress, else false
    fn take_from(
        inventory: &Inventory, 
        aid: AID<InventoryMessage>, 
        item: (Item, usize),
        // transfer_in_progress: &mut bool
    ) {
        // if *transfer_in_progress /*|| inventory.is_full() */ {
        //     return; // Should send an error or whatever
        // }

        // *transfer_in_progress = true;
        _ = aid.send(InventoryMessage::GiveMeItems(inventory.aid.clone(), item)); // Should handle Result in some way
    }

    /// Asks the other inventory to perform the take_my_items function with 
    /// this inventorys AID as 'sender' parameter
    /// 
    /// # Arguments
    /// 
    /// * 'inventory'            - Reference to the inventory to move item from 
    /// * 'aid'                  - AID of the inventory to give to
    /// * 'items'                - Tuple of Item and amount to give
    /// * 'transfer_in_progress' - Bool that is true if there is already a transfer in progress, else false
    fn give_to(
        inventory: &Inventory, 
        aid: AID<InventoryMessage>, 
        item: (Item, usize),
        // transfer_in_progress: &mut bool
    ) {
        // if *transfer_in_progress /*|| inventory.is_full() */ {
        //     return; // Should send an error or whatever
        // }

        // *transfer_in_progress = true;
        _ = aid.send(InventoryMessage::TakeMyItems(inventory.aid.clone(), item));
    }

    /// Checks if this inventory can give item and sends a result containing either a tuple containing
    /// what item and quantity, or an error containing a string explaining what went wrong.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory'            - Mutable reference to this inventory
    /// * 'sender'               - AID of the requesting inventory
    /// * 'items'                - Tuple of Item and amount to give
    /// * 'transfer_in_progress' - Bool that is true if there is already a transfer in progress, else false
    fn give_me_items(
        inventory: &mut Inventory, 
        sender: AID<InventoryMessage>, 
        item: (Item, usize),
        // transfer_in_progress: &mut bool
    ) {
        // if *transfer_in_progress {
        //     _ = sender.send(InventoryMessage::GiveMeItemResult(Result::Err("I'm busy!"))); // Sends an error; TODO: Should handle Result in some way
        //     return;
        // }

        if inventory.has_too_few_items(item) {
            _ = sender.send(InventoryMessage::GiveMeItemResult(Result::Err("I'm empty!"))); // Sends an error; TODO: Should handle Result in some way
            return;
        }


        inventory.take(item);
        _ = sender.send(InventoryMessage::GiveMeItemResult(Result::Ok(item))); // Sends a tuple containing what item it is and how many it moved; TODO: Should handle Result in some way
    }

    /// Gets the result from a GiveMeItem message and add the item to this inventory, or prints the
    /// error message if GiveMeItem failed.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory' - Mutable reference to this inventory
    /// * 'result'    - A result containing either a tuple of what item was moved and the quantity, 
    ///                 or the error message as a str
    fn give_me_items_result(
        inventory: &mut Inventory, 
        result: Result<(Item, usize), &'static str>
    ) {
        match result {
            Ok(item) => { inventory.give(item) },
            Err(msg) => { println!("{}", msg) }    // should probably do something else
        };
    }

    /// Checks if this inventory can take the items and sends a result containing either a tuple 
    /// containing what item and quantity it took, or an error containing a string explaining what 
    /// went wrong.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory'            - Mutable reference to this inventory
    /// * 'sender'               - AID of the requesting inventory
    /// * 'items'                - Tuple of Item and amount to get
    /// * 'transfer_in_progress' - Bool that is true if there is already a transfer in progress, else false
    fn take_my_items(
        inventory: &mut Inventory, 
        sender: AID<InventoryMessage>, 
        items: (Item, usize), 
        // transfer_in_progress: &mut bool
    ) {
        // if *transfer_in_progress {
        //     _ = sender.send(InventoryMessage::TakeMyItemsResult(Result::Err("I'm busy!")));
        // }

        if inventory.is_full() { // FIXME: this wont happen as is_full is not implemented
            _ = sender.send(InventoryMessage::TakeMyItemsResult(Result::Err("I'm full!"))); // Sends an error; TODO: Should handle Result in some way
        } 

        inventory.give(items);
        _ = sender.send(InventoryMessage::TakeMyItemsResult(Result::Ok(items)));
    }

    /// Gets the result from a TakeMyItems message and removes the items from this inventory, or prints the
    /// error message if TakeMyItems failed.
    /// 
    /// # Arguments
    /// 
    /// * 'inventory' - Mutable reference to this inventory
    /// * 'result'    - A result containing either a tuple of what item was moved and the quantity, 
    ///                 or the error message as a str
    fn take_my_items_result(
        inventory: &mut Inventory, 
        result: Result<(Item, usize), &'static str>
    ) {
        match result {
            Ok(item) => { _ = inventory.take(item) },
            Err(msg) => { println!("{}", msg) }    // should probably do something else
        };
    }
}