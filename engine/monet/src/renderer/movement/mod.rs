pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, Into2d, Into3d, WithUniqueOrthogonal};
use kay::World;

use {Renderer, RendererID, Eye};

#[derive(Copy, Clone)]
pub enum Movement {
    Shift(V3),
    ShiftAbsolute(V3),
    ShiftProjected(P2, P2),
    Zoom(N, P3),
    Yaw(N),
    Pitch(N),
}

pub trait EyeListener {
    fn eye_moved(&mut self, eye: Eye, movement: Movement, world: &mut World);
}

impl Renderer {
    /// Critical
    pub fn move_eye(&mut self, movement: Movement, world: &mut World) {
        match movement {
            Movement::Shift(delta) => self.movement_shift(delta),
            Movement::ShiftAbsolute(delta) => self.movement_shift_absolute(delta),
            Movement::ShiftProjected(source_2d, target_2d) => {
                self.movement_shift_projected(source_2d, target_2d)
            }
            Movement::Zoom(delta, mouse_position) => self.movement_zoom(delta, mouse_position),
            Movement::Yaw(delta) => self.movement_yaw(delta),
            Movement::Pitch(delta) => self.movement_pitch(delta),
        }

        for listener in &self.scene.eye_listeners {
            listener.eye_moved(self.scene.eye, movement, world);
        }
    }
}

impl Renderer {
    fn movement_shift(&mut self, delta: V3) {
        let eye = &mut self.scene.eye;
        let eye_direction_2d = (eye.target - eye.position).into_2d().normalize();
        let absolute_delta = delta.x * eye_direction_2d.into_3d() +
            delta.y * eye_direction_2d.orthogonal().into_3d() +
            V3::new(0.0, 0.0, delta.z);

        let dist_to_target = (eye.target - eye.position).norm();

        eye.position += absolute_delta * (dist_to_target / 500.0);
        eye.target += absolute_delta * (dist_to_target / 500.0);
    }

    fn movement_shift_absolute(&mut self, delta: V3) {
        let eye = &mut self.scene.eye;
        eye.position += delta;
        eye.target += delta;
    }

    fn movement_shift_projected(&mut self, source_2d: P2, target_2d: P2) {
        let difference_3d = self.project(source_2d) - self.project(target_2d);
        self.movement_shift_absolute(difference_3d);
    }

    fn movement_zoom(&mut self, delta: N, zoom_point: P3) {
        let eye = &mut self.scene.eye;

        // Cache common calculations
        let zoom_direction = (zoom_point - eye.position).normalize();
        let zoom_distance = (zoom_point - eye.position).norm();

        // Move eye.position towards zoom_point
        if zoom_distance > 30.0 || delta < 0.0 {
            eye.position += (zoom_direction * delta * zoom_distance) / 300.0;
        }

        let new_zoom_distance = (zoom_point - eye.position).norm();

        // Scale the distance from eye.target to zoom_point
        // with the scale between zoom distances
        eye.target = P3::from_coordinates(
            (new_zoom_distance * (eye.target - zoom_point)) / zoom_distance +
                zoom_point.coords,
        );
    }

    fn movement_yaw(&mut self, delta: N) {
        let eye = &mut self.scene.eye;
        let relative_eye_position = eye.position - eye.target;
        let iso = Iso3::new(V3::new(0.0, 0.0, 0.0), V3::new(0.0, 0.0, delta));
        let rotated_relative_eye_position = iso * relative_eye_position;

        eye.position = eye.target + rotated_relative_eye_position;
    }

    fn movement_pitch(&mut self, delta: N) {
        let eye = &mut self.scene.eye;
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

mod kay_auto;
pub use self::kay_auto::*;
