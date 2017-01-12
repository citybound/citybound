
pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};
use kay::{ID, Recipient, Fate};

use ::Renderer;

#[derive(Copy, Clone)]
pub struct Project2dTo3d {
    pub scene_id: usize,
    pub position_2d: P2,
    pub requester: ID,
}

#[derive(Copy, Clone)]
pub struct Projected3d {
    pub position_3d: P3,
}

impl Recipient<Project2dTo3d> for Renderer {
    fn receive(&mut self, msg: &Project2dTo3d) -> Fate {
        let &Project2dTo3d { scene_id, position_2d, requester } = msg;

        let eye = &self.scenes[scene_id].eye;
        let frame_size = self.render_context.window.get_framebuffer_dimensions();

        // mouse is on the close plane of the frustum
        let normalized_2d_position = V4::new((position_2d.x / (frame_size.0 as N)) * 2.0 - 1.0,
                                             (-position_2d.y / (frame_size.1 as N)) * 2.0 + 1.0,
                                             -1.0,
                                             1.0);

        let inverse_view = Iso3::look_at_rh(&eye.position, &eye.target, &eye.up)
            .to_homogeneous()
            .inverse()
            .unwrap();
        let inverse_perspective = Persp3::new(frame_size.0 as f32 / frame_size.1 as f32,
                                              eye.field_of_view,
                                              0.1,
                                              1000.0)
            .to_matrix()
            .inverse()
            .unwrap();

        // converts from frustum to position relative to camera
        let mut position_from_camera = inverse_perspective * normalized_2d_position;
        // reinterpret that as a vector (direction)
        position_from_camera.w = 0.0;
        // convert into world coordinates
        let direction_into_world = inverse_view * position_from_camera;

        let direction_into_world_3d = V3::new(direction_into_world.x,
                                              direction_into_world.y,
                                              direction_into_world.z);
        // / direction_into_world.w;

        let distance = -eye.position.z / direction_into_world_3d.z;
        let position_in_world = eye.position + distance * direction_into_world_3d;

        requester << Projected3d { position_3d: position_in_world };
        Fate::Live
    }
}
