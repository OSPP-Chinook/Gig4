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
