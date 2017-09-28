use std::net::{SocketAddr, TcpStream, TcpListener};
use std::io::{Read, Write, ErrorKind, BufReader};
use std::thread;
use std::time::Duration;
use super::inbox::Inbox;
use super::id::{ID, broadcast_machine_id};
use super::type_registry::ShortTypeId;
use super::messaging::{Message, Packet};
use byteorder::{LittleEndian, WriteBytesExt, ByteOrder};
use compact::Compact;

/// Represents all networking environment and networking state
/// of an `ActorSystem`
pub struct Networking {
    /// The machine index of this machine within the network of peers
    pub machine_id: u8,
    /// The current network turn this machine is in. Used to keep track
    /// if this machine lags behind or runs fast compared to its peers
    pub n_turns: usize,
    network: Vec<SocketAddr>,
    network_connections: Vec<Option<Connection>>,
}

impl Networking {
    /// Create network environment based on this machines id/index
    /// and all peer addresses (including this machine)
    pub fn new(machine_id: u8, network: Vec<SocketAddr>) -> Networking {
        Networking {
            machine_id,
            n_turns: 0,
            network_connections: (0..network.len()).into_iter().map(|_| None).collect(),
            network,
        }
    }

    /// Connect to all peers in the network
    pub fn connect(&mut self) {
        let listener = TcpListener::bind(self.network[self.machine_id as usize]).unwrap();

        // first wait for all smaller machine_ids to connect
        for (machine_id, _address) in self.network.iter().enumerate() {
            if machine_id < self.machine_id as usize {
                self.network_connections[machine_id] =
                    Some(Connection::new(listener.accept().unwrap().0))
            }
        }

        thread::sleep(Duration::from_secs(2));

        // then try to connecto to all larger machine_ids
        for (machine_id, address) in self.network.iter().enumerate() {
            if machine_id > self.machine_id as usize {
                self.network_connections[machine_id] =
                    Some(Connection::new(TcpStream::connect(address).unwrap()))
            }
        }

        println!("All mapped");
    }

    /// Finish the current networking turn and wait for peers which lag behind
    /// based on their turn number. This is the main backpressure mechanism.
    pub fn finish_turn(&mut self, inboxes: &mut [Option<Inbox>]) {
        let mut should_sleep = None;

        for maybe_connection in &mut self.network_connections {
            if let Some(Connection { n_turns, .. }) = *maybe_connection {
                if n_turns + 120 < self.n_turns {
                    // ~2s difference
                    should_sleep = Some((
                        Duration::from_millis(
                            ((self.n_turns - 120 - n_turns) / 10) as u64,
                        ),
                        n_turns,
                    ));
                }
            }
        }

        if let Some((duration, other_n_turns)) = should_sleep {
            // println!(
            //     "Sleeping to let other machine catch up ({} vs. {} turns)",
            //     other_n_turns,
            //     self.n_turns
            // );
            // Try to process extra messages if we are ahead
            self.send_and_receive(inboxes);
            self.send_and_receive(inboxes);
            self.send_and_receive(inboxes);
            ::std::thread::sleep(duration);
        };

        self.n_turns += 1;

        for maybe_connection in &mut self.network_connections {
            if let Some(ref mut connection) = *maybe_connection {
                // write turn end, use 0 to distinguish from actual packet
                connection.write_queue.write_u64::<LittleEndian>(0).unwrap();
                connection
                    .write_queue
                    .write_u64::<LittleEndian>(self.n_turns as u64)
                    .unwrap();
                connection.n_turns_since_own_turn = 0;
            }
        }
    }

    /// Send queued outbound messages and take incoming queued messages
    /// and forward them to their local target recipient(s)
    pub fn send_and_receive(&mut self, inboxes: &mut [Option<Inbox>]) {
        for maybe_connection in &mut self.network_connections {
            if let Some(ref mut connection) = *maybe_connection {
                connection.try_send();
                connection.try_receive(inboxes)
            }
        }
    }

    /// Enqueue a new (potentially) outbound packet
    pub fn enqueue<M: Message>(&mut self, message_type_id: ShortTypeId, mut packet: Packet<M>) {
        let packet_size = Compact::total_size_bytes(&packet);
        let total_size = ::std::mem::size_of::<ShortTypeId>() + packet_size;
        let machine_id = packet.recipient_id.machine;

        let mut connections: Vec<&mut Connection> = if machine_id == broadcast_machine_id() {
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

        let first_connection = connections.remove(0);
        first_connection.write_queue.reserve(
            ::std::mem::size_of::<u64>() +
                total_size,
        );

        let before_everything = first_connection.write_queue.len();

        // write total size (message type + packet)
        first_connection
            .write_queue
            .write_u64::<LittleEndian>(total_size as u64)
            .unwrap();

        // write message type
        first_connection
            .write_queue
            .write_u16::<LittleEndian>(message_type_id.into())
            .unwrap();

        let packet_pos = first_connection.write_queue.len();
        first_connection.write_queue.resize(
            packet_pos + packet_size,
            0,
        );

        unsafe {
            // store packet compactly in write queue
            Compact::compact_behind(
                &mut packet,
                &mut first_connection.write_queue[packet_pos] as *mut u8 as *mut Packet<M>,
            );
            ::std::mem::forget(packet);
        }

        for rest_connection in connections {
            rest_connection.write_queue.extend_from_slice(
                &first_connection.write_queue
                    [before_everything..],
            );
        }

    }

    pub fn debug_all_n_turns(&self) -> String {
        self.network_connections
            .iter()
            .enumerate()
            .map(|(i, connection)| {
                format!(
                    "{}: {}",
                    i,
                    if i == usize::from(self.machine_id) {
                        self.n_turns
                    } else {
                        connection.as_ref().unwrap().n_turns
                    }
                )
            })
            .collect::<Vec<_>>()
            .join(",\n")
    }
}

pub struct Connection {
    n_turns: usize,
    n_turns_since_own_turn: usize,
    stream: TcpStream,
    read_stream: BufReader<TcpStream>,
    write_queue: Vec<u8>,
    write_queue_pos: usize,
    reading_state: ReadingState,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        stream.set_nonblocking(true).unwrap();
        stream.set_read_timeout(None).unwrap();
        stream.set_write_timeout(None).unwrap();
        stream.set_nodelay(true).unwrap();
        Connection {
            read_stream: BufReader::with_capacity(1024 * 1024, stream.try_clone().unwrap()),
            stream,
            n_turns: 0,
            n_turns_since_own_turn: 0,
            write_queue: Vec::with_capacity(0),
            write_queue_pos: 0,
            reading_state: ReadingState::AwaitingLength(0, [0; 8]),
        }
    }
}

