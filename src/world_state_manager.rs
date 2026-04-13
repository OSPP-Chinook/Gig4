use std::{collections::HashMap, sync::mpsc::{Receiver, Sender},  thread::ThreadId};

use crate::messages::{
    EntityMessage,
    WorldManagerMessage::{self, *},
};

const WIDTH: usize = 16;
const HEIGHT: usize = 16;

pub type Pos = (usize, usize);

enum Tile {
    Empty,
    Entity(AID<EntityMessage>),
}

fn display(grid: &[[Tile; WIDTH]; HEIGHT]) -> String {
    "TODO".to_string()
}

fn gettile(grid: &mut [[Tile; WIDTH]; HEIGHT], pos: Pos) -> Option<&mut Tile> {
    grid.get_mut(pos.0)?.get_mut(pos.1)
}

pub fn main(aid: AID<WorldManagerMessage>, mailbox: Receiver<WorldManagerMessage>) {
    let mut grid: [[Tile; WIDTH]; HEIGHT] =
        std::array::from_fn(|_| std::array::from_fn(|_| Tile::Empty));
    // TODO: make AID hashable so this works
    let mut entity_lookup: HashMap<AID<EntityMessage>, Pos> = HashMap::new();

    for msg in mailbox {
        match msg {
            Move(pos) => {
                let aid; // get from sender AID
                if let Some(tile) = gettile(&mut grid, pos) {
                    *tile = Tile::Entity(aid);
                    // send Ok
                } else {
                    // send Err
                }
            }
            TileInfo(pos) => {
                if let Some(tile) = gettile(&mut grid, pos) {
                    // send tile
                } else {
                    // send Err
                }
            }
            KillMe => {
                let aid; // get from sender AID
                if (let Some(pos) = entity_lookup.get(aid)) {
                    if let Some(tile) = gettile(&mut grid, pos) {
                        *tile = Tile::Empty;
                    }
                }
                // no response necessary
            }
            GetDisplay => {
                // send display(&grid)
            }
        }
    }
}
