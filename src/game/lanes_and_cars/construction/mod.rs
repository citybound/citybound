use compact::CVec;
use kay::{ID, ActorSystem, Fate};
use kay::swarm::{Swarm, SubActor};
use descartes::{N, P2, Dot, Band, Curve, FiniteCurve, Path, RoughlyComparable, Intersect,
                WithUniqueOrthogonal};
use itertools::Itertools;
use stagemaster::geometry::CPath;
use ordered_float::OrderedFloat;

use super::lane::{Lane, TransferLane};
use super::connectivity::{Interaction, InteractionKind, OverlapKind};

pub mod materialized_reality;
use self::materialized_reality::BuildableRef;

#[derive(Compact, Clone)]
pub struct ConstructionInfo {
    pub length: f32,
    pub path: CPath,
    pub progress: f32,
    unbuilding_for: Option<ID>,
    disconnects_remaining: u8,
}

impl ConstructionInfo {
    pub fn from_path(path: CPath) -> Self {
        ConstructionInfo {
            length: path.length(),
            path: path,
            progress: 0.0,
            unbuilding_for: None,
            disconnects_remaining: 0,
        }
    }
}

#[derive(Copy, Clone)]
pub struct AdvertiseToTransferAndReport(pub ID, pub BuildableRef);

pub fn setup(system: &mut ActorSystem) {
    let all_lanes_id = system.id::<Swarm<Lane>>().broadcast();
    let all_transfer_lanes_id = system.id::<Swarm<TransferLane>>().broadcast();

    use self::materialized_reality::ReportLaneBuilt;

    system.extend(Swarm::<Lane>::subactors(move |mut each_lane| {

        each_lane.on_create_with(move |&AdvertiseToTransferAndReport(report_to, report_as),
                                       lane,
                                       world| {
            world.send(all_lanes_id,
                       Connect {
                           other_id: lane.id(),
                           other_start: lane.construction.path.start(),
                           other_end: lane.construction.path.end(),
                           other_length: lane.construction.path.length(),
                           reply_needed: true,
                       });
            world.send(all_transfer_lanes_id,
                       ConnectTransferToNormal {
                           other_id: lane.id(),
                           other_path: lane.construction.path.clone(),
                       });
            world.send(report_to, ReportLaneBuilt(lane.id(), report_as));
            super::rendering::on_build(lane, world);
            super::pathfinding::on_build(lane, world);
            Fate::Live
        });

        each_lane.on(|&AdvertiseForOverlaps { ref lanes }, lane, world| {
            for &lane_id in lanes.iter() {
                world.send(lane_id,
                           ConnectOverlaps {
                               other_id: lane.id(),
                               other_path: lane.construction.path.clone(),
                               reply_needed: true,
                           });
            }
            Fate::Live
        });

        each_lane.on(|&Connect {
                           other_id,
                           other_start,
                           other_end,
                           other_length,
                           reply_needed,
                       },
                      lane,
                      world| {
            if other_id == lane.id() {
                return Fate::Live;
            };

            let mut connected = false;

            if other_start.is_roughly_within(lane.construction.path.end(), CONNECTION_TOLERANCE) {
                connected = true;

                if !lane.connectivity
                        .interactions
                        .iter()
                        .any(|interaction| match *interaction {
                                 Interaction {
                                     partner_lane,
                                     kind: InteractionKind::Next { .. },
                                     ..
                                 } => partner_lane == other_id,
                                 _ => false,
                             }) {
                    lane.connectivity
                        .interactions
                        .push(Interaction {
                                  partner_lane: other_id,
                                  start: lane.construction.length,
                                  partner_start: 0.0,
                                  kind: InteractionKind::Next { green: false },
                              });
                }

                super::pathfinding::on_connect(lane);
            }

            if other_end.is_roughly_within(lane.construction.path.start(), CONNECTION_TOLERANCE) {
                connected = true;

                if !lane.connectivity
                        .interactions
                        .iter()
                        .any(|interaction| match *interaction {
                                 Interaction {
                                     partner_lane,
                                     kind: InteractionKind::Previous { .. },
                                     ..
                                 } => partner_lane == other_id,
                                 _ => false,
                             }) {
                    lane.connectivity
                        .interactions
                        .push(Interaction {
                                  partner_lane: other_id,
                                  start: 0.0,
                                  partner_start: other_length,
                                  kind: InteractionKind::Previous,
                              });
                }

                super::pathfinding::on_connect(lane);
            }

            if reply_needed && connected {
                world.send(other_id,
                           Connect {
                               other_id: lane.id(),
                               other_start: lane.construction.path.start(),
                               other_end: lane.construction.path.end(),
                               other_length: lane.construction.path.length(),
                               reply_needed: false,
                           });
            }

            Fate::Live
        });

        each_lane.on(|&ConnectOverlaps { other_id, ref other_path, reply_needed }, lane, world| {
            MEMOIZED_BANDS_OUTLINES.with(|memoized_bands_outlines_cell| {
                let memoized_bands_outlines = unsafe { &mut *memoized_bands_outlines_cell.get() };
                let &(ref lane_band, ref lane_outline) = memoized_bands_outlines
                    .entry(lane.id())
                    .or_insert_with(|| {
                        let band = Band::new(lane.construction.path.clone(), 4.5);
                        let outline = band.outline();
                        (band, outline)
                    }) as
                                                         &(Band<CPath>, CPath);

                let memoized_bands_outlines = unsafe { &mut *memoized_bands_outlines_cell.get() };
                let &(ref other_band, ref other_outline) = memoized_bands_outlines
                    .entry(other_id)
                    .or_insert_with(|| {
                        let band = Band::new(other_path.clone(), 4.5);
                        let outline = band.outline();
                        (band, outline)
                    }) as
                                                           &(Band<CPath>, CPath);

                let intersections = (lane_outline, other_outline).intersect();
                if intersections.len() >= 2 {
                    if let ::itertools::MinMaxResult::MinMax((entry_intersection,
                                                              entry_distance),
                                                             (exit_intersection, exit_distance)) =
                        intersections
                            .iter()
                            .map(|intersection| {
                                (intersection,
                                 lane_band.outline_distance_to_path_distance(intersection.along_a))
                            })
                            .minmax_by_key(|&(_, distance)| OrderedFloat(distance)) {
                        let other_entry_distance =
                            other_band
                                .outline_distance_to_path_distance(entry_intersection.along_b);
                        let other_exit_distance =
                            other_band.outline_distance_to_path_distance(exit_intersection.along_b);

                        let overlap_kind = if
                            other_path
                                .direction_along(other_entry_distance)
                                .is_roughly_within(lane.construction
                                                       .path
                                                       .direction_along(entry_distance),
                                                   0.1) ||
                            other_path
                                .direction_along(other_exit_distance)
                                .is_roughly_within(lane.construction
                                                       .path
                                                       .direction_along(exit_distance),
                                                   0.1) {
                            // ::stagemaster::geometry::CPath::add_debug_path(
                            //     lane.construction.path
                            //         .subsection(entry_distance, exit_distance).unwrap(),
                            //     [1.0, 0.5, 0.0],
                            //     0.3
                            // );
                            OverlapKind::Parallel
                        } else {
                            // ::stagemaster::geometry::CPath::add_debug_path(
                            //     lane.construction.path
                            //         .subsection(entry_distance, exit_distance).unwrap(),
                            //     [1.0, 0.0, 0.0],
                            //     0.3
                            // );
                            OverlapKind::Conflicting
                        };

                        lane.connectivity
                            .interactions
                            .push(Interaction {
                                      partner_lane: other_id,
                                      start: entry_distance,
                                      partner_start: other_entry_distance.min(other_exit_distance),
                                      kind: InteractionKind::Overlap {
                                          end: exit_distance,
                                          partner_end: other_exit_distance
                                              .max(other_entry_distance),
                                          kind: overlap_kind,
                                      },
                                  });
                    } else {
                        panic!("both entry and exit should exist")
                    }
                }


                if reply_needed {
                    world.send(other_id,
                               ConnectOverlaps {
                                   other_id: lane.id(),
                                   other_path: lane.construction.path.clone(),
                                   reply_needed: false,
                               });
                }
                Fate::Live
            })
        });

        each_lane.on(|&ConnectToTransfer { other_id }, lane, world| {
            world.send(other_id,
                       ConnectTransferToNormal {
                           other_id: lane.id(),
                           other_path: lane.construction.path.clone(),
                       });
            Fate::Live
        });

        each_lane.on(|&AddTransferLaneInteraction(interaction), lane, _| {
            if !lane.connectivity
                    .interactions
                    .iter()
                    .any(|existing| existing.partner_lane == interaction.partner_lane) {
                lane.connectivity.interactions.push(interaction);
                super::pathfinding::on_connect(lane);
            }
            Fate::Live
        });

        each_lane.on(|&Disconnect { other_id }, lane, world| {
            let interaction_indices_to_remove = lane.connectivity
                .interactions
                .iter()
                .enumerate()
                .filter_map(|(i, interaction)| if interaction.partner_lane == other_id {
                                Some(i)
                            } else {
                                None
                            })
                .collect::<Vec<_>>();
            // TODO: Cancel trip
            lane.microtraffic
                .cars
                .retain(|car| {
                    !interaction_indices_to_remove.contains(&(car.next_hop_interaction as usize))
                });
            lane.microtraffic
                .obstacles
                .retain(|&(_obstacle, from_id)| from_id != other_id);
            for idx in interaction_indices_to_remove.into_iter().rev() {
                lane.connectivity.interactions.remove(idx);
            }
            super::pathfinding::on_disconnect(lane, other_id);
            world.send(other_id, ConfirmDisconnect);
            Fate::Live
        });

        each_lane.on(|&Unbuild { report_to }, lane, world| {
            let mut disconnects_remaining = 0;
            for id in lane.connectivity
                    .interactions
                    .iter()
                    .map(|interaction| interaction.partner_lane)
                    .unique() {
                world.send(id, Disconnect { other_id: lane.id() });
                disconnects_remaining += 1;
            }
            super::rendering::on_unbuild(lane, world);
            MEMOIZED_BANDS_OUTLINES.with(|memoized_bands_outlines_cell| {
                let memoized_bands_outlines = unsafe { &mut *memoized_bands_outlines_cell.get() };
                memoized_bands_outlines.remove(&lane.id())
            });
            if disconnects_remaining == 0 {
                world.send(report_to, ReportLaneUnbuilt(Some(lane.id())));
                Fate::Die
            } else {
                lane.construction.disconnects_remaining = disconnects_remaining;
                lane.construction.unbuilding_for = Some(report_to);
                Fate::Live
            }
        });

        each_lane.on(|_: &ConfirmDisconnect, lane, world| {
            lane.construction.disconnects_remaining -= 1;
            if lane.construction.disconnects_remaining == 0 {
                world.send(lane.construction
                               .unbuilding_for
                               .expect("should be unbuilding"),
                           ReportLaneUnbuilt(Some(lane.id())));
                Fate::Die
            } else {
                Fate::Live
            }
        });

    }));

    system.extend(Swarm::<TransferLane>::subactors(move |mut each_t_lane| {

        each_t_lane.on_create_with(move |&AdvertiseToTransferAndReport(report_to, report_as),
                                         lane,
                                         world| {
            world.send(all_lanes_id, ConnectToTransfer { other_id: lane.id() });
            world.send(report_to, ReportLaneBuilt(lane.id(), report_as));
            super::rendering::on_build_transfer(lane, world);
            Fate::Live
        });

        each_t_lane.on(|&ConnectTransferToNormal { other_id, ref other_path }, lane, world| {
            let projections = (other_path.project(lane.construction.path.start()),
                               other_path.project(lane.construction.path.end()));
            if let (Some(lane_start_on_other_distance), Some(lane_end_on_other_distance)) =
                projections {
                if lane_start_on_other_distance < lane_end_on_other_distance &&
                   lane_end_on_other_distance - lane_start_on_other_distance > 6.0 {
                    let lane_start_on_other = other_path.along(lane_start_on_other_distance);
                    let lane_end_on_other = other_path.along(lane_end_on_other_distance);

                    if lane_start_on_other.is_roughly_within(lane.construction.path.start(), 3.0) &&
                       lane_end_on_other.is_roughly_within(lane.construction.path.end(), 3.0) {
                        world.send(other_id,AddTransferLaneInteraction(Interaction {
                                                       partner_lane: lane.id(),
                                                       start: lane_start_on_other_distance,
                                                       partner_start: 0.0,
                                                       kind: InteractionKind::Overlap {
                                                           end: lane_start_on_other_distance +
                                                                lane.construction.length,
                                                           partner_end: lane.construction.length,
                                                           kind: OverlapKind::Transfer,
                                                       },
                                                   }));

                        let mut distance_covered = 0.0;
                        let distance_map = lane.construction
                            .path
                            .segments()
                            .iter()
                            .map(|segment| {
                                distance_covered += segment.length();
                                let segment_end_on_other_distance =
                                    other_path
                                        .project(segment.end())
                                        .expect("should contain transfer lane segment end");
                                (distance_covered,
                                 segment_end_on_other_distance - lane_start_on_other_distance)
                            })
                            .collect();

                        let other_is_right =
                            (lane_start_on_other - lane.construction.path.start())
                                .dot(&lane.construction.path.start_direction().orthogonal()) >
                            0.0;

                        if other_is_right {
                            lane.connectivity.right = Some((other_id,
                                                            lane_start_on_other_distance));
                            lane.connectivity.right_distance_map = distance_map;
                        } else {
                            lane.connectivity.left = Some((other_id, lane_start_on_other_distance));
                            lane.connectivity.left_distance_map = distance_map;
                        }
                    }
                }
            }
            Fate::Live
        });

        each_t_lane.on(|&Disconnect { other_id }, lane, world| {
            lane.connectivity.left = lane.connectivity
                .left
                .and_then(|(left_id, left_start)| if left_id == other_id {
                              None
                          } else {
                              Some((left_id, left_start))
                          });
            lane.connectivity.right = lane.connectivity
                .right
                .and_then(|(right_id, right_start)| if right_id == other_id {
                              None
                          } else {
                              Some((right_id, right_start))
                          });
            world.send(other_id, ConfirmDisconnect);
            Fate::Live
        });

        each_t_lane.on(|&Unbuild { report_to }, lane, world| {
            if let Some((left_id, _)) = lane.connectivity.left {
                world.send(left_id, Disconnect { other_id: lane.id() });
            }
            if let Some((right_id, _)) = lane.connectivity.right {
                world.send(right_id, Disconnect { other_id: lane.id() });
            }
            super::rendering::on_unbuild_transfer(lane, world);
            if lane.connectivity.left.is_none() && lane.connectivity.right.is_none() {
                world.send(report_to, ReportLaneUnbuilt(Some(lane.id())));
                Fate::Die
            } else {
                lane.construction.disconnects_remaining = lane.connectivity
                    .left
                    .into_iter()
                    .chain(lane.connectivity.right)
                    .count() as u8;
                lane.construction.unbuilding_for = Some(report_to);
                Fate::Live
            }
        });

        each_t_lane.on(|_: &ConfirmDisconnect, lane, world| {
            lane.construction.disconnects_remaining -= 1;
            if lane.construction.disconnects_remaining == 0 {
                world.send(lane.construction
                               .unbuilding_for
                               .expect("should be unbuilding"),
                           ReportLaneUnbuilt(Some(lane.id())));
                Fate::Die
            } else {
                Fate::Live
            }
        })
    }));

    self::materialized_reality::setup(system);
}

