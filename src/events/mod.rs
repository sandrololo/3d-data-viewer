pub(crate) use system_events::SystemEvent;
pub(crate) use user_events::UserEvent;

mod system_events;
mod user_events;

#[allow(dead_code)]
pub(crate) enum Event {
    User(UserEvent),
    System(SystemEvent),
}

impl From<UserEvent> for Event {
    fn from(value: UserEvent) -> Self {
        Self::User(value)
    }
}

impl From<SystemEvent> for Event {
    fn from(value: SystemEvent) -> Self {
        Self::System(value)
    }
}
