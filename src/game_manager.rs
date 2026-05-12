use std::sync::mpsc;

use crate::{
    aid::AID,
    item::Item,
    messages::PlayerManagerMessage,
    player_manager,
    task_manager::{self, TaskManagerMessage},
    world_manager::{self, WorldManagerMessage, init_world_grid},
};

pub enum GameManagerMessage {
    Quit,
}

pub fn main(this: AID<GameManagerMessage>, mailbox: mpsc::Receiver<GameManagerMessage>) {
    let grid = init_world_grid();

    let (task, task_handle) = AID::new_joinable({
        let grid = grid.clone();
        |aid, mailbox| task_manager::main(aid, mailbox, grid)
    });
    let (world, world_handle) = AID::new_joinable({
        let task = task.clone();
        let grid = grid.clone();
        |aid, mailbox| world_manager::main(aid, mailbox, task, grid)
    });
    let (player, player_handle) = AID::new_joinable({
        let world = world.clone();
        let grid = grid.clone();
        |aid, mailbox| {
            let _ = player_manager::render_loop(aid, mailbox, this, world, grid);
        }
    });

    demo(&world, &task);

    for msg in mailbox {
        match msg {
            GameManagerMessage::Quit => break,
        }
    }

    let _ = world.send(WorldManagerMessage::Quit);
    let _ = task.send(TaskManagerMessage::Quit);
    let _ = player.send(PlayerManagerMessage::Quit); // probably redundant but doesn't hurt

    let _ = world_handle.join();
    let _ = task_handle.join();
    let _ = player_handle.join();
}

fn demo(world: &AID<WorldManagerMessage>, task: &AID<TaskManagerMessage>) {
    // place obstacles
    for pos in [
        (2, 3),
        (2, 2),
        (3, 2),
        (4, 2),
        (5, 2),
        (6, 2),
        (7, 2),
        (8, 2),
        (8, 3),
        (8, 4),
        (8, 5),
        (8, 6),
        (9, 6),
        (7, 6),
    ] {
        let _ = world.send(WorldManagerMessage::SpawnObstacle(pos));
    }

    let _ = world.send(WorldManagerMessage::SpawnBuilding((3, 5), false));
    let _ = world.send(WorldManagerMessage::SpawnBuilding((15, 3), true));

    let _ = world.send(WorldManagerMessage::SpawnWorker((10, 3)));

    let _ = task.send(TaskManagerMessage::CreatePath(
        Item::Mutexium,
        (15, 3),
        (3, 5),
    ));
}
