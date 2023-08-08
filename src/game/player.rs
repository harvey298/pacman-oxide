use bevy::{prelude::*}; // , render::texture::ImageSettings
// use bevy_inspector_egui::Inspectable;

use crate::game::{STEP_SIZE, util::check_collosion, AnimationDescriptor, ManualAnimationControl};

use super::{AnimationTimer, MovementHelper, TILE_SIZE, map::{TileCollider, DEFAULT_MAP_ORIGIN, SpawnPoint}, EntityState, util::{get_real_pos, calculate_next_step}, EXTRA_LIFE_SCORE_THRESHOLD, CurrentDirection, enemy::Ghost};

pub struct Player;

impl Plugin for Player {
    fn build(&self, app: &mut App) {
        app
        .add_startup_system(Player::new)
        // .insert_resource(ImageSettings::default_nearest())
        .add_system(PlayerMovement::tick)
        .add_system(PlayerMovement::r#move)
        .add_system(Player::state_checks)
        .add_system(Player::player_checks)
        
        ;
    }
}

impl Player {
    pub fn new(mut commands: Commands, asset_server: Res<AssetServer>, mut texture_atlases: ResMut<Assets<TextureAtlas>>,) {
        let texture_handle = asset_server.load("sprites.png");
        let atlas = TextureAtlas::from_grid(texture_handle, Vec2 { x: 15.0, y: 15.0 }, 2, 4, None, None);
        let texture_atlas_handle = texture_atlases.add(atlas);

        commands.spawn(SpriteSheetBundle  {
            texture_atlas: texture_atlas_handle,
             
            transform: Transform {
                translation: Vec3 { x: 0.0, y: 0.0, z: 900.0 },
                scale: Vec3::splat(TILE_SIZE),
                ..Default::default()
            },
            
            ..Default::default()
        }).insert(Name::new("player"))
            .insert(PlayerMovement)
            .insert(AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)))
            .insert(MovementHelper{ direction: None })
            // still 3 lives
            .insert(PlayerData { lives: 2, score: 0, state: EntityState::Created, extra_life_given: false })
            .insert(GameData{ level: 0, transitioning: false })
            .insert(AnimationDescriptor{ main_controller: true, reset_on_idle: true, manual: ManualAnimationControl{ max_index: 8, index: 0, current_index: None, enable: false } })

            ;

        info!("Player Ready!");
    }

    pub fn state_checks(
        mut player: Query<(&PlayerMovement, &mut Transform, &mut MovementHelper, &mut PlayerData)>,
        mut spawn: Query<(&Transform), (Without<PlayerData>, With<SpawnPoint>)>
    ) {
        
        let (player, mut player_transform,mut helper, mut data) = player.single_mut();

        match data.state {
            EntityState::Alive => {},
            EntityState::Dead => {
                data.lives -= 1;

                if data.lives == 0 {
                    todo!("restart & score system");
                } else {
                    data.state = EntityState::Respawning
                }

            },

            EntityState::Respawning => {
                let spawn = spawn.single();
                player_transform.translation = get_real_pos(spawn.translation, DEFAULT_MAP_ORIGIN);
                warn!("No Animation attached!");
                helper.direction = Some(CurrentDirection::Idle);
                data.state = EntityState::Alive
            },

            EntityState::Created => {
                let spawn = spawn.single();
                player_transform.translation = get_real_pos(spawn.translation, DEFAULT_MAP_ORIGIN);
                data.state = EntityState::Alive;
            },
            EntityState::Energized(time) => {

                if time - 1 == 0 {
                    debug!("Effect Over");
                    data.state = EntityState::Alive
                } else 
                

                {data.state = EntityState::Energized(time - 1);}

            },
        }
    }

    /// Checks Player Data for events
    pub fn player_checks( mut player: Query<(&PlayerMovement, &mut Transform, &mut MovementHelper, &mut PlayerData)>,) {
        let (player, mut player_transform,mut helper, mut data) = player.single_mut();

        if data.lives == 0 {
            todo!("Game Restart")
        }

        if data.score >= EXTRA_LIFE_SCORE_THRESHOLD && !data.extra_life_given {
            data.lives += 1;
            data.extra_life_given = true;   
        }
        

    }
    
}



