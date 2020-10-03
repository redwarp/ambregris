use crate::components::*;
use crate::game::RunState;
use crate::map::Map;
use crate::resources::PlayerInfo;
use field_of_vision::FovMap;
use legion::component;
use legion::system;
use legion::systems::CommandBuffer;
use legion::world::SubWorld;
use legion::Entity;
use legion::IntoQuery;
use legion::Schedule;

pub fn game_schedule() -> Schedule {
    Schedule::builder()
        .add_system(monster_move_system())
        .flush()
        .add_system(attack_actions_system())
        .add_system(move_actions_system())
        .flush()
        .add_system(update_map_and_position_system())
        .build()
}

#[system(for_each)]
#[filter(!component::<Player>())]
#[read_component(Player)]
pub fn monster_move(
    cmd: &mut CommandBuffer,
    body: &Body,
    _: &Monster,
    entity: &Entity,
    #[resource] player_info: &PlayerInfo,
    #[resource] run_state: &RunState,
    #[resource] fov: &FovMap,
) {
    if run_state.clone() != RunState::AiTurn {
        return;
    }
    let player_position = player_info.position;
    let distance = body.distance_to(player_position);
    if fov.is_in_fov(body.x as isize, body.y as isize) {
        println!("The {} sees you.", body.name);
        if distance >= 2.0 {
            let dx = player_position.0 - body.x;
            let dy = player_position.1 - body.y;

            let dx = (dx as f32 / distance).round() as i32;
            let dy = (dy as f32 / distance).round() as i32;
            println!("Entity {:?} set to move", entity);

            cmd.push((MoveAction {
                entity: *entity,
                dx,
                dy,
            },));
        } else {
            // Attack!
            let attack_action = AttackAction {
                attacker_entity: entity.clone(),
                target_entity: player_info.entity.clone(),
            };
            cmd.push((attack_action,));
        }
    }
}

#[system]
#[read_component(Player)]
#[read_component(Body)]
pub fn update_map_and_position(
    world: &mut SubWorld,
    #[resource] map: &mut Map,
    #[resource] player_info: &mut PlayerInfo,
) {
    for (index, tile) in map.tiles.iter().enumerate() {
        map.blocked[index] = tile.blocking;
    }

    let mut body_query = <&Body>::query();
    for body in body_query.iter_mut(world) {
        if body.blocking {
            let index = map.index(body.position());
            map.blocked[index] = true;
        }
    }
    let mut player_query = <(&Player, &Body)>::query();
    let (_, player_body) = player_query.iter(world).next().unwrap();
    player_info.position = player_body.position();
}

#[system(for_each)]
#[write_component(Body)]
pub fn move_actions(
    cmd: &mut CommandBuffer,
    world: &mut SubWorld,
    move_action: &MoveAction,
    entity: &Entity,
    #[resource] map: &mut Map,
) {
    let mut query = <&mut Body>::query();

    println!("Now moving entity {:?}", move_action.entity);

    let body = query.get_mut(world, move_action.entity);
    if let Ok(body) = body {
        let old_position = body.position();
        let new_position = (body.x + move_action.dx, body.y + move_action.dy);
        if !map.is_blocked(new_position) {
            body.set_position(new_position);
            // Update map of blocked. It can seem useless but if not for that code,
            // the next entity might try to also move on the same tile.
            let old_index = map.index(old_position);
            let new_index = map.index(new_position);
            map.blocked[old_index] = false;
            map.blocked[new_index] = true;
        }
    }

    cmd.remove(*entity);
}

#[system(for_each)]
#[read_component(CombatStats)]
#[read_component(Body)]
pub fn attack_actions(
    cmd: &mut CommandBuffer,
    world: &mut SubWorld,
    move_action: &AttackAction,
    entity: &Entity,
) {
    let (attacker_body, attacker_stats) = <(&Body, &CombatStats)>::query()
        .get(world, move_action.attacker_entity)
        .unwrap();
    let attacker_name = attacker_body.name.clone();

    let (target_body, target_stats) = <(&Body, &CombatStats)>::query()
        .get(world, move_action.target_entity)
        .unwrap();
    let target_name = target_body.name.clone();

    println!("The {} tries to attack {}", attacker_name, target_name);

    cmd.remove(*entity);
}
