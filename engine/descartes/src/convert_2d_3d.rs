use {V2, P2, V3, P3};

pub trait Into2d {
    type Target;
    fn into_2d(self) -> Self::Target;
}

impl Into2d for V3 {
    type Target = V2;
    fn into_2d(self) -> V2 {
        V2::new(self.x, self.y)
    }
}

impl Into2d for P3 {
    type Target = P2;
    fn into_2d(self) -> P2 {
        P2::new(self.x, self.y)
    }
}

pub trait Into3d {
    type Target;
    fn into_3d(self) -> Self::Target;
}

impl Into3d for V2 {
    type Target = V3;
    fn into_3d(self) -> V3 {
        V3::new(self.x, self.y, 0.0)
    }
}

impl Into3d for P2 {
    type Target = P3;
    fn into_3d(self) -> P3 {
        P3::new(self.x, self.y, 0.0)
    }
}
