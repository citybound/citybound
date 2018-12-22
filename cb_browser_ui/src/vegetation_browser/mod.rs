use kay::{World, ActorSystem, TypedID, RawID, External};
use environment::vegetation::{PlantID, PlantPrototype};
use environment::vegetation::ui::{VegetationUI, VegetationUIID};
use browser_utils::to_js_mesh;
use stdweb::serde::Serde;
use descartes::{P2, LinePath, ClosedLinePath, PrimitiveArea};
use michelangelo::{SculptLine, Sculpture, FlatSurface, Instance};
use std::collections::HashMap;
use browser_utils::{FrameListener, FrameListenerID, flatten_instances};

#[derive(Compact, Clone)]
pub struct BrowserVegetationUI {
    id: BrowserVegetationUIID,
    state: External<BrowserVegetationUINonPersistedState>,
}

impl ::std::ops::Deref for BrowserVegetationUI {
    type Target = BrowserVegetationUINonPersistedState;

    fn deref(&self) -> &BrowserVegetationUINonPersistedState {
        &self.state
    }
}

impl ::std::ops::DerefMut for BrowserVegetationUI {
    fn deref_mut(&mut self) -> &mut BrowserVegetationUINonPersistedState {
        &mut self.state
    }
}

pub struct BrowserVegetationUINonPersistedState {
    tree_positions: HashMap<RawID, P2>,
    instances_current: bool,
    trunk_color: [f32; 3],
    canopy_color: [f32; 3],
}

impl BrowserVegetationUI {
    pub fn spawn(id: BrowserVegetationUIID, world: &mut World) -> BrowserVegetationUI {
        let trunk_base = FlatSurface::from_primitive_area(
            PrimitiveArea::new(
                LinePath::new(
                    vec![
                        P2::new(0.0, 0.0),
                        P2::new(1.0, 0.0),
                        P2::new(1.0, 1.0),
                        P2::new(0.0, 0.0),
                    ]
                    .into(),
                )
                .and_then(ClosedLinePath::new)
                .unwrap(),
            ),
            0.0,
        );
        let (_, trunk_roots_surface) = trunk_base.extrude(0.0, 1.0);
        let (side_surface, top_surface) = trunk_base.extrude(2.0, -0.3);
        let trunk_mesh =
            Sculpture::new(vec![trunk_roots_surface.into(), side_surface.into()]).to_mesh();

        let canopy_base = top_surface;
        let (canopy_wall_1, canopy_middle_1) = canopy_base.extrude(1.0, 4.0);
        let (canopy_wall_2, canopy_middle_2) = canopy_middle_1.extrude(3.0, 0.0);
        let (canopy_wall_3, canopy_top) = canopy_middle_2.extrude(3.0, -2.0);

        let canopy_mesh = Sculpture::new(vec![
            canopy_wall_1.into(),
            canopy_wall_2.into(),
            canopy_wall_3.into(),
            canopy_top.into(),
        ])
        .to_mesh();

        js! {
            window.cbReactApp.boundSetState(oldState => update(oldState, {
                vegetation: {
                    trunkMesh: {"$set": @{to_js_mesh(&trunk_mesh)}},
                    canopyMesh: {"$set": @{to_js_mesh(&canopy_mesh)}}
                },
            }));
        }

        {
            PlantID::global_broadcast(world).get_render_info(id.into(), world);
        }

        BrowserVegetationUI {
            id,
            state: External::new(BrowserVegetationUINonPersistedState {
                tree_positions: HashMap::new(),
                instances_current: true,
                trunk_color: [0.0, 0.0, 0.0],
                canopy_color: [0.0, 0.0, 0.0],
            }),
        }
    }
}

impl FrameListener for BrowserVegetationUI {
    fn on_frame(&mut self, _: &mut World) {
        use ::stdweb::unstable::TryInto;

        let new_trunk_color_js: Vec<f64> = js! {
            return require("../../../src/colors").default.trunks;
        }
        .try_into()
        .unwrap();

        let new_trunk_color = [
            new_trunk_color_js[0] as f32,
            new_trunk_color_js[1] as f32,
            new_trunk_color_js[2] as f32,
        ];

        if self.trunk_color != new_trunk_color {
            self.trunk_color = new_trunk_color;
            self.instances_current = false;
        }

        let new_canopy_color_js: Vec<f64> = js! {
            return require("../../../src/colors").default.canopies;
        }
        .try_into()
        .unwrap();

        let new_canopy_color = [
            new_canopy_color_js[0] as f32,
            new_canopy_color_js[1] as f32,
            new_canopy_color_js[2] as f32,
        ];

        if self.canopy_color != new_canopy_color {
            self.canopy_color = new_canopy_color;
            self.instances_current = false;
        }

        if !self.instances_current {
            let trunk_instances = self
                .tree_positions
                .iter()
                .map(|(raw_id, pos)| Instance {
                    instance_position: [pos.x, pos.y, 0.0],
                    instance_direction: [
                        0.5 + 0.5 * (((raw_id.instance_id as usize % 10) as f32) / 10.0),
                        0.0,
                    ],
                    instance_color: self.state.trunk_color,
                })
                .collect::<Vec<_>>();

            let canopy_instances = self
                .tree_positions
                .iter()
                .map(|(raw_id, pos)| Instance {
                    instance_position: [pos.x, pos.y, 0.0],
                    instance_direction: [
                        0.5 + 0.5 * ((raw_id.instance_id as usize % 10) / 10) as f32,
                        0.0,
                    ],
                    instance_color: self.state.canopy_color,
                })
                .collect::<Vec<_>>();;

            let trunk_instances_js: ::stdweb::web::TypedArray<f32> =
                flatten_instances(&trunk_instances).into();

            let canopy_instances_js: ::stdweb::web::TypedArray<f32> =
                flatten_instances(&canopy_instances).into();

            js! {
                window.cbReactApp.boundSetState(oldState => update(oldState, {
                    vegetation: {
                        trunkInstances: {"$set": @{trunk_instances_js}},
                        canopyInstances: {"$set": @{canopy_instances_js}}
                    }
                }));

                console.log("Rebuilt vegetation");
            }

            self.instances_current = true;
        }
    }
}

impl VegetationUI for BrowserVegetationUI {
    fn on_plant_spawned(&mut self, id: PlantID, proto: &PlantPrototype, _: &mut World) {
        self.state
            .tree_positions
            .insert(id.as_raw(), proto.position);
        self.instances_current = false;
    }

    fn on_plant_destroyed(&mut self, id: PlantID, _: &mut World) {
        self.state.tree_positions.remove(&id.as_raw());
        self.instances_current = false;
    }
}

mod kay_auto;
pub use self::kay_auto::*;

pub fn setup(system: &mut ActorSystem) {
    system.register::<BrowserVegetationUI>();
    auto_setup(system);
}

pub fn spawn(world: &mut World) {
    BrowserVegetationUIID::spawn(world);
}
