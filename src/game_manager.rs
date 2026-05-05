use crate::{
    aid::AID,
    building::Building,
    entity::Entity,
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
        let _ = building.send(crate::messages::EntityMessage::Task(
            task_manager::Task::Produce(0),
        ));

        let worker = Entity::new(self.world.clone(), self.task.clone(), (10, 3));
        let _ = self
            .world
            .send(WorldManagerMessage::PlaceWorker((10, 3), worker.clone()));
        let _ = self.task.send(TaskManagerMessage::CreatePath(
            Item::Mutexium,
            (15, 3),
            (3, 5),
        ));
    }
}
