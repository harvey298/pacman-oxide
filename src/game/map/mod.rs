use std::{fs, path::Path, borrow::BorrowMut};

use bevy::utils::HashMap;
use bevy::{prelude::*, sprite::collide_aabb::collide};
use rayon::prelude::*;

use crate::game::enemy::{GhostSpawn, GhostPersonality};
use crate::game::{EntityState};
use crate::game::CurrentDirection;

use super::{POWER_PELLET_WORTH, ENERGIZED_MAX_LENGTH};
use super::enemy::Ghost;
use super::util::PowerPellets;
use super::{TILE_SIZE, player::{PlayerMovement, PlayerData, GameData}, MovementHelper, util::{check_collosion, get_real_pos}, PAC_DOT_WORTH};

const DEFAULT_MAP: &str = include_str!("../../assets/level.map");

pub const DEFAULT_MAP_ORIGIN: Vec3 = Vec3::new(-329.0, 124.0, 0.0);

const CUSTOM_MAP: &str = "./clevel.map";

#[derive(Debug, )]
pub struct TileMap;

impl Plugin for TileMap {
    fn build(&self, app: &mut App) {
        app 
            .add_startup_system(TileMap::create_map)
            .add_system(MapEnforcer::check_map)
            
            
            
        ;
    }
}

impl TileMap {
    pub fn get_map() -> String {
        if Path::new(CUSTOM_MAP).exists() {
            debug!("Found Custom Map!");
            fs::read_to_string(CUSTOM_MAP).unwrap()
        } else {
            DEFAULT_MAP.to_string()
        }
    }

