use std::{sync::mpsc::Receiver, thread, time::Duration};

use crate::{aid::AID, messages::EntityMessage, world_manager::WorldManagerMessage};

const MACHINE_TICK_SPEED: Duration = Duration::from_secs(1);

enum Item {} //Temporary

//Definition for recipe, should probably be defined somewhere else
pub struct Recipe {
    input: Vec<(Item, usize)>,
    output: Vec<(Item, usize)>,
    pub recipe_time: usize, //recipe time in machine "cycles"/ticks
}

struct Building {
    world_aid: AID<WorldManagerMessage>,
    self_aid: AID<EntityMessage>,
    mailbox: Receiver<EntityMessage>,
    //inventory: AID<InventoryMessage>
}

impl Building {
    pub fn new(world: AID<WorldManagerMessage>) -> AID<EntityMessage> {
        return AID::new(move |aid, mailbox| {
            let mut building = Building {
                world_aid: world,
                self_aid: aid.clone(),
                mailbox: mailbox,
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
                    EntityMessage::Ok => {
                        if let Some(recipe) = &active_recipe
                            && waiting
                        {
                            current_process = Some(recipe.recipe_time);
                        }
                        waiting = false;
                    }
                    EntityMessage::Err => waiting = false,
                    EntityMessage::Task(task) => continue, //Update task
                }
            }
            if let Some(recipe) = &active_recipe
                && current_process == None
                && !waiting
            {
                //request recources in inventory
            }
            if let Some(time_left) = current_process {
                if time_left == 0 {
                    //recipe done
                    //insert output to inventory
                    //continue maybe
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
