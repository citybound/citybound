use super::THE_SYSTEM;
use super::messaging::Message;
use super::type_registry::ShortTypeId;

/// The ID used to specify a specific object within the actor system
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ID {
    /// The sequentially numbered type ID of the actor within the actor system
    pub type_id: ShortTypeId,
    /// The "generation" of the ID, to help debugging as instance_IDs can be reused
    pub version: u8,
    /// The instance of the type used to address a specific actor
    /// Is broadcast if equal to `u32::max_value()`
    /// Is swarm if equal to `u32::max_value() -1`
    pub instance_id: u32,
}

impl ID {
    pub fn new(type_id: ShortTypeId, instance_id: u32, version: u8) -> Self {
        ID {
            type_id: type_id,
            version: version,
            instance_id: instance_id,
        }
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