    /// Creates the map 
    /// TODO: Rewrite
    pub fn create_map(
        mut commands: Commands, 
        asset_server: Res<AssetServer>, 
        mut texture_atlases: ResMut<Assets<TextureAtlas>>, 
        mut player: Query<(&PlayerMovement, &mut Transform)>,
        mut power_pellets_data: ResMut<PowerPellets>,
    ) {
        let map = Self::get_map();
        info!("Attempting map creation");
        let custom_map = true;
        let test_map_texture = false;

        let texture_handle = asset_server.load("maze.png");
        let atlas = TextureAtlas::from_grid(texture_handle, Vec2 { x: 224.0, y: 248.0 }, 1, 1, None, None);
        let test_map = texture_atlases.add(atlas);
        
        if test_map_texture {
            commands.spawn(SpriteSheetBundle  {
                texture_atlas: test_map,

                // /* 
                transform: Transform {
                    translation: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
                    scale: Vec3::new(3.0, 3.0, 3.1),
                    ..Default::default()
                },
                // */
                
                ..Default::default()
            }).insert(Name::new("test_map"));  
        }

        if custom_map {
            // Check Map Here
            {
                if !map.contains("S") || !map.contains(".") {
                    // If true the custom map cannot be used
                    panic!("Custom Map has no spawn or food!")
                }

                let all_opt_checks = 4;
                let mut checks = 0;
                if !map.contains("#") { warn!("Map has no walls!"); } else { checks += 1 }
                if !map.contains("|") { warn!("Map has no teleports!"); } else { checks += 1 }
                if !map.contains("@") { warn!("Map has power pellets!"); } else { checks += 1 }
                if !map.contains("F") { warn!("Map has no fruits!"); } else { checks += 1 }

                if checks == all_opt_checks {
                    info!("Map passed pre-load checks")
                } else {
                    info!("Map passed {}/{} pre-load checks! This could cause some problems!",checks, all_opt_checks)
                }
            }

            /* 
            let anchor = commands.spawn(SpriteSheetBundle  {
                //transform: Transform::from_scale(Vec3::splat(TILE_SIZE)),
                
                transform: Transform {
                    translation: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
                    scale: Vec3::splat(0.5),
                    ..Default::default()
                },
                
                ..Default::default()
            }).insert(Name::new("map-anchor"));
            */

            let mut tiles = Vec::new();

            let mut tile_num = 0;
            let mut food_num = 0;
            let mut power_pellets = 0;
            let mut y = -10;
            let mut power_pellet_prio = 0;
            let mut teleport_locations = HashMap::new();
            for line in map.lines() {
                // For Map Creation
                for (x, char) in line.chars().enumerate() {
                    if char.to_string() == "#" {
                        let texture_handle = asset_server.load("test/test_blue_single_pixel.png");
                        let atlas = TextureAtlas::from_grid(texture_handle, Vec2 { x: 15.0, y: 15.0 }, 2, 4, None, None);
                        let texture_atlas_handle = texture_atlases.add(atlas);

                        let entity = commands.spawn(SpriteSheetBundle  {
                            texture_atlas: texture_atlas_handle,
                            //transform: Transform::from_scale(Vec3::splat(TILE_SIZE)),
                            
                            transform: Transform {
                                translation: Vec3 { x: (x as f32 * TILE_SIZE * 10.0), y: -(y as f32 * TILE_SIZE) * 10.0, z: 100.0 },
                                scale: Vec3::splat(TILE_SIZE),
                                ..Default::default()
                            },
                            
                            ..Default::default()
                        }).insert(Name::new(format!("Tile ({})",tile_num))).insert(TileCollider { r#type: WallType::Blocking })               
                        .id();

                        tiles.push(entity); tile_num += 1;
                    }

                    // TODO: complete teleport init
                    if char.to_string() == "|" {
                        println!("Change TP sprite");
                        let texture_handle = asset_server.load("test/test_blue_single_pixel.png");
                        let atlas = TextureAtlas::from_grid(texture_handle, Vec2 { x: 15.0, y: 15.0 }, 2, 4, None, None);
                        let texture_atlas_handle = texture_atlases.add(atlas);

                        let translation = Vec3 { x: (x as f32 * TILE_SIZE * 10.0), y: -(y as f32 * TILE_SIZE) * 10.0, z: 100.0 };

                        let destination = if !teleport_locations.is_empty() {
                            teleport_locations.get(&1).unwrap().clone()                            
                        } else { translation };

                        let entity = commands.spawn(SpriteSheetBundle  {
                            texture_atlas: texture_atlas_handle,
                            //transform: Transform::from_scale(Vec3::splat(TILE_SIZE)),
                            
                            transform: Transform {
                                translation: translation,
                                scale: Vec3::splat(TILE_SIZE),
                                ..Default::default()
                            },
                            
                            ..Default::default()
                        }).insert(Name::new(format!("Teleport ({})",tile_num))).insert(TileCollider { r#type: WallType::Teleport { destination: destination } })               
                        .id();

                        teleport_locations.insert(teleport_locations.len()+1, translation);

                        tiles.push(entity); tile_num += 1;
                    }

                    if char.to_string() == "G" {
                        let texture_handle = asset_server.load("test/test_blue_single_pixel.png");
                        let atlas = TextureAtlas::from_grid(texture_handle, Vec2 { x: 15.0, y: 15.0 }, 2, 4, None, None);
                        let texture_atlas_handle = texture_atlases.add(atlas);

                        let entity = commands.spawn(SpriteSheetBundle  {
                            texture_atlas: texture_atlas_handle,
                            //transform: Transform::from_scale(Vec3::splat(TILE_SIZE)),
                            
                            transform: Transform {
                                translation: Vec3 { x: (x as f32 * TILE_SIZE * 10.0), y: -(y as f32 * TILE_SIZE) * 10.0, z: 100.0 },
                                scale: Vec3::splat(TILE_SIZE),
                                ..Default::default()
                            },
                            
                            ..Default::default()
                        }).insert(Name::new(format!("Gated Tile ({})",tile_num))).insert(TileCollider { r#type: WallType::Gate })               
                        .id();

                        tiles.push(entity); tile_num += 1;

                    }

                    if char.to_string() == "S" {

                        let pos = Vec3 { x: (x as f32 * TILE_SIZE * 10.0), y: -(y as f32 * TILE_SIZE) * 10.0, z: 100.0 };

                        let entity = commands.spawn(SpriteSheetBundle  {                            
                            transform: Transform {
                                translation: pos,
                                scale: Vec3::splat(TILE_SIZE),
                                ..Default::default()
                            },
                            
                            ..Default::default()
                        }).insert(Name::new(format!("Spawn Tile"))).insert(SpawnPoint)               
                        .id();

                        tiles.push(entity);
                        match player.get_single_mut() {
                            Ok(player) => {
                                let (player_move, mut transform) = player;

                                transform.translation = pos;
                            },
                            Err(_) => {
                                error!("An error occured when spawning in the Map!");
                                error!("Player not ready!");
                            },
                        }
                        
                    }

                    // For Blinky spawn point
                    if char.to_string() == "B" {
                        let entity = commands.spawn(SpatialBundle
                            { visibility: Visibility::Hidden, transform: Transform {
                                translation: Vec3 { x: (x as f32 * TILE_SIZE * 10.0), y: -(y as f32 * TILE_SIZE) * 10.0, z: 100.0 },
                                scale: Vec3::splat(TILE_SIZE),
                                ..Default::default()

                            }, ..Default::default() }
                            )
                            .insert(Name::new("Blinky_Spawn"))
                            .insert(GhostSpawn{ spawn_for: GhostPersonality::Blinky, power_pellet: power_pellet_prio })

                            .id();


                        tiles.push(entity)
                    }

                    // For Pinky spawn point
                    if char.to_string() == "P" {
                        let entity = commands.spawn(SpatialBundle
                            { visibility: Visibility::Hidden, transform: Transform {
                                translation: Vec3 { x: (x as f32 * TILE_SIZE * 10.0), y: -(y as f32 * TILE_SIZE) * 10.0, z: 100.0 },
                                scale: Vec3::splat(TILE_SIZE),
                                ..Default::default()

                            }, ..Default::default() }
                            )
                            .insert(Name::new("Pinky_Spawn"))
                            .insert(GhostSpawn{ spawn_for: GhostPersonality::Pinky, power_pellet: power_pellet_prio })

                            .id();


                        tiles.push(entity);
                        info!("Pinky is in this map!");
                    }

                    // For Inky spawn point
                    if char.to_string() == "I" {
                        let entity = commands.spawn(SpatialBundle
                            { visibility: Visibility::Hidden, transform: Transform {
                                translation: Vec3 { x: (x as f32 * TILE_SIZE * 10.0), y: -(y as f32 * TILE_SIZE) * 10.0, z: 100.0 },
                                scale: Vec3::splat(TILE_SIZE),
                                ..Default::default()

                            }, ..Default::default() }
                            )
                            .insert(Name::new("Inky_Spawn"))
                            .insert(GhostSpawn{ spawn_for: GhostPersonality::Inky, power_pellet: power_pellet_prio })

                            .id();


                        tiles.push(entity);
                        info!("Inky is in this map!");
                    }

                    // For Clyde spawn point
                    if char.to_string() == "C" {
                        let entity = commands.spawn(SpatialBundle
                            { visibility: Visibility::Hidden, transform: Transform {
                                translation: Vec3 { x: (x as f32 * TILE_SIZE * 10.0), y: -(y as f32 * TILE_SIZE) * 10.0, z: 100.0 },
                                scale: Vec3::splat(TILE_SIZE),
                                ..Default::default()

                            }, ..Default::default() }
                            )
                            .insert(Name::new("Clyde_Spawn"))
                            .insert(GhostSpawn{ spawn_for: GhostPersonality::Clyde, power_pellet: power_pellet_prio })

                            .id();


                        tiles.push(entity);
                        info!("Clyde is in this map!");
                    }

                    // For PacDot (aka food) creation
                    if char.to_string() == "." {

                        let texture_handle = asset_server.load("pacdot.png");
                        let atlas = TextureAtlas::from_grid(texture_handle, Vec2 { x: 2.0, y: 2.0 }, 1, 1, None, None);
                        let texture_atlas_handle = texture_atlases.add(atlas);

                        let pos = Vec3 { x: (x as f32 * TILE_SIZE * 10.0), y: -(y as f32 * TILE_SIZE) * 10.0, z: 100.0 };

                        let entity = commands.spawn(SpriteSheetBundle  {     
                            texture_atlas: texture_atlas_handle,                       
                            transform: Transform {
                                translation: pos,
                                scale: Vec3::splat(TILE_SIZE),
                                ..Default::default()
                            },
                            
                            ..Default::default()
                        }).insert(Name::new(format!("Food Tile ({})",food_num))).insert(Food { is_eaten: false, r#type: ConsumableType::PacDot })               
                        .id();

                        tiles.push(entity); food_num += 1;
                    }

                    // For PacDot (aka food) creation
                    if char.to_string() == "@" {

                        let texture_handle = asset_server.load("powerpellet.png");
                        let atlas = TextureAtlas::from_grid(texture_handle, Vec2 { x: 8.0, y: 8.0 }, 1, 1, None, None);
                        let texture_atlas_handle = texture_atlases.add(atlas);

                        let pos = Vec3 { x: (x as f32 * TILE_SIZE * 10.0), y: -(y as f32 * TILE_SIZE) * 10.0, z: 100.0 };

                        let entity = commands.spawn(SpriteSheetBundle  {     
                            texture_atlas: texture_atlas_handle,                       
                            transform: Transform {
                                translation: pos,
                                scale: Vec3::splat(TILE_SIZE),
                                ..Default::default()
                            },
                            
                            ..Default::default()
                        }).insert(Name::new(format!("PP Tile ({})",power_pellets))).insert(Food { is_eaten: false, r#type: ConsumableType::PowerPellet })               
                        .id();

                        power_pellets_data.pellets.insert(power_pellets,pos);

                        tiles.push(entity); power_pellets += 1;
                    }
                }
                y += 1;
            }

            // /* 
            commands.spawn(SpriteSheetBundle {
 
                transform: Transform {
                    translation: Vec3 { x: -329.0, y: 124.0, z: 100.0 },
                    ..Default::default()
                },
                ..Default::default()
            } )
                .insert(Name::new("Map"))
                .insert(Transform::default())
                .insert(GlobalTransform::default())
                .insert(MapEnforcer)
                .push_children(&tiles);

            info!("Created collision grid");


            // */

        }

    }
}

#[derive(Component)]
pub struct MapEnforcer;

/// Allows for Correction of the map
impl MapEnforcer {
    /// Checks and Corrects the map
    pub fn check_map(mut map: Query<(&MapEnforcer, &mut Transform)>) {
        
        let (_current_map, mut transform) = map.single_mut();

        if transform.translation != DEFAULT_MAP_ORIGIN {
            transform.translation = DEFAULT_MAP_ORIGIN;
            warn!("Made correction to map!")
        }

    }
}

#[derive(Debug, Clone, Copy, Reflect, PartialEq)]
pub enum WallType {
    Blocking, Teleport {destination: Vec3}, Gate
}


#[derive(Component, )]
pub struct SpawnPoint;

#[derive(Component, Reflect)]
pub struct TileCollider{
    pub r#type: WallType,
}

// For Food Creation
#[derive(Component, Reflect)]
pub struct Food {
    pub is_eaten: bool,
    pub r#type: ConsumableType
}

#[derive(Debug, Clone, Copy, Reflect, PartialEq)]
pub enum ConsumableType {
    PacDot, PowerPellet, Fruit
}

#[derive(Debug, )]
pub struct FoodSystem;

impl Plugin for FoodSystem {
    fn build(&self, app: &mut App) {
        app.add_system(Self::check_food)
        .add_system(Self::level_checker)
        
        
        ;
    }
}

impl FoodSystem {
    pub fn _checks() {

    }

    /// Makes the game change level if all food is gone
    pub fn level_checker (
        mut food: Query<(&mut Food, &mut Visibility), (Without<PlayerMovement>)>,
        mut player: Query<(&mut Transform, &mut PlayerData, &mut GameData, &mut MovementHelper), (With<PlayerMovement>, Without<Food>)>,
        mut ghosts: Query<&mut Ghost, Without<PlayerMovement>>,
    ) {

        // info!("Level checking!");

        let mut active_food = 0;
        let mut inactive_food = 0;
        for (mut food_data, mut visability) in &mut food {
            if *visability == Visibility::Hidden  && food_data.is_eaten {
                inactive_food += 1;
            } else {
                active_food += 1;
            }
        }

        let (player_transform, mut player_data, mut game_data, mut helper) = player.single_mut();

        // Do checks to ensure the game is ready to move on!
        if game_data.transitioning {
            let mut ready_ghosts: usize = 0;
            let all_ghosts = ghosts.iter().len();
            for (mut ghost) in &mut ghosts {
                if ghost.state == EntityState::Created {
                    ready_ghosts += 1;
                } else {
                    ghost.state = EntityState::Created;
                }
            }

            if ready_ghosts == all_ghosts {
                info!("All Ghosts Ready!");
                game_data.transitioning = false;
            } else {
                // info!("{}/{} Ghost Ready", ready_ghosts, all_ghosts);
            }
        }

        if active_food == 0 && !game_data.transitioning {
            info!("All Food is eaten! Changing Level");
            game_data.transitioning = true;
            game_data.level += 1;
            helper.direction = Some(CurrentDirection::Idle);
            player_data.state = EntityState::Created;
            for (mut food_data, mut visability) in &mut food {
                food_data.is_eaten = false;
                *visability = Visibility::Visible;
            }

            helper.direction = None;
            // Do checks to ensure the game is ready to move on!
            let mut ready_ghosts: usize = 0;
            let all_ghosts = ghosts.iter().len();
            for (ghost) in &ghosts {
                if ghost.state == EntityState::Created {
                    ready_ghosts += 1;
                }
            }

            if ready_ghosts == all_ghosts {
                game_data.transitioning = false;
                info!("Level Transitidion");
            } else {
                game_data.transitioning = true;
            }
            
        }
    }

    pub fn check_food(
        // mut commands: Commands,
        mut food: Query<(&mut Food, &mut Transform, &mut Visibility), (Without<PlayerMovement>)>,
        mut player: Query<(&mut Transform, &mut GameData, &mut PlayerData), (With<PlayerMovement>, Without<Food>)>
    ) {
        
        let (player_transform, mut game_data,mut player_data) = player.single_mut();

        for (mut food_data, food_transform, mut visability) in &mut food {

            match food_data.r#type {
                ConsumableType::PacDot | ConsumableType::PowerPellet => {
                    let is_powerpellet = food_data.r#type == ConsumableType::PowerPellet;

                    if game_data.transitioning { break }

                    if *visability == Visibility::Hidden || food_data.is_eaten { continue; }
        
                    let collided = collide(player_transform.translation, Vec2::splat(TILE_SIZE), get_real_pos(food_transform.translation, DEFAULT_MAP_ORIGIN), Vec2::splat(TILE_SIZE*20.0)).is_some();
                    if !collided { continue; }
                
                    player_data.score += if is_powerpellet { POWER_PELLET_WORTH as usize } else { PAC_DOT_WORTH as usize };
                    food_data.is_eaten = true;
                    *visability = Visibility::Hidden;

                    if is_powerpellet { player_data.state = EntityState::Energized(ENERGIZED_MAX_LENGTH); debug!("Effect Start"); };
                    

                },
                
                ConsumableType::Fruit => todo!(),
            }


        }
    }
}

