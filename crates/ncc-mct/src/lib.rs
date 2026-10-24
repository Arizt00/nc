pub const CRATE_ROLE: &str = "mct";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MctContract {
    Bytes,
    Symbols,
    Layout,
}
