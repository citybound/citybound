use kay::{World, External, ActorSystem, Actor};
use compact::{CString, CHashMap};

use std::net::{TcpListener, TcpStream};
#[cfg(feature = "non-dummy")]
use tungstenite::{WebSocket, Message};
use rmpv::{decode, encode, ValueRef};

#[derive(Compact, Clone)]
pub struct BrowserUI {
    id: BrowserUIID,
    #[cfg(feature = "non-dummy")]
    listener: External<TcpListener>,
    #[cfg(feature = "non-dummy")]
    websocket: External<WebSocket<TcpStream>>,
}

const MSGPACK_LITE_F32: i8 = 0x17;
const MSGPACK_LITE_U16: i8 = 0x14;
const MSGPACK_LITE_U32: i8 = 0x16;

fn vrstr(string: &str) -> ValueRef {
    ValueRef::String(string.into())
}

impl BrowserUI {
    fn handle_command<'a>(&self, command: &str, options: Option<ValueRef<'a>>, world: &mut World) {
        match (command, options) {
            ("INIT", _) => {
                ::planning::PlanManager::global_first(world).init_meshes(self.id, world);
                ::transport::lane::Lane::global_broadcast(world).get_mesh(self.id, world);
                ::transport::lane::SwitchLane::global_broadcast(world).get_mesh(self.id, world);
                println!("GOT INIT!!");
            }
            ("GET_ALL_PLANS", _) => {
                ::planning::PlanManager::global_first(world).get_all_plans(self.id, world);
            }
            (command, options) => {
                println!("Got a weird command {:?}, options: {:?}", command, options)
            }
        }
    }

    #[cfg(feature = "non-dummy")]
    fn send_command(&mut self, command: &str, options: ValueRef) {
        let mut message = Vec::new();

        encode::write_value_ref(
            &mut message,
            &ValueRef::Array(vec![vrstr(command), options]),
        ).unwrap();

        self.websocket
            .write_message(Message::Binary(message))
            .unwrap();
    }

    #[cfg(feature = "dummy")]
    fn send_command(&mut self, command: &str, options: ValueRef) {}

    pub fn add_mesh(&mut self, name: &CString, mesh: &::monet::Mesh, world: &mut World) {
        self.send_command(
            "ADD_MESH",
            ValueRef::Map(vec![
                (vrstr("name"), vrstr(&**name)),
                (
                    vrstr("vertices"),
                    ValueRef::Ext(MSGPACK_LITE_F32, into_byte_slice(&*mesh.vertices)),
                ),
                (
                    vrstr("indices"),
                    ValueRef::Ext(MSGPACK_LITE_U16, into_byte_slice(&*mesh.indices)),
                ),
            ]),
        );
    }

    pub fn remove_mesh(&mut self, name: &CString, world: &mut World) {
        self.send_command("REMOVE_MESH", vrstr(&**name));
    }

    pub fn send_all_plans(
        &mut self,
        master: &::planning::Plan,
        proposals: &CHashMap<::planning::ProposalID, ::planning::Proposal>,
        world: &mut World,
    ) {
        fn encode_gesture<'a>(gesture: &'a ::planning::Gesture) -> ValueRef<'a> {
            ValueRef::Map(vec![
                (
                    vrstr("points"),
                    ValueRef::Ext(MSGPACK_LITE_F32, into_byte_slice(&*gesture.points)),
                ),
                (
                    vrstr("intent"),
                    match gesture.intent {
                        ::planning::GestureIntent::Road(road_intent) => ValueRef::Map(vec![
                            (vrstr("type"), vrstr("Road")),
                            (
                                vrstr("forward"),
                                ValueRef::Integer(road_intent.n_lanes_forward.into()),
                            ),
                            (
                                vrstr("backward"),
                                ValueRef::Integer(road_intent.n_lanes_backward.into()),
                            ),
                        ]),
                        ::planning::GestureIntent::Zone(_) => {
                            ValueRef::Map(vec![(vrstr("type"), vrstr("Zone"))])
                        }
                        ::planning::GestureIntent::Building(_) => {
                            ValueRef::Map(vec![(vrstr("type"), vrstr("Building"))])
                        }
                    },
                ),
            ])
        }

        self.send_command(
            "UPDATE_ALL_PLANS",
            ValueRef::Map(vec![
                (
                    vrstr("master".into()),
                    ValueRef::Map(
                        master
                            .gestures
                            .pairs()
                            .map(|(id, gesture)| {
                                (
                                    ValueRef::Ext(MSGPACK_LITE_U32, id.0.as_bytes()),
                                    encode_gesture(gesture),
                                )
                            })
                            .collect(),
                    ),
                ),
                (
                    vrstr("proposals".into()),
                    ValueRef::Map(
                        proposals
                            .pairs()
                            .map(|(proposal_id, proposal)| {
                                (
                                    ValueRef::Ext(MSGPACK_LITE_U32, proposal_id.0.as_bytes()),
                                    ValueRef::Map(
                                        proposal
                                            .current_history()
                                            .last()
                                            .map(|plan| {
                                                plan.gestures
                                                    .pairs()
                                                    .map(|(id, gesture)| {
                                                        (
                                                            ValueRef::Ext(
                                                                MSGPACK_LITE_U32,
                                                                id.0.as_bytes(),
                                                            ),
                                                            encode_gesture(gesture),
                                                        )
                                                    })
                                                    .collect()
                                            })
                                            .unwrap_or_else(Vec::new),
                                    ),
                                )
                            })
                            .collect(),
                    ),
                ),
            ]),
        )
    }

    pub fn send_preview(
        &mut self,
        line_meshes: &::monet::Mesh,
        lane_meshes: &::monet::Mesh,
        switch_lane_meshes: &::monet::Mesh,
        world: &mut World,
    ) {
        self.add_mesh(&"GestureLines".to_owned().into(), line_meshes, world);
        self.add_mesh(&"PlannedLanes".to_owned().into(), lane_meshes, world);
        self.add_mesh(
            &"PlannedSwitchLanes".to_owned().into(),
            switch_lane_meshes,
            world,
        );
    }
}

