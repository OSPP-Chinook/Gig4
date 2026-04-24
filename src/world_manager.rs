use std::{collections::HashMap, sync::mpsc::Receiver};

use crate::{
    aid::AID,
    messages::{EntityMessage, PlayerManagerMessage}, player_manager::WorldArray,
};

const WIDTH: usize = 20;
const HEIGHT: usize = 10;

pub type Pos = (usize, usize);

#[derive(Clone)]
pub enum WorldManagerMessage {
    Stop, // is only necessary if there are circular AIDs (which there probably will be)
    Move(Pos, AID<EntityMessage>),
    KillMe(AID<EntityMessage>),
    TileInfo(Pos, AID<PlayerManagerMessage>),
    GetDisplay(AID<PlayerManagerMessage>),
}

#[derive(Clone)]
pub enum Tile {
    Empty,
    Worker(AID<EntityMessage>),
    Building(AID<EntityMessage>),
}

fn display(grid: &[[Tile; WIDTH]; HEIGHT]) -> String {
    return "TODO".to_string();
}

fn get_tile(grid: &mut [[Tile; WIDTH]; HEIGHT], pos: Pos) -> Option<&mut Tile> {
    return grid.get_mut(pos.1)?.get_mut(pos.0);
}

pub fn main(_this: AID<WorldManagerMessage>, mailbox: Receiver<WorldManagerMessage>) {
    let mut grid: WorldArray =
        std::array::from_fn(|_| std::array::from_fn(|_| Tile::Empty));
    let mut entity_lookup: HashMap<AID<EntityMessage>, Pos> = HashMap::new();

    for msg in mailbox {
        match msg {
            WorldManagerMessage::Stop => break,
            WorldManagerMessage::Move(pos, aid) => {
                // check if pos in bounds
                if let Some(tile) = get_tile(&mut grid, pos) {
                    // check if pos empty
                    if let Tile::Empty = *tile {
                        *tile = Tile::Worker(aid.clone());
                        entity_lookup.insert(aid.clone(), pos);
                        let _ = aid.send(EntityMessage::Ok);
                    } else {
                        let _ = aid.send(EntityMessage::Err);
                    }
                } else {
                    let _ = aid.send(EntityMessage::Err);
                }
            }
            WorldManagerMessage::TileInfo(pos, aid) => {
                if let Some(tile) = get_tile(&mut grid, pos) {
                    // TODO: send tile
                    let _ = aid.send(PlayerManagerMessage::TODO(grid.clone()));
                } else {
                    // TODO: send Err
                    let _ = aid.send(PlayerManagerMessage::TODO(grid.clone()));
                }
            }
            WorldManagerMessage::KillMe(aid) => {
                if let Some(pos) = entity_lookup.remove(&aid) {
                    if let Some(tile) = get_tile(&mut grid, pos) {
                        *tile = Tile::Empty;
                    }
                }
                // no response necessary
            }
            WorldManagerMessage::GetDisplay(aid) => {
                // TODO: send display(&grid)
                let _ = aid.send(PlayerManagerMessage::TODO(grid.clone()));
            }
        }
    }
}
