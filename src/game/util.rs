use std::{ops::Neg, path::Path, fs, io::Write};

use bevy::{prelude::*, sprite::collide_aabb::collide, utils::HashMap};
// use bevy_inspector_egui::Inspectable;

use anyhow::Result;
use serde_json::Value;

use super::{map::{TileCollider, Food, DEFAULT_MAP_ORIGIN}, player::PlayerMovement, TILE_SIZE, CurrentDirection, STEP_SIZE, AnimationDescriptor, MovementHelper, enemy::Ghost, GHOST_DEBUFF};

const CUSTOM_CHECKS: bool = false;

pub const DEFAULT_SETTINGS: &str = include_str!("../assets/settings.toml");

// Gets the Real Pos of a object
pub fn get_real_pos(pos: Vec3, origin: Vec3) -> Vec3 {
    pos + origin
}

/// Returns True is collision is detected
pub fn check_collosion(
    mut target_player_pos: Vec3,
    wall: &Query<(&Transform, &TileCollider), (Without<PlayerMovement>, Without<AnimationDescriptor>, Without<MovementHelper>, Without<Ghost>)>,
    origin: Vec3
) -> bool {

    target_player_pos.z = 100.0;
    
    let mut walls = 0;
    for (wall_data, collider) in wall {

        let mut wall = wall_data.translation + origin;

        wall.z = 100.0;

        // info!("Map Origin: {:?} | Wall Pos: {:?} | Real Wall Pos: {:?} | Player: {:?}", origin, wall_data.translation, wall, target_player_pos);
        
        let collision = if !CUSTOM_CHECKS {
            if collide(target_player_pos, Vec2::splat(TILE_SIZE*0.9), wall, Vec2::splat(TILE_SIZE*10.0)).is_some() {
                // debug!("Hit something!");
                true
            } else {
                false
            }

        } else {
            if target_player_pos == wall_data.translation {
                true
            } else {
                false
            }
        };

        walls += 1;
        if collision {
            return collision
        }

    }

    // debug!("Found {} walls!",walls);

    false
}

/// Make everything positive
pub fn calculate_distance(pos1: Vec3, pos2: Vec3) -> f32 {

    // Check to see if number is negative if yes make it positive
    /* 
    
    if pos1.x < 0.0 { pos1.x = pos1.x.neg() }
    if pos1.y < 0.0 { pos1.y = pos1.y.neg() }

    
    if pos2.x < 0.0 { pos2.x = pos2.x.neg(); println!("Player in Neg") }
    if pos2.y < 0.0 { pos2.y = pos2.y.neg(); println!("Player in Neg") }
    */


    (((pos1.x - pos2.x).powf(2.0)) + ((pos1.y - pos2.y).powf(2.0))).sqrt()


}

pub fn calculate_next_step(is_ghost: bool) -> f32 {
    let calc = STEP_SIZE * TILE_SIZE;

    if is_ghost {
        calc - GHOST_DEBUFF
    } else {
        calc
    }
}

/// Get 4 tiles in front of him
/// Fine for Pinky
/// Also fine for Blinky
/// Not fine for Inky as hes a mirror of Blinky
pub fn get_pos_infront_of_pacman(pacman_pos: Vec3, pacman_directon: CurrentDirection) -> Vec3 {

    match pacman_directon {
        CurrentDirection::Up => {(Vec3::new(0.0, calculate_next_step(false), 0.0)+Vec3::new(0.0, calculate_next_step(false), 0.0)+Vec3::new(0.0, calculate_next_step(false), 0.0)+Vec3::new(0.0, calculate_next_step(false), 0.0))+pacman_pos},
        CurrentDirection::Down => {(Vec3::new(0.0, calculate_next_step(false), 0.0)+Vec3::new(0.0, calculate_next_step(false), 0.0)+Vec3::new(0.0, calculate_next_step(false), 0.0)+Vec3::new(0.0, calculate_next_step(false), 0.0))-pacman_pos},

        CurrentDirection::Left => {(Vec3::new(calculate_next_step(false), 0.0, 0.0)+Vec3::new(calculate_next_step(false), 0.0, 0.0)+Vec3::new(calculate_next_step(false), 0.0, 0.0)+Vec3::new(calculate_next_step(false), 0.0, 0.0))-pacman_pos},
        CurrentDirection::Right => {(Vec3::new(calculate_next_step(false), 0.0, 0.0)+Vec3::new(calculate_next_step(false), 0.0, 0.0)+Vec3::new(calculate_next_step(false), 0.0, 0.0)+Vec3::new(calculate_next_step(false), 0.0, 0.0))+pacman_pos},
        CurrentDirection::Idle => pacman_pos,
    }
}

/// Meant for crybaby (Clyde), will use same logic as Blinky
pub fn do_i_run(pacman_pos: Vec3, me: Vec3) -> bool {
    if calculate_distance(pacman_pos, me) > 8.0 { true } else { false }
}

#[derive(Debug,  Clone, Copy)]
pub struct Moves {
    pub up: Option<Vec3>,
    pub down: Option<Vec3>,
    pub left: Option<Vec3>,
    pub right: Option<Vec3>,
}

