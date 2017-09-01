use std::net::{SocketAddr, TcpStream, TcpListener};
use std::io::{Read, Write, ErrorKind};

pub struct Networking {
    pub machine_id: u8,
    network: Vec<SocketAddr>,
    network_connections: Vec<Option<TcpStream>>,
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
        let mut unmapped_connections = Vec::<TcpStream>::new();

        for (machine_id, address) in self.network.iter().enumerate() {
            if machine_id > self.machine_id as usize {
                unmapped_connections.push(TcpStream::connect(address).unwrap());
            }
        }

        let listener = TcpListener::bind(self.network[self.machine_id as usize]).unwrap();

        while unmapped_connections.len() < self.network.len() - 1 {
            let (stream, connected_addr) = listener.accept().unwrap();

            println!("{} connected!", connected_addr);

            unmapped_connections.push(stream)
        }

        println!("All connected");

        for connection in &mut unmapped_connections {
            connection.write_all(&[self.machine_id]).unwrap();
            connection.flush().unwrap();
        }

        for mut connection in unmapped_connections {
            let mut buf = [0];
            connection.read_exact(&mut buf).unwrap();

            let remote_machine_id = buf[0];
            connection.set_nonblocking(true);
            self.network_connections[remote_machine_id as usize] = Some(connection)
        }

        println!("All mapped, {:?}", self.network_connections);
    }

    pub fn receive(&mut self) {
        for maybe_connection in &mut self.network_connections {
            if let Some(ref mut connection) = *maybe_connection {
                let mut buf = [0];
                match connection.read_exact(&mut buf) {
                    Ok(()) => println!("Read one byte"),
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        println!("Blocking...");
                    }
                    Err(e) => panic!("{}", e),
                }
            }
        }
    }
}