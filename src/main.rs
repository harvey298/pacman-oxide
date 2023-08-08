#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::{prelude::*, log::LogPlugin};
use anyhow::Result;
use game::{player::Player, map::{self, FoodSystem}, EntityState, enemy, GameController, util::PowerPellets};
use bevy::window::WindowResolution;

use crate::data::cargo_toml::get_version;

mod game;
mod debug;
mod data;

const CLEAR: Color = Color::rgb(0.1, 0.1, 0.1);
#[cfg(debug_assertions)]
const NAME: &str = "Pacman - Rusted (debug)";

#[cfg(not(debug_assertions))]
const NAME: &str = "Pacman - Rusted";

const ALLOW_GAME_RUN: bool = true;

fn main() -> Result<()> {

    /* 
    println!("Distance 1: {}", game::util::calculate_distance(Vec3 { x: 100.0, y: 0.0, z: 0.0 }, Vec3 { x: 50.0, y: 0.0, z: 0.0 } ) );
    println!("Distance 2: {}", game::util::calculate_distance(Vec3 { x: 100.0, y: 0.0, z: 0.0 }, Vec3 { x: -40.0, y: 0.0, z: 0.0 } ) );
    println!("Distance 2: {}", game::util::calculate_distance(Vec3 { x: 100.0, y: 0.0, z: 0.0 }, Vec3 { x: 40.0, y: 0.0, z: 0.0 } ) );
    */

    // println!("{}", get_version().unwrap());

    if ALLOW_GAME_RUN {
        let mut app = App::new();

        let mut window = WindowPlugin::default();

        window.primary_window = Some( Window {
            title: NAME.to_string(),
            resolution: WindowResolution::default(),
            ..Default::default()
        } );

        let default_plugin = DefaultPlugins.build();

        #[cfg(debug_assertions)]
        let default_plugin = default_plugin.set(LogPlugin {
            level: bevy::log::Level::TRACE,
            filter: "debug,wgpu_core=warn,wgpu_hal=warn,naga=info,mygame=debug".into(),
        });

        #[cfg(not(debug_assertions))]
        let default_plugin = default_plugin.set(LogPlugin {
            level: bevy::log::Level::INFO,
            filter: "info,wgpu_core=warn,wgpu_hal=warn".into(),
        });

        let default_plugin = default_plugin.set(window);

        app
            .add_plugins(default_plugin)
            .insert_resource(ClearColor(CLEAR))
            .init_resource::<PowerPellets>()



            .add_startup_system(game::camera)
            .add_plugin(Player)
            .add_plugin(map::TileMap)
            .add_plugin(FoodSystem)
            .add_plugin(enemy::GhostPlugin)
            .add_plugin(GameController)

        
            .add_plugin(debug::DebugPlugin)
        .run();
    }

    Ok(())
}
