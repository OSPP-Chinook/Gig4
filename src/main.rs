mod aid;
mod assets;
mod building;
mod game_manager;
mod inventory;
mod messages;
mod player_manager;
mod task_manager;
mod worker;
mod world_manager;
mod zombie;

use crate::{aid::AID, messages::EntityMessage};

fn main() {
    println!("Hello, world!");
    let _gm = AID::new_threadless(game_manager::main);
}

#[cfg(test)]
mod tests {
    use crate::assets::{Assets, ItemId, ItemStack};

    use super::*;
    use inventory::InventoryMessage;
    use std::{path::Path, sync::Arc, thread::sleep, time::Duration};

    fn do_nothing(
        _aid: aid::AID<EntityMessage>,
        _mailbox: std::sync::mpsc::Receiver<EntityMessage>,
    ) {
        loop {}
    }

    #[test]

    fn test_inventory() {
        let assets = Arc::new(Assets::load(Path::new("assets")).unwrap());

        let sender: aid::AID<EntityMessage> = aid::AID::new(do_nothing);

        let worker_aid: aid::AID<InventoryMessage> = inventory::init(assets.clone(), 10);
        let factory_aid1: aid::AID<InventoryMessage> = inventory::init(assets.clone(), 10);
        let factory_aid2: aid::AID<InventoryMessage> = inventory::init(assets.clone(), 10);

        println!("Give Factory 1 8 mutexium and 8 semaphorite");
        _ = factory_aid1.send(InventoryMessage::Add(
            sender.clone(),
            vec![
                ItemStack::new(ItemId::from("mutexium"), 8),
                ItemStack::new(ItemId::from("semaphorite"), 8),
            ],
        ));

        println!("Converting mutexium and semaphorite to Actorisite");
        for _ in 1..9 {
            _ = factory_aid1.send(InventoryMessage::Remove(
                sender.clone(),
                vec![
                    ItemStack::new(ItemId::from("mutexium"), 1),
                    ItemStack::new(ItemId::from("semaphorite"), 1),
                ],
            ));

            _ = factory_aid1.send(InventoryMessage::Add(
                sender.clone(),
                vec![ItemStack::new(ItemId::from("actorisite"), 1)],
            ));
        }

        println!("Taking 8 actorisite from factory 1 to worker, should be in waiting queue");
        _ = worker_aid.send(InventoryMessage::TakeFrom(
            sender.clone(),
            factory_aid1.clone(),
            vec![ItemStack::new(ItemId::from("actorisite"), 8)],
        ));

        println!("Giving 8 actorisite from worker to factory 2");
        _ = worker_aid.send(InventoryMessage::GiveTo(
            sender.clone(),
            factory_aid2.clone(),
            vec![ItemStack::new(ItemId::from("actorisite"), 8)],
        ));

        sleep(Duration::from_millis(500));

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
        _ = worker_aid.send(InventoryMessage::_PrintInventory(String::from("Worker")));
        sleep(Duration::from_millis(500));

        _ = factory_aid1.send(InventoryMessage::_PrintInventory(String::from("Factory 1")));
        sleep(Duration::from_millis(500));

        _ = factory_aid2.send(InventoryMessage::_PrintInventory(String::from("Factory 2")));
        sleep(Duration::from_millis(500));
    }
}
