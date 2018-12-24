use kay::{World, Fate, ActorSystem, TypedID};
use compact::CVec;
use descartes::{P2, RoughEq, AreaError};
use construction::{Constructable, ConstructableID, ConstructionID};
use planning::{Prototype, PrototypeID, PrototypeKind, PlanHistory, PlanResult, PlanManagerID,
Project, Plan, Gesture, GestureID, GestureIntent};
use transport::transport_planning::RoadPrototype;
use land_use::zone_planning::{LotPrototype, LotOccupancy};
use land_use::buildings::BuildingStyle;
use land_use::buildings::architecture::footprint_area;
use util::random::{seed, Rng};
use noise::{NoiseFn, BasicMulti, Seedable, MultiFractal};

pub mod ui;
use self::ui::VegetationUIID;

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[repr(u8)]
pub enum VegetationType {
    Shrub,
    Bush,
    SmallTree,
    MediumTree,
    LargeTree,
}

const VEGETATION_TYPES: [VegetationType; 5] = [
    VegetationType::Shrub,
    VegetationType::Bush,
    VegetationType::SmallTree,
    VegetationType::MediumTree,
    VegetationType::LargeTree,
];

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct PlantPrototype {
    pub vegetation_type: VegetationType,
    pub position: P2,
}

impl PlantPrototype {
    pub fn construct(
        &self,
        _self_id: PrototypeID,
        report_to: ConstructionID,
        world: &mut World,
    ) -> CVec<ConstructableID> {
        let id = PlantID::spawn(self.clone(), world).into();
        report_to.action_done(id, world);
        vec![id].into()
    }

    pub fn morphable_from(&self, other_plant_proto: &Self) -> bool {
        other_plant_proto.position.rough_eq_by(self.position, 0.5)
            && other_plant_proto.vegetation_type == self.vegetation_type
    }
}

#[derive(Compact, Clone, Serialize, Deserialize, Debug)]
pub enum PlantIntent {
    Individual(PlantPrototype),
    NaturalGrowth,
}

#[derive(Compact, Clone)]
pub struct Plant {
    id: PlantID,
    proto: PlantPrototype,
}

impl Plant {
    pub fn spawn(id: PlantID, proto: PlantPrototype, world: &mut World) -> Plant {
        VegetationUIID::global_broadcast(world).on_plant_spawned(id, proto.clone(), world);
        Plant { id, proto }
    }
}

impl Constructable for Plant {
    fn morph(&mut self, new_prototype: &Prototype, report_to: ConstructionID, world: &mut World) {
        if let PrototypeKind::Plant(proto) = new_prototype.kind {
            self.proto = proto;
            report_to.action_done(self.id.into(), world);
            VegetationUIID::global_broadcast(world).on_plant_destroyed(self.id, world);
            VegetationUIID::global_broadcast(world).on_plant_spawned(
                self.id,
                self.proto.clone(),
                world,
            );
        } else {
            unreachable!();
        }
    }

    fn destruct(&mut self, report_to: ConstructionID, world: &mut World) -> Fate {
        report_to.action_done(self.id.into(), world);
        VegetationUIID::global_broadcast(world).on_plant_destroyed(self.id, world);
        Fate::Die
    }
}

static mut OCC_VEG_CELLS: *mut Vec<(i32, i32)> = 0 as *mut Vec<(i32, i32)>;

