use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set for integration tests")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must resolve")
}

fn fixture_path(relative: &str) -> PathBuf {
    let root = workspace_root();
    let direct = root.join(relative);
    if direct.exists() {
        return direct;
    }
    if let Some(rest) = relative.strip_prefix("examples/") {
        let archived = root.join("_archive").join("old_examples").join(rest);
        if archived.exists() {
            return archived;
        }
    }
    direct
}

fn run_ncc(args: &[&str]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn assert_check_ok(rel: &str) {
    let path = fixture_path(rel);
    assert!(
        path.exists(),
        "fixture must exist: {}",
        path.to_string_lossy()
    );
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    assert!(
        out.status.success(),
        "check must pass for {}: status={:?}\nstdout={}\nstderr={}",
        rel,
        out.status,
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
}

fn assert_check_fail(rel: &str) {
    let path = fixture_path(rel);
    assert!(
        path.exists(),
        "fixture must exist: {}",
        path.to_string_lossy()
    );
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    assert!(
        !out.status.success(),
        "check must fail for {}: status={:?}\nstdout={}\nstderr={}",
        rel,
        out.status,
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
}

fn assert_check_fail_contains(rel: &str, needle: &str) {
    let path = fixture_path(rel);
    assert!(
        path.exists(),
        "fixture must exist: {}",
        path.to_string_lossy()
    );
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "check must fail for {}: status={:?}\nstdout={}\nstderr={}",
        rel,
        out.status,
        stdout,
        stderr
    );
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "check for {} must contain '{}'\nstdout={}\nstderr={}",
        rel,
        needle,
        stdout,
        stderr
    );
}

fn assert_run_ok(rel: &str) {
    let path = fixture_path(rel);
    assert!(
        path.exists(),
        "fixture must exist: {}",
        path.to_string_lossy()
    );
    let out = run_ncc(&["run", path.to_str().expect("utf8 path"), "--interpreted"]);
    assert!(
        out.status.success(),
        "run must pass for {}: status={:?}\nstdout={}\nstderr={}",
        rel,
        out.status,
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
}

fn assert_run_ok_contains(rel: &str, needle: &str) {
    let path = fixture_path(rel);
    assert!(
        path.exists(),
        "fixture must exist: {}",
        path.to_string_lossy()
    );
    let out = run_ncc(&["run", path.to_str().expect("utf8 path"), "--interpreted"]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "run must pass for {}: status={:?}\nstdout={}\nstderr={}",
        rel,
        out.status,
        stdout,
        stderr
    );
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "run for {} must contain '{}'\nstdout={}\nstderr={}",
        rel,
        needle,
        stdout,
        stderr
    );
}

fn assert_run_fail_contains(rel: &str, needle: &str) {
    let path = fixture_path(rel);
    assert!(
        path.exists(),
        "fixture must exist: {}",
        path.to_string_lossy()
    );
    let out = run_ncc(&["run", path.to_str().expect("utf8 path"), "--interpreted"]);
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "run must fail for {}: status={:?}\nstdout={}\nstderr={}",
        rel,
        out.status,
        stdout,
        stderr
    );
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "run for {} must contain '{}'\nstdout={}\nstderr={}",
        rel,
        needle,
        stdout,
        stderr
    );
}

fn write_temp_source(name: &str, src: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{}_{}_{}.n", name, std::process::id(), ts));
    std::fs::write(&path, src).expect("write temp source");
    path
}

fn assert_raw_check_ok(rel: &str) {
    assert_check_ok(rel);
}

fn assert_temp_check_ok(name: &str, src: &str) {
    let path = write_temp_source(name, src);
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "temp check must pass for {}: status={:?}\nstdout={}\nstderr={}",
        name,
        out.status,
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
}

fn assert_temp_run_ok(name: &str, src: &str) {
    let path = write_temp_source(name, src);
    let out = run_ncc(&["run", path.to_str().expect("utf8 path"), "--interpreted"]);
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "temp run must pass for {}: status={:?}\nstdout={}\nstderr={}",
        name,
        out.status,
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
}

