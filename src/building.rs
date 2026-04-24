use std::{sync::mpsc::Receiver, thread, time::Duration};

use crate::{
    aid::AID,
    inventory::{self, InventoryMessage},
    item::Item,
    messages::EntityMessage,
    world_manager::WorldManagerMessage,
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
    mailbox: Receiver<EntityMessage>,
    inventory: AID<InventoryMessage>,
}

impl Building {
    pub fn new(world: AID<WorldManagerMessage>) -> AID<EntityMessage> {
        return AID::new(move |aid, mailbox| {
            let mut building = Building {
                world_aid: world,
                self_aid: aid.clone(),
                mailbox: mailbox,
                inventory: inventory::init(),
            };
            building.run();
        });
    }

    fn run(&mut self) {
        let mut active_recipe: Option<Recipe> = None;
        let mut current_process: Option<usize> = None;
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
                            current_process = Some(recipe.recipe_time);
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

                    EntityMessage::Task(task) => continue, //Update task
                    EntityMessage::Ok => {}
                    EntityMessage::Err => {}
                }
            }
            if let Some(recipe) = &active_recipe
                && current_process == None
                && !waiting
            {
                let _ = self.inventory.send(InventoryMessage::Remove(
                    self.self_aid.clone(),
                    recipe.input[0],
                ));
                waiting = true;
            }
            if let Some(time_left) = current_process {
                if time_left == 0 {
                    let _ = self.inventory.send(InventoryMessage::Add(
                        self.self_aid.clone(),
                        active_recipe.as_ref().unwrap().output[0],
                    ));
                    continue;
                } else {
                    current_process = Some(time_left - 1);
                }
            }
            thread::sleep(MACHINE_TICK_SPEED);
        }
    }
}

mod tests {
    use super::*;

    #[test]
    fn create_building() {
        let world: AID<WorldManagerMessage> = AID::new(|_, _| ());
        let building = Building::new(world);
        let _ = building.send(EntityMessage::KillYourself);
    }
}
