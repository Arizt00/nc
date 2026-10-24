#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn workspace_root() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let manifest = if cfg!(windows) {
        let normalized = manifest.replace('\\', "/");
        if normalized.starts_with("/mnt/")
            && normalized.len() > 7
            && normalized.as_bytes()[5].is_ascii_alphabetic()
            && normalized.as_bytes()[6] == b'/'
        {
            let drive = (normalized.as_bytes()[5] as char).to_ascii_uppercase();
            PathBuf::from(format!("{drive}:\\{}", normalized[7..].replace('/', "\\")))
        } else {
            PathBuf::from(manifest)
        }
    } else {
        PathBuf::from(manifest)
    };
    manifest
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn example_path(relative: &str) -> PathBuf {
    let root = workspace_root();
    for base in ["examples", "_archive/old_examples"] {
        let candidate = root.join(base).join(relative);
        if candidate.exists() {
            return candidate;
        }
    }
    root.join("examples").join(relative)
}

fn temp_root(name: &str) -> PathBuf {
    workspace_root()
        .join("target")
        .join("tmp")
        .join(format!("{}_{}", name, std::process::id()))
}

fn normalize(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn combined(out: &Output) -> String {
    format!("{}{}", normalize(&out.stdout), normalize(&out.stderr))
}

fn run_ncc(args: &[String]) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn run_ncc_in_dir(args: &[String], cwd: &Path) -> Output {
    Command::new(ncc_bin())
        .args(args)
        .current_dir(cwd)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute in cwd")
}

fn assert_success(out: Output, label: &str) -> String {
    let rendered = combined(&out);
    assert!(out.status.success(), "{label} failed\n{rendered}");
    rendered
}

fn write_single_file_fixture(path: &Path, label: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("fixture parent");
    }
    fs::write(
        path,
        format!(
            r#"use::stoud::io::<output>;

{label} {{
    to main {{
        println("release run matrix ok");
    }}

    fa:
        return true;
}}
"#
        ),
    )
    .expect("write single file fixture");
}

fn shell_quote(value: &Path) -> String {
    let raw = value.to_string_lossy().replace('\'', "'\"'\"'");
    format!("'{raw}'")
}

#[test]
fn release_matrix_declares_all_shell_and_file_targets() {
    let shells = [
        "windows-cmd",
        "powershell",
        "wsl",
        "kali-ubuntu-sh",
        "macos-sh",
    ];
    let file_modes = [
        "single-file",
        "project-bridge",
        "project-child-imports",
        "paths-with-spaces",
        "n",
        "nb",
        "nbb",
        "nm",
    ];

    for required in [
        "windows-cmd",
        "powershell",
        "wsl",
        "kali-ubuntu-sh",
        "macos-sh",
    ] {
        assert!(
            shells.contains(&required),
            "missing shell target {required}"
        );
    }
    for required in [
        "single-file",
        "project-bridge",
        "project-child-imports",
        "paths-with-spaces",
        "n",
        "nb",
        "nbb",
        "nm",
    ] {
        assert!(
            file_modes.contains(&required),
            "missing file target {required}"
        );
    }
}

#[test]
fn release_run_single_file_with_spaces_works_from_absolute_path() {
    let root = temp_root("release run matrix spaces");
    let _ = fs::remove_dir_all(&root);
    let file = root.join("carpeta con espacios").join("archivo activo.n");
    write_single_file_fixture(&file, "releaseRunMatrixSingle");

    let stdout = assert_success(
        run_ncc(&[
            "run".to_string(),
            file.to_string_lossy().to_string(),
            "--show-return".to_string(),
        ]),
        "ncc run single .n with spaces",
    );
    assert!(stdout.contains("release run matrix ok"), "{stdout}");
    assert!(stdout.contains("route="), "{stdout}");
}

#[cfg(unix)]
#[test]
fn release_run_accepts_windows_drive_style_path_on_wsl_mounts() {
    let root = temp_root("release_run_matrix_windows_drive_path");
    let _ = fs::remove_dir_all(&root);
    let file = root.join("ruta windows").join("active.n");
    write_single_file_fixture(&file, "releaseRunMatrixWindowsDrivePath");

    let file = file.canonicalize().expect("canonical fixture");
    let normalized = file.to_string_lossy().replace('\\', "/");
    let bytes = normalized.as_bytes();
    if !(normalized.starts_with("/mnt/")
        && bytes.len() > 7
        && bytes[5].is_ascii_alphabetic()
        && bytes[6] == b'/')
    {
        eprintln!("skipping WSL drive-style path test outside /mnt/<drive>: {normalized}");
        return;
    }
    let drive = (bytes[5] as char).to_ascii_uppercase();
    let windows_arg = format!("{drive}:/{}", &normalized[7..]);

    let stdout = assert_success(
        run_ncc(&["run".to_string(), windows_arg, "--show-return".to_string()]),
        "ncc run C:/... path on WSL mount",
    );
    assert!(stdout.contains("release run matrix ok"), "{stdout}");
}

