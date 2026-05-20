use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};
use rand::{RngExt, SeedableRng, rngs::ChaCha8Rng};

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind, poll, read,
};

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Offset, Rect, Spacing},
    style::Stylize,
    symbols::merge::MergeStrategy,
    widgets::{Block, Borders, Clear, Padding, Paragraph},
};

use std::{
    cmp::Ordering,
    io::stdout,
    rc::Rc,
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

use crate::aid::AIDHandle;
use crate::game_manager::GameManagerMessage;
use crate::zombie;
use crate::{
    EntityMessage,
    aid::AID,
    task_manager::Task,
    world_manager::{HEIGHT, Pos, RawWorldArray, Tile, WIDTH, WorldGrid, WorldManagerMessage},
};

// Width and height of a tile on the screen in characters
// Needs to be u16 for ratatui
const TILE_SIZE: (u16, u16) = (3, 2);

// Default: 1. Set to -1 for inverted movement.
// The default setting looks weird now, but it will make sense when the world is more populated.
const MOVE_CAMERA: i32 = 1;

#[derive(Clone)]
pub enum PlayerManagerMessage {
    ShowTileInfo(Pos, Tile),
    TileNotFound(Pos),
    Notification(String), // if we ever want to notify the player of anything special
    InventoryStatusResult(Option<String>),
    CurrentTaskResult(Option<Task>),
    Quit,
}

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

enum InputResult {
    Pause,
    Quit,
    Continue,
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
    selected_aid: Option<AID<EntityMessage>>,
) -> Option<AID<EntityMessage>> {
    let mut found = match selected_aid {
        Some(_) => false,
        None => true,
    };

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let tile = &world_array[y][x];
            match tile {
                Tile::Worker(aid, _) => {
                    if found {
                        return Some(aid.clone());
                    }
                    match &selected_aid {
                        Some(sel_aid) => {
                            if aid == sel_aid {
                                found = true;
                            }
                        }
                        None => (),
                    };
                }
                _ => (),
            }
        }
    }
    return None;
}

