use N;
use line_path::LinePath;
use closed_line_path::ClosedLinePath;
use areas::Area;

#[derive(Clone)]
#[cfg_attr(feature = "compact_containers", derive(Compact))]
pub struct Band {
    pub path: LinePath,
    pub width_left: N,
    pub width_right: N,
}

impl Band {
    pub fn new(path: LinePath, width: N) -> Band {
        Band {
            path,
            width_left: width / 2.0,
            width_right: width / 2.0,
        }
    }

    pub fn new_asymmetric(path: LinePath, width_left: N, width_right: N) -> Band {
        Band {
            path,
            width_left,
            width_right,
        }
    }

    pub fn outline(&self) -> ClosedLinePath {
        let left_path = self
            .path
            .shift_orthogonally(-self.width_left)
            .unwrap_or_else(|| self.path.clone());
        let right_path = self
            .path
            .shift_orthogonally(self.width_right)
            .unwrap_or_else(|| self.path.clone())
            .reverse();

        ClosedLinePath::new(
            LinePath::new(
                left_path
                    .points
                    .iter()
                    .chain(right_path.points.iter())
                    .chain(left_path.points.first())
                    .cloned()
                    .collect(),
            ).expect("Band path should always be valid"),
        ).expect("Band path should always be closed")
    }

    pub fn outline_distance_to_path_distance(&self, distance: N) -> N {
        let full_width = self.width_left + self.width_right;

        if let (Some(left_path_length), Some(right_path_length)) = (
            self.path
                .shift_orthogonally(-self.width_left)
                .map(|p| p.length()),
            self.path
                .shift_orthogonally(self.width_right)
                .map(|p| p.length()),
        ) {
            if distance > left_path_length + full_width + right_path_length {
                // on connector2
                0.0
            } else if distance > left_path_length + full_width {
                // on right side
                (1.0 - (distance - left_path_length - full_width) / right_path_length)
                    * self.path.length()
            } else if distance > left_path_length {
                // on connector1
                self.path.length()
            } else {
                // on left side
                (distance / left_path_length) * self.path.length()
            }
        } else {
            distance
        }
    }

    pub fn as_area(&self) -> Area {
        Area::new_simple(self.outline())
    }
}