#[test]
fn release_run_recovers_single_renamed_source_in_same_directory() {
    let root = temp_root("release_run_matrix_renamed_source");
    let _ = fs::remove_dir_all(&root);
    let file = root.join("renombrado").join("byteForgeRealPtr.n");
    let missing = root.join("renombrado").join("byteForgeInteractivo.n");
    write_single_file_fixture(&file, "byteForgeRealPtr");

    let output = run_ncc(&[
        "run".to_string(),
        missing.to_string_lossy().to_string(),
        "--show-return".to_string(),
    ]);
    let rendered = assert_success(output, "ncc run renamed single source");
    assert!(rendered.contains("release run matrix ok"), "{rendered}");
    assert!(
        rendered.contains("using renamed sibling"),
        "expected renamed-file recovery warning, got {rendered}"
    );
}

#[test]
fn release_run_recovers_renamed_source_from_project_metadata_with_many_sources() {
    let root = temp_root("release_run_matrix_renamed_project_metadata");
    let _ = fs::remove_dir_all(&root);
    let dir = root.join("APNCC");
    let real = dir.join("byteForgeRealPtr.n");
    let other = dir.join("otroArchivo.n");
    let missing = dir.join("byteForgeInteractivo.n");
    write_single_file_fixture(&real, "byteForgeRealPtr");
    write_single_file_fixture(&other, "otroArchivo");
    fs::create_dir_all(dir.join(".ncc").join("code")).expect("code dir");
    fs::write(
        dir.join(".ncc").join("code").join("byteForgeRealPtr.json"),
        r#"{
  "schema": "ncc.code.v1",
  "source": "byteForgeRealPtr.n",
  "unit": "byteForgeRealPtr"
}
"#,
    )
    .expect("code json");

    let rendered = assert_success(
        run_ncc(&[
            "run".to_string(),
            missing.to_string_lossy().to_string(),
            "--show-return".to_string(),
        ]),
        "ncc run renamed source with project metadata and many sources",
    );
    assert!(rendered.contains("release run matrix ok"), "{rendered}");
    assert!(
        rendered.contains("using project metadata source"),
        "expected project metadata recovery warning, got {rendered}"
    );
}

#[test]
fn release_run_recovers_renamed_source_by_principal_container_with_many_sources() {
    let root = temp_root("release_run_matrix_renamed_principal_container");
    let _ = fs::remove_dir_all(&root);
    let dir = root.join("varios");
    let real = dir.join("nombreNuevo.n");
    let other = dir.join("otroArchivo.n");
    let missing = dir.join("nombreAnterior.n");
    write_single_file_fixture(&real, "nombreAnterior");
    write_single_file_fixture(&other, "otroArchivo");

    let rendered = assert_success(
        run_ncc(&[
            "run".to_string(),
            missing.to_string_lossy().to_string(),
            "--show-return".to_string(),
        ]),
        "ncc run renamed source by principal container with many sources",
    );
    assert!(rendered.contains("release run matrix ok"), "{rendered}");
    assert!(
        rendered.contains("matching principal container"),
        "expected principal-container recovery warning, got {rendered}"
    );
}

#[test]
fn release_run_project_bridge_and_child_imports_work_from_main() {
    let root = temp_root("release_run_matrix_project_bridge");
    let _ = fs::remove_dir_all(&root);
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "init".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project init",
    );

    fs::write(
        root.join("main.n"),
        r#"use <project:child>;

matrixMain {
    ref main ->> @fn(child.matrixChild.menu);

    fa:
        return true;
}
"#,
    )
    .expect("write main");
    fs::create_dir_all(root.join("src")).expect("src");
    fs::write(
        root.join("src").join("child.n"),
        r#"use::stoud::io::<output>;

matrixChild {
    fn:
        public:
            menu() -> bool {
                public:
                    println("project child import ok");
                    return true;
            }

    fa:
        return true;
}
"#,
    )
    .expect("write child");
    assert_success(
        run_ncc(&[
            "project".to_string(),
            "sync".to_string(),
            root.to_string_lossy().to_string(),
        ]),
        "project sync",
    );

    let stdout = assert_success(
        run_ncc_in_dir(
            &[
                "run".to_string(),
                "main.n".to_string(),
                "--interpreted".to_string(),
                "--show-return".to_string(),
            ],
            &root,
        ),
        "project bridge run",
    );
    assert!(stdout.contains("project child import ok"), "{stdout}");
    assert!(stdout.contains("[return: true"), "{stdout}");
    assert!(root.join("bridge.json").is_file(), "bridge.json must exist");
    assert!(
        root.join(".ncc").join("run").join("last.json").is_file(),
        "project run route cache must exist"
    );
}

