use rand::{RngExt, SeedableRng, rngs::ChaCha8Rng};
use std::sync::mpsc::Receiver;
use std::time::Duration;
use std::time::Instant;
use std::cmp::Ordering;
use std::cmp;

use crate::{
    aid::AID,
    messages::PlayerManagerMessage,
    EntityMessage,
    world_manager::{HEIGHT, RawWorldArray, Tile, WIDTH, WorldGrid, WorldManagerMessage},
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind, poll, read};
use ratatui::Frame;
use ratatui::layout::Constraint::Length;
use ratatui::layout::{Constraint, Layout, Margin, Rect, Offset};
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Borders, Paragraph, Clear};

// Width and height of a tile on the screen in characters
// Needs to be u16 for ratatui
const TILE_SIZE: (u16, u16) = (3, 2);

enum MouseClick {
    None,
    Left,
    Right,
}
struct Input {
    mouse_pos: Option<(u16, u16)>, // (x, y)
    mouse_click: MouseClick,
    key: Option<KeyCode>,
}

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

fn get_next_worker(
    world_array: &RawWorldArray,
    selected_aid: Option<AID<EntityMessage>>
) -> Option<AID<EntityMessage>> {
    let mut found = match selected_aid {
        Some(_) => false,
        None => true,
    };
    
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let tile = &world_array[y][x];
            match tile {
                Tile::Worker(aid) => {
                    if found {
                        return Some(aid.clone());
                    }
                    match &selected_aid {
                        Some(sel_aid) => {
                            if aid == sel_aid {
                                found = true;
                            }
                        },
                        None => (),
                    };
                }
                _ => (),
            }
        }
    }
    return None;
}

fn get_worker_camera(
    world_array: &RawWorldArray,
    sel_aid: &AID<EntityMessage>,
) -> Option<Camera> {
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let tile = &world_array[y][x];
            match tile {
                Tile::Worker(aid) => {
                    if aid == sel_aid {
                        return Some(Camera(x as i32, y as i32));
                    }
                }
                _ => (),
            }
        }
    }
    // can fail because worker might have died
    return None;
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
            (14).try_into().unwrap(),
            (7).try_into().unwrap(),
        );

        let mut old_world = get_copy_of_world(&world_array);
        
        let mut selected_aid = None;

        let mut time_to_wait = 0;

        loop {
            //read all messages in mailbox
            while let Ok(msg) = mailbox.try_recv() {
                match msg {
                    // TODO: Handle more message types
                    _ => {}
                }
            }

            let mut key_event: Option<KeyEvent> = None;
            let mut mouse_event: Option<MouseEvent> = None;

            // 50 ms looks better with animations
            if poll(Duration::from_millis(time_to_wait))? {
                match read()? {
                    Event::Key(event) if event.kind == KeyEventKind::Press => {
                        // Det här måste ske utanför input handler eftersom 
                        // det ska stänga av loopen
                        if event.code == KeyCode::Char('q') {
                            break Ok(());
                        }
                        key_event = Some(event);
                    }
                    Event::Mouse(event) => {
                        mouse_event = Some(event);
                    }
                    _ => {}
                }
            }

            let mut input: Input = Input { mouse_pos: None, mouse_click: MouseClick::None, key: None };

            parse_input_keyboard(&mut input, &key_event, &mut camera, &mut selected_aid, &old_world);
            parse_input_mouse(&mut input, &mouse_event);

            // terminal.draw(|frame| render(frame, world_array, camera, input))?;

            let time_0 = Instant::now();
            let new_world = get_copy_of_world(&world_array);
            let time_1 = Instant::now();
            terminal.draw(|frame| render(frame, &old_world, &new_world, camera, (time_0, time_1), &selected_aid))?;
            old_world = new_world;
            
            // reduce wait time by how much time we spent rendering
            // I can't tell if this makes any difference, or if it doesn't work with poll()
            let time_to_wait = 50;
            let time_to_wait = cmp::max(0, time_to_wait - time_0.elapsed().as_millis() as i64) as u64;

        }
    })
}

