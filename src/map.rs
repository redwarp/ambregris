use crate::{
    components::*,
    spawner::{self, MonsterType},
};

use legion::component;
use legion::IntoQuery;
use legion::World;
use rand::Rng;
use rand::{rngs::StdRng, SeedableRng};
use torchbearer::fov::field_of_view;
use torchbearer::Map as FieldOfVisionMap;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 40;
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOM: i32 = 30;
const MAX_ROOM_MONSTERS: i32 = 3;
const MAX_ROOM_ITEMS: i32 = 3;

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl From<Position> for (i32, i32) {
    fn from(Position { x, y }: Position) -> Self {
        (x, y)
    }
}

impl From<(i32, i32)> for Position {
    fn from((x, y): (i32, i32)) -> Self {
        Position { x, y }
    }
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Position { x, y }
    }

    pub fn distance_to(&self, position: Position) -> f32 {
        ((self.x - position.x).pow(2) as f32 + (self.y - position.y).pow(2) as f32).sqrt()
    }

    pub fn set_position(&mut self, position: &Position) {
        self.x = position.x;
        self.y = position.y;
    }
}

#[derive(Clone)]
pub struct Tile {
    pub blocking: bool,
    pub block_sight: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocking: false,
            block_sight: false,
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocking: true,
            block_sight: true,
        }
    }
}

#[derive(Debug)]
struct Rect {
    x1: i32,
    x2: i32,
    y1: i32,
    y2: i32,
}

impl Rect {
    fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Rect {
            x1: x,
            y1: y,
            x2: x + width,
            y2: y + height,
        }
    }

    fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;

        (center_x, center_y)
    }

    fn intersects_with(&self, other: &Rect) -> bool {
        (self.x1 <= other.x2)
            && (self.x2 >= other.x1)
            && (self.y1 <= other.y2)
            && (self.y2 >= other.y1)
    }
}
pub struct Map {
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<Tile>,
    pub explored_tiles: Vec<bool>,
    pub blocked: Vec<bool>,
    pub player_fov: Vec<(i32, i32)>,
    pub depth: i32,
}

impl Map {
    pub fn is_blocked(&self, position: Position) -> bool {
        self.blocked[self.index(position)]
    }

    pub fn set_blocked(&mut self, position: Position, is_blocked: bool) {
        let index = self.index(position);
        self.blocked[index] = is_blocked;
    }

    pub fn index(&self, position: Position) -> usize {
        let (x, y) = position.into();
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            0
        } else {
            (x + y * self.width) as usize
        }
    }

    pub fn is_in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.width && y >= 0 && y < self.height
    }

    pub fn is_in_player_fov(&self, x: i32, y: i32) -> bool {
        self.player_fov.contains(&(x, y))
    }

    pub fn calculate_player_fov(&mut self, x: i32, y: i32, radius: i32) {
        self.player_fov = field_of_view(self, (x, y), radius);
    }
}

impl FieldOfVisionMap for Map {
    fn dimensions(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    fn is_transparent(&self, x: i32, y: i32) -> bool {
        !self.tiles[(x + y * self.width) as usize].block_sight
    }

    fn is_walkable(&self, x: i32, y: i32) -> bool {
        !self.tiles[(x + y * self.width) as usize].blocking
    }
}

pub fn make_map(world: &mut World, level: i32) -> Map {
    let mut rng = StdRng::seed_from_u64(42 + level as u64);
    let map_size = MAP_HEIGHT as usize * MAP_WIDTH as usize;
    let mut map = Map {
        width: MAP_WIDTH,
        height: MAP_HEIGHT,
        tiles: vec![Tile::wall(); map_size],
        explored_tiles: vec![false; map_size],
        blocked: vec![false; map_size],
        player_fov: vec![],
        depth: level,
    };

    let mut rooms: Vec<Rect> = vec![];

    for _ in 0..MAX_ROOM {
        let width = rng.gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let height = rng.gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let x = rng.gen_range(0, MAP_WIDTH - width);
        let y = rng.gen_range(0, MAP_HEIGHT - height);

        let new_room = Rect::new(x, y, width, height);
        let failed = rooms.iter().any(|other| new_room.intersects_with(other));

        if !failed {
            rooms.push(new_room);
        }
    }

    for (index, new_room) in rooms.iter().enumerate() {
        create_room(&new_room, &mut map);

        let (new_x, new_y) = new_room.center();
        if index == 0 {
            let mut query = <&mut Position>::query().filter(component::<Player>());
            for coordinates in query.iter_mut(world) {
                coordinates.x = new_x;
                coordinates.y = new_y;
            }
        } else {
            let (prev_x, prev_y) = rooms[index - 1].center();

            if rng.gen::<bool>() {
                create_horizontal_tunnel(prev_x, new_x, prev_y, &mut map);
                create_vertical_tunnel(prev_y, new_y, new_x, &mut map);
            } else {
                create_vertical_tunnel(prev_y, new_y, prev_x, &mut map);
                create_horizontal_tunnel(prev_x, new_x, new_y, &mut map)
            }
        }

        if index == rooms.len() - 1 {
            // Last room, let's place the exit.
            place_stairs(world, &mut map, &new_room);
            println!("Placing stairs in room {:?}", new_room);
        }
        if !rooms.is_empty() {
            // Let's be cool and not put any monsters in the room.
            place_objects(world, &mut rng, &map, &new_room);
        }
    }

    map
}

fn create_room(room: &Rect, map: &mut Map) {
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map.tiles[x as usize + y as usize * map.width as usize] = Tile::empty();
        }
    }
}

fn create_horizontal_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    for x in x1.min(x2)..(x1.max(x2) + 1) {
        map.tiles[x as usize + y as usize * map.width as usize] = Tile::empty();
    }
}
fn create_vertical_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    for y in y1.min(y2)..(y1.max(y2) + 1) {
        map.tiles[x as usize + y as usize * map.width as usize] = Tile::empty();
    }
}

fn place_objects(world: &mut World, rng: &mut StdRng, map: &Map, room: &Rect) {
    let num_monsters = rng.gen_range(0, MAX_ROOM_MONSTERS);

    for _ in 0..num_monsters {
        let x = rng.gen_range(room.x1 + 1, room.x2);
        let y = rng.gen_range(room.y1 + 1, room.y2);

        if !map.is_blocked((x, y).into()) {
            let monster_type = if rng.gen::<f32>() < 0.8 {
                MonsterType::Orc
            } else {
                MonsterType::Troll
            };
            spawner::monster(world, monster_type, x, y);
        }
    }

    let num_items = rng.gen_range(0, MAX_ROOM_ITEMS);
    for _ in 0..num_items {
        let x = rng.gen_range(room.x1 + 1, room.x2);
        let y = rng.gen_range(room.y1 + 1, room.y2);

        if !map.is_blocked((x, y).into()) {
            match rng.gen::<f32>() {
                r if r < 0.33 => {
                    spawner::potion(world, x, y);
                }
                r if r < 0.66 => {
                    spawner::scroll_of_lightning_bolt(world, x, y);
                }
                _ => spawner::scroll_of_fireball(world, x, y),
            }
        }
    }
}

fn place_stairs(world: &mut World, map: &mut Map, room: &Rect) {
    let (x, y) = room.center();
    spawner::stairs(world, x, y);
    map.set_blocked((x, y).into(), true);
}
