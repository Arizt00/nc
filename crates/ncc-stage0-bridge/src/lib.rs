// ABI adapters only. All policy decisions are implemented in stage0_policy.n.
extern "C" {
    #[link_name = "nc$S$P$15$nccStage0Policy$14$rawDeclaration"]
    fn nc_raw_declaration(token: i32) -> i32;
    #[link_name = "nc$S$P$15$nccStage0Policy$10$callTarget"]
    fn nc_call_target(shape: i32) -> i32;
    #[link_name = "nc$S$P$15$nccStage0Policy$14$guardCondition"]
    fn nc_guard_condition(tokens: *const i32, count: i32) -> i32;
    #[link_name = "nc$S$P$15$nccStage0Policy$12$functionName"]
    fn nc_function_name(role: i32) -> i32;
    #[link_name = "nc$S$P$15$nccStage0Policy$10$prefixNext"]
    fn nc_prefix_next(token: i32) -> i32;
    #[link_name = "nc$S$P$15$nccStage0Policy$12$integerClass"]
    fn nc_integer_class(data: *const u8, count: i32, pointer_bits: i32) -> i32;
    #[link_name = "nc$S$P$15$nccStage0Policy$9$extension"]
    fn nc_extension(class: i32, source_bits: i32, target_bits: i32) -> i32;
}

pub fn raw_declaration(token: i32) -> bool {
    unsafe { nc_raw_declaration(token) != 0 }
}

pub fn call_target(shape: i32) -> i32 {
    unsafe { nc_call_target(shape) }
}

pub fn guard_condition(tokens: &[i32]) -> bool {
    let count = i32::try_from(tokens.len()).expect("token slice exceeds native ABI length");
    unsafe { nc_guard_condition(tokens.as_ptr(), count) != 0 }
}

pub fn function_name(role: i32) -> bool {
    unsafe { nc_function_name(role) != 0 }
}

pub fn prefix_next(token: i32) -> bool {
    unsafe { nc_prefix_next(token) != 0 }
}

pub fn integer_class(name: &str, pointer_bits: usize) -> i32 {
    let count = i32::try_from(name.len()).expect("source type name exceeds native ABI length");
    unsafe { nc_integer_class(name.as_ptr(), count, pointer_bits as i32) }
}

pub fn extension(class: i32, source_bits: usize, target_bits: usize) -> i32 {
    unsafe { nc_extension(class, source_bits as i32, target_bits as i32) }
}
