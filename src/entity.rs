use crate::aid::AID;
use crate::messages::{EntityMessage, PlayerManagerMessage, Task, TaskManagerMessage};
use crate::world_manager::{Pos, WorldManagerMessage};
use std::sync::mpsc::Receiver;

pub struct Entity {
    world: AID<WorldManagerMessage>,
    task: AID<TaskManagerMessage>,
    current_pos: Pos,
    pending_move: Option<Pos>,
    self_aid: AID<EntityMessage>,
}

impl Entity {
    pub fn new(
        world: AID<WorldManagerMessage>,
        task: AID<TaskManagerMessage>,
        start_pos: Pos,
    ) -> AID<EntityMessage> {
        AID::new(move |aid, mailbox| {
            let mut entity = Entity {
                world: world,
                task: task,
                current_pos: start_pos,
                pending_move: None,
                self_aid: aid.clone(),
            };

            entity.run(mailbox);
        })
    }

    fn run(&mut self, mailbox: Receiver<EntityMessage>) {
        for msg in mailbox {
            match msg {
                EntityMessage::Task(task) => {
                    self.handle_task(task);
                }

                EntityMessage::KillYourself => {
                    self.world
                        .send(WorldManagerMessage::KillMe(self.self_aid.clone()));
                    break;
                }

                EntityMessage::Ok => {
                    //world manager godkände flyyten
                    //uppdatera entitys pos ! exempel self.pos = (1,6);
                    if let Some(pos) = self.pending_move.take() {
                        self.current_pos = pos;
                    }
                }

                EntityMessage::Err => {
                    // world manager neckade flytten
                    // ingen ändring i pos
                    self.pending_move = None;
                }
            }
        }
    }

    fn handle_task(&mut self, task: Task) {
        match task {
            Task::MoveTo(pos) => {
                self.pending_move = Some(pos);

                let _ = self
                    .world
                    .send(WorldManagerMessage::Move(pos, self.self_aid.clone()));
            }

            Task::Idle => {
                //gör inget
            }
        }
    }
}
