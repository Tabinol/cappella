#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub(crate) enum FrontendMessage {
    #[default]
    None,
    Temp, // TODO Remove
}
