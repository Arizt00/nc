pub const CRATE_ROLE: &str = "primitive";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveSurface {
    Material,
    Hardware,
    Freestanding,
}