#[derive(Component, Reflect)]
pub struct PlayerMovement;

impl PlayerMovement {
    pub fn r#move(
        mut player: Query<(&PlayerMovement, &mut Transform, &mut MovementHelper)>,
        keyboard: Res<Input<KeyCode>>
    ) {
        let (player, mut transform, mut MovementHelper) = player.single_mut();

        if keyboard.pressed(KeyCode::W) || keyboard.pressed(KeyCode::Up) {
            MovementHelper.direction = Some(CurrentDirection::Up)
        }

        if keyboard.pressed(KeyCode::S) || keyboard.pressed(KeyCode::Down) {
            MovementHelper.direction = Some(CurrentDirection::Down)
        }

        if keyboard.pressed(KeyCode::A) || keyboard.pressed(KeyCode::Left) {
            MovementHelper.direction = Some(CurrentDirection::Left)
        }

        if keyboard.pressed(KeyCode::D) || keyboard.pressed(KeyCode::Right) {
            MovementHelper.direction = Some(CurrentDirection::Right)
        }

        if keyboard.pressed(KeyCode::Escape) {
            debug!("Pausing");
            MovementHelper.direction = Some(CurrentDirection::Idle)
        }

    }

    pub fn tick(time: Res<Time>, texture_atlases: Res<Assets<TextureAtlas>>,
    mut collision_checker: Query<(&Transform, &TileCollider), Without<PlayerMovement>>,
    wall: Query<(&Transform, &TileCollider), (Without<PlayerMovement>, Without<AnimationDescriptor>, Without<MovementHelper>, Without<Ghost>)>,
    mut query: Query<(
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
        &Handle<TextureAtlas>,
        &PlayerMovement, 
        &mut Transform,
        &mut MovementHelper,
        &mut PlayerData
    )>,) {

        for (mut timer, mut sprite, texture_atlas_handle, movement, mut transform, mut helper, mut player_data) in &mut query {
            
            match helper.direction {
                Some(o) => {
                    // println!("Modiying Direction: {:?}",o);
                    match o {
                        CurrentDirection::Up => {
                            let raw_target = calculate_next_step(false);
                            // T/t = Tiles per tick
                            let target = transform.translation + Vec3::new(0.0, raw_target, 0.0);
                            let touching = !check_collosion(target, &wall, DEFAULT_MAP_ORIGIN);
                            if touching { transform.translation = target }
                        },
                        CurrentDirection::Down => {
                            let raw_target = calculate_next_step(false);
                            // T/t = Tiles per tick
                            let target = transform.translation - Vec3::new(0.0, raw_target, 0.0);
                            let touching = !check_collosion(target, &wall, DEFAULT_MAP_ORIGIN);
                            if touching { transform.translation = target }

                        },
                        CurrentDirection::Left => {
                            let raw_target = calculate_next_step(false);
                            // T/t = Tiles per tick
                            let target = transform.translation - Vec3::new(raw_target, 0.0, 0.0);
                            let touching = !check_collosion(target, &wall, DEFAULT_MAP_ORIGIN);
                            if touching { transform.translation = target }

                        },
                        CurrentDirection::Right => {
                            let raw_target = calculate_next_step(false);
                            // T/t = Tiles per tick
                            let target = transform.translation + Vec3::new(raw_target, 0.0, 0.0);
                            let touching = !check_collosion(target, &wall, DEFAULT_MAP_ORIGIN);
                            if touching { transform.translation = target }
                        },
                        CurrentDirection::Idle => {}, // Don't move!
                    }
                },
                None => {helper.direction = Some(CurrentDirection::Left) },
            }

        }

    }
}

#[derive(Component, Reflect)]
pub struct PlayerData {
    pub lives: u64,
    pub score: usize,
    pub state: EntityState,
    pub extra_life_given: bool,
}

#[derive(Component, Reflect)]
pub struct GameData {
    pub level: usize,
    pub transitioning: bool,
}