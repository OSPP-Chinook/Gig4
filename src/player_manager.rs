use rand::{
    RngExt, SeedableRng, rngs::ChaCha8Rng
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
    cmp::Ordering, io::stdout, panic, rc::Rc, sync::{
        Arc, 
        mpsc::Receiver, 
    }, time::{
        Duration, Instant
    }
};

use crate::{
    EntityMessage, aid::{
        AID, AIDHandle
    }, assets::{
        Assets, BuildingId, ItemId, ItemStack, RecipeAsset, WorkerId
    }, game_manager::GameManagerMessage, task_manager::Task, world_manager::{ 
        HEIGHT, Pos, RawWorldArray, Tile, WIDTH, WorldGrid, WorldManagerMessage
    }, zombie 
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

struct Ui {
    current_recipes: Vec<RecipeAsset>,
    main_layout: Rc<[Rect]>, //index 0 is world rect, index 1 is sidebar rect
    sidebar_layout: Rc<[Rect]>,
    button_layout: Rc<[Rect]>,
    buttons: Vec<(String, fn(&Ui, usize))>,
    selected_path_start: Selection,
    selected_entity: Selection,
    status_data: StatusData,
    menu_buttons: Vec<MenuButtonWidget>,
    show_build_menu: bool,
    build_mode: BuildMode,
}

impl Ui {
    fn new(screen: &Rect) -> Self {
        let layout = get_main_layout(screen);
        let sidebar = get_sidebar_layout(&layout[1]);
        let button = get_button_layout(&sidebar[0]);
        return Ui {
            selected_path_start: Selection::Empty,
            selected_entity: Selection::Empty,
            main_layout: layout,
            sidebar_layout: sidebar,
            buttons: vec![],
            button_layout: button,
            current_recipes: vec![],
            status_data: StatusData { inventory_string: None, task: None },
            menu_buttons: vec![
            MenuButtonWidget { last_area: Rect::new(0, 0, 0, 0) }, // build worker button
            MenuButtonWidget { last_area: Rect::new(0, 0, 0, 0) }, // build factory button
            ],
            show_build_menu: false,
            build_mode: BuildMode { active: false, kind: Placable::None },

        };
    }

    fn create_button(&mut self, title: String, on_click: fn(&Ui, usize)) {
        self.buttons.push((title, on_click));
    }

}

// Takes the screen rect and divides it into world rect and sidebar rect
fn get_main_layout(screen: &Rect) -> Rc<[Rect]> {
    let width = screen.width / 3;
    let m = 2; // margin: 2 x border, which doubles as 2 x space for animation

    // pov_area encloses a whole number of tiles
    let width = (width - m) / TILE_SIZE.0 * TILE_SIZE.0 + m;
    return Layout::horizontal([Constraint::Fill(1), Constraint::Length(width)])
        .flex(ratatui::layout::Flex::End)
        .split(*screen);
}
// Takes the screen rect and divides it into world rect and sidebar rect
fn get_sidebar_layout(sidebar: &Rect) -> Rc<[Rect]> {
    let height = sidebar.height / 3;
    let m = 2; // margin: 2 x border, which doubles as 2 x space for animation

    // pov_area encloses a whole number of tiles
    let height = (height - m) / TILE_SIZE.1 * TILE_SIZE.1 + m;

    return Layout::vertical([
        Constraint::Length(sidebar.height - height),
        Constraint::Length(height),
    ])
    .spacing(Spacing::Overlap(1))
    .split(*sidebar);
}

fn get_button_layout(status: &Rect) -> Rc<[Rect]> {
    return Layout::vertical([
        Constraint::Fill(1),
        Constraint::Percentage(40),
        Constraint::Percentage(5),
    ])
    .split(*status);
}

fn is_in_rect((x, y): (u16, u16), rect: &Rect) -> bool {
    if x < rect.x || y < rect.y {
        return false;
    }
    return x - rect.x < rect.width && y - rect.y < rect.height;
}

fn is_in_layout_rect((x, y): (u16, u16), layout: Rc<[Rect]>, index: usize) -> bool {
    if index >= layout.len() {
        return false;
    }
    return x - layout[index].x < layout[index].width && y - layout[index].y < layout[index].height;
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
    Worker(usize, usize, AID<EntityMessage>, WorkerId),
    Building(usize, usize, AID<EntityMessage>, BuildingId),
}

#[derive(Clone)]
enum Placable {
    None,
    Worker(WorkerId),
    Building(BuildingId),
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

struct BuildMode {
    active: bool,
    kind: Placable,
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
                Tile::Worker(aid, id) => {
                    if found {
                        return Selection::Worker(x, y, aid.clone(), id.clone());
                    }
                    if let Selection::Worker(_, _, sel_aid, _) = &select {
                        if aid == sel_aid {
                            found = true;
                        }
                    }
                }
                Tile::Building(aid, id) => {
                    if found {
                        return Selection::Building(x, y, aid.clone(), id.clone());
                    }
                    if let Selection::Building(_, _, sel_aid, _) = &select {
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
                Tile::Worker(aid, id) => {
                    if let Selection::Worker(_, _, sel_aid, _) = &select {
                        if aid == sel_aid {
                            return Selection::Worker(x, y, aid.clone(), id.clone());
                        }
                    }
                    if let Selection::Pending(sx, sy) = select {
                        if x == sx && y == sy {
                            return Selection::Worker(x, y, aid.clone(), id.clone());
                        }
                    }
                }
                Tile::Building(aid, id) => {
                    if let Selection::Building(_, _, sel_aid, _) = &select {
                        if aid == sel_aid {
                            return Selection::Building(x, y, aid.clone(), id.clone());
                        }
                    }
                    if let Selection::Pending(sx, sy) = select {
                        if x == sx && y == sy {
                            return Selection::Building(x, y, aid.clone(), id.clone());
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

fn get_entity_camera(select: &Selection) -> Option<Camera> {
    match select {
        Selection::Empty => {None}
        Selection::Worker(x, y, _, _) => {Some(Camera(*x as i32, *y as i32))}
        Selection::Building(x, y, _, _) => {Some(Camera(*x as i32, *y as i32))}
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
        let _ = execute!(stdout(), EnableMouseCapture);
        
        let _ = panic::catch_unwind(|| io_loop(aid, &mailbox, world, grid, assets));
        let _ = execute!(stdout(), DisableMouseCapture);

        let _ = game.send(GameManagerMessage::Quit);
        drop(game);
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

        let mut old_world = get_copy_of_world(&world_array);
        let mut time_to_wait = 0;

        // For Status information
        let mut ui = Ui::new(&terminal.get_frame().area());
        
        
        let mut fps: f32 = 0.;
        let mut second_counter = Instant::now();
        let mut frames = 0;
        
        let mut paused = false;

        loop {
            if let Some(true) = check_mailbox(&mailbox, &mut ui) {
                break Ok(());
            }

            if let Selection::Building(_, _, _, id) = ui.selected_entity.clone()
                && ui.current_recipes.is_empty()
            {
                if let Some(asset) = assets
                    .buildings
                    .get(&BuildingId::from(id))
                {
                    for id in &asset.recipes {
                        if let Some(recipe_asset) = assets.recipes.get(id) {
                            ui.current_recipes.push(recipe_asset.clone());
                        }
                        ui.create_button(id.to_string(), |ui, i| {
                            if let Selection::Building(_, _, aid, _) = &ui.selected_entity {
                                _ = aid.send(EntityMessage::TaskResponse(Ok(Task::Produce(
                                    ui.current_recipes[i].id.clone(),
                                ))));
                            }
                        });
                    }
                }
            }
            ui.main_layout = get_main_layout(&terminal.get_frame().area());

            if let Some(val) = get_inputs(
                &world,
                &mut camera,
                &mut ui,
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

            if let Selection::Worker(_, _, sel_aid, _) = ui.selected_entity.clone() {
                _ = sel_aid.send(EntityMessage::FetchInventoryStatus(aid.clone()));
                _ = sel_aid.send(EntityMessage::FetchCurrentTask(aid.clone()));
            }
            if let Selection::Building(_, _, sel_aid, _) = ui.selected_entity.clone() {
                _ = sel_aid.send(EntityMessage::FetchInventoryStatus(aid.clone()));
                _ = sel_aid.send(EntityMessage::FetchCurrentTask(aid.clone()));
            }

            let time_0 = Instant::now();
            let new_world = get_copy_of_world(&world_array);
            let time_1 = Instant::now();
            ui.selected_entity = update_selection(&new_world, ui.selected_entity);
            terminal.draw(|frame| {
                render(
                    frame,
                    &old_world,
                    &new_world,
                    camera,
                    (time_0, time_1, fps),
                    &mut ui,
                    assets.clone()
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

fn check_mailbox(mailbox: &Receiver<PlayerManagerMessage>, ui: &mut Ui) -> Option<bool> {
    //read all messages in mailbox
    while let Ok(msg) = mailbox.try_recv() {
        match msg {
            PlayerManagerMessage::Quit => return Some(true),

            // TODO: Handle more message types
            PlayerManagerMessage::InventoryStatusResult(res) => {
                ui.status_data.inventory_string = res;
            }
            PlayerManagerMessage::CurrentTaskResult(res) => {
                ui.status_data.task = res;
            }
            _ => {}
        }
    }

    return Some(false);
}

fn get_inputs(
    world_manager: &AID<WorldManagerMessage>,
    camera: &mut Camera,
    ui: &mut Ui,
    old_world: &RawWorldArray,
    time_to_wait: u64,
    frame: &Rect,
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
        ui,
        &old_world, 
        world_manager
    );
    
    parse_input_mouse(
        &mut input,
        &mouse_event,
        frame,
        *camera,
        ui,
        old_world,
        world_manager,
    );

    return Some(InputResult::Continue);
}

fn parse_input_keyboard(
    input: &mut Input,
    event_opt: &Option<KeyEvent>,
    camera: &mut Camera,
    ui: &mut Ui,
    old_world: &RawWorldArray,
    world_manager: &AID<WorldManagerMessage>
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
            ui.selected_entity = Selection::Empty;
            ui.selected_path_start = Selection::Empty;
        }
        KeyCode::Char('n') => {
            ui.selected_entity = get_next_entity(&old_world, ui.selected_entity.clone());
        }
        KeyCode::Char('m') => {
            if ui.selected_entity != Selection::Empty {
                if let Some(new_camera) = get_entity_camera(&ui.selected_entity) {
                    *camera = new_camera;
                }
            }
        }
        KeyCode::Tab => {
            ui.show_build_menu = !ui.show_build_menu;
        }
        KeyCode::Char('g') => {
            if let Selection::Building(x0, y0, aid0, _) = &ui.selected_path_start {
                if let Selection::Building(x1, y1, aid1, _) = &ui.selected_entity {
                    if aid0 == aid1 {
                        // don't make a path to itself
                    } else {
                        let _ = world_manager.send(WorldManagerMessage::CreatePath(
                            ItemId::from("mutexium"),
                            (*x0, *y0),
                            (*x1, *y1),
                        ));
                        
                        ui.selected_path_start = Selection::Empty;
                    }
                } else {
                    ui.selected_path_start = Selection::Empty;
                }
            } else {
                if let Selection::Building(_, _, _, _) = &ui.selected_entity {
                    ui.selected_path_start = ui.selected_entity.clone();
                } else {
                    ui.selected_path_start = Selection::Empty;
                }
            }
        }
        KeyCode::Char('1') => {
            if let Selection::Dummy(x, y) = &ui.selected_entity {
                let _ = world_manager.send(WorldManagerMessage::RemoveDummy((*x, *y)));
                let _ = world_manager.send(WorldManagerMessage::SpawnWorker((*x, *y), WorkerId::from("worker")));
                ui.selected_entity = Selection::Pending(*x, *y);
            }
        }
        KeyCode::Char('2') => {
            if let Selection::Dummy(x, y) = &ui.selected_entity {
                let _ = world_manager.send(WorldManagerMessage::RemoveDummy((*x, *y)));
                let _ = world_manager.send(WorldManagerMessage::SpawnBuilding(
                    (*x, *y),
                    BuildingId::from("factory"),
                ));
                ui.selected_entity = Selection::Pending(*x, *y);
            }
        }
        KeyCode::Char('3') => {
            if let Selection::Dummy(x, y) = &ui.selected_entity {
                let _ = world_manager.send(WorldManagerMessage::RemoveDummy((*x, *y)));
                let _ = world_manager.send(WorldManagerMessage::SpawnBuilding(
                    (*x, *y),
                    BuildingId::from("factory"),
                ));
                ui.selected_entity = Selection::Pending(*x, *y);
            }
        }
        KeyCode::Char('4') => {
            if let Selection::Dummy(x, y) = &ui.selected_entity {
                let _ = world_manager.send(WorldManagerMessage::RemoveDummy((*x, *y)));
                let _ = world_manager.send(WorldManagerMessage::SpawnBuilding(
                    (*x, *y),
                    BuildingId::from("factory"),
                ));
                ui.selected_entity = Selection::Pending(*x, *y);
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
    ui: &mut Ui,
    old_world: &RawWorldArray,
    world_manager: &AID<WorldManagerMessage>,
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
            
            if ui.build_mode.active {
                if let Some((x, y)) = mouse_to_grid_pos((x, y), world_area, camera) {
                    let tile = &old_world[y][x];
                    match tile {
                        Tile::Empty => {
                            match ui.build_mode.kind.clone() {
                                Placable::Worker(id) => {
                                    _ = world_manager.send(WorldManagerMessage::SpawnWorker((x, y), id));
                                }

                                Placable::Building(id) => {
                                    _ = world_manager.send(WorldManagerMessage::SpawnBuilding((x, y), id));
                                }

                                Placable::None => { 
                                    // Shouldnt happen 
                                }
                            }

                            let _ = world_manager.send(WorldManagerMessage::SpawnDummy((x, y)));
                            ui.selected_entity = Selection::Pending(x, y);
                        }
                        Tile::Dummy => {
                            let _ = world_manager.send(WorldManagerMessage::RemoveDummy((x, y)));
                            ui.selected_entity = Selection::Empty;
                        }
                        _ => (),
                    }

                    ui.build_mode.active = false;
                }
            }

            else if ui.show_build_menu && event.column < 40 {
                let worker_button = &ui.menu_buttons[0];
                let building_button = &ui.menu_buttons[1];
                if worker_button.last_area.contains(Position { x, y }) 
                {
                    ui.build_mode.kind = Placable::Worker(WorkerId::from("worker"));
                    ui.build_mode.active = true;
                }

                else if building_button.last_area.contains(Position { x, y }){
                    ui.build_mode.kind = Placable::Building(BuildingId::from("factory"));
                    ui.build_mode.active = true;
                }
            }
            else if ui.selected_entity == Selection::Empty
                || is_in_layout_rect(input.mouse_pos.unwrap(), ui.main_layout.clone(), 0)
            {
                if let Some((x, y)) = mouse_to_grid_pos((event.column, event.row), world_area, camera) {
                    ui.current_recipes.clear();
                    ui.buttons.clear();
                    let tile = &old_world[y][x];
                    if let Tile::Building(aid, id) = tile {
                        ui.selected_entity = Selection::Building(x, y, aid.clone(), id.clone());
                    } else if let Tile::Worker(aid, id) = tile {
                        ui.selected_entity = Selection::Worker(x, y, aid.clone(), id.clone());
                    } else if let Tile::Dummy = tile {
                        ui.selected_entity = Selection::Dummy(x, y);
                    } else {
                        ui.selected_entity = Selection::Empty;
                    }
                }
            }

            
            
            else {
                let mut clicked = None;
                let mut index = 0;
                if is_in_rect((event.column, event.row), &ui.button_layout[1]) {
                    index = ((event.row - ui.button_layout[1].y)
                        / (ui.button_layout[1].height / ui.buttons.len() as u16))
                        as usize;
                    if index >= ui.buttons.len() {
                        return;
                    }
                    clicked = Some(ui.buttons.swap_remove(index as usize));
                }

                if let Some((title, on_click)) = clicked {
                    on_click(ui, index);
                    ui.buttons.insert(index, (title, on_click));
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
    ui: &mut Ui,
    assets: Arc<Assets>,
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

    match &ui.selected_entity {
        Selection::Empty => (),
        _ => {
            render_selected_info(
                frame,
                &world_area,
                world_array,
                old_world_array,
                ui,
                assets
            );
        }
    }
    
    if let Selection::Building(_, _, _, _) = ui.selected_path_start {
        let mut x = 0;
        if ui.show_build_menu {
            x += 40;
        }
        frame.render_widget(
            Block::new()
                .borders(Borders::ALL)
                .title("─ Go to ")
                .merge_borders(MergeStrategy::Replace),
            Rect::new(x, 0, 21, 4),
        );
        frame.render_widget(
            Paragraph::new("Select destination,\nthen press G"),
            Rect::new(x + 1, 1, 21, 2),
        );
    }

    if ui.show_build_menu {
        render_build_menu(frame, horizontal_split[0], &mut ui.menu_buttons);
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

fn render_buttons(frame: &mut Frame, ui: &Ui) {
    let layout =
        Layout::vertical(vec![Constraint::Fill(1); ui.buttons.len()]).split(ui.button_layout[1]);
    for i in 0..ui.buttons.len() {
        frame.render_widget(
            Paragraph::new(get_button_text(ui, i)).block(Block::new().borders(Borders::ALL)),
            layout[i],
        );
    }
}

fn get_button_text(ui: &Ui, i: usize) -> String {
    let current = &ui.current_recipes[i];
    return format!(
        "Recipe {}\nInput: {}\nOutput: {}\nRecipe time: {}",
        i + 1,
        get_itemlist_text(&current.inputs),
        get_itemlist_text(&current.outputs),
        current.time
    );
}

fn get_itemlist_text(items: &Vec<ItemStack>) -> String {
    let mut string: String = String::from("");
    for i in 0..items.len() {
        string += &format!("{} ({})", items[i].id, items[i].count);
        if i < items.len() - 1 {
            string += ", ";
        }
    }
    return string;
}

fn render_selected_info(
    frame: &mut Frame,
    world_area: &Rect,
    world_array: &RawWorldArray,
    old_world_array: &RawWorldArray,
    ui: &Ui,
    assets:Arc<Assets>,     
) {
    let height = world_area.height / 2;
    let m: u16 = 2; // margin: 2 x border, which doubles as 2 x space for animation

    if  height > m {

        frame.render_widget(Clear, ui.sidebar_layout[0]);
        frame.render_widget(Clear, ui.sidebar_layout[1]);

        if let Some(pov_camera) = get_entity_camera(&ui.selected_entity) {
            render_pov(frame, pov_camera, &ui.sidebar_layout, world_array, old_world_array);
        }

        render_buttons(frame, ui);

        let pov_title = match ui.selected_entity {
            Selection::Empty => "<error>",
            Selection::Pending(_, _) => "<pending...>",
            Selection::Dummy(_, _) => "dummy",
            Selection::Worker(_, _, _, _) => "worker",
            Selection::Building(_, _, _, _) => "building",
        };
        // render this last so it covers any part of the world sticking out
        frame.render_widget(
            Block::new()
                .borders(Borders::ALL)
                .title(format!("─ POV: you're a {} ", pov_title))
                .merge_borders(MergeStrategy::Replace),
            ui.sidebar_layout[1],
        );
        
        frame.render_widget(
            Block::new()
                .borders(Borders::ALL)
                .title("─ Status ")
                .merge_borders(MergeStrategy::Exact),
            ui.sidebar_layout[0],
        );

        match &ui.selected_entity {
            Selection::Empty => {},
            Selection::Pending(_, _) => {},
            Selection::Worker(_, _, _, _) => {
                render_status(frame, ui.sidebar_layout[0], &ui.status_data, assets);
            },
            Selection::Building(_, _, _, _) => {
                render_status(frame, ui.sidebar_layout[0], &ui.status_data, assets);
            },
            Selection::Dummy(_, _) => {
                let inner = ui.sidebar_layout[1].inner(Margin::new(2, 2));
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