use crate::InitializationError;

#[non_exhaustive]
pub(crate) enum ErrorEvent {
    Initialization(InitializationError),
}
