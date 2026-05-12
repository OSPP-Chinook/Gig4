use std::{
    collections::{
        HashMap,
        VecDeque,
    }, 
    sync::mpsc::TryRecvError
};

use crate::{
    aid::AID, 
    item::{Item}, messages::{EntityMessage, PlayerManagerMessage}
};

#[derive(Clone)]
pub enum InventoryMessage {
    // The following are sent by owner (entity) (public)
    Add(AID<EntityMessage>, Vec<(Item, usize)>),
    Remove(AID<EntityMessage>, Vec<(Item, usize)>),
    TakeFrom(AID<EntityMessage>, AID<InventoryMessage>, Vec<(Item, usize)>), 
    GiveTo(AID<EntityMessage>, AID<InventoryMessage>, Vec<(Item, usize)>),
    PrintInventory(String),
    GiveStatus(AID<PlayerManagerMessage>),
    Kill,

    // The following are sent by another inventory (private)
    GiveMeItems(AID<EntityMessage>, AID<InventoryMessage>, Vec<(Item, usize)>),      // From TakeFrom 
    GiveMeItemResult(AID<EntityMessage>, Result<Vec<(Item, usize)>, &'static str>),  // From GiveMeItems
    TakeMyItems(AID<EntityMessage>, AID<InventoryMessage>, Vec<(Item, usize)>),      // From GiveTo
    TakeMyItemsResult(AID<EntityMessage>, Result<(), Vec<(Item, usize)>>), // From TakeMyItems
}

struct Inventory {
    aid: AID<InventoryMessage>,
    max: usize,                           // ???
    waiting: VecDeque<InventoryMessage>,
    items: HashMap<Item, (usize, usize)>, // Key: Name of item -> Value: (count, max).
}

impl Inventory {
    fn construct(aid: AID<InventoryMessage>) -> Self {
        let mut inventory = Inventory { 
            aid, 
            max: 10, 
            waiting: VecDeque::new(), 
            items: HashMap::new() 
        };

        inventory.items.insert(Item::Mutexium, (0, 0));
        
        return inventory;
    }

    fn add(&mut self, (item, count): (Item, usize)) -> bool {
        if self.is_full() {
            return false; 
        }

        self.items.entry(item).or_insert((0, 0)).0 += count;

        return true;
    }

    fn can_remove(&mut self, items: &Vec<(Item, usize)>) -> bool {
        let mut can_take: bool = true; 

        for (item, count) in items {
            can_take = can_take && (self.items
                                        .entry(*item)
                                        .or_insert((0, 0)).0 >= *count);
        };

        return can_take;
    }

