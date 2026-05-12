mod aid;
mod building;
mod game_manager;
mod inventory;
mod item;
mod messages;
mod player_manager;
mod task_manager;
mod worker;
mod world_manager;

use crate::{aid::AID, messages::EntityMessage};

fn main() {
    println!("Hello, world!");
    let _gm = AID::new_threadless(game_manager::main);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Item;
    use core::time;
    use inventory::InventoryMessage;
    use std::thread::sleep;

    fn do_nothing(
        _aid: aid::AID<EntityMessage>,
        _mailbox: std::sync::mpsc::Receiver<EntityMessage>,
    ) {
        loop {}
    }

    #[test]
    fn test_inventory() {
        let sender: aid::AID<EntityMessage> = aid::AID::new(do_nothing);

        let worker_aid: aid::AID<InventoryMessage> = inventory::init();
        let factory_aid1: aid::AID<InventoryMessage> = inventory::init();
        let factory_aid2: aid::AID<InventoryMessage> = inventory::init();

        println!("Give Factory 1 8 mutexium and 8 semaphorite");
        _ = factory_aid1.send(InventoryMessage::Add(
            sender.clone(),
            vec![(Item::Mutexium, 8), (Item::Semaphorite, 8)],
        ));

        println!("Converting mutexium and semaphorite to Actorisite");
        for _ in 1..9 {
            _ = factory_aid1.send(InventoryMessage::Remove(
                sender.clone(),
                vec![(Item::Mutexium, 1), (Item::Semaphorite, 1)],
            ));

            _ = factory_aid1.send(InventoryMessage::Add(
                sender.clone(),
                vec![(Item::Actorisite, 1)],
            ));
        }

        println!("Taking 8 actorisite from factory 1 to worker, should be in waiting queue");
        _ = worker_aid.send(InventoryMessage::TakeFrom(
            sender.clone(),
            factory_aid1.clone(),
            vec![(Item::Actorisite, 8)],
        ));

        println!("Giving 8 actorisite from worker to factory 2");
        _ = worker_aid.send(InventoryMessage::GiveTo(
            sender.clone(),
            factory_aid2.clone(),
            vec![(Item::Actorisite, 8)],
        ));

        sleep(time::Duration::from_millis(500));

        print_system_status(
            worker_aid.clone(),
            factory_aid1.clone(),
            factory_aid2.clone(),
        );
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
}
