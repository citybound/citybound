use descartes::{P2, V2, Band, Segment, Path, Circle};
use compact::CVec;
use kay::{ActorSystem, Fate, World, ID};
use kay::swarm::Swarm;
use monet::{Instance, MSG_Renderable_setup_in_scene, MSG_Renderable_render_to_scene};
use stagemaster::geometry::{CPath, band_to_thing, AnyShape};
use stagemaster::{UserInterface, AddInteractable, AddInteractable2d, Event3d, DrawUI2d, Ui2dDrawn};
use imgui::ImGuiSetCond_FirstUseEver;

use super::{Building, BuildingID};
use economy::households::HouseholdID;

#[derive(Default)]
pub struct BuildingInspector {
    pub current_building: Option<BuildingID>,
    pub current_households: CVec<HouseholdID>,
    pub households_todo: CVec<HouseholdID>,
    pub return_ui_to: Option<ID>
}

#[derive(Compact, Clone)]
pub struct SetInspectedBuilding(BuildingID, CVec<HouseholdID>);

pub fn setup(system: &mut ActorSystem) {
    system.extend::<Swarm<Building>, _>(|mut buildings_swarm| {
        buildings_swarm.on(|&MSG_Renderable_setup_in_scene(renderer_id, scene_id),
         _,
         world| {
            let band_path = CPath::new(vec![
                Segment::arc_with_direction(
                    P2::new(5.0, 0.0),
                    V2::new(0.0, 1.0),
                    P2::new(-5.0, 0.0)
                ),
                Segment::arc_with_direction(
                    P2::new(-5.0, 0.0),
                    V2::new(0.0, -1.0),
                    P2::new(5.0, 0.0)
                ),
            ]);
            let building_circle = band_to_thing(&Band::new(band_path, 2.0), 0.0);
            renderer_id.add_batch(scene_id, 11111, building_circle, world);

            Fate::Live
        });
    });

    system.extend(Swarm::<Building>::subactors(|mut each_building| {
        each_building.on(|&MSG_Renderable_render_to_scene(renderer_id, scene_id),
         building,
         world| {
            renderer_id.add_instance(
                scene_id,
                11111,
                Instance {
                    instance_position: [building.lot.position.x, building.lot.position.y, 0.0],
                    instance_direction: [1.0, 0.0],
                    instance_color: [0.8, 0.5, 0.0],
                },
                world,
            );
            Fate::Live
        });

        each_building.on(|event: &Event3d, building, world| {
            if let Event3d::DragFinished { .. } = *event {
                let inspector_id = world.id::<BuildingInspector>();
                world.send(inspector_id, SetInspectedBuilding(building.id, building.households.clone()));
            };
            Fate::Live
        });
    }));

    system.add::<BuildingInspector, _>(BuildingInspector::default(), |mut the_inspector| {
        the_inspector.on(|&DrawUI2d { ref imgui_ui, return_to }, inspector, world| {
            let ui = imgui_ui.steal();
            inspector.return_ui_to = Some(return_to);

            let new_building = if let Some(building) = inspector.current_building {
                let mut opened = true;

                ui.window(im_str!("Building"))
                    .size((200.0, 300.0), ImGuiSetCond_FirstUseEver)
                    .collapsible(false)
                    .opened(&mut opened)
                    .build(|| {
                        ui.text(im_str!("Building ID: {:?}", building._raw_id));
                        ui.text(im_str!("# of households: {}", inspector.current_households.len()))
                    });

                inspector.households_todo = inspector.current_households.clone();
                inspector.return_ui_to = Some(return_to);
                let inspector_id = world.id::<BuildingInspector>();
                world.send(inspector_id, Ui2dDrawn { imgui_ui: ui });

                if opened {
                    Some(building)
                } else {
                    None
                }
            } else {
                world.send(return_to, Ui2dDrawn { imgui_ui: ui });
                None
            };

            inspector.current_building = new_building;

            Fate::Live
        });

        the_inspector.on(|&Ui2dDrawn{ref imgui_ui}, inspector, world| {
            let ui = imgui_ui.steal();

            if let Some(household) = inspector.households_todo.pop() {
                let inspector_id = world.id::<BuildingInspector>();
                household.inspect(ui, inspector_id, world);
            } else {
                world.send(inspector.return_ui_to.expect("Should have return to set for UI"), Ui2dDrawn { imgui_ui: ui });
            }

            Fate::Live
        });

        the_inspector.on(|&SetInspectedBuilding(building, ref households), inspector, world| {
            inspector.current_building = Some(building);
            inspector.current_households = households.clone();
            inspector.households_todo.clear();

            let inspector_id = world.id::<BuildingInspector>();
            world.send_to_id_of::<UserInterface, _>(AddInteractable2d(inspector_id));

            Fate::Live
        });
    });
}

pub fn on_add(building_id: BuildingID, pos: P2, world: &mut World) {
    let ui_id = world.id::<UserInterface>();
                world.send(ui_id, AddInteractable( building_id._raw_id, AnyShape::Circle(Circle{center: pos, radius: 5.0}), 10))
}