    fn remove(&mut self, (item, count): (Item, usize)) -> bool {
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

    fn to_string(&self) -> String {        
        let mut string: String = String::from("Inventory");

        for (key, value) in &self.items {
            if value.0 > 0 {
                string.push_str(format!("\n{0} - {1}/{2}", key.to_str(), value.0, value.1).as_str());
            }
        }

        return string;
    }

    // For debugging as of now
    fn print_inv(&self, name: String) {
        println!("{0}:", name);
        
        for (key, (amount, max)) in &self.items {
            println!("      {0} - {1}/{2}", key.to_str(), amount, max);
        }
    }
}

/// Initializes a new inventory and returns its AID
pub fn init() -> AID<InventoryMessage> {
    return AID::new(inventory_loop);
}

fn inventory_loop(
        aid: AID<InventoryMessage>, 
        mailbox: std::sync::mpsc::Receiver<InventoryMessage>
    ){
    let mut inventory: Inventory = Inventory::construct(aid);

    loop {
        if !inventory.waiting.is_empty() {
            // println!("Inventory has a queue of requests");
            match_message(inventory.waiting.pop_front().unwrap().clone(), &mut inventory);
        }
        
        let message: Result<InventoryMessage, TryRecvError> = mailbox.try_recv();

        match message {
            Ok(m) => match_message(m, &mut inventory),

            Err(e) => {
                if e == TryRecvError::Disconnected  {
                    return; // Could possibly do something different.
                }
            }
        };
    }
}

fn match_message(message: InventoryMessage, inventory: &mut Inventory) {
    match message {
        InventoryMessage::Add(sender, items) => 
            add(sender, inventory, items),

        InventoryMessage::Remove(sender, items) => 
            remove(sender, inventory, items),

        InventoryMessage::TakeFrom(sender, other, items) => 
            take_from(sender, &inventory, other, items),

        InventoryMessage::GiveTo(sender, other, items) => 
            give_to(sender, inventory, other, items),

        InventoryMessage::PrintInventory(name) => inventory.print_inv(name),
        
        InventoryMessage::GiveStatus(pm_aid) => {
            _ = pm_aid.send(PlayerManagerMessage::InventoryStatusResult(inventory.to_string()));
        }

        InventoryMessage::Kill => return, // Should probably take care of all messages in mailbox somehow


        InventoryMessage::GiveMeItems(sender, sending_inventory, items) => 
            give_me_items(sender, inventory, sending_inventory, items),

        InventoryMessage::GiveMeItemResult(sender, result) =>
            give_me_items_result(sender, inventory, result),

        InventoryMessage::TakeMyItems(sender, sending_inventory, items) => 
            take_my_items(sender, inventory, sending_inventory, items),

        InventoryMessage::TakeMyItemsResult(sender, result) => 
            take_my_items_result(sender, inventory, result),
    }
}

/// Gives some count of Item to inventory.
/// 
/// # Arguments
/// 
/// * 'sender'    - AID of entity that sent the Add message
/// * 'inventory' - Mutable reference to the inventory to increase
/// * 'item'      - Tuple of Item and amount to take
fn add(sender: AID<EntityMessage>, inventory: &mut Inventory, items: Vec<(Item, usize)>) {
    for item in items {
        inventory.add(item);
    }

    _ = sender.send(EntityMessage::InventoryOk);
    return;
}

/// Takes some count of Items from inventory, will not do anything if inventory is empty.
/// 
/// # Arguments
/// 
/// * 'sender'    - AID of entity that sent the Remove message
/// * 'inventory' - Mutable reference to the inventory to take from
/// * 'item'      - Tuple of Item and amount to take
fn remove(sender: AID<EntityMessage>, inventory: &mut Inventory, items: Vec<(Item, usize)>) {
    if inventory.can_remove(&items) {
        for item in items {
            inventory.remove(item);
        }

        _ = sender.send(EntityMessage::InventoryOk);
        return;
    }

    _ = sender.send(EntityMessage::InventoryErr);
}

/// Asks the other inventory to perform the give_me_items function with 
/// this inventorys AID as 'sender' parameter
/// 
/// # Arguments
/// 
/// * 'sender'               - AID of entity that sent the TakeFrom message
/// * 'inventory'            - Reference to the inventory to move item to 
/// * 'aid'                  - AID of the inventory to take from
/// * 'items'                - Tuple of Item and amount to take
fn take_from(
    sender: AID<EntityMessage>,
    inventory: &Inventory, 
    aid: AID<InventoryMessage>, 
    items: Vec<(Item, usize)>,
) {
    if inventory.is_full() {
        return; // Should send an error or whatever
    }

    _ = aid.send(InventoryMessage::GiveMeItems(sender, inventory.aid.clone(), items)); // Should handle Result in some way
}

/// Asks the other inventory to perform the take_my_items function with 
/// this inventorys AID as 'sender' parameter
/// 
/// # Arguments
/// 
/// * 'sender'               - AID of entity that sent the GiveTo message
/// * 'inventory'            - Reference to the inventory to move item from 
/// * 'aid'                  - AID of the inventory to give to
/// * 'items'                - Tuple of Item and amount to give
fn give_to(
    sender: AID<EntityMessage>,
    inventory: &mut Inventory, 
    aid: AID<InventoryMessage>, 
    items: Vec<(Item, usize)>,
) {
    if inventory.can_remove(&items) {
        for item in &items {
            inventory.remove(*item);
        }
        
        _ = aid.send(InventoryMessage::TakeMyItems(sender, inventory.aid.clone(), items));
        return;
    }

    inventory.waiting.push_back(InventoryMessage::GiveTo(sender.clone(), aid, items));
}

/// Checks if this inventory can give item and sends a result containing either a tuple containing
/// what item and quantity, or an error containing a string explaining what went wrong.
/// 
/// # Arguments
/// 
/// * 'sender'               - AID of entity that sent the original TakeFrom message
/// * 'inventory'            - Mutable reference to this inventory
/// * 'sender'               - AID of the requesting inventory
/// * 'items'                - Tuple of Item and amount to give
fn give_me_items(
    sender: AID<EntityMessage>,
    inventory: &mut Inventory, 
    sending_inventory: AID<InventoryMessage>, 
    items: Vec<(Item, usize)>,
) {
    if !inventory.can_remove(&items) {
        // println!("had too few items");
        inventory.waiting.push_back(
            InventoryMessage::GiveMeItems(sender.clone(), sending_inventory.clone(), items)
        );
        // TODO: Figure out how to know if an inventory of a factory has changed production rules, 
        //       making the request impossible to fulfill. Should send error in that case.
        return;
    }

    for item in &items {
        inventory.remove(*item);
    }

    _ = sending_inventory.send(InventoryMessage::GiveMeItemResult(sender, Result::Ok(items)));
}

/// Gets the result from a GiveMeItem message and add the item to this inventory, or prints the
/// error message if GiveMeItem failed.
/// 
/// # Arguments
/// 
/// * 'sender'    - AID of entity that sent the original TakeFrom message
/// * 'inventory' - Mutable reference to this inventory
/// * 'result'    - A result containing either a tuple of what item was moved and the quantity, 
///                 or the error message as a str
fn give_me_items_result(
    sender: AID<EntityMessage>,
    inventory: &mut Inventory, 
    result: Result<Vec<(Item, usize)>, &'static str>
) {
    match result {
        Ok(items) => { 
            for item in &items {
                inventory.add(*item);
            }

            _ = sender.send(EntityMessage::InventoryOk); 
        },
        Err(msg) => { 
            println!("{}", msg); // should probably do something else
            _ = sender.send(EntityMessage::InventoryErr); 
        }
    };
}

/// Checks if this inventory can take the items and sends a result containing either a tuple 
/// containing what item and quantity it took, or an error containing a string explaining what 
/// went wrong.
/// 
/// # Arguments
/// 
/// * 'sender'               - AID of entity that sent the original GiveTo message
/// * 'inventory'            - Mutable reference to this inventory
/// * 'sender'               - AID of the requesting inventory
/// * 'items'                - Tuple of Item and amount to get
fn take_my_items(
    sender: AID<EntityMessage>,
    inventory: &mut Inventory, 
    sending_inventory: AID<InventoryMessage>, 
    items: Vec<(Item, usize)>, 
) {
    if inventory.is_full() { // FIXME: this wont happen as is_full is not implemented
        inventory.waiting.push_back(InventoryMessage::TakeMyItems(sender.clone(), sending_inventory.clone(), items));
        // TODO: Should send error if inventory is full.
        return;
    } 

    for item in &items {
        inventory.add(*item);
    }

    _ = sending_inventory.send(InventoryMessage::TakeMyItemsResult(sender.clone(), Result::Ok(())));
}

/// Gets the result from a TakeMyItems message and removes the items from this inventory, or prints the
/// error message if TakeMyItems failed.
/// 
/// # Arguments
/// 
/// * 'sender'    - AID of entity that sent the original GiveTo message
/// * 'inventory' - Mutable reference to this inventory
/// * 'result'    - A result containing either a tuple of what item was moved and the quantity, 
///                 or the error message as a str
fn take_my_items_result(
    sender: AID<EntityMessage>,
    inventory: &mut Inventory, 
    result: Result<(), Vec<(Item, usize)>>
) {
    match result {
        Ok(_) => { 
            _ = sender.send(EntityMessage::InventoryOk);
        },
        Err(items) => { 
            for item in &items {
                _ = inventory.add(*item); // Revert removal
            }

            _ = sender.send(EntityMessage::InventoryErr);
        }    
    };
}