fn into_byte_slice<T: Sized>(slice: &[T]) -> &[u8] {
    unsafe {
        ::std::slice::from_raw_parts(
            slice.as_ptr() as *const u8,
            slice.len() * ::std::mem::size_of::<T>(),
        )
    }
}

impl BrowserUI {
    pub fn spawn(id: BrowserUIID, world: &mut World) -> Self {
        #[cfg(feature = "non-dummy")]
        {
            let listener = TcpListener::bind("127.0.0.1:9999").unwrap();
            println!("Awaiting TCP connection");
            let stream = listener.accept().unwrap().0;
            let mut websocket = ::tungstenite::server::accept(stream).unwrap();
            websocket.get_mut().set_nonblocking(true).unwrap();
            BrowserUI {
                id,
                listener: External::new(listener),
                websocket: External::new(websocket),
            }
        }
        #[cfg(feature = "dummy")]
        {
            BrowserUI { id }
        }
    }

    pub fn process_messages(&mut self, world: &mut World) {
        #[cfg(feature = "non-dummy")]
        {
            let reading_successful = match self.websocket.read_message() {
                Ok(::tungstenite::Message::Binary(raw_msg)) => {
                    let msg = decode::value_ref::read_value_ref(&mut raw_msg.as_slice());
                    match msg {
                        Ok(ValueRef::Array(command_info)) => {
                            if let Some(command_str) = command_info.get(0).and_then(|v| {
                                if let ValueRef::String(string) = v {
                                    string.as_str()
                                } else {
                                    None
                                }
                            }) {
                                self.handle_command(
                                    command_str,
                                    command_info.get(1).cloned(),
                                    world,
                                )
                            } else {
                                println!("Got a weird command {:?}", command_info)
                            }
                        }
                        Ok(weird) => println!("Got a weird message: {:?}", weird),
                        Err(err) => println!("Got a message read error: {:?}", err),
                    };
                    true
                }
                Ok(_) => true,
                Err(::tungstenite::Error::Io(ref io_err))
                    if io_err.kind() == ::std::io::ErrorKind::WouldBlock =>
                {
                    true
                }
                Err(err) => {
                    println!("WebSocket Error: {:?}", err);
                    false
                }
            };

            if !reading_successful {
                println!("Awaiting TCP re-connection");
                let stream = self.listener.accept().unwrap().0;
                let mut websocket = ::tungstenite::server::accept(stream).unwrap();
                websocket.get_mut().set_nonblocking(true).unwrap();
                self.websocket = External::new(websocket);
            }
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<BrowserUI>();
    auto_setup(system);
}

pub fn spawn(world: &mut World) -> BrowserUIID {
    BrowserUIID::spawn(world)
}

mod kay_auto;
pub use self::kay_auto::*;
