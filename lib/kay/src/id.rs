use super::THE_SYSTEM;
use super::messaging::Message;
use core::nonzero::NonZero;

/// The ID used to specify a specific object within the actor system
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ID {
    /// The sequentially numbered type ID of the actor within the actor system
    pub type_id: NonZero<u16>,
    /// The "generation" of the ID, to help debugging as instance_IDs can be reused
    pub version: u8,
    /// The instance of the type used to address a specific actor
    /// Is broadcast if equal to `u32::max_value()`
    /// Is swarm if equal to `u32::max_value() -1`
    pub instance_id: u32,
}

impl ID {
    pub fn individual(individual_type_id: usize) -> ID {
        ID {
            type_id: unsafe { NonZero::new(individual_type_id as u16) },
            version: 0,
            instance_id: 0,
        }
    }

    /// Construct a broadcast ID to the type
    pub fn broadcast(type_id: usize) -> ID {
        ID {
            type_id: unsafe { NonZero::new(type_id as u16) },
            version: 0,
            instance_id: u32::max_value(),
        }
    }

    /// Construct an ID which points to an actor instance in a swarm
    pub fn instance(type_id: usize, instance_id_and_version: (usize, usize)) -> ID {
        ID {
            type_id: unsafe { NonZero::new(type_id as u16) },
            version: instance_id_and_version.1 as u8,
            instance_id: instance_id_and_version.0 as u32,
        }
    }

    /// Checks if ID is a broadcast ID
    pub fn is_broadcast(&self) -> bool {
        self.instance_id == u32::max_value()
    }

    /// Created swarm ID with type ID specified
    pub fn swarm(type_id: usize) -> ID {
        ID {
            type_id: unsafe { NonZero::new(type_id as u16) },
            version: 0,
            instance_id: u32::max_value() - 1,
        }
    }

    /// Checks if ID is a swarm ID
    pub fn is_swarm(&self) -> bool {
        self.instance_id == u32::max_value() - 1
    }
}

impl ::std::fmt::Debug for ID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f,
               "ID {}_{}_{}",
               *(self.type_id),
               self.version,
               self.instance_id)
    }
}

impl<M: Message> ::std::ops::Shl<M> for ID {
    type Output = ();
    /// The shift left operator is overloaded to use to send messages
    /// e.g. recipient_ID << message
    fn shl(self, rhs: M) {
        unsafe {
            (*THE_SYSTEM).send(self, rhs);
        }
    }
}
