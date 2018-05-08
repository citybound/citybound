use kay::{World, Fate, ActorSystem};
use descartes::{SimpleShape, WithUniqueOrthogonal, Path, FiniteCurve};

use land_use::zone_planning::{Lot, BuildingIntent};
use land_use::buildings::BuildingStyle;
use land_use::buildings::architecture::ideal_lot_shape;
use economy::immigration_and_development::DevelopmentManagerID;
use itertools::{Itertools, MinMaxResult};

use construction::{ConstructionID, Constructable, ConstructableID};
use planning::Prototype;

#[derive(Compact, Clone)]
pub struct VacantLot {
    pub id: VacantLotID,
    pub lot: Lot,
}

impl Lot {
    pub fn rough_width_height(&self) -> (f32, f32) {
        let midpoints = self.shape
            .outline()
            .segments()
            .iter()
            .map(|segment| segment.midpoint())
            .collect::<Vec<_>>();
        let length_direction = (self.center_point - self.connection_point).normalize();
        let width_direction = length_direction.orthogonal();

        let length = if let MinMaxResult::MinMax(front, back) =
            midpoints.iter().minmax_by_key(|midpoint| {
                (*midpoint - self.connection_point).dot(&length_direction)
            })
        {
            (back - front).norm()
        } else {
            0.0
        };

        let width = if let MinMaxResult::MinMax(left, right) =
            midpoints.iter().minmax_by_key(|midpoint| {
                (*midpoint - self.connection_point).dot(&width_direction)
            })
        {
            (right - left).norm()
        } else {
            0.0
        };

        (width, length)
    }
}

impl VacantLot {
    pub fn spawn(id: VacantLotID, lot: &Lot, _world: &mut World) -> VacantLot {
        VacantLot { id, lot: lot.clone() }
    }

    pub fn suggest_lot(
        &mut self,
        building_style: BuildingStyle,
        requester: DevelopmentManagerID,
        world: &mut World,
    ) {
        let current_shape = self.lot.rough_width_height();
        let needed_shape = ideal_lot_shape(building_style);

        println!(
            "Trying to suggest lot for {:?}. Is: {:?} Needed: {:?}",
            building_style,
            current_shape,
            needed_shape
        );

        let width_ratio = current_shape.0 / needed_shape.0;
        let length_ratio = current_shape.1 / needed_shape.1;

        if width_ratio > 0.75 && width_ratio < 1.5 && length_ratio > 0.75 && length_ratio < 1.5 {
            requester.on_suggested_lot(
                BuildingIntent { lot: self.lot.clone(), building_style },
                world,
            )
        }
    }
}

impl Constructable for VacantLot {
    fn morph(&mut self, _: &Prototype, _report_to: ConstructionID, _world: &mut World) {
        unreachable!()
    }

    fn destruct(&mut self, report_to: ConstructionID, world: &mut World) -> Fate {
        report_to.action_done(self.id.into(), world);
        Fate::Die
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<VacantLot>();

    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
