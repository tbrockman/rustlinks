use std::hash::Hash;

pub struct Golink {
    pub alias: String,
    pub url: Option<String>,
}

impl PartialEq for Golink {
    fn eq(&self, other: &Self) -> bool {
        self.alias == other.alias
    }
}
impl Eq for Golink {}
impl Hash for Golink {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.alias.hash(state);
    }
}