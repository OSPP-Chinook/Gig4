use crate::{
    inventory::{GiveMeItemsError, InventoryMessage, TakeMyItemsError},
    messages::{
        EntityMessage, GetInventoryError, ItemTransferError, MoveError, PlayerManagerMessage,
        TaskError,
    },
    task_manager::TaskManagerMessage,
    world_manager::WorldManagerMessage,
};

pub fn entity_zombie(mailbox: impl IntoIterator<Item = EntityMessage>) {
    for msg in mailbox {
        match msg {
            // I did not combine these into a default: do nothing since
            // we want it to error if something else is added that
            // requires a response. If we forget to add something here, it
            // can cause deadlocks.
            EntityMessage::KillYourself => {}
            EntityMessage::GetInventoryResponse(_) => {}
            EntityMessage::ItemTransferResponse(_) => {}
            EntityMessage::MoveResponse(_) => {}
            EntityMessage::TaskResponse(_) => {}

            EntityMessage::GetInventory(aid) => {
                let _ = aid.send(EntityMessage::GetInventoryResponse(Err(
                    GetInventoryError::ImDead,
                )));
            }

            EntityMessage::FetchInventoryStatus(aid) => {
                let _ = aid.send(PlayerManagerMessage::InventoryStatusResult(None));
            }

            EntityMessage::FetchCurrentTask(aid) => {
                let _ = aid.send(PlayerManagerMessage::CurrentTaskResult(None));
            }
        }
    }
}

pub fn inventory_zombie(mailbox: impl IntoIterator<Item = InventoryMessage>) {
    for msg in mailbox {
        match msg {
            InventoryMessage::ChangeRecipe => {}
            InventoryMessage::PrintInventory(_) => {}
            InventoryMessage::KillYourself => {}

            InventoryMessage::Add(entity, _)
            | InventoryMessage::Remove(entity, _)
            | InventoryMessage::TakeFrom(entity, _, _)
            | InventoryMessage::GiveTo(entity, _, _)
            | InventoryMessage::GiveMeItemsResult(entity, _)
            | InventoryMessage::TakeMyItemsResult(entity, _) => {
                let _ = entity.send(EntityMessage::ItemTransferResponse(Err(
                    ItemTransferError::ImDead,
                )));
            }

            InventoryMessage::GiveMeItems(entity, inventory, _) => {
                let _ = inventory.send(InventoryMessage::GiveMeItemsResult(
                    entity,
                    Err(GiveMeItemsError::ImDead),
                ));
            }
            InventoryMessage::TakeMyItems(entity, inventory, items) => {
                let _ = inventory.send(InventoryMessage::TakeMyItemsResult(
                    entity,
                    Err((items, TakeMyItemsError::ImDead)),
                ));
            }

            InventoryMessage::GiveStatus(aid) => {
                let _ = aid.send(PlayerManagerMessage::InventoryStatusResult(None));
            }
        }
    }
}

pub fn world_manager_zombie(mailbox: impl IntoIterator<Item = WorldManagerMessage>) {
    for msg in mailbox {
        match msg {
            WorldManagerMessage::Quit => {}
            WorldManagerMessage::SpawnObstacle(_) => {}
            WorldManagerMessage::SpawnWorker(_, _) => {}
            WorldManagerMessage::SpawnBuilding(_, _, _) => {}
            WorldManagerMessage::KillEntity(_) => {}

            WorldManagerMessage::Move(_, aid) => {
                let _ = aid.send(EntityMessage::MoveResponse(Err(MoveError::ImDead)));
            }
        }
    }
}

pub fn task_manager_zombie(mailbox: impl IntoIterator<Item = TaskManagerMessage>) {
    for msg in mailbox {
        match msg {
            TaskManagerMessage::Quit => {}
            TaskManagerMessage::KillMe(_) => {}
            TaskManagerMessage::RemoveMyTask(_) => {}
            TaskManagerMessage::GiveTaskTo(_, _) => {}
            TaskManagerMessage::CreatePath(_, _, _) => {}
            TaskManagerMessage::CreateMoveTask(_) => {}

            TaskManagerMessage::GiveMeNewTask(aid) => {
                let _ = aid.send(EntityMessage::TaskResponse(Err(TaskError::ImDead)));
            }
        }
    }
}

