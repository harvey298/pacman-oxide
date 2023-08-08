use std::{sync::{Arc, Mutex}, ops::{Add, Sub}, rc::Rc, f32::INFINITY};

use bevy::{prelude::*, sprite::collide_aabb::collide};
use bevy_inspector_egui::prelude::*;

use crate::game::{TILE_SIZE, MovementHelper, AnimationTimer, AnimationDescriptor, ManualAnimationControl};

use super::{util::{get_heighest_distance, calculate_next_step, chase, get_real_pos, calculate_distance, check_collosion, get_pos_infront_of_pacman, PowerPellets}, map::{Food, WallType}, player::PlayerData, ENERGIZED_GHOST_DEBUFF, PINKY_LEAVE_TIME, INKY_LEAVE_TIME, CLYDE_LEAVE_TIME};

use super::{player::{PlayerMovement, GameData}, STEP_SIZE, map::{TileCollider, DEFAULT_MAP_ORIGIN}, EntityState, CurrentDirection};

mod util;

use util::GhostState;

// TODO: Give Blinky "Cruise Elroy"

// Ghost gang limiter
const ALLOW_BLINKY: bool = true;
const ALLOW_PINKY: bool = true;
const ALLOW_INKY: bool = true;
const ALLOW_CLYDE: bool = true;

// Extra Ghost gangers
const ALLOW_SUE: bool = false;

const STUCK_TICKS: isize = 60;

#[derive(Debug, Clone, Copy, Component)]
pub struct GhostSpawn {
    pub spawn_for: GhostPersonality,
    pub power_pellet: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub enum GhostPersonality {
    Blinky, Pinky, Inky, Clyde
}

#[derive(Debug, Clone, Component, Reflect)]
// #[(Debug)]
pub struct Ghost {
    pub alive: bool,
    pub personality: GhostPersonality,
    pub state: EntityState,
    pub has_ai: bool,

    /// To ensure the ghost doesn't get stuck!
    pub stuck_ticks: Option<isize>,
    
    /// Allows the ghost to be given an AI
    pub award_ai: bool,

    pub steps: Vec<Vec3>,

    pub scatter_zone: usize,

    pub house_time: u64,
}

#[derive(Debug, Clone, Copy, Resource, Component)]
pub struct Enemy;

impl Ghost {
    pub fn new(personaility: GhostPersonality, scatter_zone: usize) -> Self {

        let house_time = match personaility {
            GhostPersonality::Blinky => 0,
            GhostPersonality::Pinky => PINKY_LEAVE_TIME,
            GhostPersonality::Inky => INKY_LEAVE_TIME,
            GhostPersonality::Clyde => CLYDE_LEAVE_TIME,
        };

        Self { 
            alive: true,
            personality: personaility,
            state: EntityState::Created,
            has_ai: true,
            award_ai: true,
            stuck_ticks: None,
            steps: Vec::new(),
            scatter_zone: scatter_zone,
            house_time: house_time
        }
    }

    fn _spawn_internal(mut commands: Arc<Mutex<Commands>>, personaility: GhostPersonality, asset_server: &AssetServer, mut texture_atlases: Rc<Mutex<ResMut<Assets<TextureAtlas>>>>, scatter_zone: usize) {

        debug!("personaility: {:?}", personaility);

        // Chaser - Red guy - Urchin - Macky - Shadow - Blinky
        let (texture_handle, name) = match personaility {
            GhostPersonality::Blinky => { (asset_server.load("ghosts/blinky.png"), "Blinky") },
            GhostPersonality::Pinky => (asset_server.load("ghosts/pinky.png"), "Pinky"),
            GhostPersonality::Inky => (asset_server.load("ghosts/inky.png"), "Inky"),
            GhostPersonality::Clyde => (asset_server.load("ghosts/clyde.png"), "Clyde"),
        };

        let atlas = TextureAtlas::from_grid(texture_handle, Vec2 { x: 16.0, y: 14.0 }, 1, 8, None, None);
        let texture_atlas_handle = texture_atlases.lock().unwrap().add(atlas);

        commands.clone().lock().unwrap().spawn(SpriteSheetBundle  {
            texture_atlas: texture_atlas_handle,
             
            transform: Transform {
                translation: Vec3 { x: 0.0, y: 0.0, z: 900.0 },
                scale: Vec3::splat(TILE_SIZE),
                ..Default::default()
            },
            
            ..Default::default()
        })
        .insert(Name::new(name))
        .insert(Self::new(personaility, scatter_zone))
        .insert(MovementHelper{ direction: None })
        .insert(AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)))
        .insert(AnimationDescriptor{ main_controller: true, reset_on_idle: true, manual: ManualAnimationControl{ max_index: 8, index: 0, current_index: None, enable: false } })
        .insert(Enemy)
        