/// Mainly for Blinky
/// TODO
pub fn chase(player_pos: Vec3, my_current_direction: CurrentDirection, my_pos: Vec3, walls: &Query<(&Transform, &TileCollider), (Without<PlayerMovement>, Without<AnimationDescriptor>, Without<MovementHelper>, Without<Ghost>)>) -> Option<MoveDesc> {
    let mut moves = determine_possible_moves(my_pos, my_current_direction, player_pos, walls);

    let mut lowest_distance: f32 = f32::MAX;
    let mut itr = 0;
    let mut best_choise = 0;
    let possible_moves = moves.all.clone();
    for possible_move in possible_moves.clone() {
        let distance = calculate_distance(possible_move, player_pos);

        // info!("Option {} | distance: {}", itr, distance);

        if distance < lowest_distance {
            lowest_distance = distance;
            best_choise = itr;
        }
        itr += 1;
    }

    //info!("Possible Moves: {} | Lowest Distance: {} | Picked: {}", possible_moves.len(), lowest_distance, best_choise);

    if possible_moves.len() != 0 {
        moves.choice = Some(possible_moves[best_choise]);
        Some(moves)
    } else { None }
}

pub fn get_heighest_distance(player_pos: Vec3, my_current_direction: CurrentDirection, my_pos: Vec3, walls: &Query<(&Transform, &TileCollider), (Without<PlayerMovement>, Without<AnimationDescriptor>, Without<MovementHelper>, Without<Ghost>)>) -> Option<MoveDesc> {
    let mut moves = determine_possible_moves(my_pos, my_current_direction, player_pos, walls);

    let mut heightest_distance: f32 = f32::MIN;
    let mut itr = 0;
    let mut best_choise = 0;
    let possible_moves = moves.all.clone();
    for possible_move in possible_moves.clone() {
        let distance = calculate_distance(possible_move, player_pos);

        // info!("Option {} | distance: {}", itr, distance);

        if distance < heightest_distance {
            heightest_distance = distance;
            best_choise = itr;
        }
        itr += 1;
    }

    //info!("Possible Moves: {} | Lowest Distance: {} | Picked: {}", possible_moves.len(), lowest_distance, best_choise);

    if possible_moves.len() != 0 {
        moves.choice = Some(possible_moves[best_choise]);
        Some(moves)
    } else { None }
}

pub struct MoveDesc {
    pub up: Option<Vec3>,
    pub down: Option<Vec3>,
    pub left: Option<Vec3>,
    pub right: Option<Vec3>,
    pub all: Vec<Vec3>,
    pub choice: Option<Vec3>,
    pub alt_choice: Option<Vec3>,
}

/// Reminder that ghosts are NOT allowed to turn around at crossroads this is TODO
pub fn determine_possible_moves(my_pos: Vec3, my_current_direction: CurrentDirection, player_pos: Vec3, walls: &Query<(&Transform, &TileCollider), (Without<PlayerMovement>, Without<AnimationDescriptor>, Without<MovementHelper>, Without<Ghost>)>) -> MoveDesc {
    let up = my_pos + Vec3::new(0.0, calculate_next_step(true), 0.0);
    let down = my_pos - Vec3::new(0.0, calculate_next_step(true), 0.0);

    let left = my_pos - Vec3::new(calculate_next_step(true), 0.0, 0.0);
    let right = my_pos + Vec3::new(calculate_next_step(true), 0.0, 0.0);

    // if player_pos.x < 0.0 { pos2.x = pos2.x.neg(); println!("Player in Neg") }
    // if player_pos.y < 0.0 { pos2.y = pos2.y.neg(); println!("Player in Neg") }

    // info!("UP: {:?} | Down: {:?} | Left: {:?} | Right: {:?}", up, down, left, right);

    let mut move_data = MoveDesc { up: None, down: None, left: None, right: None, all: Vec::new(), choice: None, alt_choice: None };

    let mut moves = Vec::new();
    if !check_collosion(up, walls, DEFAULT_MAP_ORIGIN) && my_current_direction.opposite() != CurrentDirection::Up {
        move_data.up = Some(up);
        moves.push(up)
    }
    if !check_collosion(down, walls, DEFAULT_MAP_ORIGIN) && my_current_direction.opposite() != CurrentDirection::Down {
        move_data.down = Some(down);
        moves.push(down)
    }
    if !check_collosion(left, walls, DEFAULT_MAP_ORIGIN) && my_current_direction.opposite() != CurrentDirection::Left {
        move_data.left = Some(left);
        moves.push(left)
    }
    if !check_collosion(right, walls, DEFAULT_MAP_ORIGIN) && my_current_direction.opposite() != CurrentDirection::Right {
        move_data.right = Some(right);
        moves.push(right)
    }

    move_data.all = moves;

    move_data
}

pub fn get_settings() -> Result<Value> {

    let data: Value = if Path::new("./settings.toml").exists() {
        toml::from_str(fs::read_to_string("./settings.toml")?.as_str())?
    } else {
        toml::from_str(DEFAULT_SETTINGS)?
    };

    Ok(data)
}

pub fn drop_settings() -> Result<()> {

    if !Path::new("./settings.toml").exists() {
        fs::File::create("./settings.toml")?.write(DEFAULT_SETTINGS.as_bytes())?;
    }

    Ok(())
}

#[derive(Resource, Reflect)]
pub struct PowerPellets {
    pub pellets: HashMap<usize, Vec3>,
}

impl Default for PowerPellets {
    fn default() -> Self {
        Self { pellets: HashMap::new() }
    }
}