pub fn player_manager_zombie(mailbox: impl IntoIterator<Item = PlayerManagerMessage>) {
    for msg in mailbox {
        match msg {
            PlayerManagerMessage::Quit => {}
            PlayerManagerMessage::ShowTileInfo(_, _) => {}
            PlayerManagerMessage::TileNotFound(_) => {}
            PlayerManagerMessage::Notification(_) => {}
            PlayerManagerMessage::InventoryStatusResult(_) => {}
            PlayerManagerMessage::CurrentTaskResult(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, sync::Arc, thread, time::Duration};

    use crate::{
        aid::AID,
        assets::{Assets, BuildingId, ItemId, ItemStack, WorkerId},
        building::Building,
        inventory, player_manager, task_manager,
        worker::Worker,
        world_manager,
    };

    use super::*;

    #[test]
    fn worker_dies() {
        let assets = Arc::new(Assets::load(Path::new("assets")).unwrap());

        let world = AID::mock().0;
        let task = AID::mock().0;
        let (mock, mailbox) = AID::mock();

        let (worker_aid, worker_handle) =
            Worker::new_joinable(world, task, (0, 0), assets, WorkerId::from("worker"));

        let _ = worker_aid.send(EntityMessage::KillYourself);
        thread::sleep(Duration::from_millis(250));
        assert!(worker_aid.send(EntityMessage::GetInventory(mock)).is_ok());
        assert!(matches!(
            mailbox.recv(),
            Ok(EntityMessage::GetInventoryResponse(Err(
                GetInventoryError::ImDead
            )))
        ));

        drop(worker_aid);
        let _ = worker_handle.join();
    }

    #[test]
    fn building_dies() {
        let assets = Arc::new(Assets::load(Path::new("assets")).unwrap());

        let world = AID::mock().0;
        let task = AID::mock().0;
        let (mock, mailbox) = AID::mock();

        let (building_aid, building_handle) =
            Building::new_joinable(world, task, assets, BuildingId::from("factory"));

        let _ = building_aid.send(EntityMessage::KillYourself);
        thread::sleep(Duration::from_millis(250));
        assert!(building_aid.send(EntityMessage::GetInventory(mock)).is_ok());
        assert!(matches!(
            mailbox.recv(),
            Ok(EntityMessage::GetInventoryResponse(Err(
                GetInventoryError::ImDead
            )))
        ));

        drop(building_aid);
        let _ = building_handle.join();
    }

    #[test]
    fn inventory_dies() {
        let assets = Arc::new(Assets::load(Path::new("assets")).unwrap());

        let mock_sender = AID::mock().0;
        let (mock_inv, mailbox) = AID::mock();

        let (inventory_aid, inventory_handle) = inventory::init_joinable(assets, 10);

        let _ = inventory_aid.send(InventoryMessage::KillYourself);
        thread::sleep(Duration::from_millis(250));
        assert!(
            inventory_aid
                .send(InventoryMessage::TakeMyItems(
                    mock_sender.clone(),
                    mock_inv,
                    vec![ItemStack::new(ItemId::from("mutexium"), 10)]
                ))
                .is_ok()
        );

        assert!(
            matches!(mailbox.recv(), Ok(InventoryMessage::TakeMyItemsResult(
            sender,
            Err((items, TakeMyItemsError::ImDead)))) if sender == mock_sender && items == vec![ItemStack::new(ItemId::from("mutexium"), 10)])
        );

        drop(inventory_aid);
        let _ = inventory_handle.join();
    }

    #[test]
    fn world_dies() {
        let assets = Arc::new(Assets::load(Path::new("assets")).unwrap());

        let (mock, mailbox) = AID::mock();
        let task = AID::mock().0;
        let grid = world_manager::init_world_grid();
        let (world_aid, world_handle) = world_manager::new_joinable(grid, task, assets);

        let _ = world_aid.send(WorldManagerMessage::Quit);
        thread::sleep(Duration::from_millis(250));
        assert!(
            world_aid
                .send(WorldManagerMessage::Move((0, 0), mock))
                .is_ok()
        );

        assert!(matches!(
            mailbox.recv(),
            Ok(EntityMessage::MoveResponse(Err(MoveError::ImDead)))
        ));

        drop(world_aid);
        let _ = world_handle.join();
    }

    #[test]
    fn task_dies() {
        let (mock, mailbox) = AID::mock();
        let grid = world_manager::init_world_grid();
        let (task_aid, task_handle) = task_manager::new_joinable(grid);

        let _ = task_aid.send(TaskManagerMessage::Quit);
        thread::sleep(Duration::from_millis(250));
        assert!(
            task_aid
                .send(TaskManagerMessage::GiveMeNewTask(mock))
                .is_ok()
        );

        assert!(matches!(
            mailbox.recv(),
            Ok(EntityMessage::TaskResponse(Err(TaskError::ImDead)))
        ));

        drop(task_aid);
        let _ = task_handle.join();
    }

    #[test]
    fn player_dies() {
        let world = AID::mock().0;
        let game = AID::mock().0;
        let grid = world_manager::init_world_grid();
        let (player_aid, player_handle) = player_manager::new_joinable(grid, world, game);

        let _ = player_aid.send(PlayerManagerMessage::Quit);
        drop(player_aid);
        let _ = player_handle.join();
    }
}
