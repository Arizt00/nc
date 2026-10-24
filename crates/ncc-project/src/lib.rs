pub const CRATE_ROLE: &str = "project";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectUnit {
    pub name: String,
    pub source: String,
}
