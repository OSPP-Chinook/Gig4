use std::{thread, time::Duration};

use crate::{
    aid::AID,
    building::Building,
    entity::Entity,
    item::Item,
    messages::{EntityMessage, PlayerManagerMessage},
    player_manager,
    task_manager::{self, Task, TaskManagerMessage},
    world_manager::{self, WorldManagerMessage},
};

pub struct GameManager {
    world: AID<WorldManagerMessage>,
    task: AID<TaskManagerMessage>,
    player: AID<PlayerManagerMessage>,
}

impl GameManager {
    pub fn new() -> Self {
        let world = AID::new(world_manager::main);
        let task = AID::new(|aid, mailbox| {
            task_manager::main(aid, mailbox);
        });
        let player = AID::new({
            let world = world.clone();
            move |aid, mailbox| {
                let _ = player_manager::render_loop(aid, mailbox, world);
            }
        });

        Self {
            world,
            task,
            player,
        }
    }

    pub fn run(&self) {
        self.demo();

        loop {
            std::thread::park();
        }
    }

    fn demo(&self) {
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
        ] {
            let _ = self.world.send(WorldManagerMessage::PlaceObstacle(pos));
        }

        let building = Building::new(self.world.clone());
        let _ = self
            .world
            .send(WorldManagerMessage::PlaceBuilding((3, 5), building.clone()));

        let building = Building::new(self.world.clone());
        let _ = self.world.send(WorldManagerMessage::PlaceBuilding(
            (15, 3),
            building.clone(),
        ));

        let worker = Entity::new(self.world.clone(), self.task.clone(), (10, 3));
        let _ = self
            .world
            .send(WorldManagerMessage::PlaceWorker((10, 3), worker.clone()));

        let worker = Entity::new(self.world.clone(), self.task.clone(), (10, 5));
        let _ = self
            .world
            .send(WorldManagerMessage::PlaceWorker((10, 5), worker.clone()));

        let _ = self.task.send(TaskManagerMessage::CreatePath(
            Item::Mutexium,
            (14, 3),
            (3, 4),
        ));
        let _ = self.task.send(TaskManagerMessage::CreatePath(
            Item::Mutexium,
            (14, 3),
            (3, 4),
        ));
    }
}
