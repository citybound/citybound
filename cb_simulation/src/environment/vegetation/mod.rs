use kay::{World, Fate, ActorSystem, TypedID};
use compact::CVec;
use descartes::{P2, RoughEq, AreaError, AreaEmbedding, AreaFilter, PointContainer};
use construction::{Constructable, ConstructableID, ConstructionID};
use planning::{Prototype, PrototypeID, PrototypeKind, PlanHistory, PlanResult, PlanManagerID,
Project, Plan, Gesture, GestureID, GestureIntent, VersionedGesture};
use transport::transport_planning::RoadPrototype;
use land_use::zone_planning::{LotPrototype, LotOccupancy};
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

pub fn calculate_prototypes(
    history: &PlanHistory,
    current_result: &PlanResult,
) -> Result<Vec<Prototype>, AreaError> {
    #[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
    enum ConstructedLabel {
        Paved,
        Building,
    }

    let mut constructed_areas = Vec::new();

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
                ..
            } => constructed_areas.push(footprint_area(lot, style, 5.0)),
            _ => {}
        }
    }

    let mut prototypes = Vec::with_capacity(100000);

    for (gesture_id, versioned_gesture) in history.gestures.pairs() {
        if let GestureIntent::Plant(ref plant_intent) = versioned_gesture.0.intent {
            match plant_intent {
                PlantIntent::Individual(proto) => prototypes.push(Prototype::new_with_influences(
                    gesture_id,
                    PrototypeKind::Plant(proto.clone()),
                )),
                PlantIntent::NaturalGrowth => {
                    let mut multi_noise = BasicMulti::new()
                        .set_seed(gesture_id.0.as_fields().0)
                        .set_octaves(9)
                        .set_persistence(0.98);

                    for x_cell in -200..200 {
                        for y_cell in -200..200 {
                            let mut position = [x_cell as f64 * 10.0, y_cell as f64 * 10.0];
                            let mut rand = seed((x_cell, y_cell));
                            position[0] += rand.gen_range(-10.0, 10.0);
                            position[1] += rand.gen_range(-10.0, 10.0);
                            let vegetation_type = VegetationType::LargeTree;
                            if multi_noise.get([position[0] / 500.0, position[1] / 500.0]) > 0.13 {
                                let position_p2 = P2::new(position[0] as f32, position[1] as f32);
                                if !constructed_areas
                                    .iter()
                                    .any(|area| area.contains(position_p2))
                                {
                                    prototypes.push(Prototype::new_with_influences(
                                        (gesture_id, x_cell, y_cell),
                                        PrototypeKind::Plant(PlantPrototype {
                                            position: position_p2,
                                            vegetation_type,
                                        }),
                                    ));
                                }
                            }
                        }
                    }
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
