use kay::{World, ActorSystem, TypedID, RawID};
use compact::{CVec, CString};

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Entry {
    from: Option<RawID>,
    topic_start: u32,
    message_start: u32,
    message_len: u32,
    level: LogLevel,
}

#[derive(Compact, Clone)]
pub struct Log {
    id: LogID,
    entries: CVec<Entry>,
    text: CString,
}

pub trait LogRecipient {
    fn receive_newest_logs(
        &mut self,
        entries: &CVec<Entry>,
        text: &CString,
        effective_last: u32,
        effective_text_start: u32,
        world: &mut World,
    );
}

impl Log {
    pub fn spawn(id: LogID, _: &mut World) -> Log {
        Log {
            id,
            entries: CVec::new(),
            text: CString::new(),
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
        let topic_start = self.text.len() as u32;
        self.text.push_str(topic);
        let message_start = self.text.len() as u32;
        self.text.push_str(message);
        self.entries.push(Entry {
            from,
            topic_start,
            message_start,
            message_len: message.len() as u32,
            level,
        });
    }

    pub fn get_after(
        &mut self,
        last_known: u32,
        max_diff: u32,
        recipient: LogRecipientID,
        world: &mut World,
    ) {
        let effective_last = (last_known as usize).max(self.entries.len() - max_diff as usize);
        if effective_last < self.entries.len() {
            let entries = self.entries[effective_last..].to_vec().into();
            let effective_text_start = self.entries[effective_last].topic_start as usize;
            let text = self.text[effective_text_start..].to_owned().into();
            recipient.receive_newest_logs(
                entries,
                text,
                effective_last as u32,
                effective_text_start as u32,
                world,
            );
        }
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
