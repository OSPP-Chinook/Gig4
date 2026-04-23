mod inventory;
mod aid;
mod building;
mod messages;
mod item;
mod world_manager;
mod player_manager;

use core::time;
use std::thread::sleep;

use inventory::{
    InventoryMessage,
};

use item::Item;

fn main() {
    println!("Hello, world!");
    test_inventory();
    let _ = player_manager::render_loop();
}

fn test_inventory() {
    let worker_aid: aid::AID<InventoryMessage> = inventory::inventory::init();
    let factory_aid1: aid::AID<InventoryMessage> = inventory::inventory::init();
    let factory_aid2: aid::AID<InventoryMessage> = inventory::inventory::init();

    println!("Creating mutexium in factory 1");
    _ = factory_aid1.send(InventoryMessage::Add((Item::Semaphorite, 8))); // Factory 1 produces 8 mutexium
    
    println!("Taking 8 mutexium from factory 1 to worker");
    _ = worker_aid.send(InventoryMessage::TakeFrom(factory_aid1.clone(), (Item::Semaphorite, 8))); // Worker takes 8 Mutexium from factory 1

    println!("Giving 8 mutexium from worker to factory 2");
    _ = worker_aid.send(InventoryMessage::GiveTo(factory_aid2.clone(), (Item::Semaphorite, 8))); // Worker gives 8 Mutexium to factory 2

    print_system_status(worker_aid.clone(), factory_aid1.clone(), factory_aid2.clone());
}

fn print_system_status(
    worker_aid: aid::AID<InventoryMessage>, 
    factory_aid1: aid::AID<InventoryMessage>,
    factory_aid2: aid::AID<InventoryMessage>,
) {
    _ = worker_aid.send(InventoryMessage::PrintInventory(String::from("Worker")));
    sleep(time::Duration::from_millis(500));

    _ = factory_aid1.send(InventoryMessage::PrintInventory(String::from("Factory 1")));
    sleep(time::Duration::from_millis(500));

    _ = factory_aid2.send(InventoryMessage::PrintInventory(String::from("Factory 2")));
    sleep(time::Duration::from_millis(500));
}