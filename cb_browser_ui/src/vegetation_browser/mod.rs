use kay::{World, ActorSystem, TypedID, RawID, External};
use environment::vegetation::{PlantID, PlantPrototype, VegetationType};
use environment::vegetation::ui::{VegetationUI, VegetationUIID};
use browser_utils::to_js_mesh;
use descartes::{P2, LinePath, ClosedLinePath, PrimitiveArea};
use michelangelo::{Sculpture, FlatSurface, Instance};
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
    tree_protos: HashMap<RawID, PlantPrototype>,
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
                        P2::new(0.0, -0.40),
                        P2::new(0.38, -0.12),
                        P2::new(0.24, 0.32),
                        P2::new(-0.24, 0.32),
                        P2::new(-0.38, -0.12),
                        P2::new(0.0, -0.40),
                    ]
                    .into(),
                )
                .and_then(ClosedLinePath::new)
                .unwrap(),
            ),
            0.0,
        );
        let (_, trunk_roots_surface) = trunk_base.extrude(0.0, 0.8);
        let (side_surface, top_surface) = trunk_base.extrude(2.0, -0.1);
        let trunk_mesh =
            Sculpture::new(vec![trunk_roots_surface.into(), side_surface.into()]).to_mesh();

        let medium_canopy_base = top_surface;
        let (medium_canopy_wall_1, medium_canopy_middle_1) = medium_canopy_base.extrude(0.8, 3.2);
        let (medium_canopy_wall_2, medium_canopy_middle_2) =
            medium_canopy_middle_1.extrude(2.4, 0.0);
        let (medium_canopy_wall_3, medium_canopy_top) = medium_canopy_middle_2.extrude(2.4, -1.6);

        let medium_canopy_mesh = Sculpture::new(vec![
            medium_canopy_wall_1.into(),
            medium_canopy_wall_2.into(),
            medium_canopy_wall_3.into(),
            medium_canopy_top.into(),
        ])
        .to_mesh();

        let (_, small_canopy_base) = medium_canopy_base.extrude(-1.0, 0.0);
        let (small_canopy_wall_1, small_canopy_middle_1) = small_canopy_base.extrude(0.8, 2.4);
        let (small_canopy_wall_2, small_canopy_middle_2) = small_canopy_middle_1.extrude(1.6, 0.0);
        let (small_canopy_wall_3, small_canopy_top) = small_canopy_middle_2.extrude(1.6, -1.6);

        let small_canopy_mesh = Sculpture::new(vec![
            small_canopy_wall_1.into(),
            small_canopy_wall_2.into(),
            small_canopy_wall_3.into(),
            small_canopy_top.into(),
        ])
        .to_mesh();

        let large_canopy_base = medium_canopy_base;
        let (large_canopy_wall_1, large_canopy_middle_1) = large_canopy_base.extrude(2.0, 5.0);
        let (large_canopy_wall_2, large_canopy_middle_2) = large_canopy_middle_1.extrude(4.0, 0.0);
        let (large_canopy_wall_3, large_canopy_top) = large_canopy_middle_2.extrude(4.0, -4.0);

        let large_canopy_mesh = Sculpture::new(vec![
            large_canopy_wall_1.into(),
            large_canopy_wall_2.into(),
            large_canopy_wall_3.into(),
            large_canopy_top.into(),
        ])
        .to_mesh();

        js! {
            window.cbReactApp.boundSetState(oldState => update(oldState, {
                vegetation: {
                    trunkMesh: {"$set": @{to_js_mesh(&trunk_mesh)}},
                    mediumCanopyMesh: {"$set": @{to_js_mesh(&medium_canopy_mesh)}},
                    smallCanopyMesh:  {"$set": @{to_js_mesh(&small_canopy_mesh)}},
                    largeCanopyMesh:  {"$set": @{to_js_mesh(&large_canopy_mesh)}},
                },
            }));
        }

        {
            PlantID::global_broadcast(world).get_render_info(id.into(), world);
        }

        BrowserVegetationUI {
            id,
            state: External::new(BrowserVegetationUINonPersistedState {
                tree_protos: HashMap::new(),
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
                .tree_protos
                .iter()
                .filter_map(|(raw_id, proto)| match proto.vegetation_type {
                    VegetationType::SmallTree
                    | VegetationType::MediumTree
                    | VegetationType::LargeTree => Some(Instance {
                        instance_position: [proto.position.x, proto.position.y, 0.0],
                        instance_direction: [
                            0.5 + 0.5 * (((raw_id.instance_id as usize % 10) as f32) / 10.0),
                            0.0,
                        ],
                        instance_color: self.state.trunk_color,
                    }),
                    _ => None,
                })
                .collect::<Vec<_>>();

            let small_canopy_instances = self
                .tree_protos
                .iter()
                .filter_map(|(raw_id, proto)| {
                    if proto.vegetation_type == VegetationType::SmallTree {
                        Some(Instance {
                            instance_position: [proto.position.x, proto.position.y, 0.0],
                            instance_direction: [
                                0.5 + 0.5 * ((raw_id.instance_id as usize % 10) / 10) as f32,
                                0.0,
                            ],
                            instance_color: self.state.canopy_color,
                        })
                    } else if proto.vegetation_type == VegetationType::Shrub {
                        Some(Instance {
                            instance_position: [proto.position.x, proto.position.y, -1.0],
                            instance_direction: [
                                0.5 + 0.5 * ((raw_id.instance_id as usize % 10) / 10) as f32,
                                0.0,
                            ],
                            instance_color: self.state.canopy_color,
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            let medium_canopy_instances = self
                .tree_protos
                .iter()
                .filter_map(|(raw_id, proto)| {
                    if proto.vegetation_type == VegetationType::MediumTree {
                        Some(Instance {
                            instance_position: [proto.position.x, proto.position.y, 0.0],
                            instance_direction: [
                                0.5 + 0.5 * ((raw_id.instance_id as usize % 10) / 10) as f32,
                                0.0,
                            ],
                            instance_color: self.state.canopy_color,
                        })
                    } else if proto.vegetation_type == VegetationType::Bush {
                        Some(Instance {
                            instance_position: [proto.position.x, proto.position.y, -2.0],
                            instance_direction: [
                                0.5 + 0.5 * ((raw_id.instance_id as usize % 10) / 10) as f32,
                                0.0,
                            ],
                            instance_color: self.state.canopy_color,
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            let large_canopy_instances = self
                .tree_protos
                .iter()
                .filter_map(|(raw_id, proto)| {
                    if proto.vegetation_type == VegetationType::LargeTree {
                        Some(Instance {
                            instance_position: [proto.position.x, proto.position.y, 0.0],
                            instance_direction: [
                                0.5 + 0.5 * ((raw_id.instance_id as usize % 10) / 10) as f32,
                                0.0,
                            ],
                            instance_color: self.state.canopy_color,
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            let trunk_instances_js: ::stdweb::web::TypedArray<f32> =
                flatten_instances(&trunk_instances).into();

            let small_canopy_instances_js: ::stdweb::web::TypedArray<f32> =
                flatten_instances(&small_canopy_instances).into();

            let medium_canopy_instances_js: ::stdweb::web::TypedArray<f32> =
                flatten_instances(&medium_canopy_instances).into();

            let large_canopy_instances_js: ::stdweb::web::TypedArray<f32> =
                flatten_instances(&large_canopy_instances).into();

            js! {
                window.cbReactApp.boundSetState(oldState => update(oldState, {
                    vegetation: {
                        trunkInstances: {"$set": @{trunk_instances_js}},
                        smallCanopyInstances: {"$set": @{small_canopy_instances_js}},
                        mediumCanopyInstances: {"$set": @{medium_canopy_instances_js}},
                        largeCanopyInstances: {"$set": @{large_canopy_instances_js}}
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
        self.state.tree_protos.insert(id.as_raw(), proto.clone());
        self.instances_current = false;
    }

    fn on_plant_destroyed(&mut self, id: PlantID, _: &mut World) {
        self.state.tree_protos.remove(&id.as_raw());
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