fn get_worker_camera(world_array: &RawWorldArray, sel_aid: &AID<EntityMessage>) -> Option<Camera> {
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let tile = &world_array[y][x];
            match tile {
                Tile::Worker(aid, _) => {
                    if aid == sel_aid {
                        return Some(Camera(x as i32, y as i32));
                    }
                }
                Tile::Building(aid, _) => {
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

pub fn new_joinable(
    grid: WorldGrid,
    world: AID<WorldManagerMessage>,
    game: AID<GameManagerMessage>,
) -> (AID<PlayerManagerMessage>, AIDHandle) {
    return AID::new_joinable(|aid, mailbox| {
        let _ = render_loop(aid, &mailbox, world, grid);

        let _ = game.send(GameManagerMessage::Quit);
        drop(game);
        let _ = execute!(stdout(), DisableMouseCapture);
        zombie::player_manager_zombie(mailbox);
    });
}

fn render_loop(
    aid: AID<PlayerManagerMessage>,
    mailbox: &Receiver<PlayerManagerMessage>,
    world: AID<WorldManagerMessage>,
    world_array: WorldGrid,
) -> Result<(), Box<dyn std::error::Error>> {
    ratatui::run(|terminal| {
        // camera starts centered on the world
        let mut camera = Camera(
            (WIDTH / 2).try_into().unwrap(),
            (HEIGHT / 2).try_into().unwrap(),
        );

        let _ = execute!(stdout(), EnableMouseCapture);

        let mut old_world = get_copy_of_world(&world_array);
        let mut selected_aid = None;
        let mut time_to_wait = 0;

        // For Status information
        let mut inventory_string: Option<String> = None;
        let mut task: Option<Task> = None;

        let mut fps: f32 = 0.;
        let mut second_counter = Instant::now();
        let mut frames = 0;

        let mut paused = false;

        loop {
            if let Some(true) = check_mailbox(&mailbox, &mut inventory_string, &mut task) {
                break Ok(());
            }

            if let Some(val) = get_inputs(
                &mut camera,
                &mut selected_aid,
                &old_world,
                time_to_wait,
                &terminal.get_frame().area(),
            ) {
                match val {
                    InputResult::Continue => {}

                    InputResult::Quit => {
                        break Ok(());
                    }

                    InputResult::Pause => {
                        if paused {
                            //unpause
                            _ = world.send(WorldManagerMessage::Unpause);
                        } else {
                            //pause
                            _ = world.send(WorldManagerMessage::Pause);
                        }
                        paused = !paused;
                    }
                }
            }

            if let Some(selected_aid) = selected_aid.clone() {
                _ = selected_aid.send(EntityMessage::FetchInventoryStatus(aid.clone()));
                _ = selected_aid.send(EntityMessage::FetchCurrentTask(aid.clone()));
            }

            let time_0 = Instant::now();
            let new_world = get_copy_of_world(&world_array);
            let time_1 = Instant::now();
            terminal.draw(|frame| {
                render(
                    frame,
                    &old_world,
                    &new_world,
                    camera,
                    (time_0, time_1, fps),
                    &selected_aid,
                    &inventory_string,
                    &task,
                )
            })?;
            frames += 1;
            old_world = new_world;

            if second_counter.elapsed() >= Duration::from_secs(1) {
                fps = frames as f32 / second_counter.elapsed().as_secs_f32();
                frames = 0;
                second_counter = Instant::now();
            }

            // reduce wait time by how much time we spent rendering
            time_to_wait = 50u128
                .checked_sub(time_0.elapsed().as_millis())
                .unwrap_or(0) as u64;
        }
    })
}

fn mouse_to_grid_pos((x, y): (u16, u16), world_area: &Rect, camera: Camera) -> (i32, i32) {
    let box_w = world_area.width / TILE_SIZE.0;
    let box_h = world_area.height / TILE_SIZE.1;
    return (
        (x / TILE_SIZE.0) as i32 - (box_w / 2) as i32 + camera.0,
        (y / TILE_SIZE.1) as i32 - (box_h / 2) as i32 + camera.1,
    );
}

fn check_mailbox(
    mailbox: &Receiver<PlayerManagerMessage>,
    inventory_string: &mut Option<String>,
    task: &mut Option<Task>,
) -> Option<bool> {
    //read all messages in mailbox
    while let Ok(msg) = mailbox.try_recv() {
        match msg {
            PlayerManagerMessage::Quit => return Some(true),

            // TODO: Handle more message types
            PlayerManagerMessage::InventoryStatusResult(res) => {
                *inventory_string = res;
            }
            PlayerManagerMessage::CurrentTaskResult(res) => {
                *task = res;
            }
            _ => {}
        }
    }

    return Some(false);
}

fn get_inputs(
    camera: &mut Camera,
    selected_aid: &mut Option<AID<EntityMessage>>,
    old_world: &RawWorldArray,
    time_to_wait: u64,
    frame: &Rect,
) -> Option<InputResult> {
    let mut key_event: Option<KeyEvent> = None;
    let mut mouse_event: Option<MouseEvent> = None;

    // 50 ms looks better with animations
    if poll(Duration::from_millis(time_to_wait)).ok()? {
        match read().ok()? {
            Event::Key(event) if event.kind == KeyEventKind::Press => {
                // Det här måste ske utanför input handler eftersom
                // det ska stänga av loopen
                if event.code == KeyCode::Char('q') {
                    return Some(InputResult::Quit); // Break
                }
                if event.code == KeyCode::Char('p') {
                    return Some(InputResult::Pause);
                }
                key_event = Some(event);
            }
            Event::Mouse(event) => {
                mouse_event = Some(event);
            }
            _ => {}
        }
    }

    let mut input: Input = Input {
        mouse_pos: None,
        mouse_click: MouseClick::None,
        key: None,
    };

    parse_input_keyboard(&mut input, &key_event, camera, selected_aid, &old_world);
    parse_input_mouse(
        &mut input,
        &mouse_event,
        frame,
        *camera,
        selected_aid,
        old_world,
    );

    return Some(InputResult::Continue);
}

fn parse_input_keyboard(
    input: &mut Input,
    event_opt: &Option<KeyEvent>,
    camera: &mut Camera,
    selected_aid: &mut Option<AID<EntityMessage>>,
    old_world: &RawWorldArray,
) {
    if event_opt.is_none() {
        return;
    }

    let event: KeyEvent = event_opt.unwrap();
    match event.code {
        KeyCode::Char('w') => {
            camera.change(0, -MOVE_CAMERA);
        }
        KeyCode::Char('s') => {
            camera.change(0, MOVE_CAMERA);
        }
        KeyCode::Char('a') => {
            camera.change(-MOVE_CAMERA, 0);
        }
        KeyCode::Char('d') => {
            camera.change(MOVE_CAMERA, 0);
        }
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
        _ => input.key = Some(event.code),
    }
}

fn parse_input_mouse(
    input: &mut Input,
    event_opt: &Option<MouseEvent>,
    world_area: &Rect,
    camera: Camera,
    selected_aid: &mut Option<AID<EntityMessage>>,
    old_world: &RawWorldArray,
) {
    if event_opt.is_none() {
        return;
    }

    let event: MouseEvent = event_opt.unwrap();
    match event.kind {
        MouseEventKind::Moved => {
            input.mouse_pos = Some((event.column, event.row));
        }
        MouseEventKind::Down(MouseButton::Left) => {
            input.mouse_pos = Some((event.column, event.row));
            input.mouse_click = MouseClick::Left;
            let (x, y) = mouse_to_grid_pos((event.column, event.row), world_area, camera);
            if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
                let tile = &old_world[y as usize][x as usize];
                if let Tile::Building(aid, _) = tile {
                    *selected_aid = Some(aid.clone());
                } else if let Tile::Worker(aid, _) = tile {
                    *selected_aid = Some(aid.clone());
                } else {
                    *selected_aid = None;
                }
            } else {
                *selected_aid = None;
            }
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
        Tile::Worker(aid, _) => match new_tile {
            Tile::Worker(aid_new, _) => {
                return aid == aid_new;
            }
            _ => false,
        },
        Tile::Building(aid, _) => match new_tile {
            Tile::Building(aid_new, _) => {
                return aid == aid_new;
            }
            _ => false,
        },
        _ => false,
    }
}

fn get_movement(old_world: &RawWorldArray, tile: &Tile, (x, y): (usize, usize)) -> (i32, i32) {
    let mut dx = 0;
    let mut dy = 0;
    if y > 0 && is_same_tile(&old_world[y - 1][x], tile) {
        dy = -1
    }
    if y + 1 < HEIGHT && is_same_tile(&old_world[y + 1][x], tile) {
        dy = 1
    }
    if x > 0 && is_same_tile(&old_world[y][x - 1], tile) {
        dx = -1
    }
    if x + 1 < WIDTH && is_same_tile(&old_world[y][x + 1], tile) {
        dx = 1
    }
    return (dx, dy);
}

fn render(
    frame: &mut Frame,
    old_world_array: &RawWorldArray,
    world_array: &RawWorldArray,
    camera: Camera,
    (time_0, time_1, fps): (Instant, Instant, f32),
    selected_aid: &Option<AID<EntityMessage>>,
    inventory_string: &Option<String>,
    task: &Option<Task>,
) {
    let world_area = frame.area();

    render_world_in_area(frame, &world_area, &old_world_array, &world_array, camera);

    if let Some(sel_aid) = selected_aid {
        render_selected_info(
            frame,
            sel_aid,
            &world_area,
            world_array,
            old_world_array,
            inventory_string,
            task,
        );
    }

    // return; // don't draw fps
    let time_2 = Instant::now();
    render_fps(
        frame,
        time_1.duration_since(time_0),
        time_2.duration_since(time_1),
        fps,
    );
}

fn render_selected_info(
    frame: &mut Frame,
    sel_aid: &AID<EntityMessage>,
    world_area: &Rect,
    world_array: &RawWorldArray,
    old_world_array: &RawWorldArray,
    inventory_string: &Option<String>,
    task: &Option<Task>,
) {
    let (width, height) = (world_area.width / 3, world_area.height / 2);
    let m = 2; // margin: 2 x border, which doubles as 2 x space for animation

    if width > m && height > m {
        // pov_area encloses a whole number of tiles
        let width = (width - m) / TILE_SIZE.0 * TILE_SIZE.0 + m;
        let height = (height - m) / TILE_SIZE.1 * TILE_SIZE.1 + m;

        let horizontal_layout = Layout::horizontal([width])
            .flex(ratatui::layout::Flex::End)
            .split(frame.area());

        let layout = Layout::vertical([
            Constraint::Length(frame.area().height - height),
            Constraint::Length(height),
        ])
        .spacing(Spacing::Overlap(1))
        .split(horizontal_layout[0]);

        frame.render_widget(Clear, layout[0]);
        frame.render_widget(Clear, layout[1]);

        if let Some(pov_camera) = get_worker_camera(&world_array, sel_aid) {
            render_pov(frame, pov_camera, &layout, world_array, old_world_array);
        }

        // render this last so it covers any part of the world sticking out
        frame.render_widget(
            Block::new()
                .borders(Borders::ALL)
                .title("─ POV: you're a worker ")
                .merge_borders(MergeStrategy::Replace),
            layout[1],
        );

        render_status(frame, layout, inventory_string, task);
    }
}

fn render_pov(
    frame: &mut Frame,
    pov_camera: Camera,
    layout: &Rc<[Rect]>,
    world_array: &RawWorldArray,
    old_world_array: &RawWorldArray,
) {
    let pov_area_inner = layout[1].inner(Margin::new(1, 1));
    let Camera(x, y) = pov_camera;
    let (x, y) = (x as usize, y as usize);
    let (dx, dy) = get_movement(&old_world_array, &world_array[y][x], (x, y));
    let pov_area_inner = pov_area_inner.offset(Offset { x: -dx, y: -dy });
    render_world_in_area(
        frame,
        &pov_area_inner,
        &old_world_array,
        &world_array,
        pov_camera,
    );
}

fn render_status(
    frame: &mut Frame,
    layout: Rc<[Rect]>,
    inventory_string: &Option<String>,
    task: &Option<Task>,
) {
    let sub_layout =
        Layout::vertical([Constraint::Length(7), Constraint::Percentage(75)]).split(layout[0]);

    if let Some(task) = task {
        frame.render_widget(
            Paragraph::new(parse_task(task))
                .block(Block::new().padding(Padding::uniform(2)))
                .alignment(Alignment::Center),
            sub_layout[0],
        );
    } else {
        frame.render_widget(
            Paragraph::new(format!("Fetching Task..."))
                .block(Block::new().padding(Padding::uniform(2)))
                .alignment(Alignment::Center),
            sub_layout[0],
        );
    }

    if let Some(inventory_string) = inventory_string {
        frame.render_widget(
            Paragraph::new(inventory_string.clone())
                .block(Block::new().padding(Padding::uniform(2)))
                .alignment(Alignment::Center),
            sub_layout[1],
        );
    } else {
        frame.render_widget(
            Paragraph::new(format!("Fetching Data..."))
                .block(Block::new().padding(Padding::uniform(2)))
                .alignment(Alignment::Center),
            sub_layout[1],
        );
    }

    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .title("─ Status ")
            .merge_borders(MergeStrategy::Exact),
        layout[0],
    );
}

fn parse_task(task: &Task) -> String {
    let mut parsed_task: String = String::from("Current Task:\n");

    match task {
        Task::MoveTo(pos) => {
            parsed_task.push_str(format!("Moving to ({0}, {1})", pos.0, pos.1).as_str());
        }
        Task::DeliverItem(item, from, to) => {
            parsed_task.push_str(
                format!(
                    "Delivering {0} from ({1}, {2}) to ({3}, {4})",
                    item.to_string(),
                    from.1.0,
                    from.1.1,
                    to.1.0,
                    to.1.1
                )
                .as_str(),
            );
        }
        Task::Idle => {
            parsed_task.push_str("Idling...");
        }
        Task::Produce(amount) => {
            parsed_task.push_str(format!("Producing {} things", amount).as_str());
        }
    }

    return parsed_task;
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

            let rect_at_pos = match get_rect_from_world_xy(x as i32, y as i32) {
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
                // could display different types of workers differently depending on their id
                Tile::Worker(_, _id) => {
                    let square = Paragraph::new("╭─╮\n╰─╯").blue();
                    frame.render_widget(square, rect_at_pos);
                }
                Tile::Building(_, _id) => {
                    let square = Paragraph::new("╔═╗\n╚═╝").red();
                    frame.render_widget(square, rect_at_pos);
                }
            }
        }
    }
}

fn render_fps(frame: &mut Frame, dur_copy: Duration, dur_render: Duration, fps: f32) {
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

    let text = format!("FPS: {:.2}", fps);
    let len = text.len() as u16;
    let rect = Rect::new(width - len, 2, len, 1);
    frame.render_widget(Paragraph::new(text), rect);
}
