use descartes::{P2, V2, Band, Segment, Path, Circle};
use kay::{ActorSystem, Fate, World};
use kay::swarm::Swarm;
use monet::{Instance, MSG_Renderable_setup_in_scene, MSG_Renderable_render_to_scene};
use stagemaster::geometry::{CPath, band_to_thing, AnyShape};
use stagemaster::{UserInterface, AddInteractable, AddInteractable2d, RemoveInteractable2d, Event3d, DrawUI2d, Ui2dDrawn};
use imgui::ImGuiSetCond_FirstUseEver;

use super::{Building, BuildingID};

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

        each_building.on(|&DrawUI2d { ref imgui_ui, return_to }, building, world| {
            let ui = imgui_ui.steal();

            let mut opened = true;

            ui.window(im_str!("Building"))
                .size((200.0, 300.0), ImGuiSetCond_FirstUseEver)
                .collapsible(false)
                .opened(&mut opened)
                .build(|| {
                    ui.text(im_str!("Building ID: {:?}", building.id._raw_id))
                });

            world.send(return_to, Ui2dDrawn { imgui_ui: ui });

            if !opened {
                let ui_id = world.id::<UserInterface>();
                world.send(ui_id, RemoveInteractable2d(building.id._raw_id))
            }

            Fate::Live
        });

        each_building.on(|event: &Event3d, building, world| {
            if let Event3d::DragFinished { .. } = *event {
                let ui_id = world.id::<UserInterface>();
                world.send(ui_id, AddInteractable2d(building.id._raw_id));
            };
            Fate::Live
        });
    }));
}

pub fn on_add(building_id: BuildingID, pos: P2, world: &mut World) {
    let ui_id = world.id::<UserInterface>();
                world.send(ui_id, AddInteractable( building_id._raw_id, AnyShape::Circle(Circle{center: pos, radius: 5.0}), 10))
}