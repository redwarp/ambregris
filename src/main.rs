use crate::game::State;
use crate::map::Map;
use crate::pistonengine::Engine as PistonEngine;
use crate::resources::PlayerInfo;
use crate::tcodengine::Engine;

use field_of_vision::FovMap;
use legion::{Resources, World};
use rand::rngs::StdRng;
use rand::SeedableRng;
mod components;
mod game;
mod map;
mod pistonengine;
mod resources;
mod spawner;
mod systems;
mod tcodengine;

// actual size of the window
const SCREEN_WIDTH: u32 = 80;
const SCREEN_HEIGHT: u32 = 50;

fn main() {
    println!("Hello, world!");

    let mut rng = StdRng::seed_from_u64(42);
    let mut world = World::default();
    let mut resources = Resources::default();
    let player_entity = world.push(spawner::spawn_player(-1, -1));
    let map = crate::map::make_map(&mut world, &mut rng);
    let fov = make_fov(&map);
    resources.insert(map);
    resources.insert(fov);
    let mut state = State {
        world,
        resources,
        player_entity,
    };
    state.resources.insert(PlayerInfo {
        entity: player_entity,
        position: (-1, -1),
    });

    // let mut renderer = Engine::new(SCREEN_WIDTH, SCREEN_HEIGHT);

    // renderer.run(&mut state);

    let mut renderer = PistonEngine::new("Ambergris", SCREEN_WIDTH, SCREEN_HEIGHT);
    renderer.run(&mut state);
}

fn make_fov(map: &Map) -> FovMap {
    let mut fov = FovMap::new(map.width as isize, map.height as isize);

    for y in 0..map.height {
        for x in 0..map.width {
            fov.set_transparent(
                x as isize,
                y as isize,
                !map.tiles[x as usize + y as usize * map.width as usize].block_sight,
            );
        }
    }

    fov
}
