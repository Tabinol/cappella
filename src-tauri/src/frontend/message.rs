#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub enum Message {
    #[default]
    None,
    Temp, // TODO Remove
}
