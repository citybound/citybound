use kay::{ActorSystem, External, World, Actor};
use compact::CVec;
use planning::plan_manager::{PlanManager, PlanManagerID, Intent, IntentProgress};
use descartes::{P2, Into2d, RoughlyComparable, Path, SimpleShape, Segment, Band};
use stagemaster::combo::{Bindings, Combo2};
use stagemaster::geometry::{AnyShape, CPath, CShape, band_to_geometry};
use monet::{RendererID, Renderable, RenderableID, Geometry, Instance};
use stagemaster::{UserInterfaceID, Event3d, Interactable3d, Interactable3dID, Interactable2d,
                  Interactable2dID};
use imgui::ImGuiSetCond_FirstUseEver;
use super::super::{ZonePlanAction, Zone, ZoneMeaning, LandUse};


#[derive(Compact, Clone)]
pub struct ZoneInteraction {
    canvas: ZoneCanvasID,
}

impl ZoneInteraction {
    pub fn init(
        world: &mut World,
        renderer_id: RendererID,
        user_interface: UserInterfaceID,
        id: PlanManagerID,
    ) -> ZoneInteraction {
        ZoneInteraction { canvas: ZoneCanvasID::spawn(user_interface, id, world) }
    }
}

#[derive(Compact, Clone)]
pub struct ZoneCanvas {
    id: ZoneCanvasID,
    points: CVec<P2>,
    shape_finished: bool,
    plan_manager: PlanManagerID,
}

impl ZoneCanvas {
    pub fn spawn(
        id: ZoneCanvasID,
        user_interface: UserInterfaceID,
        plan_manager: PlanManagerID,
        world: &mut World,
    ) -> ZoneCanvas {
        user_interface.add(
            ::ui_layers::ZONE_LAYER,
            id.into(),
            AnyShape::Everywhere,
            1,
            world,
        );
        user_interface.add_2d(id.into(), world);
        ZoneCanvas {
            id,
            points: CVec::new(),
            plan_manager,
            shape_finished: false,
        }
    }
}

const FINISH_STROKE_TOLERANCE: f32 = 5.0;

impl Interactable3d for ZoneCanvas {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        if let Event3d::DragStarted { at, .. } = event {
            let new_point = at.into_2d();
            let maybe_first_point = self.points.first().cloned();

            self.shape_finished = if let Some(first_point) = maybe_first_point {
                new_point.is_roughly_within(first_point, FINISH_STROKE_TOLERANCE)
            } else {
                false
            };

            if !self.shape_finished {
                self.points.push(new_point);
            }
        }
    }
}

impl Renderable for ZoneCanvas {
    fn setup_in_scene(&mut self, _: RendererID, scene_id: usize, world: &mut World) {}

    fn render_to_scene(
        &mut self,
        renderer_id: RendererID,
        scene_id: usize,
        frame: usize,
        world: &mut World,
    ) {
        let geometry = if self.points.len() >= 2 {
            let mut points = self.points.clone();
            if self.shape_finished {
                points.push(self.points.first().cloned().expect("Should have one point"));
            }

            let path = CPath::new(
                points
                    .windows(2)
                    .map(|window| {
                        Segment::line(window[0], window[1]).expect("Should be valid line")
                    })
                    .collect::<Vec<_>>(),
            );
            let band = Band::new(path, 0.4);
            band_to_geometry(&band, 1.0)
        } else {
            Geometry::empty()
        };

        renderer_id.update_individual(
            scene_id,
            40_000,
            geometry,
            Instance::with_color([0.0, 0.0, 0.0]),
            true,
            world,
        )
    }
}

impl Interactable2d for ZoneCanvas {
    fn draw(&mut self, world: &mut World, ui: &::imgui::Ui<'static>) {
        if self.shape_finished {
            ui.window(im_str!("Zone Settings"))
                .size((200.0, 50.0), ImGuiSetCond_FirstUseEver)
                .collapsible(false)
                .build(|| {
                    if ui.small_button(im_str!("Residential")) {
                        let mut points = self.points.clone();
                        points.push(self.points.first().cloned().expect("Should have one point"));
                        self.plan_manager.change_intent(
                            Intent::ZoneIntent(ZonePlanAction::Add(Zone {
                                meaning: ZoneMeaning::LandUse(LandUse::Residential),
                                shape: CShape::new(CPath::new(
                                    points
                                        .windows(2)
                                        .map(|window| {
                                            Segment::line(window[0], window[1]).expect(
                                                "should be a valid line",
                                            )
                                        })
                                        .collect::<Vec<_>>(),
                                )),
                            })),
                            IntentProgress::Finished,
                            world,
                        );
                    }
                    if ui.small_button(im_str!("Commercial")) {}
                    if ui.small_button(im_str!("Industrial")) {}
                    if ui.small_button(im_str!("Agricultural")) {}
                    if ui.small_button(im_str!("Recreational")) {}
                    if ui.small_button(im_str!("Official")) {}
                });
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<ZoneCanvas>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;