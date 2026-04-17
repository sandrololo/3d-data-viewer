pub(crate) use crate::events::error_event::ErrorEvent;
use futures::future::Shared;
pub(crate) use system_events::SystemEvent;
pub(crate) use user_events::UserEvent;

mod error_event;
mod system_events;
mod user_events;

pub(crate) type SharedFuture<T> = Shared<std::pin::Pin<Box<dyn std::future::Future<Output = T>>>>;

pub(crate) enum Event {
    User(UserEvent),
    System(SystemEvent),
    Error(ErrorEvent),
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

impl From<ErrorEvent> for Event {
    fn from(value: ErrorEvent) -> Self {
        Self::Error(value)
    }
}
