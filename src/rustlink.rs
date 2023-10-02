use std::hash::Hash;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Rustlink {
    pub alias: String,
    pub url: Option<String>,
}

impl PartialEq for Rustlink {
    fn eq(&self, other: &Self) -> bool {
        self.alias == other.alias
    }
}
impl Eq for Rustlink {}
impl Hash for Rustlink {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.alias.hash(state);
    }
}
