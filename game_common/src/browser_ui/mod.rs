use kay::{World, External, ActorSystem, Actor};
use compact::{CString, CHashMap};

#[derive(Compact, Clone)]
pub struct BrowserUI {
    id: BrowserUIID,
}

fn flatten_vertices(vertices: &[::monet::Vertex]) -> &[f32] {
    let new_len = vertices.len() * 3;
    unsafe { ::std::slice::from_raw_parts(vertices.as_ptr() as *const f32, new_len) }
}

fn flatten_points(points: &[::descartes::P3]) -> &[f32] {
    let new_len = points.len() * 3;
    unsafe { ::std::slice::from_raw_parts(points.as_ptr() as *const f32, new_len) }
}

#[cfg(feature = "browser")]
fn to_js_mesh(mesh: &::monet::Mesh) -> ::stdweb::Value {
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

impl BrowserUI {
    pub fn spawn(id: BrowserUIID, world: &mut World) -> BrowserUI {
        #[cfg(feature = "browser")]
        {
            for (name, mesh) in ::planning::rendering::static_meshes() {
                js! {
                    window.cbclient.setState(oldState => update(oldState, {
                        planning: {
                            rendering: {
                                staticMeshes: {
                                    [@{name}]: {"$set": @{to_js_mesh(&mesh)}}
                                }
                            }
                        }
                    }));
                }
            }

            // ::transport::lane::Lane::global_broadcast(world).get_mesh(id, world);
            // ::transport::lane::SwitchLane::global_broadcast(world).get_mesh(id, world);
        }

        BrowserUI { id }
    }

    pub fn on_frame(&mut self, world: &mut World) {
        #[cfg(feature = "browser")]
        {
            ::planning::PlanManager::global_broadcast(world).get_all_plans(self.id, world);
        }
    }

    pub fn on_plans_update(
        &mut self,
        master: &::planning::Plan,
        proposals: &CHashMap<::planning::ProposalID, ::planning::Proposal>,
        world: &mut World,
    ) {
        #[cfg(feature = "browser")]
        {
            js! {
                window.cbclient.setState(oldState => update(oldState, {
                    planning: {
                        master: {"$set": @{::stdweb::serde::Serde(master)}},
                        proposals: {"$set": @{::stdweb::serde::Serde(proposals)}}
                    }
                }));
            }
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<BrowserUI>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
