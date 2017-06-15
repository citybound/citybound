use descartes::{P2, V2, Band, Segment, WithUniqueOrthogonal, Norm, Path, Dot, RoughlyComparable};
use kay::{ActorSystem, Fate};
use kay::swarm::Swarm;
use monet::{Instance, Vertex, Renderer, SetupInScene, RenderToScene};
use stagemaster::geometry::{CPath, band_to_thing};

use super::Building;

pub fn setup(system: &mut ActorSystem) {
    system.extend::<Swarm<Building>, _>(|mut buildings_swarm| {
        buildings_swarm.on(|&SetupInScene { renderer_id, scene_id }, _, world| {
            let band_path = CPath::new(vec![Segment::arc_with_direction(P2::new(2.0, 0.0),
                                                                        V2::new(0.0, 1.0),
                                                                        P2::new(-2.0, 0.0)),
                                            Segment::arc_with_direction(P2::new(-2.0, 0.0),
                                                                        V2::new(0.0, -1.0),
                                                                        P2::new(2.0, 0.0))]);
            let building_circle = band_to_thing(&Band::new(band_path, 0.5), 0.0);
            renderer_id.add_batch(scene_id, 11111, building_circle, world);

            Fate::Live
        });
    });

    system.extend(Swarm::<Building>::subactors(|mut each_building| {
        each_building.on(|&RenderToScene { renderer_id, scene_id },
         building,
         world| {
            renderer_id.add_instance(scene_id,
                                     11111,
                                     Instance {
                                         instance_position: [building.lot.position.x,
                                                             building.lot.position.y,
                                                             0.0],
                                         instance_direction: [1.0, 0.0],
                                         instance_color: [0.0, 0.0, 0.0],
                                     },
                                     world);
            Fate::Live
        });
    }));
}