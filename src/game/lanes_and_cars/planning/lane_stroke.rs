use descartes::{P2, V2, Path, Segment, Band, Curve, FiniteCurve, N, RoughlyComparable};
use compact::CVec;
use kay::{ID, Swarm, CreateWith};
use monet::Thing;
use core::geometry::{CPath, band_to_thing};
use super::materialized_reality::BuildableRef;
use super::super::{Lane, TransferLane, AdvertiseToTransferAndReport};

#[derive(Compact, Clone)]
pub struct LaneStroke {
    nodes: CVec<LaneStrokeNode>,
    _memoized_path: CPath,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct LaneStrokeNodeRef(pub usize, pub usize);

pub const MIN_NODE_DISTANCE: f32 = 1.0;

#[derive(Debug)]
pub enum LaneStrokeError {
    NodesTooClose,
    LessThanTwoNodes,
}

impl LaneStroke {
    pub fn new(nodes: CVec<LaneStrokeNode>) -> Result<Self, LaneStrokeError> {
        if nodes.windows(2).any(|window|
            window[0].position.is_roughly_within(window[1].position, MIN_NODE_DISTANCE)
            //::core::geometry::add_debug_point(window[0].position, [1.0, 0.0, 1.0], 0.5);
            //::core::geometry::add_debug_point(window[1].position, [1.0, 0.0, 1.0], 0.5);
        ) {
            Result::Err(LaneStrokeError::NodesTooClose)
        } else if nodes.len() <= 1 {
            Result::Err(LaneStrokeError::LessThanTwoNodes)
        } else {
            Result::Ok(LaneStroke{nodes: nodes, _memoized_path: CPath::new(vec![])})
        }
    }

