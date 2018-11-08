use kay::{World};

pub trait FrameListener {
    fn on_frame(&mut self, world: &mut World);
}

pub fn flatten_vertices(vertices: &[::michelangelo::Vertex]) -> &[f32] {
    let new_len = vertices.len() * 3;
    unsafe { ::std::slice::from_raw_parts(vertices.as_ptr() as *const f32, new_len) }
}

pub fn flatten_instances(instances: &[::michelangelo::Instance]) -> &[f32] {
    let new_len = instances.len() * 8;
    unsafe { ::std::slice::from_raw_parts(instances.as_ptr() as *const f32, new_len) }
}

pub fn to_js_mesh(mesh: &::michelangelo::Mesh) -> ::stdweb::Value {
    let vertices: ::stdweb::web::TypedArray<f32> = flatten_vertices(&mesh.vertices).into();
    let indices: ::stdweb::web::TypedArray<u16> = (&*mesh.indices).into();

    let value = js! {
        return {
            vertices: @{vertices},
            indices: @{indices}
        };
    };
    value
}

pub fn updated_groups_to_js(group_changes: Vec<::michelangelo::GroupChange>) -> ::stdweb::Array {
    ::stdweb::Array::from(
        group_changes
            .iter()
            .map(|change| {
                ::stdweb::Array::from(vec![
                    ::stdweb::Value::from(change.group_id as u32),
                    to_js_mesh(&change.new_group_mesh),
                ])
            })
            .collect::<Vec<_>>(),
    )
}

mod kay_auto;
pub use self::kay_auto::*;
