use crate::{
    aid::{AID, AIDHandle},
    assets::{Assets, BuildingId, RecipeAsset},
    inventory::{self, InventoryMessage},
    messages::{EntityMessage, ItemTransferError, PlayerManagerMessage, TaskError},
    task_manager::{Task, TaskManagerMessage},
    world_manager::WorldManagerMessage,
    zombie,
};
use std::{
    sync::{Arc, mpsc::Receiver},
    thread,
    time::Duration,
    vec,
};

const MACHINE_TICK_SPEED: Duration = Duration::from_millis(100);

pub struct Building {
    world_aid: AID<WorldManagerMessage>,
    task_aid: AID<TaskManagerMessage>,
    self_aid: AID<EntityMessage>,
    inventory: AID<InventoryMessage>,
    assets: Arc<Assets>,
    id: BuildingId,
}

impl Building {
    pub fn new(
        world: AID<WorldManagerMessage>,
        task: AID<TaskManagerMessage>,
        assets: Arc<Assets>,
        id: BuildingId,
    ) -> AID<EntityMessage> {
        return Building::new_joinable(world, task, assets, id).0;
    }

    pub fn new_joinable(
        world: AID<WorldManagerMessage>,
        task: AID<TaskManagerMessage>,
        assets: Arc<Assets>,
        id: BuildingId,
    ) -> (AID<EntityMessage>, AIDHandle) {
        return AID::new_joinable(move |aid, mailbox| {
            let mut building = Building::create(aid, world, task, assets, id);
            building.run(&mailbox);

            building.destroy();
            zombie::entity_zombie(mailbox);
        });
    }

    fn create(
        self_aid: AID<EntityMessage>,
        world_aid: AID<WorldManagerMessage>,
        task_aid: AID<TaskManagerMessage>,
        assets: Arc<Assets>,
        id: BuildingId,
    ) -> Self {
        let inventory_size = assets.buildings.get(&id).unwrap().inventory_size;

        Building {
            world_aid,
            task_aid,
            self_aid,
            inventory: inventory::init(assets.clone(), inventory_size),
            assets,
            id,
        }
    }

    fn destroy(self) {
        let _ = self.inventory.send(InventoryMessage::KillYourself);
        let _ = self
            .task_aid
            .send(TaskManagerMessage::KillMe(self.self_aid.clone()));
        drop(self);
    }

    fn run(&mut self, mailbox: &Receiver<EntityMessage>) {
        let mut current_task = Task::Idle;
        let mut active_recipe: Option<RecipeAsset> = None;
        let mut current_process: Option<Duration> = None;
        let mut waiting = false;
        let mut paused = false;
        let mut pause_messages: Vec<EntityMessage> = vec![];
        'outer: loop {
            //read all messages in mailbox
            while paused {
                if let Ok(msg) = mailbox.recv() {
                    match msg {
                        EntityMessage::Unpause => {
                            paused = false;
                            //send back all messages received while paused, could also send back 
                            //directly but then it would constantly read messages and never sleep
                            while let Some(pause_message) = pause_messages.pop() {
                                _ = self.self_aid.send(pause_message);
                            }
                        }
                        
                        EntityMessage::KillYourself => {
                            break 'outer;
                        }

                        EntityMessage::FetchInventoryStatus(pm_aid) => {
                            _ = self.inventory.send(InventoryMessage::GiveStatus(pm_aid));
                        }

                        EntityMessage::FetchCurrentTask(pm_aid) => {
                            _ = pm_aid.send(PlayerManagerMessage::CurrentTaskResult(Some(
                                current_task.clone(),
                            )));
                        }

                        _ => {
                            pause_messages.push(msg);
                        }
                    }
                }
                continue;
            }
            while let Ok(msg) = mailbox.try_recv() {
                match msg {
                    EntityMessage::KillYourself => {
                        break 'outer;
                    }
                    EntityMessage::ItemTransferResponse(res) => match res {
                        Ok(()) => {
                            if let Some(recipe) = &active_recipe
                                && waiting
                                && current_process == None
                            {
                                current_process = Some(Duration::from_millis(recipe.time as u64));
                            }
                            if let Some(time) = &current_process
                                && waiting
                            {
                                current_process = None;
                            }
                            waiting = false;
                        }
                        Err(
                            ItemTransferError::RecipeChange
                            | ItemTransferError::InsufficientItems
                            | ItemTransferError::TooManyItems,
                        ) => {
                            current_process = None;
                            waiting = false;
                        }
                        Err(ItemTransferError::ImDead | ItemTransferError::TheyreDead) => {
                            // something has gone very wrong
                            break 'outer;
                        }
                    },

                    EntityMessage::TaskResponse(res) => match res {
                        Ok(task) => {
                            if let Task::Produce(index) = task.clone() {
                                current_task = task;
                                // Get recipes from static data
                                if let Some(recipe) = self.assets.recipes.get(&index) {
                                    active_recipe = Some(recipe.clone());
                                }

                                // inform waitign of recipe change
                                let _ = self.inventory.send(InventoryMessage::ChangeRecipe);
                            }
                        }
                        Err(TaskError::ImDead) => {} // should receive KillYourself shortly
                    },
                    EntityMessage::GetInventory(aid) => {
                        let _ = aid.send(EntityMessage::GetInventoryResponse(Ok(self
                            .inventory
                            .clone())));
                    }

                    EntityMessage::FetchInventoryStatus(pm_aid) => {
                        _ = self.inventory.send(InventoryMessage::GiveStatus(pm_aid));
                    }

                    EntityMessage::FetchCurrentTask(pm_aid) => {
                        _ = pm_aid.send(PlayerManagerMessage::CurrentTaskResult(Some(
                            current_task.clone(),
                        )));
                    }

                    EntityMessage::FetchAsset(pm_aid) => {
                        _ = pm_aid.send(PlayerManagerMessage::AssetResult(self.id.to_string(), true));
                    } 

                    EntityMessage::Pause => {
                        paused = true;
                        continue 'outer;
                    }


                    EntityMessage::GetInventoryResponse(_)
                    | EntityMessage::MoveResponse(_)
                    | EntityMessage::Unpause => {} // not supposed to happen
                }
            }

            if let Some(recipe) = &active_recipe
                && current_process == None
                && !waiting
            {
                if recipe.inputs.is_empty() {
                    current_process = Some(Duration::from_millis(recipe.time as u64));
                    waiting = false;
                } else {
                    _ = self.inventory.send(InventoryMessage::Remove(
                        self.self_aid.clone(),
                        recipe.inputs.clone(),
                    ));
                    waiting = true;
                }
            }

            if let Some(time_left) = current_process {
                if time_left.is_zero() {
                    _ = self.inventory.send(InventoryMessage::Add(
                        self.self_aid.clone(),
                        active_recipe.as_ref().unwrap().outputs.clone(),
                    ));
                    current_process = None;
                    continue;
                } else {
                    current_process = Some(time_left.saturating_sub(MACHINE_TICK_SPEED));
                }
            }
            thread::sleep(MACHINE_TICK_SPEED);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn create_building() {
        let assets = Arc::new(Assets::load(Path::new("assets")).unwrap());
        let world = AID::mock().0;
        let task = AID::mock().0;
        let building = Building::new(world, task, assets, BuildingId::from("factory"));
        let _ = building.send(EntityMessage::KillYourself);
    }
}
