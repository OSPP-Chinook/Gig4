mod inventory;
mod aid;

use inventory::{
    Item,
    InventoryMessage,
};

fn main() {
    println!("Hello, world!");
    test_inventory();
}

fn test_inventory() {
    let inventory_aid: aid::AID<InventoryMessage> = inventory::inventory::init();
    let inventory_aid2: aid::AID<InventoryMessage> = inventory::inventory::init();

    _ = inventory_aid.send(InventoryMessage::Give((Item::Mutexium, 9))); // Adds 9 Mutexium to inventory 1

    println!("Moving from inventory 1 to 2");

    _ = inventory_aid2.send(InventoryMessage::TakeFrom(inventory_aid.clone(), (Item::Mutexium, 8))); // Take 8 Mutexium from inventory 1

    loop {};
}
