use kay::{World, TypedID, Actor};
use compact::CDict;
use monet::RendererID;
use super::{PlanManager, PlanManagerID};

use monet::{Renderable, RenderableID};

use transport::transport_planning_old::plan_manager::rendering as road_rendering;
use land_use::buildings::rendering as building_rendering;

impl Renderable for PlanManager {
    fn setup_in_scene(&mut self, _renderer_id: RendererID, _scene_id: usize, _: &mut World) {}

    fn render_to_scene(
        &mut self,
        renderer_id: RendererID,
        scene_id: usize,
        _frame: usize,
        world: &mut World,
    ) {
        if self.preview_rendered_in.get(renderer_id).is_none() {
            let origin_machine = self.id.as_raw().machine;
            let preview = if self.preview.is_none() {
                self.preview_rendered_in = CDict::new();
                self.update_preview(world)
            } else {
                self.preview.as_ref().unwrap()
            };
            road_rendering::render_strokes(
                origin_machine.0,
                &preview.plan_delta.roads,
                renderer_id,
                scene_id,
                world,
            );
        }
        if !self.interactables_valid {
            self.update_interactables(world);
        }
        if let Some(ref result_delta) = *self.preview_result_delta {
            // TODO: add something like prepare-render to monet to make sure
            // we have new state in time
            if self.preview_result_delta_rendered_in
                .get(renderer_id)
                .is_none()
            {
                self.preview_result_delta_rendered_in.insert(
                    renderer_id,
                    (),
                );

                road_rendering::render_trimmed_strokes(
                    &result_delta.roads,
                    renderer_id,
                    scene_id,
                    world,
                );
                road_rendering::render_intersections(
                    &result_delta.roads,
                    renderer_id,
                    scene_id,
                    world,
                );
                road_rendering::render_transfer_lanes(
                    &result_delta.roads,
                    renderer_id,
                    scene_id,
                    world,
                );
            }

            // TODO: render this more seldomly
            // TODO: not that nice to have to use local_first here
            building_rendering::BuildingRenderer::local_first(world)
                .update_buildings_to_be_destroyed(
                    renderer_id,
                    scene_id,
                    result_delta.buildings.clone(),
                    world,
                );
        }

        road_rendering::render_selections(
            &self.preview.as_ref().unwrap().selections,
            &self.preview.as_ref().unwrap().plan_delta.roads,
            &self.materialized_view,
            renderer_id,
            scene_id,
            world,
        );
    }
}



mod kay_auto;
pub use self::kay_auto::*;
