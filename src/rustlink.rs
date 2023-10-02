#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Rustlink {
    pub alias: String,
    pub url: Option<String>,
}
