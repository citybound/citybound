use descartes::{P2, V2, Band, Segment, Path, Circle};
use compact::CVec;
use kay::{ActorSystem, Fate, World, External};
use kay::swarm::Swarm;
use monet::{Instance, MSG_Renderable_setup_in_scene, MSG_Renderable_render_to_scene};
use stagemaster::geometry::{CPath, band_to_thing, AnyShape};
use stagemaster::{UserInterfaceID, Event3d, Interactable3d, Interactable3dID, Interactable2d,
                  Interactable2dID, MSG_Interactable3d_on_event, MSG_Interactable2d_draw_ui_2d};
use imgui::ImGuiSetCond_FirstUseEver;

use super::{Building, BuildingID};
use economy::households::HouseholdID;

#[derive(Compact, Clone)]
pub struct BuildingInspector {
    id: BuildingInspectorID,
    user_interface: UserInterfaceID,
    current_building: Option<BuildingID>,
    current_households: CVec<HouseholdID>,
    households_todo: CVec<HouseholdID>,
    return_ui_to: Option<UserInterfaceID>,
}

impl BuildingInspector {
    pub fn spawn(
        id: BuildingInspectorID,
        user_interface: UserInterfaceID,
        _: &mut World,
    ) -> BuildingInspector {
        BuildingInspector {
            id,
            user_interface,
            current_building: None,
            current_households: CVec::new(),
            households_todo: CVec::new(),
            return_ui_to: None,
        }
    }

    pub fn set_inspected_building(
        &mut self,
        building: BuildingID,
        households: &CVec<HouseholdID>,
        world: &mut World,
    ) {
        self.current_building = Some(building);
        self.current_households = households.clone();
        self.households_todo.clear();
        self.user_interface.add_2d(self.id.into(), world);
    }

    pub fn ui_drawn(&mut self, imgui_ui: &External<::imgui::Ui<'static>>, world: &mut World) {
        let ui = imgui_ui.steal();

        if let Some(household) = self.households_todo.pop() {
            household.inspect(ui, self.id, world);
        } else {
            self.return_ui_to
                .expect("Should have return to set for UI")
                .ui_drawn(ui, world);
        }
    }
}

impl Interactable2d for BuildingInspector {
    fn draw_ui_2d(
        &mut self,
        imgui_ui: &External<::imgui::Ui<'static>>,
        return_to: UserInterfaceID,
        world: &mut World,
    ) {
        let ui = imgui_ui.steal();
        self.return_ui_to = Some(return_to);

        let new_building = if let Some(building) = self.current_building {
            let mut opened = true;

            ui.window(im_str!("Building"))
                .size((200.0, 300.0), ImGuiSetCond_FirstUseEver)
                .collapsible(false)
                .opened(&mut opened)
                .build(|| {
                    ui.text(im_str!("Building ID: {:?}", building._raw_id));
                    ui.text(im_str!(
                        "# of households: {}",
                        self.current_households.len()
                    ))
                });

            self.households_todo = self.current_households.clone();
            self.return_ui_to = Some(return_to);
            self.id.ui_drawn(ui, world);

            if opened { Some(building) } else { None }
        } else {
            return_to.ui_drawn(ui, world);
            None
        };

        self.current_building = new_building;
    }
}

impl Interactable3d for Building {
    fn on_event(&mut self, event: Event3d, world: &mut World) {
        if let Event3d::DragFinished { .. } = event {
            BuildingInspectorID::local_first(world)
                .set_inspected_building(self.id, self.households.clone(), world);
        };
    }
}

pub fn setup(system: &mut ActorSystem, user_interface: UserInterfaceID) {
    // TODO: pull out into newstyle BuildingRenderer
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
    }));

    system.add(Swarm::<BuildingInspector>::new(), |_| {});
    auto_setup(system);

    BuildingInspectorID::spawn(user_interface, &mut system.world());
}

pub fn on_add(building_id: BuildingID, pos: P2, world: &mut World) {
    // TODO: not sure if correct
    UserInterfaceID::local_first(world).add(
        building_id.into(),
        AnyShape::Circle(Circle { center: pos, radius: 5.0 }),
        10,
        world,
    );
}

mod kay_auto;
pub use self::kay_auto::*;