    pub fn nodes(&self) -> &CVec<LaneStrokeNode> {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut CVec<LaneStrokeNode> {
        self._memoized_path = CPath::new(vec![]);
        &mut self.nodes
    }

    pub fn path(&self) -> &CPath {
        // TODO: replace by proper Option
        if self._memoized_path.segments().len() == 0 {
            // TODO: maybe there is something less damn dangerous
            #[allow(mutable_transmutes)]
            let unsafe_memoized_path: &mut CPath =
                unsafe { ::std::mem::transmute(&self._memoized_path) };
            *unsafe_memoized_path = Path::new(self.nodes
                .windows(2)
                .flat_map(|window| {
                    Segment::biarc(window[0].position,
                                   window[0].direction,
                                   window[1].position,
                                   window[1].direction)
                })
                .collect::<Vec<_>>())
        }
        &self._memoized_path
    }

    pub fn preview_thing(&self) -> Thing {
        band_to_thing(&Band::new(Band::new(self.path().clone(), 4.85).outline(), 0.6),
                      0.0)
    }

    // TODO: this is slightly ugly
    pub fn subsection(&self, start: N, end: N) -> Option<Self> {
        let path = self.path();
        if let Some(cut_path) = path.subsection(start, end) {
            let nodes = cut_path.segments()
                .iter()
                .map(|segment| {
                    LaneStrokeNode {
                        position: segment.start(),
                        direction: segment.start_direction(),
                    }
                })
                .chain(cut_path.segments()
                    .last()
                    .map(|last_segment| {
                        LaneStrokeNode {
                            position: last_segment.end(),
                            direction: last_segment.end_direction(),
                        }
                    })
                    .into_iter())
                .collect();
            LaneStroke::new(nodes).ok()
        } else {
            None
        }
    }

    pub fn with_subsection_moved(&self, start: N, end: N, delta: V2) -> MovedSubsectionInfo {
        let nodes_before = self.nodes
            .iter()
            .take_while(|node| self.path().project(node.position).unwrap() < start - 5.0)
            .cloned()
            .collect::<Vec<_>>();

        let new_subsection = self.subsection(start, end)
            .into_iter()
            .flat_map(|subsection| subsection.nodes)
            .map(|node| {
                LaneStrokeNode {
                    position: node.position + delta,
                    direction: node.direction,
                }
            })
            .collect::<Vec<_>>();

        let nodes_after = self.nodes
            .iter()
            .skip_while(|node| self.path().project(node.position).unwrap() < end + 5.0)
            .cloned()
            .collect::<Vec<_>>();

        let mut maybe_before_connector = None;
        let mut maybe_after_connector = None;

        if let (Some(&last_node_before), Some(&first_moved_node)) =
            (nodes_before.last(), new_subsection.get(0)) {
            maybe_before_connector = biarc_connection_node(last_node_before, first_moved_node);
        }

        if let (Some(&last_moved_node), Some(&first_node_after)) =
            (new_subsection.last(), nodes_after.get(0)) {
            maybe_after_connector = biarc_connection_node(last_moved_node, first_node_after);
        }

        (nodes_before, maybe_before_connector, new_subsection, maybe_after_connector, nodes_after)
    }

    pub fn build(&self, report_to: ID, report_as: BuildableRef) {
        Swarm::<Lane>::all() <<
        CreateWith(Lane::new(self.path().clone(), false, CVec::new()),
                   AdvertiseToTransferAndReport(report_to, report_as));
    }

    pub fn build_intersection(&self, report_to: ID, report_as: BuildableRef, timings: CVec<bool>) {
        Swarm::<Lane>::all() <<
        CreateWith(Lane::new(self.path().clone(), true, timings),
                   AdvertiseToTransferAndReport(report_to, report_as));
    }

    pub fn build_transfer(&self, report_to: ID, report_as: BuildableRef) {
        Swarm::<TransferLane>::all() <<
        CreateWith(TransferLane::new(self.path().clone()),
                   AdvertiseToTransferAndReport(report_to, report_as));
    }
}

pub type MovedSubsectionInfo = (Vec<LaneStrokeNode>,
                                Option<LaneStrokeNode>,
                                Vec<LaneStrokeNode>,
                                Option<LaneStrokeNode>,
                                Vec<LaneStrokeNode>);

fn biarc_connection_node(start_node: LaneStrokeNode,
                         end_node: LaneStrokeNode)
                         -> Option<LaneStrokeNode> {
    let connection_segments = Segment::biarc(start_node.position,
                                             start_node.direction,
                                             end_node.position,
                                             end_node.direction);

    if connection_segments.len() > 1 {
        let connection_node = LaneStrokeNode {
            position: connection_segments[0].end(),
            direction: connection_segments[0].end_direction(),
        };
        if !connection_node.position.is_roughly_within(start_node.position, MIN_NODE_DISTANCE) &&
           !connection_node.position.is_roughly_within(end_node.position, MIN_NODE_DISTANCE) {
            Some(connection_node)
        } else {
            None
        }
    } else {
        None
    }
}

impl<'a> RoughlyComparable for &'a LaneStroke {
    fn is_roughly_within(&self, other: &LaneStroke, tolerance: N) -> bool {
        self.nodes.len() == other.nodes.len() &&
        self.nodes
            .iter()
            .zip(other.nodes.iter())
            .all(|(n1, n2)| n1.is_roughly_within(n2, tolerance))
    }
}

#[derive(Copy, Clone)]
pub struct LaneStrokeNode {
    pub position: P2,
    pub direction: V2,
}

impl<'a> RoughlyComparable for &'a LaneStrokeNode {
    fn is_roughly_within(&self, other: &LaneStrokeNode, tolerance: N) -> bool {
        self.position.is_roughly_within(other.position, tolerance)
        // && (
        //     (self.direction.is_none() && other.direction.is_none())
        //     || (self.direction.is_some() && other.direction.is_some()
        //         && self.direction.unwrap().is_roughly_within(other.direction.unwrap(),
        //                                                      tolerance)))
    }
}
