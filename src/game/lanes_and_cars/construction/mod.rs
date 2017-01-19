use compact::CVec;
use kay::{ID, Actor, Recipient, Fate};
use kay::swarm::{Swarm, SubActor, CreateWith};
use descartes::{N, P2, Dot, Band, Curve, FiniteCurve, Path, RoughlyComparable, Intersect,
                WithUniqueOrthogonal};
use itertools::Itertools;
use ::core::geometry::CPath;
use ::ordered_float::OrderedFloat;

use super::lane::{Lane, TransferLane};
use super::connectivity::{Interaction, InteractionKind, OverlapKind};

pub mod materialized_reality;
use self::materialized_reality::BuildableRef;

#[derive(Copy, Clone)]
pub struct AdvertiseToTransferAndReport(pub ID, pub BuildableRef);

use self::materialized_reality::ReportLaneBuilt;

impl Recipient<AdvertiseToTransferAndReport> for Lane {
    fn receive(&mut self, msg: &AdvertiseToTransferAndReport) -> Fate {
        match *msg {
            AdvertiseToTransferAndReport(report_to, report_as) => {
                Swarm::<Lane>::all() <<
                Connect {
                    other_id: self.id(),
                    other_start: self.path.start(),
                    other_end: self.path.end(),
                    other_length: self.path.length(),
                    reply_needed: true,
                };
                Swarm::<TransferLane>::all() <<
                ConnectTransferToNormal {
                    other_id: self.id(),
                    other_path: self.path.clone(),
                };
                report_to << ReportLaneBuilt(self.id(), report_as);
                super::rendering::on_build(self);
                super::pathfinding::on_build(self);
                Fate::Live
            }
        }
    }
}

impl Recipient<AdvertiseToTransferAndReport> for TransferLane {
    fn receive(&mut self, msg: &AdvertiseToTransferAndReport) -> Fate {
        match *msg {
            AdvertiseToTransferAndReport(report_to, report_as) => {
                Swarm::<Lane>::all() << ConnectToTransfer { other_id: self.id() };
                report_to << ReportLaneBuilt(self.id(), report_as);
                super::rendering::on_build_transfer(self);
                Fate::Live
            }
        }
    }
}

#[derive(Compact, Clone)]
pub struct AdvertiseForOverlaps {
    lanes: CVec<ID>,
}

