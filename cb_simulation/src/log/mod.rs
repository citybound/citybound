use kay::{World, ActorSystem, TypedID, RawID};
use compact::{CVec, CString};

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Compact, Clone, Serialize, Deserialize)]
pub struct Entry {
    from: Option<RawID>,
    topic: CString,
    message: CString,
    level: LogLevel,
}

#[derive(Compact, Clone)]
pub struct Log {
    id: LogID,
    entries: CVec<Entry>,
}

pub trait LogRecipient {
    fn receive_newest_logs(&mut self, entries: &CVec<Entry>, world: &mut World);
}

impl Log {
    pub fn spawn(id: LogID, _: &mut World) -> Log {
        Log {
            id,
            entries: CVec::new(),
        }
    }

    pub fn log(
        &mut self,
        topic: &CString,
        message: &CString,
        from: Option<RawID>,
        level: LogLevel,
        _: &mut World,
    ) {
        self.entries.push(Entry {
            from,
            topic: topic.clone(),
            message: message.clone(),
            level,
        });
    }

    pub fn get_newest_n(&mut self, n: u32, recipient: LogRecipientID, world: &mut World) {
        let first = self.entries.len().saturating_sub(n as usize + 1);
        recipient.receive_newest_logs(self.entries[first..].to_vec().into(), world);
    }
}

pub fn log<S1: Into<String>, S2: Into<String>, I: TypedID>(
    topic: S1,
    message: S2,
    level: LogLevel,
    from: I,
    world: &mut World,
) {
    let topic_as_string: String = topic.into();
    let message_as_string: String = message.into();
    LogID::local_first(world).log(
        topic_as_string.into(),
        message_as_string.into(),
        Some(from.as_raw()),
        level,
        world,
    );
}

pub fn debug<S1: Into<String>, S2: Into<String>, I: TypedID>(
    topic: S1,
    message: S2,
    from: I,
    world: &mut World,
) {
    log(topic, message, LogLevel::Debug, from, world);
}

pub fn info<S1: Into<String>, S2: Into<String>, I: TypedID>(
    topic: S1,
    message: S2,
    from: I,
    world: &mut World,
) {
    log(topic, message, LogLevel::Info, from, world);
}

pub fn warn<S1: Into<String>, S2: Into<String>, I: TypedID>(
    topic: S1,
    message: S2,
    from: I,
    world: &mut World,
) {
    log(topic, message, LogLevel::Warning, from, world);
}

pub fn error<S1: Into<String>, S2: Into<String>, I: TypedID>(
    topic: S1,
    message: S2,
    from: I,
    world: &mut World,
) {
    log(topic, message, LogLevel::Error, from, world);
}

mod kay_auto;
pub use self::kay_auto::*;

pub fn setup(system: &mut ActorSystem) {
    system.register::<Log>();
    auto_setup(system);
}

pub fn spawn(world: &mut World) {
    LogID::spawn(world);
}