        ; info!("{} is here!",name);        

    }

    pub fn spawn(mut commands: Commands, spawn_points: Query<(&GhostSpawn, &Transform)>, asset_server: Res<AssetServer>, mut texture_atlases: ResMut<Assets<TextureAtlas>>) {

        let texture_atlases = Rc::new(Mutex::new(texture_atlases));
        let commands = Arc::new(Mutex::new(commands));

        let mut pinky_present = false;

        for (point, point_transform) in &spawn_points {

            let personaility = point.spawn_for;
            let scatter_zone = point.power_pellet;

            info!("Attempt Spawning {:?}", personaility);

            // Check to see if the ghost is allowed
            match personaility {
                GhostPersonality::Blinky => { if !ALLOW_BLINKY { info!("Sorry Blinky! Not today :("); continue } },
                GhostPersonality::Pinky => { if !ALLOW_PINKY { info!("Sorry Pinky! Maybe next time! :("); continue } else { pinky_present = true; } },
                GhostPersonality::Inky => { if !ALLOW_INKY { info!("Sorry Inky! You little trouble maker! :("); continue} },
                GhostPersonality::Clyde => { if !ALLOW_CLYDE { info!("A dream for you Clyde!"); continue} },
            }

            Self::_spawn_internal(commands.clone(), personaility, &asset_server, texture_atlases.clone(), scatter_zone)
            
        }

        if ALLOW_INKY && !pinky_present {
            panic!("Pinky must be present for Inky to spawn");
        }


    }

    /// The Brains of the ghosts
    /// TODO - Re-write
    pub fn tick(
        mut me: Query<(&mut Self, &mut Transform, &mut MovementHelper), Without<PlayerMovement>>,
        target: Query<(&Transform, &MovementHelper, &GameData, &PlayerData), (With<PlayerMovement>, Without<TileCollider>)>,
        walls: Query<(&Transform, &TileCollider), (Without<PlayerMovement>, Without<AnimationDescriptor>, Without<MovementHelper>, Without<Ghost>)>,
        spawn_points: Query<(&GhostSpawn, &Transform), (Without<PlayerMovement>, Without<AnimationDescriptor>, Without<MovementHelper>, Without<Ghost>, Without<TileCollider>)>,
        food: Query<(&Food, &Visibility, &Transform), (Without<PlayerMovement>, Without<AnimationDescriptor>, Without<MovementHelper>, Without<Ghost>, Without<TileCollider>, Without<GhostSpawn>)>,
        mut power_pellets_data: ResMut<PowerPellets>,
    ) {

        let mut pinky_pos = None;
        let mut inky_index = None;
        let mut raw_target = calculate_next_step(true);

        let (player, player_movement_helper, game_data, player_data) = target.single();

        let scatter_mode = match player_data.state {
            EntityState::Alive => {false},
            EntityState::Dead => {false},
            EntityState::Respawning => {false},
            EntityState::Created => {false},
            EntityState::Energized(_) => {true},
        };

        for (ghost_index, (mut ghost, mut transform, mut my_helper)) in &mut me.iter_mut().enumerate() {

            let scatter_zone = get_real_pos(power_pellets_data.pellets.get(&ghost.scatter_zone).unwrap().clone(), DEFAULT_MAP_ORIGIN);

            match ghost.state {
                EntityState::Alive => {
                    if ghost.has_ai {
                
                        // For game resetting, also enforced else where
                        if game_data.transitioning { ghost.state = EntityState::Created; return }

                        let player_pos = if scatter_mode { 
                            raw_target -= ENERGIZED_GHOST_DEBUFF;
                            
                            scatter_zone

                        } else { player.translation };

                        let my_pos = transform.translation;
                        
        
                        match ghost.personality {
                            GhostPersonality::Blinky => {
                                        
                                match chase(player_pos, my_helper.direction.unwrap_or(CurrentDirection::Idle), my_pos, &walls) {
                                    Some(obj) => {

                                        // Corner check
                                        if obj.all.len() <= 1 || ghost.stuck_ticks.is_some() {
                                            
                                            // info!("Stuck at a conrer | {:?}", ghost.stuck_ticks);
                                            // info!("Crossroads detected!");
                                            if ghost.stuck_ticks.is_none() {
                                                ghost.stuck_ticks = Some(STUCK_TICKS);
                                            }

                                            // > STUCK_TICKS
                                            if ghost.stuck_ticks.unwrap() < -1 {
                                                ghost.stuck_ticks = None;
                                            }
                                            // else {
                                            //     ghost.stuck_ticks = Some(ghost.stuck_ticks.unwrap().add(1));
                                            // }

                                            // Does this have to be a option? the chance of this returning Option::None is very small
                                            let heighst = get_heighest_distance(player_pos, my_helper.direction.unwrap_or(CurrentDirection::Idle), my_pos, &walls);

                                            match heighst {
                                                Some(o) => {

                                                    match o.choice {
                                                        Some(h) => {
                    
                                                            // warning, this code is for when the ghost is stuck
                                                            if ghost.stuck_ticks == Some(STUCK_TICKS) {

                                                                // Animation
                                                                if obj.up == Some(h) {
                                                                    my_helper.direction = Some(CurrentDirection::Up)

                                                                } else if obj.down == Some(h) {
                                                                    my_helper.direction = Some(CurrentDirection::Down)

                                                                } else if obj.left == Some(h) {
                                                                    my_helper.direction = Some(CurrentDirection::Left);

                                                                } else if obj.right == Some(h) {
                                                                    my_helper.direction = Some(CurrentDirection::Right)

                                                                } else {
                                                                    my_helper.direction = None;
                                                                }

                                                                transform.translation = h;
                                                            } else {
                                                                // TODO: Redo this!
                                                                match my_helper.direction {
                                                                    Some(o) => {
                                                                        // println!("Modiying Direction: {:?}",o);
                                                                        match o {
                                                                            CurrentDirection::Up => {
                                                                                // T/t = Tiles per tick
                                                                                let target = transform.translation + Vec3::new(0.0, raw_target, 0.0);
                                                                                let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                                if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                                                            },
                                                                            CurrentDirection::Down => {
                                                                                // T/t = Tiles per tick
                                                                                let target = transform.translation - Vec3::new(0.0, raw_target, 0.0);
                                                                                let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                                if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                                    
                                                                            },
                                                                            CurrentDirection::Left => {
                                                                                // T/t = Tiles per tick
                                                                                let target = transform.translation - Vec3::new(raw_target, 0.0, 0.0);
                                                                                let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                                if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                                    
                                                                            },
                                                                            CurrentDirection::Right => {
                                                                                // T/t = Tiles per tick
                                                                                let target = transform.translation + Vec3::new(raw_target, 0.0, 0.0);
                                                                                let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                                if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                                                            },
                                                                            CurrentDirection::Idle => {}, // Don't move!
                                                                        }
                                                                    },
                                                                    None => {my_helper.direction = Some(CurrentDirection::Idle) },
                                                                }
                                                            }
                                                            

                                                            if ghost.stuck_ticks.is_some() {
                                                                ghost.stuck_ticks = Some(ghost.stuck_ticks.unwrap().sub(1));
                                                            }
                                                            
                                                        },
                                                        None => todo!("enemy: 140"),
                                                    }
                                                },
                                                // This should be safe
                                                None => todo!(),
                                            }
                                            
                                        
                                        } else {
                                            // Here is the code for normal and actual movement
                                            match obj.choice {
                                                Some(o) => {
            
                                                    // Animation
                                                    if obj.up == Some(o) {
                                                        my_helper.direction = Some(CurrentDirection::Up)
                                                        
                                                    } else if obj.down == Some(o) {
                                                        my_helper.direction = Some(CurrentDirection::Down)

                                                    } else if obj.left == Some(o) {
                                                        my_helper.direction = Some(CurrentDirection::Left)

                                                    } else if obj.right == Some(o) {
                                                        my_helper.direction = Some(CurrentDirection::Right)

                                                    } else {
                                                        my_helper.direction = None;
                                                    }
                                                    
                                                    // let mut found_good_pos = false;
                                                    // for item in food.iter() {
                                                    //     let food_pos = get_real_pos(item.2.translation, DEFAULT_MAP_ORIGIN);
                                                    //     let distance = calculate_distance(food_pos,my_pos);
                                                    //     info!("Distance: {}", distance);
                                                    //     if distance <= 90.0 {
                                                    //         // info!("{:?}", food_pos.distance(my_pos));
                                                    //         transform.translation = food_pos;
                                                    //         found_good_pos = true;
                                                    //         break;
                                                    //     }
                                                        
                                                    // }

                                                    transform.translation = o;

                                                },
                                                None => todo!("enemy: 181"),
                                            }
                                        }
                                    },
                                    None => {info!("I am stuck!")},
                                }
                            }
                            GhostPersonality::Pinky =>  {

                                let player_pos = get_pos_infront_of_pacman(player_pos, player_movement_helper.direction.unwrap_or(CurrentDirection::Idle));

                                match chase(player_pos, my_helper.direction.unwrap_or(CurrentDirection::Idle), my_pos, &walls) {
                                    Some(obj) => {

                                        // Corner check
                                        if obj.all.len() <= 1 || ghost.stuck_ticks.is_some() {
                                            
                                            // info!("Stuck at a conrer | {:?}", ghost.stuck_ticks);
                                            // info!("Crossroads detected!");
                                            if ghost.stuck_ticks.is_none() {
                                                ghost.stuck_ticks = Some(STUCK_TICKS);
                                            }

                                            // > STUCK_TICKS
                                            if ghost.stuck_ticks.unwrap() < -1 {
                                                ghost.stuck_ticks = None;
                                            }
                                            // else {
                                            //     ghost.stuck_ticks = Some(ghost.stuck_ticks.unwrap().add(1));
                                            // }

                                            // Does this have to be a option? the chance of this returning Option::None is very small
                                            let heighst = get_heighest_distance(player_pos, my_helper.direction.unwrap_or(CurrentDirection::Idle), my_pos, &walls);

                                            match heighst {
                                                Some(o) => {

                                                    match o.choice {
                                                        Some(h) => {
                    
                                                            // warning, this code is for when the ghost is stuck
                                                            if ghost.stuck_ticks == Some(STUCK_TICKS) {

                                                                // Animation
                                                                if obj.up == Some(h) {
                                                                    my_helper.direction = Some(CurrentDirection::Up)

                                                                } else if obj.down == Some(h) {
                                                                    my_helper.direction = Some(CurrentDirection::Down)

                                                                } else if obj.left == Some(h) {
                                                                    my_helper.direction = Some(CurrentDirection::Left);

                                                                } else if obj.right == Some(h) {
                                                                    my_helper.direction = Some(CurrentDirection::Right)

                                                                } else {
                                                                    my_helper.direction = None;
                                                                }

                                                                transform.translation = h;
                                                            } else {
                                                                // TODO: Redo this!
                                                                match my_helper.direction {
                                                                    Some(o) => {
                                                                        // println!("Modiying Direction: {:?}",o);
                                                                        match o {
                                                                            CurrentDirection::Up => {
                                                                                // T/t = Tiles per tick
                                                                                let target = transform.translation + Vec3::new(0.0, raw_target, 0.0);
                                                                                let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                                if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                                                            },
                                                                            CurrentDirection::Down => {
                                                                                // T/t = Tiles per tick
                                                                                let target = transform.translation - Vec3::new(0.0, raw_target, 0.0);
                                                                                let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                                if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                                    
                                                                            },
                                                                            CurrentDirection::Left => {
                                                                                // T/t = Tiles per tick
                                                                                let target = transform.translation - Vec3::new(raw_target, 0.0, 0.0);
                                                                                let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                                if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                                    
                                                                            },
                                                                            CurrentDirection::Right => {
                                                                                // T/t = Tiles per tick
                                                                                let target = transform.translation + Vec3::new(raw_target, 0.0, 0.0);
                                                                                let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                                if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                                                            },
                                                                            CurrentDirection::Idle => {}, // Don't move!
                                                                        }
                                                                    },
                                                                    None => {my_helper.direction = Some(CurrentDirection::Idle) },
                                                                }
                                                            }
                                                            

                                                            if ghost.stuck_ticks.is_some() {
                                                                ghost.stuck_ticks = Some(ghost.stuck_ticks.unwrap().sub(1));
                                                            }
                                                            
                                                        },
                                                        None => todo!("enemy: 140"),
                                                    }
                                                },
                                                // This should be safe
                                                None => todo!(),
                                            }
                                            
                                        
                                        } else {
                                            // Here is the code for normal and actual movement
                                            match obj.choice {
                                                Some(o) => {
            
                                                    // Animation
                                                    if obj.up == Some(o) {
                                                        my_helper.direction = Some(CurrentDirection::Up)
                                                        
                                                    } else if obj.down == Some(o) {
                                                        my_helper.direction = Some(CurrentDirection::Down)

                                                    } else if obj.left == Some(o) {
                                                        my_helper.direction = Some(CurrentDirection::Left)

                                                    } else if obj.right == Some(o) {
                                                        my_helper.direction = Some(CurrentDirection::Right)

                                                    } else {
                                                        my_helper.direction = None;
                                                    }
                                                    
                                                    // let mut found_good_pos = false;
                                                    // for item in food.iter() {
                                                    //     let food_pos = get_real_pos(item.2.translation, DEFAULT_MAP_ORIGIN);
                                                    //     let distance = calculate_distance(food_pos,my_pos);
                                                    //     info!("Distance: {}", distance);
                                                    //     if distance <= 90.0 {
                                                    //         // info!("{:?}", food_pos.distance(my_pos));
                                                    //         transform.translation = food_pos;
                                                    //         found_good_pos = true;
                                                    //         break;
                                                    //     }
                                                        
                                                    // }

                                                    transform.translation = o;

                                                },
                                                None => todo!("enemy: 181"),
                                            }
                                        }
                                    },
                                    None => {info!("I am stuck!")},
                                }

                                pinky_pos = Some(transform.translation);
                            },
                            GhostPersonality::Inky => inky_index = Some(ghost_index),
                            GhostPersonality::Clyde => {

                                let player_pos = if calculate_distance(player_pos, my_pos) < 8.0 {
                                    scatter_zone
                                } else { player_pos };

                                match chase(player_pos, my_helper.direction.unwrap_or(CurrentDirection::Idle), my_pos, &walls) {
                                    Some(obj) => {

                                        // Corner check
                                        if obj.all.len() <= 1 || ghost.stuck_ticks.is_some() {
                                            
                                            // info!("Stuck at a conrer | {:?}", ghost.stuck_ticks);
                                            // info!("Crossroads detected!");
                                            if ghost.stuck_ticks.is_none() {
                                                ghost.stuck_ticks = Some(STUCK_TICKS);
                                            }

                                            // > STUCK_TICKS
                                            if ghost.stuck_ticks.unwrap() < -1 {
                                                ghost.stuck_ticks = None;
                                            }
                                            // else {
                                            //     ghost.stuck_ticks = Some(ghost.stuck_ticks.unwrap().add(1));
                                            // }

                                            // Does this have to be a option? the chance of this returning Option::None is very small
                                            let heighst = get_heighest_distance(player_pos, my_helper.direction.unwrap_or(CurrentDirection::Idle), my_pos, &walls);

                                            match heighst {
                                                Some(o) => {

                                                    match o.choice {
                                                        Some(h) => {
                    
                                                            // warning, this code is for when the ghost is stuck
                                                            if ghost.stuck_ticks == Some(STUCK_TICKS) {

                                                                // Animation
                                                                if obj.up == Some(h) {
                                                                    my_helper.direction = Some(CurrentDirection::Up)

                                                                } else if obj.down == Some(h) {
                                                                    my_helper.direction = Some(CurrentDirection::Down)

                                                                } else if obj.left == Some(h) {
                                                                    my_helper.direction = Some(CurrentDirection::Left);

                                                                } else if obj.right == Some(h) {
                                                                    my_helper.direction = Some(CurrentDirection::Right)

                                                                } else {
                                                                    my_helper.direction = None;
                                                                }

                                                                transform.translation = h;
                                                            } else {
                                                                // TODO: Redo this!
                                                                match my_helper.direction {
                                                                    Some(o) => {
                                                                        // println!("Modiying Direction: {:?}",o);
                                                                        match o {
                                                                            CurrentDirection::Up => {
                                                                                // T/t = Tiles per tick
                                                                                let target = transform.translation + Vec3::new(0.0, raw_target, 0.0);
                                                                                let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                                if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                                                            },
                                                                            CurrentDirection::Down => {
                                                                                // T/t = Tiles per tick
                                                                                let target = transform.translation - Vec3::new(0.0, raw_target, 0.0);
                                                                                let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                                if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                                    
                                                                            },
                                                                            CurrentDirection::Left => {
                                                                                // T/t = Tiles per tick
                                                                                let target = transform.translation - Vec3::new(raw_target, 0.0, 0.0);
                                                                                let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                                if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                                    
                                                                            },
                                                                            CurrentDirection::Right => {
                                                                                // T/t = Tiles per tick
                                                                                let target = transform.translation + Vec3::new(raw_target, 0.0, 0.0);
                                                                                let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                                if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                                                            },
                                                                            CurrentDirection::Idle => {}, // Don't move!
                                                                        }
                                                                    },
                                                                    None => {my_helper.direction = Some(CurrentDirection::Idle) },
                                                                }
                                                            }
                                                            

                                                            if ghost.stuck_ticks.is_some() {
                                                                ghost.stuck_ticks = Some(ghost.stuck_ticks.unwrap().sub(1));
                                                            }
                                                            
                                                        },
                                                        None => todo!("enemy: 140"),
                                                    }
                                                },
                                                // This should be safe
                                                None => todo!(),
                                            }
                                            
                                        
                                        } else {
                                            // Here is the code for normal and actual movement
                                            match obj.choice {
                                                Some(o) => {
            
                                                    // Animation
                                                    if obj.up == Some(o) {
                                                        my_helper.direction = Some(CurrentDirection::Up)
                                                        
                                                    } else if obj.down == Some(o) {
                                                        my_helper.direction = Some(CurrentDirection::Down)

                                                    } else if obj.left == Some(o) {
                                                        my_helper.direction = Some(CurrentDirection::Left)

                                                    } else if obj.right == Some(o) {
                                                        my_helper.direction = Some(CurrentDirection::Right)

                                                    } else {
                                                        my_helper.direction = None;
                                                    }
                                                

                                                    transform.translation = o;
                                                    // todo!("Clyde Scatter Mode")

                                                },
                                                None => todo!("enemy: 181"),
                                            }
                                        }
                                    },
                                    None => {info!("I am stuck!")},
                                }

                            },
                        }
                    
                    }
                },
                // TODO - Finish
                EntityState::Dead => { ghost.state = EntityState::Created },
                EntityState::Respawning => { if ghost.house_time != 0 { ghost.house_time -= 1; continue; } else { ghost.state = EntityState::Alive;
                    
                    for (wall_pos, wall_type) in &walls {
                        
                        let wall_type = wall_type.r#type;
                        if wall_type != WallType::Gate { continue; }

                        let wall_pos = get_real_pos(wall_pos.translation, DEFAULT_MAP_ORIGIN);
                        
                        let mut offset = 0.0;

                        loop {
                            let target = Vec3 { x: wall_pos.x, y: wall_pos.y+calculate_next_step(false)+offset, z: 900.0 };

                            // True for available
                            let available = !collide(target, Vec2::splat(TILE_SIZE*0.9), wall_pos, Vec2::splat(TILE_SIZE*10.0)).is_some();

                            transform.translation = target;
                            if available {
                                
                                break
                            } else {
                                
                                offset += 1.0;
                            }
                            
                        }
                        
                        
                        // transform.translation = Vec3 { x: wall_pos.x, y: wall_pos.y+calculate_next_step(false), z: 900.0 }; 
                        break;

                    }


                 } },
                EntityState::Created => {
                    ghost.has_ai = false;
                    for (spawn_point, spawn_pos) in &spawn_points {
                        let personaility = ghost.personality;
                        if spawn_point.spawn_for == personaility {
                            info!("Ghost in Created State! Resetting");
                            transform.translation = get_real_pos(spawn_pos.translation, DEFAULT_MAP_ORIGIN);

                            let house_time = match personaility {
                                GhostPersonality::Blinky => 0,
                                GhostPersonality::Pinky => PINKY_LEAVE_TIME,
                                GhostPersonality::Inky => INKY_LEAVE_TIME,
                                GhostPersonality::Clyde => CLYDE_LEAVE_TIME,
                            };

                            ghost.house_time = house_time;

                            if !game_data.transitioning {
                                ghost.state = EntityState::Respawning;

                                if ghost.award_ai {
                                    ghost.has_ai = true;
                                }
                            }

                            break
                        }
                    }

                },
                EntityState::Energized(_) => todo!(),
            }

        }

        // Inky AI is stashed here
        if pinky_pos.is_some() && inky_index.is_some() {
            for (mut ghost, mut transform, mut my_helper) in &mut me {
                if !ghost.alive || ghost.personality != GhostPersonality::Inky || !ghost.has_ai { continue; }
                    
                    let player = player.translation;
                    let pinky_pos = pinky_pos.unwrap();

                    let dx = 2.0 * (player.x - pinky_pos.x);
                    let dy = 2.0 * (player.y - pinky_pos.y);
                    let target = (player.x + dx, player.y + dy);

                    let target = Vec3::new(target.0, target.1, transform.translation.z);
                    let player_pos = target.clone();
                    let my_pos = transform.translation;

                    match chase(player_pos, my_helper.direction.unwrap_or(CurrentDirection::Idle), my_pos, &walls) {
                        Some(obj) => {

                            // Corner check
                            if obj.all.len() <= 1 || ghost.stuck_ticks.is_some() {
                                
                                // info!("Stuck at a conrer | {:?}", ghost.stuck_ticks);
                                // info!("Crossroads detected!");
                                if ghost.stuck_ticks.is_none() {
                                    ghost.stuck_ticks = Some(STUCK_TICKS);
                                }

                                // > STUCK_TICKS
                                if ghost.stuck_ticks.unwrap() < -1 {
                                    ghost.stuck_ticks = None;
                                }
                                // else {
                                //     ghost.stuck_ticks = Some(ghost.stuck_ticks.unwrap().add(1));
                                // }

                                // Does this have to be a option? the chance of this returning Option::None is very small
                                let heighst = get_heighest_distance(player_pos, my_helper.direction.unwrap_or(CurrentDirection::Idle), my_pos, &walls);

                                match heighst {
                                    Some(o) => {

                                        match o.choice {
                                            Some(h) => {
        
                                                // warning, this code is for when the ghost is stuck
                                                if ghost.stuck_ticks == Some(STUCK_TICKS) {

                                                    // Animation
                                                    if obj.up == Some(h) {
                                                        my_helper.direction = Some(CurrentDirection::Up)

                                                    } else if obj.down == Some(h) {
                                                        my_helper.direction = Some(CurrentDirection::Down)

                                                    } else if obj.left == Some(h) {
                                                        my_helper.direction = Some(CurrentDirection::Left);

                                                    } else if obj.right == Some(h) {
                                                        my_helper.direction = Some(CurrentDirection::Right)

                                                    } else {
                                                        my_helper.direction = None;
                                                    }

                                                    transform.translation = h;
                                                } else {
                                                    // TODO: Redo this!
                                                    match my_helper.direction {
                                                        Some(o) => {
                                                            // println!("Modiying Direction: {:?}",o);
                                                            match o {
                                                                CurrentDirection::Up => {
                                                                    // T/t = Tiles per tick
                                                                    let target = transform.translation + Vec3::new(0.0, raw_target, 0.0);
                                                                    let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                    if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                                                },
                                                                CurrentDirection::Down => {
                                                                    // T/t = Tiles per tick
                                                                    let target = transform.translation - Vec3::new(0.0, raw_target, 0.0);
                                                                    let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                    if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                        
                                                                },
                                                                CurrentDirection::Left => {
                                                                    // T/t = Tiles per tick
                                                                    let target = transform.translation - Vec3::new(raw_target, 0.0, 0.0);
                                                                    let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                    if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                        
                                                                },
                                                                CurrentDirection::Right => {
                                                                    // T/t = Tiles per tick
                                                                    let target = transform.translation + Vec3::new(raw_target, 0.0, 0.0);
                                                                    let touching = !check_collosion(target, &walls, DEFAULT_MAP_ORIGIN);
                                                                    if touching { transform.translation = target } else { ghost.stuck_ticks = None; }
                                                                },
                                                                CurrentDirection::Idle => {}, // Don't move!
                                                            }
                                                        },
                                                        None => {my_helper.direction = Some(CurrentDirection::Idle) },
                                                    }
                                                }
                                                

                                                if ghost.stuck_ticks.is_some() {
                                                    ghost.stuck_ticks = Some(ghost.stuck_ticks.unwrap().sub(1));
                                                }
                                                
                                            },
                                            None => todo!("enemy: 140"),
                                        }
                                    },
                                    // This should be safe
                                    None => todo!(),
                                }
                                
                            
                            } else {
                                // Here is the code for normal and actual movement
                                match obj.choice {
                                    Some(o) => {

                                        // Animation
                                        if obj.up == Some(o) {
                                            my_helper.direction = Some(CurrentDirection::Up)
                                            
                                        } else if obj.down == Some(o) {
                                            my_helper.direction = Some(CurrentDirection::Down)

                                        } else if obj.left == Some(o) {
                                            my_helper.direction = Some(CurrentDirection::Left)

                                        } else if obj.right == Some(o) {
                                            my_helper.direction = Some(CurrentDirection::Right)

                                        } else {
                                            my_helper.direction = None;
                                        }
                                        
                                        // let mut found_good_pos = false;
                                        // for item in food.iter() {
                                        //     let food_pos = get_real_pos(item.2.translation, DEFAULT_MAP_ORIGIN);
                                        //     let distance = calculate_distance(food_pos,my_pos);
                                        //     info!("Distance: {}", distance);
                                        //     if distance <= 90.0 {
                                        //         // info!("{:?}", food_pos.distance(my_pos));
                                        //         transform.translation = food_pos;
                                        //         found_good_pos = true;
                                        //         break;
                                        //     }
                                            
                                        // }

                                        transform.translation = o;

                                    },
                                    None => todo!("enemy: 181"),
                                }
                            }
                        },
                        None => {info!("I am stuck!")},
                    }


            }
        }

    }

    /// Ensures all ghosts are spawned
    pub fn enforcer(mut commands: Commands, spawn_points: Query<(&GhostSpawn, &Transform)>, asset_server: Res<AssetServer>, 
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    ghosts: Query<(&Enemy)>
    ) {

        if !spawn_points.is_empty() && ghosts.is_empty() {
            Self::spawn(commands, spawn_points, asset_server, texture_atlases);
        }

    }
}

// Ghost Core
pub struct GhostPlugin;

impl Plugin for GhostPlugin {
    /// Create the Ghosts here
    fn build(&self, app: &mut App) {
        info!("Here Come the Ghost Gang!");

        app
            // .add_startup_system(Ghost::spawn)
            .add_system(Ghost::enforcer)
            .add_system(Ghost::tick)
            .add_state::<crate::enemy::GhostState>()


        ;
    }

}