pub enum ReadingState {
    AwaitingLength(usize, [u8; 8]),
    AwaitingTurnEnd(usize, [u8; 8]),
    AwaitingPacket(usize, Vec<u8>),
}

impl Connection {
    pub fn try_send(&mut self) {
        loop {
            match self.stream.write(&self.write_queue[self.write_queue_pos..]) {
                Ok(bytes_written) => {
                    if bytes_written > 0 {
                        self.write_queue_pos += bytes_written;
                        let cutoff = self.write_queue.len() * 2 / 3;
                        if cutoff > 1000 && self.write_queue_pos >= cutoff {
                            self.write_queue.drain(..self.write_queue_pos);
                            self.write_queue_pos = 0;
                        }
                    } else {
                        break;
                    }
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => panic!("{}", e),
            }
        }
    }

    pub fn try_receive(&mut self, inboxes: &mut [Option<Inbox>]) {
        loop {
            let (blocked, maybe_new_state) = match self.reading_state {
                ReadingState::AwaitingLength(ref mut bytes_read, ref mut length_buffer) => {
                    match self.read_stream.read(&mut length_buffer[*bytes_read..]) {
                        Ok(additional_bytes_read) => {
                            *bytes_read += additional_bytes_read;
                            if *bytes_read == length_buffer.len() {
                                let length = LittleEndian::read_u64(length_buffer) as usize;
                                if length > 0 {
                                    // println!("Expecting package of length {}", length);
                                    (
                                        false,
                                        Some(ReadingState::AwaitingPacket(0, vec![0; length])),
                                    )
                                } else {
                                    // special marker of length == 0 means turn end comes next
                                    (false, Some(ReadingState::AwaitingTurnEnd(0, [0; 8])))
                                }
                            } else {
                                (false, None)
                            }
                        }
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => (true, None),
                        Err(e) => panic!("{}", e),
                    }
                }
                ReadingState::AwaitingPacket(ref mut bytes_read, ref mut packet_buffer) => {
                    match self.read_stream.read(&mut packet_buffer[*bytes_read..]) {
                        Ok(additional_bytes_read) => {
                            *bytes_read += additional_bytes_read;
                            if *bytes_read == packet_buffer.len() {
                                // let message_type_id =
                                //               (&buf[0] as *const u8) as *const ShortTypeId;
                                let recipient_type_id =
                                    (&packet_buffer[::std::mem::size_of::<ShortTypeId>()] as
                                         *const u8) as
                                        *const ID;

                                unsafe {
                                    // println!("Receiving packet of size {}, msg {} for actor {}",
                                    //              length, (*message_type_id).as_usize(),
                                    //              (*recipient_type_id).type_id.as_usize());
                                    if let Some(ref mut inbox) =
                                        inboxes[(*recipient_type_id).type_id.as_usize()]
                                    {
                                        inbox.put_raw(packet_buffer);
                                    } else {
                                        panic!(
                                            "No inbox for {:?} (coming from network)",
                                            (*recipient_type_id).type_id.as_usize()
                                        )
                                    }
                                }

                                (false, Some(ReadingState::AwaitingLength(0, [0; 8])))
                            } else {
                                (false, None)
                            }
                        }
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => (true, None),
                        Err(e) => panic!("{}", e),
                    }
                }
                ReadingState::AwaitingTurnEnd(ref mut bytes_read, ref mut n_turns_buffer) => {
                    match self.read_stream.read(&mut n_turns_buffer[*bytes_read..]) {
                        Ok(additional_bytes_read) => {
                            *bytes_read += additional_bytes_read;
                            if *bytes_read == n_turns_buffer.len() {
                                self.n_turns = LittleEndian::read_u64(n_turns_buffer) as usize;
                                self.n_turns_since_own_turn += 1;

                                // pretend that we're blocked so we only ever process all
                                // messages of 5 incoming turns within one of our own turns,
                                // applying backpressure
                                let block = self.n_turns_since_own_turn >= 5;

                                (block, Some(ReadingState::AwaitingLength(0, [0; 8])))
                            } else {
                                (false, None)
                            }
                        }
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => (true, None),
                        Err(e) => panic!("{}", e),
                    }
                }
            };

            if let Some(new_state) = maybe_new_state {
                self.reading_state = new_state;
            }

            if blocked {
                break;
            }
        }
    }
}
