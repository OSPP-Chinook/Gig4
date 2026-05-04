use std::{collections::HashMap, sync::mpsc::Receiver};

use crate::{
    aid::AID,
    messages::{EntityMessage, PlayerManagerMessage},
};

pub const WIDTH: usize = 32;
pub const HEIGHT: usize = 12;

pub type Pos = (usize, usize);

#[derive(Clone)]
pub enum WorldManagerMessage {
    Stop, // is only necessary if there are circular AIDs (which there probably will be)
    Move(Pos, AID<EntityMessage>),
    PlaceWorker(Pos, AID<EntityMessage>),
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

pub type WorldGrid = [[Tile; WIDTH]; HEIGHT];
type WorldLookup = HashMap<AID<EntityMessage>, Pos>;

fn get_tile(grid: &mut WorldGrid, pos: Pos) -> Option<&mut Tile> {
    return grid.get_mut(pos.1)?.get_mut(pos.0);
}

fn place_tile(
    grid: &mut WorldGrid,
    entity_lookup: &mut WorldLookup,
    pos: Pos,
    aid: AID<EntityMessage>,
    tile: Tile,
) {
    // check that it does not already have a position
    if let None = entity_lookup.get(&aid) {
        // check if pos in bounds
        if let Some(dest) = get_tile(grid, pos) {
            // check if pos empty
            if let Tile::Empty = *dest {
                *dest = tile;
                // send early to not have to clone aid again
                let _ = aid.send(EntityMessage::Ok);
                entity_lookup.insert(aid, pos);
                return;
            }
        }
    }

    let _ = aid.send(EntityMessage::Err);
}

fn move_tile(
    grid: &mut WorldGrid,
    entity_lookup: &mut WorldLookup,
    pos: Pos,
    aid: AID<EntityMessage>,
) {
    // check if pos is valid
    if let Some(dest) = get_tile(grid, pos) {
        if let Tile::Empty = *dest {
            if let Some(old_pos) = entity_lookup.get(&aid) {
                // all positions in entity_lookup are valid so unwrap will never panic
                let old_tile = get_tile(grid, *old_pos).unwrap();
                let temp = old_tile.clone();
                *old_tile = Tile::Empty;

                // already checked that pos is valid so unwrap will never panic
                *get_tile(grid, pos).unwrap() = temp;
                // send early to not have to clone aid again
                let _ = aid.send(EntityMessage::Ok);
                entity_lookup.insert(aid, pos);
                return;
            }
        }
    }

    let _ = aid.send(EntityMessage::Err);
}

pub fn main(_this: AID<WorldManagerMessage>, mailbox: Receiver<WorldManagerMessage>) {
    let mut grid: WorldGrid = std::array::from_fn(|_| std::array::from_fn(|_| Tile::Empty));
    let mut entity_lookup: WorldLookup = HashMap::new();

    for msg in mailbox {
        match msg {
            WorldManagerMessage::Stop => break,
            WorldManagerMessage::Move(pos, aid) => {
                move_tile(&mut grid, &mut entity_lookup, pos, aid)
            }
            WorldManagerMessage::PlaceWorker(pos, aid) => place_tile(
                &mut grid,
                &mut entity_lookup,
                pos,
                aid.clone(),
                Tile::Worker(aid),
            ),
            WorldManagerMessage::PlaceBuilding(pos, aid) => place_tile(
                &mut grid,
                &mut entity_lookup,
                pos,
                aid.clone(),
                Tile::Building(aid),
            ),
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
