extern crate compact;
#[macro_use]
extern crate compact_macros;
extern crate kay;
extern crate browser_monet;
use browser_monet as monet;
extern crate descartes;

pub mod user_interface;
pub mod debug;
pub mod environment;
pub mod combo;
pub mod camera_control;

pub use user_interface::{UserInterface, UserInterfaceID, Interactable3d, Interactable3dID,
Event3d, UserInterfaceLayer, Interactable2d, Interactable2dID, setup, spawn};
