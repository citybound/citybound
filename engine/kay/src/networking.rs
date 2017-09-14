use std::net::{SocketAddr, TcpStream, TcpListener};
use std::io::{Read, Write, ErrorKind};
use std::thread;
use std::time::Duration;
use super::inbox::Inbox;
use super::id::{ID, broadcast_machine_id};
use super::type_registry::ShortTypeId;
use super::messaging::{Message, Packet};
use compact::Compact;

pub struct Networking {
    pub machine_id: u8,
    network: Vec<SocketAddr>,
    network_connections: Vec<Option<Connection>>,
}

impl Networking {
    pub fn new(machine_id: u8, network: Vec<SocketAddr>) -> Networking {
        Networking {
            machine_id,
            network_connections: (0..network.len()).into_iter().map(|_| None).collect(),
            network,
        }
    }

    pub fn connect(&mut self) {
        let listener = TcpListener::bind(self.network[self.machine_id as usize]).unwrap();

        // first wait for all smaller machine_ids to connect
        for (machine_id, address) in self.network.iter().enumerate() {
            if machine_id < self.machine_id as usize {
                self.network_connections[machine_id] =
                    Some(Connection::new(listener.accept().unwrap().0, true))
            }
        }

        thread::sleep(Duration::from_secs(2));

        // then try to connecto to all larger machine_ids
        for (machine_id, address) in self.network.iter().enumerate() {
            if machine_id > self.machine_id as usize {
                self.network_connections[machine_id] =
                    Some(Connection::new(TcpStream::connect(address).unwrap(), false))
            }
        }

        println!("All mapped");
    }

    pub fn receive(&mut self, inboxes: &mut [Option<Inbox>]) {
        for maybe_connection in &mut self.network_connections {
            if let Some(ref mut connection) = *maybe_connection {
                connection.try_receive(inboxes)
            }
        }
    }

    pub fn send<M: Message>(&mut self, message_type_id: ShortTypeId, mut packet: Packet<M>) {
        let total_size = ::std::mem::size_of::<ShortTypeId>() + Compact::total_size_bytes(&packet);
        let machine_id = packet.recipient_id.machine;

        // store packet compactly in buffer
        let mut packet_buf: Vec<u8> = vec![0; Compact::total_size_bytes(&packet)];

        unsafe {
            Compact::compact_behind(&mut packet, &mut packet_buf[0] as *mut u8 as *mut Packet<M>);
        }

        let connections: Vec<&mut Connection> = if machine_id == broadcast_machine_id() {
            self.network_connections
                .iter_mut()
                .filter_map(|maybe_connection| maybe_connection.as_mut())
                .collect()
        } else {
            vec![
                self.network_connections
                    .get_mut(machine_id as usize)
                    .expect("Expected machine index to exist")
                    .as_mut()
                    .expect("Expected connection to exist for machine"),
            ]
        };

        for connection in connections {
            // println!(
            //     "Sending package of size {}, msg {} for actor {}",
            //     total_size,
            //     message_type_id.as_usize(),
            //     packet.recipient_id.type_id.as_usize()
            // );

            // write total size (message type + packet)
            connection
                .stream
                .write_all(unsafe {
                    ::std::slice::from_raw_parts(
                        &total_size as *const usize as *const u8,
                        ::std::mem::size_of::<usize>(),
                    )
                })
                .unwrap();

            // write message type
            connection
                .stream
                .write_all(unsafe {
                    ::std::slice::from_raw_parts(
                        &message_type_id as *const ShortTypeId as *const u8,
                        ::std::mem::size_of::<ShortTypeId>(),
                    )
                })
                .unwrap();

            // write packet
            connection.stream.write_all(packet_buf.as_slice()).unwrap()
        }

    }
}

pub struct Connection {
    stream: TcpStream,
    reading_state: ReadingState,
    is_client: bool,
}

impl Connection {
    pub fn new(stream: TcpStream, is_client: bool) -> Connection {
        Connection {
            stream,
            reading_state: ReadingState::AwaitingLength,
            is_client,
        }
    }
}

pub enum ReadingState {
    AwaitingLength,
    AwaitingPacketOfLength(usize),
}

impl Connection {
    pub fn try_receive(&mut self, inboxes: &mut [Option<Inbox>]) {
        self.reading_state = match self.reading_state {
            ReadingState::AwaitingLength => {
                let mut length_buf = [0u8; 8];
                match self.stream.read_exact(&mut length_buf) {
                    Ok(()) => ReadingState::AwaitingPacketOfLength(unsafe { ::std::mem::transmute(length_buf) }),
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => ReadingState::AwaitingLength,
                    Err(e) => panic!("{}", e),
                }
            }
            ReadingState::AwaitingPacketOfLength(length) => {
                let mut buf = vec![0u8; length];
                match self.stream.read_exact(&mut buf) {
                    Ok(()) => {
                        // let message_type_id = (&buf[0] as *const u8) as *const ShortTypeId;
                        let recipient_type_id = (&buf[::std::mem::size_of::<ShortTypeId>()] as *const u8) as *const ID;

                        unsafe {
                            // println!("Receiving packet of size {}, msg {} for actor {}", length, (*message_type_id).as_usize(), (*recipient_type_id).type_id.as_usize());
                            if let Some(ref mut inbox) = inboxes[(*recipient_type_id).type_id.as_usize()] {
                                inbox.put_raw(&buf);
                            } else {
                                panic!("No inbox for {:?} (coming from network)", (*recipient_type_id).type_id.as_usize())
                            }
                        }

                        ReadingState::AwaitingLength
                    }
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        ReadingState::AwaitingPacketOfLength(length)
                    }
                    Err(e) => panic!("{}", e),
                }
            }
        }
    }
}