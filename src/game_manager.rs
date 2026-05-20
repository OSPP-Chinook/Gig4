use std::{sync::mpsc, thread, time::Duration};

use crate::{
    aid::AID,
    assets::{Assets, BuildingId, ItemId, WorkerId},
    messages::PlayerManagerMessage,
    player_manager,
    task_manager::{self, TaskManagerMessage},
    world_manager::{self, WorldManagerMessage, init_world_grid},
};
use std::{path::Path, sync::Arc};

#[derive(Clone)]
pub enum GameManagerMessage {
    Quit,
}

pub fn main(this: AID<GameManagerMessage>, mailbox: mpsc::Receiver<GameManagerMessage>) {
    let assets = match Assets::load(Path::new("assets")) {
        Ok(assets) => Arc::new(assets),
        Err(err) => {
            eprintln!("Failed to load assets. {err}");
            return;
        }
    };

    let grid = init_world_grid();

    let (task, task_handle) = task_manager::new_joinable(grid.clone());
    let (world, world_handle) = world_manager::new_joinable(grid.clone(), task.clone(), assets.clone());
    let (player, player_handle) =
        player_manager::new_joinable(grid.clone(), world.clone(), this.clone(), assets);

    demo(&world, &task);

    for msg in mailbox {
        match msg {
            GameManagerMessage::Quit => break,
        }
    }

    let _ = world.send(WorldManagerMessage::Quit);
    let _ = task.send(TaskManagerMessage::Quit);
    let _ = player.send(PlayerManagerMessage::Quit); // probably redundant but doesn't hurt

    drop(world);
    drop(task);
    drop(player);

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

    let _ = world.send(WorldManagerMessage::SpawnBuilding(
        (3, 5),
        BuildingId::from("factory"),
        false,
    ));
    let _ = world.send(WorldManagerMessage::SpawnBuilding(
        (15, 3),
        BuildingId::from("factory"),
        false,
    ));

    let _ = world.send(WorldManagerMessage::SpawnWorker(
        (10, 3),
        WorkerId::from("worker"),
    ));

    // TODO: make CreatePath use aids so this is not necessary
    thread::sleep(Duration::from_millis(500));

    let _ = task.send(TaskManagerMessage::CreatePath(
        ItemId::from("mutexium"),
        (15, 3),
        (3, 5),
    ));
}
