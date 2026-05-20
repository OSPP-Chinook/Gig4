use crate::{
    aid::{AID, AIDHandle},
    assets::{Assets, ItemId, ItemList, ItemStack},
    messages::{EntityMessage, ItemTransferError, PlayerManagerMessage},
    zombie,
};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

#[derive(Clone)]
pub enum GiveMeItemsError {
    ImDead,
    RecipeChange,
}

#[derive(Clone)]
pub enum TakeMyItemsError {
    ImDead,
}

#[derive(Clone)]
pub enum InventoryMessage {
    // The following are sent by owner (entity) (public)
    Add(AID<EntityMessage>, ItemList),
    Remove(AID<EntityMessage>, ItemList),
    TakeFrom(AID<EntityMessage>, AID<InventoryMessage>, ItemList),
    GiveTo(AID<EntityMessage>, AID<InventoryMessage>, ItemList),
    ChangeRecipe, // tell all waiting that their request can't be fulfilled
    _PrintInventory(String), // Debugging
    GiveStatus(AID<PlayerManagerMessage>),
    KillYourself,

    // The following are sent by another inventory (private)
    GiveMeItems(AID<EntityMessage>, AID<InventoryMessage>, ItemList), // From TakeFrom
    GiveMeItemsResult(AID<EntityMessage>, Result<ItemList, GiveMeItemsError>), // From GiveMeItems
    TakeMyItems(AID<EntityMessage>, AID<InventoryMessage>, ItemList), // From GiveTo
    TakeMyItemsResult(AID<EntityMessage>, Result<(), (ItemList, TakeMyItemsError)>), // From TakeMyItems
}

struct Inventory {
    aid: AID<InventoryMessage>,
    alive: bool,
    size: usize, // Number of slots the inventory has
    waiting: VecDeque<InventoryMessage>,
    items: HashMap<ItemId, usize>, // Key: item ID -> Value: count (each item takes exactly 1 slot)
    assets: Arc<Assets>,
}

impl Inventory {
    fn construct(aid: AID<InventoryMessage>, assets: Arc<Assets>, size: usize) -> Self {
        Inventory {
            aid,
            alive: true,
            size,
            waiting: VecDeque::new(),
            items: HashMap::new(),
            assets,
        }
    }

    fn destroy(self) {
        drop(self);
    }

    fn add(&mut self, stack: ItemStack) {
        *self.items.entry(stack.id).or_insert(0) += stack.count;
    }

    // Assumes no duplicates in the items vector
    fn can_add(&self, items: &[ItemStack]) -> bool {
        let mut slots = 0;

        for stack in items {
            let current = self.items.get(&stack.id).copied().unwrap_or(0);
            let limit = self.assets.items[&stack.id].stack_limit;

            if current + stack.count > limit {
                return false;
            }

            if current == 0 {
                slots += 1;
            }
        }
        self.items.len() + slots <= self.size
    }

    fn remove(&mut self, stack: ItemStack) {
        if let Some(value) = self.items.get_mut(&stack.id) {
            *value -= stack.count;
            if *value == 0 {
                self.items.remove(&stack.id);
            }
        }
    }

    // Assumes no duplicates in the items vector
    fn can_remove(&self, items: &[ItemStack]) -> bool {
        for stack in items {
            let avail = self.items.get(&stack.id).copied().unwrap_or(0);
            if avail < stack.count {
                return false;
            }
        }
        true
    }

    fn to_string(&self) -> String {
        let mut string: String = String::from("Inventory");

        for (key, value) in &self.items {
            if *value > 0 {
                string.push_str(format!("\n{0} - {1}", &key, value).as_str());
            }
        }

        string.push_str(format!("\n{0} / {1} Slots used", self.items.len(), self.size).as_str());

        return string;
    }

    fn change_recipe(&mut self) {
        for msg in self.waiting.iter() {
            match msg {
                InventoryMessage::Remove(entity, _) => {
                    let _ = entity.send(EntityMessage::ItemTransferResponse(Err(
                        ItemTransferError::RecipeChange,
                    )));
                }

                InventoryMessage::GiveTo(entity, _, _) => {
                    let _ = entity.send(EntityMessage::ItemTransferResponse(Err(
                        ItemTransferError::RecipeChange,
                    )));
                }

                InventoryMessage::GiveMeItems(entity, inventory, _) => {
                    let _ = inventory.send(InventoryMessage::GiveMeItemsResult(
                        entity.clone(),
                        Err(GiveMeItemsError::RecipeChange),
                    ));
                }

                _ => {}
            }
        }

        self.waiting.clear();
    }

