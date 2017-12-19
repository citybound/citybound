#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate compact;
#[macro_use]
extern crate compact_macros;
extern crate kay;
extern crate monet;
extern crate descartes;
#[macro_use]
extern crate imgui;
extern crate imgui_sys;
extern crate imgui_glium_renderer;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate app_dirs;

pub mod user_interface;
pub mod geometry;
pub mod environment;
pub mod combo;
pub mod camera_control;

pub use user_interface::{UserInterface, UserInterfaceID, Interactable3d, Interactable3dID,
                         Event3d, UserInterfaceLayer, Interactable2d, Interactable2dID, setup};
