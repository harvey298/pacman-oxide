use bevy::prelude::*;
// use bevy_inspector_egui::;

const STEP_SIZE: f32 = 1.0;
pub const TILE_SIZE: f32 = 2.5;
pub const GHOST_DEBUFF: f32 = 0.5;
pub const ENERGIZED_GHOST_DEBUFF: f32 = 1.0;

/// In ticks
/// 1 second = 66.67 ticks
pub const ENERGIZED_MAX_LENGTH: u64 = 1333;

pub const PINKY_LEAVE_TIME: u64 = 300;
pub const INKY_LEAVE_TIME: u64 = 600;
pub const CLYDE_LEAVE_TIME: u64 = 900;

pub mod player;
pub mod enemy;
pub mod map;
pub mod util;

pub const EXTRA_LIFE_SCORE_THRESHOLD: usize = 10000;

/// PacDot Worth (in pts)
pub const PAC_DOT_WORTH: i64 = 10;
pub const POWER_PELLET_WORTH: i64 = 50;

pub fn camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    info!("Spawned Camera");
}


#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(Timer);

#[derive(Component, Reflect)]
pub struct AnimationDescriptor {
    /// Allows the main animation controller to control the animation
    pub main_controller: bool,
    pub reset_on_idle: bool,
    pub manual: ManualAnimationControl
}

#[derive(Clone, Copy, Debug, Reflect)]
pub struct ManualAnimationControl {
    pub enable: bool,
    pub max_index: usize,
    pub index: usize,
    pub current_index: Option<usize>
}

#[derive(Component)] // Reflect
pub struct MovementHelper {
    direction: Option<CurrentDirection>
}

// impl FromReflect for MovementHelper {
//     // fn from_reflect(_: &dyn Reflect) -> Self {
//     //     Self {
//     //         direction: None
//     //     }
//     // }

//     fn take_from_reflect(reflect: Box<dyn Reflect>) -> Result<Self, Box<dyn Reflect>> {
//         match reflect.take::<Self>() {
//             Ok(value) => Ok(value),
//             Err(value) => match Self::from_reflect(value.as_ref()) {
//                 None => Err(value),
//                 Some(value) => Ok(value),
//             },
//         }
//     }

//     fn from_reflect(reflect: &dyn Reflect) -> Option<Self> {
//         todo!()
//     }
// }

/// Alive = Alive entity
/// Dead = Dead entity, is dead the entity state will change to Respawning during an animation
/// Respawning = an entity thats respawning, this is when animations play, changes to Alive when animations have completed
/// Created = an entity just created by a system, players this just has the same effect as Respawning does but without animations, also prevents any movement, changes to Alive
#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub enum EntityState {
    Alive, Dead, Respawning, Created, Energized(u64)
}

#[derive(Debug, Clone, Copy, Reflect, PartialEq, Eq, PartialOrd, Ord)]
pub enum CurrentDirection {
    Up, Down, Left, Right, Idle
}

impl CurrentDirection {
    pub fn opposite(&self) -> Self {
        match self {
            CurrentDirection::Up => CurrentDirection::Down,
            CurrentDirection::Down => CurrentDirection::Up,
            CurrentDirection::Left => CurrentDirection::Right,
            CurrentDirection::Right => CurrentDirection::Left,
            CurrentDirection::Idle => CurrentDirection::Idle,
        }
    }
}

#[derive(Debug, )]
pub struct GameController;

impl Plugin for GameController {
    fn build(&self, app: &mut App) {
        app
            .add_system(Self::animation_controller)

        ;
    }
}

impl GameController {
    pub fn animation_controller(mut entities: Query<(&mut AnimationTimer, &MovementHelper, &mut TextureAtlasSprite, &mut AnimationDescriptor)>, time: Res<Time>,) {

        for (mut timer, helper, mut sprite, mut desc) in &mut entities {
            timer.tick(time.delta());

            if timer.just_finished() && desc.main_controller {
                                
                match helper.direction {
                    Some(o) => {
                        
                        match o {
                            CurrentDirection::Up => { sprite.index = ( if sprite.index == 4 { sprite.index + 1 } else if sprite.index == 5 { sprite.index - 1 } else { 4 }  ) % 6 },
                            CurrentDirection::Down => { sprite.index = ( if sprite.index == 6 { sprite.index + 1 } else if sprite.index == 7 { sprite.index - 1 } else { 6 }  ) % 8 },
                            CurrentDirection::Left => { sprite.index = ( if sprite.index == 2 { sprite.index + 1 } else if sprite.index == 3 { sprite.index - 1 } else { 2 }  ) % 4 },
                            CurrentDirection::Right => { sprite.index = (sprite.index + 1) % 2 },
                            CurrentDirection::Idle => { if desc.reset_on_idle { sprite.index = 0 } },

                        }

                    },
                    None => {},
                }

            }

            if desc.manual.enable {
                let mut man = desc.manual;
                if man.max_index > man.index {
                    man.index = 0
                } else {
                    sprite.index = man.index;
                    man.current_index = Some(sprite.index);
                }
            }
        }
    }
}