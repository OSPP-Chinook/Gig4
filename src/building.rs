use std::{sync::mpsc::Receiver, thread, time::Duration};

use crate::{aid::AID, messages::EntityMessage, recipe::Recipe};


const MACHINE_TICK_SPEED: Duration = Duration::from_secs(1);

pub fn main(aid: AID<EntityMessage>, mailbox: Receiver<EntityMessage>) {
    //needs world state AID
    //needs to be able to set building stats (allowed recipes, speed etc).
    //needs to create inventory
    let mut active_recipe: Option<Recipe> = None;
    let mut current_process: Option<usize> = None;
    let mut waiting = false;
    'outer: loop {
        //read all messages in mailbox
        while let Ok(msg) = mailbox.try_recv() {
            match msg {
                EntityMessage::KillYourself => {
                    //message world killme
                    break 'outer;
                }
                EntityMessage::Ok => {
                    waiting = false;
                    if let Some(recipe) = &active_recipe
                        && waiting
                    {
                        current_process = Some(recipe.recipe_time);
                    }
                }
                EntityMessage::Err => waiting = false,
                EntityMessage::Task(task) => continue, //TODO
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
