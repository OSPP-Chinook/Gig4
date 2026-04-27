use std::{collections::HashMap, sync::mpsc::Receiver};

use crate::{
    aid::AID,
    messages::{EntityMessage, PlayerManagerMessage},
    player_manager::WorldArray,
};

pub const WIDTH: usize = 16;
pub const HEIGHT: usize = 16;

pub type Pos = (usize, usize);

#[derive(Clone)]
pub enum WorldManagerMessage {
    Stop, // is only necessary if there are circular AIDs (which there probably will be)
    Move(Pos, AID<EntityMessage>),
    PlaceBuilding(Pos, AID<EntityMessage>),
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

fn get_tile(grid: &mut [[Tile; WIDTH]; HEIGHT], pos: Pos) -> Option<&mut Tile> {
    return grid.get_mut(pos.1)?.get_mut(pos.0);
}

pub fn main(_this: AID<WorldManagerMessage>, mailbox: Receiver<WorldManagerMessage>) {
    let mut grid: WorldArray = std::array::from_fn(|_| std::array::from_fn(|_| Tile::Empty));
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
                        let old_pos = entity_lookup.insert(aid.clone(), pos);
                        // remove from old pos if it had one
                        if let Some(old_pos) = old_pos {
                            if let Some(old_tile) = get_tile(&mut grid, old_pos) {
                                *old_tile = Tile::Empty;
                            }
                        }
                        let _ = aid.send(EntityMessage::Ok);
                    } else {
                        let _ = aid.send(EntityMessage::Err);
                    }
                } else {
                    let _ = aid.send(EntityMessage::Err);
                }
            }
            WorldManagerMessage::PlaceBuilding(pos, aid) => {
                // check that it does not already have a position
                if let None = entity_lookup.get(&aid) {
                    // check if pos in bounds
                    if let Some(tile) = get_tile(&mut grid, pos) {
                        // check if pos empty
                        if let Tile::Empty = *tile {
                            *tile = Tile::Building(aid.clone());
                            entity_lookup.insert(aid.clone(), pos);
                            let _ = aid.send(EntityMessage::Ok);
                        } else {
                            let _ = aid.send(EntityMessage::Err);
                        }
                    } else {
                        let _ = aid.send(EntityMessage::Err);
                    }
                } else {
                    let _ = aid.send(EntityMessage::Err);
                }
            }
            WorldManagerMessage::TileInfo(pos, aid) => {
                if let Some(tile) = get_tile(&mut grid, pos) {
                    let _ = aid.send(PlayerManagerMessage::ShowTileInfo(pos, tile.clone()));
                } else {
                    let _ = aid.send(PlayerManagerMessage::TileNotFound(pos));
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
                let _ = aid.send(PlayerManagerMessage::WorldUpdate(grid.clone()));
            }
        }
    }
}