fn parse_input_keyboard(
    input: &mut Input,
    event_opt: &Option<KeyEvent>,
    camera: &mut Camera,
    selected_aid: &mut Option<AID<EntityMessage>>,
    old_world: &RawWorldArray,
) {
    if event_opt.is_none() {return;}

    let event: KeyEvent = event_opt.unwrap();
    match event.code {
        KeyCode::Char('w') => {camera.change(0, -MOVE_CAMERA);}
        KeyCode::Char('s') => {camera.change(0,  MOVE_CAMERA);}
        KeyCode::Char('a') => {camera.change(-MOVE_CAMERA, 0);}
        KeyCode::Char('d') => {camera.change( MOVE_CAMERA, 0);}
        KeyCode::Char('n') => {
            *selected_aid = get_next_worker(&old_world, selected_aid.clone());
        }
        KeyCode::Char('m') => {
            if let Some(sel_aid) = &selected_aid {
                if let Some(new_camera) = get_worker_camera(&old_world, &sel_aid) {
                    *camera = new_camera;
                }
            }
        }
        _ => {input.key = Some(event.code)}
    }
}

fn parse_input_mouse(input: &mut Input, event_opt: &Option<MouseEvent>) {
    if event_opt.is_none() {return;}

    let event: MouseEvent = event_opt.unwrap();
    match event.kind {
        MouseEventKind::Moved => {
            input.mouse_pos = Some((event.column, event.row));
        }
        MouseEventKind::Down(MouseButton::Left) => {
            input.mouse_pos = Some((event.column, event.row));
            input.mouse_click = MouseClick::Left;
        }
        MouseEventKind::Down(MouseButton::Right) => {
            input.mouse_pos = Some((event.column, event.row));
            input.mouse_click = MouseClick::Right;
        }
        _ => {}
    }
}

fn is_same_tile(old_tile: &Tile, new_tile: &Tile) -> bool {
    match old_tile {
        Tile::Worker(aid) => match new_tile {
            Tile::Worker(aid_new) => {
                return aid == aid_new;
            }
            _ => false,
        },
        Tile::Building(aid) => match new_tile {
            Tile::Building(aid_new) => {
                return aid == aid_new;
            }
            _ => false,
        },
        _ => false,
    }
}

fn get_movement(
    old_world: &RawWorldArray,
    tile: &Tile,
    (x, y): (usize, usize),
) -> (i32, i32) {
    let mut dx = 0;
    let mut dy = 0;
    if y > 0          && is_same_tile(&old_world[y - 1][x], tile) {dy = -1}
    if y + 1 < HEIGHT && is_same_tile(&old_world[y + 1][x], tile) {dy = 1}
    if x > 0          && is_same_tile(&old_world[y][x - 1], tile) {dx = -1}
    if x + 1 < WIDTH  && is_same_tile(&old_world[y][x + 1], tile) {dx = 1}
    return (dx, dy);
}

fn render(
    frame: &mut Frame,
    old_world_array: &RawWorldArray,
    world_array: &RawWorldArray,
    camera: Camera,
    (time_0, time_1): (Instant, Instant),
    selected_aid: &Option<AID<EntityMessage>>,
) {
    let world_area = frame.area();
    
    render_world_in_area(frame, &world_area, &old_world_array, &world_array, camera);
    
    if let Some(sel_aid) = selected_aid {
        let (width, height) = (world_area.width / 3, world_area.height / 2);
        let m = 2; // margin: 2 x border, which doubles as 2 x space for animation
        if width > m && height > m {
            // pov_area encloses a whole number of tiles
            let width = (width - m) / TILE_SIZE.0 * TILE_SIZE.0 + m;
            let height = (height - m) / TILE_SIZE.1 * TILE_SIZE.1 + m;
            let pov_area = Rect::new(world_area.width - width, world_area.height - height, width, height);

            frame.render_widget(Clear, pov_area);
            
            let pov_area_inner = pov_area.inner(Margin::new(1, 1));
            if let Some(pov_camera) = get_worker_camera(&world_array, sel_aid) {
                let Camera(x, y) = pov_camera;
                let (x, y) = (x as usize, y as usize);
                let (dx, dy) = get_movement(&old_world_array, &world_array[y][x], (x, y));
                let pov_area_inner = pov_area_inner.offset(Offset {x: -dx, y: -dy});
                render_world_in_area(frame, &pov_area_inner, &old_world_array, &world_array, pov_camera);
            }
            
            // render this last so it covers any part of the world sticking out
            frame.render_widget(
                Block::new().borders(Borders::ALL).title("─ POV: you're a worker "),
                pov_area,
            );
        }
    }
    
    // return; // don't draw fps
    let time_2 = Instant::now();
    render_fps(
        frame,
        time_1.duration_since(time_0),
        time_2.duration_since(time_1),
    );
}

