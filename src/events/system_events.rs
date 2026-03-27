use crate::State;

#[non_exhaustive]
#[allow(dead_code)]
pub(crate) enum SystemEvent {
    SetState(State),
}
