use crate::game::State;
use crate::pistonengine::Engine as PistonEngine;
use crate::resources::SharedInfo;

use game::Journal;
use legion::{Resources, World};

mod colors;
mod components;
mod game;
mod inventory;
mod map;
mod palette;
mod pistonengine;
mod renderer;
mod resources;
mod spawner;
mod systems;
mod utils;

// actual size of the window
const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;

fn main() {
    let mut world = World::default();
    let mut resources = Resources::default();
    let player_entity = spawner::player(&mut world, -1, -1);
    let map = crate::map::make_map(&mut world, 1);
    let journal = Journal::new();
    resources.insert(map);
    resources.insert(journal);
    resources.insert(SharedInfo {
        player_entity: player_entity,
        player_position: (-1, -1).into(),
        alive: true,
    });
    let mut state = State {
        world,
        resources,
        player_entity,
    };
    state.log("Welcome to Ambergris");

    let mut renderer = PistonEngine::new("Ambergris", SCREEN_WIDTH, SCREEN_HEIGHT);
    renderer.run(&mut state);
}
