use std::{sync::mpsc::Receiver, thread, time::Duration};

use crate::{
    aid::{AID, AIDHandle},
    inventory::{self, InventoryMessage},
    item::Item,
    messages::{EntityMessage, ItemTransferError, TaskError},
    task_manager::Task,
    world_manager::WorldManagerMessage,
    zombie,
};

const MACHINE_TICK_SPEED: Duration = Duration::from_secs(1);

//Definition for recipe, should probably be defined somewhere else
pub struct Recipe {
    input: Vec<(Item, usize)>,
    output: Vec<(Item, usize)>,
    pub recipe_time: usize, //recipe time in machine "cycles"/ticks
}

pub struct Building {
    world_aid: AID<WorldManagerMessage>,
    self_aid: AID<EntityMessage>,
    inventory: AID<InventoryMessage>,
}

impl Building {
    pub fn new(world: AID<WorldManagerMessage>) -> AID<EntityMessage> {
        return Building::new_joinable(world).0;
    }

    pub fn new_joinable(world: AID<WorldManagerMessage>) -> (AID<EntityMessage>, AIDHandle) {
        return AID::new_joinable(move |aid, mailbox| {
            let mut building = Building::create(aid, world);
            building.run(&mailbox);

            building.destroy();
            zombie::entity_zombie(mailbox);
        });
    }

    fn create(self_aid: AID<EntityMessage>, world_aid: AID<WorldManagerMessage>) -> Self {
        Building {
            world_aid,
            self_aid,
            inventory: inventory::init(),
        }
    }

    fn destroy(self) {
        let _ = self.inventory.send(InventoryMessage::KillYourself);
        drop(self);
    }

    fn run(&mut self, mailbox: &Receiver<EntityMessage>) {
        let mut active_recipe: Option<Recipe> = None;
        let mut current_process: Option<usize> = None;
        let mut waiting = false;
        'outer: loop {
            //read all messages in mailbox
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
                                current_process = Some(recipe.recipe_time);
                            }
                            if let Some(time) = &current_process
                                && waiting
                            {
                                current_process = None;
                            }
                            waiting = false;
                        }
                        Err(
                            ItemTransferError::RecipeChange | ItemTransferError::InsufficientItems,
                        ) => {
                            current_process = None;
                            waiting = false;
                        }
                        Err(ItemTransferError::ImDead | ItemTransferError::TheyreDead) => {} // TODO: something went wrong
                    },

                    EntityMessage::TaskResponse(res) => match res {
                        Ok(task) => {
                            if let Task::Produce(index) = task {
                                //get recipes from static data.
                                active_recipe = Some(Recipe {
                                    input: vec![],
                                    output: vec![(Item::Mutexium, 10)],
                                    recipe_time: 5,
                                });

                                // inform waitign of recipe change
                                let _ = self.inventory.send(InventoryMessage::ChangeRecipe);
                            }
                        }
                        Err(TaskError::ImDead) => {} // should receive KillYourself shortly
                    }, //Update task
                    EntityMessage::GetInventory(aid) => {
                        let _ = aid.send(EntityMessage::GetInventoryResponse(Ok(self
                            .inventory
                            .clone())));
                    }
                    _ => {}
                }
            }

            if let Some(recipe) = &active_recipe
                && current_process == None
                && !waiting
            {
                if recipe.input.is_empty() {
                    current_process = Some(recipe.recipe_time);
                    waiting = false;
                } else {
                    _ = self.inventory.send(InventoryMessage::Remove(
                        self.self_aid.clone(),
                        recipe.input.clone(),
                    ));
                    waiting = true;
                }
            }

            if let Some(time_left) = current_process {
                if time_left == 0 {
                    _ = self.inventory.send(InventoryMessage::Add(
                        self.self_aid.clone(),
                        active_recipe.as_ref().unwrap().output.clone(),
                    ));
                    current_process = None;
                    continue;
                } else {
                    current_process = Some(time_left - 1);
                }
            }
            thread::sleep(MACHINE_TICK_SPEED);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_building() {
        let world: AID<WorldManagerMessage> = AID::new(|_, _| ());
        let building = Building::new(world);
        let _ = building.send(EntityMessage::KillYourself);
    }
}