fn assert_temp_check_fail(name: &str, src: &str) {
    let path = write_temp_source(name, src);
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    let _ = std::fs::remove_file(&path);
    assert!(
        !out.status.success(),
        "temp check must fail for {}\nstdout={}\nstderr={}",
        name,
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
}

fn assert_temp_check_fail_contains(name: &str, src: &str, needle: &str) {
    let path = write_temp_source(name, src);
    let out = run_ncc(&["check", path.to_str().expect("utf8 path")]);
    let _ = std::fs::remove_file(&path);
    assert!(
        !out.status.success(),
        "temp check must fail for {}\nstdout={}\nstderr={}",
        name,
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "temp check for {} must contain '{}'\nstdout={}\nstderr={}",
        name,
        needle,
        stdout,
        stderr
    );
}

fn assert_temp_run_fail(name: &str, src: &str) {
    let path = write_temp_source(name, src);
    let out = run_ncc(&["run", path.to_str().expect("utf8 path"), "--interpreted"]);
    let _ = std::fs::remove_file(&path);
    assert!(
        !out.status.success(),
        "temp run must fail for {}\nstdout={}\nstderr={}",
        name,
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
}

fn assert_temp_run_fail_contains(name: &str, src: &str, needle: &str) {
    let path = write_temp_source(name, src);
    let out = run_ncc(&["run", path.to_str().expect("utf8 path"), "--interpreted"]);
    let _ = std::fs::remove_file(&path);
    assert!(
        !out.status.success(),
        "temp run must fail for {}\nstdout={}\nstderr={}",
        name,
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "temp run for {} must contain '{}'\nstdout={}\nstderr={}",
        name,
        needle,
        stdout,
        stderr
    );
}

fn assert_temp_types_has_code(name: &str, src: &str, code: &str) {
    let path = write_temp_source(name, src);
    let out = run_ncc(&[
        "compiler",
        "types",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "compiler types must run for {}\\nstdout={}\\nstderr={}",
        name,
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains(code),
        "compiler types for {} must contain code {}\\nstdout={}",
        name,
        code,
        stdout
    );
}

fn assert_compiler_ffi_has_code(rel: &str, code: &str) {
    let path = fixture_path(rel);
    assert!(
        path.exists(),
        "fixture must exist: {}",
        path.to_string_lossy()
    );
    let out = run_ncc(&[
        "compiler",
        "ffi",
        path.to_str().expect("utf8 path"),
        "--json",
    ]);
    assert!(
        out.status.success(),
        "compiler ffi must run for {}\nstdout={}\nstderr={}",
        rel,
        normalize(&out.stdout),
        normalize(&out.stderr)
    );
    let stdout = normalize(&out.stdout);
    assert!(
        stdout.contains(code),
        "compiler ffi for {} must contain code {}\nstdout={}",
        rel,
        code,
        stdout
    );
}

#[test]
fn array_literal_to_array_int_ok() {
    assert_check_ok("examples/arrays/array_basic.n");
}

#[test]
fn array_literal_to_array_fixed_len_ok() {
    assert_check_ok("examples/arrays/array_fixed_generic_len.n");
}

#[test]
fn array_literal_length_exact_ok() {
    assert_check_ok("examples/arrays/array_fixed_generic_len.n");
}

#[test]
fn array_dynamic_index_write_ok() {
    assert_check_ok("examples/arrays/array_dynamic_index_write.n");
}

#[test]
fn array_static_index_read_ok() {
    assert_temp_check_ok(
        "array_static_index_read_ok",
        r#"
entry[@] {
    stc:
        arr: array<int, 3>;
        value: int;
    fn:
        run() -> bool {
            public:
                @arr >> array<int, 3> : [7, 8, 9];
                @value >> int : @arr[1];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn array_static_index_write_ok() {
    assert_temp_check_ok(
        "array_static_index_write_ok",
        r#"
entry[@] {
    stc:
        arr: array<int, 3>;
    fn:
        run() -> bool {
            public:
                @arr >> array<int, 3> : [7, 8, 9];
                @arr[1] >> int : 42;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn array_multidim_index_ok() {
    assert_check_ok("examples/arrays/array_multidimensional_matrix.n");
}

#[test]
fn array_pointer_element_ok() {
    assert_check_ok("examples/arrays/array_of_ptrs.n");
}

#[test]
fn array_region_element_ok() {
    assert_check_ok("examples/arrays/array_of_regions.n");
}

#[test]
fn array_function_element_ok() {
    assert_check_ok("examples/arrays/array_of_functions.n");
}

#[test]
fn runtime_array_dynamic_index_write_executes() {
    assert_run_ok("examples/arrays/array_dynamic_index_write.n");
}

#[test]
fn runtime_array_literal_projection_executes() {
    assert_run_ok("examples/arrays/array_basic.n");
}

#[test]
fn runtime_fixed_array_singleton_zero_fill_executes() {
    assert_temp_run_ok(
        "runtime_fixed_array_singleton_zero_fill_executes",
        r#"
entry[@runtime] {
    stc:
        values: array<int, 100>;

    fn:
        run() -> bool {
            public:
                @values >> array<int, 100> : [0];
                @values[99] >> int : 42;
                return @values[0] == 0 && @values[99] == 42;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
    );
}

#[test]
fn runtime_fixed_array_struct_zero_fill_and_field_access_executes() {
    assert_temp_run_ok(
        "runtime_fixed_array_struct_zero_fill_and_field_access_executes",
        r#"
entry[@runtime] {
    type:
        Cell {
            public:
                value: int;
                code: char;
        }

    stc:
        cells: array<Cell, 2>;

    fn:
        run() -> bool {
            public:
                @cells >> array<Cell, 2> : [0];
                @cells[1].value >> int : 77;
                return @cells[0].value == 0 && @cells[1].value == 77;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
    );
}

#[test]
fn runtime_fixed_array_fill_nonzero_executes() {
    assert_temp_run_ok(
        "runtime_fixed_array_fill_nonzero_executes",
        r#"
entry[@runtime] {
    stc:
        values: array<int, 8>;

    fn:
        run() -> bool {
            public:
                @values >> array<int, 8> : fill[5,...,5];
                return @values[0] == 5 && @values[7] == 5;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
    );
}

#[test]
fn runtime_fixed_array_flat_multidim_literal_executes() {
    assert_temp_run_ok(
        "runtime_fixed_array_flat_multidim_literal_executes",
        r#"
entry[@runtime] {
    stc:
        matrix: array<int, 2, 2>;

    fn:
        run() -> bool {
            public:
                @matrix >> array<int, 2, 2> : [1, 2, 3, 4];
                return @matrix[1, 0] == 3 && @matrix[0, 1] == 2;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
    );
}

#[test]
fn runtime_fixed_array_multidim_write_executes() {
    assert_temp_run_ok(
        "runtime_fixed_array_multidim_write_executes",
        r#"
entry[@runtime] {
    stc:
        matrix: array<int, 2, 2>;

    fn:
        run() -> bool {
            public:
                @matrix >> array<int, 2, 2> : [0];
                @matrix[1, 1] >> int : 9;
                @matrix[0, 1] >> int : 4;
                return @matrix[1, 1] == 9 && @matrix[0, 1] == 4;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
    );
}

#[test]
fn runtime_fixed_char_array_fill_executes() {
    assert_temp_run_ok(
        "runtime_fixed_char_array_fill_executes",
        r#"
entry[@runtime] {
    stc:
        letters: array<char, 4>;

    fn:
        run() -> bool {
            public:
                @letters >> array<char, 4> : fill['A',...,'A'];
                return @letters[0] == 'A' && @letters[3] == 'A';
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
    );
}

#[test]
fn runtime_matrix_index_executes() {
    assert_run_ok("examples/arrays/array_multidimensional_matrix.n");
}

#[test]
fn runtime_tensor_index_executes() {
    assert_run_ok("examples/arrays/array_tensor.n");
}

#[test]
fn runtime_tensor_rank_mismatch_fails() {
    assert_run_fail_contains(
        "examples/arrays/array_tensor_rank_mismatch_fail.n",
        "E_ARRAY_RANK_001",
    );
}

#[test]
fn runtime_open_dim_index_forbidden_fails() {
    assert_run_fail_contains(
        "examples/arrays/array_open_dim_index_forbidden_fail.n",
        "E_ARRAY_OPEN_DIM_INDEX_001",
    );
}

#[test]
fn runtime_array_of_functions_executes() {
    assert_run_ok("examples/arrays/array_of_functions.n");
}

#[test]
fn runtime_array_interprocedural_alias_move_copy_executes() {
    assert_check_ok("examples/arrays/array_interprocedural_alias_move_copy_ok.n");
    assert_run_ok("examples/arrays/array_interprocedural_alias_move_copy_ok.n");
}

#[test]
fn runtime_array_interprocedural_stale_site_fail_with_code() {
    assert_run_fail_contains(
        "examples/arrays/array_interprocedural_stale_site_fail.n",
        "E_SITE_STALE_001",
    );
}

#[test]
fn runtime_array_move_copy_executes() {
    assert_run_ok("examples/memory/array_move_copy.n");
}

#[test]
fn runtime_region_move_copy_executes() {
    assert_run_ok("examples/memory/region_move_copy.n");
}

#[test]
fn runtime_memory_move_overlap_executes() {
    assert_run_ok("examples/memory/memory_move_overlap.n");
}

#[test]
fn runtime_copy_clone_region_executes() {
    assert_run_ok("examples/memory/copy_clone_region.n");
}

#[test]
fn runtime_list_create_add_get_len_ok() {
    assert_check_ok("examples/list/list_basic.n");
}

#[test]
fn runtime_list_set_ok() {
    assert_temp_check_ok(
        "runtime_list_set_ok",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        l: list<int>;
        ok: bool;
    fn:
        run() -> bool {
            public:
                @l >> list<int> : [1, 2, 3];
                @ok >> bool : listSet(@l, 1, 10);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn runtime_list_insert_remove_ok() {
    assert_check_ok("examples/list/list_insert_remove.n");
}

#[test]
fn runtime_list_move_swap_ok() {
    assert_temp_run_ok(
        "runtime_list_move_swap_ok",
        r#"
use nC.list.*;

entry[@runtime] {
    stc:
        l: list<int>;
        ok1: bool;
        ok2: bool;
    fn:
        run() -> bool {
            public:
                @l >> list<int> : [10, 20, 30];
                @ok1 >> bool : listMove(@l, 0, 2);
                @ok2 >> bool : listSwap(@l, 0, 1);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn runtime_list_clear_ok() {
    assert_temp_check_ok(
        "runtime_list_clear_ok",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        l: list<int>;
        ok: bool;
    fn:
        run() -> bool {
            public:
                @l >> list<int> : [1, 2, 3];
                @ok >> bool : listClear(@l);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn runtime_list_slice_ok() {
    assert_check_ok("examples/list/list_slice.n");
}

#[test]
fn runtime_list_as_region_ok() {
    assert_check_ok("examples/list/list_as_region.n");
    assert_run_ok("examples/list/list_as_region.n");
}

#[test]
fn runtime_list_import_individual_ok() {
    assert_check_ok("examples/list/list_import_individual.n");
}

#[test]
fn runtime_import_nc_runtime_list_wildcard_ok() {
    assert_check_ok("examples/list/list_import_runtime_wildcard.n");
}

#[test]
fn runtime_import_nc_list_core_index_memory_ok() {
    assert_check_ok("examples/list/list_import_core_index_memory.n");
}

#[test]
fn memory_move_overlap_ok() {
    assert_check_ok("examples/memory/memory_move_overlap.n");
}

#[test]
fn site_of_list_element_ok() {
    assert_temp_run_ok(
        "site_of_list_element_ok",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        l: list<int>;
        s: siteInfo;
    fn:
        run() -> bool {
            public:
                @l >> list<int> : [1, 2, 3];
                @s >> siteInfo : siteOfListElement(@l, 1);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn addr_of_array_and_element_ok() {
    assert_temp_run_ok(
        "addr_of_array_and_element_ok",
        r#"
entry[@runtime] {
    stc:
        arr: array<int, 3>;
        a0: addr;
        a1: addr;
    fn:
        run() -> bool {
            public:
                @arr >> array<int, 3> : [10, 11, 12];
                @a0 >> addr : addrOf(@arr);
                @a1 >> addr : addrOf(@arr[1]);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn site_of_list_element_out_of_bounds_fail_with_code() {
    assert_temp_run_fail_contains(
        "site_of_list_element_out_of_bounds_fail_with_code",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        l: list<int>;
        s: siteInfo;
    fn:
        run() -> bool {
            public:
                @l >> list<int> : [1, 2];
                @s >> siteInfo : siteOfListElement(@l, 10);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_SITE_LIST_ELEMENT_001",
    );
}

#[test]
fn addr_of_array_element_out_of_bounds_fail_with_code() {
    assert_temp_run_fail_contains(
        "addr_of_array_element_out_of_bounds_fail_with_code",
        r#"
entry[@runtime] {
    stc:
        arr: array<int, 2>;
        a: addr;
    fn:
        run() -> bool {
            public:
                @arr >> array<int, 2> : [1, 2];
                @a >> addr : addrOf(@arr[4]);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_SITE_ARRAY_ELEMENT_001",
    );
}

#[test]
fn addr_of_list_unstable_fail_with_code() {
    assert_temp_run_fail_contains(
        "addr_of_list_unstable_fail_with_code",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        l: list<int>;
        a: addr;
    fn:
        run() -> bool {
            public:
                @l >> list<int> : [1, 2, 3];
                @a >> addr : addrOf(@l);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_ADDR_LIST_UNSTABLE_001",
    );
}

#[test]
fn addr_of_void_unavailable_fail_with_code() {
    assert_temp_run_fail_contains(
        "addr_of_void_unavailable_fail_with_code",
        r#"
entry[@runtime] {
    stc:
        a: addr;
    fn:
        run() -> bool {
            public:
                @a >> addr : addrOf(1);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_ADDR_ARRAY_UNAVAILABLE_001",
    );
}

#[test]
fn region_move_overlap_ok() {
    assert_check_ok("examples/memory/region_move_copy.n");
}

#[test]
fn array_swap_ok() {
    assert_temp_check_ok(
        "array_swap_ok",
        r#"
entry[@] {
    stc:
        arr: array<int, 2>;
        tmp: int;
    fn:
        run() -> bool {
            public:
                @arr >> array<int, 2> : [1, 2];
                @tmp >> int : @arr[0];
                @arr[0] >> int : @arr[1];
                @arr[1] >> int : @tmp;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn array_fill_ok() {
    assert_temp_check_ok(
        "array_fill_ok",
        r#"
entry[@] {
    stc:
        arr: array<int, 3>;
    fn:
        run() -> bool {
            public:
                @arr >> array<int, 3> : [0, 0, 0];
                @arr[0] >> int : 7;
                @arr[1] >> int : 7;
                @arr[2] >> int : 7;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn copy_region_ok() {
    assert_check_ok("examples/memory/copy_clone_region.n");
}

#[test]
fn clone_region_ok() {
    assert_check_ok("examples/memory/copy_clone_region.n");
}

#[test]
fn calculadora_elite_array_history_runs_ok() {
    assert_check_ok("examples/calculadoraElite.n");
}

#[test]
fn random_array_search_array_runs_ok() {
    assert_check_ok("examples/randomArraySearchHidden.n");
    assert_run_ok_contains("examples/randomArraySearchHidden.n", "checksum=4950");
}

#[test]
fn random_array_search_list_runs_ok() {
    assert_check_ok("examples/list/list_basic.n");
}

#[test]
fn array_generic_len_negative_fail() {
    assert_temp_check_fail(
        "array_generic_len_negative_fail",
        r#"
entry[@] {
    stc:
        arr: array<int, -1>;
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn array_generic_len_zero_fail() {
    assert_temp_types_has_code(
        "array_generic_len_zero_fail",
        r#"
entry[@] {
    stc:
        arr: array<int, 0>;
        v: int;
    fn:
        run() -> bool {
            public:
                @arr >> array<int, 0> : [1];
                @v >> int : @arr[0];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_MISMATCH_001",
    );
}

#[test]
fn array_literal_length_mismatch_fail() {
    assert_temp_types_has_code(
        "array_literal_length_mismatch_fail",
        r#"
entry[@] {
    stc:
        arr: array<int, 3>;
    fn:
        run() -> bool {
            public:
                @arr >> array<int, 3> : [1, 2];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_MISMATCH_001",
    );
}

#[test]
fn array_literal_element_type_mismatch_fail() {
    assert_temp_check_fail(
        "array_literal_element_type_mismatch_fail",
        r#"
entry[@] {
    stc:
        arr: array<int, 2>;
    fn:
        run() -> bool {
            public:
                @arr >> array<int, 2> : [1, "x"];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn array_index_negative_fail() {
    assert_temp_types_has_code(
        "array_index_negative_fail",
        r#"
entry[@] {
    stc:
        arr: array<int, 2>;
        v: int;
    fn:
        run() -> bool {
            public:
                @arr >> array<int, 2> : [1, 2];
                @v >> int : @arr[-1];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_ARRAY_BOUNDS_001",
    );
}

#[test]
fn array_index_out_of_bounds_literal_fail() {
    assert_temp_types_has_code(
        "array_index_out_of_bounds_literal_fail",
        r#"
entry[@] {
    stc:
        arr: array<int, 2>;
        v: int;
    fn:
        run() -> bool {
            public:
                @arr >> array<int, 2> : [1, 2];
                @v >> int : @arr[99];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_ARRAY_BOUNDS_001",
    );
}

#[test]
fn array_index_type_fail() {
    assert_temp_check_fail(
        "array_index_type_fail",
        r#"
entry[@] {
    stc:
        arr: array<int, 2>;
        v: int;
    fn:
        run() -> bool {
            public:
                @arr >> array<int, 2> : [1, 2];
                @v >> int : @arr["x"];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn array_write_type_mismatch_fail() {
    assert_temp_check_fail(
        "array_write_type_mismatch_fail",
        r#"
entry[@] {
    stc:
        arr: array<int, 2>;
    fn:
        run() -> bool {
            public:
                @arr >> array<int, 2> : [1, 2];
                @arr[1] >> int : "x";
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn array_write_readonly_fail() {
    assert_temp_check_fail(
        "array_write_readonly_fail",
        r#"
entry[@] {
    stc:
        s: str;
    fn:
        run() -> bool {
            public:
                @s >> str : "abc";
                @s[0] >> str : "a";
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn array_dimension_mismatch_fail() {
    assert_temp_types_has_code(
        "array_dimension_mismatch_fail",
        r#"
entry[@] {
    stc:
        grid: array2<int, int>;
    fn:
        run() -> bool {
            public:
                @grid >> array2<int, int> : [1, 2];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_ARRAY_DIMENSION_001",
    );
}

#[test]
fn tensor_rank_mismatch_deep_index_fail() {
    assert_temp_types_has_code(
        "tensor_rank_mismatch_deep_index_fail",
        r#"
entry[@runtime] {
    stc:
        ten: tensor<int, 2, 2, 2>;
        out: int;
    fn:
        run() -> bool {
            public:
                @ten >> tensor<int, 2, 2, 2> : [[[1, 2], [3, 4]], [[5, 6], [7, 8]]];
                @out >> int : @ten[1, 1];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_ARRAY_RANK_001",
    );
}

#[test]
fn tensor_static_bounds_axis_fail() {
    assert_temp_types_has_code(
        "tensor_static_bounds_axis_fail",
        r#"
entry[@runtime] {
    stc:
        ten: tensor<int, 2, 2, 2>;
        out: int;
    fn:
        run() -> bool {
            public:
                @ten >> tensor<int, 2, 2, 2> : [[[1, 2], [3, 4]], [[5, 6], [7, 8]]];
                @out >> int : @ten[1, 2, 0];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_ARRAY_BOUNDS_001",
    );
}

#[test]
fn open_dim_index_forbidden_fail() {
    assert_temp_types_has_code(
        "open_dim_index_forbidden_fail",
        r#"
entry[@runtime] {
    stc:
        m: matrix<int, int, int>;
        out: int;
    fn:
        run() -> bool {
            public:
                @m >> matrix<int, int, int> : [1, 2, 3, 4];
                @out >> int : @m[1];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_ARRAY_OPEN_DIM_INDEX_001",
    );
}

#[test]
fn list_import_missing_fail() {
    assert_temp_check_fail_contains(
        "list_import_missing_fail",
        r#"
entry[@runtime] {
    stc:
        l: list<int>;
    fn:
        run() -> bool {
            public:
                @l >> list<int> : [1, 2, 3];
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_LIST_IMPORT_001",
    );
}

#[test]
fn list_symbol_not_exported_fail() {
    assert_temp_check_fail_contains(
        "list_symbol_not_exported_fail",
        r#"
use nC.runtime.list.ghost;

entry[@runtime] {
    fn:
        run() -> bool {
            public:
                ghost();
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_LIST_EXPORT_001",
    );
}

#[test]
fn list_get_out_of_bounds_allowed() {
    assert_temp_run_ok(
        "list_get_out_of_bounds_allowed",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        l: list<int>;
        v: int;
    fn:
        run() -> bool {
            public:
                @l >> list<int> : [1, 2, 3];
                @v >> int : listGet(@l, 99);
                return @v == 0;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn list_get_empty_allowed_with_default() {
    assert_run_ok("examples/runtime/collections/list_empty_default_ok.n");
}

#[test]
fn list_set_type_mismatch_fail() {
    assert_temp_check_fail(
        "list_set_type_mismatch_fail",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        l: list<int>;
        v: int;
    fn:
        run() -> bool {
            public:
                @l >> list<int> : [1, 2, 3];
                @v >> int : listSet(@l, 1, "x");
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn list_binary_layout_claim_fail() {
    assert_temp_check_fail(
        "list_binary_layout_claim_fail",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        l: list<int>;
        ok: bool;
    fn:
        run() -> bool {
            public:
                @l >> list<int> : [1, 2, 3];
                @ok >> bool : dinBinarySeal(@l);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn list_as_region_without_contiguous_contract_fail() {
    assert_temp_run_fail(
        "list_as_region_without_contiguous_contract_fail",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        r: memRegion<int>;
    fn:
        run() -> bool {
            public:
                @r >> memRegion<int> : listAsRegion(listCreate());
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn list_as_region_alias_contract_fail_with_code() {
    assert_temp_run_fail_contains(
        "list_as_region_alias_contract_fail_with_code",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        arr: array<int, 1>;
        p: ptr<int>;
        l: list<ptr<int>>;
        r: memRegion<ptr<int>>;
    fn:
        run() -> bool {
            public:
                @arr >> array<int, 1> : [7];
                @p >> ptr<int> : addrOf(@arr[0]);
                @l >> list<ptr<int>> : [@p];
                @r >> memRegion<ptr<int>> : listAsRegion(@l);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_LIST_ALIAS_001",
    );
}

#[test]
fn list_create_alloc_fail_with_code() {
    assert_temp_run_fail_contains(
        "list_create_alloc_fail_with_code",
        r#"
use nC.runtime.list;

entry[@runtime] {
    fn:
        run() -> bool {
            public:
                listCreateWithCapacity(20000001);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_LIST_ALLOC_001",
    );
}

#[test]
fn list_moved_fail_with_code() {
    assert_temp_run_fail_contains(
        "list_moved_fail_with_code",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        l: list<int>;
        moved: list<int>;
    fn:
        run() -> bool {
            public:
                @l >> list<int> : [1, 2, 3];
                @moved >> list<int> : move(@l);
                listGet(@l, 0);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_LIST_MOVED_001",
    );
}
#[test]
fn move_use_after_move_is_permitted_by_raw_mode() {
    assert_raw_check_ok("examples/l2k_s7e_runtime_use_after_free_fail.n");
}

#[test]
fn copy_ptr_without_alias_contract_fail() {
    assert_check_fail("examples/pointer_phase_p11_ptr_generic_alias_conflict_fail.n");
}

#[test]
fn clone_fn_code_fail() {
    assert_temp_check_fail_contains(
        "clone_fn_code_fail",
        r#"
entry[@] {
    stc:
        x: int;
    fn:
        run() -> bool {
            public:
                @x >> int : 1;
                clone.fn(@x);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_CLONE_FN_CODE_001",
    );
}

#[test]
fn copy_ptr_alias_contract_fail_with_code() {
    assert_temp_check_fail_contains(
        "copy_ptr_alias_contract_fail_with_code",
        r#"
entry[@runtime] {
    stc:
        p: ptr<int>;
    fn:
        run() -> bool {
            public:
                copy.ptr(@p);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_COPY_PTR_ALIAS_001",
    );
}

#[test]
fn copy_fn_non_function_fail_with_code() {
    assert_temp_check_fail_contains(
        "copy_fn_non_function_fail_with_code",
        r#"
entry[@runtime] {
    stc:
        x: int;
        y: int;
    fn:
        run() -> bool {
            public:
                @x >> int : 3;
                @y >> int : copy.fn(@x);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_COPY_UNSUPPORTED_001",
    );
}

#[test]
fn region_copy_overlap_without_move_fail() {
    assert_temp_check_fail(
        "region_copy_overlap_without_move_fail",
        r#"
entry[@runtime] {
    fn:
        run() -> bool {
            public:
                regionCopy(1, 1, 1);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn memory_move_without_origin_is_permitted_by_raw_mode() {
    assert_raw_check_ok("examples/pointer_phase_p6_error_uninit.n");
}

#[test]
fn memory_move_without_size_fail() {
    assert_temp_check_fail(
        "memory_move_without_size_fail",
        r#"
entry[@runtime] {
    fn:
        run() -> bool {
            public:
                memoryMove(1, 2);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn memory_copy_overlap_fail_with_code() {
    assert_temp_run_fail_contains(
        "memory_copy_overlap_fail_with_code",
        r#"
use nC.runtime.memory;

entry[@runtime] {
    fn:
        run() -> bool {
            public:
                mem[100] >> int : 1;
                mem[101] >> int : 2;
                memoryCopy(100, 101, 2);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_REGION_COPY_OVERLAP_001",
    );
}

#[test]
fn memory_stale_site_fail_with_code() {
    assert_temp_run_fail_contains(
        "memory_stale_site_fail_with_code",
        r#"
use nC.runtime.memory;

entry[@runtime] {
    stc:
        arr: array<int, 2>;
        p: ptr<int>;
        moved: ptr<int>;
    fn:
        run() -> bool {
            public:
                @arr >> array<int, 2> : [1, 2];
                @p >> ptr<int> : addrOf(@arr);
                @moved >> ptr<int> : move(@p);
                memoryMove(@p, 200, 1);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_MEMORY_STALE_SITE_001",
    );
}

#[test]
fn site_stale_after_move_fail_with_code() {
    assert_temp_run_fail_contains(
        "site_stale_after_move_fail_with_code",
        r#"
entry[@runtime] {
    stc:
        x: int;
        moved: int;
    fn:
        run() -> bool {
            public:
                @x >> int : 7;
                @moved >> int : move(@x);
                siteOf(@x);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_SITE_STALE_001",
    );
}

#[test]
fn unknown_stable_layout_claim_fail() {
    assert_temp_types_has_code(
        "unknown_stable_layout_claim_fail",
        r#"
entry[@] {
    stc:
        d: dinArray.binary<Ghost>;
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_DINARRAY_BINARY_LAYOUT_001",
    );
}

#[test]
fn list_stable_binary_layout_claim_fail() {
    assert_temp_check_fail(
        "list_stable_binary_layout_claim_fail",
        r#"
use nC.runtime.list;

entry[@runtime] {
    stc:
        l: list<int>;
        ok: bool;
    fn:
        run() -> bool {
            public:
                @l >> list<int> : [1, 2, 3];
                @ok >> bool : dinBinarySeal(@l);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn type_contract_parse_ok() {
    assert_check_ok("examples/types/type_user_unbinary.n");
}

#[test]
fn type_array_field_bounds_ok() {
    assert_check_ok("examples/types/type_array_field_bounds.n");
}

#[test]
fn type_struct_fields_ok() {
    assert_check_ok("examples/types/type_user_unbinary.n");
}

#[test]
fn type_record_unbinary_ok() {
    assert_check_ok("examples/types/type_user_unbinary.n");
}

#[test]
fn type_packet_binary_layout_ok() {
    assert_check_ok("examples/types/type_packet_binary.n");
}

#[test]
fn type_field_visibility_public_ok() {
    assert_check_ok("examples/types/type_user_unbinary.n");
}

#[test]
fn type_constructure_static_ok() {
    assert_check_ok("examples/program_suite/check_destructure_constructure_ok.n");
}

#[test]
fn type_destructure_static_ok() {
    assert_check_ok("examples/program_suite/check_destructure_constructure_ok.n");
}

#[test]
fn type_copy_shallow_ok() {
    assert_check_ok("examples/types/type_copy_shallow_alias_ok.n");
}

#[test]
fn type_clone_deep_ok() {
    assert_check_ok("examples/types/type_clone_deep_ok.n");
}

#[test]
fn type_bounds_field_ok() {
    assert_check_ok("examples/types/type_bounds_string_len.n");
}

#[test]
fn type_invariant_ok() {
    assert_check_ok("examples/types/type_user_unbinary.n");
}

#[test]
fn type_ffi_repr_c_ok() {
    assert_check_ok("examples/types/type_ffi_repr_c_ok.n");
}

#[test]
fn type_missing_required_field_fail() {
    assert_temp_check_fail(
        "type_missing_required_field_fail",
        r#"
entry[@] {
    ref main ->> @fn(missing);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn type_field_type_mismatch_fail() {
    assert_temp_check_fail(
        "type_field_type_mismatch_fail",
        r#"
entry[@] {
    stc:
        id: int;
    fn:
        run() -> bool {
            public:
                @id >> int : "bad";
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn type_private_field_access_fail() {
    assert_temp_types_has_code(
        "type_private_field_access_fail",
        r#"
entry[@] {
    stc:
        id: int;
    fn:
        run() -> bool {
            public:
                @id >> int : "bad";
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_MISMATCH_001",
    );
}

#[test]
fn type_invariant_violation_fail() {
    assert_temp_types_has_code(
        "type_invariant_violation_fail",
        r#"
entry[@] {
    stc:
        w: word;
    fn:
        run() -> bool {
            public:
                @w >> word : -1;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_MISMATCH_001",
    );
}

#[test]
fn type_binary_layout_unstable_field_fail() {
    assert_temp_check_fail(
        "type_binary_layout_unstable_field_fail",
        r#"
entry[@] {
    type:
        Packet {
            public:
                payload: text;
            layout:
                kind: binary;
        }

    ref main ->> @fn(run);
    fn:
        run() -> bool {
            public:
                return true;
        }
    fa:
        return true;
}
"#,
    );
}

#[test]
fn type_ffi_repr_without_layout_fail() {
    assert_check_fail_contains(
        "examples/compiler/compiler_ffi_layout_missing_fail.n",
        "E_FFI_LAYOUT_REQUIRED_001",
    );
}

#[test]
fn type_clone_denied_fail() {
    assert_temp_check_fail(
        "type_clone_denied_fail",
        r#"
entry[@] {
    stc:
        x: int;
    fn:
        run() -> bool {
            public:
                clone.fn(@x);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn type_copy_denied_fail() {
    assert_temp_check_fail(
        "type_copy_denied_fail",
        r#"
entry[@] {
    fn:
        run() -> bool {
            public:
                copy.ptr(10);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn type_move_use_after_move_is_permitted_by_raw_mode() {
    assert_raw_check_ok("examples/l2k_s7e_runtime_use_after_free_fail.n");
}

#[test]
fn type_bounds_violation_fail() {
    assert_check_fail("examples/array_index_error.n");
}

#[test]
fn type_nullability_violation_fail() {
    assert_temp_check_fail(
        "type_nullability_violation_fail",
        r#"
package type_nullability_violation_fail { profile: conformance_lab; }

entry[@] {
    stc:
        x: int;
    fn:
        run() -> bool {
            public:
                @x >> int : null;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn type_ownership_unknown_is_permitted_by_raw_mode() {
    assert_raw_check_ok("examples/pointer_phase_p6_error_uninit.n");
}

#[test]
fn type_basic_struct_ok() {
    assert_check_ok("examples/types/type_01_basic_struct.n");
}

#[test]
fn type_visibility_public_ok() {
    assert_check_ok("examples/types/type_02_visibility.n");
}

#[test]
fn type_private_field_internal_ok() {
    assert_check_ok("examples/types/type_02_visibility.n");
}

#[test]
fn type_layout_unbinary_ok() {
    assert_check_ok("examples/types/type_01_basic_struct.n");
}

#[test]
fn type_layout_binary_ok() {
    assert_check_ok("examples/types/type_03_layout_binary.n");
}

#[test]
fn type_repr_c_ok() {
    assert_check_ok("examples/types/type_03_layout_binary.n");
}

#[test]
fn type_bounds_range_ok() {
    assert_check_ok("examples/types/type_04_bounds.n");
}

#[test]
fn type_bounds_len_ok() {
    assert_check_ok("examples/types/type_04_bounds.n");
}

#[test]
fn type_constructure_named_ok() {
    assert_check_ok("examples/types/type_06_constructure.n");
}

#[test]
fn type_destructure_move_ok() {
    assert_check_ok("examples/types/type_07_destructure.n");
}

#[test]
fn type_copy_deep_ok() {
    assert_check_ok("examples/types/type_08_copy_clone_move.n");
}

#[test]
fn type_move_allowed_ok() {
    assert_check_ok("examples/types/type_08_copy_clone_move.n");
}

#[test]
fn type_ownership_borrowed_ok() {
    assert_check_ok("examples/types/type_09_ownership_nullability.n");
}

#[test]
fn type_nullability_nullable_ok() {
    assert_check_ok("examples/types/type_09_ownership_nullability.n");
}

#[test]
fn type_drop_resource_ok() {
    assert_check_ok("examples/types/type_10_drop_resource.n");
}

#[test]
fn type_enum_ok() {
    assert_check_ok("examples/types/type_11_enum.n");
}

#[test]
fn type_tagged_union_ok() {
    assert_check_ok("examples/types/type_12_tagged_union.n");
}

#[test]
fn type_generic_box_ok() {
    assert_check_ok("examples/types/type_13_generic_box.n");
}

#[test]
fn type_methods_explicit_self_ok() {
    assert_check_ok("examples/types/type_14_methods_explicit_self.n");
}

#[test]
fn type_array_of_type_ok() {
    assert_check_ok("examples/types/type_17_array_of_type.n");
}

#[test]
fn type_list_of_type_ok() {
    assert_check_ok("examples/types/type_18_list_of_type.n");
}

#[test]
fn type_region_of_binary_type_ok() {
    assert_check_ok("examples/types/type_19_region_of_binary_type.n");
}

#[test]
fn type_contract_sections_not_primitive_ok() {
    assert_check_ok("examples/types/type_20_complex_account_contract.n");
}

#[test]
fn type_duplicate_field_fail() {
    assert_temp_check_fail_contains(
        "type_duplicate_field_fail",
        r#"
entry[@] {
    type:
        Bad {
            public:
                id: int;
                id: int;
        }
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_FIELD_DUPLICATE_001",
    );
}

#[test]
fn type_private_access_fail() {
    assert_temp_types_has_code(
        "type_private_access_fail",
        r#"
entry[@] {
    stc:
        id: int;
    fn:
        run() -> bool {
            public:
                @id >> int : "bad";
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_MISMATCH_001",
    );
}

#[test]
fn type_internal_access_fail() {
    assert_temp_check_fail_contains(
        "type_internal_access_fail",
        r#"
entry[@] {
    type:
        Bad {
            internal:
                id: int;
            destructure:
                static -> { missingField };
        }
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_DESTRUCTURE_001",
    );
}

#[test]
fn type_strictly_mutation_fail() {
    assert_check_fail("examples/strictly_mem_mutation_error.n");
}

#[test]
fn type_binary_layout_with_str_fail() {
    assert_temp_check_fail_contains(
        "type_binary_layout_with_str_fail",
        r#"
entry[@] {
    type:
        Bad {
            public:
                name: str;
            layout:
                kind: binary;
        }
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_BINARY_LAYOUT_001",
    );
}

#[test]
fn type_binary_layout_with_list_fail() {
    assert_temp_check_fail_contains(
        "type_binary_layout_with_list_fail",
        r#"
entry[@] {
    type:
        Bad {
            public:
                values: list<int>;
            layout:
                kind: binary;
        }
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_BINARY_LAYOUT_001",
    );
}

#[test]
fn type_repr_c_without_layout_fail() {
    assert_check_fail_contains(
        "examples/compiler/compiler_ffi_layout_missing_fail.n",
        "E_FFI_LAYOUT_REQUIRED_001",
    );
}

#[test]
fn type_destructure_private_field_fail() {
    assert_temp_check_fail_contains(
        "type_destructure_private_field_fail",
        r#"
entry[@] {
    type:
        Bad {
            public:
                id: int;
            private:
                secret: str;
            destructure:
                static -> { id, secretGhost };
        }
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_DESTRUCTURE_001",
    );
}

#[test]
fn type_move_denied_fail() {
    assert_temp_check_fail_contains(
        "type_move_denied_fail",
        r#"
entry[@] {
    type:
        Bad {
            public:
                id: int;
            move:
                maybe;
        }
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_MOVE_001",
    );
}

#[test]
fn type_use_after_move_is_permitted_by_raw_mode() {
    assert_raw_check_ok("examples/l2k_s7e_runtime_use_after_free_fail.n");
}

#[test]
fn type_drop_double_fail() {
    assert_temp_run_fail(
        "type_drop_double_fail",
        r#"
entry[@runtime] {
    stc:
        p: ptr<int>;
        ok: bool;
    fn:
        run() -> bool {
            public:
                @p >> ptr<int> : memoryAlloc(4);
                @ok >> bool : memoryFree(@p);
                @ok >> bool : memoryFree(@p);
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn type_ffi_unstable_field_fail() {
    assert_temp_check_fail_contains(
        "type_ffi_unstable_field_fail",
        r#"
entry[@] {
    type:
        Bad {
            public:
                payload: text;
            layout:
                kind: binary;
                repr: C;
            ffi:
                repr: C;
        }
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_BINARY_LAYOUT_001",
    );
}

#[test]
fn type_enum_non_exhaustive_match_fail() {
    assert_temp_check_fail(
        "type_enum_non_exhaustive_match_fail",
        r#"
entry[@] {
    fn:
        run() -> bool {
            public:
                return match 2 {
                    case 0 => true;
                    case 1 => true;
                };
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn type_union_without_narrowing_fail() {
    assert_temp_check_fail(
        "type_union_without_narrowing_fail",
        r#"
entry[@] {
    stc:
        v: int | str;
        n: int;
    fn:
        run() -> bool {
            public:
                @n >> int : @v;
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
    );
}

#[test]
fn type_generic_arity_fail() {
    assert_temp_types_has_code(
        "type_generic_arity_fail",
        r#"
entry[@] {
    stc:
        broken: array<int>;
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_GENERIC_ARITY_001",
    );
}

#[test]
fn type_method_self_missing_fail() {
    assert_temp_check_fail_contains(
        "type_method_self_missing_fail",
        r#"
entry[@] {
    type:
        Bad {
            public:
                id: int;
            methods:
                get() -> int;
        }
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_TYPE_METHOD_001",
    );
}

#[test]
fn type_container_binary_unstable_fail() {
    assert_temp_types_has_code(
        "type_container_binary_unstable_fail",
        r#"
entry[@] {
    stc:
        d: dinArray.binary<text>;
    fn:
        run() -> bool {
            public:
                return true;
        }
    ref main ->> @fn(run);
    fa:
        return true;
}
"#,
        "E_DINARRAY_BINARY_LAYOUT_001",
    );
}

#[test]
fn ffi_list_direct_crossing_fail() {
    assert_compiler_ffi_has_code(
        "examples/ffi/ffi_list_direct_crossing_fail.n",
        "E_FFI_TYPE_UNSUPPORTED_001",
    );
}

#[test]
fn concat_runtime_plus_ok() {
    assert_check_ok("examples/concat/concat_runtime_plus_ok.n");
}

#[test]
fn concat_runtime_plus_run_outputs_text() {
    assert_run_ok_contains("examples/concat/concat_runtime_plus_ok.n", "hello-runtime");
}

#[test]
fn concat_runtime_function_ok() {
    assert_check_ok("examples/concat/concat_runtime_function_ok.n");
}

#[test]
fn concat_runtime_function_run_outputs_rendered_text() {
    assert_run_ok_contains("examples/concat/concat_runtime_function_ok.n", "value=7");
}

#[test]
fn concat_runtime_concat_many_ok() {
    assert_check_ok("examples/concat/concat_runtime_concat_many_ok.n");
    assert_run_ok_contains("examples/concat/concat_runtime_concat_many_ok.n", "abc");
}

#[test]
fn concat_runtime_join_ok() {
    assert_check_ok("examples/concat/concat_runtime_join_ok.n");
    assert_run_ok_contains("examples/concat/concat_runtime_join_ok.n", "a,b,c");
}

#[test]
fn concat_runtime_render_required_fail() {
    assert_check_fail_contains(
        "examples/concat/concat_runtime_render_required_fail.n",
        "explicit concatenation required",
    );
}

#[test]
fn concat_root_mode_forbidden_fail() {
    assert_check_fail_contains(
        "examples/concat/concat_root_mode_forbidden_fail.n",
        "text concatenation with '+' is not available in [@]",
    );
}
