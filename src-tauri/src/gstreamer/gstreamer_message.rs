#[derive(Clone, Debug, Default)]
pub(crate) enum GstreamerMessage {
    #[default]
    None,
    Play,
    Pause,
    Stop,
    End,
}
