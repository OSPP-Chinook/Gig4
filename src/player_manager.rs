use std::sync::mpsc::Receiver;
use std::time::Duration;
use rand::{rngs::ChaCha8Rng, RngExt, SeedableRng};

use crate::{
    aid::AID,
    messages::PlayerManagerMessage,
    world_manager::{HEIGHT, Tile, WIDTH, RawWorldArray, WorldGrid, WorldManagerMessage},
};
use crossterm::event::{Event, KeyCode, KeyEventKind, poll, read};
use ratatui::Frame;
use ratatui::layout::Constraint::Length;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Borders, Paragraph};

// Width and height of a tile on the screen in characters
// Needs to be u16 for ratatui
const TILE_SIZE: (u16, u16) = (3, 2);

#[derive(Copy, Clone)]
struct Camera(i32, i32);

impl Camera {
    fn change(&mut self, dx: i32, dy: i32) {
        // limit camera from going outside world
        let width = WIDTH.try_into().unwrap();
        let height = HEIGHT.try_into().unwrap();
        let mut x = self.0 + dx;
        let mut y = self.1 + dy;
        if x < 0 {
            x = 0;
        }
        if y < 0 {
            y = 0;
        }
        if x >= width {
            x = width - 1;
        }
        if y >= height {
            y = height - 1;
        }

        self.0 = x;
        self.1 = y;
    }
}

// Default: 1. Set to -1 for inverted movement.
// The default setting looks weird now, but it will make sense when the world is more populated.
const MOVE_CAMERA: i32 = 1;

// We do this for two reasons:
// 1. To have 2 copies of the world for comparing
// 2. To not lock the whole world while rendering
fn get_copy_of_world(world_array: &WorldGrid) -> RawWorldArray {
    let world = &world_array.lock().unwrap();
    let copy = world.to_vec();
    return copy;
}

pub fn render_loop(
    aid: AID<PlayerManagerMessage>,
    mailbox: Receiver<PlayerManagerMessage>,
    world: AID<WorldManagerMessage>,
    world_array: WorldGrid,
) -> Result<(), Box<dyn std::error::Error>> {
    ratatui::run(|terminal| {
        // camera starts centered on the world
        let mut camera = Camera(
            (WIDTH / 2).try_into().unwrap(),
            (HEIGHT / 2).try_into().unwrap(),
        );

        
        let mut old_world = get_copy_of_world(&world_array);
        
        loop {
            //read all messages in mailbox
            while let Ok(msg) = mailbox.try_recv() {
                match msg {
                    // TODO: Handle more message types
                    _ => {}
                }
            }

            let new_world = get_copy_of_world(&world_array);
            terminal.draw(|frame| render(frame, &old_world, &new_world, camera))?;
            old_world = new_world;

            // 50 ms looks better with animations
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

fn render(frame: &mut Frame, old_world_array: &RawWorldArray, world_array: &RawWorldArray, camera: Camera) {
    let world_area = frame.area();

    let box_w = world_area.width / TILE_SIZE.0;
    let box_h = world_area.height / TILE_SIZE.1;
    
    // this is repeated several times, so it's a closure here
    let get_rect_from_world_xy = |x: i32, y: i32| {
        // divide by 2 to get center of screen
        let draw_pos = (
            x + (box_w / 2) as i32 - camera.0,
            y + (box_h / 2) as i32 - camera.1,
        );
        return if
            0 <= draw_pos.0 && draw_pos.0 < box_w.into() &&
            0 <= draw_pos.1 && draw_pos.1 < box_h.into()
        {
            // tile in visible area
            let rx: i32 = world_area.x as i32 + TILE_SIZE.0 as i32 * draw_pos.0;
            let ry: i32 = world_area.y as i32 + TILE_SIZE.1 as i32 * draw_pos.1;
            Some(Rect::new(rx as u16, ry as u16, TILE_SIZE.0, TILE_SIZE.1))
        } else {
            // tile outside visible area
            None
        };
    };

    // iterate over visible area
    // draw solid background if outside world
    for y in 0..box_h {
        for x in 0..box_w {
            // inverse of draw_pos higher up
            let world_pos = (
                x as i32 - (box_w / 2) as i32 + camera.0,
                y as i32 - (box_h / 2) as i32 + camera.1,
            );
            
            if
                0 <= world_pos.0 && world_pos.0 < WIDTH as i32 &&
                0 <= world_pos.1 && world_pos.1 < HEIGHT as i32
            {
                // if inside world: skip
                continue;
            }
            
            // draw a background in the "World" area
            // this is so the player can tell the difference between buildable area and surrounding borders
            let rect = Rect::new(
                world_area.x + TILE_SIZE.0 * x,
                world_area.y + TILE_SIZE.1 * y,
                TILE_SIZE.0,
                TILE_SIZE.1,
            );
            
            let border_tile = "...\n...";
            let square = Paragraph::new(border_tile).gray();
            frame.render_widget(square, rect);
        }
    }

    // aquire lock until it falls out of scope
    // let world_array = &world_array.lock().unwrap();
    
    // a set seed gives us the same values in the world every time
    let mut rng = ChaCha8Rng::seed_from_u64(5);

    // draw background
    // we do this separately so objects are correctly layered on top of the background
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            // needs to run on every iteration
            // skipping an iteration would mess up the order
            let tile_rand: u16 = rng.random();
            
            let mut rect_at_pos = match get_rect_from_world_xy(x as i32, y as i32) {
                None => continue,
                Some(rect) => rect
            };
            
            let random_tiles = [".", " .", "   .", "\n.", "\n .", "\n  ."];
            
            // 1/16 chance
            if tile_rand < 4004 {
                let square = Paragraph::new(random_tiles[(tile_rand % 6) as usize]).gray();
                frame.render_widget(square, rect_at_pos);
            }
        }
    }

    // draw world
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let tile = &world_array[y][x];
            
            let mut animation_dx = 0;
            let mut animation_dy = 0;
            
            if y > 0        && is_same_tile(&old_world_array[y-1][x], tile) {animation_dy = -1}
            if y+1 < HEIGHT && is_same_tile(&old_world_array[y+1][x], tile) {animation_dy = 1}
            if x > 0        && is_same_tile(&old_world_array[y][x-1], tile) {animation_dx = -1}
            if x+1 < WIDTH  && is_same_tile(&old_world_array[y][x+1], tile) {animation_dx = 1}
            
            // divide by 2 to get center of screen
            let draw_pos = (
                x as i32 + (box_w / 2) as i32 - camera.0,
                y as i32 + (box_h / 2) as i32 - camera.1,
            );
            let mut rect_at_pos = if
                0 <= draw_pos.0 && draw_pos.0 < box_w.into() &&
                0 <= draw_pos.1 && draw_pos.1 < box_h.into()
            {
                // tile in visible area
                let rx: i32 = world_area.x as i32 + TILE_SIZE.0 as i32 * draw_pos.0 + animation_dx;
                let ry: i32 = world_area.y as i32 + TILE_SIZE.1 as i32 * draw_pos.1 + animation_dy;
                Rect::new(rx as u16, ry as u16, TILE_SIZE.0, TILE_SIZE.1)
            } else {
                // tile outside visible area
                continue;
            };
            
            let mut rect_at_pos = match get_rect_from_world_xy(x as i32, y as i32) {
                None => continue,
                Some(rect) => rect
            };
            
            rect_at_pos.x = (rect_at_pos.x as i32 + animation_dx) as u16;
            rect_at_pos.y = (rect_at_pos.y as i32 + animation_dy) as u16;
            
            
            match tile {
                Tile::Empty => {}
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