impl Recipient<AdvertiseForOverlaps> for Lane {
    fn receive(&mut self, msg: &AdvertiseForOverlaps) -> Fate {
        match *msg {
            AdvertiseForOverlaps { ref lanes } => {
                for &lane in lanes.iter() {
                    lane <<
                    ConnectOverlaps {
                        other_id: self.id(),
                        other_path: self.path.clone(),
                        reply_needed: true,
                    };
                }
                Fate::Live
            }
        }
    }
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

impl Recipient<Connect> for Lane {
    #[inline(never)]
    fn receive(&mut self, msg: &Connect) -> Fate {
        match *msg {
            Connect { other_id, other_start, other_end, other_length, reply_needed } => {
                if other_id == self.id() {
                    return Fate::Live;
                };

                let mut connected = false;

                if other_start.is_roughly_within(self.path.end(), CONNECTION_TOLERANCE) {
                    connected = true;

                    if !self.interactions.iter().any(|interaction| match *interaction {
                        Interaction { partner_lane, kind: InteractionKind::Next { .. }, .. } => {
                            partner_lane == other_id
                        }
                        _ => false,
                    }) {
                        self.interactions.push(Interaction {
                            partner_lane: other_id,
                            start: self.length,
                            partner_start: 0.0,
                            kind: InteractionKind::Next { green: false },
                        });
                    }

                    super::pathfinding::on_connect(self);
                }

                if other_end.is_roughly_within(self.path.start(), CONNECTION_TOLERANCE) {
                    connected = true;

                    if !self.interactions.iter().any(|interaction| match *interaction {
                        Interaction { partner_lane,
                                      kind: InteractionKind::Previous { .. },
                                      .. } => partner_lane == other_id,
                        _ => false,
                    }) {
                        self.interactions.push(Interaction {
                            partner_lane: other_id,
                            start: 0.0,
                            partner_start: other_length,
                            kind: InteractionKind::Previous,
                        });
                    }

                    super::pathfinding::on_connect(self);
                }

                if reply_needed && connected {
                    other_id <<
                    Connect {
                        other_id: self.id(),
                        other_start: self.path.start(),
                        other_end: self.path.end(),
                        other_length: self.path.length(),
                        reply_needed: false,
                    };
                }

                Fate::Live
            }
        }
    }
}

use fnv::FnvHashMap;
use ::std::cell::UnsafeCell;
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

impl Recipient<ConnectOverlaps> for Lane {
    fn receive(&mut self, msg: &ConnectOverlaps) -> Fate {
        match *msg {
            ConnectOverlaps { other_id, ref other_path, reply_needed } => {
                MEMOIZED_BANDS_OUTLINES.with(|memoized_bands_outlines_cell| {
                    let memoized_bands_outlines =
                        unsafe { &mut *memoized_bands_outlines_cell.get() };
                    let &(ref self_band, ref self_outline) =
                        memoized_bands_outlines.entry(self.id())
                            .or_insert_with(|| {
                                let band = Band::new(self.path.clone(), 4.5);
                                let outline = band.outline();
                                (band, outline)
                            }) as &(Band<CPath>, CPath);

                    let memoized_bands_outlines =
                        unsafe { &mut *memoized_bands_outlines_cell.get() };
                    let &(ref other_band, ref other_outline) =
                        memoized_bands_outlines.entry(other_id)
                            .or_insert_with(|| {
                                let band = Band::new(other_path.clone(), 4.5);
                                let outline = band.outline();
                                (band, outline)
                            }) as &(Band<CPath>, CPath);

                    let intersections = (self_outline, other_outline).intersect();
                    if intersections.len() >= 2 {
                        if let ::itertools::MinMaxResult::MinMax((entry_intersection,
                                                                  entry_distance),
                                                                 (exit_intersection,
                                                                  exit_distance)) =
                            intersections.iter()
                                .map(|intersection| {
                                    (intersection, self_band
                                        .outline_distance_to_path_distance(intersection.along_a))
                                })
                                .minmax_by_key(|&(_, distance)| OrderedFloat(distance)) {
                            let other_entry_distance = other_band
                            .outline_distance_to_path_distance(entry_intersection.along_b);
                            let other_exit_distance = other_band
                            .outline_distance_to_path_distance(exit_intersection.along_b);

                            let overlap_kind = if other_path.direction_along(other_entry_distance)
                                .is_roughly_within(self.path.direction_along(entry_distance),
                                                   0.1) ||
                                                  other_path.direction_along(other_exit_distance)
                                .is_roughly_within(self.path.direction_along(exit_distance), 0.1) {
                                // ::core::geometry::add_debug_path(
                                //     self.path.subsection(entry_distance, exit_distance).unwrap(),
                                //     [1.0, 0.5, 0.0],
                                //     0.3
                                // );
                                OverlapKind::Parallel
                            } else {
                                // ::core::geometry::add_debug_path(
                                //     self.path.subsection(entry_distance, exit_distance).unwrap(),
                                //     [1.0, 0.0, 0.0],
                                //     0.3
                                // );
                                OverlapKind::Conflicting
                            };

                            self.interactions.push(Interaction {
                                partner_lane: other_id,
                                start: entry_distance,
                                partner_start: other_entry_distance.min(other_exit_distance),
                                kind: InteractionKind::Overlap {
                                    end: exit_distance,
                                    partner_end: other_exit_distance.max(other_entry_distance),
                                    kind: overlap_kind,
                                },
                            });
                        } else {
                            panic!("both entry and exit should exist")
                        }
                    }


                    if reply_needed {
                        other_id <<
                        ConnectOverlaps {
                            other_id: self.id(),
                            other_path: self.path.clone(),
                            reply_needed: false,
                        };
                    }
                    Fate::Live
                })
            }
        }
    }
}

#[derive(Compact, Clone)]
pub struct ConnectToTransfer {
    other_id: ID,
}

impl Recipient<ConnectToTransfer> for Lane {
    fn receive(&mut self, msg: &ConnectToTransfer) -> Fate {
        match *msg {
            ConnectToTransfer { other_id } => {
                other_id <<
                ConnectTransferToNormal {
                    other_id: self.id(),
                    other_path: self.path.clone(),
                };
                Fate::Live
            }
        }
    }
}

#[derive(Compact, Clone)]
pub struct ConnectTransferToNormal {
    other_id: ID,
    other_path: CPath,
}

impl Recipient<ConnectTransferToNormal> for TransferLane {
    #[inline(never)]
    fn receive(&mut self, msg: &ConnectTransferToNormal) -> Fate {
        match *msg {
            ConnectTransferToNormal { other_id, ref other_path } => {
                let projections = (other_path.project(self.path.start()),
                                   other_path.project(self.path.end()));
                if let (Some(self_start_on_other_distance), Some(self_end_on_other_distance)) =
                    projections {
                    if self_start_on_other_distance < self_end_on_other_distance &&
                       self_end_on_other_distance - self_start_on_other_distance > 6.0 {
                        let self_start_on_other = other_path.along(self_start_on_other_distance);
                        let self_end_on_other = other_path.along(self_end_on_other_distance);

                        if self_start_on_other.is_roughly_within(self.path.start(), 3.0) &&
                           self_end_on_other.is_roughly_within(self.path.end(), 3.0) {
                            other_id <<
                            AddTransferLaneInteraction(Interaction {
                                partner_lane: self.id(),
                                start: self_start_on_other_distance,
                                partner_start: 0.0,
                                kind: InteractionKind::Overlap {
                                    end: self_start_on_other_distance + self.length,
                                    partner_end: self.length,
                                    kind: OverlapKind::Transfer,
                                },
                            });

                            let mut distance_covered = 0.0;
                            let distance_map = self.path
                                .segments()
                                .iter()
                                .map(|segment| {
                                    distance_covered += segment.length();
                                    let segment_end_on_other_distance =
                                        other_path.project(segment.end())
                                            .expect("should contain transfer lane segment end");
                                    (distance_covered,
                                     segment_end_on_other_distance - self_start_on_other_distance)
                                })
                                .collect();

                            let other_is_right = (self_start_on_other - self.path.start())
                                .dot(&self.path.start_direction().orthogonal()) >
                                                 0.0;

                            if other_is_right {
                                self.right = Some((other_id, self_start_on_other_distance));
                                self.right_distance_map = distance_map;
                            } else {
                                self.left = Some((other_id, self_start_on_other_distance));
                                self.left_distance_map = distance_map;
                            }
                        }
                    }
                }
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct AddTransferLaneInteraction(Interaction);

impl Recipient<AddTransferLaneInteraction> for Lane {
    fn receive(&mut self, msg: &AddTransferLaneInteraction) -> Fate {
        match *msg {
            AddTransferLaneInteraction(interaction) => {
                if !self.interactions
                    .iter()
                    .any(|existing| existing.partner_lane == interaction.partner_lane) {
                    self.interactions.push(interaction);
                    super::pathfinding::on_connect(self);
                }
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct Disconnect {
    other_id: ID,
}
#[derive(Copy, Clone)]
pub struct ConfirmDisconnect;

impl Recipient<Disconnect> for Lane {
    fn receive(&mut self, msg: &Disconnect) -> Fate {
        match *msg {
            Disconnect { other_id } => {
                let interaction_indices_to_remove = self.interactions
                    .iter()
                    .enumerate()
                    .filter_map(|(i, interaction)| if interaction.partner_lane == other_id {
                        Some(i)
                    } else {
                        None
                    })
                    .collect::<Vec<_>>();
                // TODO: Cancel trip
                self.cars.retain(|car| {
                    !interaction_indices_to_remove.contains(&(car.next_hop_interaction as usize))
                });
                self.obstacles.retain(|&(_obstacle, from_id)| from_id != other_id);
                for idx in interaction_indices_to_remove.into_iter().rev() {
                    self.interactions.remove(idx);
                }
                super::pathfinding::on_disconnect(self, other_id);
                other_id << ConfirmDisconnect;
                Fate::Live
            }
        }
    }
}

impl Recipient<Disconnect> for TransferLane {
    fn receive(&mut self, msg: &Disconnect) -> Fate {
        match *msg {
            Disconnect { other_id } => {
                self.left = self.left.and_then(|(left_id, left_start)| if left_id == other_id {
                    None
                } else {
                    Some((left_id, left_start))
                });
                self.right = self.right
                    .and_then(|(right_id, right_start)| if right_id == other_id {
                        None
                    } else {
                        Some((right_id, right_start))
                    });
                other_id << ConfirmDisconnect;
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct Unbuild {
    pub report_to: ID,
}
use self::materialized_reality::ReportLaneUnbuilt;

impl Recipient<Unbuild> for Lane {
    fn receive(&mut self, msg: &Unbuild) -> Fate {
        match *msg {
            Unbuild { report_to } => {
                let mut disconnects_remaining = 0;
                for id in self.interactions
                    .iter()
                    .map(|interaction| interaction.partner_lane)
                    .unique() {
                    id << Disconnect { other_id: self.id() };
                    disconnects_remaining += 1;
                }
                super::rendering::on_unbuild(self);
                MEMOIZED_BANDS_OUTLINES.with(|memoized_bands_outlines_cell| {
                    let memoized_bands_outlines =
                        unsafe { &mut *memoized_bands_outlines_cell.get() };
                    memoized_bands_outlines.remove(&self.id())
                });
                if disconnects_remaining == 0 {
                    report_to << ReportLaneUnbuilt(Some(self.id()));
                    Fate::Die
                } else {
                    self.disconnects_remaining = disconnects_remaining;
                    self.unbuilding_for = Some(report_to);
                    Fate::Live
                }
            }
        }
    }
}

impl Recipient<Unbuild> for TransferLane {
    fn receive(&mut self, msg: &Unbuild) -> Fate {
        match *msg {
            Unbuild { report_to } => {
                if let Some((left_id, _)) = self.left {
                    left_id << Disconnect { other_id: self.id() };
                }
                if let Some((right_id, _)) = self.right {
                    right_id << Disconnect { other_id: self.id() };
                }
                super::rendering::on_unbuild_transfer(self);
                if self.left.is_none() && self.right.is_none() {
                    report_to << ReportLaneUnbuilt(Some(self.id()));
                    Fate::Die
                } else {
                    self.disconnects_remaining =
                        self.left.into_iter().chain(self.right).count() as u8;
                    self.unbuilding_for = Some(report_to);
                    Fate::Live
                }
            }
        }
    }
}

impl Recipient<ConfirmDisconnect> for Lane {
    fn receive(&mut self, _msg: &ConfirmDisconnect) -> Fate {
        self.disconnects_remaining -= 1;
        if self.disconnects_remaining == 0 {
            self.unbuilding_for.expect("should be unbuilding") <<
            ReportLaneUnbuilt(Some(self.id()));
            Fate::Die
        } else {
            Fate::Live
        }
    }
}

impl Recipient<ConfirmDisconnect> for TransferLane {
    fn receive(&mut self, _msg: &ConfirmDisconnect) -> Fate {
        self.disconnects_remaining -= 1;
        if self.disconnects_remaining == 0 {
            self.unbuilding_for.expect("should be unbuilding") <<
            ReportLaneUnbuilt(Some(self.id()));
            Fate::Die
        } else {
            Fate::Live
        }
    }
}

pub fn setup() {
    Swarm::<Lane>::handle::<CreateWith<Lane, AdvertiseToTransferAndReport>>();
    Swarm::<Lane>::handle::<AdvertiseForOverlaps>();
    Swarm::<Lane>::handle::<Connect>();
    Swarm::<Lane>::handle::<ConnectToTransfer>();
    Swarm::<Lane>::handle::<ConnectOverlaps>();
    Swarm::<Lane>::handle::<AddTransferLaneInteraction>();
    Swarm::<Lane>::handle::<Disconnect>();
    Swarm::<Lane>::handle::<Unbuild>();
    Swarm::<Lane>::handle::<ConfirmDisconnect>();

    Swarm::<TransferLane>::handle::<CreateWith<TransferLane, AdvertiseToTransferAndReport>>();
    Swarm::<TransferLane>::handle::<ConnectTransferToNormal>();
    Swarm::<TransferLane>::handle::<Disconnect>();
    Swarm::<TransferLane>::handle::<Unbuild>();
    Swarm::<TransferLane>::handle::<ConfirmDisconnect>();

    self::materialized_reality::setup();
}