fn render_world_in_area(
    frame: &mut Frame,
    world_area: &Rect,
    old_world_array: &RawWorldArray,
    world_array: &RawWorldArray,
    camera: Camera,
) {
    let box_w = world_area.width / TILE_SIZE.0;
    let box_h = world_area.height / TILE_SIZE.1;
    
    let is_row_in_world = |y: i32| {
        let draw_y = y + (box_h / 2) as i32 - camera.1;
        if draw_y < 0 {
            return Ordering::Less;
        }
        if draw_y >= box_h.into() {
            return Ordering::Greater;
        }
        return Ordering::Equal;
    };

    // this is repeated several times, so it's a closure here
    let get_rect_from_world_xy = |x: i32, y: i32| {
        // divide by 2 to get center of screen
        let draw_pos = (
            x + (box_w / 2) as i32 - camera.0,
            y + (box_h / 2) as i32 - camera.1,
        );
        return if 0 <= draw_pos.0
            && draw_pos.0 < box_w.into()
            && 0 <= draw_pos.1
            && draw_pos.1 < box_h.into()
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

            if 0 <= world_pos.0
                && world_pos.0 < WIDTH as i32
                && 0 <= world_pos.1
                && world_pos.1 < HEIGHT as i32
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
    // this rng is used to set the seed for every row
    let mut rng = ChaCha8Rng::seed_from_u64(5);

    // draw background
    // we do this separately so objects are correctly layered on top of the background
    for y in 0..HEIGHT {
        // this rng applies to every row
        // needs to run on every iteration
        // skipping an iteration would mess up the order
        let mut rng_row = ChaCha8Rng::seed_from_u64(rng.random());
        
        match is_row_in_world(y as i32) {
            Ordering::Less => continue,
            Ordering::Greater => break,
            _ => (),
        }
        
        for x in 0..WIDTH {
            // needs to run on every iteration
            // skipping an iteration would mess up the order
            let tile_rand: u16 = rng_row.random();

            let mut rect_at_pos = match get_rect_from_world_xy(x as i32, y as i32) {
                None => continue,
                Some(rect) => rect,
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
        match is_row_in_world(y as i32) {
            Ordering::Less => continue,
            Ordering::Greater => break,
            _ => (),
        }
        
        for x in 0..WIDTH {
            let tile = &world_array[y][x];

            let mut animation_dx = 0;
            let mut animation_dy = 0;

            let mut rect_at_pos = match get_rect_from_world_xy(x as i32, y as i32) {
                None => continue,
                Some(rect) => rect,
            };
            
            let (animation_dx, animation_dy) = get_movement(&old_world_array, tile, (x, y));

            rect_at_pos.x = (rect_at_pos.x as i32 + animation_dx) as u16;
            rect_at_pos.y = (rect_at_pos.y as i32 + animation_dy) as u16;

            match tile {
                Tile::Empty => {}
                Tile::Obstacle => {
                    let square = Paragraph::new("███\n███").green();
                    frame.render_widget(square, rect_at_pos);
                }
                Tile::Worker(_aid) => {
                    let square = Paragraph::new("╭─╮\n╰─╯").blue();
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

fn render_fps(frame: &mut Frame, dur_copy: Duration, dur_render: Duration) {
    let width = frame.area().width;
    
    // time to run get_copy_of_world() 
    let text = format!("{} ms", dur_copy.as_millis());
    let len = text.len() as u16;
    let rect = Rect::new(width - len, 0, len, 1);
    frame.render_widget(Paragraph::new(text), rect);
    
    // time to run render()
    let text = format!("{} ms", dur_render.as_millis());
    let len = text.len() as u16;
    let rect = Rect::new(width - len, 1, len, 1);
    frame.render_widget(Paragraph::new(text), rect);
}
