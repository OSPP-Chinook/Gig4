mod inventory;
mod aid;
mod entity;
mod building;
mod messages;
mod item;
mod game_manager;
mod world_manager;
mod player_manager;


use core::time;
use std::thread::sleep;

use inventory::{
    InventoryMessage,
};

use item::Item;

use crate::{game_manager::GameManager, messages::EntityMessage};

fn main() {
    println!("Hello, world!");
    // let gm = GameManager::new();
    // gm.run();
    test_inventory();
}

fn do_nothing(_aid: aid::AID<EntityMessage>, _mailbox: std::sync::mpsc::Receiver<EntityMessage>) {
    loop {};
}

fn test_inventory() {
    let sender: aid::AID<EntityMessage> = aid::AID::new(do_nothing);

    let worker_aid: aid::AID<InventoryMessage> = inventory::init();
    let factory_aid1: aid::AID<InventoryMessage> = inventory::init();
    let factory_aid2: aid::AID<InventoryMessage> = inventory::init();

    println!("Produce mutexium in factory 1");
    _ = factory_aid1.send( // Factory 1 produces 8 mutexium
        InventoryMessage::Add(sender.clone(), (Item::Mutexium, 8))
    ); 
    
    println!("Taking 9 mutexium from factory 1 to worker, should be in waiting queue");
    _ = worker_aid.send( // Worker takes 9 Mutexium from factory 1
        InventoryMessage::TakeFrom(sender.clone(), factory_aid1.clone(), (Item::Mutexium, 9))
    ); 

    println!("Produce 1 more mutexium in factory 1");
    _ = factory_aid1.send( // Factory 1 produces 8 mutexium
        InventoryMessage::Add(sender.clone(), (Item::Mutexium, 1))
    ); 

    println!("Giving 8 mutexium from worker to factory 2");
    _ = worker_aid.send( // Worker gives 8 Mutexium to factory 2
        InventoryMessage::GiveTo(sender.clone(), factory_aid2.clone(), (Item::Mutexium, 8))
    ); 

    sleep(time::Duration::from_millis(500));
    
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