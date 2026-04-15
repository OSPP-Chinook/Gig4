use core::time;
use std::{thread::sleep, time::Duration};

mod inventory;
mod aid;

fn main() {
    println!("Hello, world!");
}

fn test_inventory() {
    let inventory_aid: aid::AID<inventory::InventoryMessage> = inventory::inventory::init();
    let inventory_aid2: aid::AID<inventory::InventoryMessage> = inventory::inventory::init();

    for _ in 0..10 {
        _ = inventory_aid.send(inventory::InventoryMessage::Increase); // Minskar inventoryt med 1, om det inte redan är tomt
        sleep(time::Duration::from_millis(500));
    }

    println!("Moving from inventory 1 to 2");

    for _ in 0..5 {
        _ = inventory_aid2.send(inventory::InventoryMessage::TakeFrom(inventory_aid.clone()));
        sleep(time::Duration::from_millis(500));
    }

    loop{}
}
