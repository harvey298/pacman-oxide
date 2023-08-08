use bevy::prelude::*;
// use bevy_inspector_egui::{WorldInspectorPlugin, RegisterInspectable};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use crate::game::{player::{PlayerMovement, PlayerData, GameData}, map::{TileCollider, Food}, MovementHelper, enemy::Ghost, AnimationDescriptor};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        if cfg!(debug_assertions) {
            info!("Debug Enabled");
            app.add_plugin(WorldInspectorPlugin::new())
                .register_type::<PlayerMovement>()
                .register_type::<TileCollider>()
                .register_type::<PlayerData>()
                .register_type::<GameData>()
                // .register_type::<MovementHelper>()
                .register_type::<Ghost>()
                .register_type::<AnimationDescriptor>()
                .register_type::<Food>()
                // .insert_resource(LogSettings {
                //     filter: "info,wgpu_core=warn,wgpu_hal=warn,mygame=debug".into(),
                //     level: bevy::log::Level::DEBUG,
                //     ..Default::default()
                // })
            


            ;
        }
    }
}