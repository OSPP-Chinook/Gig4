use std::sync::mpsc::Receiver;
use std::time::Duration;

use crate::{
    aid::AID,
    messages::PlayerManagerMessage,
    world_manager::{HEIGHT, Tile, WIDTH, WorldGrid, WorldManagerMessage},
};
use crossterm::event::{Event, KeyCode, KeyEventKind, poll, read};
use ratatui::Frame;
use ratatui::layout::Constraint::Length;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Borders, Paragraph};

// Width and height of a tile on the screen in characters
const TILE_SIZE: usize = 2;

#[derive(Copy, Clone)]
struct Camera(i32, i32);

impl Camera {
    fn change(&mut self, dx: i32, dy: i32) {
        // limit camera from going outside world
        let width = WIDTH.try_into().unwrap();
        let height = HEIGHT.try_into().unwrap();
        let mut x = self.0 + dx;
        let mut y = self.1 + dy;
        if x < 0 {x = 0;}
        if y < 0 {y = 0;}
        if x >= width {x = width - 1;}
        if y >= height {y = height - 1;}
        
        self.0 = x;
        self.1 = y;
    }
}

// Default: 1. Set to -1 for inverted movement.
// The default setting looks weird now, but it will make sense when the world is more populated.
const MOVE_CAMERA: i32 = 1;

pub fn render_loop(
    aid: AID<PlayerManagerMessage>,
    mailbox: Receiver<PlayerManagerMessage>,
    world: AID<WorldManagerMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    ratatui::run(|terminal| {
        // camera starts centered on the world
        let mut camera = Camera((WIDTH / 2).try_into().unwrap(), (HEIGHT / 2).try_into().unwrap());
        
        let mut world_array: WorldGrid =
                std::array::from_fn(|_| std::array::from_fn(|_| Tile::Empty));
        let mut old_world_array = world_array.clone();
        
        loop {
            let _ = world.send(WorldManagerMessage::GetDisplay(aid.clone()));

            

            for msg in &mailbox {
                match msg {
                    PlayerManagerMessage::WorldUpdate(arr) => {
                        old_world_array = world_array;
                        world_array = arr;
                        let _ = world.send(WorldManagerMessage::GetDisplay(aid.clone()));
                        break;
                    }
                    // TODO: Handle more message types
                    _ => {}
                }
            }

            terminal.draw(|frame| render(frame, &old_world_array, &world_array, camera))?;

            if poll(Duration::from_millis(50))? {
                match read()? {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        match key_event.code {
                            KeyCode::Char('q') => {
                                break Ok(());
                            }
                            KeyCode::Char('w') => {camera.change(0, -MOVE_CAMERA);}
                            KeyCode::Char('s') => {camera.change(0,  MOVE_CAMERA);}
                            KeyCode::Char('a') => {camera.change(-MOVE_CAMERA, 0);}
                            KeyCode::Char('d') => {camera.change( MOVE_CAMERA, 0);}
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    })
}

fn is_same_tile(old_tile: &Tile, new_tile: &Tile) -> bool {
    match old_tile {
        Tile::Empty => {
            false
        }
        Tile::Worker(aid) => {
            match new_tile {
                Tile::Worker(aid_new) => {
                    return aid == aid_new;
                }
                _ => false
            }
        }
        Tile::Building(aid) => {
            match new_tile {
                Tile::Building(aid_new) => {
                    return aid == aid_new;
                }
                _ => false
            }
        }
    }
}

fn render(frame: &mut Frame, old_world_array: &WorldGrid, world_array: &WorldGrid, camera: Camera) {
    let world_area = frame.area().inner(Margin::new(4, 4));
    
    frame.render_widget(
        Block::new().borders(Borders::ALL).title("World"),
        world_area.outer(Margin::new(1, 1)),
    );
    
    let box_dx = 3;
    let box_dy = 2;
    let box_w = world_area.width / box_dx;
    let box_h = world_area.height / box_dy;
    
    for y in 0..box_h {
        for x in 0..box_w {
            // draw a background in the "World" area
            // this is so the player can tell the difference between buildable area and surrounding borders
            let rect = Rect::new(world_area.x + box_dx*x, world_area.y + box_dy*y, 2, 2);
            let square = Paragraph::new(".").gray();
            // frame.render_widget(square, rect);
        }
    }
    
    for y in 0+1..HEIGHT-1 {
        for x in 0+1..WIDTH-1 {
            let tile = &world_array[y][x];
            let old_tile_n = &old_world_array[y-1][x];
            let old_tile_s = &old_world_array[y+1][x];
            let old_tile_w = &old_world_array[y][x-1];
            let old_tile_e = &old_world_array[y][x+1];
            
            let y: i32 = y.try_into().unwrap();
            let x: i32 = x.try_into().unwrap();
            let draw_pos = (x + (box_w/2) as i32 - camera.0, y + (box_h/2) as i32 - camera.1);
            let mut rect_at_pos = if
                0 <= draw_pos.0 && draw_pos.0 < box_w.into() &&
                0 <= draw_pos.1 && draw_pos.1 < box_h.into()
            {
                // tile in visible grid
                let rx = world_area.x + box_dx*(draw_pos.0 as u16);
                let ry = world_area.y + box_dy*(draw_pos.1 as u16);
                Rect::new(rx, ry, box_dx, box_dy)
            } else {
                continue;
            };
            
            if is_same_tile(old_tile_n, tile) {rect_at_pos.y -= 1}
            if is_same_tile(old_tile_s, tile) {rect_at_pos.y += 1}
            if is_same_tile(old_tile_w, tile) {rect_at_pos.x -= 1}
            if is_same_tile(old_tile_e, tile) {rect_at_pos.x += 1}
            
            match tile {
                Tile::Empty => {
                    // overwrite background
                    // let square = Paragraph::new("  \n  ");
                    // frame.render_widget(square, rect_at_pos);
                }
                Tile::Worker(_aid) => {
                    let square = Paragraph::new("╭—╮\n╰—╯").blue();
                    frame.render_widget(square, rect_at_pos);
                }
                Tile::Building(_aid) => {
                    let square = Paragraph::new("╔═╗\n╚═╝").red();
                    frame.render_widget(square, rect_at_pos);
                }
            }
        }
    }
}
