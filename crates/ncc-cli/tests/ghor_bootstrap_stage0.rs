use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn native_host_target() -> &'static str {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("x86_64", "windows") => "x86_64-pc-windows-msvc",
        ("aarch64", "windows") => "aarch64-pc-windows-msvc",
        ("x86_64", "macos") => "x86_64-apple-darwin",
        ("aarch64", "macos") => "aarch64-apple-darwin",
        ("aarch64", "linux") => "aarch64-unknown-linux-gnu",
        ("riscv64", "linux") => "riscv64gc-unknown-linux-gnu",
        _ => "x86_64-unknown-linux-gnu",
    }
}

struct LocalRegistryServer {
    base: String,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl LocalRegistryServer {
    fn start(root: PathBuf) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local registry HTTP server");
        listener
            .set_nonblocking(true)
            .expect("nonblocking registry listener");
        let base = format!(
            "http://{}",
            listener.local_addr().expect("registry address")
        );
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker = std::thread::spawn(move || {
            while !worker_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream
                            .set_nonblocking(false)
                            .expect("blocking accepted registry stream");
                        stream
                            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                            .expect("registry read timeout");
                        stream
                            .set_write_timeout(Some(std::time::Duration::from_secs(5)))
                            .expect("registry write timeout");
                        serve_registry_file(stream, &root);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            base,
            stop,
            worker: Some(worker),
        }
    }
}

impl Drop for LocalRegistryServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.join().expect("join registry server");
        }
    }
}

fn serve_registry_file(mut stream: TcpStream, root: &std::path::Path) {
    let mut request = Vec::new();
    let mut header_end = None;
    let mut content_length = 0_usize;
    loop {
        let mut chunk = [0_u8; 16 * 1024];
        let Ok(read) = stream.read(&mut chunk) else {
            return;
        };
        if read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read]);
        if header_end.is_none() {
            header_end = request
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|at| at + 4);
            if let Some(end) = header_end {
                let headers = String::from_utf8_lossy(&request[..end]);
                content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.split_once(':').and_then(|(name, value)| {
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                    })
                    .unwrap_or(0);
                if headers.lines().any(|line| {
                    line.split_once(':').is_some_and(|(name, value)| {
                        name.eq_ignore_ascii_case("expect")
                            && value.trim().eq_ignore_ascii_case("100-continue")
                    })
                }) {
                    let _ = stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n");
                }
            }
        }
        if header_end.is_some_and(|end| request.len() >= end + content_length) {
            break;
        }
        if request.len() > 16 * 1024 * 1024 {
            write_http(&mut stream, 413, b"too large", None);
            return;
        }
    }
    let Some(header_end) = header_end else {
        write_http(&mut stream, 400, b"headers", None);
        return;
    };
    let headers = String::from_utf8_lossy(&request[..header_end]);
    let mut lines = headers.lines();
    let Some(first) = lines.next() else {
        return;
    };
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("");
    let relative = parts
        .next()
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/")
        .trim_start_matches('/');
    if relative.is_empty() || relative.split('/').any(|part| part == "..") {
        write_http(&mut stream, 400, b"path", None);
        return;
    }
    let authorized = headers.lines().skip(1).any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.eq_ignore_ascii_case("authorization")
                && value.trim().starts_with("Bearer ")
                && value.trim().len() > "Bearer ".len()
        })
    });
    let body_end = header_end.saturating_add(content_length).min(request.len());
    let body = &request[header_end..body_end];

    if relative.starts_with("api/v1/") {
        if method != "GET" && !authorized {
            write_http(&mut stream, 401, b"auth", None);
            return;
        }
        if let Ok(mut events) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("api-events.log"))
        {
            let _ = writeln!(events, "{method} /{relative} bytes={}", body.len());
        }
        write_http(&mut stream, 200, br#"{"ok":true}"#, None);
        return;
    }

    let path = root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
    if method == "PUT" {
        if !authorized {
            write_http(&mut stream, 401, b"auth", None);
            return;
        }
        if let Some(parent) = path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                write_http(&mut stream, 500, b"directory", None);
                return;
            }
        }
        if std::fs::write(&path, body).is_err() {
            write_http(&mut stream, 500, b"write", None);
            return;
        }
        write_http(&mut stream, 200, br#"{"ok":true}"#, None);
        return;
    }
    if method != "GET" {
        write_http(&mut stream, 405, b"method", None);
        return;
    }
    let Ok(bytes) = std::fs::read(path) else {
        write_http(&mut stream, 404, b"missing", None);
        return;
    };
    let etag = format!("\"{}\"", sha256(&bytes));
    let not_modified = headers.lines().skip(1).any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.eq_ignore_ascii_case("if-none-match") && value.trim() == etag
        })
    });
    if not_modified {
        write_http(&mut stream, 304, b"", Some(&etag));
    } else {
        write_http(&mut stream, 200, &bytes, Some(&etag));
    }
}

fn write_http(stream: &mut TcpStream, status: u16, body: &[u8], etag: Option<&str>) {
    let reason = match status {
        200 => "OK",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        413 => "Content Too Large",
        500 => "Internal Server Error",
        404 => "Not Found",
        _ => "Method Not Allowed",
    };
    let etag = etag
        .map(|value| format!("ETag: {value}\r\n"))
        .unwrap_or_default();
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\n{etag}Connection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .expect("write registry response");
    stream.write_all(body).expect("write registry body");
}

fn binary(name: &str) -> PathBuf {
    std::env::var_os(format!("CARGO_BIN_EXE_{name}"))
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("CARGO_BIN_EXE_{name} must be set"))
}

fn run_ghor(args: &[&str], stage1: Option<&PathBuf>, force_stage0: bool) -> Output {
    let mut command = Command::new(binary("ghor"));
    command.args(args).env("NC_NO_COLOR", "1");
    match stage1 {
        Some(path) => {
            command.env("NC_GHOR_STAGE1", path);
        }
        None => {
            command.env_remove("NC_GHOR_STAGE1");
        }
    }
    if force_stage0 {
        command.env("NC_GHOR_FORCE_STAGE0", "1");
    } else {
        command.env_remove("NC_GHOR_FORCE_STAGE0");
    }
    command.output().expect("ghor command must execute")
}

#[test]
fn bootstrap_reports_the_canonical_nc_stage1_without_claiming_completion() {
    let output = run_ghor(&["bootstrap", "--json"], None, true);
    assert!(
        output.status.success(),
        "bootstrap stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("bootstrap JSON");
    assert_eq!(report["schema"], "ghor.bootstrap.v1");
    assert_eq!(report["active_executor"], "rust-stage0");
    assert_eq!(report["stage1"]["language"], "nC");
    assert_eq!(report["stage1"]["module_count"], 29);
    assert_eq!(report["commands"]["count"], 84);
    assert_eq!(report["commands"]["native_e2e"], 4);
    assert_eq!(report["commands"]["stage1_native_e2e"], 80);
    assert_eq!(report["commands"]["stage1_native_partial"], 0);
    assert_eq!(report["commands"]["stage1_pending"], 0);
    assert_eq!(report["bootstrap_complete"], false);
    assert!(report["diagnostics"]["entries"].as_u64().unwrap_or(0) >= 2_000);
}

#[test]
fn nc_stage1_command_catalog_covers_the_cli_surface() {
    let output = run_ghor(&["--list"], None, true);
    assert!(output.status.success());
    let commands = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert_eq!(commands.len(), 84);

    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    let stage1 = std::fs::read_to_string(workspace.join("tools/ghor/nc/src/command.n"))
        .expect("nC stage1 command catalog");
    for command in commands {
        assert!(
            stage1.contains(&format!("\"{command}\"")),
            "nC stage1 command catalog is missing {command}"
        );
    }
}

#[test]
fn configured_stage1_receives_commands_and_force_stage0_remains_available() {
    let ncc = binary("ncc");
    let delegated = run_ghor(&["--version"], Some(&ncc), false);
    assert!(delegated.status.success());
    assert!(String::from_utf8_lossy(&delegated.stdout).starts_with("ncc "));

    let recovered = run_ghor(&["--version"], Some(&ncc), true);
    assert!(recovered.status.success());
    assert!(String::from_utf8_lossy(&recovered.stdout).starts_with("ghor "));

    let retained_stage0 = run_ghor(&["--list"], Some(&ncc), false);
    assert!(retained_stage0.status.success());
    assert!(String::from_utf8_lossy(&retained_stage0.stdout)
        .lines()
        .any(|line| line == "manifest"));

    let help = run_ghor(&["run", "--help"], Some(&ncc), false);
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("Usage: ghor run"));
}

