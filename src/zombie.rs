use crate::{
    inventory::{GiveMeItemsError, InventoryMessage, TakeMyItemsError},
    messages::{EntityMessage, GetInventoryError, ItemTransferError},
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
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use crate::{
        aid::AID,
        building::Building,
        inventory::{self, InventoryMessage, TakeMyItemsError},
        item::Item,
        messages::{EntityMessage, GetInventoryError},
        worker::Worker,
    };

    #[test]
    fn worker_dies() {
        let world = AID::mock().0;
        let task = AID::mock().0;
        let (mock, mailbox) = AID::mock();

        let (worker_aid, worker_handle) = Worker::new_joinable(world, task, (0, 0));

        let _ = worker_aid.send(EntityMessage::KillYourself).is_ok();
        thread::sleep(Duration::from_millis(250));
        assert!(worker_aid.send(EntityMessage::GetInventory(mock)).is_ok());
        assert!(matches!(
            mailbox.recv(),
            Ok(EntityMessage::GetInventoryResponse(Err(
                GetInventoryError::ImDead
            )))
        ));

        drop(worker_aid);
        thread::sleep(Duration::from_millis(250));
        let _ = worker_handle.join();
    }

    #[test]
    fn building_dies() {
        let world = AID::mock().0;
        let (mock, mailbox) = AID::mock();

        let (building_aid, building_handle) = Building::new_joinable(world);

        let _ = building_aid.send(EntityMessage::KillYourself).is_ok();
        thread::sleep(Duration::from_millis(250));
        assert!(building_aid.send(EntityMessage::GetInventory(mock)).is_ok());
        assert!(matches!(
            mailbox.recv(),
            Ok(EntityMessage::GetInventoryResponse(Err(
                GetInventoryError::ImDead
            )))
        ));

        drop(building_aid);
        thread::sleep(Duration::from_millis(250));
        let _ = building_handle.join();
    }

    #[test]
    fn inventory_dies() {
        let mock_sender = AID::mock().0;
        let (mock_inv, mailbox) = AID::mock();

        let (inventory_aid, inventory_handle) = inventory::init_joinable();

        let _ = inventory_aid.send(InventoryMessage::KillYourself).is_ok();
        thread::sleep(Duration::from_millis(250));
        assert!(
            inventory_aid
                .send(InventoryMessage::TakeMyItems(
                    mock_sender.clone(),
                    mock_inv,
                    vec![(Item::Mutexium, 10)]
                ))
                .is_ok()
        );

        assert!(
            matches!(mailbox.recv(), Ok(InventoryMessage::TakeMyItemsResult(
            sender,
            Err((items, TakeMyItemsError::ImDead)))) if sender == mock_sender && items == vec![(Item::Mutexium, 10)])
        );

        drop(inventory_aid);
        thread::sleep(Duration::from_millis(250));
        let _ = inventory_handle.join();
    }
}
