mod inventory;
mod aid;
mod entity;
mod building;
mod messages;
mod assets;
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
    let gm = GameManager::new();
    if let Ok(gm) = gm {
        gm.run();
    }
    //test_inventory();
}

fn do_nothing(_aid: aid::AID<EntityMessage>, _mailbox: std::sync::mpsc::Receiver<EntityMessage>) {
    loop {};
}

fn test_inventory() {
    let sender: aid::AID<EntityMessage> = aid::AID::new(do_nothing);

    let worker_aid: aid::AID<InventoryMessage> = inventory::init();
    let factory_aid1: aid::AID<InventoryMessage> = inventory::init();
    let factory_aid2: aid::AID<InventoryMessage> = inventory::init();

    println!("Creating mutexium in factory 1");
    _ = factory_aid1.send( // Factory 1 produces 8 mutexium
        InventoryMessage::Add(sender.clone(), (Item::Semaphorite, 8))
    ); 
    
    println!("Taking 8 mutexium from factory 1 to worker");
    _ = worker_aid.send( // Worker takes 8 Mutexium from factory 1
        InventoryMessage::TakeFrom(sender.clone(), factory_aid1.clone(), (Item::Semaphorite, 8))
    ); 

    println!("Giving 8 mutexium from worker to factory 2");
    _ = worker_aid.send( // Worker gives 8 Mutexium to factory 2
        InventoryMessage::GiveTo(sender.clone(), factory_aid2.clone(), (Item::Semaphorite, 8))
    ); 

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