pub fn calculate_prototypes(
    history: &PlanHistory,
    current_result: &PlanResult,
) -> Result<Vec<Prototype>, AreaError> {
    let mut constructed_areas = Vec::new();
    let mut prototypes = Vec::with_capacity(100000);

    for prototype in current_result.prototypes.values() {
        match *prototype {
            Prototype {
                kind: PrototypeKind::Road(RoadPrototype::PavedArea(ref area)),
                ..
            } => constructed_areas.push(area.clone()),
            Prototype {
                kind:
                    PrototypeKind::Lot(LotPrototype {
                        ref lot,
                        occupancy: LotOccupancy::Occupied(style),
                    }),
                id,
                ..
            } => {
                constructed_areas.push(footprint_area(lot, style, 5.0));
                if style == BuildingStyle::Field {
                    let boundary = lot.original_area.primitives[0].boundary.path();
                    let mut pos_along = 0.0;
                    let mut i = 0;
                    let mut rand = seed(lot.original_lot_id);

                    while pos_along < boundary.length() {
                        i += 1;
                        pos_along += rand.gen_range(5.0, 60.0);
                        let vegetation_type = rand.choose(&VEGETATION_TYPES).unwrap().clone();
                        let pos = boundary.along(pos_along);
                        if let Some((_, projected_pos)) =
                            lot.area.primitives[0].boundary.path().project(pos)
                        {
                            prototypes.push(Prototype::new_with_influences(
                                (id, i),
                                PrototypeKind::Plant(PlantPrototype {
                                    vegetation_type,
                                    position: projected_pos,
                                }),
                                projected_pos,
                            ))
                        }
                    }
                }
            }
            _ => {}
        }
    }

    for (gesture_id, versioned_gesture) in history.gestures.pairs() {
        if let GestureIntent::Plant(ref plant_intent) = versioned_gesture.0.intent {
            match plant_intent {
                PlantIntent::Individual(proto) => prototypes.push(Prototype::new_with_influences(
                    gesture_id,
                    PrototypeKind::Plant(proto.clone()),
                    proto.position,
                )),
                PlantIntent::NaturalGrowth => {
                    let mut positions = Vec::new();
                    let mut prototypes_before_difference = Vec::new();

                    if unsafe { OCC_VEG_CELLS == 0 as *mut Vec<(i32, i32)> } {
                        let mut multi_noise = BasicMulti::new()
                            .set_seed(gesture_id.0.as_fields().0)
                            .set_octaves(9)
                            .set_persistence(0.98);

                        let cells = (-50..50)
                            .into_iter()
                            .flat_map(|x_cell| {
                                (-50..50)
                                    .into_iter()
                                    .filter_map(|y_cell| {
                                        if multi_noise
                                            .get([x_cell as f64 / 50.0, y_cell as f64 / 50.0])
                                            > 0.12
                                        {
                                            Some((x_cell, y_cell))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>();

                        unsafe { OCC_VEG_CELLS = Box::into_raw(Box::new(cells)) }
                    }

                    let occ_vec_cells = unsafe { &(*OCC_VEG_CELLS) };

                    for (x_cell, y_cell) in occ_vec_cells {
                        let mut position = [*x_cell as f32 * 10.0, *y_cell as f32 * 10.0];
                        let mut rand = seed((x_cell, y_cell));
                        position[0] += rand.gen_range(-10.0, 10.0);
                        position[1] += rand.gen_range(-10.0, 10.0);

                        let position_p2 = P2::new(position[0], position[1]);
                        let vegetation_type = rand.choose(&VEGETATION_TYPES).unwrap();

                        positions.push(position_p2);
                        prototypes_before_difference.push(Prototype::new_with_influences(
                            (gesture_id, x_cell, y_cell),
                            PrototypeKind::Plant(PlantPrototype {
                                position: position_p2,
                                vegetation_type: *vegetation_type,
                            }),
                            position_p2,
                        ));
                    }

                    let mut winding_numbers = vec![0.0; positions.len()];

                    for area in &constructed_areas {
                        area.add_winding_numbers_batching(
                            &positions,
                            winding_numbers.as_mut_slice(),
                        );
                    }

                    prototypes.extend(
                        prototypes_before_difference
                            .into_iter()
                            .zip(winding_numbers)
                            .filter_map(|(proto, winding_number)| {
                                if winding_number.abs() < 0.01 {
                                    // Outside
                                    Some(proto)
                                } else {
                                    None
                                }
                            }),
                    )
                }
            }
        }
    }

    Ok(prototypes)
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<Plant>();
    kay_auto::auto_setup(system);
    self::ui::auto_setup(system);
}

pub fn spawn(world: &mut World, plan_manager: PlanManagerID) {
    let gestures = Some((
        GestureID::new(),
        Gesture::new(
            CVec::new(),
            GestureIntent::Plant(PlantIntent::NaturalGrowth),
        ),
    ));
    let project = Project::from_plan(Plan::from_gestures(gestures));

    plan_manager.implement_artificial_project(project, CVec::new(), world);
}

mod kay_auto;
pub use self::kay_auto::*;
