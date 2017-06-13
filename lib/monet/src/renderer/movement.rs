
pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};
use kay::{Fate, ActorSystem};

use {Renderer, Eye};

#[derive(Copy, Clone)]
pub enum Movement {
    Shift(V3),
    ShiftAbsolute(V3),
    Zoom(N, P3),
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

pub fn setup(system: &mut ActorSystem) {
    system.extend::<Renderer, _>(|mut the_renderer| {
        the_renderer.on_critical(|&MoveEye { scene_id, movement }, renderer, world| {
            match movement {
                Movement::Shift(delta) => renderer.movement_shift(scene_id, delta),
                Movement::ShiftAbsolute(delta) => renderer.movement_shift_absolute(scene_id, delta),
                Movement::Zoom(delta, mouse_position) => {
                    renderer.movement_zoom(scene_id, delta, mouse_position)
                }
                Movement::Yaw(delta) => renderer.movement_yaw(scene_id, delta),
                Movement::Pitch(delta) => renderer.movement_pitch(scene_id, delta),
            }

            for &id in &renderer.scenes[scene_id].eye_listeners {
                world.send(id,
                           EyeMoved {
                               eye: renderer.scenes[scene_id].eye,
                               movement: movement,
                           });
            }
            Fate::Live
        });
    });
}

impl Renderer {
    fn movement_shift(&mut self, scene_id: usize, delta: V3) {
        let eye = &mut self.scenes[scene_id].eye;
        let eye_direction_2d = (eye.target - eye.position).into_2d().normalize();
        let absolute_delta = delta.x * eye_direction_2d.into_3d() +
                             delta.y * eye_direction_2d.orthogonal().into_3d() +
                             V3::new(0.0, 0.0, delta.z);

        let dist_to_target = (eye.target - eye.position).norm();

        eye.position += absolute_delta * (dist_to_target / 500.0);
        eye.target += absolute_delta * (dist_to_target / 500.0);
    }

    fn movement_shift_absolute(&mut self, scene_id: usize, delta: V3) {
        let eye = &mut self.scenes[scene_id].eye;
        eye.position += delta;
        eye.target += delta;
    }

    fn movement_zoom(&mut self, scene_id: usize, delta: N, zoom_point: P3) {
        let eye = &mut self.scenes[scene_id].eye;

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
        eye.target = ((new_zoom_distance * (eye.target - zoom_point)) / zoom_distance +
                      zoom_point.to_vector())
            .to_point();
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
