use {V2, N, dot, PI};

#[inline]
pub fn angle_to(a: V2, b: V2) -> N {
    let theta: N = dot(&a, &b) / (a.norm() * b.norm());
    theta.min(1.0).max(-1.0).acos()
}

#[inline]
pub fn angle_along_to(a: V2, a_direction: V2, b: V2) -> N {
    let simple_angle = angle_to(a, b);
    let linear_direction = (b - a).normalize();

    if a_direction.dot(&linear_direction) >= 0.0 {
        simple_angle
    } else {
        2.0 * PI - simple_angle
    }
}

#[inline]
pub fn signed_angle_to(a: V2, b: V2) -> N {
    // https://stackoverflow.com/a/2150475
    let det = a.x * b.y - a.y * b.x;
    let dot = a.x * b.x + a.y * b.y;
    (det).atan2(dot)
}

//
//  DESCARTES ASSUMES
//  A RIGHT HAND COORDINATE SYSTEM
//

pub trait WithUniqueOrthogonal {
    fn orthogonal(&self) -> Self;
}

impl WithUniqueOrthogonal for V2 {
    fn orthogonal(&self) -> V2 {
        V2::new(self.y, -self.x)
    }
}