    // For debugging as of now
    fn print_inv(&self, name: String) {
        println!("{0}:", name);

        for (key, amount) in &self.items {
            let item = self.assets.items.get(key).unwrap();
            println!("      {0} - {1}/{2}", item.name, amount, item.stack_limit);
        }
    }
}

/// Initializes a new inventory and returns its AID
pub fn init(assets: Arc<Assets>, size: usize) -> AID<InventoryMessage> {
    return init_joinable(assets, size).0;
}

pub fn init_joinable(assets: Arc<Assets>, size: usize) -> (AID<InventoryMessage>, AIDHandle) {
    return AID::new_joinable(move |aid, mailbox| inventory_loop(aid, mailbox, assets, size));
}

fn inventory_loop(
    aid: AID<InventoryMessage>,
    mailbox: std::sync::mpsc::Receiver<InventoryMessage>,
    assets: Arc<Assets>,
    size: usize,
) {
    let mut inventory: Inventory = Inventory::construct(aid, assets, size);

    while inventory.alive {
        if !inventory.waiting.is_empty() {
            // println!("Inventory has a queue of requests");
            match_message(
                inventory.waiting.pop_front().unwrap().clone(),
                &mut inventory,
            );
        }

        let message = mailbox.recv();

        match message {
            Ok(m) => match_message(m, &mut inventory),
            Err(_) => inventory.alive = false,
        };
    }

    zombie::inventory_zombie(inventory.waiting);
    inventory.waiting = VecDeque::new();
    inventory.destroy();
    zombie::inventory_zombie(mailbox);
}

fn match_message(message: InventoryMessage, inventory: &mut Inventory) {
    match message {
        InventoryMessage::Add(sender, items) => add(sender, inventory, items),

        InventoryMessage::Remove(sender, items) => remove(sender, inventory, items),

        InventoryMessage::TakeFrom(sender, other, items) => {
            take_from(sender, &inventory, other, items)
        }

        InventoryMessage::GiveTo(sender, other, items) => give_to(sender, inventory, other, items),

        InventoryMessage::ChangeRecipe => inventory.change_recipe(),

        InventoryMessage::_PrintInventory(name) => inventory.print_inv(name),

        InventoryMessage::KillYourself => inventory.alive = false,

        InventoryMessage::GiveStatus(pm_aid) => {
            _ = pm_aid.send(PlayerManagerMessage::InventoryStatusResult(Some(
                inventory.to_string(),
            )));
        }

        InventoryMessage::GiveMeItems(sender, sending_inventory, items) => {
            give_me_items(sender, inventory, sending_inventory, items)
        }

        InventoryMessage::GiveMeItemsResult(sender, result) => {
            give_me_items_result(sender, inventory, result)
        }

        InventoryMessage::TakeMyItems(sender, sending_inventory, items) => {
            take_my_items(sender, inventory, sending_inventory, items)
        }

        InventoryMessage::TakeMyItemsResult(sender, result) => {
            take_my_items_result(sender, inventory, result)
        }
    }
}

/// Gives some count of Item to inventory.
///
/// # Arguments
///
/// * 'sender'    - AID of entity that sent the Add message
/// * 'inventory' - Mutable reference to the inventory to increase
/// * 'item'      - Tuple of Item and amount to take
fn add(sender: AID<EntityMessage>, inventory: &mut Inventory, items: ItemList) {
    if !inventory.can_add(&items) {
        let _ = sender.send(EntityMessage::ItemTransferResponse(Err(
            ItemTransferError::TooManyItems,
        )));
        return;
    }

    for item in items {
        inventory.add(item);
    }

    _ = sender.send(EntityMessage::ItemTransferResponse(Ok(())));
}

/// Takes some count of Items from inventory, will not do anything if inventory is empty.
///
/// # Arguments
///
/// * 'sender'    - AID of entity that sent the Remove message
/// * 'inventory' - Mutable reference to the inventory to take from
/// * 'item'      - Tuple of Item and amount to take
fn remove(sender: AID<EntityMessage>, inventory: &mut Inventory, items: ItemList) {
    if !inventory.can_remove(&items) {
        let _ = sender.send(EntityMessage::ItemTransferResponse(Err(
            ItemTransferError::InsufficientItems,
        )));
        return;
    }

    for item in items {
        inventory.remove(item);
    }
    let _ = sender.send(EntityMessage::ItemTransferResponse(Ok(())));
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
    items: ItemList,
) {
    if !inventory.can_add(&items) {
        return; // Should send an error or whatever
    }

    let _ = aid.send(InventoryMessage::GiveMeItems(
        sender,
        inventory.aid.clone(),
        items,
    )); // Should handle Result in some way
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
    items: ItemList,
) {
    if !inventory.can_remove(&items) {
        inventory
            .waiting
            .push_back(InventoryMessage::GiveTo(sender.clone(), aid, items));
        return;
    }

    for item in &items {
        inventory.remove(item.clone());
    }
    let _ = aid.send(InventoryMessage::TakeMyItems(
        sender,
        inventory.aid.clone(),
        items,
    ));
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
    items: ItemList,
) {
    if !inventory.can_remove(&items) {
        inventory.waiting.push_back(InventoryMessage::GiveMeItems(
            sender.clone(),
            sending_inventory.clone(),
            items,
        ));
        return;
    }

    for item in &items {
        inventory.remove(item.clone());
    }

    let _ = sending_inventory.send(InventoryMessage::GiveMeItemsResult(sender, Ok(items)));
}

