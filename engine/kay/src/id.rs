use super::type_registry::ShortTypeId;

/// An ID that uniquely identifies an `Actor`, or even a `SubActor` within a `Swarm`
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ID {
    /// An ID for the type of the identified `Actor`, used to dispatch messages
    /// to the message handling functions registered for this type
    pub type_id: ShortTypeId,
    /// For future use: ID for the machine in a computing cluster
    /// or multiplayer environment that the identified `Actor` lives on
    pub machine: u8,
    /// For future use: allows safe reuse of an ID after `Actor`/`SubActor` death.
    /// The version is incremented to make the new (otherwise same) ID distinguishable
    /// from erroneous references to the `Actor`/`SubActor` previously identified
    pub version: u8,
    /// Used to identify sub-actors within a top-level `Actor`. The main use-case is
    /// `Swarm` identifying and dispatching to its `SubActors` using this field
    pub sub_actor_id: u32,
}

pub fn broadcast_sub_actor_id() -> u32 {
    u32::max_value()
}

impl ID {
    /// Create a new ID
    pub fn new(type_id: ShortTypeId, sub_actor_id: u32, machine: u8, version: u8) -> Self {
        ID {
            type_id: type_id,
            machine: machine,
            version: version,
            sub_actor_id: sub_actor_id,
        }
    }

    /// Get a version of an actor ID that signals that a message
    /// should be delivered to all sub-actors of that actor,
    /// like in the case of a [`Swarm`](swarm/struct.Swarm.html).
    pub fn broadcast(&self) -> ID {
        ID {
            sub_actor_id: broadcast_sub_actor_id(),
            ..*self
        }
    }
}

impl ::std::fmt::Debug for ID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(
            f,
            "ID {}_{}_{}",
            u16::from(self.type_id),
            self.version,
            self.sub_actor_id
        )
    }
}
