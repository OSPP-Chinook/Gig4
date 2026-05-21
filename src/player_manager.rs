use rand::{
    RngExt, SeedableRng, distr::slice::Empty, rngs::ChaCha8Rng, seq::IndexedMutRandom
};

use crossterm::{
    execute,
    event::{
        DisableMouseCapture, 
        EnableMouseCapture, 
        Event, 
        KeyCode, 
        KeyEvent,
        KeyEventKind, 
        MouseButton, 
        MouseEvent, 
        MouseEventKind, 
        poll, 
        read,
    }
};

use ratatui::{
    Frame,
    macros::ratatui_core::widgets,
    style::Stylize,
    symbols::merge::MergeStrategy,
    layout::{
        Alignment, Constraint, Direction, Flex, Layout, Margin, Offset, Position, Rect, Spacing
    },
    widgets::{
        Block, Borders, Clear, Padding, Paragraph, Widget
    },
};

use std::{
    cmp::Ordering, 
    io::stdout,
    rc::Rc, 
    sync::{
        Arc, 
        mpsc::Receiver, 
    },
    time::{
        Duration, Instant
    }
};

use crate::{
    EntityMessage, 
    zombie,
    game_manager::GameManagerMessage,
    task_manager::Task,
    aid::{
        AID, AIDHandle
    }, 
    assets::{
        Assets, WorkerId, BuildingId, RecipeId, ItemId,
    }, 
    world_manager::{ 
        HEIGHT, Pos, RawWorldArray, Tile, WIDTH, WorldGrid, WorldManagerMessage
    }, 
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

#[derive(Clone, PartialEq)]
enum Selection {
    Empty,
    Pending(usize, usize),
    Dummy(usize, usize),
    Worker(usize, usize, AID<EntityMessage>),
    Building(usize, usize, AID<EntityMessage>),
}

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

struct StatusData { 
    inventory_string: Option<String>,
    task: Option<Task>,
}

struct MenuButtonWidget {
    last_area: Rect,
}

impl Widget for &mut MenuButtonWidget {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let block = Block::default().borders(Borders::all());
        self.last_area = block.inner(area);
        block.render(area, buf);
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

fn get_next_entity(
    world_array: &RawWorldArray,
    select: Selection,
) -> Selection {
    let mut found = match select {
        Selection::Empty => true,
        _ => false,
    };

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let tile = &world_array[y][x];
            match tile {
                Tile::Worker(aid, _) => {
                    if found {
                        return Selection::Worker(x, y, aid.clone());
                    }
                    if let Selection::Worker(_, _, sel_aid) = &select {
                        if aid == sel_aid {
                            found = true;
                        }
                    }
                }
                Tile::Building(aid, _) => {
                    if found {
                        return Selection::Building(x, y, aid.clone());
                    }
                    if let Selection::Building(_, _, sel_aid) = &select {
                        if aid == sel_aid {
                            found = true;
                        }
                    }
                }

                _ => (),
            }
        }
    }
    return Selection::Empty;
}

fn update_selection(world_array: &RawWorldArray, select: Selection) -> Selection {
    // skip these - they don't change
    match select {
        Selection::Empty => {return select}
        Selection::Dummy(x, y) => {
            let tile = &world_array[y][x];
            match tile {
                Tile::Dummy => {return select}
                Tile::Empty => {return Selection::Empty}
                _ => {return Selection::Pending(x, y)}
            }
        }
        _ => (),
    }
    // update position, or change pending if found
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let tile = &world_array[y][x];
            match tile {
                Tile::Worker(aid, _) => {
                    if let Selection::Worker(_, _, sel_aid) = &select {
                        if aid == sel_aid {
                            return Selection::Worker(x, y, aid.clone());
                        }
                    }
                    if let Selection::Pending(sx, sy) = select {
                        if x == sx && y == sy {
                            return Selection::Worker(x, y, aid.clone());
                        }
                    }
                }
                Tile::Building(aid, _) => {
                    if let Selection::Building(_, _, sel_aid) = &select {
                        if aid == sel_aid {
                            return Selection::Building(x, y, aid.clone());
                        }
                    }
                    if let Selection::Pending(sx, sy) = select {
                        if x == sx && y == sy {
                            return Selection::Building(x, y, aid.clone());
                        }
                    }
                }
                Tile::Dummy => {
                    if let Selection::Pending(sx, sy) = select {
                        if x == sx && y == sy {
                            return Selection::Dummy(x, y);
                        }
                    }
                }
                _ => (),
            }
        }
    }
    return select;
}