#[test]
fn native_nc_stage1_parses_a_real_manifest() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let out_dir = std::env::temp_dir().join(format!(
        "ncc-ghor-native-stage1-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&out_dir).expect("stage1 output directory");
    let executable = out_dir.join(if cfg!(windows) {
        "ghor-stage1.exe"
    } else {
        "ghor-stage1"
    });
    let project = workspace.join("tools/ghor/nc/main.n");
    let build = Command::new(binary("ncc"))
        .arg("build")
        .arg(&project)
        .arg(&executable)
        .arg("--out-dir")
        .arg(&out_dir)
        .arg("--profile")
        .arg("release-fast")
        .arg("--no-incremental")
        .output()
        .expect("build native nC stage1");
    assert!(
        build.status.success(),
        "stage1 build stdout={} stderr={}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let self_test = Command::new(&executable)
        .output()
        .expect("run native stage1 self-test");
    assert!(
        self_test.status.success(),
        "stage1 self-test status={} stdout={} stderr={}",
        self_test.status,
        String::from_utf8_lossy(&self_test.stdout),
        String::from_utf8_lossy(&self_test.stderr)
    );

    let native_bootstrap = Command::new(&executable)
        .args(["bootstrap", "--json"])
        .output()
        .expect("run native stage1 bootstrap authority");
    assert!(
        native_bootstrap.status.success(),
        "native bootstrap stdout={} stderr={}",
        String::from_utf8_lossy(&native_bootstrap.stdout),
        String::from_utf8_lossy(&native_bootstrap.stderr)
    );
    let native_bootstrap_report: serde_json::Value =
        serde_json::from_slice(&native_bootstrap.stdout).expect("native bootstrap JSON");
    assert_eq!(native_bootstrap_report["active_executor"], "nc-stage1");
    assert_eq!(native_bootstrap_report["stage1"]["module_count"], 29);
    assert_eq!(native_bootstrap_report["commands"]["stage1_native_e2e"], 80);
    assert_eq!(
        native_bootstrap_report["commands"]["stage1_native_partial"],
        0
    );
    assert_eq!(native_bootstrap_report["commands"]["stage1_pending"], 0);

    let native_contracts = Command::new(&executable)
        .args(["report", "command-contracts", "--json"])
        .output()
        .expect("run native stage1 command contracts");
    assert!(native_contracts.status.success());
    let native_contracts_report: serde_json::Value =
        serde_json::from_slice(&native_contracts.stdout).expect("native command contracts JSON");
    for (name, route) in [
        ("cargo-exec", "nc-stage1-native-cargo-execution"),
        ("build-script", "nc-stage1-native-build-script-execution"),
        ("proc-macro", "nc-stage1-native-proc-macro-execution"),
        ("interop", "nc-stage1-native-foreign-interop"),
        ("external-compat", "nc-stage1-native-external-compatibility"),
    ] {
        let command = native_contracts_report["commands"]
            .as_array()
            .expect("native command contracts")
            .iter()
            .find(|entry| entry["name"] == name)
            .unwrap_or_else(|| panic!("missing native command {name}"));
        assert_eq!(command["route"], route);
        assert_eq!(command["status"], "stage1_native_e2e");
    }

    let interop = out_dir.join("native-foreign-interop");
    let native_interop = Command::new(&executable)
        .args(["interop", "scaffold", "--out"])
        .arg(&interop)
        .output()
        .expect("run native stage1 foreign interop scaffold");
    assert!(
        native_interop.status.success(),
        "native interop stdout={} stderr={}",
        String::from_utf8_lossy(&native_interop.stdout),
        String::from_utf8_lossy(&native_interop.stderr)
    );
    let native_interop_report: serde_json::Value =
        serde_json::from_slice(&native_interop.stdout).expect("native interop JSON");
    assert_eq!(native_interop_report["executor"], "nc-stage1");
    assert_eq!(native_interop_report["status"], "scaffolded");
    assert!(interop.join("ghor_interop_manifest.json").is_file());

    for (host, marker) in [
        ("nC", "src/lexer.n"),
        ("c", "src/lexer.c"),
        ("python", "src/lexer.py"),
    ] {
        let forge = out_dir.join(format!("native-language-forge-{}", host.to_lowercase()));
        let output = Command::new(&executable)
            .args(["lang", "new", &format!("native_{host}")])
            .arg("--host")
            .arg(host)
            .arg("--out")
            .arg(&forge)
            .output()
            .expect("run native language forge");
        assert!(
            output.status.success(),
            "native lang {host} stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("native language forge JSON");
        assert_eq!(report["schema"], "ghor.language.new.v1");
        assert_eq!(report["executor"], "nc-stage1");
        assert!(forge.join(marker).is_file(), "missing {host} forge marker");
        let conformance: serde_json::Value = serde_json::from_slice(
            &std::fs::read(forge.join("reports/conformance_scaffold.json"))
                .expect("native conformance scaffold"),
        )
        .expect("native conformance JSON");
        assert_eq!(conformance["executor"], "nc-stage1");
    }
    let rejected_forge = out_dir.join("native-language-forge-unsupported");
    let rejected_language = Command::new(&executable)
        .args(["lang", "new", "unsupported_forge", "--host", "unknown"])
        .arg("--out")
        .arg(&rejected_forge)
        .output()
        .expect("reject unsupported native language forge");
    assert_eq!(rejected_language.status.code(), Some(2));
    assert!(!rejected_forge.exists());
    assert!(String::from_utf8_lossy(&rejected_language.stdout)
        .contains("E_GHOR_LANGUAGE_UNSUPPORTED_001"));

    let fixture = out_dir.join("ghor.toml");
    std::fs::write(
        &fixture,
        "[package]\nname = \"native_stage1\"\nversion = \"1.2.3\"\n\n[profiles.release]\nopt = 3\n",
    )
    .expect("write manifest fixture");
    let output = Command::new(&executable)
        .arg("manifest")
        .arg("--manifest-path")
        .arg(&fixture)
        .arg("--json")
        .output()
        .expect("run native nC stage1");
    assert!(
        output.status.success(),
        "stage1 stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stage1 manifest JSON");
    assert_eq!(report["schema"], "ghor.manifest.v1");
    assert_eq!(report["executor"], "nc-stage1");
    assert_eq!(report["package"]["name"], "native_stage1");
    assert_eq!(report["package"]["version"], "1.2.3");
    assert_eq!(report["profiles"]["release"]["opt"], 3);
    assert_eq!(report["diagnostics"].as_array().map(Vec::len), Some(0));

    let package_project = out_dir.join("native-package-project");
    std::fs::create_dir_all(package_project.join("src")).expect("package source directory");
    let package_manifest = package_project.join("ghor.toml");
    std::fs::write(
        &package_manifest,
        "[package]\nname = \"native_archive\"\nversion = \"2.0.0\"\ndescription = \"native reproducible package\"\nlicense = \"MIT\"\nrepository = \"https://example.invalid/native-archive\"\nkind = \"bin\"\n",
    )
    .expect("package manifest");
    std::fs::write(
        package_project.join("src/main.n"),
        "nativeArchive { run :: fn() -> bool { return true; } ref main ->> run; fa: return true; }\n",
    )
    .expect("package source");
    std::fs::write(package_project.join("README.md"), "# Native archive\n")
        .expect("package readme");

    let stage1_ncc = binary("ncc");
    let checked = Command::new(&executable)
        .arg("check")
        .arg("--manifest-path")
        .arg(&package_manifest)
        .arg("--json")
        .env("NC_GHOR_NCC", &stage1_ncc)
        .output()
        .expect("run native stage1 check");
    assert!(
        checked.status.success(),
        "native check stdout={} stderr={}",
        String::from_utf8_lossy(&checked.stdout),
        String::from_utf8_lossy(&checked.stderr)
    );
    let check_report: serde_json::Value =
        serde_json::from_slice(&checked.stdout).expect("native check JSON");
    assert_eq!(check_report["schema"], "ghor.check.v1");
    assert_eq!(check_report["executor"], "nc-stage1");
    assert_eq!(check_report["status"], "ok");
    assert_eq!(check_report["checked"], 1);

    let invalid_project = out_dir.join("invalid-check-project");
    std::fs::create_dir_all(invalid_project.join("src")).expect("invalid source directory");
    let invalid_manifest = invalid_project.join("ghor.toml");
    std::fs::write(
        &invalid_manifest,
        "[package]\nname = \"invalid_check\"\nversion = \"0.1.0\"\nkind = \"bin\"\n",
    )
    .expect("invalid check manifest");
    std::fs::write(
        invalid_project.join("src/main.n"),
        "invalidCheck { broken :: fn( -> bool { return true; } }\n",
    )
    .expect("invalid check source");
    let rejected = Command::new(&executable)
        .arg("check")
        .arg("--manifest-path")
        .arg(&invalid_manifest)
        .arg("--json")
        .env("NC_GHOR_NCC", &stage1_ncc)
        .output()
        .expect("run native rejected check");
    assert!(!rejected.status.success());
    let rejected_report: serde_json::Value =
        serde_json::from_slice(&rejected.stdout).expect("rejected check JSON");
    assert_eq!(rejected_report["executor"], "nc-stage1");
    assert_eq!(rejected_report["status"], "error");
    assert_eq!(rejected_report["checked"], 1);
    assert_eq!(
        rejected_report["diagnostics"][0]["code"],
        "E_GHOR_CHECK_001"
    );

    let built = Command::new(&executable)
        .arg("build")
        .arg("--manifest-path")
        .arg(&package_manifest)
        .arg("--profile")
        .arg("debug")
        .arg("--target")
        .arg(native_host_target())
        .arg("--json")
        .env("NC_GHOR_NCC", &stage1_ncc)
        .output()
        .expect("run native stage1 build");
    assert!(
        built.status.success(),
        "native build stdout={} stderr={}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
    let build_report: serde_json::Value =
        serde_json::from_slice(&built.stdout).expect("native build JSON");
    assert_eq!(build_report["schema"], "ghor.build.v1");
    assert_eq!(build_report["executor"], "nc-stage1");
    assert_eq!(build_report["status"], "built");
    let artifact_report = serde_json::from_slice::<serde_json::Value>(
        &std::fs::read(
            build_report["artifacts"][0]["artifact"]
                .as_str()
                .expect("stage1 artifact path"),
        )
        .expect("stage1 artifact bytes"),
    )
    .expect("stage1 artifact JSON");
    let native_build = &artifact_report["native_build"];
    for field in ["executable", "nm", "object"] {
        assert!(PathBuf::from(
            native_build[field]
                .as_str()
                .unwrap_or_else(|| panic!("native build {field}"))
        )
        .is_file());
    }

    std::fs::create_dir_all(package_project.join("tests")).expect("native tests directory");
    std::fs::write(
        package_project.join("tests/smoke.n"),
        "nativeArchiveSmoke { run :: fn() -> bool { return true; } ref main ->> run; fa: return true; }\n",
    )
    .expect("native smoke source");
    let tested = Command::new(&executable)
        .arg("test")
        .arg("--manifest-path")
        .arg(&package_manifest)
        .arg("--target")
        .arg(native_host_target())
        .arg("--json")
        .env("NC_GHOR_NCC", &stage1_ncc)
        .output()
        .expect("run native stage1 test");
    assert!(
        tested.status.success(),
        "native test stdout={} stderr={}",
        String::from_utf8_lossy(&tested.stdout),
        String::from_utf8_lossy(&tested.stderr)
    );
    let test_report: serde_json::Value =
        serde_json::from_slice(&tested.stdout).expect("native test JSON");
    assert_eq!(test_report["schema"], "ghor.test.v1");
    assert_eq!(test_report["executor"], "nc-stage1");
    assert_eq!(test_report["status"], "ok");
    assert_eq!(test_report["checked"], 1);
    assert_eq!(test_report["executions"][0]["mode"], "native-execution");

    let ran = Command::new(&executable)
        .arg("run")
        .arg("--manifest-path")
        .arg(&package_manifest)
        .arg("--profile")
        .arg("debug")
        .arg("--json")
        .args(["--", "--target", "child-program-value"])
        .env("NC_GHOR_NCC", &stage1_ncc)
        .env("NC_GHOR_HOST_TARGET", native_host_target())
        .output()
        .expect("run package through native stage1");
    assert!(
        ran.status.success(),
        "native run stdout={} stderr={}",
        String::from_utf8_lossy(&ran.stdout),
        String::from_utf8_lossy(&ran.stderr)
    );
    let run_report: serde_json::Value =
        serde_json::from_slice(&ran.stdout).expect("native run JSON");
    assert_eq!(run_report["schema"], "ghor.run.v1");
    assert_eq!(run_report["executor"], "nc-stage1");
    assert_eq!(run_report["status"], "ok");
    assert_eq!(run_report["execution"], "native_machine_executable");
    assert_eq!(run_report["exit_code"], 0);
    for field in ["executable", "nm", "object"] {
        assert!(PathBuf::from(
            run_report["build"][field]
                .as_str()
                .unwrap_or_else(|| panic!("native run build {field}"))
        )
        .is_file());
    }

    std::fs::create_dir_all(package_project.join("benches")).expect("native benches directory");
    std::fs::write(
        package_project.join("benches/native_bench.n"),
        "nativeArchiveBench { run :: fn() -> bool { value >> int <- 1; value += 1; return value == 2; } ref main ->> run; fa: return true; }\n",
    )
    .expect("native benchmark source");
    let benched = Command::new(&executable)
        .arg("bench")
        .arg("--manifest-path")
        .arg(&package_manifest)
        .args(["--bench", "native_bench"])
        .args(["--iterations", "3"])
        .args(["--warmup", "0"])
        .arg("--target")
        .arg(native_host_target())
        .arg("--json")
        .env("NC_GHOR_NCC", &stage1_ncc)
        .env("NC_GHOR_HOST_TARGET", native_host_target())
        .output()
        .expect("benchmark package through native stage1");
    assert!(
        benched.status.success(),
        "native bench stdout={} stderr={}",
        String::from_utf8_lossy(&benched.stdout),
        String::from_utf8_lossy(&benched.stderr)
    );
    let bench_report: serde_json::Value =
        serde_json::from_slice(&benched.stdout).expect("native bench JSON");
    assert_eq!(bench_report["schema"], "ghor.bench.v1");
    assert_eq!(bench_report["executor"], "nc-stage1");
    assert_eq!(bench_report["status"], "ok");
    assert_eq!(bench_report["profile"], "release-fast");
    assert_eq!(bench_report["benchmarks"][0]["name"], "native_bench");
    assert_eq!(bench_report["benchmarks"][0]["warmup"], 0);
    assert_eq!(bench_report["benchmarks"][0]["iterations"], 3);
    assert_eq!(
        bench_report["benchmarks"][0]["samples_ns"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
    assert!(bench_report["benchmarks"][0]["mean_ns"]
        .as_i64()
        .is_some_and(|value| value >= 0));
    for field in ["executable", "nm", "object"] {
        assert!(PathBuf::from(
            bench_report["benchmarks"][0][field]
                .as_str()
                .unwrap_or_else(|| panic!("native bench {field}"))
        )
        .is_file());
    }

    let mut archives = Vec::new();
    for name in ["package-a", "package-b"] {
        let package_out = out_dir.join(name);
        let packaged = Command::new(&executable)
            .arg("package")
            .arg("--manifest-path")
            .arg(&package_manifest)
            .arg("--out")
            .arg(&package_out)
            .arg("--json")
            .output()
            .expect("run native package");
        assert!(
            packaged.status.success(),
            "package stdout={} stderr={}",
            String::from_utf8_lossy(&packaged.stdout),
            String::from_utf8_lossy(&packaged.stderr)
        );
        let package_report: serde_json::Value =
            serde_json::from_slice(&packaged.stdout).expect("native package JSON");
        assert_eq!(package_report["executor"], "nc-stage1");
        assert_eq!(package_report["status"], "packaged");
        assert_eq!(package_report["verification"]["status"], "verified");
        assert_eq!(
            package_report["verification"]["verification_scope"],
            "archive-structure"
        );
        assert_eq!(package_report["verification"]["source_checks"], 0);
        assert_eq!(package_report["verification"]["source_members"], 3);
        let archive = PathBuf::from(
            package_report["archive"]
                .as_str()
                .expect("native package archive path"),
        );
        assert_eq!(
            package_report["checksum"],
            sha256(&std::fs::read(&archive).expect("native package bytes"))
        );
        archives.push(archive);
    }
    assert_eq!(
        std::fs::read(&archives[0]).expect("first archive"),
        std::fs::read(&archives[1]).expect("second archive"),
        "native package output must be byte-for-byte reproducible"
    );
    let tar_list = Command::new("tar")
        .arg("-tzf")
        .arg(&archives[0])
        .output()
        .expect("list native package with host tar");
    assert!(
        tar_list.status.success(),
        "tar stderr={}",
        String::from_utf8_lossy(&tar_list.stderr)
    );
    let tar_members = String::from_utf8_lossy(&tar_list.stdout);
    assert!(tar_members.contains("native_archive-2.0.0/ghor.toml"));
    assert!(tar_members.contains("native_archive-2.0.0/src/main.n"));
    assert!(tar_members.contains("native_archive-2.0.0/benches/native_bench.n"));

    let cargo_project = out_dir.join("cargo-config-project");
    let cargo_dir = cargo_project.join(".cargo");
    std::fs::create_dir_all(cargo_dir.join("certs")).expect("Cargo config directory");
    let cargo_manifest = cargo_project.join("ghor.toml");
    std::fs::write(
        &cargo_manifest,
        "[package]\nname = \"cargo_config_native\"\nversion = \"0.1.0\"\n",
    )
    .expect("Cargo config manifest");
    let registry_root = out_dir.join("cargo-config-registry");
    std::fs::create_dir_all(&registry_root).expect("Cargo config registry");
    std::fs::write(
        cargo_dir.join("config.toml"),
        format!(
            "[registry]\ndefault = \"mirror\"\n\n[registries.mirror]\nindex = \"{}\"\ntoken = \"secret-not-reported\"\n\n[net]\noffline = true\nretry = 4\n\n[http]\ntimeout = 11\ncainfo = \"certs/ca.pem\"\nproxy-cainfo = \"certs/proxy-ca.pem\"\ncheck-revoke = false\n\n[git]\ncainfo = \"certs/git-ca.pem\"\nssl-verify = false\n",
            registry_root.to_string_lossy().replace('\\', "/")
        ),
    )
    .expect("Cargo-compatible config fixture");
    let cargo_config_output = Command::new(&executable)
        .args(["registry", "cargo-config", "--manifest-path"])
        .arg(&cargo_manifest)
        .arg("--json")
        .output()
        .expect("run native Cargo config report");
    assert!(
        cargo_config_output.status.success(),
        "Cargo config stdout={} stderr={}",
        String::from_utf8_lossy(&cargo_config_output.stdout),
        String::from_utf8_lossy(&cargo_config_output.stderr)
    );
    let cargo_config_report: serde_json::Value =
        serde_json::from_slice(&cargo_config_output.stdout).expect("Cargo config JSON");
    assert_eq!(cargo_config_report["executor"], "nc-stage1");
    assert_eq!(cargo_config_report["registry_transport"], "filesystem");
    assert_eq!(
        cargo_config_report["config"]["registry"]["default"],
        "mirror"
    );
    assert_eq!(cargo_config_report["config"]["net"]["offline"], "true");
    assert_eq!(cargo_config_report["config"]["net"]["retry"], "4");
    assert_eq!(cargo_config_report["config"]["http"]["timeout"], "11");
    assert_eq!(cargo_config_report["config"]["git"]["ssl-verify"], "false");
    assert_eq!(
        cargo_config_report["config"]["selected_registry_name"],
        "mirror"
    );
    assert_eq!(
        cargo_config_report["config"]["registries"]["mirror"]["token"],
        "<redacted>"
    );
    assert!(!String::from_utf8_lossy(&cargo_config_output.stdout).contains("secret-not-reported"));
    let curl_policy = std::fs::read_to_string(
        cargo_config_report["config"]["curl_policy"]
            .as_str()
            .expect("curl policy path"),
    )
    .expect("materialized curl policy");
    assert!(curl_policy.contains("retry = 4"));
    assert!(curl_policy.contains("max-time = 11"));
    assert!(curl_policy.contains("ssl-no-revoke"));
    assert!(curl_policy.contains("certs\\ca.pem") || curl_policy.contains("certs/ca.pem"));
    let git_policy = std::fs::read_to_string(
        cargo_config_report["config"]["git_policy"]
            .as_str()
            .expect("git policy path"),
    )
    .expect("materialized git policy");
    assert!(git_policy.contains("sslVerify = false"));
    assert!(git_policy.contains("git-ca.pem"));

    let provider_project = out_dir.join("cargo-provider-project");
    std::fs::create_dir_all(provider_project.join(".cargo"))
        .expect("credential provider config directory");
    let provider_manifest = provider_project.join("ghor.toml");
    std::fs::write(
        &provider_manifest,
        "[package]\nname = \"cargo_provider_native\"\nversion = \"0.1.0\"\n",
    )
    .expect("credential provider manifest");
    let provider_alias = if cfg!(windows) {
        "[\"powershell.exe\", \"-NoProfile\", \"-Command\", \"Write-Output phase12-secret\"]"
    } else {
        "[\"sh\", \"-c\", \"printf phase12-secret\"]"
    };
    std::fs::write(
        provider_project.join(".cargo/config.toml"),
        format!(
            "[registries.secure]\nindex = \"https://example.invalid/index\"\ncredential-provider = \"phase12-token\"\n\n[credential-alias]\nphase12-token = {provider_alias}\n"
        ),
    )
    .expect("credential provider Cargo config");
    let provider_output = Command::new(&executable)
        .args(["registry", "cargo-config", "--manifest-path"])
        .arg(&provider_manifest)
        .args(["--registry", "secure", "--json"])
        .output()
        .expect("run native credential provider");
    assert!(
        provider_output.status.success(),
        "provider stdout={} stderr={}",
        String::from_utf8_lossy(&provider_output.stdout),
        String::from_utf8_lossy(&provider_output.stderr)
    );
    let provider_report: serde_json::Value =
        serde_json::from_slice(&provider_output.stdout).expect("credential provider JSON");
    assert_eq!(provider_report["executor"], "nc-stage1");
    assert_eq!(provider_report["credential_status"], "available");
    assert_eq!(provider_report["config"]["token_available"], true);
    assert!(!String::from_utf8_lossy(&provider_output.stdout).contains("phase12-secret"));

    let workspace_fixture = out_dir.join("workspace");
    for member in ["app", "tool", "shared", "skip"] {
        let member_root = workspace_fixture.join("crates").join(member);
        std::fs::create_dir_all(member_root.join("src")).expect("workspace member directory");
        std::fs::create_dir_all(member_root.join("tests"))
            .expect("workspace member tests directory");
    }
    std::fs::write(
        workspace_fixture.join("ghor.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"crates/skip\"]\ndefault-members = [\"crates/app\"]\n\n[workspace.package]\nversion = \"1.2.3\"\nedition = \"2026\"\nlicense = \"MIT\"\n\n[workspace.dependencies]\nshared_dep = { path = \"crates/shared\", version = \"0.5.0\" }\n\n[workspace.lints.rust]\nunsafe_code = \"deny\"\n",
    )
    .expect("workspace manifest");
    std::fs::write(
        workspace_fixture.join("crates/app/ghor.toml"),
        "[package]\nname = \"workspace_app\"\nversion.workspace = true\nedition.workspace = true\nlicense.workspace = true\nkind = \"bin\"\n\n[dependencies]\nshared_dep.workspace = true\n\n[lints]\nworkspace = true\n",
    )
    .expect("workspace app manifest");
    std::fs::write(
        workspace_fixture.join("crates/tool/ghor.toml"),
        "[package]\nname = \"workspace_tool\"\nversion = \"0.1.0\"\nkind = \"bin\"\n",
    )
    .expect("workspace tool manifest");
    std::fs::write(
        workspace_fixture.join("crates/shared/ghor.toml"),
        "[package]\nname = \"shared_dep\"\nversion = \"0.5.0\"\nkind = \"lib\"\n",
    )
    .expect("workspace shared manifest");
    std::fs::write(
        workspace_fixture.join("crates/skip/ghor.toml"),
        "[package]\nname = \"workspace_skip\"\nversion = \"0.1.0\"\nkind = \"bin\"\n",
    )
    .expect("workspace excluded manifest");
    for (member, container, executable) in [
        ("app", "workspaceApp", true),
        ("tool", "workspaceTool", true),
        ("shared", "workspaceShared", false),
    ] {
        let body = if executable {
            format!(
                "{container} {{ run :: fn() -> bool {{ return true; }} ref main ->> run; fa: return true; }}\n"
            )
        } else {
            format!("{container} {{ value :: fn() -> int {{ return 7; }} fa: return true; }}\n")
        };
        std::fs::write(
            workspace_fixture
                .join("crates")
                .join(member)
                .join("src/main.n"),
            body,
        )
        .expect("workspace source");
        let smoke = format!(
            "{container}Smoke {{ run :: fn() -> bool {{ return true; }} ref main ->> run; fa: return true; }}\n"
        );
        std::fs::write(
            workspace_fixture
                .join("crates")
                .join(member)
                .join("tests")
                .join(format!("{container}Smoke.n")),
            smoke,
        )
        .expect("workspace smoke source");
    }
    std::fs::write(
        workspace_fixture.join("crates/skip/src/main.n"),
        "this source is intentionally invalid and must remain excluded\n",
    )
    .expect("excluded workspace source");
    let app_manifest = workspace_fixture.join("crates/app/ghor.toml");
    let workspace_output = Command::new(&executable)
        .arg("workspace")
        .arg("metadata")
        .arg("--manifest-path")
        .arg(&app_manifest)
        .arg("--json")
        .output()
        .expect("run native workspace metadata");
    assert!(
        workspace_output.status.success(),
        "workspace stdout={} stderr={}",
        String::from_utf8_lossy(&workspace_output.stdout),
        String::from_utf8_lossy(&workspace_output.stderr)
    );
    let workspace_report: serde_json::Value =
        serde_json::from_slice(&workspace_output.stdout).expect("workspace metadata JSON");
    assert_eq!(workspace_report["schema"], "ghor.workspace.metadata.v2");
    assert_eq!(workspace_report["executor"], "nc-stage1");
    assert_eq!(workspace_report["virtual_manifest"], true);
    assert_eq!(
        workspace_report["members"].as_array().map(Vec::len),
        Some(3)
    );
    assert_eq!(
        workspace_report["default_members"][0]["package"],
        "workspace_app"
    );
    assert!(!workspace_report["members"]
        .as_array()
        .expect("workspace members")
        .iter()
        .any(|member| member["package"] == "workspace_skip"));

    let workspace_check = Command::new(&executable)
        .arg("check")
        .arg("--manifest-path")
        .arg(&app_manifest)
        .arg("--workspace")
        .arg("--json")
        .env("NC_GHOR_NCC", &stage1_ncc)
        .output()
        .expect("run native workspace check");
    assert!(
        workspace_check.status.success(),
        "workspace check stdout={} stderr={}",
        String::from_utf8_lossy(&workspace_check.stdout),
        String::from_utf8_lossy(&workspace_check.stderr)
    );
    let workspace_check_report: serde_json::Value =
        serde_json::from_slice(&workspace_check.stdout).expect("workspace check JSON");
    assert_eq!(workspace_check_report["executor"], "nc-stage1");
    assert_eq!(workspace_check_report["status"], "ok");
    assert_eq!(workspace_check_report["checked"], 3);
    assert_eq!(
        workspace_check_report["packages"].as_array().map(Vec::len),
        Some(3)
    );

    let workspace_test = Command::new(&executable)
        .arg("test")
        .arg("--manifest-path")
        .arg(&app_manifest)
        .arg("--workspace")
        .arg("--target")
        .arg(native_host_target())
        .arg("--json")
        .env("NC_GHOR_NCC", &stage1_ncc)
        .output()
        .expect("run native workspace tests");
    assert!(
        workspace_test.status.success(),
        "workspace test stdout={} stderr={}",
        String::from_utf8_lossy(&workspace_test.stdout),
        String::from_utf8_lossy(&workspace_test.stderr)
    );
    let workspace_test_report: serde_json::Value =
        serde_json::from_slice(&workspace_test.stdout).expect("workspace test JSON");
    assert_eq!(workspace_test_report["executor"], "nc-stage1");
    assert_eq!(workspace_test_report["status"], "ok");
    assert_eq!(workspace_test_report["checked"], 3);
    assert_eq!(
        workspace_test_report["executions"].as_array().map(Vec::len),
        Some(3)
    );

    let inherited_manifest = Command::new(&executable)
        .arg("manifest")
        .arg("--manifest-path")
        .arg(&app_manifest)
        .arg("--json")
        .output()
        .expect("run inherited workspace manifest");
    assert!(inherited_manifest.status.success());
    let inherited_report: serde_json::Value =
        serde_json::from_slice(&inherited_manifest.stdout).expect("inherited manifest JSON");
    assert_eq!(inherited_report["package"]["version"], "1.2.3");
    assert_eq!(inherited_report["package"]["edition"], "2026");
    assert_eq!(inherited_report["workspace"]["members"][0], "crates/*");
    assert_eq!(inherited_report["workspace"]["exclude"][0], "crates/skip");
    assert_eq!(
        inherited_report["workspace"]["default-members"][0],
        "crates/app"
    );
    assert_eq!(inherited_report["dependencies"][0]["name"], "shared_dep");
    assert_eq!(
        inherited_report["dependencies"][0]["workspace_inherited"],
        true
    );
    assert_eq!(inherited_report["lints"]["rust.unsafe_code"], "deny");

    let metadata_output = Command::new(&executable)
        .arg("metadata")
        .arg("--manifest-path")
        .arg(&app_manifest)
        .arg("--json")
        .output()
        .expect("run native package metadata");
    assert!(metadata_output.status.success());
    let metadata_report: serde_json::Value =
        serde_json::from_slice(&metadata_output.stdout).expect("package metadata JSON");
    assert_eq!(metadata_report["schema"], "ghor.metadata.v1");
    assert_eq!(metadata_report["executor"], "nc-stage1");
    assert_eq!(metadata_report["manifest"]["package"]["version"], "1.2.3");
    assert_eq!(
        metadata_report["manifest"]["dependencies"][0]["name"],
        "shared_dep"
    );
    assert_eq!(
        metadata_report["packages"].as_array().map(Vec::len),
        Some(3)
    );
    assert_eq!(
        metadata_report["workspace_default_members"][0],
        "workspace_app 1.2.3"
    );
    assert_eq!(
        metadata_report["resolve"]["nodes"].as_array().map(Vec::len),
        Some(3)
    );

    let edit_project = out_dir.join("dependency-edit-project");
    let edit_child = edit_project.join("child");
    std::fs::create_dir_all(edit_child.join("src")).expect("dependency edit child source");
    let edit_manifest = edit_project.join("ghor.toml");
    let edit_source = "[package]\nname = \"edit_root\"\nversion = \"0.1.0\"\n\n# dependency comment survives transactions\n[dependencies]\nexisting = { path = \"child\" }\n\n[profiles.release]\nopt = 3\n";
    std::fs::write(&edit_manifest, edit_source).expect("dependency edit manifest");
    std::fs::write(
        edit_child.join("ghor.toml"),
        "[package]\nname = \"child\"\nversion = \"1.0.0\"\nkind = \"lib\"\n",
    )
    .expect("dependency edit child manifest");
    std::fs::write(
        edit_child.join("src/main.n"),
        "child { value :: fn() -> int { return 7; } fa: return true; }\n",
    )
    .expect("dependency edit child unit");

    let add = Command::new(&executable)
        .current_dir(&edit_project)
        .args([
            "add",
            "child@1.0.0",
            "--path",
            "child",
            "--features",
            "fast,trace",
            "--optional",
            "--json",
        ])
        .output()
        .expect("native add path dependency");
    assert!(
        add.status.success(),
        "add stdout={} stderr={}",
        String::from_utf8_lossy(&add.stdout),
        String::from_utf8_lossy(&add.stderr)
    );
    let add_report: serde_json::Value =
        serde_json::from_slice(&add.stdout).expect("native add JSON");
    assert_eq!(add_report["executor"], "nc-stage1");
    assert_eq!(add_report["status"], "added");
    assert_eq!(add_report["replaced"], false);
    let added_source = std::fs::read_to_string(&edit_manifest).expect("added manifest");
    assert!(added_source.contains("# dependency comment survives transactions"));
    assert!(added_source.contains("features = [\"fast\", \"trace\"]"));
    assert!(added_source.contains("optional = true"));
    assert!(added_source.contains("\n\n[profiles.release]"));

    let replace = Command::new(&executable)
        .current_dir(&edit_project)
        .args(["add", "child@1.1.0", "--path", "child", "--json"])
        .output()
        .expect("native replace path dependency");
    assert!(replace.status.success());
    let replace_report: serde_json::Value =
        serde_json::from_slice(&replace.stdout).expect("native replace JSON");
    assert_eq!(replace_report["status"], "replaced");
    assert_eq!(replace_report["replaced"], true);
    let replaced_source = std::fs::read_to_string(&edit_manifest).expect("replaced manifest");
    assert_eq!(replaced_source.matches("\"child\" =").count(), 1);
    assert!(replaced_source.contains("version = \"1.1.0\""));

    for args in [
        vec![
            "add",
            "child",
            "--rename",
            "dev_child",
            "--path",
            "child",
            "--dev",
            "--manifest-path",
            edit_manifest.to_str().expect("edit manifest"),
            "--json",
        ],
        vec![
            "add",
            "child",
            "--rename",
            "build_child",
            "--path",
            "child",
            "--build",
            "--target",
            "cfg(windows)",
            "--manifest-path",
            edit_manifest.to_str().expect("edit manifest"),
            "--json",
        ],
    ] {
        let output = Command::new(&executable)
            .args(args)
            .output()
            .expect("native categorized add");
        assert!(
            output.status.success(),
            "categorized add stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let categorized_manifest = Command::new(&executable)
        .arg("manifest")
        .arg("--manifest-path")
        .arg(&edit_manifest)
        .arg("--json")
        .output()
        .expect("inspect categorized dependency manifest");
    assert!(categorized_manifest.status.success());
    let categorized_report: serde_json::Value =
        serde_json::from_slice(&categorized_manifest.stdout).expect("categorized manifest JSON");
    assert_eq!(
        categorized_report["dev_dependencies"][0]["name"],
        "dev_child"
    );
    assert_eq!(categorized_report["dev_dependencies"][0]["rename"], "child");
    assert_eq!(
        categorized_report["target_dependencies"][0]["name"],
        "build_child"
    );
    assert_eq!(
        categorized_report["target_dependencies"][0]["kind"],
        "build"
    );
    assert_eq!(
        categorized_report["target_dependencies"][0]["target"],
        "cfg(windows)"
    );

    let tree = Command::new(&executable)
        .arg("tree")
        .arg("--manifest-path")
        .arg(&edit_manifest)
        .arg("--include-dev")
        .arg("--json")
        .output()
        .expect("native dependency tree");
    assert!(
        tree.status.success(),
        "tree stdout={} stderr={}",
        String::from_utf8_lossy(&tree.stdout),
        String::from_utf8_lossy(&tree.stderr)
    );
    let tree_report: serde_json::Value =
        serde_json::from_slice(&tree.stdout).expect("native dependency tree JSON");
    assert_eq!(tree_report["schema"], "ghor.tree.v2");
    assert_eq!(tree_report["executor"], "nc-stage1");
    assert_eq!(tree_report["resolver"]["include_dev"], true);
    assert!(tree_report["edges"]
        .as_array()
        .expect("tree edges")
        .iter()
        .any(|edge| edge["from"] == "edit_root" && edge["to"] == "dev_child"));

    for args in [
        vec![
            "remove",
            "build_child",
            "--build",
            "--target",
            "cfg(windows)",
            "--manifest-path",
            edit_manifest.to_str().expect("edit manifest"),
            "--json",
        ],
        vec![
            "remove",
            "dev_child",
            "--dev",
            "--manifest-path",
            edit_manifest.to_str().expect("edit manifest"),
            "--json",
        ],
        vec![
            "remove",
            "child",
            "--manifest-path",
            edit_manifest.to_str().expect("edit manifest"),
            "--json",
        ],
    ] {
        let output = Command::new(&executable)
            .args(args)
            .output()
            .expect("native dependency remove");
        assert!(
            output.status.success(),
            "remove stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert_eq!(
        std::fs::read_to_string(&edit_manifest).expect("restored dependency manifest"),
        edit_source
    );
    let invalid_add = Command::new(&executable)
        .arg("add")
        .arg("invalid_remote@1.0.0")
        .arg("--checksum")
        .arg("not-a-sha256")
        .arg("--manifest-path")
        .arg(&edit_manifest)
        .arg("--json")
        .output()
        .expect("reject invalid registry checksum");
    assert!(!invalid_add.status.success());
    let invalid_add_report: serde_json::Value =
        serde_json::from_slice(&invalid_add.stdout).expect("invalid add JSON");
    assert_eq!(invalid_add_report["executor"], "nc-stage1");
    assert_eq!(
        invalid_add_report["diagnostics"][0]["code"],
        "E_GHOR_CHECKSUM_001"
    );
    let missing_remove = Command::new(&executable)
        .arg("remove")
        .arg("missing_dependency")
        .arg("--manifest-path")
        .arg(&edit_manifest)
        .arg("--json")
        .output()
        .expect("reject missing dependency removal");
    assert!(!missing_remove.status.success());
    assert_eq!(
        std::fs::read_to_string(&edit_manifest).expect("negative edits preserve manifest"),
        edit_source
    );

    let registry = out_dir.join("registry");
    std::fs::create_dir_all(&registry).expect("filesystem registry");
    let checksum_100 = "1".repeat(64);
    let checksum_150 = "2".repeat(64);
    let checksum_200 = "3".repeat(64);
    let checksum_child = "4".repeat(64);
    let registry_index = serde_json::json!({
        "schema": "ghor.registry.v1",
        "packages": [
            {
                "name": "semver_dep",
                "version": "1.0.0",
                "checksum": checksum_100,
                "yanked": false,
                "metadata": {"vers": "1.0.0", "deps": [], "features": {"default": []}}
            },
            {
                "name": "semver_dep",
                "version": "1.5.0",
                "checksum": checksum_150,
                "yanked": false,
                "metadata": {
                    "vers": "1.5.0",
                    "deps": [{"name": "child_dep", "req": "^1.0", "kind": "normal", "features": []}],
                    "features": {"default": ["std"], "serde": ["derive"]}
                }
            },
            {
                "name": "semver_dep",
                "version": "2.0.0",
                "checksum": checksum_200,
                "yanked": false,
                "metadata": {"vers": "2.0.0", "deps": [], "features": {"default": []}}
            },
            {
                "name": "child_dep",
                "version": "1.1.0",
                "checksum": checksum_child,
                "yanked": false,
                "metadata": {"vers": "1.1.0", "deps": [], "features": {"default": []}}
            }
        ]
    });
    std::fs::write(
        registry.join("index.json"),
        serde_json::to_vec_pretty(&registry_index).expect("registry JSON"),
    )
    .expect("write filesystem registry");

    let registry_edit = out_dir.join("registry-edit-project");
    std::fs::create_dir_all(&registry_edit).expect("registry edit project");
    let registry_edit_manifest = registry_edit.join("ghor.toml");
    let registry_edit_source = "[package]\nname = \"registry_edit\"\nversion = \"0.1.0\"\n";
    std::fs::write(&registry_edit_manifest, registry_edit_source).expect("registry edit manifest");
    let registry_add = Command::new(&executable)
        .arg("add")
        .arg("semver_dep@^1.0")
        .arg("--checksum")
        .arg(&checksum_150)
        .arg("--registry")
        .arg(&registry)
        .arg("--manifest-path")
        .arg(&registry_edit_manifest)
        .arg("--json")
        .output()
        .expect("native registry add");
    assert!(registry_add.status.success());
    let registry_add_report: serde_json::Value =
        serde_json::from_slice(&registry_add.stdout).expect("registry add JSON");
    assert_eq!(registry_add_report["executor"], "nc-stage1");
    let registry_tree = Command::new(&executable)
        .arg("tree")
        .arg("--manifest-path")
        .arg(&registry_edit_manifest)
        .arg("--registry")
        .arg(&registry)
        .arg("--json")
        .output()
        .expect("native registry tree after add");
    assert!(
        registry_tree.status.success(),
        "registry tree stdout={} stderr={}",
        String::from_utf8_lossy(&registry_tree.stdout),
        String::from_utf8_lossy(&registry_tree.stderr)
    );
    let registry_tree_report: serde_json::Value =
        serde_json::from_slice(&registry_tree.stdout).expect("registry tree JSON");
    assert!(registry_tree_report["nodes"]
        .as_array()
        .expect("registry tree nodes")
        .iter()
        .any(|node| node["name"] == "semver_dep" && node["version"] == "1.5.0"));
    assert!(registry_tree_report["nodes"]
        .as_array()
        .expect("registry tree nodes")
        .iter()
        .any(|node| node["name"] == "child_dep" && node["version"] == "1.1.0"));
    let registry_remove = Command::new(&executable)
        .arg("remove")
        .arg("semver_dep")
        .arg("--manifest-path")
        .arg(&registry_edit_manifest)
        .arg("--json")
        .output()
        .expect("native registry remove");
    assert!(registry_remove.status.success());
    assert_eq!(
        std::fs::read_to_string(&registry_edit_manifest).expect("restored registry edit manifest"),
        registry_edit_source
    );
    let resolver_fixture = out_dir.join("resolver-project");
    std::fs::create_dir_all(&resolver_fixture).expect("resolver fixture");
    let resolver_manifest = resolver_fixture.join("ghor.toml");
    let resolver_source = format!(
        "[package]\nname = \"resolver_consumer\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n[dependencies]\nsemver_dep = {{ version = \"^1.0\", checksum = \"{}\", features = [\"serde\"], default-features = false }}\n",
        "2".repeat(64)
    );
    std::fs::write(&resolver_manifest, &resolver_source).expect("resolver manifest");

    let resolve_output = Command::new(&executable)
        .arg("resolve")
        .arg("--manifest-path")
        .arg(&resolver_manifest)
        .arg("--registry")
        .arg(&registry)
        .arg("--json")
        .output()
        .expect("run native resolver");
    assert!(
        resolve_output.status.success(),
        "resolve stdout={} stderr={}",
        String::from_utf8_lossy(&resolve_output.stdout),
        String::from_utf8_lossy(&resolve_output.stderr)
    );
    let resolve_report: serde_json::Value =
        serde_json::from_slice(&resolve_output.stdout).expect("resolve JSON");
    assert_eq!(resolve_report["schema"], "ghor.resolve.v2");
    assert_eq!(resolve_report["executor"], "nc-stage1");
    assert_eq!(resolve_report["resolver"]["include_dev"], false);
    assert_eq!(resolve_report["manifest"]["virtual_manifest"], false);
    let resolved = resolve_report["resolved"]
        .as_array()
        .expect("resolved nodes");
    assert_eq!(resolved.len(), 2);
    assert!(resolved
        .iter()
        .any(|node| node["name"] == "semver_dep" && node["version"] == "1.5.0"));
    assert!(resolved
        .iter()
        .any(|node| node["name"] == "child_dep" && node["version"] == "1.1.0"));
    let feature_group = resolve_report["feature_groups"]
        .as_array()
        .expect("feature groups")
        .iter()
        .find(|group| group["group"] == "semver_dep@1.5.0:normal")
        .expect("semver feature group");
    assert!(feature_group["features"]
        .as_array()
        .expect("resolved features")
        .iter()
        .any(|feature| feature == "serde"));
    assert!(feature_group["features"]
        .as_array()
        .expect("resolved features")
        .iter()
        .any(|feature| feature == "derive"));

    let lock_output = Command::new(&executable)
        .arg("lock")
        .arg("--manifest-path")
        .arg(&resolver_manifest)
        .arg("--registry")
        .arg(&registry)
        .arg("--json")
        .output()
        .expect("run native lock");
    assert!(lock_output.status.success());
    let lock_report: serde_json::Value =
        serde_json::from_slice(&lock_output.stdout).expect("lock result JSON");
    assert_eq!(lock_report["schema"], "ghor.lock.result.v1");
    assert_eq!(lock_report["status"], "locked");
    let lockfile = std::fs::read_to_string(resolver_fixture.join("ghor.lock"))
        .expect("generated Cargo-compatible lockfile");
    assert!(lockfile.contains("version = 4"));
    assert!(lockfile.contains("name = \"semver_dep\""));
    assert!(lockfile.contains("name = \"child_dep\""));
    let lock_metadata: serde_json::Value = serde_json::from_slice(
        &std::fs::read(resolver_fixture.join("ghor.lock.meta.json")).expect("lock metadata"),
    )
    .expect("lock metadata JSON");
    assert_eq!(lock_metadata["schema"], "ghor.lock.v1");

    let verify_output = Command::new(&executable)
        .arg("verify-lock")
        .arg("--manifest-path")
        .arg(&resolver_manifest)
        .arg("--registry")
        .arg(&registry)
        .arg("--json")
        .output()
        .expect("verify native lock");
    assert!(verify_output.status.success());
    let verify_report: serde_json::Value =
        serde_json::from_slice(&verify_output.stdout).expect("verify-lock JSON");
    assert_eq!(verify_report["schema"], "ghor.verify-lock.v1");
    assert_eq!(verify_report["status"], "ok");

    std::fs::write(&resolver_manifest, resolver_source.replace("^1.0", "^2.0"))
        .expect("introduce lock drift");
    let drift_output = Command::new(&executable)
        .arg("verify-lock")
        .arg("--manifest-path")
        .arg(&resolver_manifest)
        .arg("--registry")
        .arg(&registry)
        .arg("--json")
        .output()
        .expect("detect native lock drift");
    assert_eq!(drift_output.status.code(), Some(2));
    let drift_report: serde_json::Value =
        serde_json::from_slice(&drift_output.stdout).expect("drift JSON");
    assert_eq!(drift_report["status"], "error");
    assert_eq!(
        drift_report["diagnostics"][0]["code"],
        "E_GHOR_LOCK_DRIFT_001"
    );
    std::fs::write(&resolver_manifest, &resolver_source).expect("restore resolver manifest");

    let material_registry = out_dir.join("material-registry");
    let material_package_dir = material_registry.join("packages/material_dep/0.1.0");
    std::fs::create_dir_all(&material_package_dir).expect("material registry package directory");
    let material_source = out_dir.join("material-source");
    let material_root = material_source.join("material_dep-0.1.0");
    std::fs::create_dir_all(material_root.join("src")).expect("material package source");
    std::fs::write(
        material_root.join("ghor.toml"),
        "[package]\nname = \"material_dep\"\nversion = \"0.1.0\"\nkind = \"lib\"\n",
    )
    .expect("material package manifest");
    std::fs::write(
        material_root.join("src/main.n"),
        "materialDep { fn: public: value() -> int { return 7; } fa: return true; }\n",
    )
    .expect("material package source file");
    let archive = material_package_dir.join("material_dep-0.1.0.crate");
    let tar = Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(&material_source)
        .arg("material_dep-0.1.0")
        .output()
        .expect("create material .crate with tar");
    assert!(
        tar.status.success(),
        "tar stdout={} stderr={}",
        String::from_utf8_lossy(&tar.stdout),
        String::from_utf8_lossy(&tar.stderr)
    );
    let archive_hash = sha256(&std::fs::read(&archive).expect("material archive bytes"));
    let package_manifest = serde_json::json!({
        "archive_name": "material_dep-0.1.0.crate",
        "checksum": archive_hash,
        "ffi_link_manifest": {
            "compiler_ffi_entry_count": 0,
            "diagnostics": [],
            "schema": "ghor.ffi_manifest.v1",
            "status": "closed"
        },
        "files": ["ghor.toml", "src/main.n"],
        "format": "cargo-compatible .crate tar.gz",
        "name": "material_dep",
        "package_root": "material_dep-0.1.0",
        "schema": "ghor.package.v2",
        "verification": {"source_checks": 2, "status": "verified"},
        "version": "0.1.0"
    });
    let package_manifest_hash = sha256(
        serde_json::to_string(&package_manifest)
            .expect("compact material manifest")
            .as_bytes(),
    );
    std::fs::write(
        material_package_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&package_manifest).expect("material manifest JSON"),
    )
    .expect("write material package manifest");
    let material_index = serde_json::json!({
        "schema": "ghor.registry.v1",
        "packages": [{
            "archive": "packages/material_dep/0.1.0/material_dep-0.1.0.crate",
            "archive_name": "material_dep-0.1.0.crate",
            "checksum": archive_hash,
            "files": ["ghor.toml", "src/main.n"],
            "manifest": "packages/material_dep/0.1.0/manifest.json",
            "metadata": {
                "deps": [],
                "features": {"default": []},
                "name": "material_dep",
                "vers": "0.1.0"
            },
            "name": "material_dep",
            "package_manifest": package_manifest,
            "package_manifest_hash": package_manifest_hash,
            "verification": {"source_checks": 2, "status": "verified"},
            "version": "0.1.0",
            "yanked": false
        }]
    });
    std::fs::write(
        material_registry.join("index.json"),
        serde_json::to_vec_pretty(&material_index).expect("material registry JSON"),
    )
    .expect("write material registry");

    let native_search = Command::new(&executable)
        .args(["search", "material", "--registry"])
        .arg(&material_registry)
        .arg("--json")
        .output()
        .expect("run native registry search");
    assert!(
        native_search.status.success(),
        "search stdout={} stderr={}",
        String::from_utf8_lossy(&native_search.stdout),
        String::from_utf8_lossy(&native_search.stderr)
    );
    let native_search_report: serde_json::Value =
        serde_json::from_slice(&native_search.stdout).expect("native search JSON");
    assert_eq!(native_search_report["schema"], "ghor.search.v1");
    assert_eq!(native_search_report["executor"], "nc-stage1");
    assert_eq!(native_search_report["packages"][0]["name"], "material_dep");

    let native_info = Command::new(&executable)
        .args(["info", "material_dep", "--registry"])
        .arg(&material_registry)
        .arg("--json")
        .output()
        .expect("run native registry info");
    assert!(native_info.status.success());
    let native_info_report: serde_json::Value =
        serde_json::from_slice(&native_info.stdout).expect("native info JSON");
    assert_eq!(native_info_report["schema"], "ghor.info.v1");
    assert_eq!(native_info_report["packages"][0]["version"], "0.1.0");

    for (action, schema) in [
        ("doctor", "ghor.registry.doctor.v1"),
        ("index", "ghor.registry.index.v1"),
        ("config", "ghor.registry.config.v1"),
    ] {
        let output = Command::new(&executable)
            .args(["registry", action, "--registry"])
            .arg(&material_registry)
            .arg("--json")
            .output()
            .expect("run native registry inspection");
        assert!(
            output.status.success(),
            "registry {action} stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("native registry inspection JSON");
        assert_eq!(report["schema"], schema);
        assert_eq!(report["executor"], "nc-stage1");
    }

    let missing_info = Command::new(&executable)
        .args(["info", "--registry"])
        .arg(&material_registry)
        .arg("--json")
        .output()
        .expect("reject native info without package");
    assert_eq!(missing_info.status.code(), Some(2));
    let missing_info_report: serde_json::Value =
        serde_json::from_slice(&missing_info.stdout).expect("missing info JSON");
    assert_eq!(
        missing_info_report["diagnostics"][0]["code"],
        "E_GHOR_INFO_001"
    );

    let credentials_home = out_dir.join("ghor-home");
    let credential_token = "phase9-private-token";
    let native_login = Command::new(&executable)
        .args(["login", "--registry"])
        .arg(&material_registry)
        .args(["--token", credential_token, "--json"])
        .env("GHOR_HOME", &credentials_home)
        .output()
        .expect("run native registry login");
    assert!(native_login.status.success());
    assert!(!String::from_utf8_lossy(&native_login.stdout).contains(credential_token));
    let native_login_report: serde_json::Value =
        serde_json::from_slice(&native_login.stdout).expect("native login JSON");
    assert_eq!(native_login_report["auth"], "token_stored");
    let credentials_path = credentials_home.join("credentials.json");
    let credential_document =
        std::fs::read_to_string(&credentials_path).expect("native credentials file");
    assert!(credential_document.contains(credential_token));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&credentials_path)
                .expect("credential metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(&credentials_home)
                .expect("credential home metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
    let native_logout = Command::new(&executable)
        .args(["logout", "--registry"])
        .arg(&material_registry)
        .arg("--json")
        .env("GHOR_HOME", &credentials_home)
        .output()
        .expect("run native registry logout");
    assert!(native_logout.status.success());
    let native_logout_report: serde_json::Value =
        serde_json::from_slice(&native_logout.stdout).expect("native logout JSON");
    assert_eq!(native_logout_report["removed"], true);
    assert!(!std::fs::read_to_string(&credentials_path)
        .expect("credentials after logout")
        .contains(credential_token));

    let registry_text = material_registry.to_string_lossy().to_string();
    let delegated_search = run_ghor(
        &["search", "material", "--registry", &registry_text, "--json"],
        Some(&executable),
        false,
    );
    assert!(
        delegated_search.status.success(),
        "delegated search stdout={} stderr={}",
        String::from_utf8_lossy(&delegated_search.stdout),
        String::from_utf8_lossy(&delegated_search.stderr)
    );
    let delegated_search_report: serde_json::Value =
        serde_json::from_slice(&delegated_search.stdout).expect("delegated search JSON");
    assert_eq!(delegated_search_report["executor"], "nc-stage1");

    let owner_add = Command::new(&executable)
        .args([
            "owner",
            "add",
            "material_dep",
            "phase10-owner",
            "--registry",
        ])
        .arg(&material_registry)
        .arg("--json")
        .output()
        .expect("native owner add");
    assert!(
        owner_add.status.success(),
        "owner add stdout={} stderr={}",
        String::from_utf8_lossy(&owner_add.stdout),
        String::from_utf8_lossy(&owner_add.stderr)
    );
    let owner_add_report: serde_json::Value =
        serde_json::from_slice(&owner_add.stdout).expect("owner add JSON");
    assert_eq!(owner_add_report["executor"], "nc-stage1");
    assert_eq!(owner_add_report["transaction"], "committed");
    assert!(owner_add_report["owners"]
        .as_array()
        .expect("owners")
        .iter()
        .any(|owner| owner == "phase10-owner"));

    let yank = Command::new(&executable)
        .args(["yank", "material_dep", "--version", "0.1.0", "--registry"])
        .arg(&material_registry)
        .arg("--json")
        .output()
        .expect("native yank");
    assert!(
        yank.status.success(),
        "yank stdout={} stderr={}",
        String::from_utf8_lossy(&yank.stdout),
        String::from_utf8_lossy(&yank.stderr)
    );
    let yank_report: serde_json::Value = serde_json::from_slice(&yank.stdout).expect("yank JSON");
    assert_eq!(yank_report["executor"], "nc-stage1");
    assert_eq!(yank_report["yanked"], true);

    let unyank = Command::new(&executable)
        .args([
            "yank",
            "material_dep",
            "--version",
            "0.1.0",
            "--undo",
            "--registry",
        ])
        .arg(&material_registry)
        .arg("--json")
        .output()
        .expect("native unyank");
    assert!(unyank.status.success());

    let publish_project = out_dir.join("phase10-publish-project");
    std::fs::create_dir_all(publish_project.join("src")).expect("publish source directory");
    let publish_manifest = publish_project.join("ghor.toml");
    std::fs::write(
        &publish_manifest,
        "[package]\nname = \"phase10_native\"\nversion = \"1.0.0\"\ndescription = \"native stage1 registry mutation\"\nrepository = \"https://example.invalid/phase10\"\nlicense = \"MIT\"\nkind = \"bin\"\n",
    )
    .expect("publish manifest");
    std::fs::write(
        publish_project.join("src/main.n"),
        "phase10Native { run :: fn() -> bool { return true; } ref main ->> run; fa: return true; }\n",
    )
    .expect("publish source");
    let publish = Command::new(&executable)
        .arg("publish")
        .arg("--manifest-path")
        .arg(&publish_manifest)
        .arg("--registry")
        .arg(&material_registry)
        .args(["--allow-dirty", "--json"])
        .output()
        .expect("native publish");
    assert!(
        publish.status.success(),
        "publish stdout={} stderr={}",
        String::from_utf8_lossy(&publish.stdout),
        String::from_utf8_lossy(&publish.stderr)
    );
    let publish_report: serde_json::Value =
        serde_json::from_slice(&publish.stdout).expect("publish JSON");
    assert_eq!(publish_report["executor"], "nc-stage1");
    assert_eq!(publish_report["transaction"], "committed");
    assert!(material_registry
        .join("packages/phase10_native/1.0.0/phase10_native-1.0.0.crate")
        .is_file());

    let administration_token = "phase24-administration-token";
    let governance = Command::new(&executable)
        .arg("governance")
        .arg("--registry")
        .arg(&material_registry)
        .args([
            "--init",
            "--role",
            "alice:admin",
            "--token",
            administration_token,
            "--json",
        ])
        .output()
        .expect("native governance transaction");
    assert!(
        governance.status.success(),
        "governance stdout={} stderr={}",
        String::from_utf8_lossy(&governance.stdout),
        String::from_utf8_lossy(&governance.stderr)
    );
    let governance_text = String::from_utf8_lossy(&governance.stdout);
    assert!(!governance_text.contains(administration_token));
    let governance_report: serde_json::Value =
        serde_json::from_slice(&governance.stdout).expect("governance JSON");
    assert_eq!(governance_report["executor"], "nc-stage1");
    assert_eq!(governance_report["mutated"], true);
    assert_eq!(governance_report["policy"]["roles"]["alice"], "admin");
    assert!(material_registry.join("governance.json").is_file());

    for (action, expected_status, expected_yanked) in
        [("block", "blocked", true), ("restore", "restored", false)]
    {
        let moderation = Command::new(&executable)
            .args(["moderation", "phase10_native", "--version", "1.0.0"])
            .arg("--registry")
            .arg(&material_registry)
            .args([
                "--action",
                action,
                "--reason",
                "phase24-native-contract",
                "--token",
                administration_token,
                "--json",
            ])
            .output()
            .expect("native moderation transaction");
        assert!(
            moderation.status.success(),
            "moderation {action} stdout={} stderr={}",
            String::from_utf8_lossy(&moderation.stdout),
            String::from_utf8_lossy(&moderation.stderr)
        );
        assert!(!String::from_utf8_lossy(&moderation.stdout).contains(administration_token));
        let moderation_report: serde_json::Value =
            serde_json::from_slice(&moderation.stdout).expect("moderation JSON");
        assert_eq!(moderation_report["executor"], "nc-stage1");
        assert_eq!(moderation_report["moderation_status"], expected_status);
        assert_eq!(moderation_report["mutated"], true);
        let registry_index: serde_json::Value = serde_json::from_slice(
            &std::fs::read(material_registry.join("index.json")).expect("moderated index"),
        )
        .expect("moderated index JSON");
        let entry = registry_index["packages"]
            .as_array()
            .expect("registry packages")
            .iter()
            .find(|entry| entry["name"] == "phase10_native" && entry["version"] == "1.0.0")
            .expect("moderated phase10 package");
        assert_eq!(entry["moderation_status"], expected_status);
        assert_eq!(entry["yanked"], expected_yanked);
    }
    let cargo_shard = std::fs::read_to_string(material_registry.join("ph/as/phase10_native"))
        .expect("moderated Cargo shard");
    assert!(cargo_shard.contains("\"moderation_status\":\"restored\""));
    assert!(cargo_shard.contains("\"yanked\":false"));
    let administration_audit =
        std::fs::read_to_string(material_registry.join("registry-audit.json"))
            .expect("registry audit");
    assert!(administration_audit.contains("governance.update"));
    assert!(administration_audit.contains("moderation.update"));
    assert!(!administration_audit.contains(administration_token));

    let cdn_root = out_dir.join("phase24-cdn");
    let cdn = Command::new(&executable)
        .args(["cdn", "build", "--registry"])
        .arg(&material_registry)
        .arg("--out")
        .arg(&cdn_root)
        .arg("--json")
        .output()
        .expect("native CDN build");
    assert!(
        cdn.status.success(),
        "cdn stdout={} stderr={}",
        String::from_utf8_lossy(&cdn.stdout),
        String::from_utf8_lossy(&cdn.stderr)
    );
    let cdn_report: serde_json::Value =
        serde_json::from_slice(&cdn.stdout).expect("CDN build JSON");
    assert_eq!(cdn_report["executor"], "nc-stage1");
    assert_eq!(cdn_report["manifest"]["content_addressed"], true);
    let cdn_entry = cdn_report["manifest"]["entries"]
        .as_array()
        .expect("CDN entries")
        .iter()
        .find(|entry| entry["name"] == "phase10_native")
        .expect("phase10 CDN entry");
    let blob_hash = cdn_entry["sha256"].as_str().expect("CDN SHA-256");
    let blob_path = cdn_root.join(format!("blobs/sha256/{blob_hash}.crate"));
    assert_eq!(
        sha256(&std::fs::read(&blob_path).expect("CDN blob bytes")),
        blob_hash
    );
    let purge = Command::new(&executable)
        .args(["cdn", "purge", "--sha256", blob_hash, "--out"])
        .arg(&cdn_root)
        .arg("--json")
        .output()
        .expect("native CDN purge");
    assert!(purge.status.success());
    let purge_report: serde_json::Value =
        serde_json::from_slice(&purge.stdout).expect("CDN purge JSON");
    assert_eq!(purge_report["removed"], true);
    assert!(!blob_path.exists());

    for (label, arguments, expected_code) in [
        (
            "governance without token",
            vec![
                "governance",
                "--registry",
                material_registry.to_str().expect("registry path"),
                "--init",
                "--json",
            ],
            "E_GHOR_AUTH_001",
        ),
        (
            "governance git mutation",
            vec![
                "governance",
                "--registry",
                "git+file:///phase24-read-only-registry",
                "--init",
                "--token",
                administration_token,
                "--json",
            ],
            "E_GHOR_REGISTRY_GIT_001",
        ),
    ] {
        let rejected = Command::new(&executable)
            .args(arguments)
            .output()
            .unwrap_or_else(|error| panic!("{label}: {error}"));
        assert!(!rejected.status.success(), "{label} must fail");
        assert!(
            String::from_utf8_lossy(&rejected.stdout).contains(expected_code)
                || String::from_utf8_lossy(&rejected.stderr).contains(expected_code),
            "{label} must report {expected_code}"
        );
    }

    let install_root = out_dir.join("phase15-native-install");
    let install = Command::new(&executable)
        .args(["install", "phase10_native", "--version", "^1", "--registry"])
        .arg(&material_registry)
        .arg("--root")
        .arg(&install_root)
        .args(["--profile", "release-fast", "--json"])
        .env("NC_GHOR_NCC", &stage1_ncc)
        .env("NCC_CHILD_QUIET", "1")
        .output()
        .expect("native registry install");
    assert!(
        install.status.success(),
        "install stdout={} stderr={}",
        String::from_utf8_lossy(&install.stdout),
        String::from_utf8_lossy(&install.stderr)
    );
    let install_report: serde_json::Value =
        serde_json::from_slice(&install.stdout).expect("native install JSON");
    assert_eq!(install_report["executor"], "nc-stage1");
    assert_eq!(install_report["status"], "installed");
    assert_eq!(install_report["transaction"], "committed");
    assert_eq!(install_report["version"], "1.0.0");
    assert_eq!(
        install_report["package_manifest_validation"]["status"],
        "validated"
    );
    for field in ["binary", "nm", "object", "manifest", "cache_path"] {
        assert!(
            PathBuf::from(
                install_report[field]
                    .as_str()
                    .unwrap_or_else(|| panic!("install report missing {field}"))
            )
            .exists(),
            "installed {field} must exist"
        );
    }
    let installed_binary = PathBuf::from(
        install_report["binary"]
            .as_str()
            .expect("installed executable path"),
    );
    let installed_run = Command::new(&installed_binary)
        .output()
        .expect("run installed native executable");
    assert!(
        installed_run.status.success(),
        "installed executable status={} stdout={} stderr={}",
        installed_run.status,
        String::from_utf8_lossy(&installed_run.stdout),
        String::from_utf8_lossy(&installed_run.stderr)
    );

    let reinstall = Command::new(&executable)
        .args([
            "install",
            "phase10_native",
            "--version",
            "1.0.0",
            "--registry",
        ])
        .arg(&material_registry)
        .arg("--root")
        .arg(&install_root)
        .args(["--profile", "release-fast", "--json"])
        .env("NC_GHOR_NCC", &stage1_ncc)
        .env("NCC_CHILD_QUIET", "1")
        .output()
        .expect("native registry reinstall");
    assert!(
        reinstall.status.success(),
        "reinstall stdout={} stderr={}",
        String::from_utf8_lossy(&reinstall.stdout),
        String::from_utf8_lossy(&reinstall.stderr)
    );
    let reinstall_report: serde_json::Value =
        serde_json::from_slice(&reinstall.stdout).expect("native reinstall JSON");
    assert_eq!(reinstall_report["transaction"], "committed");

    let uninstall = Command::new(&executable)
        .args([
            "uninstall",
            "phase10_native",
            "--version",
            "1.0.0",
            "--root",
        ])
        .arg(&install_root)
        .arg("--json")
        .output()
        .expect("native uninstall");
    assert!(
        uninstall.status.success(),
        "uninstall stdout={} stderr={}",
        String::from_utf8_lossy(&uninstall.stdout),
        String::from_utf8_lossy(&uninstall.stderr)
    );
    let uninstall_report: serde_json::Value =
        serde_json::from_slice(&uninstall.stdout).expect("native uninstall JSON");
    assert_eq!(uninstall_report["executor"], "nc-stage1");
    assert_eq!(uninstall_report["status"], "uninstalled");
    assert_eq!(uninstall_report["transaction"], "committed");
    assert!(!installed_binary.exists());
    assert!(!install_root
        .join("registry-src/phase10_native-1.0.0")
        .exists());
    assert!(!install_root
        .join("registry-cache/phase10_native-1.0.0.crate")
        .exists());
    assert!(!install_root.join("installed/phase10_native.json").exists());

    let path_install_root = out_dir.join("phase15-native-path-install");
    let path_install = Command::new(&executable)
        .arg("install")
        .arg("--path")
        .arg(&publish_project)
        .arg("--root")
        .arg(&path_install_root)
        .args(["--profile", "release-fast", "--json"])
        .env("NC_GHOR_NCC", &stage1_ncc)
        .env("NCC_CHILD_QUIET", "1")
        .output()
        .expect("native path install");
    assert!(
        path_install.status.success(),
        "path install stdout={} stderr={}",
        String::from_utf8_lossy(&path_install.stdout),
        String::from_utf8_lossy(&path_install.stderr)
    );
    let path_report: serde_json::Value =
        serde_json::from_slice(&path_install.stdout).expect("native path install JSON");
    assert_eq!(path_report["source"], "path");
    assert_eq!(path_report["owns_source_dir"], false);
    assert!(PathBuf::from(
        path_report["binary"]
            .as_str()
            .expect("path install executable")
    )
    .exists());
    let path_uninstall = Command::new(&executable)
        .args(["uninstall", "phase10_native", "--root"])
        .arg(&path_install_root)
        .arg("--json")
        .output()
        .expect("native path uninstall");
    assert!(path_uninstall.status.success());
    assert!(publish_project.join("src/main.n").is_file());

    let owner_remove = Command::new(&executable)
        .args([
            "owner",
            "remove",
            "material_dep",
            "phase10-owner",
            "--registry",
        ])
        .arg(&material_registry)
        .arg("--json")
        .output()
        .expect("native owner remove");
    assert!(owner_remove.status.success());

    let interrupted_index = material_registry.join("index.json");
    let interrupted_backup = material_registry.join("index.json.bak");
    std::fs::copy(&interrupted_index, &interrupted_backup).expect("stage index backup");
    std::fs::remove_file(&interrupted_index).expect("simulate interrupted index replacement");
    let recovered_owner_list = Command::new(&executable)
        .args(["owner", "list", "material_dep", "--registry"])
        .arg(&material_registry)
        .arg("--json")
        .output()
        .expect("recover registry index before owner list");
    assert!(
        recovered_owner_list.status.success(),
        "recovery stdout={} stderr={}",
        String::from_utf8_lossy(&recovered_owner_list.stdout),
        String::from_utf8_lossy(&recovered_owner_list.stderr)
    );
    assert!(interrupted_index.is_file());
    assert!(!interrupted_backup.exists());

    let material_consumer = out_dir.join("material-consumer");
    std::fs::create_dir_all(&material_consumer).expect("material consumer");
    let material_manifest = material_consumer.join("ghor.toml");
    std::fs::write(
        &material_manifest,
        format!(
            "[package]\nname = \"material_consumer\"\nversion = \"0.1.0\"\n\n[dependencies]\nmaterial_dep = {{ version = \"0.1.0\", checksum = \"{archive_hash}\" }}\n"
        ),
    )
    .expect("material consumer manifest");

    let fetch_output = Command::new(&executable)
        .arg("fetch")
        .arg("--manifest-path")
        .arg(&material_manifest)
        .arg("--registry")
        .arg(&material_registry)
        .arg("--json")
        .output()
        .expect("run native fetch");
    assert!(
        fetch_output.status.success(),
        "fetch stdout={} stderr={}",
        String::from_utf8_lossy(&fetch_output.stdout),
        String::from_utf8_lossy(&fetch_output.stderr)
    );
    let fetch_report: serde_json::Value =
        serde_json::from_slice(&fetch_output.stdout).expect("fetch JSON");
    assert_eq!(fetch_report["executor"], "nc-stage1");
    assert_eq!(fetch_report["status"], "ok");
    let cached_archive = PathBuf::from(
        fetch_report["fetched"][0]["cache_path"]
            .as_str()
            .expect("fetch cache path"),
    );
    assert_eq!(
        sha256(&std::fs::read(cached_archive).expect("cached archive")),
        archive_hash
    );

    let vendor_dir = out_dir.join("material-vendor");
    let vendor_output = Command::new(&executable)
        .arg("vendor")
        .arg(&vendor_dir)
        .arg("--manifest-path")
        .arg(&material_manifest)
        .arg("--registry")
        .arg(&material_registry)
        .arg("--versioned-dirs")
        .arg("--json")
        .output()
        .expect("run native vendor");
    assert!(
        vendor_output.status.success(),
        "vendor stdout={} stderr={}",
        String::from_utf8_lossy(&vendor_output.stdout),
        String::from_utf8_lossy(&vendor_output.stderr)
    );
    let vendor_report: serde_json::Value =
        serde_json::from_slice(&vendor_output.stdout).expect("vendor JSON");
    assert_eq!(vendor_report["executor"], "nc-stage1");
    assert_eq!(vendor_report["status"], "ok");
    let vendored_root = vendor_dir.join("material_dep-0.1.0");
    let checksums: serde_json::Value = serde_json::from_slice(
        &std::fs::read(vendored_root.join(".cargo-checksum.json"))
            .expect("vendor checksum document"),
    )
    .expect("vendor checksum JSON");
    for (relative, expected) in checksums["files"].as_object().expect("vendor file hashes") {
        assert_eq!(
            sha256(&std::fs::read(vendored_root.join(relative)).expect("vendored file")),
            expected.as_str().expect("vendored checksum")
        );
    }

    let update_output = Command::new(&executable)
        .arg("update")
        .arg("--manifest-path")
        .arg(&material_manifest)
        .arg("--registry")
        .arg(&material_registry)
        .arg("--json")
        .output()
        .expect("run native update");
    assert!(update_output.status.success());
    let update_report: serde_json::Value =
        serde_json::from_slice(&update_output.stdout).expect("update JSON");
    assert_eq!(update_report["executor"], "nc-stage1");
    assert_eq!(update_report["changed"], true);
    let repeated_update = Command::new(&executable)
        .arg("update")
        .arg("--manifest-path")
        .arg(&material_manifest)
        .arg("--registry")
        .arg(&material_registry)
        .arg("--json")
        .output()
        .expect("repeat native update");
    assert!(repeated_update.status.success());
    let repeated_report: serde_json::Value =
        serde_json::from_slice(&repeated_update.stdout).expect("repeated update JSON");
    assert_eq!(repeated_report["changed"], false);
    assert_eq!(
        repeated_report["before_hash"],
        repeated_report["after_hash"]
    );

    let selective_update = Command::new(&executable)
        .arg("update")
        .arg("--manifest-path")
        .arg(&resolver_manifest)
        .arg("--registry")
        .arg(&registry)
        .arg("-p")
        .arg("semver_dep")
        .arg("--precise")
        .arg("1.0.0")
        .arg("--dry-run")
        .arg("--json")
        .output()
        .expect("run selective precise native update");
    assert!(
        selective_update.status.success(),
        "selective update stdout={} stderr={}",
        String::from_utf8_lossy(&selective_update.stdout),
        String::from_utf8_lossy(&selective_update.stderr)
    );
    let selective_report: serde_json::Value =
        serde_json::from_slice(&selective_update.stdout).expect("selective update JSON");
    assert_eq!(selective_report["executor"], "nc-stage1");
    assert_eq!(selective_report["status"], "dry_run");
    assert_eq!(selective_report["package_specs"][0], "semver_dep");
    assert_eq!(selective_report["precise"], "1.0.0");
    assert!(selective_report["resolve"]["resolved"]
        .as_array()
        .expect("selective resolved graph")
        .iter()
        .any(|node| node["name"] == "semver_dep" && node["version"] == "1.0.0"));
    assert!(!selective_report["resolve"]["resolved"]
        .as_array()
        .expect("selective resolved graph")
        .iter()
        .any(|node| node["name"] == "child_dep"));

    let http_registry = LocalRegistryServer::start(material_registry.clone());
    std::fs::write(
        material_registry.join("config.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "dl": format!("{}/packages/{{crate}}/{{version}}/{{crate}}-{{version}}.crate", http_registry.base),
            "api": http_registry.base,
            "auth-required": true,
            "ghor-protocol": "cargo-index-compatible"
        }))
        .expect("HTTP registry config JSON"),
    )
    .expect("HTTP registry config");
    let remote_token = "phase10-http-token";
    let remote_credentials_home = out_dir.join("ghor-http-home");
    let remote_login = Command::new(&executable)
        .arg("login")
        .arg("--registry")
        .arg(&http_registry.base)
        .args(["--token", remote_token, "--json"])
        .env("GHOR_HOME", &remote_credentials_home)
        .output()
        .expect("native HTTP login");
    assert!(remote_login.status.success());
    assert!(!String::from_utf8_lossy(&remote_login.stdout).contains(remote_token));
    let remote_login_report: serde_json::Value =
        serde_json::from_slice(&remote_login.stdout).expect("HTTP login JSON");
    assert_eq!(remote_login_report["executor"], "nc-stage1");
    assert_eq!(remote_login_report["auth"], "token_stored");

    let remote_search = Command::new(&executable)
        .args(["search", "material", "--registry"])
        .arg(&http_registry.base)
        .args(["--token", remote_token, "--json"])
        .output()
        .expect("native HTTP search");
    assert!(remote_search.status.success());
    let remote_search_report: serde_json::Value =
        serde_json::from_slice(&remote_search.stdout).expect("HTTP search JSON");
    assert_eq!(remote_search_report["executor"], "nc-stage1");
    assert_eq!(remote_search_report["registry_transport"], "http");
    assert_eq!(remote_search_report["packages"][0]["name"], "material_dep");

    let remote_info = Command::new(&executable)
        .args(["info", "material_dep", "--registry"])
        .arg(&http_registry.base)
        .args(["--token", remote_token, "--json"])
        .output()
        .expect("native HTTP info");
    assert!(remote_info.status.success());
    let remote_info_report: serde_json::Value =
        serde_json::from_slice(&remote_info.stdout).expect("HTTP info JSON");
    assert_eq!(remote_info_report["executor"], "nc-stage1");
    assert_eq!(remote_info_report["packages"][0]["version"], "0.1.0");

    let remote_registry = Command::new(&executable)
        .args(["registry", "config", "--registry"])
        .arg(&http_registry.base)
        .args(["--token", remote_token, "--json"])
        .output()
        .expect("native HTTP registry config");
    assert!(remote_registry.status.success());
    let remote_registry_report: serde_json::Value =
        serde_json::from_slice(&remote_registry.stdout).expect("HTTP registry config JSON");
    assert_eq!(remote_registry_report["executor"], "nc-stage1");
    assert_eq!(remote_registry_report["registry_transport"], "http");
    assert_eq!(remote_registry_report["config"]["auth-required"], true);

    let remote_logout = Command::new(&executable)
        .arg("logout")
        .arg("--registry")
        .arg(&http_registry.base)
        .arg("--json")
        .env("GHOR_HOME", &remote_credentials_home)
        .output()
        .expect("native HTTP logout");
    assert!(remote_logout.status.success());
    let remote_logout_report: serde_json::Value =
        serde_json::from_slice(&remote_logout.stdout).expect("HTTP logout JSON");
    assert_eq!(remote_logout_report["executor"], "nc-stage1");
    assert_eq!(remote_logout_report["removed"], true);

    let http_consumer = out_dir.join("http-material-consumer");
    std::fs::create_dir_all(&http_consumer).expect("HTTP material consumer");
    let http_manifest = http_consumer.join("ghor.toml");
    std::fs::copy(&material_manifest, &http_manifest).expect("HTTP consumer manifest");
    let http_fetch = Command::new(&executable)
        .arg("fetch")
        .arg("--manifest-path")
        .arg(&http_manifest)
        .arg("--registry")
        .arg(&http_registry.base)
        .arg("--json")
        .output()
        .expect("run native HTTP fetch");
    assert!(
        http_fetch.status.success(),
        "HTTP fetch stdout={} stderr={}",
        String::from_utf8_lossy(&http_fetch.stdout),
        String::from_utf8_lossy(&http_fetch.stderr)
    );
    let http_report: serde_json::Value =
        serde_json::from_slice(&http_fetch.stdout).expect("HTTP fetch JSON");
    assert_eq!(http_report["executor"], "nc-stage1");
    assert_eq!(http_report["registry_transport"], "http");
    assert_eq!(http_report["network"], "materialized_remote");
    assert_eq!(http_report["fetched"][0]["registry_transport"], "http");
    assert_eq!(
        sha256(
            &std::fs::read(
                http_report["fetched"][0]["cache_path"]
                    .as_str()
                    .expect("HTTP cache path")
            )
            .expect("HTTP cached archive")
        ),
        archive_hash
    );
    let offline_http_fetch = Command::new(&executable)
        .arg("fetch")
        .arg("--manifest-path")
        .arg(&http_manifest)
        .arg("--registry")
        .arg(&http_registry.base)
        .arg("--offline")
        .arg("--json")
        .output()
        .expect("reuse native HTTP cache offline");
    assert!(offline_http_fetch.status.success());
    let offline_http_report: serde_json::Value =
        serde_json::from_slice(&offline_http_fetch.stdout).expect("offline HTTP fetch JSON");
    assert_eq!(offline_http_report["status"], "ok");

    let sparse_registry = format!("sparse+{}", http_registry.base);
    let sparse_consumer = out_dir.join("sparse-material-consumer");
    std::fs::create_dir_all(&sparse_consumer).expect("sparse material consumer");
    let sparse_manifest = sparse_consumer.join("ghor.toml");
    std::fs::copy(&material_manifest, &sparse_manifest).expect("sparse consumer manifest");
    let sparse_fetch = Command::new(&executable)
        .arg("fetch")
        .arg("--manifest-path")
        .arg(&sparse_manifest)
        .arg("--registry")
        .arg(&sparse_registry)
        .arg("--json")
        .output()
        .expect("run native sparse HTTP fetch");
    assert!(
        sparse_fetch.status.success(),
        "sparse fetch stdout={} stderr={}",
        String::from_utf8_lossy(&sparse_fetch.stdout),
        String::from_utf8_lossy(&sparse_fetch.stderr)
    );
    let sparse_report: serde_json::Value =
        serde_json::from_slice(&sparse_fetch.stdout).expect("sparse fetch JSON");
    assert_eq!(sparse_report["registry_transport"], "sparse-http");

    let remote_owner = Command::new(&executable)
        .args(["owner", "add", "material_dep", "remote-owner", "--registry"])
        .arg(&http_registry.base)
        .args(["--token", remote_token, "--json"])
        .output()
        .expect("native HTTP owner add");
    assert!(
        remote_owner.status.success(),
        "HTTP owner stdout={} stderr={}",
        String::from_utf8_lossy(&remote_owner.stdout),
        String::from_utf8_lossy(&remote_owner.stderr)
    );
    let remote_owner_report: serde_json::Value =
        serde_json::from_slice(&remote_owner.stdout).expect("HTTP owner JSON");
    assert_eq!(remote_owner_report["executor"], "nc-stage1");
    assert_eq!(remote_owner_report["transaction"], "committed");

    let remote_yank = Command::new(&executable)
        .args(["yank", "material_dep", "--version", "0.1.0", "--registry"])
        .arg(&http_registry.base)
        .args(["--token", remote_token, "--json"])
        .output()
        .expect("native HTTP yank");
    assert!(
        remote_yank.status.success(),
        "HTTP yank stdout={} stderr={}",
        String::from_utf8_lossy(&remote_yank.stdout),
        String::from_utf8_lossy(&remote_yank.stderr)
    );

    let remote_publish = Command::new(&executable)
        .arg("publish")
        .arg("--manifest-path")
        .arg(&publish_manifest)
        .arg("--registry")
        .arg(&http_registry.base)
        .args(["--token", remote_token, "--allow-dirty", "--json"])
        .output()
        .expect("native HTTP publish");
    assert!(
        remote_publish.status.success(),
        "HTTP publish stdout={} stderr={}",
        String::from_utf8_lossy(&remote_publish.stdout),
        String::from_utf8_lossy(&remote_publish.stderr)
    );
    let remote_publish_report: serde_json::Value =
        serde_json::from_slice(&remote_publish.stdout).expect("HTTP publish JSON");
    assert_eq!(remote_publish_report["executor"], "nc-stage1");
    assert_eq!(remote_publish_report["web_api"], "committed");
    assert!(material_registry
        .join("packages/phase10_native/1.0.0/phase10_native-1.0.0.crate")
        .is_file());

    let remote_governance = Command::new(&executable)
        .arg("governance")
        .arg("--registry")
        .arg(&http_registry.base)
        .args([
            "--init",
            "--role",
            "remote-admin:admin",
            "--token",
            remote_token,
            "--json",
        ])
        .output()
        .expect("native HTTP governance");
    assert!(
        remote_governance.status.success(),
        "HTTP governance stdout={} stderr={}",
        String::from_utf8_lossy(&remote_governance.stdout),
        String::from_utf8_lossy(&remote_governance.stderr)
    );
    assert!(!String::from_utf8_lossy(&remote_governance.stdout).contains(remote_token));
    let remote_governance_report: serde_json::Value =
        serde_json::from_slice(&remote_governance.stdout).expect("HTTP governance JSON");
    assert_eq!(remote_governance_report["executor"], "nc-stage1");
    assert_eq!(remote_governance_report["registry_transport"], "http");
    assert_eq!(
        remote_governance_report["policy"]["roles"]["remote-admin"],
        "admin"
    );
    assert!(material_registry.join("governance.json").is_file());

    for (action, expected_status) in [("block", "blocked"), ("restore", "restored")] {
        let remote_moderation = Command::new(&executable)
            .args(["moderation", "phase10_native", "--version", "1.0.0"])
            .arg("--registry")
            .arg(&http_registry.base)
            .args([
                "--action",
                action,
                "--reason",
                "phase24-http-contract",
                "--token",
                remote_token,
                "--json",
            ])
            .output()
            .expect("native HTTP moderation");
        assert!(
            remote_moderation.status.success(),
            "HTTP moderation {action} stdout={} stderr={}",
            String::from_utf8_lossy(&remote_moderation.stdout),
            String::from_utf8_lossy(&remote_moderation.stderr)
        );
        let remote_moderation_report: serde_json::Value =
            serde_json::from_slice(&remote_moderation.stdout).expect("HTTP moderation JSON");
        assert_eq!(remote_moderation_report["executor"], "nc-stage1");
        assert_eq!(remote_moderation_report["registry_transport"], "http");
        assert_eq!(
            remote_moderation_report["moderation_status"],
            expected_status
        );
    }
    assert!(material_registry.join("moderation.json").is_file());

    let http_cdn_root = out_dir.join("phase24-http-cdn");
    let http_cdn = Command::new(&executable)
        .args(["cdn", "build", "--manifest-path"])
        .arg(&publish_manifest)
        .arg("--registry")
        .arg(&http_registry.base)
        .arg("--out")
        .arg(&http_cdn_root)
        .arg("--json")
        .output()
        .expect("native HTTP CDN build");
    assert!(
        http_cdn.status.success(),
        "HTTP CDN stdout={} stderr={}",
        String::from_utf8_lossy(&http_cdn.stdout),
        String::from_utf8_lossy(&http_cdn.stderr)
    );
    let http_cdn_report: serde_json::Value =
        serde_json::from_slice(&http_cdn.stdout).expect("HTTP CDN JSON");
    assert_eq!(http_cdn_report["executor"], "nc-stage1");
    assert_eq!(http_cdn_report["manifest"]["registry_transport"], "http");
    let http_cdn_entry = http_cdn_report["manifest"]["entries"]
        .as_array()
        .expect("HTTP CDN entries")
        .iter()
        .find(|entry| entry["name"] == "phase10_native")
        .expect("HTTP CDN phase10 entry");
    let http_cdn_hash = http_cdn_entry["sha256"]
        .as_str()
        .expect("HTTP CDN checksum");
    assert_eq!(
        sha256(
            &std::fs::read(http_cdn_root.join(format!("blobs/sha256/{http_cdn_hash}.crate")))
                .expect("HTTP CDN blob")
        ),
        http_cdn_hash
    );

    let remote_index: serde_json::Value = serde_json::from_slice(
        &std::fs::read(material_registry.join("index.json")).expect("mutated HTTP index"),
    )
    .expect("mutated HTTP index JSON");
    assert!(remote_index["packages"]
        .as_array()
        .expect("remote packages")
        .iter()
        .any(|entry| {
            entry["name"] == "material_dep"
                && entry["yanked"] == true
                && entry["owners"]
                    .as_array()
                    .is_some_and(|owners| owners.iter().any(|owner| owner == "remote-owner"))
        }));
    let api_events =
        std::fs::read_to_string(material_registry.join("api-events.log")).expect("API events");
    for expected in [
        "PUT /api/v1/crates/material_dep/owners",
        "DELETE /api/v1/crates/material_dep/0.1.0/yank",
        "PUT /api/v1/crates/new",
    ] {
        assert!(
            api_events.contains(expected),
            "missing API event {expected}"
        );
    }

    let remote_unyank = Command::new(&executable)
        .args([
            "yank",
            "material_dep",
            "--version",
            "0.1.0",
            "--undo",
            "--registry",
        ])
        .arg(&http_registry.base)
        .args(["--token", remote_token, "--json"])
        .output()
        .expect("native HTTP unyank");
    assert!(
        remote_unyank.status.success(),
        "HTTP unyank stdout={} stderr={}",
        String::from_utf8_lossy(&remote_unyank.stdout),
        String::from_utf8_lossy(&remote_unyank.stderr)
    );
    assert!(
        std::fs::read_to_string(material_registry.join("api-events.log"))
            .expect("API events after unyank")
            .contains("PUT /api/v1/crates/material_dep/0.1.0/unyank")
    );

    for args in [
        vec!["init"],
        vec!["config", "user.email", "registry-stage1@example.invalid"],
        vec!["config", "user.name", "Ghor Registry Stage1"],
        vec!["add", "."],
        vec!["commit", "-m", "registry fixture"],
    ] {
        let output = Command::new("git")
            .arg("-C")
            .arg(&material_registry)
            .args(args)
            .output()
            .expect("prepare git registry");
        assert!(output.status.success());
    }
    let git_registry_ref = format!(
        "git+{}",
        material_registry.to_string_lossy().replace('\\', "/")
    );
    let git_registry_consumer = out_dir.join("git-registry-consumer");
    std::fs::create_dir_all(&git_registry_consumer).expect("git registry consumer");
    let git_registry_manifest = git_registry_consumer.join("ghor.toml");
    std::fs::copy(&material_manifest, &git_registry_manifest).expect("git registry manifest");
    let git_registry_fetch = Command::new(&executable)
        .arg("fetch")
        .arg("--manifest-path")
        .arg(&git_registry_manifest)
        .arg("--registry")
        .arg(&git_registry_ref)
        .arg("--json")
        .output()
        .expect("run native git registry fetch");
    assert!(
        git_registry_fetch.status.success(),
        "git registry stdout={} stderr={}",
        String::from_utf8_lossy(&git_registry_fetch.stdout),
        String::from_utf8_lossy(&git_registry_fetch.stderr)
    );
    let git_registry_report: serde_json::Value =
        serde_json::from_slice(&git_registry_fetch.stdout).expect("git registry fetch JSON");
    assert_eq!(git_registry_report["executor"], "nc-stage1");
    assert_eq!(git_registry_report["registry_transport"], "git");
    assert_eq!(
        git_registry_report["fetched"][0]["registry_transport"],
        "git"
    );
    assert_eq!(
        sha256(
            &std::fs::read(
                git_registry_report["fetched"][0]["cache_path"]
                    .as_str()
                    .expect("git registry cache path")
            )
            .expect("git registry cached archive")
        ),
        archive_hash
    );

    let replacement_consumer = out_dir.join("replacement-consumer");
    std::fs::create_dir_all(&replacement_consumer).expect("replacement consumer");
    let replacement_manifest = replacement_consumer.join("ghor.toml");
    let material_registry_toml = material_registry.to_string_lossy().replace('\\', "/");
    std::fs::write(
        &replacement_manifest,
        format!(
            "[package]\nname = \"replacement_consumer\"\nversion = \"0.1.0\"\n\n[dependencies]\nmaterial_dep = {{ version = \"0.1.0\", checksum = \"{archive_hash}\" }}\n\n[source.crates-io]\nreplace-with = \"first-hop\"\n\n[source.first-hop]\nreplace-with = \"mirror\"\n\n[registries.mirror]\nindex = \"{material_registry_toml}\"\n"
        ),
    )
    .expect("source replacement manifest");
    let replacement_resolve = Command::new(&executable)
        .args(["resolve", "--manifest-path"])
        .arg(&replacement_manifest)
        .arg("--json")
        .output()
        .expect("resolve source replacement chain");
    assert!(
        replacement_resolve.status.success(),
        "replacement stdout={} stderr={}",
        String::from_utf8_lossy(&replacement_resolve.stdout),
        String::from_utf8_lossy(&replacement_resolve.stderr)
    );
    let replacement_report: serde_json::Value =
        serde_json::from_slice(&replacement_resolve.stdout).expect("source replacement JSON");
    assert_eq!(replacement_report["executor"], "nc-stage1");
    assert_eq!(replacement_report["resolved"][0]["name"], "material_dep");
    assert_eq!(
        replacement_report["resolved"][0]["source"]["name"],
        "crates-io"
    );
    assert_eq!(
        replacement_report["resolved"][0]["source"]["effective"],
        "mirror"
    );
    assert_eq!(
        replacement_report["resolved"][0]["source"]["replacement_kind"],
        "alternate_registry"
    );

    let cycle_manifest = replacement_consumer.join("cycle.toml");
    std::fs::write(
        &cycle_manifest,
        format!(
            "[package]\nname = \"cycle_consumer\"\nversion = \"0.1.0\"\n\n[dependencies]\nmaterial_dep = \"0.1.0\"\n\n[source.crates-io]\nreplace-with = \"first-hop\"\n\n[source.first-hop]\nreplace-with = \"crates-io\"\n\n[registries.mirror]\nindex = \"{material_registry_toml}\"\n"
        ),
    )
    .expect("source replacement cycle manifest");
    let cycle_resolve = Command::new(&executable)
        .args(["resolve", "--manifest-path"])
        .arg(&cycle_manifest)
        .arg("--json")
        .output()
        .expect("reject source replacement cycle");
    assert_eq!(cycle_resolve.status.code(), Some(2));
    let cycle_report: serde_json::Value =
        serde_json::from_slice(&cycle_resolve.stdout).expect("source replacement cycle JSON");
    assert_eq!(
        cycle_report["diagnostics"][0]["code"],
        "E_GHOR_REGISTRY_RESOLUTION_001"
    );
    let missing_source_manifest = replacement_consumer.join("missing-source.toml");
    std::fs::write(
        &missing_source_manifest,
        "[package]\nname = \"missing_source\"\nversion = \"0.1.0\"\n\n[dependencies]\nmaterial_dep = \"0.1.0\"\n\n[source.crates-io]\nreplace-with = \"undefined-mirror\"\n",
    )
    .expect("missing replacement source manifest");
    let missing_source = Command::new(&executable)
        .args(["resolve", "--manifest-path"])
        .arg(&missing_source_manifest)
        .arg("--json")
        .output()
        .expect("reject missing replacement source");
    assert_eq!(missing_source.status.code(), Some(2));
    let missing_source_report: serde_json::Value =
        serde_json::from_slice(&missing_source.stdout).expect("missing replacement source JSON");
    assert_eq!(
        missing_source_report["diagnostics"][0]["code"],
        "E_GHOR_REGISTRY_RESOLUTION_001"
    );

    let basal_registry = out_dir.join("basal-registry");
    let basal_source = workspace.join("_archive/old_examples/basal/ffi/foreign_c_contract.nb");
    let basal_publish = Command::new(binary("ncc"))
        .args(["basal", "publish"])
        .arg(&basal_source)
        .arg("--registry")
        .arg(&basal_registry)
        .arg("--json")
        .output()
        .expect("publish official basal package");
    assert!(
        basal_publish.status.success(),
        "basal publish stdout={} stderr={}",
        String::from_utf8_lossy(&basal_publish.stdout),
        String::from_utf8_lossy(&basal_publish.stderr)
    );
    let basal_publish_report: serde_json::Value =
        serde_json::from_slice(&basal_publish.stdout).expect("basal publish JSON");
    assert_eq!(basal_publish_report["schema"], "basal.registry.publish.v1");
    assert_eq!(basal_publish_report["status"], "published");
    let basal_checksum = basal_publish_report["checksum"]
        .as_str()
        .expect("basal checksum");
    let basal_safe_name = basal_publish_report["safe_name"]
        .as_str()
        .expect("basal safe name");
    let basal_version = basal_publish_report["version"]
        .as_str()
        .expect("basal version");
    let basal_consumer = out_dir.join("basal-consumer");
    std::fs::create_dir_all(&basal_consumer).expect("basal consumer");
    let basal_manifest = basal_consumer.join("ghor.toml");
    let basal_registry_toml = basal_registry.to_string_lossy().replace('\\', "/");
    std::fs::write(
        &basal_manifest,
        format!(
            "[package]\nname = \"basal_consumer\"\nversion = \"0.1.0\"\n\n[basal-dependencies]\n{basal_safe_name} = {{ version = \"{basal_version}\", registry = \"{basal_registry_toml}\", package = \"nC.root.ffi.foreign_c\", checksum = \"{basal_checksum}\" }}\n"
        ),
    )
    .expect("basal consumer manifest");
    let basal_resolve = Command::new(&executable)
        .args(["resolve", "--manifest-path"])
        .arg(&basal_manifest)
        .arg("--json")
        .output()
        .expect("resolve basal dependency");
    assert!(
        basal_resolve.status.success(),
        "basal resolve stdout={} stderr={}",
        String::from_utf8_lossy(&basal_resolve.stdout),
        String::from_utf8_lossy(&basal_resolve.stderr)
    );
    let basal_resolve_report: serde_json::Value =
        serde_json::from_slice(&basal_resolve.stdout).expect("basal resolve JSON");
    assert_eq!(basal_resolve_report["executor"], "nc-stage1");
    assert_eq!(
        basal_resolve_report["resolved"][0]["status"],
        "resolved_basal_registry"
    );
    assert_eq!(basal_resolve_report["resolved"][0]["package_kind"], "basal");
    assert_eq!(
        basal_resolve_report["resolved"][0]["package_manifest_validation"]["status"],
        "validated"
    );
    assert_eq!(
        basal_resolve_report["resolved"][0]["package_manifest_validation"]["nbb"]["magic"],
        "NBB0"
    );
    assert_eq!(
        basal_resolve_report["resolved"][0]["package_manifest_validation"]["ffi"]["status"],
        "validated"
    );

    let basal_lock = Command::new(&executable)
        .args(["lock", "--manifest-path"])
        .arg(&basal_manifest)
        .arg("--json")
        .output()
        .expect("lock basal dependency");
    assert!(basal_lock.status.success());
    let basal_lock_text =
        std::fs::read_to_string(basal_consumer.join("ghor.lock")).expect("basal lockfile");
    assert!(basal_lock_text.contains("source = \"basal+"));
    assert!(basal_lock_text.contains(&format!("checksum = \"{basal_checksum}\"")));

    let basal_fetch = Command::new(&executable)
        .args(["fetch", "--manifest-path"])
        .arg(&basal_manifest)
        .arg("--json")
        .output()
        .expect("fetch basal dependency");
    assert!(basal_fetch.status.success());
    let basal_fetch_report: serde_json::Value =
        serde_json::from_slice(&basal_fetch.stdout).expect("basal fetch JSON");
    assert_eq!(
        basal_fetch_report["fetched"][0]["status"],
        "fetched_basal_registry"
    );
    assert_eq!(basal_fetch_report["fetched"][0]["package_kind"], "basal");
    let basal_cached = PathBuf::from(
        basal_fetch_report["fetched"][0]["cache_path"]
            .as_str()
            .expect("basal cache path"),
    );
    assert!(basal_cached
        .to_string_lossy()
        .contains("basal-registry-cache"));
    assert_eq!(
        sha256(&std::fs::read(basal_cached).expect("basal cached archive")),
        basal_checksum
    );

    let basal_vendor = out_dir.join("basal-vendor");
    let basal_vendor_output = Command::new(&executable)
        .arg("vendor")
        .arg(&basal_vendor)
        .args(["--manifest-path"])
        .arg(&basal_manifest)
        .args(["--versioned-dirs", "--json"])
        .output()
        .expect("vendor basal dependency");
    assert!(basal_vendor_output.status.success());
    let basal_vendor_report: serde_json::Value =
        serde_json::from_slice(&basal_vendor_output.stdout).expect("basal vendor JSON");
    assert_eq!(basal_vendor_report["vendored"][0]["package_kind"], "basal");
    let basal_vendor_root = basal_vendor.join(format!("{basal_safe_name}-{basal_version}"));
    let basal_vendor_manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(basal_vendor_root.join("manifest.json")).expect("vendored basal manifest"),
    )
    .expect("vendored basal manifest JSON");
    assert_eq!(basal_vendor_manifest["schema"], "basal.package.v1");
    assert_eq!(basal_vendor_manifest["nbb"]["magic"], "NBB0");
    let basal_checksum_document: serde_json::Value = serde_json::from_slice(
        &std::fs::read(basal_vendor_root.join(".basal-checksum.json"))
            .expect("vendored basal checksum document"),
    )
    .expect("vendored basal checksum JSON");
    assert_eq!(
        basal_checksum_document["schema"],
        "ghor.basal.vendor.checksum.v1"
    );
    assert_eq!(
        basal_checksum_document["basal"]["root_id"],
        "nC.root.ffi.foreign_c"
    );

    let basal_http = LocalRegistryServer::start(basal_registry.clone());
    let basal_http_manifest = basal_consumer.join("http.toml");
    std::fs::write(
        &basal_http_manifest,
        format!(
            "[package]\nname = \"basal_http\"\nversion = \"0.1.0\"\n\n[basal-dependencies]\n{basal_safe_name} = {{ version = \"{basal_version}\", registry = \"{}\", package = \"nC.root.ffi.foreign_c\", checksum = \"{basal_checksum}\" }}\n",
            basal_http.base
        ),
    )
    .expect("HTTP basal manifest");
    let basal_http_fetch = Command::new(&executable)
        .args(["fetch", "--manifest-path"])
        .arg(&basal_http_manifest)
        .arg("--json")
        .output()
        .expect("fetch basal dependency over HTTP");
    assert!(
        basal_http_fetch.status.success(),
        "basal HTTP stdout={} stderr={}",
        String::from_utf8_lossy(&basal_http_fetch.stdout),
        String::from_utf8_lossy(&basal_http_fetch.stderr)
    );
    let basal_http_report: serde_json::Value =
        serde_json::from_slice(&basal_http_fetch.stdout).expect("basal HTTP JSON");
    assert_eq!(
        basal_http_report["fetched"][0]["registry_transport"],
        "http"
    );

    let basal_bad_checksum_manifest = basal_consumer.join("bad-checksum.toml");
    std::fs::write(
        &basal_bad_checksum_manifest,
        format!(
            "[package]\nname = \"basal_bad_checksum\"\nversion = \"0.1.0\"\n\n[basal-dependencies]\n{basal_safe_name} = {{ version = \"{basal_version}\", registry = \"{basal_registry_toml}\", package = \"nC.root.ffi.foreign_c\", checksum = \"{}\" }}\n",
            "00".repeat(32)
        ),
    )
    .expect("bad basal checksum manifest");
    let basal_bad_checksum = Command::new(&executable)
        .args(["resolve", "--manifest-path"])
        .arg(&basal_bad_checksum_manifest)
        .arg("--json")
        .output()
        .expect("reject bad basal checksum");
    assert_eq!(basal_bad_checksum.status.code(), Some(2));
    let basal_bad_checksum_report: serde_json::Value =
        serde_json::from_slice(&basal_bad_checksum.stdout).expect("bad basal checksum JSON");
    assert_eq!(
        basal_bad_checksum_report["diagnostics"][0]["code"],
        "E_GHOR_BASAL_PACKAGE_001"
    );

    let basal_package_manifest = basal_registry.join(
        basal_publish_report["package_manifest"]
            .as_str()
            .expect("basal package manifest path"),
    );
    let basal_index_path = basal_registry.join("index.json");
    let original_basal_manifest =
        std::fs::read(&basal_package_manifest).expect("original basal package manifest");
    let original_basal_index = std::fs::read(&basal_index_path).expect("original basal index");
    let mut corrupt_manifest: serde_json::Value =
        serde_json::from_slice(&original_basal_manifest).expect("basal manifest JSON");
    corrupt_manifest["schema"] = serde_json::Value::String("basal.package.invalid".into());
    let corrupt_manifest_bytes =
        serde_json::to_vec_pretty(&corrupt_manifest).expect("corrupt basal manifest bytes");
    std::fs::write(&basal_package_manifest, &corrupt_manifest_bytes)
        .expect("write corrupt basal manifest");
    let mut corrupt_index: serde_json::Value =
        serde_json::from_slice(&original_basal_index).expect("basal index JSON");
    corrupt_index["packages"][0]["package_manifest_hash"] =
        serde_json::Value::String(sha256(&corrupt_manifest_bytes));
    std::fs::write(
        &basal_index_path,
        serde_json::to_vec_pretty(&corrupt_index).expect("corrupt basal index bytes"),
    )
    .expect("write corrupt basal index");
    let basal_invalid_contract = Command::new(&executable)
        .args(["resolve", "--manifest-path"])
        .arg(&basal_manifest)
        .arg("--json")
        .output()
        .expect("reject invalid basal contract");
    assert_eq!(basal_invalid_contract.status.code(), Some(2));
    let basal_invalid_report: serde_json::Value =
        serde_json::from_slice(&basal_invalid_contract.stdout).expect("invalid basal JSON");
    assert_eq!(
        basal_invalid_report["diagnostics"][0]["code"],
        "E_GHOR_BASAL_PACKAGE_001"
    );
    std::fs::write(&basal_package_manifest, original_basal_manifest)
        .expect("restore basal package manifest");
    std::fs::write(&basal_index_path, original_basal_index).expect("restore basal index");

    let launcher = out_dir.join(if cfg!(windows) { "ghor.exe" } else { "ghor" });
    std::fs::copy(binary("ghor"), &launcher).expect("copy stage0 launcher beside stage1");
    let delegated_bootstrap = Command::new(&launcher)
        .args(["bootstrap", "--json"])
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("delegate bootstrap to sibling nC stage1");
    assert!(delegated_bootstrap.status.success());
    let delegated_bootstrap_report: serde_json::Value =
        serde_json::from_slice(&delegated_bootstrap.stdout).expect("delegated bootstrap JSON");
    assert_eq!(delegated_bootstrap_report["active_executor"], "nc-stage1");
    let delegated_forge = out_dir.join("delegated-language-forge");
    let delegated_language = Command::new(&launcher)
        .args(["lang", "new", "delegated_forge", "--host", "nC"])
        .arg("--out")
        .arg(&delegated_forge)
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("delegate language forge to sibling nC stage1");
    assert!(delegated_language.status.success());
    let delegated_language_report: serde_json::Value =
        serde_json::from_slice(&delegated_language.stdout).expect("delegated language JSON");
    assert_eq!(delegated_language_report["executor"], "nc-stage1");
    assert!(delegated_forge.join("src/lexer.n").is_file());

    for (arguments, schema, status) in [
        (
            vec!["machine", "doctor", "--json"],
            None,
            Some("machine_native_asm_total_closed"),
        ),
        (
            vec!["basal", "doctor", "--json"],
            None,
            Some("basal_phase7_release_seal_closed"),
        ),
        (
            vec!["runtime", "abi", "--json"],
            Some("nc.runtime.symbol_abi.v1"),
            None,
        ),
        (vec!["compiler", "doctor", "--json"], None, Some("ok")),
        (
            vec!["primitive", "doctor", "--json"],
            Some("nc.primitive.doctor.v1"),
            None,
        ),
    ] {
        let output = Command::new(&launcher)
            .args(&arguments)
            .env("NC_GHOR_NCC", &stage1_ncc)
            .env_remove("NC_GHOR_STAGE1")
            .env_remove("NC_GHOR_FORCE_STAGE0")
            .output()
            .unwrap_or_else(|error| panic!("native subsystem {arguments:?}: {error}"));
        assert!(
            output.status.success(),
            "native subsystem {arguments:?} stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("native subsystem {arguments:?} JSON"));
        if let Some(schema) = schema {
            assert_eq!(report["schema"], schema);
        }
        if let Some(status) = status {
            assert_eq!(report["status"], status);
        }
    }

    let subsystem_failure = Command::new(&launcher)
        .args(["runtime", "unknown-action"])
        .env("NC_GHOR_NCC", &stage1_ncc)
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("native subsystem failure propagation");
    assert!(!subsystem_failure.status.success());
    assert!(String::from_utf8_lossy(&subsystem_failure.stderr).contains("unknown-action"));

    let discovered = Command::new(&launcher)
        .arg("manifest")
        .arg("--manifest-path")
        .arg(&fixture)
        .arg("--json")
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("run side-by-side stage1 discovery");
    assert!(
        discovered.status.success(),
        "discovered stage1 stdout={} stderr={}",
        String::from_utf8_lossy(&discovered.stdout),
        String::from_utf8_lossy(&discovered.stderr)
    );
    let discovered_report: serde_json::Value =
        serde_json::from_slice(&discovered.stdout).expect("discovered stage1 JSON");
    assert_eq!(discovered_report["executor"], "nc-stage1");
    let discovered_metadata = Command::new(&launcher)
        .arg("metadata")
        .arg("--manifest-path")
        .arg(&app_manifest)
        .arg("--json")
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("run discovered metadata stage1");
    assert!(discovered_metadata.status.success());
    let discovered_metadata_report: serde_json::Value =
        serde_json::from_slice(&discovered_metadata.stdout).expect("discovered metadata JSON");
    assert_eq!(discovered_metadata_report["executor"], "nc-stage1");

    let discovered_provider = Command::new(&launcher)
        .args(["registry", "cargo-config", "--manifest-path"])
        .arg(&provider_manifest)
        .args(["--registry", "secure", "--json"])
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("run discovered credential provider through stage1");
    assert!(
        discovered_provider.status.success(),
        "discovered provider stdout={} stderr={}",
        String::from_utf8_lossy(&discovered_provider.stdout),
        String::from_utf8_lossy(&discovered_provider.stderr)
    );
    let discovered_provider_report: serde_json::Value =
        serde_json::from_slice(&discovered_provider.stdout).expect("discovered provider JSON");
    assert_eq!(discovered_provider_report["executor"], "nc-stage1");
    assert_eq!(discovered_provider_report["credential_status"], "available");
    assert!(!String::from_utf8_lossy(&discovered_provider.stdout).contains("phase12-secret"));

    let discovered_add = Command::new(&launcher)
        .current_dir(&edit_project)
        .args(["add", "child@1.0.0", "--path", "child", "--json"])
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("run discovered native add");
    assert!(
        discovered_add.status.success(),
        "discovered add stdout={} stderr={}",
        String::from_utf8_lossy(&discovered_add.stdout),
        String::from_utf8_lossy(&discovered_add.stderr)
    );
    let discovered_add_report: serde_json::Value =
        serde_json::from_slice(&discovered_add.stdout).expect("discovered add JSON");
    assert_eq!(discovered_add_report["executor"], "nc-stage1");
    let discovered_tree = Command::new(&launcher)
        .arg("tree")
        .arg("--manifest-path")
        .arg(&edit_manifest)
        .arg("--json")
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("run discovered native tree");
    assert!(discovered_tree.status.success());
    let discovered_tree_report: serde_json::Value =
        serde_json::from_slice(&discovered_tree.stdout).expect("discovered tree JSON");
    assert_eq!(discovered_tree_report["executor"], "nc-stage1");
    let discovered_remove = Command::new(&launcher)
        .current_dir(&edit_project)
        .args(["remove", "child", "--json"])
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("run discovered native remove");
    assert!(discovered_remove.status.success());
    let discovered_remove_report: serde_json::Value =
        serde_json::from_slice(&discovered_remove.stdout).expect("discovered remove JSON");
    assert_eq!(discovered_remove_report["executor"], "nc-stage1");
    assert_eq!(
        std::fs::read_to_string(&edit_manifest).expect("launcher restored edit manifest"),
        edit_source
    );

    let discovered_resolve = Command::new(&launcher)
        .arg("resolve")
        .arg("--manifest-path")
        .arg(&resolver_manifest)
        .arg("--registry")
        .arg(&registry)
        .arg("--json")
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("run discovered local resolver stage1");
    assert!(
        discovered_resolve.status.success(),
        "discovered resolve stdout={} stderr={}",
        String::from_utf8_lossy(&discovered_resolve.stdout),
        String::from_utf8_lossy(&discovered_resolve.stderr)
    );
    let discovered_resolve_report: serde_json::Value =
        serde_json::from_slice(&discovered_resolve.stdout).expect("discovered resolve JSON");
    assert_eq!(discovered_resolve_report["executor"], "nc-stage1");

    let discovered_fetch = Command::new(&launcher)
        .arg("fetch")
        .arg("--manifest-path")
        .arg(&material_manifest)
        .arg("--registry")
        .arg(&material_registry)
        .arg("--json")
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("run discovered local fetch stage1");
    assert!(discovered_fetch.status.success());
    let discovered_fetch_report: serde_json::Value =
        serde_json::from_slice(&discovered_fetch.stdout).expect("discovered fetch JSON");
    assert_eq!(discovered_fetch_report["executor"], "nc-stage1");

    let discovered_http_fetch = Command::new(&launcher)
        .arg("fetch")
        .arg("--manifest-path")
        .arg(&http_manifest)
        .arg("--registry")
        .arg(&http_registry.base)
        .arg("--json")
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("run discovered HTTP fetch stage1");
    assert!(discovered_http_fetch.status.success());
    let discovered_http_report: serde_json::Value =
        serde_json::from_slice(&discovered_http_fetch.stdout).expect("discovered HTTP fetch JSON");
    assert_eq!(discovered_http_report["executor"], "nc-stage1");
    assert_eq!(discovered_http_report["registry_transport"], "http");

    let discovered_replacement = Command::new(&launcher)
        .args(["resolve", "--manifest-path"])
        .arg(&replacement_manifest)
        .arg("--json")
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("run source replacement through discovered stage1");
    assert!(
        discovered_replacement.status.success(),
        "launcher replacement stdout={} stderr={}",
        String::from_utf8_lossy(&discovered_replacement.stdout),
        String::from_utf8_lossy(&discovered_replacement.stderr)
    );
    let discovered_replacement_report: serde_json::Value =
        serde_json::from_slice(&discovered_replacement.stdout)
            .expect("launcher source replacement JSON");
    assert_eq!(discovered_replacement_report["executor"], "nc-stage1");
    assert_eq!(
        discovered_replacement_report["resolved"][0]["source"]["effective"],
        "mirror"
    );

    let external_config_consumer = out_dir.join("external-config-consumer");
    std::fs::create_dir_all(external_config_consumer.join(".cargo"))
        .expect("external Cargo config directory");
    std::fs::create_dir_all(registry.join("se/mv")).expect("Cargo index shard");
    std::fs::write(
        registry.join("se/mv/semver_dep"),
        format!(
            "{}\n",
            serde_json::json!({
                "name": "semver_dep",
                "vers": "1.5.0",
                "deps": [],
                "cksum": "2".repeat(64),
                "features": {"default": []},
                "yanked": false
            })
        ),
    )
    .expect("Cargo index source replacement entry");
    let external_config_manifest = external_config_consumer.join("ghor.toml");
    std::fs::write(
        &external_config_manifest,
        format!(
            "[package]\nname = \"external_config_consumer\"\nversion = \"0.1.0\"\n\n[dependencies]\nsemver_dep = {{ version = \"^1.0\", checksum = \"{}\" }}\n",
            "2".repeat(64)
        ),
    )
    .expect("external Cargo config consumer manifest");
    let registry_toml = registry.to_string_lossy().replace('\\', "/");
    std::fs::write(
        external_config_consumer.join(".cargo/config.toml"),
        format!(
            "[source.crates-io]\nreplace-with = \"external-mirror\"\n\n[source.external-mirror]\ndirectory = \"{registry_toml}\"\n"
        ),
    )
    .expect("external Cargo source replacement");
    let external_config_resolve = Command::new(&launcher)
        .args(["resolve", "--manifest-path"])
        .arg(&external_config_manifest)
        .arg("--json")
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("resolve external Cargo config through discovered stage1");
    assert!(
        external_config_resolve.status.success(),
        "external config stdout={} stderr={}",
        String::from_utf8_lossy(&external_config_resolve.stdout),
        String::from_utf8_lossy(&external_config_resolve.stderr)
    );
    let external_config_report: serde_json::Value =
        serde_json::from_slice(&external_config_resolve.stdout).expect("external config JSON");
    assert_eq!(external_config_report["executor"], "nc-stage1");
    assert_eq!(
        external_config_report["resolved"][0]["source"]["effective"],
        "external-mirror"
    );

    let discovered_basal = Command::new(&launcher)
        .args(["resolve", "--manifest-path"])
        .arg(&basal_manifest)
        .arg("--json")
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("run basal registry through discovered stage1");
    assert!(
        discovered_basal.status.success(),
        "launcher basal stdout={} stderr={}",
        String::from_utf8_lossy(&discovered_basal.stdout),
        String::from_utf8_lossy(&discovered_basal.stderr)
    );
    let discovered_basal_report: serde_json::Value =
        serde_json::from_slice(&discovered_basal.stdout).expect("launcher basal JSON");
    assert_eq!(discovered_basal_report["executor"], "nc-stage1");
    assert_eq!(
        discovered_basal_report["resolved"][0]["status"],
        "resolved_basal_registry"
    );

    let git_package = out_dir.join("git-package");
    std::fs::create_dir_all(git_package.join("src")).expect("git package source");
    std::fs::write(
        git_package.join("ghor.toml"),
        "[package]\nname = \"remote_dep\"\nversion = \"0.3.0\"\nkind = \"lib\"\n",
    )
    .expect("git package manifest");
    std::fs::write(
        git_package.join("src/main.n"),
        "remoteDep { value :: fn() -> int { return 3; } fa: return true; }\n",
    )
    .expect("git package source");
    for args in [
        vec!["init"],
        vec!["config", "user.email", "ghor-stage1@example.invalid"],
        vec!["config", "user.name", "Ghor Stage1"],
        vec!["add", "."],
        vec!["commit", "-m", "fixture"],
    ] {
        let output = Command::new("git")
            .arg("-C")
            .arg(&git_package)
            .args(args)
            .output()
            .expect("prepare git dependency");
        assert!(
            output.status.success(),
            "git fixture stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let git_head = Command::new("git")
        .arg("-C")
        .arg(&git_package)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git fixture HEAD");
    assert!(git_head.status.success());
    let git_head = String::from_utf8(git_head.stdout)
        .expect("git HEAD UTF-8")
        .trim()
        .to_string();
    let git_consumer = out_dir.join("git-consumer");
    std::fs::create_dir_all(&git_consumer).expect("git consumer");
    let git_manifest = git_consumer.join("ghor.toml");
    let git_url = git_package.to_string_lossy().replace('\\', "/");

    let git_edit = out_dir.join("git-edit-project");
    std::fs::create_dir_all(&git_edit).expect("git edit project");
    let git_edit_manifest = git_edit.join("ghor.toml");
    let git_edit_source = "[package]\nname = \"git_edit\"\nversion = \"0.1.0\"\n";
    std::fs::write(&git_edit_manifest, git_edit_source).expect("git edit manifest");
    let git_add = Command::new(&executable)
        .arg("add")
        .arg("remote_dep@0.3.0")
        .arg("--git")
        .arg(&git_url)
        .arg("--rev")
        .arg(&git_head)
        .arg("--no-default-features")
        .arg("--manifest-path")
        .arg(&git_edit_manifest)
        .arg("--json")
        .output()
        .expect("native git add");
    assert!(
        git_add.status.success(),
        "git add stdout={} stderr={}",
        String::from_utf8_lossy(&git_add.stdout),
        String::from_utf8_lossy(&git_add.stderr)
    );
    let git_tree = Command::new(&executable)
        .arg("tree")
        .arg("--manifest-path")
        .arg(&git_edit_manifest)
        .arg("--json")
        .output()
        .expect("native git tree after add");
    assert!(
        git_tree.status.success(),
        "git tree stdout={} stderr={}",
        String::from_utf8_lossy(&git_tree.stdout),
        String::from_utf8_lossy(&git_tree.stderr)
    );
    let git_tree_report: serde_json::Value =
        serde_json::from_slice(&git_tree.stdout).expect("git tree JSON");
    assert_eq!(git_tree_report["nodes"][0]["status"], "resolved_git");
    assert_eq!(
        git_tree_report["nodes"][0]["source"]["resolved_head"],
        git_head
    );
    let git_remove = Command::new(&executable)
        .arg("remove")
        .arg("remote_dep")
        .arg("--manifest-path")
        .arg(&git_edit_manifest)
        .arg("--json")
        .output()
        .expect("native git remove");
    assert!(git_remove.status.success());
    assert_eq!(
        std::fs::read_to_string(&git_edit_manifest).expect("restored git edit manifest"),
        git_edit_source
    );

    std::fs::write(
        &git_manifest,
        format!(
            "[package]\nname = \"git_consumer\"\nversion = \"0.1.0\"\n\n[dependencies]\nremote_dep = {{ git = \"{git_url}\", rev = \"{git_head}\" }}\n"
        ),
    )
    .expect("git dependency manifest");
    let native_git_resolver = Command::new(&executable)
        .arg("resolve")
        .arg("--manifest-path")
        .arg(&git_manifest)
        .arg("--json")
        .output()
        .expect("resolve native git dependency");
    assert!(
        native_git_resolver.status.success(),
        "git resolve stdout={} stderr={}",
        String::from_utf8_lossy(&native_git_resolver.stdout),
        String::from_utf8_lossy(&native_git_resolver.stderr)
    );
    let native_git_report: serde_json::Value =
        serde_json::from_slice(&native_git_resolver.stdout).expect("git resolve JSON");
    assert_eq!(native_git_report["executor"], "nc-stage1");
    assert_eq!(native_git_report["resolved"][0]["status"], "resolved_git");
    assert_eq!(native_git_report["resolved"][0]["version"], "0.3.0");
    assert_eq!(
        native_git_report["resolved"][0]["source"]["resolved_head"],
        git_head
    );
    let discovered_git_resolver = Command::new(&launcher)
        .arg("resolve")
        .arg("--manifest-path")
        .arg(&git_manifest)
        .arg("--offline")
        .arg("--json")
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("reuse native git dependency offline through launcher");
    assert!(discovered_git_resolver.status.success());
    let discovered_git_report: serde_json::Value =
        serde_json::from_slice(&discovered_git_resolver.stdout).expect("offline git resolve JSON");
    assert_eq!(discovered_git_report["executor"], "nc-stage1");
    assert_eq!(
        discovered_git_report["resolved"][0]["source"]["resolved_head"],
        git_head
    );

    let project_tools_root = out_dir.join("project-tools-created");
    let native_new = Command::new(&executable)
        .args(["new", "project_tools_created", "--out"])
        .arg(&project_tools_root)
        .arg("--json")
        .output()
        .expect("native stage1 new");
    assert!(
        native_new.status.success(),
        "native new stdout={} stderr={}",
        String::from_utf8_lossy(&native_new.stdout),
        String::from_utf8_lossy(&native_new.stderr)
    );
    let native_new_report: serde_json::Value =
        serde_json::from_slice(&native_new.stdout).expect("native new JSON");
    assert_eq!(native_new_report["schema"], "ghor.new.v1");
    assert_eq!(native_new_report["executor"], "nc-stage1");
    assert_eq!(native_new_report["kind"], "bin");
    assert!(project_tools_root.join("ghor.toml").is_file());
    assert!(project_tools_root.join("nc.toml").is_file());
    assert!(project_tools_root.join("src/main.n").is_file());

    for (language, expected) in [
        ("c", "src/main.c"),
        ("cpp", "src/main.cpp"),
        ("asm", "src/main.S"),
        ("rust", "src/main.rs"),
        ("java", "src/main/java/Main.java"),
        ("python", "pyproject.toml"),
        ("javascript", "src/main.js"),
        ("typescript", "src/main.ts"),
    ] {
        let root = out_dir.join(format!("project-tools-{language}"));
        let output = Command::new(&executable)
            .args(["new", "adapter_project", "--out"])
            .arg(&root)
            .args(["--lang", language, "--json"])
            .output()
            .unwrap_or_else(|error| panic!("native {language} scaffold must execute: {error}"));
        assert!(
            output.status.success(),
            "native {language} scaffold stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("native {language} scaffold JSON"));
        assert_eq!(report["language"], language);
        assert_eq!(report["kind"], "foreign");
        assert!(root.join(expected).is_file(), "missing {language} scaffold");
    }

    for (kind, expected) in [
        ("--machine", "machine/main.nm"),
        ("--basal", "basal/root.nb"),
        ("--primitive", "primitive/device.ncp"),
    ] {
        let root = out_dir.join(format!("project-tools-{}", &kind[2..]));
        let output = Command::new(&executable)
            .args(["new", "low_level_project", "--out"])
            .arg(&root)
            .args([kind, "--json"])
            .output()
            .unwrap_or_else(|error| panic!("native {kind} scaffold must execute: {error}"));
        assert!(output.status.success());
        assert!(root.join(expected).is_file(), "missing {kind} scaffold");
    }

    let duplicate_new = Command::new(&executable)
        .args(["new", "project_tools_created", "--out"])
        .arg(&project_tools_root)
        .output()
        .expect("reject duplicate native stage1 new");
    assert_eq!(duplicate_new.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&duplicate_new.stdout).contains("E_GHOR_NEW_002"));

    let project_tools_manifest = project_tools_root.join("ghor.toml");
    for (command, schema) in [("fmt", "ghor.fmt.v1"), ("lint", "ghor.lint.v1")] {
        let output = Command::new(&executable)
            .arg(command)
            .arg("--manifest-path")
            .arg(&project_tools_manifest)
            .arg(if command == "fmt" {
                "--check"
            } else {
                "--json"
            })
            .env("NC_GHOR_NCC", &stage1_ncc)
            .output()
            .unwrap_or_else(|error| panic!("native {command} must execute: {error}"));
        assert!(
            output.status.success(),
            "native {command} stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("native {command} JSON"));
        assert_eq!(report["schema"], schema);
        assert_eq!(report["executor"], "nc-stage1");
        assert_eq!(report["status"], "ok");
        assert_eq!(report["checked"], 1);
    }

    let native_doc = Command::new(&executable)
        .args(["doc", "--manifest-path"])
        .arg(&project_tools_manifest)
        .output()
        .expect("native stage1 doc");
    assert!(native_doc.status.success());
    let native_doc_report: serde_json::Value =
        serde_json::from_slice(&native_doc.stdout).expect("native doc JSON");
    assert_eq!(native_doc_report["schema"], "ghor.doc.v1");
    assert_eq!(native_doc_report["executor"], "nc-stage1");
    assert_eq!(native_doc_report["source_count"], 1);
    let inventory = project_tools_root.join("target/doc/inventory.json");
    let inventory_report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&inventory).expect("native documentation inventory"))
            .expect("native inventory JSON");
    assert_eq!(inventory_report["schema"], "ghor.doc.inventory.v1");
    assert_eq!(
        inventory_report["sources"][0]["sha256"],
        sha256(&std::fs::read(project_tools_root.join("src/main.n")).expect("scaffold source"))
    );

    let dry_clean = Command::new(&executable)
        .args(["clean", "--manifest-path"])
        .arg(&project_tools_manifest)
        .arg("--dry-run")
        .output()
        .expect("native stage1 clean dry run");
    assert!(dry_clean.status.success());
    assert!(project_tools_root.join("target").is_dir());
    let clean = Command::new(&executable)
        .args(["clean", "--manifest-path"])
        .arg(&project_tools_manifest)
        .output()
        .expect("native stage1 clean");
    assert!(clean.status.success());
    assert!(!project_tools_root.join("target").exists());

    let init_root = out_dir.join("project-tools-initialized");
    std::fs::create_dir_all(&init_root).expect("native init root");
    let native_init = Command::new(&launcher)
        .args(["init", "--path"])
        .arg(&init_root)
        .arg("--lib")
        .arg("--json")
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("discovered native stage1 init");
    assert!(
        native_init.status.success(),
        "native init stdout={} stderr={}",
        String::from_utf8_lossy(&native_init.stdout),
        String::from_utf8_lossy(&native_init.stderr)
    );
    let native_init_report: serde_json::Value =
        serde_json::from_slice(&native_init.stdout).expect("native init JSON");
    assert_eq!(native_init_report["executor"], "nc-stage1");
    assert_eq!(native_init_report["kind"], "lib");
    assert!(init_root.join("src/main.n").is_file());

    let missing_installed_ncc = out_dir.join("missing-installed-ncc");
    let native_doctor = Command::new(&launcher)
        .args(["doctor", "--json"])
        .env("NC_GHOR_NCC", &stage1_ncc)
        .env("GHOR_INSTALLED_NCC", &missing_installed_ncc)
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("discovered native stage1 doctor");
    assert!(
        native_doctor.status.success(),
        "native doctor stdout={} stderr={}",
        String::from_utf8_lossy(&native_doctor.stdout),
        String::from_utf8_lossy(&native_doctor.stderr)
    );
    let native_doctor_report: serde_json::Value =
        serde_json::from_slice(&native_doctor.stdout).expect("native doctor JSON");
    assert_eq!(native_doctor_report["schema"], "ghor.doctor.v1");
    assert_eq!(native_doctor_report["executor"], "nc-stage1");
    assert_eq!(native_doctor_report["status"], "closed");
    assert_eq!(native_doctor_report["machine"]["status"], "ok");
    assert_eq!(native_doctor_report["native_doctor"]["status"], "ok");
    assert_eq!(native_doctor_report["basal"]["status"], "ok");

    for (command, schema, expected_status) in [
        ("toolchain", "ghor.toolchain.v1", None),
        ("verify-toolchain", "ghor.toolchain.verify.v1", Some("ok")),
        (
            "sync-toolchain",
            "ghor.toolchain.sync.v1",
            Some("suggested"),
        ),
    ] {
        let output = Command::new(&launcher)
            .arg(command)
            .arg("--manifest-path")
            .arg(&project_tools_manifest)
            .arg("--json")
            .env("NC_GHOR_NCC", &stage1_ncc)
            .env("GHOR_INSTALLED_NCC", &missing_installed_ncc)
            .env_remove("NC_GHOR_STAGE1")
            .env_remove("NC_GHOR_FORCE_STAGE0")
            .output()
            .unwrap_or_else(|error| panic!("native {command} must execute: {error}"));
        assert!(
            output.status.success(),
            "native {command} stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("native {command} JSON"));
        assert_eq!(report["schema"], schema);
        assert_eq!(report["executor"], "nc-stage1");
        if let Some(status) = expected_status {
            assert_eq!(report["status"], status);
        }
    }

    let native_verify = Command::new(&launcher)
        .args(["verify", "--manifest-path"])
        .arg(&project_tools_manifest)
        .arg("--json")
        .env("NC_GHOR_NCC", &stage1_ncc)
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("discovered native verification");
    assert!(native_verify.status.success());
    let native_verify_report: serde_json::Value =
        serde_json::from_slice(&native_verify.stdout).expect("native verify JSON");
    assert_eq!(native_verify_report["executor"], "nc-stage1");
    assert_eq!(native_verify_report["status"], "ok");

    for (command, schema, expected_status) in [
        ("evidence", "ghor.evidence.v1", "ok"),
        ("seal", "ghor.seal.result.v1", "sealed"),
        ("conformance", "ghor.conformance.v1", "closed"),
        ("release-check", "ghor.release-check.v1", "ok"),
    ] {
        let output = Command::new(&launcher)
            .arg(command)
            .arg("--manifest-path")
            .arg(&project_tools_manifest)
            .arg("--json")
            .env("NC_GHOR_NCC", &stage1_ncc)
            .env("GHOR_INSTALLED_NCC", &missing_installed_ncc)
            .env_remove("NC_GHOR_STAGE1")
            .env_remove("NC_GHOR_FORCE_STAGE0")
            .output()
            .unwrap_or_else(|error| panic!("native {command} must execute: {error}"));
        assert!(
            output.status.success(),
            "native {command} stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("native {command} JSON"));
        assert_eq!(report["schema"], schema);
        assert_eq!(report["executor"], "nc-stage1");
        assert_eq!(report["status"], expected_status);
    }
    assert!(project_tools_root.join("target/seal.json").is_file());

    let beta_plan = Command::new(&launcher)
        .args(["public-beta", "--root"])
        .arg(&workspace)
        .args(["--plan", "--json"])
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("native public beta plan");
    assert!(beta_plan.status.success());
    let beta_report: serde_json::Value =
        serde_json::from_slice(&beta_plan.stdout).expect("native public beta plan JSON");
    assert_eq!(beta_report["schema"], "ghor.public-beta.plan.v1");
    assert_eq!(beta_report["executor"], "nc-stage1");
    assert_eq!(beta_report["steps"].as_array().map(Vec::len), Some(5));

    let smoke_bin = out_dir.join("release-smoke-bin");
    std::fs::create_dir_all(&smoke_bin).expect("native release smoke bin");
    for name in ["ghor", "ncc", "nclsp", "nfmt", "nlint"] {
        std::fs::copy(
            &launcher,
            smoke_bin.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)),
        )
        .expect("native release smoke binary");
    }
    let install_smoke = Command::new(&launcher)
        .args(["install-smoke", "--bin-dir"])
        .arg(&smoke_bin)
        .arg("--json")
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("native install smoke");
    assert!(install_smoke.status.success());
    let install_report: serde_json::Value =
        serde_json::from_slice(&install_smoke.stdout).expect("native install smoke JSON");
    assert_eq!(install_report["schema"], "ghor.release_install_smoke.v1");
    assert_eq!(install_report["executor"], "nc-stage1");
    assert_eq!(install_report["status"], "ok");
    assert_eq!(install_report["binaries"].as_array().map(Vec::len), Some(5));

    for (command, schema) in [
        ("audit", "ghor.audit.v1"),
        ("sbom", "ghor.sbom.v1"),
        ("license", "ghor.license.v1"),
        ("capability-audit", "ghor.capability-audit.v1"),
    ] {
        let output = Command::new(&launcher)
            .arg(command)
            .arg("--manifest-path")
            .arg(&project_tools_manifest)
            .arg("--json")
            .env_remove("NC_GHOR_STAGE1")
            .env_remove("NC_GHOR_FORCE_STAGE0")
            .output()
            .unwrap_or_else(|error| panic!("native {command} must execute: {error}"));
        assert!(
            output.status.success(),
            "native {command} stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("native {command} JSON"));
        assert_eq!(report["schema"], schema);
        assert_eq!(report["executor"], "nc-stage1");
    }

    let supply_chain = Command::new(&launcher)
        .args(["supply-chain", "verify", "--manifest-path"])
        .arg(&project_tools_manifest)
        .arg("--json")
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("native supply-chain verify");
    assert!(supply_chain.status.success());
    let supply_report: serde_json::Value =
        serde_json::from_slice(&supply_chain.stdout).expect("native supply-chain JSON");
    assert_eq!(supply_report["schema"], "ghor.audit.v1");
    assert_eq!(supply_report["executor"], "nc-stage1");
    assert_eq!(supply_report["status"], "ok");

    for (arguments, schema) in [
        (vec!["target", "list"], "ghor.target.matrix.v1"),
        (vec!["profile"], "ghor.profile.v1"),
        (vec!["features"], "ghor.features.v1"),
        (vec!["graph"], "ghor.graph.v1"),
        (vec!["bin"], "ghor.bin.v1"),
        (vec!["report", "commands"], "ghor.command-contracts.v1"),
    ] {
        let mut command = Command::new(&launcher);
        command
            .args(arguments)
            .arg("--manifest-path")
            .arg(&project_tools_manifest);
        command
            .arg("--json")
            .env_remove("NC_GHOR_STAGE1")
            .env_remove("NC_GHOR_FORCE_STAGE0");
        let output = command.output().expect("native project inspection command");
        assert!(
            output.status.success(),
            "native project inspection stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("native project inspection JSON");
        assert_eq!(report["schema"], schema);
        if schema == "ghor.command-contracts.v1" {
            assert_eq!(report["command_count"], 84);
            assert_eq!(report["stage1_native_e2e_count"], 80);
            assert_eq!(report["stage1_native_partial_count"], 0);
            assert_eq!(report["stage1_pending_count"], 0);
        }
    }

    for (arguments, schema) in [
        (
            vec!["cross", "--target", native_host_target()],
            "ghor.cross.v1",
        ),
        (vec!["native-deps"], "ghor.native_deps.v1"),
        (vec!["ffi-manifest"], "ghor.ffi_manifest.v1"),
        (vec!["link"], "ghor.link.v1"),
    ] {
        let output = Command::new(&launcher)
            .args(arguments)
            .arg("--manifest-path")
            .arg(&project_tools_manifest)
            .arg("--json")
            .env("NC_GHOR_NCC", &stage1_ncc)
            .env_remove("NC_GHOR_STAGE1")
            .env_remove("NC_GHOR_FORCE_STAGE0")
            .output()
            .expect("native build-boundary command");
        assert!(
            output.status.success(),
            "native build-boundary stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("native build-boundary JSON");
        assert_eq!(report["schema"], schema);
        assert_eq!(report["executor"], "nc-stage1");
    }

    let artifact_path = out_dir.join("phase22-artifact.json");
    let artifact_output = Command::new(&launcher)
        .arg("artifact")
        .arg("--manifest-path")
        .arg(&project_tools_manifest)
        .arg("--out")
        .arg(&artifact_path)
        .arg("--json")
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("native artifact command");
    assert!(artifact_output.status.success());
    let artifact_report: serde_json::Value =
        serde_json::from_slice(&artifact_output.stdout).expect("native artifact result JSON");
    assert_eq!(artifact_report["schema"], "ghor.artifact.result.v1");
    assert_eq!(artifact_report["executor"], "nc-stage1");
    assert_eq!(artifact_report["boundary_status"], "closed");
    assert!(artifact_path.is_file());
    let retained_artifact: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&artifact_path).expect("retained phase22 artifact"))
            .expect("retained phase22 artifact JSON");
    assert_eq!(retained_artifact["status"], "recorded");
    assert_eq!(
        retained_artifact["native_boundary_hash"]
            .as_str()
            .map(str::len),
        Some(64)
    );
    assert_eq!(
        retained_artifact["ffi_manifest_hash"]
            .as_str()
            .map(str::len),
        Some(64)
    );

    let boundary_project = out_dir.join("phase22-boundary-project");
    std::fs::create_dir_all(boundary_project.join("src")).expect("boundary source directory");
    std::fs::create_dir_all(boundary_project.join("native")).expect("boundary native directory");
    let native_artifact = boundary_project.join("native/libphase22.a");
    std::fs::write(&native_artifact, b"ghor phase22 native artifact")
        .expect("boundary native artifact");
    let native_checksum = sha256(&std::fs::read(&native_artifact).expect("native bytes"));
    let boundary_manifest = boundary_project.join("ghor.toml");
    std::fs::write(
        &boundary_manifest,
        format!(
            "[package]\nname = \"phase22_boundary\"\nversion = \"1.0.0\"\n\n[target.default]\ntriple = \"{}\"\n\n[native-dependencies.phase22]\nlanguage = \"nC\"\nabi = \"nC\"\nkind = \"library\"\nlink_paths = [\"native\"]\nlibs = [\"phase22\"]\nartifact = \"native/libphase22.a\"\nchecksum = \"sha256:{native_checksum}\"\nlink_mode = \"static\"\n",
            native_host_target()
        ),
    )
    .expect("boundary manifest");
    std::fs::write(
        boundary_project.join("src/main.n"),
        format!(
            "use foreign {{ rust }};\nphase22Boundary {{\n    foreign:\n        rust symbol \"phase22_add\" {{\n            abi: C;\n            library: \"{}\";\n            artifact: \"{}\";\n            linkage: static;\n            checksum: \"sha256:{native_checksum}\";\n            signature: fn(int, int) -> int;\n            layout: repr(C);\n            ownership: borrowed;\n            panicBoundary: noUnwind;\n            sideEffects: none;\n        }}\n    fa:\n        return true;\n}}\n",
            native_artifact.to_string_lossy().replace('\\', "/"),
            native_artifact.to_string_lossy().replace('\\', "/")
        ),
    )
    .expect("boundary FFI source");

    let native_output = Command::new(&launcher)
        .arg("native-deps")
        .arg("--manifest-path")
        .arg(&boundary_manifest)
        .arg("--json")
        .output()
        .expect("native dependency validation");
    assert!(native_output.status.success());
    let native_report: serde_json::Value =
        serde_json::from_slice(&native_output.stdout).expect("native dependency JSON");
    assert_eq!(native_report["executor"], "nc-stage1");
    assert_eq!(native_report["native_dependencies"][0]["name"], "phase22");
    assert_eq!(
        native_report["native_dependencies"][0]["link_args"]["lib_flags"][0],
        "-lphase22"
    );

    let ffi_output = Command::new(&launcher)
        .arg("ffi-manifest")
        .arg("--manifest-path")
        .arg(&boundary_manifest)
        .arg("--json")
        .env("NC_GHOR_NCC", &stage1_ncc)
        .output()
        .expect("native FFI aggregation");
    assert!(
        ffi_output.status.success(),
        "native FFI stdout={} stderr={}",
        String::from_utf8_lossy(&ffi_output.stdout),
        String::from_utf8_lossy(&ffi_output.stderr)
    );
    let ffi_report: serde_json::Value =
        serde_json::from_slice(&ffi_output.stdout).expect("native FFI JSON");
    assert_eq!(ffi_report["executor"], "nc-stage1");
    assert_eq!(ffi_report["compiler_ffi_entry_count"], 1);
    assert_eq!(
        ffi_report["compiler_ffi_link_manifests"][0]["entries"][0]["symbol"],
        "phase22_add"
    );

    let unsafe_manifest = out_dir.join("security-missing-checksum.toml");
    std::fs::write(
        &unsafe_manifest,
        "[package]\nname = \"security_missing_checksum\"\nversion = \"1.0.0\"\nlicense = \"MIT\"\n\n[dependencies]\nremote = \"1.0.0\"\n",
    )
    .expect("security negative manifest");
    let rejected = Command::new(&executable)
        .args(["audit", "--manifest-path"])
        .arg(&unsafe_manifest)
        .arg("--json")
        .output()
        .expect("native security negative audit");
    assert_eq!(rejected.status.code(), Some(1));
    let rejected_report: serde_json::Value =
        serde_json::from_slice(&rejected.stdout).expect("native rejected audit JSON");
    assert_eq!(rejected_report["status"], "error");
    assert_eq!(rejected_report["summary"]["checksum_errors"], 1);
    assert_eq!(
        rejected_report["diagnostics"][0]["code"],
        "E_GHOR_CHECKSUM_001"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let stale_ncc = out_dir.join("stale-installed-ncc");
        std::fs::write(
            &stale_ncc,
            "#!/usr/bin/env sh\nprintf '{\"status\":\"basal_lowering_deferred_machine\",\"dependencies\":{\"machine\":\"blocked_by_machine\"}}\\n'\n",
        )
        .expect("native stale ncc");
        let mut permissions = std::fs::metadata(&stale_ncc)
            .expect("native stale ncc metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&stale_ncc, permissions).expect("native stale ncc chmod");
        let stale_doctor = Command::new(&executable)
            .args(["doctor", "--json"])
            .env("NC_GHOR_NCC", &stage1_ncc)
            .env("GHOR_INSTALLED_NCC", &stale_ncc)
            .output()
            .expect("native stale toolchain doctor");
        assert!(stale_doctor.status.success());
        let report: serde_json::Value =
            serde_json::from_slice(&stale_doctor.stdout).expect("native stale doctor JSON");
        assert_eq!(report["status"], "warning");
        assert!(report["diagnostics"]
            .as_array()
            .expect("native stale diagnostics")
            .iter()
            .any(|diagnostic| diagnostic["code"] == "E_GHOR_TOOLCHAIN_STALE_001"));
    }

    let retained_workspace_action = Command::new(&launcher)
        .arg("workspace")
        .arg("unsupported")
        .arg("--manifest-path")
        .arg(&app_manifest)
        .env_remove("NC_GHOR_STAGE1")
        .env_remove("NC_GHOR_FORCE_STAGE0")
        .output()
        .expect("retain unimplemented workspace action in stage0");
    assert!(!retained_workspace_action.status.success());
    assert!(
        String::from_utf8_lossy(&retained_workspace_action.stderr).contains("E_GHOR_WORKSPACE_001")
    );
    let _ = std::fs::remove_dir_all(out_dir);
}
