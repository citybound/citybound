
pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};
use kay::{Recipient, Fate};

use ::{Renderer, Eye};

#[derive(Copy, Clone)]
pub enum Movement {
    Shift(V3),
    Zoom(N),
    Yaw(N),
    Pitch(N),
}

#[derive(Copy, Clone)]
pub struct MoveEye {
    pub scene_id: usize,
    pub movement: Movement,
}
#[derive(Copy, Clone)]
pub struct EyeMoved {
    pub eye: Eye,
    pub movement: Movement,
}

impl Recipient<MoveEye> for Renderer {
    fn receive(&mut self, msg: &MoveEye) -> Fate {
        match *msg {
            MoveEye { scene_id, movement } => {
                match movement {
                    Movement::Shift(delta) => self.movement_shift(scene_id, delta),
                    Movement::Zoom(delta) => self.movement_zoom(scene_id, delta),
                    Movement::Yaw(delta) => self.movement_yaw(scene_id, delta),
                    Movement::Pitch(delta) => self.movement_pitch(scene_id, delta),
                }

                for &id in &self.scenes[scene_id].eye_listeners {
                    id <<
                    EyeMoved {
                        eye: self.scenes[scene_id].eye,
                        movement: movement,
                    };
                }
                Fate::Live
            }
        }
    }
}

impl Renderer {
    fn movement_shift(&mut self, scene_id: usize, delta: V3) {
        let eye = &mut self.scenes[scene_id].eye;
        let eye_direction_2d = (eye.target - eye.position).into_2d().normalize();
        let absolute_delta = delta.x * eye_direction_2d.into_3d() +
                             delta.y * eye_direction_2d.orthogonal().into_3d() +
                             V3::new(0.0, 0.0, delta.z);

        eye.position += absolute_delta * (eye.position.z / 100.0);
        eye.target += absolute_delta * (eye.position.z / 100.0);
    }

    fn movement_zoom(&mut self, scene_id: usize, delta: N) {
        let eye = &mut self.scenes[scene_id].eye;
        let eye_direction = (eye.target - eye.position).normalize();
        if (eye.target - eye.position).norm() > 30.0 || delta < 0.0 {
            eye.position += eye_direction * delta * (eye.position.z / 100.0);
        }
    }

    fn movement_yaw(&mut self, scene_id: usize, delta: N) {
        let eye = &mut self.scenes[scene_id].eye;
        let relative_eye_position = eye.position - eye.target;
        let iso = Iso3::new(V3::new(0.0, 0.0, 0.0), V3::new(0.0, 0.0, delta));
        let rotated_relative_eye_position = iso.rotate(&relative_eye_position);

        eye.position = eye.target + rotated_relative_eye_position;
    }

    fn movement_pitch(&mut self, scene_id: usize, delta: N) {
        let eye = &mut self.scenes[scene_id].eye;
        let relative_eye_position = eye.position - eye.target;

        // Convert relative eye position to spherical coordinates
        let r = relative_eye_position.norm();
        let mut inc = (relative_eye_position.z / r).acos();
        let azi = relative_eye_position.y.atan2(relative_eye_position.x);

        // Add delta to the inclination
        inc += delta;

        // Clamp the inclination to within 0;1.5 radians
        inc = if inc < 0.0001 { 0.0001 } else { inc }; // Check lower bounds
        inc = if inc > 1.5 { 1.5 } else { inc }; // Check upper bounds;

        // Convert spherical coordinates back into carteesiam coordinates
        let x = r * inc.sin() * azi.cos();
        let y = r * inc.sin() * azi.sin();
        let z = r * inc.cos();

        // The spherical coordinates are calculated relative to the target.
        eye.position = eye.target + V3::new(x, y, z);
    }
}
