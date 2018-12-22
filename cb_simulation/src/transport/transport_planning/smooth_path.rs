use descartes::{N, P2, V2, ArcLinePath, WithUniqueOrthogonal};
use itertools::Itertools;

const MAX_DISTANCE_TO_LINE: N = 3.0;
const MAX_DISTANCE_TO_CURVE: N = 6.0;

#[derive(Clone)]
enum PartialPathSegment {
    Line {
        start: P2,
        end: P2,
        targets: Vec<P2>,
    },
    Curve {
        start: P2,
        maybe_start_direction: Option<V2>,
        end: P2,
        maybe_end_direction: Option<V2>,
        targets: Vec<P2>,
    },
}

struct PartialPath(Vec<PartialPathSegment>);

fn curve_to_path(
    start: P2,
    maybe_start_direction: Option<V2>,
    end: P2,
    maybe_end_direction: Option<V2>,
) -> Option<ArcLinePath> {
    match (maybe_start_direction, maybe_end_direction) {
        (Some(start_direction), Some(end_direction)) => {
            ArcLinePath::biarc(start, start_direction, end, end_direction)
        }
        (Some(start_direction), None) => ArcLinePath::arc(start, start_direction, end),
        (None, Some(end_direction)) => {
            ArcLinePath::arc(end, -end_direction, start).map(|path| path.reverse())
        }
        (None, None) => ArcLinePath::line(start, end),
    }
}

impl PartialPath {
    fn to_arc_line_path(&self) -> Option<ArcLinePath> {
        self.0
            .iter()
            .map(|part| match part {
                &PartialPathSegment::Curve {
                    start,
                    maybe_start_direction,
                    end,
                    maybe_end_direction,
                    ..
                } => curve_to_path(start, maybe_start_direction, end, maybe_end_direction),
                &PartialPathSegment::Line { start, end, .. } => ArcLinePath::line(start, end),
            })
            .fold(None, |maybe_path, maybe_part_path| {
                maybe_path.map_or(maybe_part_path.clone(), |path| {
                    maybe_part_path.and_then(|part_path| path.concat(&part_path).ok())
                })
            })
    }

