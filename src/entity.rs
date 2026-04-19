use crate::aid::AID;
use crate::messages::{EntityMessage, Task, TaskManagerMessage};
use crate::world_manager::{Pos, WorldManagerMessage};
use std::sync::mpsc::Receiver;


/// Ren logik- och state för en entity.
/// 
/// EntityCore ansvarar för:
/// - att hålla nuvarande position (`current_pos`)
/// - att lagra en väntande flytt (`pending_move`)
/// - att behandla inkommande tasks
/// - att uppdatera state baserat på Ok/Err från WorldManager
///
/// Innehåller ingen actor‑logik.  
/// Används av `Entity` som den faktiska logikdelen.
pub struct EntityCore {
    current_pos: Pos,
    pending_move: Option<Pos>,
}

impl EntityCore {
    // skapar en EntityCore  med given start position
    pub fn new(start_pos: Pos) -> EntityCore {
        EntityCore {
            current_pos: start_pos,
            pending_move: None,
        }
    }

    /// Behandlar en Task och returnerar eventuell Move-position
    /// som Entity-aktorn ska skicka till WorldManager.
    pub fn apply_task(&mut self, task: Task)-> Option<Pos> {
        match task {
            Task::MoveTo(pos) => {
                self.pending_move = Some(pos);
                Some(pos)
            }

            Task::Idle => None
        }
    }
    /// Anropas när WorldManager godkänner en flytt.
    /// Uppdaterar current_pos och tömmer pending_move.
    pub fn apply_ok(&mut self) {
        if let Some(pos) = self.pending_move.take() {
            self.current_pos = pos;
        }
    }
    /// Anropas när WorldManager nekar en flytt.
    /// Tömmer pending_move utan att ändra current_pos.
    pub fn apply_err(&mut self) {
        self.pending_move = None;
    }
}


/// Actor som representerar en Entity i världen.
/// 
/// Entity ansvarar för:
/// - att ta emot `EntityMessage`
/// - att vidarebefordra tasks till `EntityCore`
/// - att skicka `WorldManagerMessage::Move` när core signalerar en flytt
/// - att uppdatera core-state baserat på Ok/Err från WorldManager
///
/// All logik ligger i `EntityCore`.  
/// Entity själv hanterar endast actor‑beteende och message‑flow.
pub struct Entity {
    core: EntityCore,
    world_aid: AID<WorldManagerMessage>,
    task_aid: AID<TaskManagerMessage>,
    // senare task_id: AID<inventoryMesseges>
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
                core: EntityCore::new(start_pos),
                world_aid: world,
                task_aid: task,
                self_aid: aid.clone(),
            };

            entity.run(mailbox);
        })
    }

    fn run(&mut self, mailbox: Receiver<EntityMessage>) {
        for msg in mailbox {
            match msg {
                EntityMessage::Task(task) => {
                    
                    if let Some(pos) = self.core.apply_task(task) {
                        
                        let _ = self.world_aid.send(WorldManagerMessage::Move(pos, self.self_aid.clone()));
                    }
                }

                EntityMessage::KillYourself => {
                    let _ = self
                        .world_aid
                        .send(WorldManagerMessage::KillMe(self.self_aid.clone()));
                    break;
                }

                EntityMessage::Ok => {
                    //world manager godkände flyyten
                    //uppdatera EntityCore-> cunnrent_pos
                    self.core.apply_ok();
                }

                EntityMessage::Err => {
                    // world manager neckade flytten
                    // ingen ändring i pos
                    self.core.apply_err();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn apply_task() {
        
        let start_pos = (1,1);
        let mut core = EntityCore::new(start_pos);

        let new_pos = (10,10);
        let task = Task::MoveTo(new_pos);

        assert_eq!(core.apply_task(task), Some(new_pos));
        assert_eq!(core.pending_move, Some(new_pos));

    }

    #[test]
    fn apply_ok(){

        let start_pos = (1,1);
        let mut core = EntityCore::new(start_pos);

        let new_pos = (20,20);
        let task = Task::MoveTo(new_pos);
        core.apply_task(task);
        core.apply_ok();
        assert_eq!(core.current_pos, new_pos);

    }

    #[test]
    fn apply_err() {
        
        let start_pos = (1,1);
        let mut core = EntityCore::new(start_pos);

        let new_pos = (3,8);

        let task = Task::MoveTo(new_pos);

        core.apply_task(task);
        core.apply_err();

        assert_eq!(core.current_pos, start_pos);
        assert_eq!(core.pending_move,None);

    }
}
