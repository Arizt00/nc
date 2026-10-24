pub const CRATE_ROLE: &str = "backend";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendSurface {
    Mct,
    Llvm,
    NativeFast,
    Machine,
}
