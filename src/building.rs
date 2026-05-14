use crate::{
    aid::AID,
    assets::{Assets, BuildingId, RecipeAsset},
    inventory::{self, InventoryMessage},
    messages::EntityMessage,
    task_manager::Task,
    world_manager::WorldManagerMessage,
};
use std::{
    sync::{Arc, mpsc::Receiver},
    thread,
    time::Duration,
};

const MACHINE_TICK_SPEED: Duration = Duration::from_millis(100);

pub struct Building {
    world_aid: AID<WorldManagerMessage>,
    self_aid: AID<EntityMessage>,
    mailbox: Receiver<EntityMessage>,
    inventory: AID<InventoryMessage>,
    assets: Arc<Assets>,
    id: BuildingId,
}

impl Building {
    pub fn new(
        world: AID<WorldManagerMessage>,
        assets: Arc<Assets>,
        id: BuildingId,
    ) -> AID<EntityMessage> {
        return AID::new(move |aid, mailbox| {
            let inventory_size = assets.buildings.get(&id).unwrap().inventory_size;

            let mut building = Building {
                world_aid: world,
                self_aid: aid.clone(),
                mailbox: mailbox,
                inventory: inventory::init(assets.clone(), inventory_size),
                assets,
                id,
            };
            building.run();
        });
    }

    fn run(&mut self) {
        let mut active_recipe: Option<RecipeAsset> = None;
        let mut current_process: Option<Duration> = None;
        let mut waiting = false;
        'outer: loop {
            //read all messages in mailbox
            while let Ok(msg) = self.mailbox.try_recv() {
                match msg {
                    EntityMessage::KillYourself => {
                        let _ = self
                            .world_aid
                            .send(WorldManagerMessage::KillMe(self.self_aid.clone()));
                        break 'outer;
                    }
                    EntityMessage::InventoryOk => {
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
                    EntityMessage::InventoryErr => {
                        current_process = None;
                        waiting = false;
                    }

                    EntityMessage::Task(task) => {
                        if let Task::Produce(index) = task {
                            // Get recipes from static data
                            if let Some(recipe) = self.assets.recipes.get(&index) {
                                active_recipe = Some(recipe.clone());
                            }
                        }
                    } // Update task
                    EntityMessage::Ok => {}
                    EntityMessage::Err => {}
                    EntityMessage::GetInventory(aid) => {
                        let _ = aid.send(EntityMessage::SendInventory(self.inventory.clone()));
                    }
                    _ => {}
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
                        recipe
                            .inputs
                            .iter()
                            .map(|s| (s.id.clone(), s.count))
                            .collect(),
                    ));
                    waiting = true;
                }
            }

            if let Some(time_left) = current_process {
                if time_left.is_zero() {
                    _ = self.inventory.send(InventoryMessage::Add(
                        self.self_aid.clone(),
                        active_recipe
                            .as_ref()
                            .unwrap()
                            .outputs
                            .iter()
                            .map(|s| (s.id.clone(), s.count))
                            .collect(),
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
        let world: AID<WorldManagerMessage> = AID::new(|_, _| ());
        let building = Building::new(world, assets, BuildingId::from("factory"));
        let _ = building.send(EntityMessage::KillYourself);
    }
}
