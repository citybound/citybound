use descartes::Circle;
use compact::{CVec, CDict};
use kay::{ActorSystem, World, External, TypedID, Actor};
use monet::{RendererID, Renderable, RenderableID, GrouperID, GrouperIndividualID, Geometry,
            Instance};
use style::colors;

use super::{ZonePlan, ZoneMeaning};

#[derive(Compact, Clone)]
pub struct ZoneRenderer {
    id: ZoneRendererID,
    current_plan: ZonePlan,
    land_use_groupers: [GrouperID; 6],
}

impl ZoneRenderer {
    pub fn spawn(id: ZoneRendererID, world: &mut World) -> ZoneRenderer {
        ZoneRenderer {
            id,
            current_plan: ZonePlan::default(),
            land_use_groupers: [
                GrouperID::spawn(colors::RECREATIONAL, 6000, true, world),
                GrouperID::spawn(colors::COMMERCIAL, 6100, true, world),
                GrouperID::spawn(colors::INDUSTRIAL, 6200, true, world),
                GrouperID::spawn(colors::AGRICULTURAL, 6300, true, world),
                GrouperID::spawn(colors::RECREATIONAL, 6400, true, world),
                GrouperID::spawn(colors::OFFICIAL, 6500, true, world),
            ],
        }
    }

    pub fn update(&mut self, new_plan: &ZonePlan, world: &mut World) {
        for grouper in &mut self.land_use_groupers {
            grouper.clear(world);
        }

        for (i, zone) in new_plan.zones.iter().enumerate() {
            if let ZoneMeaning::LandUse(land_use) = zone.meaning {
                self.land_use_groupers[land_use as usize].add_frozen(
                    unsafe { ::std::mem::transmute(i) },
                    Geometry::from_shape(&zone.shape),
                    world,
                )
            }
        }
    }
}

impl Renderable for ZoneRenderer {
    fn setup_in_scene(&mut self, renderer_id: RendererID, scene_id: usize, world: &mut World) {
        for grouper in &self.land_use_groupers {
            Into::<RenderableID>::into(*grouper).setup_in_scene(renderer_id, scene_id, world);
        }
    }

    fn render_to_scene(
        &mut self,
        renderer_id: RendererID,
        scene_id: usize,
        frame: usize,
        world: &mut World,
    ) {
        for grouper in &self.land_use_groupers {
            Into::<RenderableID>::into(*grouper).render_to_scene(
                renderer_id,
                scene_id,
                frame,
                world,
            );
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<ZoneRenderer>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;