/// Gets the result from a GiveMeItem message and add the item to this inventory, or sends an
/// error to the entity
///
/// # Arguments
///
/// * 'sender'    - AID of entity that sent the original TakeFrom message
/// * 'inventory' - Mutable reference to this inventory
/// * 'result'    - A result containing either a tuple of what item was moved and the quantity,
///                 or the error
fn give_me_items_result(
    sender: AID<EntityMessage>,
    inventory: &mut Inventory,
    result: Result<ItemList, GiveMeItemsError>,
) {
    match result {
        Ok(items) => {
            for item in &items {
                inventory.add(item.clone());
            }

            let _ = sender.send(EntityMessage::ItemTransferResponse(Ok(())));
        }
        Err(GiveMeItemsError::RecipeChange) => {
            let _ = sender.send(EntityMessage::ItemTransferResponse(Err(
                ItemTransferError::RecipeChange,
            )));
        }
        Err(GiveMeItemsError::ImDead) => {
            let _ = sender.send(EntityMessage::ItemTransferResponse(Err(
                ItemTransferError::TheyreDead,
            )));
        }
    }
}

/// Checks if this inventory can take the items and sends a result containing either a tuple
/// containing what item and quantity it took, or an error explaining what went wrong.
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
    items: ItemList,
) {
    if !inventory.can_add(&items) {
        inventory.waiting.push_back(InventoryMessage::TakeMyItems(
            sender.clone(),
            sending_inventory.clone(),
            items,
        ));
        return;
    }

    for item in &items {
        inventory.add(item.clone());
    }

    let _ = sending_inventory.send(InventoryMessage::TakeMyItemsResult(
        sender.clone(),
        Result::Ok(()),
    ));
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
    result: Result<(), (ItemList, TakeMyItemsError)>,
) {
    match result {
        Ok(_) => {
            let _ = sender.send(EntityMessage::ItemTransferResponse(Ok(())));
        }
        Err((items, TakeMyItemsError::ImDead)) => {
            for item in &items {
                inventory.add(item.clone()); // Revert removal
            }
            let _ = sender.send(EntityMessage::ItemTransferResponse(Err(
                ItemTransferError::TheyreDead,
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn mock(size: usize) -> Inventory {
        let assets = Arc::new(Assets::load(Path::new("assets")).unwrap());

        let (aid, _) = AID::mock();
        Inventory::construct(aid, assets, size)
    }

    #[test]
    fn test_add() {
        let mut inv = mock(1);
        let m = ItemId::from("mutexium");

        assert!(inv.can_add(&vec![ItemStack::new(m.clone(), 100)])); // Within limits
        assert!(!inv.can_add(&vec![ItemStack::new(m.clone(), 257)])); // Exceeds stack limit

        inv.add(ItemStack::new(m.clone(), 200));
        assert_eq!(inv.items.len(), 1);

        assert!(inv.can_add(&vec![ItemStack::new(m.clone(), 56)])); // 200 + 56 <= 256
        assert!(!inv.can_add(&vec![ItemStack::new(m.clone(), 57)])); // 200 + 57 > 256 (exceeds limit existing)

        // Exceeds inventory slot size limit
        assert!(!inv.can_add(&vec![ItemStack::new(ItemId::from("semaphorite"), 1)]));
    }

    #[test]
    fn test_remove() {
        let mut inv = mock(2);
        let s = ItemId::from("semaphorite");
        inv.add(ItemStack::new(s.clone(), 50));

        assert!(inv.can_remove(&vec![ItemStack::new(s.clone(), 30)]));
        inv.remove(ItemStack::new(s.clone(), 30));
        assert_eq!(*inv.items.get(&s).unwrap(), 20);

        assert!(inv.can_remove(&vec![ItemStack::new(s.clone(), 20)]));
        inv.remove(ItemStack::new(s.clone(), 20));
        assert!(inv.items.is_empty());
    }
}
