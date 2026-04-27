use std::{thread, time::Duration};

use crate::{
    aid::AID,
    building::Building,
    entity::Entity,
    messages::{PlayerManagerMessage, TaskManagerMessage},
    player_manager,
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
        let task = AID::new(|_, _| {});
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
        let building = Building::new(self.world.clone());
        let building2 = Building::new(self.world.clone());
        let worker = Entity::new(self.world.clone(), self.task.clone(), (10, 3));
        let _ = self
            .world
            .send(WorldManagerMessage::PlaceBuilding((3, 3), building.clone()));
        let _ = self.world.send(WorldManagerMessage::PlaceBuilding(
            (15, 3),
            building2.clone(),
        ));
        let _ = self
            .world
            .send(WorldManagerMessage::Move((10, 3), worker.clone()));
        thread::sleep(Duration::from_secs(1));
        let _ = worker.send(crate::messages::EntityMessage::Task(
            crate::messages::Task::MoveTo((14, 3)),
        ));
        thread::sleep(Duration::from_secs(1));
    }
}