#[test]
fn release_run_accepts_n_nb_nbb_and_nm_extensions() {
    let root = temp_root("release_run_matrix_extensions");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("extension root");

    let n_file = root.join("normal.n");
    write_single_file_fixture(&n_file, "releaseRunMatrixN");
    let n_stdout = assert_success(
        run_ncc(&["run".to_string(), n_file.to_string_lossy().to_string()]),
        "run .n",
    );
    assert!(n_stdout.contains("release run matrix ok"), "{n_stdout}");

    let nb_file = example_path("basal/root_minimal.nb");
    let nb_stdout = assert_success(
        run_ncc(&["run".to_string(), nb_file.to_string_lossy().to_string()]),
        "run .nb",
    );
    assert!(nb_stdout.contains("OK: basal check"), "{nb_stdout}");

    let nbb_file = root.join("root_minimal.nbb");
    assert_success(
        run_ncc(&[
            "basal".to_string(),
            "emit-nbb".to_string(),
            nb_file.to_string_lossy().to_string(),
            "--out".to_string(),
            nbb_file.to_string_lossy().to_string(),
        ]),
        "emit .nbb",
    );
    let nbb_stdout = assert_success(
        run_ncc(&["run".to_string(), nbb_file.to_string_lossy().to_string()]),
        "run .nbb",
    );
    assert!(nbb_stdout.contains("OK: basal check"), "{nbb_stdout}");

    let nm_file = example_path("machine/object_elf64_minimal.nm");
    let nm_stdout = assert_success(
        run_ncc(&["run".to_string(), nm_file.to_string_lossy().to_string()]),
        "run .nm",
    );
    assert!(nm_stdout.contains("OK: machine check"), "{nm_stdout}");
}

#[test]
fn release_run_resolves_bare_filename_inside_many_file_folder() {
    let root = temp_root("release_run_matrix_many_files");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("src");
    write_single_file_fixture(&root.join("src").join("active.n"), "releaseRunMatrixBare");
    write_single_file_fixture(&root.join("src").join("other.n"), "releaseRunMatrixOther");

    let stdout = assert_success(
        run_ncc_in_dir(&["run".to_string(), "active.n".to_string()], &root),
        "run bare filename",
    );
    assert!(stdout.contains("release run matrix ok"), "{stdout}");
}

#[cfg(unix)]
#[test]
fn release_run_works_through_posix_sh_for_wsl_kali_ubuntu_family() {
    let root = temp_root("release_run_matrix_posix_sh");
    let _ = fs::remove_dir_all(&root);
    let file = root.join("active.n");
    write_single_file_fixture(&file, "releaseRunMatrixSh");
    let command = format!(
        "{} run {} --show-return",
        shell_quote(&ncc_bin()),
        shell_quote(&file)
    );
    let out = Command::new("sh")
        .arg("-lc")
        .arg(command)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("sh must execute ncc");
    let stdout = assert_success(out, "posix sh run");
    assert!(stdout.contains("release run matrix ok"), "{stdout}");
}

#[cfg(windows)]
#[test]
fn release_run_works_through_windows_cmd() {
    let root = temp_root("release_run_matrix_cmd");
    let _ = fs::remove_dir_all(&root);
    let file = root.join("active.n");
    write_single_file_fixture(&file, "releaseRunMatrixCmd");
    let ncc = ncc_bin();
    let ncc_dir = ncc.parent().expect("ncc binary parent");
    let file_arg = file
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_string();
    let command = format!("ncc.exe run \"{}\" --show-return", file_arg);
    let out = Command::new("cmd")
        .current_dir(ncc_dir)
        .args(["/C", command.as_str()])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("cmd must execute ncc");
    let stdout = assert_success(out, "cmd run");
    assert!(stdout.contains("release run matrix ok"), "{stdout}");
}

#[cfg(windows)]
#[test]
fn release_run_works_through_powershell() {
    let root = temp_root("release_run_matrix_powershell");
    let _ = fs::remove_dir_all(&root);
    let file = root.join("active.n");
    write_single_file_fixture(&file, "releaseRunMatrixPowerShell");
    let command = format!(
        "& '{}' run '{}' --show-return",
        ncc_bin().to_string_lossy().replace('\'', "''"),
        file.to_string_lossy().replace('\'', "''")
    );
    let out = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            command.as_str(),
        ])
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("powershell must execute ncc");
    let stdout = assert_success(out, "powershell run");
    assert!(stdout.contains("release run matrix ok"), "{stdout}");
}

#[cfg(target_os = "macos")]
#[test]
fn release_run_works_through_macos_sh() {
    let root = temp_root("release_run_matrix_macos_sh");
    let _ = fs::remove_dir_all(&root);
    let file = root.join("active.n");
    write_single_file_fixture(&file, "releaseRunMatrixMacos");
    let command = format!(
        "{} run {} --show-return",
        shell_quote(&ncc_bin()),
        shell_quote(&file)
    );
    let out = Command::new("sh")
        .arg("-lc")
        .arg(command)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("macOS sh must execute ncc");
    let stdout = assert_success(out, "macos sh run");
    assert!(stdout.contains("release run matrix ok"), "{stdout}");
}