    fn find_lines(self) -> Self {
        PartialPath(
            self.0
                .into_iter()
                .flat_map(|part| match part {
                    PartialPathSegment::Curve {
                        start: original_curve_start,
                        maybe_start_direction,
                        end: original_curve_end,
                        maybe_end_direction,
                        ref targets,
                    } if targets.len() >= 2 => {
                        #[derive(Debug)]
                        enum LinesPart {
                            Line { start: P2, end: P2, points: Vec<P2> },
                            Undetermined { points: Vec<P2> },
                        }

                        let mut remaining_points = &targets[..];

                        let maybe_start_line = maybe_start_direction.and_then(|start_direction| {
                            let start_direction_orth = start_direction.orthogonal_right();

                            remaining_points
                                .iter()
                                .enumerate()
                                .skip(1)
                                .take_while(|(_i, p)| {
                                    (*p - original_curve_start).dot(&start_direction_orth).abs()
                                        < MAX_DISTANCE_TO_LINE
                                })
                                .last()
                                .map(|(last_on_line_idx, last_on_line_point)| {
                                    let line = LinesPart::Line {
                                        start: original_curve_start,
                                        end: *last_on_line_point,
                                        points: remaining_points[..last_on_line_idx + 1].to_vec(),
                                    };
                                    remaining_points = &remaining_points[last_on_line_idx + 1..];
                                    line
                                })
                        });

                        let maybe_end_line = if remaining_points.len() >= 2 {
                            maybe_end_direction.and_then(|end_direction| {
                                let end_direction_orth = end_direction.orthogonal_right();

                                remaining_points
                                    .iter()
                                    .enumerate()
                                    .rev()
                                    .skip(1)
                                    .take_while(|(_i, p)| {
                                        (original_curve_end - *p).dot(&end_direction_orth).abs()
                                            < MAX_DISTANCE_TO_LINE
                                    })
                                    .last()
                                    .map(|(earliest_on_line_idx, earliest_on_line_point)| {
                                        let line = LinesPart::Line {
                                            start: *earliest_on_line_point,
                                            end: original_curve_end,
                                            points: remaining_points[earliest_on_line_idx..]
                                                .to_vec(),
                                        };
                                        remaining_points =
                                            &remaining_points[..earliest_on_line_idx];
                                        line
                                    })
                            })
                        } else {
                            None
                        };

                        let mut middle_parts = Vec::new();
                        let mut latest_undetermined_points = Vec::new();

                        if remaining_points.len() >= 2 {
                            let mut start_idx = 0;
                            while start_idx < remaining_points.len() {
                                let start = remaining_points[start_idx];

                                if let Some((end_idx, end)) =
                                    remaining_points.iter().enumerate().rev().find(
                                        |(potential_end_idx, potential_end)| {
                                            *potential_end_idx > start_idx && {
                                                let direction_orth = (*potential_end - start)
                                                    .orthogonal_right()
                                                    .normalize();
                                                remaining_points[start_idx + 1..*potential_end_idx]
                                                    .iter()
                                                    .all(|middle_point| {
                                                        (middle_point - start)
                                                            .dot(&direction_orth)
                                                            .abs()
                                                            < MAX_DISTANCE_TO_LINE
                                                    })
                                            }
                                        },
                                    )
                                {
                                    middle_parts.push(LinesPart::Undetermined {
                                        points: latest_undetermined_points.clone(),
                                    });
                                    latest_undetermined_points.clear();
                                    middle_parts.push(LinesPart::Line {
                                        start,
                                        end: *end,
                                        points: remaining_points[start_idx..end_idx + 1].to_vec(),
                                    });
                                    start_idx = end_idx + 1
                                } else {
                                    latest_undetermined_points.push(start);
                                    start_idx += 1;
                                }
                            }
                        }

                        let maybe_end_connector = if let Some(LinesPart::Line {
                            end: last_end,
                            ..
                        }) = middle_parts.last()
                        {
                            if *last_end != original_curve_end {
                                Some(LinesPart::Undetermined {
                                    points: latest_undetermined_points,
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        let parts = maybe_start_line
                            .into_iter()
                            .chain(middle_parts)
                            .chain(maybe_end_connector)
                            .chain(maybe_end_line)
                            .collect::<Vec<_>>();

                        let mut partial_paths = Vec::new();

                        for i in 0..parts.len() {
                            partial_paths.push(match &parts[i] {
                                &LinesPart::Line {
                                    start,
                                    end,
                                    ref points,
                                } => PartialPathSegment::Line {
                                    start,
                                    end,
                                    targets: points.clone(),
                                },
                                &LinesPart::Undetermined { ref points } => {
                                    let (start, maybe_start_direction) =
                                        if let Some(LinesPart::Line {
                                            start: previous_start,
                                            end: previous_end,
                                            ..
                                        }) =
                                            i.checked_sub(1usize).and_then(|i_m_1| parts.get(i_m_1))
                                        {
                                            (
                                                *previous_end,
                                                Some((previous_end - previous_start).normalize()),
                                            )
                                        } else {
                                            (*points.get(0).unwrap_or(&original_curve_start), None)
                                        };

                                    let (end, maybe_end_direction) =
                                        if let Some(LinesPart::Line {
                                            start: next_start,
                                            end: next_end,
                                            ..
                                        }) = parts.get(i + 1)
                                        {
                                            (*next_start, Some((next_end - next_start).normalize()))
                                        } else {
                                            (
                                                *points
                                                    .get(points.len() - 1)
                                                    .unwrap_or(&original_curve_end),
                                                None,
                                            )
                                        };

                                    PartialPathSegment::Curve {
                                        start,
                                        maybe_start_direction,
                                        end,
                                        maybe_end_direction,
                                        targets: points.to_vec(),
                                    }
                                }
                            })
                        }

                        partial_paths
                    }
                    other => vec![other],
                })
                .collect(),
        )
    }

    fn simplify(self) -> Self {
        PartialPath(
            self.0
                .into_iter()
                .coalesce(|part_a, part_b| {
                    let (start, maybe_start_direction, end, maybe_end_direction, combined_targets):
                            (P2, Option<V2>, P2, Option<V2>, Vec<P2>) =
                        match (&part_a, &part_b) {
                            (
                                &PartialPathSegment::Curve {
                                    start: curve_start,
                                    maybe_start_direction,
                                    targets: ref curve_targets,
                                    ..
                                },
                                &PartialPathSegment::Line {
                                    start: line_start,
                                    end: line_end,
                                    targets: ref line_targets,
                                },
                            ) => {
                                let end_direction = (line_end - line_start).normalize();
                                (
                                    curve_start,
                                    maybe_start_direction,
                                    line_end,
                                    Some(end_direction),
                                    curve_targets.iter().chain(line_targets).cloned().collect(),
                                )
                            }
                            (
                                &PartialPathSegment::Line {
                                    start: line_start,
                                    end: line_end,
                                    targets: ref line_targets,
                                },
                                &PartialPathSegment::Curve {
                                    end: curve_end,
                                    maybe_end_direction,
                                    targets: ref curve_targets,
                                    ..
                                },
                            ) => {
                                let start_direction = (line_end - line_start).normalize();
                                (
                                    line_start,
                                    Some(start_direction),
                                    curve_end,
                                    maybe_end_direction,
                                    line_targets.iter().chain(curve_targets).cloned().collect(),
                                )
                            }
                            (
                                &PartialPathSegment::Curve {
                                    start,
                                    maybe_start_direction,
                                    targets: ref targets_a,
                                    ..
                                },
                                &PartialPathSegment::Curve {
                                    end,
                                    maybe_end_direction,
                                    targets: ref targets_b,
                                    ..
                                },
                            ) => (
                                start,
                                maybe_start_direction,
                                end,
                                maybe_end_direction,
                                targets_a.iter().chain(targets_b).cloned().collect(),
                            ),
                            _ => unreachable!("Should never create line-line"),
                        };

                    let maybe_merged =
                        curve_to_path(start, maybe_start_direction, end, maybe_end_direction)
                            .and_then(|combined_curve_path| {
                                let combined_curve_path_linearized =
                                    combined_curve_path.to_line_path_with_max_angle(0.06);
                                if combined_targets.iter().all(|target| {
                                    combined_curve_path_linearized.distance_to(*target)
                                        < MAX_DISTANCE_TO_CURVE
                                }) {
                                    Some(PartialPathSegment::Curve {
                                        start,
                                        maybe_start_direction,
                                        end,
                                        maybe_end_direction,
                                        targets: combined_targets,
                                    })
                                } else {
                                    None
                                }
                            });

                    maybe_merged.ok_or((part_a, part_b))
                })
                .collect(),
        )
    }

    fn smooth_corners(mut self, smoothening_step: N) -> Self {
        {
            let parts = &mut self.0;
            if parts.len() >= 3 {
                for i in 1..parts.len() - 1 {
                    let (beginning_a, rest_a) = parts.split_at_mut(i);
                    let (b_slice, rest_b) = rest_a.split_at_mut(1);
                    let (a, b, c) = (
                        &mut beginning_a[beginning_a.len() - 1],
                        &mut b_slice[0],
                        &mut rest_b[0],
                    );

                    if let (
                        &mut PartialPathSegment::Line {
                            start: ref mut prev_start,
                            end: ref mut prev_end,
                            ..
                        },
                        &mut PartialPathSegment::Curve {
                            ref mut start,
                            ref mut end,
                            ref targets,
                            maybe_start_direction: Some(start_direction),
                            maybe_end_direction: Some(end_direction),
                        },
                        &mut PartialPathSegment::Line {
                            start: ref mut next_start,
                            end: ref mut next_end,
                            ..
                        },
                    ) = (a, b, c)
                    {
                        if ((*prev_end - *prev_start).norm() > 1.5 * smoothening_step)
                            && ((*next_end - *next_start).norm() > 1.5 * smoothening_step)
                        {
                            let new_prev_end = *prev_end - smoothening_step * start_direction;
                            let new_next_start = *next_start + smoothening_step * end_direction;

                            if let Some(new_curve) = ArcLinePath::biarc(
                                new_prev_end,
                                start_direction,
                                new_next_start,
                                end_direction,
                            ) {
                                let new_curve_linearized =
                                    new_curve.to_line_path_with_max_angle(0.06);
                                if targets.iter().all(|target| {
                                    new_curve_linearized.distance_to(*target)
                                        < MAX_DISTANCE_TO_CURVE
                                }) {
                                    *prev_end = new_prev_end;
                                    *start = new_prev_end;
                                    *end = new_next_start;
                                    *next_start = new_next_start;
                                }
                            }
                        }
                    }
                }
            }
        }

        self
    }
}

pub fn smooth_path_from(points: &[P2]) -> Option<ArcLinePath> {
    let initial = PartialPath(vec![PartialPathSegment::Curve {
        start: points[0],
        maybe_start_direction: None, //Some((points[1] - points[0]).normalize()),
        end: points[points.len() - 1],
        maybe_end_direction: None, //Some(
        // (points[points.len() - 1] - points[points.len() - 2]).normalize(),
        // ),
        targets: points.to_vec(),
    }]);
    let solved = initial
        .find_lines()
        .simplify()
        .simplify()
        .smooth_corners(2.0)
        .smooth_corners(5.0)
        .simplify()
        .simplify()
        .smooth_corners(10.0)
        .smooth_corners(20.0)
        .simplify()
        .simplify();
    solved.to_arc_line_path()
}