fn get_worker_camera(_world_array: &RawWorldArray, select: &Selection) -> Option<Camera> {
    match select {
        Selection::Empty => {None}
        Selection::Worker(x, y, _) => {Some(Camera(*x as i32, *y as i32))}
        Selection::Building(x, y, _) => {Some(Camera(*x as i32, *y as i32))}
        Selection::Dummy(x, y) => {Some(Camera(*x as i32, *y as i32))}
        Selection::Pending(x, y) => {Some(Camera(*x as i32, *y as i32))}
    }
}

pub fn new_joinable(
    grid: WorldGrid,
    world: AID<WorldManagerMessage>,
    game: AID<GameManagerMessage>,
    assets: Arc<Assets>,
) -> (AID<PlayerManagerMessage>, AIDHandle) {
    return AID::new_joinable(|aid, mailbox| {
        let _ = io_loop(aid, &mailbox, world, grid, assets);

        let _ = game.send(GameManagerMessage::Quit);
        drop(game);
        let _ = execute!(stdout(), DisableMouseCapture);
        zombie::player_manager_zombie(mailbox);
    });
}

fn io_loop(
    aid: AID<PlayerManagerMessage>,
    mailbox: &Receiver<PlayerManagerMessage>,
    world: AID<WorldManagerMessage>,
    world_array: WorldGrid,
    assets: Arc<Assets>,
) -> Result<(), Box<dyn std::error::Error>> {
    ratatui::run(|terminal| {
        // camera starts centered on the world
        let mut camera = Camera(
            (WIDTH / 2).try_into().unwrap(),
            (HEIGHT / 2).try_into().unwrap(),
        );

        let _ = execute!(stdout(), EnableMouseCapture);

        let mut old_world = get_copy_of_world(&world_array);
        let mut select = Selection::Empty;
        let mut select_2 = Selection::Empty;
        let mut time_to_wait = 0;

        // For Status information
        let mut status_data: StatusData = StatusData { 
            inventory_string: None, 
            task: None 
        };

        let mut show_build_menu: bool = false;

        let mut fps: f32 = 0.;
        let mut second_counter = Instant::now();
        let mut frames = 0;

        let mut paused = false;

        let mut menu_buttons: Vec<MenuButtonWidget> = vec![
            MenuButtonWidget { last_area: Rect::new(0, 0, 0, 0) }, // build worker button
            MenuButtonWidget { last_area: Rect::new(0, 0, 0, 0) }, // build factory button
        ];

        let mut is_in_place_mode = false;

        loop {
            if let Some(true) = check_mailbox(&mailbox, &mut status_data) {
                break Ok(());
            }

            if let Some(val) = get_inputs(
                &world,
                &mut camera,
                &mut select,
                &mut select_2,
                &old_world,
                time_to_wait,
                &terminal.get_frame().area(),
                &mut show_build_menu,
                &menu_buttons,
                &mut is_in_place_mode,
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

            if let Selection::Worker(_, _, sel_aid) = select.clone() {
                _ = sel_aid.send(EntityMessage::FetchInventoryStatus(aid.clone()));
                _ = sel_aid.send(EntityMessage::FetchCurrentTask(aid.clone()));
            }
            if let Selection::Building(_, _, sel_aid) = select.clone() {
                _ = sel_aid.send(EntityMessage::FetchInventoryStatus(aid.clone()));
                _ = sel_aid.send(EntityMessage::FetchCurrentTask(aid.clone()));
            }

            let time_0 = Instant::now();
            let new_world = get_copy_of_world(&world_array);
            let time_1 = Instant::now();
            select = update_selection(&new_world, select);
            terminal.draw(|frame| {
                render(
                    frame,
                    &old_world,
                    &new_world,
                    camera,
                    (time_0, time_1, fps),
                    &status_data,
                    show_build_menu,
                    assets.clone(),
                    &select,
                    &select_2,
                    &mut menu_buttons,
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

fn mouse_to_grid_pos((x, y): (u16, u16), world_area: &Rect, camera: Camera) -> Option<(usize, usize)> {
    let box_w = world_area.width / TILE_SIZE.0;
    let box_h = world_area.height / TILE_SIZE.1;
    let grid_x = (x / TILE_SIZE.0) as i32 - (box_w / 2) as i32 + camera.0;
    let grid_y = (y / TILE_SIZE.1) as i32 - (box_h / 2) as i32 + camera.1;
    
    if grid_x >= 0 && grid_x < WIDTH as i32 && grid_y >= 0 && grid_y < HEIGHT as i32 {
        return Some((grid_x as usize, grid_y as usize));
    } else {
        return None;
    }
}

fn check_mailbox(
    mailbox: &Receiver<PlayerManagerMessage>,
    status_data: &mut StatusData,
) -> Option<bool> {
    //read all messages in mailbox
    while let Ok(msg) = mailbox.try_recv() {
        match msg {
            PlayerManagerMessage::Quit => return Some(true),

            // TODO: Handle more message types
            PlayerManagerMessage::InventoryStatusResult(res) => {
                status_data.inventory_string = res;
            }
            PlayerManagerMessage::CurrentTaskResult(res) => {
                status_data.task = res;
            }
            _ => {}
        }
    }

    return Some(false);
}

fn get_inputs(
    world_manager: &AID<WorldManagerMessage>,
    camera: &mut Camera,
    select: &mut Selection,
    select_2: &mut Selection,
    old_world: &RawWorldArray,
    time_to_wait: u64,
    frame: &Rect,
    show_build_menu: &mut bool,
    menu_buttons: &Vec<MenuButtonWidget>,
    is_in_place_mode: &mut bool,
) -> Option<InputResult> {
    let mut key_event: Option<KeyEvent> = None;
    let mut mouse_event: Option<MouseEvent> = None;

    let mut input: Input = Input {
        mouse_pos: None,
        mouse_click: MouseClick::None,
        key: None,
    };

    // 50 ms looks better with animations
    let poll_start = Instant::now();
    let get_time_left = || {Duration::from_millis(time_to_wait).saturating_sub(poll_start.elapsed())};
    while poll(get_time_left()).ok()? {
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
                match event.kind {
                    MouseEventKind::Moved => {
                        input.mouse_pos = Some((event.column, event.row));
                        // don't re-render everything just because of a mouse move
                        continue;
                    }
                    _ => (),
                }
                mouse_event = Some(event);
            }
            _ => {}
        }
        break;
    }

    parse_input_keyboard(
        &mut input, 
        &key_event, 
        camera, 
        select, 
        select_2, 
        &old_world, 
        show_build_menu,
        world_manager
    );
    
    parse_input_mouse(
        &mut input,
        &mouse_event,
        frame,
        *camera,
        select,
        old_world,
        world_manager,
        show_build_menu,
        menu_buttons,
        is_in_place_mode,
    );

    return Some(InputResult::Continue);
}

fn parse_input_keyboard(
    input: &mut Input,
    event_opt: &Option<KeyEvent>,
    camera: &mut Camera,
    select: &mut Selection,
    select_2: &mut Selection,
    old_world: &RawWorldArray,
    show_build_menu: &mut bool,
    world_manager: &AID<WorldManagerMessage>,
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
        KeyCode::Esc => {
            *select = Selection::Empty;
            *select_2 = Selection::Empty;
        }
        KeyCode::Char('n') => {
            *select = get_next_entity(&old_world, select.clone());
        }
        KeyCode::Char('m') => {
            if *select != Selection::Empty {
                if let Some(new_camera) = get_worker_camera(&old_world, &select) {
                    *camera = new_camera;
                }
            }
        }
        KeyCode::Tab => {
            *show_build_menu = !*show_build_menu;
        }
        KeyCode::Char('g') => {
            if let Selection::Building(x0, y0, aid0) = select_2 {
                if let Selection::Building(x1, y1, aid1) = select {
                    if aid0 == aid1 {
                        // don't make a path to itself
                    } else {
                        let _ = world_manager.send(WorldManagerMessage::CreatePath(
                            ItemId::from("mutexium"),
                            (*x0, *y0),
                            (*x1, *y1),
                        ));
                        
                        *select_2 = Selection::Empty;
                    }
                } else {
                    *select_2 = Selection::Empty;
                }
            } else {
                if let Selection::Building(x, y, aid) = select {
                    *select_2 = select.clone();
                } else {
                    *select_2 = Selection::Empty;
                }
            }
        }
        KeyCode::Char('1') => {
            if let Selection::Dummy(x, y) = select {
                let _ = world_manager.send(WorldManagerMessage::RemoveDummy((*x, *y)));
                let _ = world_manager.send(WorldManagerMessage::SpawnWorker((*x, *y), WorkerId::from("worker")));
                *select = Selection::Pending(*x, *y);
            }
        }
        KeyCode::Char('2') => {
            if let Selection::Dummy(x, y) = select {
                let _ = world_manager.send(WorldManagerMessage::RemoveDummy((*x, *y)));
                let _ = world_manager.send(WorldManagerMessage::SpawnBuilding(
                    (*x, *y),
                    BuildingId::from("factory"),
                    Task::Idle,
                ));
                *select = Selection::Pending(*x, *y);
            }
        }
        KeyCode::Char('3') => {
            if let Selection::Dummy(x, y) = select {
                let _ = world_manager.send(WorldManagerMessage::RemoveDummy((*x, *y)));
                let _ = world_manager.send(WorldManagerMessage::SpawnBuilding(
                    (*x, *y),
                    BuildingId::from("factory"),
                    Task::Produce(RecipeId::from("recipe_mutexium")),
                ));
                *select = Selection::Pending(*x, *y);
            }
        }
        KeyCode::Char('4') => {
            if let Selection::Dummy(x, y) = select {
                let _ = world_manager.send(WorldManagerMessage::RemoveDummy((*x, *y)));
                let _ = world_manager.send(WorldManagerMessage::SpawnBuilding(
                    (*x, *y),
                    BuildingId::from("factory"),
                    Task::Produce(RecipeId::from("recipe_mutexium_double")),
                ));
                *select = Selection::Pending(*x, *y);
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
    select: &mut Selection,
    old_world: &RawWorldArray,
    world_manager: &AID<WorldManagerMessage>,
    menu_open: &bool,
    menu_buttons: &Vec<MenuButtonWidget>,
    is_in_place_mode: &mut bool,
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
            let (x, y) = (event.column, event.row);
            input.mouse_pos = Some((x, y));
            input.mouse_click = MouseClick::Left;
            
            if *is_in_place_mode {
                if let Some((x, y)) = mouse_to_grid_pos((x, y), world_area, camera) {
                    let tile = &old_world[y][x];
                    match tile {
                        Tile::Empty => {
                            let _ = world_manager.send(WorldManagerMessage::SpawnDummy((x, y)));
                            *select = Selection::Pending(x, y);
                        }
                        Tile::Dummy => {
                            let _ = world_manager.send(WorldManagerMessage::RemoveDummy((x, y)));
                            *select = Selection::Empty;
                        }
                        _ => (),
                    }

                    *is_in_place_mode = false;
                }
            }

            else if *menu_open && event.column < 40 {
                let worker_button = &menu_buttons[0];
                let building_button = &menu_buttons[1];
                if worker_button.last_area.contains(Position::new(x, y)) || 
                    building_button.last_area.contains(Position::new(event.column, event.row)) 
                {
                    *is_in_place_mode = true;
                } 
            }

            else {
                if let Some((x, y)) = mouse_to_grid_pos((event.column, event.row), world_area, camera) {
                    let tile = &old_world[y][x];
                    if let Tile::Building(aid, _) = tile {
                        *select = Selection::Building(x, y, aid.clone());
                    } else if let Tile::Worker(aid, _) = tile {
                        *select = Selection::Worker(x, y, aid.clone());
                    } else if let Tile::Dummy = tile {
                        *select = Selection::Dummy(x, y);
                    } else {
                        *select = Selection::Empty;
                    }
                } else {
                    *select = Selection::Empty;
                }
            }
        }
        // MouseEventKind::Down(MouseButton::Right) => {
        //     input.mouse_pos = Some((event.column, event.row));
        //     input.mouse_click = MouseClick::Right;
            
        //     if let Some((x, y)) = mouse_to_grid_pos((event.column, event.row), world_area, camera) {
        //         let tile = &old_world[y][x];
        //         match tile {
        //             Tile::Empty => {
        //                 let _ = world_manager.send(WorldManagerMessage::SpawnDummy((x, y)));
        //                 *select = Selection::Pending(x, y);
        //             }
        //             Tile::Dummy => {
        //                 let _ = world_manager.send(WorldManagerMessage::RemoveDummy((x, y)));
        //                 *select = Selection::Empty;
        //             }
        //             _ => (),
        //         }
        //     }
        // }
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
    status_data: &StatusData,
    show_build_menu: bool,
    assets: Arc<Assets>,
    select: &Selection,
    select_2: &Selection,
    menu_buttons: &mut Vec<MenuButtonWidget>
) {
    let world_area = frame.area();

    render_world_in_area(frame, &world_area, &old_world_array, &world_array, camera);

    let margin: u16 = 2;
    let selected_width: u16 = (world_area.width / 3 - margin) / TILE_SIZE.0 * TILE_SIZE.0 + margin;
    let menu_width = 40;

    let horizontal_split = Layout::horizontal([
        menu_width, 
        world_area.width - selected_width - menu_width, 
        selected_width
    ])
    .flex(ratatui::layout::Flex::Start)
    .split(frame.area());

    match select {
        Selection::Empty => (),
        _ => {
            render_selected_info(
                frame,
                select,
                &world_area,
                world_array,
                old_world_array,
                status_data,
                horizontal_split[2],
                assets,
            );
        }
    }
    
    if let Selection::Building(x, y, _) = select_2 {
        frame.render_widget(
            Block::new()
                .borders(Borders::ALL)
                .title("─ Go to ")
                .merge_borders(MergeStrategy::Replace),
            Rect::new(0, 0, 21, 4),
        );
        frame.render_widget(
            Paragraph::new("Select destination,\nthen press G"),
            Rect::new(1, 1, 21, 2),
        );
    }

    if show_build_menu {
        render_build_menu(frame, horizontal_split[0], menu_buttons);
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
    select: &Selection,
    world_area: &Rect,
    world_array: &RawWorldArray,
    old_world_array: &RawWorldArray,
    status_data: &StatusData,
    selected_window_slice: Rect,
    assets: Arc<Assets>,
) {
    let height = world_area.height / 2;
    let m: u16 = 2; // margin: 2 x border, which doubles as 2 x space for animation

    if  height > m {
        let height = (height - m) / TILE_SIZE.1 * TILE_SIZE.1 + m;

        let selected_layout = Layout::vertical([
            Constraint::Length(frame.area().height - height),
            Constraint::Length(height),
        ])
        .spacing(Spacing::Overlap(1))
        .split(selected_window_slice);

        frame.render_widget(Clear, selected_layout[0]);
        frame.render_widget(Clear, selected_layout[1]);

        if let Some(pov_camera) = get_worker_camera(&world_array, select) {
            render_pov(frame, pov_camera, &selected_layout, world_array, old_world_array);
        }

        let pov_title = match select {
            Selection::Empty => "<error>",
            Selection::Pending(_, _) => "<pending...>",
            Selection::Dummy(_, _) => "dummy",
            Selection::Worker(_, _, _) => "worker",
            Selection::Building(_, _, _) => "building",
        };
        // render this last so it covers any part of the world sticking out
        frame.render_widget(
            Block::new()
                .borders(Borders::ALL)
                .title(format!("─ POV: you're a {} ", pov_title))
                .merge_borders(MergeStrategy::Replace),
            selected_layout[0],
        );
        
        frame.render_widget(
            Block::new()
                .borders(Borders::ALL)
                .title("─ Status ")
                .merge_borders(MergeStrategy::Exact),
            selected_layout[1],
        );

        match select {
            Selection::Empty => {},
            Selection::Pending(_, _) => {},
            Selection::Worker(_, _, _) => {
                render_status(frame, selected_layout[1], status_data, assets);
            },
            Selection::Building(_, _, _) => {
                render_status(frame, selected_layout[1], status_data, assets);
            },
            Selection::Dummy(sx, sy) => {
                let inner = selected_layout[1].inner(Margin::new(2, 2));
                frame.render_widget(
                    Paragraph::new(concat!(
                        "Create:\n",
                        "1 - Worker\n",
                        "2 - Building (empty)\n",
                        "3 - Building that produces mutexium\n",
                        "4 - Building that uses mutexium\n",
                    )),
                    inner,
                );
            },
        };
    }
}

fn render_pov(
    frame: &mut Frame,
    pov_camera: Camera,
    layout: &Rc<[Rect]>,
    world_array: &RawWorldArray,
    old_world_array: &RawWorldArray,
) {
    let pov_area_inner = layout[0].inner(Margin::new(1, 1));
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
    area_rect: Rect,
    status_data: &StatusData,
    assets: Arc<Assets>,
) {
    fn parse_task(task: &Task, assets: Arc<Assets>) -> String {
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
            Task::Produce(recipe) => {
                if let Some(recipe_asset) = assets.recipes.get(recipe) {
                    let mut output_string: String = String::from("nothing");

                    if recipe_asset.outputs.len() > 0 {
                        output_string = String::new();

                        for output in recipe_asset.outputs.clone() {
                            output_string.push_str(&output.id.to_string());
                        }
                    } 

                    let mut input_string: String = String::from("nothing");

                    if recipe_asset.inputs.len() > 0 {
                        input_string = String::new();

                        for input in recipe_asset.inputs.clone() {
                            input_string.push_str(&input.id.to_string());
                        }
                    } 

                    parsed_task.push_str(format!(
                        "Producing {0} from {1}", 
                        output_string, 
                        input_string
                    ).as_str());
                }
            }
        }

        return parsed_task;
    }

    let sub_layout =
        Layout::vertical([Constraint::Length(7), Constraint::Percentage(75)]).split(area_rect);

    // Render Task
    let mut task_text: String = format!("Fetching Task..."); 

    if let Some(task) = &status_data.task {
        task_text = parse_task(task, assets);
    }

    frame.render_widget(            
        Paragraph::new(task_text)
            .block(Block::new().padding(Padding::uniform(2)))
            .alignment(Alignment::Center),
            sub_layout[0]
    );

    // Render Inventory
    let mut inventory_text: String = format!("Fetching Data...");

    if let Some(inventory_string) = &status_data.inventory_string {
        inventory_text = inventory_string.clone();
    }

    frame.render_widget(
        Paragraph::new(inventory_text)
            .block(Block::new().padding(Padding::uniform(2)))
            .alignment(Alignment::Center),
        sub_layout[1],
    );

    // Render Status Box
    frame.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .title("─ Status ")
            .merge_borders(MergeStrategy::Exact),
        area_rect,
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
                Tile::Dummy => {
                    let square = Paragraph::new("┌─┐\n└─┘").yellow();
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

fn render_build_menu(frame: &mut Frame, build_menu_slice: Rect, menu_buttons: &mut Vec<MenuButtonWidget>) {
    frame.render_widget(Clear, build_menu_slice);

    let margin = 2;
    let items = Layout::new(Direction::Vertical, [
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(10),
    ]).flex(Flex::Start)
        .split(build_menu_slice.inner(Margin::new(margin, margin))); 


    let build_menu_box = Block::new().borders(Borders::ALL).title("─ Build Menu ");

    let worker_button = &mut menu_buttons[0];

    worker_button.render(
        items[0].centered(
            Constraint::Percentage(100), 
            Constraint::Percentage(100), 
        ),
        frame.buffer_mut()
    );

    frame.render_widget(
        Paragraph::new("100 Mutexium")
            .block(Block::bordered().title("─ Worker ")), 
        items[0].centered(
            Constraint::Percentage(100), 
            Constraint::Percentage(100)
        ));

    let building_button = &mut menu_buttons[1];
    building_button.render(
        items[1].centered(
            Constraint::Percentage(100), 
            Constraint::Percentage(100), 
        ),
        frame.buffer_mut()
    );

    frame.render_widget(
        Paragraph::new("200 Semaphorite")
            .block(Block::bordered().title("─ Factory ")), 
        items[1].centered(
            Constraint::Percentage(100), 
            Constraint::Percentage(100)
        ));

    frame.render_widget(
        build_menu_box
        , build_menu_slice);
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