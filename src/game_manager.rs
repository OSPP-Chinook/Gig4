use crate::{
    aid::AID,
    building::Building,
    worker::Worker,
    item::Item,
    messages::PlayerManagerMessage,
    player_manager,
    task_manager::{self, TaskManagerMessage},
    world_manager::{self, WorldManagerMessage, init_world_grid},
};

pub struct GameManager {
    world: AID<WorldManagerMessage>,
    task: AID<TaskManagerMessage>,
    player: AID<PlayerManagerMessage>,
}

impl GameManager {
    pub fn new() -> Self {
        let grid = init_world_grid();

        let world = AID::new({
            let grid = grid.clone();
            |aid, mailbox| world_manager::main(aid, mailbox, grid)
        });
        let task = AID::new({
            let grid = grid.clone();
            |aid, mailbox| task_manager::main(aid, mailbox, grid)
        });
        let player = AID::new({
            let world = world.clone();
            let grid = grid.clone();
            |aid, mailbox| {
                let _ = player_manager::render_loop(aid, mailbox, world, grid);
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
            (7, 6),
            (65, 35),
            (65, 36),
            (65, 37),
            (65, 38),
            (65, 39),
            (65, 40),
            (65, 41),
            (65, 42),
            (65, 43),
            (65, 44),
            (65, 45),
            (55, 35),
            (55, 36),
            (55, 37),
            (55, 38),
            (55, 39),
            (55, 40),
            (55, 41),
            (55, 42),
            (55, 43),
            (55, 44),
            (55, 45),

            (55, 45),
            (56, 45),
            (57, 45),
            (58, 45),
            (59, 45),
            (60, 45),
            (61, 45),
            (62, 45),
            (63, 45),
            (64, 45),
            (65, 45),
            (55, 35),
            (56, 35),
            (57, 35),
            (58, 35),
            (59, 35),
            (60, 35),
            (61, 35),
            (62, 35),
            (63, 35),
            (64, 35),
            (65, 35),

        ] {
            let _ = self.world.send(WorldManagerMessage::PlaceObstacle(pos));
        }

        let building = Building::new(self.world.clone());
        let _ = self
            .world
            .send(WorldManagerMessage::PlaceBuilding((3, 5), building.clone()));
        let _ = building.send(crate::messages::EntityMessage::Task(
            task_manager::Task::Produce(0),
        ));
        let building = Building::new(self.world.clone());
        let _ = self.world.send(WorldManagerMessage::PlaceBuilding(
            (23, 12),
            building.clone(),
        ));
        let _ = building.send(crate::messages::EntityMessage::Task(
            task_manager::Task::Produce(0),
        ));
        let building = Building::new(self.world.clone());
        let _ = self.world.send(WorldManagerMessage::PlaceBuilding(
            (25, 4),
            building.clone(),
        ));
        let building = Building::new(self.world.clone());
        let _ = self.world.send(WorldManagerMessage::PlaceBuilding(
            (6, 12),
            building.clone(),
        ));
        let building = Building::new(self.world.clone());
        let _ = self.world.send(WorldManagerMessage::PlaceBuilding(
            (15, 3),
            building.clone(),
        ));
        let _ = building.send(crate::messages::EntityMessage::Task(
            task_manager::Task::Produce(0),
        ));

        let worker = Worker::new(self.world.clone(), self.task.clone(), (10, 3));
        let _ = self
            .world
            .send(WorldManagerMessage::PlaceWorker((10, 3), worker.clone()));
        let worker = Worker::new(self.world.clone(), self.task.clone(), (7, 10));
        let _ = self
            .world
            .send(WorldManagerMessage::PlaceWorker((7, 10), worker.clone()));
        let worker = Worker::new(self.world.clone(), self.task.clone(), (60, 40));
        let _ = self
            .world
            .send(WorldManagerMessage::PlaceWorker((60, 40), worker.clone()));
        let _ = self.task.send(TaskManagerMessage::CreatePath(
            Item::Mutexium,
            (15, 3),
            (3, 5),
        ));
        let _ = self.task.send(TaskManagerMessage::CreatePath(
            Item::Mutexium,
            (6, 12),
            (15, 3),
        ));
        let _ = self.task.send(TaskManagerMessage::CreatePath(
            Item::Mutexium,
            (23, 12),
            (25, 4),
        ));
        let _ = self.task.send(TaskManagerMessage::CreatePath(
            Item::Mutexium,
            (3, 5),
            (6, 12),
        ));

    }
}