#[derive(Compact, Clone)]
pub struct AdvertiseForOverlaps {
    lanes: CVec<ID>,
}

const CONNECTION_TOLERANCE: f32 = 0.1;

#[derive(Copy, Clone)]
pub struct Connect {
    other_id: ID,
    other_start: P2,
    other_end: P2,
    other_length: N,
    reply_needed: bool,
}

use fnv::FnvHashMap;
use std::cell::UnsafeCell;
thread_local! (
    static MEMOIZED_BANDS_OUTLINES: UnsafeCell<
        FnvHashMap<ID, (Band<CPath>, CPath)>
        > = UnsafeCell::new(FnvHashMap::default());
);

#[derive(Compact, Clone)]
pub struct ConnectOverlaps {
    other_id: ID,
    other_path: CPath,
    reply_needed: bool,
}

#[derive(Compact, Clone)]
pub struct ConnectToTransfer {
    other_id: ID,
}

#[derive(Compact, Clone)]
pub struct ConnectTransferToNormal {
    other_id: ID,
    other_path: CPath,
}

#[derive(Copy, Clone)]
pub struct AddTransferLaneInteraction(Interaction);

#[derive(Copy, Clone)]
pub struct Disconnect {
    other_id: ID,
}
#[derive(Copy, Clone)]
pub struct ConfirmDisconnect;

#[derive(Copy, Clone)]
pub struct Unbuild {
    pub report_to: ID,
}
use self::materialized_reality::ReportLaneUnbuilt;

use game::economy::households::buildings::FindLot;
use game::economy::households::buildings::FoundLot;
use game::economy::households::buildings::CheckLot;
use game::economy::households::buildings::LotResult;
