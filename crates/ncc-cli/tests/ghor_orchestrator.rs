use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn ghor_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ghor")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ghor must be set")
}

fn temp_dir(name: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("ncc_ghor_{name}_{}_{}", std::process::id(), ts))
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

fn run_ghor(args: &[&str]) -> Output {
    Command::new(ghor_bin())
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ghor command must execute")
}

fn run_ghor_env(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(ghor_bin());
    command.args(args).env("NC_NO_COLOR", "1");
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("ghor command must execute")
}

fn run_ghor_color_forced(args: &[&str]) -> Output {
    Command::new(ghor_bin())
        .args(args)
        .env_remove("NC_NO_COLOR")
        .env_remove("NO_COLOR")
        .env("GHOR_COLOR", "always")
        .output()
        .expect("ghor command must execute")
}

fn run_ncc_ghor(args: &[&str]) -> Output {
    let mut all = vec!["ghor"];
    all.extend_from_slice(args);
    Command::new(ncc_bin())
        .args(all)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc ghor command must execute")
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

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn n_literal_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn cargo_index_path(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    let chars = lower.chars().collect::<Vec<_>>();
    match chars.len() {
        0 => "1/_".to_string(),
        1 => format!("1/{lower}"),
        2 => format!("2/{lower}"),
        3 => format!("3/{}/{}", chars[0], lower),
        _ => format!(
            "{}{}/{}{}/{}",
            chars[0], chars[1], chars[2], chars[3], lower
        ),
    }
}

fn assert_success(out: Output, label: &str) -> String {
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        out.status.success(),
        "{label}\nstdout={stdout}\nstderr={stderr}"
    );
    stdout
}

fn assert_failure_contains(out: Output, label: &str, needle: &str) {
    let stdout = normalize(&out.stdout);
    let stderr = normalize(&out.stderr);
    assert!(
        !out.status.success(),
        "{label} should fail\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains(needle) || stderr.contains(needle),
        "{label} must contain {needle:?}\nstdout={stdout}\nstderr={stderr}"
    );
}

fn populate_install_smoke_bin_dir(bin_dir: &Path, missing: &[&str]) {
    fs::create_dir_all(bin_dir).expect("bin dir");
    for name in ["ghor", "ncc", "nclsp", "nfmt", "nlint"] {
        if missing.contains(&name) {
            continue;
        }
        let file_name = format!("{name}{}", std::env::consts::EXE_SUFFIX);
        fs::copy(ghor_bin(), bin_dir.join(file_name)).expect("copy smoke binary");
    }
}

fn tar_list(archive: &Path) -> String {
    assert_success(
        Command::new("tar")
            .arg("-tzf")
            .arg(archive)
            .output()
            .expect("tar list"),
        "tar -tzf",
    )
}

fn create_project(name: &str) -> PathBuf {
    let dir = temp_dir(name);
    let stdout = assert_success(
        run_ghor(&["new", name, "--out", dir.to_str().expect("utf8 dir")]),
        "ghor new",
    );
    assert!(stdout.contains("created nC project"));
    dir
}

fn insert_package_fields(raw: &str, fields: &str) -> String {
    raw.replace(
        "authors = [\"Aristote\"]\n",
        &format!("authors = [\"Aristote\"]\n{fields}"),
    )
}

fn registry_index_package_checksum(registry: &Path, package: &str) -> String {
    let raw = fs::read_to_string(registry.join("index.json")).expect("registry index");
    let value: serde_json::Value = serde_json::from_str(&raw).expect("registry json");
    value["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .find(|entry| entry["name"].as_str() == Some(package))
        .and_then(|entry| entry["checksum"].as_str())
        .expect("checksum")
        .to_string()
}

fn registry_index_package_checksum_version(
    registry: &Path,
    package: &str,
    version: &str,
) -> String {
    let raw = fs::read_to_string(registry.join("index.json")).expect("registry index");
    let value: serde_json::Value = serde_json::from_str(&raw).expect("registry json");
    value["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .find(|entry| {
            entry["name"].as_str() == Some(package) && entry["version"].as_str() == Some(version)
        })
        .and_then(|entry| entry["checksum"].as_str())
        .expect("checksum")
        .to_string()
}

fn publish_project_version(registry: &Path, name: &str, version: &str, features: &str) -> PathBuf {
    let package = create_project(&format!("{name}_{version}_fixture").replace('.', "_"));
    let manifest = package.join("ghor.toml");
    let mut raw = fs::read_to_string(&manifest).expect("manifest");
    raw = raw.replace(
        &format!(
            "name = \"{}\"",
            format!("{name}_{version}_fixture").replace('.', "_")
        ),
        &format!("name = \"{name}\""),
    );
    raw = raw.replace("version = \"0.1.0\"", &format!("version = \"{version}\""));
    if !features.trim().is_empty() {
        raw.push_str("\n[features]\n");
        raw.push_str(features);
        raw.push('\n');
    }
    fs::write(&manifest, raw).expect("manifest write");
    assert_success(
        run_ghor(&[
            "publish",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--json",
        ]),
        "publish version fixture",
    );
    manifest
}

fn create_ffi_manifest_project(name: &str, symbol: &str) -> (PathBuf, String) {
    let dir = create_project(name);
    fs::create_dir_all(dir.join("native")).expect("native dir");
    let artifact = dir.join(format!("native/lib{symbol}.a"));
    fs::write(
        &artifact,
        format!("ghor compiler ffi artifact for {symbol}"),
    )
    .expect("native artifact");
    let checksum = sha256_hex(&fs::read(&artifact).expect("artifact bytes"));
    fs::write(
        dir.join("src/main.n"),
        format!(
            r#"
use foreign {{ c }};
ffiManifestRegistry :: runtime {{
    foreign:
        c symbol "{symbol}" {{
            abi: C;
            header: "ffi_manifest.h";
            library: "{}";
            artifact: "{}";
            linkage: static;
            checksum: "sha256:{checksum}";
            signature: fn(int, int) -> int;
            layout: repr(C);
            ownership: borrowed;
            sideEffects: none;
        }}
    fa:
        return true;
}}
"#,
            n_literal_path(&artifact),
            n_literal_path(&artifact)
        ),
    )
    .expect("source write");
    (dir, checksum)
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn cargo_available() -> bool {
    Command::new("cargo")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn command_available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn write_consumer_manifest(dir: &Path, package: &str, checksum: &str) -> PathBuf {
    fs::create_dir_all(dir.join("src")).expect("consumer src");
    fs::write(dir.join("src/main.n"), "fn main() -> i32 { return 0; }\n").expect("consumer src");
    let manifest = dir.join("ghor.toml");
    fs::write(
        &manifest,
        format!(
            "[package]\nname = \"consumer_{package}\"\nversion = \"0.1.0\"\nkind = \"bin\"\n\n[target.default]\ntriple = \"x86_64-unknown-linux-gnu\"\n\n[languages]\nprimary = \"nC\"\nadapters = []\n\n[dependencies]\n{package} = {{ version = \"0.1.0\", checksum = \"{checksum}\" }}\n"
        ),
    )
    .expect("consumer manifest");
    manifest
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn basal_registry_package_checksum(registry: &Path, package: &str) -> String {
    let raw = fs::read_to_string(registry.join("index.json")).expect("basal registry index");
    let value: Value = serde_json::from_str(&raw).expect("basal registry json");
    value["packages"]
        .as_array()
        .expect("basal packages")
        .iter()
        .find(|entry| {
            entry["name"].as_str() == Some(package)
                || entry["safe_name"].as_str() == Some(package)
                || entry["root_id"].as_str() == Some(package)
        })
        .and_then(|entry| entry["checksum"].as_str())
        .expect("basal package checksum")
        .to_string()
}

fn update_basal_index_entry<F>(registry: &Path, package: &str, mut update: F)
where
    F: FnMut(&mut Value),
{
    let path = registry.join("index.json");
    let mut value: Value =
        serde_json::from_slice(&fs::read(&path).expect("basal registry index bytes"))
            .expect("basal registry index json");
    for entry in value["packages"].as_array_mut().expect("basal packages") {
        if entry["name"].as_str() == Some(package)
            || entry["safe_name"].as_str() == Some(package)
            || entry["root_id"].as_str() == Some(package)
        {
            update(entry);
        }
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&value).expect("basal registry index"),
    )
    .expect("basal registry index write");
}

fn publish_basal_registry_fixture(registry: &Path) -> Value {
    let source = fixture_path("examples/basal/ffi/foreign_c_contract.nb");
    let publish = assert_success(
        run_ncc(&[
            "basal",
            "publish",
            source.to_str().expect("basal source"),
            "--registry",
            registry.to_str().expect("basal registry"),
            "--json",
        ]),
        "ncc basal publish fixture",
    );
    serde_json::from_str(&publish).expect("basal publish json")
}

fn write_basal_consumer_manifest(
    dir: &Path,
    root_id: &str,
    version: &str,
    registry: &Path,
    checksum: &str,
) -> PathBuf {
    fs::create_dir_all(dir.join("src")).expect("basal consumer src");
    fs::write(dir.join("src/main.n"), "fn main() -> i32 { return 0; }\n")
        .expect("basal consumer src");
    let manifest = dir.join("ghor.toml");
    fs::write(
        &manifest,
        format!(
            "[package]\n\
name = \"ghor_basal_consumer\"\n\
version = \"0.1.0\"\n\
kind = \"bin\"\n\n\
[target.default]\n\
triple = \"x86_64-unknown-linux-gnu\"\n\n\
[languages]\n\
primary = \"nC\"\n\
adapters = []\n\n\
[basal-dependencies]\n\
nc_root_ffi_foreign_c = {{ version = \"{version}\", registry = \"{}\", package = \"{root_id}\", checksum = \"{checksum}\" }}\n",
            registry.display()
        ),
    )
    .expect("basal consumer manifest");
    manifest
}

fn create_workspace_fixture(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = temp_dir(name);
    let app = root.join("crates/app");
    let tool = root.join("crates/tool");
    let shared = root.join("crates/shared_dep");
    let skipped = root.join("crates/skip");
    for (dir, root_name) in [
        (&app, "workspace_app"),
        (&tool, "workspace_tool"),
        (&shared, "shared_dep"),
        (&skipped, "workspace_skip"),
    ] {
        fs::create_dir_all(dir.join("src")).expect("workspace src");
        fs::write(
            dir.join("src/main.n"),
            format!(
                "{root_name} {{\n    to main {{\n        return true;\n    }}\n\n    fa:\n        return true;\n}}\n"
            ),
        )
        .expect("workspace source");
    }
    fs::write(
        root.join("ghor.toml"),
        "[workspace]\n\
members = [\"crates/*\"]\n\
exclude = [\"crates/skip\"]\n\
default-members = [\"crates/app\"]\n\
resolver = \"3\"\n\n\
[workspace.package]\n\
version = \"1.2.3\"\n\
edition = \"2026\"\n\
license = \"MIT\"\n\n\
[workspace.dependencies]\n\
shared_dep = { path = \"crates/shared_dep\", version = \"0.5.0\", features = [\"std\"] }\n\n\
[workspace.lints.rust]\n\
unsafe_code = \"deny\"\n",
    )
    .expect("workspace root manifest");
    fs::write(
        app.join("ghor.toml"),
        "[package]\n\
name = \"workspace_app\"\n\
version.workspace = true\n\
edition.workspace = true\n\
license.workspace = true\n\
kind = \"bin\"\n\n\
[target.default]\n\
triple = \"x86_64-unknown-linux-gnu\"\n\n\
[languages]\n\
primary = \"nC\"\n\
adapters = []\n\n\
[dependencies]\n\
shared_dep.workspace = true\n\n\
[lints]\n\
workspace = true\n",
    )
    .expect("workspace app manifest");
    fs::write(
        tool.join("ghor.toml"),
        "[package]\nname = \"workspace_tool\"\nversion = \"0.1.0\"\nkind = \"bin\"\n\n[target.default]\ntriple = \"x86_64-unknown-linux-gnu\"\n\n[languages]\nprimary = \"nC\"\nadapters = []\n",
    )
    .expect("workspace tool manifest");
    fs::write(
        shared.join("ghor.toml"),
        "[package]\nname = \"shared_dep\"\nversion = \"0.5.0\"\nkind = \"lib\"\n\n[target.default]\ntriple = \"x86_64-unknown-linux-gnu\"\n\n[languages]\nprimary = \"nC\"\nadapters = []\n",
    )
    .expect("workspace shared manifest");
    fs::write(
        skipped.join("ghor.toml"),
        "[package]\nname = \"workspace_skip\"\nversion = \"0.1.0\"\nkind = \"bin\"\n\n[target.default]\ntriple = \"x86_64-unknown-linux-gnu\"\n\n[languages]\nprimary = \"nC\"\nadapters = []\n",
    )
    .expect("workspace skipped manifest");
    (root.clone(), root.join("ghor.toml"), app.join("ghor.toml"))
}

fn start_registry_http_server(root: PathBuf) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind registry server");
    let addr = listener.local_addr().expect("registry addr");
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            handle_registry_connection(stream, &root);
        }
    });
    format!("http://{}", addr)
}

fn handle_registry_connection(mut stream: TcpStream, root: &Path) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut data = Vec::new();
    let mut buf = [0_u8; 4096];
    loop {
        let read = match stream.read(&mut buf) {
            Ok(read) => read,
            Err(_) => return,
        };
        if read == 0 {
            break;
        }
        data.extend_from_slice(&buf[..read]);
        if let Some(header_end) = find_header_end(&data) {
            let headers = String::from_utf8_lossy(&data[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if data.len() >= header_end + 4 + content_length {
                break;
            }
        }
    }
    let Some(header_end) = find_header_end(&data) else {
        return;
    };
    let request = String::from_utf8_lossy(&data[..header_end]);
    let mut parts = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    let method = parts.next().unwrap_or_default();
    let raw_path = parts.next().unwrap_or("/");
    let relative = raw_path
        .split('?')
        .next()
        .unwrap_or("/")
        .trim_start_matches('/')
        .trim();
    if relative.contains("..") {
        write_http_response(&mut stream, 400, b"bad path");
        return;
    }
    let body = &data[header_end + 4..];
    if method == "PUT" && relative == "api/v1/crates/new" {
        match handle_cargo_publish_api(root, body) {
            Ok(()) => write_json_response(
                &mut stream,
                json!({"warnings": {"invalid_categories": [], "invalid_badges": [], "other": []}}),
            ),
            Err(err) => write_json_error(&mut stream, 400, &err),
        }
        return;
    }
    if let Some((package, version)) = parse_yank_endpoint(relative) {
        if method == "DELETE" {
            match set_test_registry_yanked(root, package, version, true) {
                Ok(()) => write_json_response(&mut stream, json!({"ok": true})),
                Err(err) => write_json_error(&mut stream, 404, &err),
            }
            return;
        }
    }
    if let Some(package) = parse_owner_endpoint(relative) {
        match method {
            "GET" => {
                let owners = test_registry_owners(root, package);
                write_json_response(
                    &mut stream,
                    json!({"users": owners.into_iter().enumerate().map(|(id, login)| json!({"id": id as u32 + 1, "login": login, "name": null})).collect::<Vec<_>>()}),
                );
                return;
            }
            "PUT" | "DELETE" => {
                let add = method == "PUT";
                match update_test_registry_owners(root, package, body, add) {
                    Ok(()) => write_json_response(
                        &mut stream,
                        json!({"ok": true, "msg": "owners updated"}),
                    ),
                    Err(err) => write_json_error(&mut stream, 400, &err),
                }
                return;
            }
            _ => {}
        }
    }
    let path = root.join(if relative.is_empty() {
        "index.json"
    } else {
        relative
    });
    match method {
        "GET" => match fs::read(&path) {
            Ok(bytes) => {
                let etag = format!("\"{}\"", sha256_hex(&bytes));
                let status = if request_header(&request, "if-none-match").as_deref()
                    == Some(etag.as_str())
                {
                    append_registry_request_log(root, method, relative, 304);
                    write_http_response_with_headers(
                        &mut stream,
                        304,
                        &[],
                        &[
                            ("ETag", etag.as_str()),
                            ("Last-Modified", "Wed, 01 Jan 2025 00:00:00 GMT"),
                        ],
                    );
                    return;
                } else {
                    200
                };
                append_registry_request_log(root, method, relative, status);
                write_http_response_with_headers(
                    &mut stream,
                    status,
                    &bytes,
                    &[
                        ("ETag", etag.as_str()),
                        ("Last-Modified", "Wed, 01 Jan 2025 00:00:00 GMT"),
                    ],
                )
            }
            Err(_) => write_http_response(&mut stream, 404, b"missing"),
        },
        "PUT" => {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            match fs::write(&path, body) {
                Ok(()) => write_http_response(&mut stream, 200, b"ok"),
                Err(_) => write_http_response(&mut stream, 500, b"write error"),
            }
        }
        _ => write_http_response(&mut stream, 405, b"method"),
    }
}

fn find_header_end(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|window| window == b"\r\n\r\n")
}

fn write_http_response(stream: &mut TcpStream, status: u16, body: &[u8]) {
    write_http_response_with_headers(stream, status, body, &[])
}

fn write_http_response_with_headers(
    stream: &mut TcpStream,
    status: u16,
    body: &[u8],
    headers: &[(&str, &str)],
) {
    let reason = match status {
        200 => "OK",
        304 => "Not Modified",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    let _ = write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (name, value) in headers {
        let _ = write!(stream, "{name}: {value}\r\n");
    }
    let _ = write!(stream, "\r\n");
    let _ = stream.write_all(body);
}

fn write_json_response(stream: &mut TcpStream, value: Value) {
    let body = serde_json::to_vec(&value).expect("json response");
    write_http_response_with_headers(stream, 200, &body, &[("Content-Type", "application/json")]);
}

fn write_json_error(stream: &mut TcpStream, status: u16, detail: &str) {
    let body = serde_json::to_vec(&json!({"errors": [{"detail": detail}]})).expect("json error");
    write_http_response_with_headers(
        stream,
        status,
        &body,
        &[("Content-Type", "application/json")],
    );
}

fn request_header(request: &str, name: &str) -> Option<String> {
    request.lines().find_map(|line| {
        let (header, value) = line.split_once(':')?;
        header
            .trim()
            .eq_ignore_ascii_case(name)
            .then(|| value.trim().to_string())
    })
}

fn append_registry_request_log(root: &Path, method: &str, relative: &str, status: u16) {
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("requests.log"))
        .and_then(|mut file| writeln!(file, "{method} {relative} {status}"));
}

fn parse_yank_endpoint(relative: &str) -> Option<(&str, &str)> {
    let parts = relative.split('/').collect::<Vec<_>>();
    (parts.len() == 6
        && parts[0] == "api"
        && parts[1] == "v1"
        && parts[2] == "crates"
        && parts[5] == "yank")
        .then(|| (parts[3], parts[4]))
}

fn parse_owner_endpoint(relative: &str) -> Option<&str> {
    let parts = relative.split('/').collect::<Vec<_>>();
    (parts.len() == 5
        && parts[0] == "api"
        && parts[1] == "v1"
        && parts[2] == "crates"
        && parts[4] == "owners")
        .then(|| parts[3])
}

fn handle_cargo_publish_api(root: &Path, body: &[u8]) -> Result<(), String> {
    if body.len() < 8 {
        return Err("publish body too short".to_string());
    }
    let metadata_len = u32::from_le_bytes(body[0..4].try_into().expect("len")) as usize;
    if body.len() < 4 + metadata_len + 4 {
        return Err("publish metadata length out of bounds".to_string());
    }
    let metadata_start = 4;
    let metadata_end = metadata_start + metadata_len;
    let metadata: Value = serde_json::from_slice(&body[metadata_start..metadata_end])
        .map_err(|err| format!("invalid metadata json: {err}"))?;
    let crate_len = u32::from_le_bytes(
        body[metadata_end..metadata_end + 4]
            .try_into()
            .expect("crate len"),
    ) as usize;
    let crate_start = metadata_end + 4;
    if body.len() < crate_start + crate_len {
        return Err("crate length out of bounds".to_string());
    }
    let crate_bytes = &body[crate_start..crate_start + crate_len];
    let name = metadata
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing package name".to_string())?;
    let version = metadata
        .get("vers")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing package version".to_string())?;
    let archive_rel = format!("packages/{name}/{version}/{name}-{version}.crate");
    let manifest_rel = format!("packages/{name}/{version}/manifest.json");
    let archive_path = root.join(&archive_rel);
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(&archive_path, crate_bytes).map_err(|err| err.to_string())?;
    fs::write(
        root.join(&manifest_rel),
        serde_json::to_vec_pretty(&metadata).expect("metadata"),
    )
    .map_err(|err| err.to_string())?;
    let checksum = sha256_hex(crate_bytes);
    let deps = metadata
        .get("deps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|dep| {
            json!({
                "name": dep.get("name").cloned().unwrap_or(Value::Null),
                "req": dep.get("version_req").cloned().unwrap_or(Value::String("*".to_string())),
                "features": dep.get("features").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
                "optional": dep.get("optional").cloned().unwrap_or(Value::Bool(false)),
                "default_features": dep.get("default_features").cloned().unwrap_or(Value::Bool(true)),
                "target": dep.get("target").cloned().unwrap_or(Value::Null),
                "kind": dep.get("kind").cloned().unwrap_or(Value::String("normal".to_string())),
                "registry": dep.get("registry").cloned().unwrap_or(Value::Null),
                "package": Value::Null
            })
        })
        .collect::<Vec<_>>();
    let entry = json!({
        "name": name,
        "vers": version,
        "deps": deps,
        "cksum": checksum,
        "features": metadata.get("features").cloned().unwrap_or_else(|| json!({})),
        "yanked": false,
        "rust_version": metadata.get("rust_version").cloned().unwrap_or(Value::Null),
        "v": 2,
        "ghor_archive": archive_rel,
        "ghor_manifest": manifest_rel,
        "ghor_owners": []
    });
    upsert_test_cargo_entry(root, name, version, entry)
}

fn upsert_test_cargo_entry(
    root: &Path,
    name: &str,
    version: &str,
    entry: Value,
) -> Result<(), String> {
    let package_path = root.join(cargo_index_path(name));
    if let Some(parent) = package_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let mut entries = if package_path.exists() {
        fs::read_to_string(&package_path)
            .map_err(|err| err.to_string())?
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str::<Value>(line).map_err(|err| err.to_string()))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    entries.retain(|existing| existing.get("vers").and_then(Value::as_str) != Some(version));
    entries.push(entry.clone());
    let mut raw = String::new();
    for entry in &entries {
        raw.push_str(&serde_json::to_string(entry).map_err(|err| err.to_string())?);
        raw.push('\n');
    }
    fs::write(package_path, raw).map_err(|err| err.to_string())?;

    let mut compat = if root.join("index.json").exists() {
        serde_json::from_slice::<Value>(
            &fs::read(root.join("index.json")).map_err(|err| err.to_string())?,
        )
        .map_err(|err| err.to_string())?
    } else {
        json!({"schema": "ghor.registry.v1", "packages": []})
    };
    let packages = compat["packages"]
        .as_array_mut()
        .ok_or_else(|| "compat packages not array".to_string())?;
    packages.retain(|existing| {
        existing.get("name").and_then(Value::as_str) != Some(name)
            || existing.get("version").and_then(Value::as_str) != Some(version)
    });
    packages.push(json!({
        "schema": "ghor.registry.package.v1",
        "name": name,
        "version": version,
        "yanked": entry.get("yanked").cloned().unwrap_or(Value::Bool(false)),
        "owners": entry.get("ghor_owners").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "archive": entry.get("ghor_archive").cloned().unwrap_or(Value::Null),
        "manifest": entry.get("ghor_manifest").cloned().unwrap_or(Value::Null),
        "checksum": entry.get("cksum").cloned().unwrap_or(Value::Null),
        "source": "http-api"
    }));
    fs::write(
        root.join("index.json"),
        serde_json::to_vec_pretty(&compat).expect("compat json"),
    )
    .map_err(|err| err.to_string())
}

fn set_test_registry_yanked(
    root: &Path,
    package: &str,
    version: &str,
    yanked: bool,
) -> Result<(), String> {
    update_test_cargo_entry(root, package, version, |entry| {
        entry["yanked"] = Value::Bool(yanked);
    })
}

fn update_test_registry_owners(
    root: &Path,
    package: &str,
    body: &[u8],
    add: bool,
) -> Result<(), String> {
    let request: Value = serde_json::from_slice(body).map_err(|err| err.to_string())?;
    let requested = request
        .get("users")
        .and_then(Value::as_array)
        .ok_or_else(|| "owners request missing users".to_string())?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    update_test_all_cargo_entries(root, package, |entry| {
        if entry.get("ghor_owners").and_then(Value::as_array).is_none() {
            entry["ghor_owners"] = Value::Array(Vec::new());
        }
        let owners = entry["ghor_owners"].as_array_mut().expect("owners array");
        for owner in &requested {
            if add {
                if !owners
                    .iter()
                    .any(|value| value.as_str() == Some(owner.as_str()))
                {
                    owners.push(Value::String(owner.clone()));
                }
            } else {
                owners.retain(|value| value.as_str() != Some(owner.as_str()));
            }
        }
    })
}

fn test_registry_owners(root: &Path, package: &str) -> Vec<String> {
    let path = root.join(cargo_index_path(package));
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut owners = Vec::new();
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(entry) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if let Some(values) = entry.get("ghor_owners").and_then(Value::as_array) {
            for owner in values.iter().filter_map(Value::as_str) {
                if !owners.contains(&owner.to_string()) {
                    owners.push(owner.to_string());
                }
            }
        }
    }
    owners
}

fn update_test_cargo_entry<F>(
    root: &Path,
    package: &str,
    version: &str,
    mut update: F,
) -> Result<(), String>
where
    F: FnMut(&mut Value),
{
    update_test_all_cargo_entries(root, package, |entry| {
        if entry.get("vers").and_then(Value::as_str) == Some(version) {
            update(entry);
        }
    })
}

fn update_test_all_cargo_entries<F>(root: &Path, package: &str, mut update: F) -> Result<(), String>
where
    F: FnMut(&mut Value),
{
    let path = root.join(cargo_index_path(package));
    let raw = fs::read_to_string(&path).map_err(|err| err.to_string())?;
    let mut entries = raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<Value>(line).map_err(|err| err.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    for entry in &mut entries {
        update(entry);
    }
    let mut output = String::new();
    for entry in &entries {
        output.push_str(&serde_json::to_string(entry).map_err(|err| err.to_string())?);
        output.push('\n');
    }
    fs::write(&path, output).map_err(|err| err.to_string())?;
    if root.join("index.json").exists() {
        let mut compat = serde_json::from_slice::<Value>(
            &fs::read(root.join("index.json")).map_err(|err| err.to_string())?,
        )
        .map_err(|err| err.to_string())?;
        if let Some(packages) = compat["packages"].as_array_mut() {
            for package_entry in packages {
                if package_entry.get("name").and_then(Value::as_str) == Some(package) {
                    if let Some(version) = package_entry.get("version").and_then(Value::as_str) {
                        if let Some(index_entry) = entries.iter().find(|entry| {
                            entry.get("vers").and_then(Value::as_str) == Some(version)
                        }) {
                            package_entry["yanked"] = index_entry
                                .get("yanked")
                                .cloned()
                                .unwrap_or(Value::Bool(false));
                            package_entry["owners"] = index_entry
                                .get("ghor_owners")
                                .cloned()
                                .unwrap_or_else(|| Value::Array(Vec::new()));
                        }
                    }
                }
            }
        }
        fs::write(
            root.join("index.json"),
            serde_json::to_vec_pretty(&compat).expect("compat"),
        )
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

#[test]
fn ghor_help_ok() {
    let stdout = assert_success(run_ghor(&["--help"]), "ghor --help");
    assert!(stdout.contains("nC package manager and build orchestrator"));
    assert!(stdout.contains("Usage: ghor [OPTIONS] <COMMAND>"));
    assert!(stdout.contains("Options:"));
    assert!(stdout.contains("Commands:"));
    assert!(stdout.contains("build, b"));
    assert!(stdout.contains("check, c"));
    assert!(stdout.contains("machine"));
    assert!(stdout.contains("See 'ghor help <command>'"));

    let stdout = assert_success(run_ncc_ghor(&["--help"]), "ncc ghor --help");
    assert!(stdout.contains("nC package manager and build orchestrator"));
    assert!(stdout.contains("Usage: ghor [OPTIONS] <COMMAND>"));
}

#[test]
fn ghor_command_help_ok() {
    let stdout = assert_success(run_ghor(&["help", "new"]), "ghor help new");
    assert!(stdout.contains("Create a new nC package"));
    assert!(stdout.contains("Usage: ghor new <NAME> [OPTIONS]"));
    assert!(stdout.contains("Examples:"));

    let stdout = assert_success(run_ghor(&["new", "--help"]), "ghor new --help");
    assert!(stdout.contains("Usage: ghor new <NAME> [OPTIONS]"));
    assert!(!stdout.contains("[E_GHOR_NEW_001]"));
    assert!(!stdout.contains("created nC project"));

    let stdout = assert_success(run_ghor(&["new", "help"]), "ghor new help");
    assert!(stdout.contains("Usage: ghor new <NAME> [OPTIONS]"));
    assert!(!stdout.contains("[E_GHOR_NEW_001]"));
    assert!(!stdout.contains("created nC project"));

    let stdout = assert_success(run_ghor(&["help", "lock"]), "ghor help lock");
    assert!(stdout.contains("Cargo.lock-style TOML primary lockfile"));
    assert!(stdout.contains("ghor.lock.meta.json"));
}

#[test]
fn ghor_list_and_short_version_ok() {
    let stdout = assert_success(run_ghor(&["--list"]), "ghor --list");
    assert!(stdout.lines().any(|line| line == "build"));
    assert!(stdout.lines().any(|line| line == "conformance"));
    assert!(stdout.lines().any(|line| line == "machine"));

    let stdout = assert_success(run_ghor(&["-V"]), "ghor -V");
    assert!(stdout.contains("ghor 0.1.0"));
}

#[test]
fn ghor_command_contract_report_accounts_for_every_command() {
    let output = assert_success(
        run_ghor(&["report", "commands", "--json"]),
        "ghor report commands",
    );
    let report: Value = serde_json::from_str(&output).expect("command contracts json");
    assert_eq!(report["schema"], "ghor.command-contracts.v1");
    assert_eq!(report["command_count"], 77);
    assert_eq!(report["native_e2e_count"], 4);
    assert_eq!(report["stage1_native_e2e_count"], 57);
    assert_eq!(report["stage1_native_partial_count"], 16);
    assert_eq!(report["stage1_pending_count"], 0);
    let commands = report["commands"].as_array().expect("commands");
    for (name, route) in [
        ("run", "nc-stage1-native-machine-execution"),
        ("bench", "nc-stage1-native-machine-benchmark"),
    ] {
        let command = commands
            .iter()
            .find(|command| command["name"] == name)
            .unwrap_or_else(|| panic!("missing command contract {name}"));
        assert_eq!(command["route"], route);
        assert_eq!(command["current_executor"], "nc-stage1-when-available");
        assert_eq!(command["direct_machine_artifact"], true);
        assert_eq!(command["status"], "native_e2e");
    }
    for (name, route) in [
        ("bootstrap", "nc-stage1-native-bootstrap-authority"),
        ("lang", "nc-stage1-native-language-forge"),
        ("cargo-exec", "nc-stage1-native-cargo-execution"),
        ("build-script", "nc-stage1-native-build-script-execution"),
        ("proc-macro", "nc-stage1-native-proc-macro-execution"),
        ("interop", "nc-stage1-native-foreign-interop"),
        ("external-compat", "nc-stage1-native-external-compatibility"),
    ] {
        let command = commands
            .iter()
            .find(|command| command["name"] == name)
            .unwrap_or_else(|| panic!("missing command contract {name}"));
        assert_eq!(command["route"], route);
        assert_eq!(command["current_executor"], "nc-stage1-when-available");
        assert_eq!(command["status"], "stage1_native_e2e");
    }
    let machine = commands
        .iter()
        .find(|command| command["name"] == "machine")
        .expect("missing machine command contract");
    assert_eq!(
        machine["route"],
        "nc-stage1-native-ncc-subsystem-passthrough"
    );
    assert_eq!(machine["current_executor"], "nc-stage1-when-available");
    assert_eq!(machine["direct_machine_artifact"], false);
    assert_eq!(machine["status"], "stage1_native_e2e");
    for (name, route) in [
        ("check", "nc-stage1-native-ncc-parser-sema"),
        ("build", "nc-stage1-native-ncc-machine-artifact"),
        ("test", "nc-stage1-native-ncc-test-execution"),
    ] {
        let command = commands
            .iter()
            .find(|command| command["name"] == name)
            .unwrap_or_else(|| panic!("missing command contract {name}"));
        assert_eq!(command["route"], route);
        assert_eq!(command["current_executor"], "nc-stage1-when-available");
    }
    let manifest = commands
        .iter()
        .find(|command| command["name"] == "manifest")
        .expect("manifest command contract");
    assert_eq!(manifest["route"], "nc-stage1-native-manifest");
    assert_eq!(manifest["current_executor"], "nc-stage1-when-available");
    assert_eq!(manifest["status"], "stage1_native_e2e");
    let metadata = commands
        .iter()
        .find(|command| command["name"] == "metadata")
        .expect("metadata command contract");
    assert_eq!(metadata["route"], "nc-stage1-native-metadata");
    assert_eq!(metadata["status"], "stage1_native_e2e");
    for (name, route) in [
        ("add", "nc-stage1-native-validated-manifest-add"),
        ("remove", "nc-stage1-native-validated-manifest-remove"),
        ("package", "nc-stage1-native-reproducible-package"),
        ("install", "nc-stage1-native-material-install-transaction"),
        (
            "uninstall",
            "nc-stage1-native-owned-material-uninstall-transaction",
        ),
    ] {
        let command = commands
            .iter()
            .find(|command| command["name"] == name)
            .unwrap_or_else(|| panic!("missing command contract {name}"));
        assert_eq!(command["route"], route);
        assert_eq!(command["current_executor"], "nc-stage1-when-available");
        assert_eq!(command["status"], "stage1_native_e2e");
    }
    let install = commands
        .iter()
        .find(|command| command["name"] == "install")
        .expect("install command contract");
    assert_eq!(install["direct_machine_artifact"], true);
    for (name, route) in [
        ("cdn", "nc-stage1-native-content-addressed-cdn"),
        ("governance", "nc-stage1-native-registry-governance"),
        ("moderation", "nc-stage1-native-registry-moderation"),
    ] {
        let command = commands
            .iter()
            .find(|command| command["name"] == name)
            .unwrap_or_else(|| panic!("missing command contract {name}"));
        assert_eq!(command["route"], route);
        assert_eq!(command["current_executor"], "nc-stage1-when-available");
        assert_eq!(command["direct_machine_artifact"], false);
        assert_eq!(command["status"], "stage1_native_e2e");
    }
    for (name, route) in [
        ("new", "nc-stage1-native-project-scaffold"),
        ("init", "nc-stage1-native-project-scaffold"),
        ("clean", "nc-stage1-native-target-clean"),
        ("fmt", "nc-stage1-native-nfmt-orchestration"),
        ("lint", "nc-stage1-native-nlint-orchestration"),
        ("doc", "nc-stage1-native-documentation-inventory"),
        ("doctor", "nc-stage1-native-host-doctor"),
        ("toolchain", "nc-stage1-native-toolchain-inspection"),
        (
            "verify-toolchain",
            "nc-stage1-native-toolchain-verification",
        ),
        ("sync-toolchain", "nc-stage1-native-toolchain-sync-plan"),
        ("verify", "nc-stage1-native-verification"),
        ("evidence", "nc-stage1-native-evidence"),
        ("seal", "nc-stage1-native-seal"),
        ("conformance", "nc-stage1-native-conformance"),
        ("public-beta", "nc-stage1-native-public-beta"),
        ("release-check", "nc-stage1-native-release-check"),
        ("release-hardening", "nc-stage1-native-release-hardening"),
        ("install-smoke", "nc-stage1-native-install-smoke"),
        ("audit", "nc-stage1-native-security-audit"),
        ("sbom", "nc-stage1-native-complete-sbom"),
        ("license", "nc-stage1-native-license-report"),
        ("supply-chain", "nc-stage1-native-supply-chain-verify"),
        ("capability-audit", "nc-stage1-native-capability-audit"),
    ] {
        let command = commands
            .iter()
            .find(|command| command["name"] == name)
            .unwrap_or_else(|| panic!("missing command contract {name}"));
        assert_eq!(command["route"], route);
        assert_eq!(command["current_executor"], "nc-stage1-when-available");
        assert_eq!(command["direct_machine_artifact"], false);
        assert_eq!(command["status"], "stage1_native_e2e");
    }
    for (name, route) in [
        (
            "resolve",
            "nc-stage1-native-filesystem-http-sparse-git-source-replacement-basal-resolver",
        ),
        (
            "lock",
            "nc-stage1-native-filesystem-http-sparse-git-source-replacement-basal-lock",
        ),
        (
            "verify-lock",
            "nc-stage1-native-filesystem-http-sparse-git-source-replacement-basal-lock-verify",
        ),
        (
            "fetch",
            "nc-stage1-native-filesystem-http-sparse-git-source-replacement-basal-fetch",
        ),
        (
            "vendor",
            "nc-stage1-native-filesystem-http-sparse-git-source-replacement-basal-vendor",
        ),
        ("update", "nc-stage1-native-selective-precise-update"),
        ("tree", "nc-stage1-native-resolved-dependency-tree"),
    ] {
        let command = commands
            .iter()
            .find(|command| command["name"] == name)
            .unwrap_or_else(|| panic!("missing command contract {name}"));
        assert_eq!(command["route"], route);
        assert_eq!(command["current_executor"], "nc-stage1-when-available");
        assert_eq!(command["status"], "stage1_native_partial");
    }
    let workspace = commands
        .iter()
        .find(|command| command["name"] == "workspace")
        .expect("workspace command contract");
    assert_eq!(workspace["route"], "nc-stage1-native-workspace-metadata");
    assert_eq!(workspace["status"], "stage1_native_partial");
    for (name, route) in [
        ("search", "nc-stage1-native-registry-search"),
        ("info", "nc-stage1-native-registry-info"),
        ("login", "nc-stage1-native-private-registry-credentials"),
        ("logout", "nc-stage1-native-private-registry-credentials"),
        ("registry", "nc-stage1-native-registry-client"),
    ] {
        let command = commands
            .iter()
            .find(|command| command["name"] == name)
            .unwrap_or_else(|| panic!("missing command contract {name}"));
        assert_eq!(command["route"], route);
        assert_eq!(command["current_executor"], "nc-stage1-when-available");
        assert_eq!(command["status"], "stage1_native_partial");
    }
}

#[test]
fn ghor_conformance_includes_security_hardening_ok() {
    let stdout = assert_success(
        run_ghor(&["conformance", "--json"]),
        "ghor conformance --json",
    );
    let value: Value = serde_json::from_str(&stdout).expect("conformance json");
    assert_eq!(value["status"], "closed");
    assert_eq!(value["security_hardening"]["status"], "closed");
    assert_eq!(value["security_hardening"]["token_redaction"], "closed");
    assert_eq!(
        value["security_hardening"]["corrupt_package_rejection"],
        "closed"
    );
    assert_eq!(value["final_user_docs"]["status"], "closed");
    assert_eq!(
        value["final_user_docs"]["manual"],
        "docs/spec/ghor/ghor_user_manual.md"
    );
    assert_eq!(value["c_cpp_rust_interop"]["status"], "closed");
    assert_eq!(value["c_cpp_rust_interop"]["abi"], "C");
}

#[test]
fn ghor_c_cpp_rust_interop_smoke_ok() {
    let c_available =
        command_available("cc") || command_available("gcc") || command_available("clang");
    let cpp_available =
        command_available("c++") || command_available("g++") || command_available("clang++");
    if !cargo_available() || !c_available || !cpp_available {
        return;
    }
    let out_dir = temp_dir("c_cpp_rust_interop");
    let stdout = assert_success(
        run_ghor(&[
            "interop",
            "--out",
            out_dir.to_str().expect("interop out"),
            "--json",
        ]),
        "ghor interop",
    );
    let value: Value = serde_json::from_str(&stdout).expect("interop json");
    assert_eq!(value["schema"], "ghor.interop.c_cpp_rust.v1");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["bridge"]["status"], "closed");
    assert_eq!(value["smoke"]["status"], "ok");
    assert!(out_dir.join("ghor_interop_manifest.json").is_file());
    assert!(out_dir.join("include/ghor_interop.h").is_file());
    let steps = value["smoke"]["steps"].as_array().expect("smoke steps");
    for label in [
        "c_compile",
        "cpp_to_c_link",
        "cpp_to_c_run",
        "rust_staticlib_build",
        "c_to_rust_link",
        "c_to_rust_run",
    ] {
        assert!(
            steps
                .iter()
                .any(|step| step["label"] == label && step["status"] == "ok"),
            "interop step {label} must be ok: {steps:?}"
        );
    }
    assert!(value["artifacts"]
        .as_array()
        .expect("artifacts")
        .iter()
        .any(|artifact| artifact["kind"] == "rust_staticlib"));
}

#[test]
fn ghor_final_user_manual_is_complete_ok() {
    let manual_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("docs/spec/ghor/ghor_user_manual.md");
    let manual = fs::read_to_string(manual_path).expect("ghor user manual");
    let word_count = manual.split_whitespace().count();
    assert!(
        word_count >= 25_000,
        "manual word count {word_count} is below 25000"
    );
    for required in [
        "Manual final de usuario de ghor",
        "Tabla global de comandos tipo Cargo",
        "Registry setup local",
        "Publicar paquetes",
        "Instalar y desinstalar",
        "Vendor, fetch y offline",
        "Diferencias con Cargo",
    ] {
        assert!(manual.contains(required), "manual missing {required}");
    }
}

#[test]
fn ghor_help_lilac_color_ok() {
    let stdout = assert_success(run_ghor_color_forced(&["--help"]), "ghor --help color");
    assert!(stdout.starts_with("\x1b[38;2;203;166;247m"));
    assert!(stdout.contains("nC package manager and build orchestrator"));
    assert!(stdout.contains("\x1b[0m"));
}

#[test]
fn ghor_version_ok() {
    let stdout = assert_success(run_ghor(&["--version"]), "ghor --version");
    assert!(stdout.contains("ghor 0.1.0"));
}

#[test]
fn ghor_metadata_ok() {
    let stdout = assert_success(run_ncc_ghor(&["metadata", "--json"]), "ncc ghor metadata");
    assert!(stdout.contains("\"schema\": \"ghor.metadata.v1\""));
    assert!(stdout.contains("\"machine_origin\": true"));
}

#[test]
fn ghor_native_deps_manifest_metadata_lock_and_link_ok() {
    let dir = create_project("native_deps_ok");
    fs::create_dir_all(dir.join("native")).expect("native dir");
    let artifact = dir.join("native/libnc_math.a");
    fs::write(&artifact, b"native archive smoke").expect("native artifact");
    let checksum = sha256_hex(&fs::read(&artifact).expect("artifact bytes"));
    let manifest = dir.join("ghor.toml");
    let mut raw = fs::read_to_string(&manifest).expect("manifest");
    raw = raw.replace(
        "[native-dependencies]\n",
        &format!(
            "[native-dependencies]\nlibm = {{ language = \"c\", abi = \"c\", kind = \"system\", headers = [\"math.h\"], libs = [\"m\"], symbols = [\"sqrt\"], link_mode = \"dynamic\", system = true }}\nnc_math = {{ language = \"cpp\", abi = \"itanium-cxxabi\", kind = \"library\", headers = [\"nc_math.hpp\"], include_paths = [\"native/include\"], link_paths = [\"native\"], libs = [\"nc_math\"], symbols = [\"nc_math_add\"], artifact = \"native/libnc_math.a\", checksum = \"sha256:{checksum}\", link_mode = \"static\" }}\n"
        ),
    );
    fs::write(&manifest, raw).expect("manifest write");
    let manifest_arg = manifest.to_str().expect("manifest utf8");

    let native = assert_success(
        run_ghor(&["native-deps", "--manifest-path", manifest_arg, "--json"]),
        "ghor native-deps",
    );
    assert!(native.contains("\"schema\": \"ghor.native_deps.v1\""));
    assert!(native.contains("\"name\": \"libm\""));
    assert!(native.contains("\"name\": \"nc_math\""));
    assert!(native.contains("\"lib_flags\""));
    assert!(native.contains("-lm"));
    assert!(native.contains("-lnc_math"));

    let manifest_json = assert_success(
        run_ghor(&["manifest", "--manifest-path", manifest_arg, "--json"]),
        "ghor manifest native deps",
    );
    assert!(manifest_json.contains("\"native_dependencies\""));
    assert!(manifest_json.contains("\"artifact\": \"native/libnc_math.a\""));

    let metadata = assert_success(
        run_ghor(&["metadata", "--manifest-path", manifest_arg, "--json"]),
        "ghor metadata native deps",
    );
    assert!(metadata.contains("\"native_dependencies\""));
    assert!(metadata.contains("\"ghor\""));

    let link = assert_success(
        run_ghor(&["link", "--manifest-path", manifest_arg, "--json"]),
        "ghor link native deps",
    );
    assert!(link.contains("\"native_link_args\""));
    assert!(link.contains("-Lnative"));

    assert_success(
        run_ghor(&["lock", "--manifest-path", manifest_arg, "--json"]),
        "ghor lock native deps",
    );
    let lock_meta =
        fs::read_to_string(dir.join("ghor.lock.meta.json")).expect("lock metadata sidecar");
    assert!(lock_meta.contains("\"native_dependencies\""));
    assert!(lock_meta.contains("\"nc_math\""));
}

#[test]
fn ghor_native_deps_bad_checksum_fails() {
    let dir = create_project("native_deps_bad_checksum");
    fs::create_dir_all(dir.join("native")).expect("native dir");
    fs::write(dir.join("native/libbad.a"), b"bad checksum smoke").expect("native artifact");
    let manifest = dir.join("ghor.toml");
    let mut raw = fs::read_to_string(&manifest).expect("manifest");
    raw = raw.replace(
        "[native-dependencies]\n",
        "[native-dependencies]\nbad = { language = \"c\", abi = \"c\", kind = \"library\", libs = [\"bad\"], artifact = \"native/libbad.a\", checksum = \"sha256:0000\", link_mode = \"static\" }\n",
    );
    fs::write(&manifest, raw).expect("manifest write");
    assert_failure_contains(
        run_ghor(&[
            "native-deps",
            "--manifest-path",
            manifest.to_str().expect("manifest utf8"),
            "--json",
        ]),
        "ghor native-deps bad checksum",
        "E_GHOR_NATIVE_CHECKSUM_001",
    );
}

#[test]
fn ghor_ffi_manifest_integrates_compiler_source_artifacts_ok() {
    let dir = create_project("ffi_manifest_source_ok");
    fs::create_dir_all(dir.join("native")).expect("native dir");
    let artifact = dir.join("native/libffi_manifest_source.a");
    fs::write(&artifact, b"ghor compiler ffi source artifact").expect("native artifact");
    let checksum = sha256_hex(&fs::read(&artifact).expect("artifact bytes"));
    fs::write(
        dir.join("src/main.n"),
        format!(
            r#"
use foreign {{ c }};
ffiManifestSource :: runtime {{
    foreign:
        c symbol "ffi_manifest_add" {{
            abi: C;
            header: "ffi_manifest.h";
            library: "{}";
            artifact: "{}";
            linkage: static;
            checksum: "sha256:{checksum}";
            signature: fn(int, int) -> int;
            layout: repr(C);
            ownership: borrowed;
            sideEffects: none;
        }}
    fa:
        return true;
}}
"#,
            n_literal_path(&artifact),
            n_literal_path(&artifact)
        ),
    )
    .expect("source write");
    let manifest = dir.join("ghor.toml");
    let manifest_arg = manifest.to_str().expect("manifest utf8");

    let ffi_manifest = assert_success(
        run_ghor(&["ffi-manifest", "--manifest-path", manifest_arg, "--json"]),
        "ghor ffi-manifest",
    );
    assert!(ffi_manifest.contains("\"schema\": \"ghor.ffi_manifest.v1\""));
    assert!(ffi_manifest.contains("\"status\": \"closed\""));
    assert!(ffi_manifest.contains("\"compiler_ffi_entry_count\": 1"));
    assert!(ffi_manifest.contains("ffi_manifest_add"));
    assert!(ffi_manifest.contains(&checksum));

    let metadata = assert_success(
        run_ghor(&["metadata", "--manifest-path", manifest_arg, "--json"]),
        "ghor metadata ffi manifest",
    );
    assert!(metadata.contains("\"ffi_link_manifest\""));
    assert!(metadata.contains("ffi_manifest_add"));

    let link = assert_success(
        run_ghor(&["link", "--manifest-path", manifest_arg, "--json"]),
        "ghor link ffi manifest",
    );
    assert!(link.contains("\"compiler_ffi_link_manifest\""));
    assert!(link.contains("ffi_manifest_add"));

    assert_success(
        run_ghor(&["lock", "--manifest-path", manifest_arg, "--json"]),
        "ghor lock ffi manifest",
    );
    let lock_meta =
        fs::read_to_string(dir.join("ghor.lock.meta.json")).expect("lock metadata sidecar");
    assert!(lock_meta.contains("\"ffi_link_manifest\""));
    assert!(lock_meta.contains("ffi_manifest_add"));

    let package_dir = dir.join("pkg");
    assert_success(
        run_ghor(&[
            "package",
            "--manifest-path",
            manifest_arg,
            "--out",
            package_dir.to_str().expect("package dir"),
            "--no-verify",
            "--json",
        ]),
        "ghor package ffi manifest",
    );
    let package_manifest =
        fs::read_to_string(package_dir.join("manifest.json")).expect("package manifest");
    assert!(package_manifest.contains("\"ffi_link_manifest\""));
    assert!(package_manifest.contains("ffi_manifest_add"));
}

#[test]
fn ghor_ffi_manifest_bad_checksum_fails() {
    let dir = create_project("ffi_manifest_bad_checksum");
    fs::create_dir_all(dir.join("native")).expect("native dir");
    let artifact = dir.join("native/libffi_manifest_bad.a");
    fs::write(&artifact, b"ghor compiler ffi bad checksum artifact").expect("native artifact");
    fs::write(
        dir.join("src/main.n"),
        format!(
            r#"
use foreign {{ c }};
ffiManifestBad :: runtime {{
    foreign:
        c symbol "ffi_manifest_bad" {{
            abi: C;
            header: "ffi_manifest.h";
            library: "{}";
            artifact: "{}";
            linkage: static;
            checksum: "sha256:0000000000000000000000000000000000000000000000000000000000000000";
            signature: fn(int, int) -> int;
            layout: repr(C);
            ownership: borrowed;
            sideEffects: none;
        }}
    fa:
        return true;
}}
"#,
            n_literal_path(&artifact),
            n_literal_path(&artifact)
        ),
    )
    .expect("source write");
    let manifest = dir.join("ghor.toml");
    assert_failure_contains(
        run_ghor(&[
            "ffi-manifest",
            "--manifest-path",
            manifest.to_str().expect("manifest utf8"),
            "--json",
        ]),
        "ghor ffi-manifest bad checksum",
        "E_FFI_CHECKSUM_001",
    );
}

#[test]
fn ghor_registry_ffi_manifest_publish_fetch_install_ok() {
    let (dir, checksum) =
        create_ffi_manifest_project("ffi_registry_manifest_ok", "ffi_registry_add");
    let registry = temp_dir("ffi_registry_manifest_ok_registry");
    let manifest = dir.join("ghor.toml");
    let manifest_arg = manifest.to_str().expect("manifest utf8");

    let publish = assert_success(
        run_ghor(&[
            "publish",
            "--manifest-path",
            manifest_arg,
            "--registry",
            registry.to_str().expect("registry utf8"),
            "--no-verify",
            "--json",
        ]),
        "ghor publish ffi registry manifest",
    );
    assert!(publish.contains("\"package_manifest_hash\""));
    assert!(publish.contains("\"ffi_link_manifest_hash\""));

    let package_manifest_path =
        registry.join("packages/ffi_registry_manifest_ok/0.1.0/manifest.json");
    let package_manifest =
        fs::read_to_string(&package_manifest_path).expect("registry package manifest");
    assert!(package_manifest.contains("\"ffi_link_manifest\""));
    assert!(package_manifest.contains("ffi_registry_add"));
    assert!(package_manifest.contains(&checksum));

    let cargo_index =
        fs::read_to_string(registry.join(cargo_index_path("ffi_registry_manifest_ok")))
            .expect("cargo index entry");
    assert!(cargo_index.contains("ghor_package_manifest_hash"));
    assert!(cargo_index.contains("ghor_ffi_link_manifest_hash"));

    let checksum = registry_index_package_checksum(&registry, "ffi_registry_manifest_ok");
    let consumer = temp_dir("ffi_registry_manifest_ok_consumer");
    let consumer_manifest =
        write_consumer_manifest(&consumer, "ffi_registry_manifest_ok", &checksum);

    let fetch = assert_success(
        run_ghor(&[
            "fetch",
            "--manifest-path",
            consumer_manifest.to_str().expect("consumer manifest utf8"),
            "--registry",
            registry.to_str().expect("registry utf8"),
            "--json",
        ]),
        "ghor fetch ffi registry manifest",
    );
    assert!(fetch.contains("\"package_manifest_validation\""));
    assert!(fetch.contains("\"ffi_link_manifest_hash\""));
    assert!(fetch.contains("\"status\": \"validated\""));

    let install_root = temp_dir("ffi_registry_manifest_ok_install_root");
    let install = assert_success(
        run_ghor(&[
            "install",
            "ffi_registry_manifest_ok",
            "--registry",
            registry.to_str().expect("registry utf8"),
            "--root",
            install_root.to_str().expect("install root utf8"),
            "--version",
            "0.1.0",
            "--json",
        ]),
        "ghor install ffi registry manifest",
    );
    assert!(install.contains("\"package_manifest_validation\""));
    assert!(install.contains("\"ffi_link_manifest_hash\""));
    assert!(install.contains("\"status\": \"validated\""));
}

#[test]
fn ghor_registry_corrupt_ffi_manifest_fails() {
    let (dir, _) = create_ffi_manifest_project("ffi_registry_manifest_bad", "ffi_registry_bad");
    let registry = temp_dir("ffi_registry_manifest_bad_registry");
    let manifest = dir.join("ghor.toml");
    let manifest_arg = manifest.to_str().expect("manifest utf8");
    assert_success(
        run_ghor(&[
            "publish",
            "--manifest-path",
            manifest_arg,
            "--registry",
            registry.to_str().expect("registry utf8"),
            "--no-verify",
            "--json",
        ]),
        "ghor publish ffi corrupt fixture",
    );

    let package_manifest_path =
        registry.join("packages/ffi_registry_manifest_bad/0.1.0/manifest.json");
    let mut package_manifest: Value =
        serde_json::from_slice(&fs::read(&package_manifest_path).expect("package manifest bytes"))
            .expect("package manifest json");
    package_manifest["ffi_link_manifest"]["status"] = Value::String("failed".to_string());
    package_manifest["ffi_link_manifest"]["diagnostics"] = json!([{
        "code": "E_GHOR_FFI_MANIFEST_001",
        "severity": "error",
        "message": "registry fixture corruption"
    }]);
    fs::write(
        &package_manifest_path,
        serde_json::to_vec_pretty(&package_manifest).expect("manifest json"),
    )
    .expect("corrupt package manifest write");
    let manifest_hash = sha256_hex(
        serde_json::to_string(&package_manifest)
            .expect("canonical package manifest")
            .as_bytes(),
    );
    update_test_cargo_entry(&registry, "ffi_registry_manifest_bad", "0.1.0", |entry| {
        entry["ghor_package_manifest_hash"] = Value::String(manifest_hash.clone());
    })
    .expect("index update");

    let checksum = registry_index_package_checksum(&registry, "ffi_registry_manifest_bad");
    let consumer = temp_dir("ffi_registry_manifest_bad_consumer");
    let consumer_manifest =
        write_consumer_manifest(&consumer, "ffi_registry_manifest_bad", &checksum);
    assert_failure_contains(
        run_ghor(&[
            "fetch",
            "--manifest-path",
            consumer_manifest.to_str().expect("consumer manifest utf8"),
            "--registry",
            registry.to_str().expect("registry utf8"),
            "--json",
        ]),
        "ghor fetch corrupt ffi registry manifest",
        "E_GHOR_FFI_MANIFEST_001",
    );

    let install_root = temp_dir("ffi_registry_manifest_bad_install_root");
    assert_failure_contains(
        run_ghor(&[
            "install",
            "ffi_registry_manifest_bad",
            "--registry",
            registry.to_str().expect("registry utf8"),
            "--root",
            install_root.to_str().expect("install root utf8"),
            "--version",
            "0.1.0",
            "--json",
        ]),
        "ghor install corrupt ffi registry manifest",
        "E_GHOR_FFI_MANIFEST_001",
    );
}

#[test]
fn ghor_basal_dependency_resolve_fetch_metadata_vendor_ok() {
    let registry = temp_dir("basal_dependency_registry");
    let publish = publish_basal_registry_fixture(&registry);
    assert_eq!(
        publish["schema"].as_str(),
        Some("basal.registry.publish.v1")
    );
    assert_eq!(publish["ghor_compatible"].as_bool(), Some(true));

    let checksum = basal_registry_package_checksum(&registry, "nc_root_ffi_foreign_c");
    let consumer = temp_dir("basal_dependency_consumer");
    let consumer_manifest = write_basal_consumer_manifest(
        &consumer,
        "nC.root.ffi.foreign_c",
        "1",
        &registry,
        &checksum,
    );
    let manifest_arg = consumer_manifest.to_str().expect("consumer manifest");

    let resolve = assert_success(
        run_ghor(&["resolve", "--manifest-path", manifest_arg, "--json"]),
        "ghor resolve basal dependency",
    );
    let resolve_json: Value = serde_json::from_str(&resolve).expect("resolve json");
    let basal_entry = resolve_json["resolved"]
        .as_array()
        .expect("resolved")
        .iter()
        .find(|entry| entry["status"].as_str() == Some("resolved_basal_registry"))
        .expect("resolved basal registry entry");
    assert_eq!(
        basal_entry["root_id"].as_str(),
        Some("nC.root.ffi.foreign_c")
    );
    assert_eq!(
        basal_entry["safe_name"].as_str(),
        Some("nc_root_ffi_foreign_c")
    );
    assert_eq!(basal_entry["checksum"].as_str(), Some(checksum.as_str()));
    assert_eq!(
        basal_entry["package_manifest_validation"]["package_kind"].as_str(),
        Some("basal")
    );
    assert_eq!(
        basal_entry["package_manifest_validation"]["status"].as_str(),
        Some("validated")
    );
    assert_eq!(
        basal_entry["package_manifest_validation"]["nbb"]["magic"].as_str(),
        Some("NBB0")
    );
    assert_eq!(
        basal_entry["package_manifest_validation"]["ffi"]["status"].as_str(),
        Some("validated")
    );
    assert_eq!(
        basal_entry["package_manifest_validation"]["ffi"]["compiler_ffi_entry_count"].as_u64(),
        Some(1)
    );

    let metadata = assert_success(
        run_ghor(&["metadata", "--manifest-path", manifest_arg, "--json"]),
        "ghor metadata basal dependency",
    );
    assert!(metadata.contains("\"basal_dependencies\""));
    assert!(metadata.contains("nC.root.ffi.foreign_c"));
    assert!(metadata.contains("\"basal-registry\""));

    let fetch = assert_success(
        run_ghor(&["fetch", "--manifest-path", manifest_arg, "--json"]),
        "ghor fetch basal dependency",
    );
    assert!(fetch.contains("\"fetched_basal_registry\""));
    assert!(consumer
        .join("target/ghor/basal-registry-cache/nc_root_ffi_foreign_c/1/nc_root_ffi_foreign_c-1.nbp.tar.gz")
        .exists());

    let vendor_dir = consumer.join("vendor/basal");
    let vendor = assert_success(
        run_ghor(&[
            "vendor",
            vendor_dir.to_str().expect("vendor"),
            "--versioned-dirs",
            "--manifest-path",
            manifest_arg,
            "--json",
        ]),
        "ghor vendor basal dependency",
    );
    assert!(vendor.contains("\"package_kind\": \"basal\""));
    let vendored = vendor_dir.join("nc_root_ffi_foreign_c-1");
    assert!(vendored.join(".basal-checksum.json").exists());
    assert!(vendored.join("manifest.json").exists());
    assert!(vendored.join("ffi_link_manifest.json").exists());
    assert!(vendored
        .join("artifacts/nc_root_ffi_foreign_c.nbb")
        .exists());
    let checksum_json: Value = serde_json::from_slice(
        &fs::read(vendored.join(".basal-checksum.json")).expect("basal vendor checksum"),
    )
    .expect("basal vendor checksum json");
    assert_eq!(
        checksum_json["schema"].as_str(),
        Some("ghor.basal.vendor.checksum.v1")
    );
    assert_eq!(checksum_json["package"].as_str(), Some(checksum.as_str()));
    assert_eq!(
        checksum_json["basal"]["package_manifest_validation"]["nbb"]["magic"].as_str(),
        Some("NBB0")
    );
}

#[test]
fn ghor_basal_dependency_corrupt_manifest_hash_fails() {
    let registry = temp_dir("basal_dependency_corrupt_registry");
    publish_basal_registry_fixture(&registry);
    let checksum = basal_registry_package_checksum(&registry, "nc_root_ffi_foreign_c");
    update_basal_index_entry(&registry, "nc_root_ffi_foreign_c", |entry| {
        entry["package_manifest_hash"] = Value::String("00".repeat(32));
    });

    let consumer = temp_dir("basal_dependency_corrupt_consumer");
    let consumer_manifest = write_basal_consumer_manifest(
        &consumer,
        "nC.root.ffi.foreign_c",
        "1",
        &registry,
        &checksum,
    );
    assert_failure_contains(
        run_ghor(&[
            "resolve",
            "--manifest-path",
            consumer_manifest.to_str().expect("consumer manifest"),
            "--json",
        ]),
        "ghor resolve corrupt basal dependency",
        "E_GHOR_BASAL_PACKAGE_001",
    );
}

#[test]
fn ghor_cargo_toml_real_project_metadata_matches_cargo_subset_ok() {
    if !cargo_available() {
        return;
    }
    let dir = temp_dir("cargo_real_metadata");
    fs::create_dir_all(dir.join("src")).expect("src");
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\n\
name = \"cargo_real_metadata\"\n\
version = \"0.1.0\"\n\
edition = \"2021\"\n\
license = \"MIT OR Apache-2.0\"\n\
rust-version = \"1.70\"\n\
description = \"Cargo metadata compatibility fixture\"\n\
repository = \"https://example.invalid/cargo_real_metadata\"\n\
build = \"build.rs\"\n\n\
[package.metadata.nc]\n\
mode = \"compat\"\n\n\
[lib]\n\
proc-macro = true\n\
path = \"src/lib.rs\"\n\n\
[features]\n\
default = [\"std\"]\n\
std = []\n",
    )
    .expect("Cargo.toml");
    fs::write(dir.join("src/lib.rs"), "pub fn marker() {}\n").expect("lib");
    fs::write(dir.join("build.rs"), "fn main() {}\n").expect("build");

    let cargo_out = assert_success(
        Command::new("cargo")
            .arg("metadata")
            .arg("--format-version")
            .arg("1")
            .arg("--no-deps")
            .current_dir(&dir)
            .output()
            .expect("cargo metadata"),
        "cargo metadata",
    );
    let cargo_json: Value = serde_json::from_str(&cargo_out).expect("cargo metadata json");
    let ghor_out = assert_success(
        run_ghor(&[
            "metadata",
            "--manifest-path",
            dir.join("Cargo.toml").to_str().expect("cargo manifest"),
            "--json",
        ]),
        "ghor metadata Cargo.toml",
    );
    let ghor_json: Value = serde_json::from_str(&ghor_out).expect("ghor metadata json");

    let cargo_package = &cargo_json["packages"][0];
    let ghor_package = &ghor_json["packages"][0];
    assert_eq!(ghor_package["id"], cargo_package["id"]);
    assert_eq!(
        ghor_json["workspace_members"],
        cargo_json["workspace_members"]
    );
    assert_eq!(ghor_package["name"], cargo_package["name"]);
    assert_eq!(ghor_package["version"], cargo_package["version"]);
    assert_eq!(ghor_package["edition"], cargo_package["edition"]);
    assert_eq!(ghor_package["license"], cargo_package["license"]);
    assert_eq!(ghor_package["rust_version"], cargo_package["rust_version"]);
    assert_eq!(ghor_package["metadata"]["nc"]["mode"], "compat");
    let target_kinds = ghor_package["targets"]
        .as_array()
        .expect("targets")
        .iter()
        .flat_map(|target| target["kind"].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(target_kinds.contains(&"proc-macro"));
    assert!(target_kinds.contains(&"custom-build"));
}

#[test]
fn ghor_external_compat_real_cargo_varied_projects_ok() {
    if !cargo_available() {
        return;
    }

    let simple = temp_dir("external_compat_simple");
    fs::create_dir_all(simple.join("src")).expect("simple src");
    fs::write(
        simple.join("Cargo.toml"),
        "[package]\n\
name = \"external_compat_simple\"\n\
version = \"0.1.0\"\n\
edition = \"2021\"\n\
rust-version = \"1.70\"\n\
license = \"MIT\"\n\
build = \"build.rs\"\n\n\
[features]\n\
default = [\"std\"]\n\
std = []\n\n\
[lib]\n\
path = \"src/lib.rs\"\n\n\
[[bin]]\n\
name = \"external_compat_simple\"\n\
path = \"src/main.rs\"\n",
    )
    .expect("simple manifest");
    fs::write(
        simple.join("src/lib.rs"),
        "pub fn simple() -> bool { true }\n",
    )
    .expect("lib");
    fs::write(simple.join("src/main.rs"), "fn main() {}\n").expect("main");
    fs::write(simple.join("build.rs"), "fn main() {}\n").expect("build");
    fs::create_dir_all(simple.join("tests")).expect("simple tests");
    fs::write(
        simple.join("tests/smoke.rs"),
        "#[test]\nfn smoke() { assert!(external_compat_simple::simple()); }\n",
    )
    .expect("integration test");

    let dep = temp_dir("external_compat_path_dep");
    fs::create_dir_all(dep.join("src")).expect("dep src");
    fs::write(
        dep.join("Cargo.toml"),
        "[package]\nname = \"external_compat_path_dep\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("dep manifest");
    fs::write(dep.join("src/lib.rs"), "pub fn dep() -> u32 { 1 }\n").expect("dep lib");
    let app = temp_dir("external_compat_path_app");
    fs::create_dir_all(app.join("src")).expect("app src");
    fs::write(
        app.join("Cargo.toml"),
        format!(
            "[package]\nname = \"external_compat_path_app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nexternal_compat_path_dep = {{ path = \"{}\" }}\n",
            dep.to_string_lossy().replace('\\', "/")
        ),
    )
    .expect("app manifest");
    fs::write(
        app.join("src/main.rs"),
        "fn main() { let _ = external_compat_path_dep::dep(); }\n",
    )
    .expect("app main");

    let proc_macro = temp_dir("external_compat_proc_macro");
    fs::create_dir_all(proc_macro.join("src")).expect("proc macro src");
    fs::write(
        proc_macro.join("Cargo.toml"),
        "[package]\nname = \"external_compat_proc_macro\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\nproc-macro = true\n",
    )
    .expect("proc macro manifest");
    fs::write(
        proc_macro.join("src/lib.rs"),
        "extern crate proc_macro;\nuse proc_macro::TokenStream;\n#[proc_macro]\npub fn marker(_input: TokenStream) -> TokenStream { \"fn generated() {}\".parse().unwrap() }\n",
    )
    .expect("proc macro lib");

    for manifest in [
        simple.join("Cargo.toml"),
        app.join("Cargo.toml"),
        proc_macro.join("Cargo.toml"),
    ] {
        let out = assert_success(
            run_ghor(&[
                "external-compat",
                "--manifest-path",
                manifest.to_str().expect("manifest"),
                "--json",
            ]),
            "ghor external-compat",
        );
        let value: Value = serde_json::from_str(&out).expect("external compat json");
        assert_eq!(value["status"], "ok");
        assert_eq!(
            value["comparisons"].as_array().expect("comparisons").len(),
            5
        );
        assert!(value["accepted_differences"]
            .as_array()
            .expect("accepted differences")
            .iter()
            .any(|entry| entry["surface"] == "vendor"));
        if manifest == simple.join("Cargo.toml") {
            let metadata = value["comparisons"]
                .as_array()
                .expect("comparisons")
                .iter()
                .find(|comparison| comparison["command"] == "metadata")
                .expect("metadata comparison");
            assert!(metadata["details"]["ghor_target_kinds"]
                .as_array()
                .expect("ghor target kinds")
                .iter()
                .any(|kind| kind == "test"));
        }
    }
}

#[test]
fn ghor_external_compat_requires_cargo_manifest_fail() {
    let project = create_project("external_compat_requires_cargo");
    assert_failure_contains(
        run_ghor(&[
            "external-compat",
            "--manifest-path",
            project.join("ghor.toml").to_str().expect("manifest"),
            "--json",
        ]),
        "ghor external-compat requires Cargo.toml",
        "E_GHOR_EXTERNAL_COMPAT_001",
    );
}

#[test]
fn ghor_release_install_smoke_bins_ok() {
    let bin_dir = temp_dir("release_install_smoke_ok");
    populate_install_smoke_bin_dir(&bin_dir, &[]);
    let out = assert_success(
        run_ghor(&[
            "install-smoke",
            "--bin-dir",
            bin_dir.to_str().expect("bin dir"),
            "--json",
        ]),
        "ghor install-smoke",
    );
    let value: Value = serde_json::from_str(&out).expect("install smoke json");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["binaries"].as_array().expect("binaries").len(), 5);
    for name in ["ghor", "ncc", "nclsp", "nfmt", "nlint"] {
        assert!(value["binaries"]
            .as_array()
            .expect("binaries")
            .iter()
            .any(|binary| binary["name"] == name && binary["status"] == "ok"));
    }
}

#[test]
fn ghor_release_install_smoke_missing_bin_fail() {
    let bin_dir = temp_dir("release_install_smoke_missing");
    populate_install_smoke_bin_dir(&bin_dir, &["nlint"]);
    assert_failure_contains(
        run_ghor(&[
            "install-smoke",
            "--bin-dir",
            bin_dir.to_str().expect("bin dir"),
            "--json",
        ]),
        "ghor install-smoke missing",
        "E_GHOR_INSTALL_SMOKE_001",
    );
}

#[test]
fn ghor_cargo_toml_discovery_semver_and_target_cfg_ok() {
    let registry = temp_dir("cargo_semver_registry");
    publish_project_version(&registry, "cargo_semver_dep", "0.2.4", "");
    publish_project_version(&registry, "cargo_semver_dep", "0.2.9", "");
    publish_project_version(&registry, "cargo_semver_dep", "0.3.0", "");
    publish_project_version(&registry, "cargo_target_dep", "1.0.0", "");

    let dir = temp_dir("cargo_semver_consumer");
    fs::create_dir_all(dir.join("src")).expect("src");
    fs::write(dir.join("src/lib.rs"), "pub fn marker() {}\n").expect("lib");
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\n\
name = \"cargo_semver_consumer\"\n\
version = \"0.1.0\"\n\
edition = \"2021\"\n\n\
[dependencies]\n\
cargo_semver_dep = \"0.2\"\n\n\
[target.'cfg(unix)'.dependencies]\n\
cargo_target_dep = \"1.*\"\n",
    )
    .expect("Cargo.toml");

    let resolve = assert_success(
        run_ghor(&[
            "resolve",
            "--manifest-path",
            dir.join("Cargo.toml").to_str().expect("cargo manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--target",
            "x86_64-unknown-linux-gnu",
            "--json",
        ]),
        "ghor resolve Cargo.toml semver target cfg",
    );
    let value: Value = serde_json::from_str(&resolve).expect("resolve json");
    assert_eq!(value["status"], "ok");
    let resolved = value["resolved"].as_array().expect("resolved");
    assert!(resolved.iter().any(|entry| {
        entry["name"] == "cargo_semver_dep"
            && entry["version"] == "0.2.9"
            && entry["status"] == "resolved_registry"
    }));
    assert!(resolved.iter().any(|entry| {
        entry["name"] == "cargo_target_dep"
            && entry["version"] == "1.0.0"
            && entry["target"] == "cfg(unix)"
    }));

    let windows = assert_success(
        run_ghor(&[
            "resolve",
            "--manifest-path",
            dir.join("Cargo.toml").to_str().expect("cargo manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--target",
            "x86_64-pc-windows-msvc",
            "--json",
        ]),
        "ghor resolve Cargo.toml windows target cfg",
    );
    let value: Value = serde_json::from_str(&windows).expect("resolve json");
    assert!(value["resolved"]
        .as_array()
        .expect("resolved")
        .iter()
        .any(|entry| {
            entry["name"] == "cargo_target_dep" && entry["status"] == "skipped_inactive_target"
        }));
}

#[test]
fn ghor_cargo_toml_source_replacement_uses_local_mirror_ok() {
    let registry = temp_dir("cargo_source_replacement_registry");
    publish_project_version(&registry, "source_replaced_dep", "0.1.0", "");

    let dir = temp_dir("cargo_source_replacement_consumer");
    fs::create_dir_all(dir.join("src")).expect("src");
    fs::write(dir.join("src/lib.rs"), "pub fn marker() {}\n").expect("lib");
    fs::write(
        dir.join("Cargo.toml"),
        format!(
            "[package]\n\
name = \"cargo_source_replacement_consumer\"\n\
version = \"0.1.0\"\n\
edition = \"2021\"\n\n\
[dependencies]\n\
source_replaced_dep = \"0.1\"\n\n\
[source.crates-io]\n\
replace-with = \"local-mirror\"\n\n\
[source.local-mirror]\n\
directory = \"{}\"\n",
            registry.to_string_lossy().replace('\\', "/")
        ),
    )
    .expect("Cargo.toml");

    let resolve = assert_success(
        run_ghor(&[
            "resolve",
            "--manifest-path",
            dir.join("Cargo.toml").to_str().expect("cargo manifest"),
            "--json",
        ]),
        "ghor resolve Cargo.toml source replacement",
    );
    let value: Value = serde_json::from_str(&resolve).expect("resolve json");
    assert_eq!(value["status"], "ok");
    let dep = value["resolved"]
        .as_array()
        .expect("resolved")
        .iter()
        .find(|entry| entry["name"] == "source_replaced_dep")
        .expect("source replaced dep");
    assert_eq!(dep["status"], "resolved_registry");
    assert_eq!(dep["source"]["replacement_kind"], "directory");
    assert_eq!(dep["source"]["effective"], "local-mirror");
}

#[test]
fn ghor_manifest_registry_corpus_fuzz_smoke_ok() {
    let registry = temp_dir("manifest_corpus_registry");
    publish_project_version(
        &registry,
        "corpus_dep",
        "1.0.0",
        "default = [\"derive\"]\nderive = []",
    );
    publish_project_version(&registry, "corpus_dep", "1.1.0", "default = []\nfast = []");

    for idx in 0..24 {
        let dir = temp_dir(&format!("manifest_corpus_{idx}"));
        fs::create_dir_all(dir.join("src")).expect("src");
        fs::write(dir.join("src/lib.rs"), "pub fn marker() {}\n").expect("lib");
        let req = match idx % 6 {
            0 => "^1.0",
            1 => "~1.0",
            2 => "1.*",
            3 => ">=1.0, <2.0",
            4 => "=1.1.0",
            _ => "*",
        };
        let default_features = if idx % 2 == 0 { "true" } else { "false" };
        let feature = if idx % 3 == 0 {
            ", features = [\"fast\"]"
        } else {
            ""
        };
        fs::write(
            dir.join("Cargo.toml"),
            format!(
                "[package]\n\
name = \"manifest_corpus_{idx}\"\n\
version = \"0.1.0\"\n\
edition = \"2021\"\n\n\
[dependencies]\n\
corpus_dep = {{ version = \"{req}\", default-features = {default_features}{feature} }}\n"
            ),
        )
        .expect("Cargo.toml");
        let resolve = assert_success(
            run_ghor(&[
                "resolve",
                "--manifest-path",
                dir.join("Cargo.toml").to_str().expect("cargo manifest"),
                "--registry",
                registry.to_str().expect("registry"),
                "--json",
            ]),
            "ghor corpus resolve",
        );
        let value: Value = serde_json::from_str(&resolve).expect("resolve json");
        assert_eq!(value["status"], "ok", "case {idx}: {resolve}");
        assert!(value["resolved"]
            .as_array()
            .expect("resolved")
            .iter()
            .any(|entry| {
                entry["name"] == "corpus_dep" && entry["status"] == "resolved_registry"
            }));
    }
}

#[test]
fn ghor_new_check_build_package_verify_ok() {
    let dir = create_project("hello_ghor_test");
    let manifest = dir.join("nc.toml");
    let manifest_arg = manifest.to_str().expect("utf8 manifest");
    assert_success(
        run_ghor(&["check", "--manifest", manifest_arg]),
        "ghor check",
    );
    assert_success(
        run_ghor(&["build", "--manifest", manifest_arg]),
        "ghor build",
    );
    let build_report: Value = serde_json::from_str(
        &fs::read_to_string(dir.join("target/ghor/debug/hello_ghor_test.build.json"))
            .expect("native build report"),
    )
    .expect("native build report json");
    assert_eq!(build_report["native_build"]["status"], "materialized");
    for field in ["nm", "object", "executable"] {
        let path = build_report["native_build"][field]
            .as_str()
            .unwrap_or_else(|| panic!("missing native artifact field {field}"));
        assert!(Path::new(path).is_file(), "missing {field} artifact {path}");
    }
    let run = assert_success(
        run_ghor(&["run", "--manifest", manifest_arg, "--json"]),
        "ghor native run",
    );
    assert!(run.contains("\"execution\": \"native_machine_executable\""));
    assert!(run.contains("\"exit_code\": 0"));
    fs::create_dir_all(dir.join("tests")).expect("tests dir");
    fs::write(
        dir.join("tests/native_smoke.n"),
        "ghorNativeTest {\n    to main {\n        return true;\n    }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("native test source");
    let tests = assert_success(
        run_ghor(&["test", "--manifest", manifest_arg, "--json"]),
        "ghor native test",
    );
    assert!(tests.contains("\"mode\": \"native-execution\""));

    fs::create_dir_all(dir.join("benches")).expect("benches dir");
    fs::write(
        dir.join("benches/native_bench.n"),
        "ghorNativeBench {\n    to main {\n        return true;\n    }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("native bench source");
    let bench = assert_success(
        run_ghor(&["bench", "--manifest", manifest_arg, "--json"]),
        "ghor native bench",
    );
    assert!(bench.contains("\"profile\": \"release-fast\""));
    assert!(bench.contains("\"status\": \"ok\""));
    let package_dir = dir.join("pkg");
    let stdout = assert_success(
        run_ghor(&[
            "package",
            "--manifest",
            manifest_arg,
            "--out",
            package_dir.to_str().expect("utf8 package"),
        ]),
        "ghor package",
    );
    assert!(stdout.contains("\"status\": \"packaged\""));
    assert!(package_dir.join("manifest.json").exists());
    let archive = package_dir.join("hello_ghor_test-0.1.0.crate");
    assert!(archive.exists());
    let archive_list = tar_list(&archive);
    assert!(archive_list.contains("hello_ghor_test-0.1.0/ghor.toml"));
    assert!(archive_list.contains("hello_ghor_test-0.1.0/ghor.toml.orig"));
    assert!(archive_list.contains("hello_ghor_test-0.1.0/ghor.lock.meta.json"));
    assert!(archive_list.contains("hello_ghor_test-0.1.0/.cargo_vcs_info.json"));
    assert_success(run_ghor(&["lock", "--manifest", manifest_arg]), "ghor lock");
    assert_success(
        run_ghor(&["verify", "--manifest", manifest_arg]),
        "ghor verify",
    );
}

#[test]
fn ghor_package_crate_include_exclude_metadata_verify_ok() {
    let dir = create_project("crate_format_pkg");
    let manifest = dir.join("ghor.toml");
    let raw = fs::read_to_string(&manifest).expect("manifest");
    let raw = insert_package_fields(
        &raw,
        "description = \"Cargo-compatible ghor crate fixture\"\n\
repository = \"https://example.com/nc/ghor\"\n\
homepage = \"https://example.com/ghor\"\n\
documentation = \"https://example.com/ghor/docs\"\n\
readme = \"README.md\"\n\
keywords = [\"ghor\", \"nc\"]\n\
categories = [\"development-tools\"]\n\
include = [\"ghor.toml\", \"src/**\", \"README.md\", \"LICENSE\", \"docs/include.md\"]\n\
exclude = [\"docs/excluded.md\"]\n",
    );
    fs::write(&manifest, raw).expect("manifest write");
    fs::write(dir.join("README.md"), "# crate format fixture\n").expect("readme");
    fs::write(dir.join("LICENSE"), "MIT\n").expect("license");
    fs::write(dir.join("docs/include.md"), "included\n").expect("include doc");
    fs::write(dir.join("docs/excluded.md"), "excluded\n").expect("excluded doc");
    fs::create_dir_all(dir.join("target")).expect("target");
    fs::write(dir.join("target/junk.bin"), "junk\n").expect("target junk");

    let list = assert_success(
        run_ghor(&[
            "package",
            "--list",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--json",
        ]),
        "ghor package --list",
    );
    assert!(list.contains("src/main.n"));
    assert!(list.contains("docs/include.md"));
    assert!(!list.contains("docs/excluded.md"));
    assert!(!list.contains("target/junk.bin"));

    let package_dir = dir.join("pkg");
    let package = assert_success(
        run_ghor(&[
            "package",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--out",
            package_dir.to_str().expect("package dir"),
            "--json",
        ]),
        "ghor package .crate",
    );
    assert!(package.contains("\"archive_name\": \"crate_format_pkg-0.1.0.crate\""));
    assert!(package.contains("\"status\": \"verified\""));
    assert!(package.contains("\"readme\": \"README.md\""));
    let package_manifest: Value = serde_json::from_slice(
        &fs::read(package_dir.join("manifest.json")).expect("manifest json"),
    )
    .expect("manifest json");
    assert_eq!(package_manifest["format"], "cargo-compatible .crate tar.gz");
    let archive = package_dir.join("crate_format_pkg-0.1.0.crate");
    assert!(archive.exists());
    let archive_list = tar_list(&archive);
    assert!(archive_list.contains("crate_format_pkg-0.1.0/ghor.toml"));
    assert!(archive_list.contains("crate_format_pkg-0.1.0/ghor.toml.orig"));
    assert!(archive_list.contains("crate_format_pkg-0.1.0/ghor.lock"));
    assert!(archive_list.contains("crate_format_pkg-0.1.0/ghor.lock.meta.json"));
    assert!(archive_list.contains("crate_format_pkg-0.1.0/.cargo_vcs_info.json"));
    assert!(archive_list.contains("crate_format_pkg-0.1.0/README.md"));
    assert!(archive_list.contains("crate_format_pkg-0.1.0/docs/include.md"));
    assert!(!archive_list.contains("docs/excluded.md"));
    assert!(!archive_list.contains("target/junk.bin"));
}

#[test]
fn ghor_release_check_preflights_package_without_archiving_ok() {
    let dir = create_project("release_check_preflight_pkg");
    let manifest = dir.join("ghor.toml");
    let out = assert_success(
        run_ghor(&[
            "release-check",
            "--manifest-path",
            manifest.to_str().expect("manifest path"),
            "--json",
        ]),
        "ghor release-check",
    );
    assert!(out.contains("\"schema\": \"ghor.release-check.v1\""));
    assert!(out.contains("\"package_preflight\""));
    assert!(out.contains("\"archive_written\": false"));
    assert!(out.contains("release-check defaults to package preflight"));
    let parsed: Value =
        serde_json::from_str(&out).expect("release-check must emit one JSON document");
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["package_preflight"]["mode"], "bounded_preflight");
    assert!(
        !dir.join("target/package/release_check_preflight_pkg-0.1.0.crate")
            .exists(),
        "release-check must not archive unless --package is explicit"
    );
}

#[test]
fn ghor_publish_dry_run_and_metadata_ok() {
    let dir = create_project("crate_publish_meta_pkg");
    let manifest = dir.join("ghor.toml");
    let raw = fs::read_to_string(&manifest).expect("manifest");
    let raw = insert_package_fields(
        &raw,
        "description = \"ghor publish metadata fixture\"\n\
repository = \"https://example.com/nc/ghor\"\n\
homepage = \"https://example.com/ghor\"\n\
documentation = \"https://example.com/ghor/docs\"\n\
readme = \"README.md\"\n\
keywords = [\"ghor\", \"registry\"]\n\
categories = [\"development-tools\"]\n",
    );
    fs::write(&manifest, raw).expect("manifest write");
    fs::write(dir.join("README.md"), "# publish metadata\n").expect("readme");
    let registry = temp_dir("publish_dry_run_registry");
    let publish = assert_success(
        run_ghor(&[
            "publish",
            "--dry-run",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--json",
        ]),
        "ghor publish dry-run",
    );
    assert!(publish.contains("\"status\": \"dry_run\""));
    assert!(publish.contains("\"description\": \"ghor publish metadata fixture\""));
    assert!(publish.contains("\"repository\": \"https://example.com/nc/ghor\""));
    assert!(publish.contains("\"keywords\""));
    assert!(!registry.join("index.json").exists());
}

#[test]
fn ghor_publish_preflight_rejects_publish_false_and_bad_keyword_ok() {
    let dir = create_project("crate_publish_reject_pkg");
    let manifest = dir.join("ghor.toml");
    let raw = fs::read_to_string(&manifest).expect("manifest");
    let raw = insert_package_fields(&raw, "publish = false\n");
    fs::write(&manifest, raw).expect("manifest write");
    let registry = temp_dir("publish_reject_registry");
    assert_failure_contains(
        run_ghor(&[
            "publish",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--json",
        ]),
        "ghor publish=false reject",
        "E_GHOR_PUBLISH_RESTRICTED_001",
    );

    let bad = create_project("crate_bad_keyword_pkg");
    let bad_manifest = bad.join("ghor.toml");
    let bad_raw = fs::read_to_string(&bad_manifest).expect("bad manifest");
    let bad_raw = insert_package_fields(&bad_raw, "keywords = [\"bad keyword with spaces\"]\n");
    fs::write(&bad_manifest, bad_raw).expect("bad manifest write");
    assert_failure_contains(
        run_ghor(&[
            "package",
            "--manifest-path",
            bad_manifest.to_str().expect("bad manifest"),
            "--out",
            bad.join("pkg").to_str().expect("pkg"),
            "--json",
        ]),
        "ghor bad keyword reject",
        "E_GHOR_PACKAGE_KEYWORDS_001",
    );
}

#[test]
fn ghor_new_multi_language_templates_ok() {
    let languages = [
        ("nC", "src/main.n"),
        ("c", "src/main.c"),
        ("cpp", "src/main.cpp"),
        ("rust", "Cargo.toml"),
        ("java", "src/main/java/Main.java"),
        ("python", "pyproject.toml"),
        ("javascript", "package.json"),
        ("typescript", "tsconfig.json"),
    ];
    for (lang, marker) in languages {
        let dir = temp_dir(&format!("new_lang_{}", lang.to_lowercase()));
        let stdout = assert_success(
            run_ghor(&[
                "new",
                &format!("demo_{}", lang.to_lowercase()),
                "--lang",
                lang,
                "--out",
                dir.to_str().expect("utf8 dir"),
            ]),
            "ghor new --lang",
        );
        assert!(stdout.contains("created"));
        assert!(dir.join("nc.toml").exists());
        assert!(dir.join(marker).exists(), "{lang} marker {marker} missing");
        let manifest = dir.join("nc.toml");
        let manifest_out = assert_success(
            run_ghor(&[
                "manifest",
                "--manifest",
                manifest.to_str().expect("utf8 manifest"),
            ]),
            "ghor manifest language",
        );
        assert!(manifest_out.contains("\"languages\""));
    }
}

#[test]
fn ghor_lang_new_language_forge_ok() {
    for (host, marker) in [
        ("nC", "src/lexer.n"),
        ("c", "src/lexer.c"),
        ("python", "src/lexer.py"),
    ] {
        let dir = temp_dir(&format!("lang_forge_{}", host.to_lowercase()));
        let stdout = assert_success(
            run_ghor(&[
                "lang",
                "new",
                &format!("lumen_{}", host.to_lowercase()),
                "--host",
                host,
                "--out",
                dir.to_str().expect("utf8 dir"),
            ]),
            "ghor lang new",
        );
        assert!(stdout.contains("\"status\": \"created\""));
        assert!(dir.join("nc.toml").exists());
        assert!(dir.join(marker).exists(), "{host} marker {marker} missing");
        assert!(dir.join("reports/conformance_scaffold.json").exists());
    }
}

#[test]
fn ghor_manifest_language_adapter_missing_fail() {
    let dir = temp_dir("adapter_missing");
    std::fs::create_dir_all(&dir).expect("create temp");
    let manifest = dir.join("nc.toml");
    std::fs::write(
        &manifest,
        "[package]\nname = \"adapter_missing\"\nversion = \"0.1.0\"\nkind = \"foreign\"\n\n[languages]\nprimary = \"python\"\nadapters = [\"python\"]\n",
    )
    .expect("write manifest");
    assert_failure_contains(
        run_ghor(&[
            "manifest",
            "--manifest",
            manifest.to_str().expect("utf8 manifest"),
        ]),
        "ghor manifest adapter missing",
        "E_GHOR_LANGUAGE_ADAPTER_MISSING_001",
    );
}

#[test]
fn ghor_manifest_missing_name_fail() {
    let dir = temp_dir("missing_name");
    std::fs::create_dir_all(&dir).expect("create temp");
    let manifest = dir.join("nc.toml");
    std::fs::write(
        &manifest,
        "[package]\nversion = \"0.1.0\"\nkind = \"bin\"\n\n[dependencies]\n",
    )
    .expect("write manifest");
    assert_failure_contains(
        run_ghor(&[
            "manifest",
            "--manifest",
            manifest.to_str().expect("utf8 manifest"),
        ]),
        "ghor manifest missing name",
        "E_GHOR_MANIFEST_MISSING_NAME_001",
    );
}

#[test]
fn ghor_remote_dependency_without_checksum_fail() {
    let dir = create_project("dep_checksum");
    let manifest = dir.join("nc.toml");
    let mut raw = std::fs::read_to_string(&manifest).expect("manifest");
    raw = raw.replace(
        "[dependencies]\n",
        "[dependencies]\nremote_dep = \"1.0.0\"\n",
    );
    std::fs::write(&manifest, raw).expect("manifest write");
    assert_failure_contains(
        run_ghor(&[
            "resolve",
            "--manifest",
            manifest.to_str().expect("utf8 manifest"),
        ]),
        "ghor resolve checksum",
        "E_GHOR_CHECKSUM_001",
    );
}

#[test]
fn ghor_pre_phase17_manifest_surface_ok() {
    let dir = create_project("phase17_manifest_surface");
    let manifest = dir.join("ghor.toml");
    let compat_manifest = dir.join("nc.toml");
    assert!(manifest.exists(), "official ghor.toml must be created");
    assert!(
        compat_manifest.exists(),
        "nc.toml compatibility manifest must remain"
    );
    let manifest_arg = manifest.to_str().expect("utf8 manifest");

    let manifest_out = assert_success(
        run_ghor(&["manifest", "--manifest-path", manifest_arg]),
        "ghor manifest --manifest-path",
    );
    assert!(manifest_out.contains("\"schema\": \"ghor.manifest.v1\""));
    assert!(manifest_out.contains("\"build\""));
    assert!(manifest_out.contains("\"artifact\""));

    let metadata = assert_success(
        run_ghor(&["metadata", "--manifest-path", manifest_arg, "--json"]),
        "ghor metadata",
    );
    assert!(metadata.contains("\"official_manifest\": \"ghor.toml\""));
    assert!(metadata.contains("\"target_matrix\""));
    assert!(metadata.contains("\"foreign_toolchain_matrix\""));

    let targets = assert_success(run_ghor(&["target", "list", "--json"]), "ghor target list");
    assert!(targets.contains("x86_64-unknown-linux-gnu"));
    assert!(targets.contains("wasm32-unknown-unknown"));
}

#[test]
fn ghor_pre_phase17_universal_command_surface_ok() {
    let dir = create_project("phase17_command_surface");
    let manifest = dir.join("ghor.toml");
    let manifest_arg = manifest.to_str().expect("utf8 manifest");

    for (label, args) in [
        (
            "ghor profile",
            vec!["profile", "--manifest-path", manifest_arg, "--json"],
        ),
        (
            "ghor features",
            vec!["features", "--manifest-path", manifest_arg, "--json"],
        ),
        (
            "ghor fetch",
            vec!["fetch", "--manifest-path", manifest_arg, "--json"],
        ),
        (
            "ghor cross",
            vec![
                "cross",
                "--manifest-path",
                manifest_arg,
                "--target",
                "x86_64-unknown-linux-gnu",
                "--json",
            ],
        ),
        (
            "ghor link",
            vec!["link", "--manifest-path", manifest_arg, "--json"],
        ),
        (
            "ghor report",
            vec![
                "report",
                "summary",
                "--manifest-path",
                manifest_arg,
                "--json",
            ],
        ),
    ] {
        let stdout = assert_success(run_ghor(&args), label);
        assert!(stdout.contains("\"schema\": \"ghor."));
    }

    let artifact = dir.join("artifact.json");
    let stdout = assert_success(
        run_ghor(&[
            "artifact",
            "--manifest-path",
            manifest_arg,
            "--out",
            artifact.to_str().expect("utf8 artifact"),
            "--json",
        ]),
        "ghor artifact",
    );
    assert!(stdout.contains("\"status\": \"recorded\""));
    assert!(artifact.exists(), "artifact manifest must be written");
}

#[test]
fn ghor_pre_phase17_asm_template_ok() {
    let dir = temp_dir("asm_template");
    let stdout = assert_success(
        run_ghor(&[
            "new",
            "asm_demo",
            "--lang",
            "asm",
            "--out",
            dir.to_str().expect("utf8 dir"),
        ]),
        "ghor new --lang asm",
    );
    assert!(stdout.contains("created"));
    assert!(dir.join("ghor.toml").exists());
    assert!(dir.join("src/main.S").exists());
    let manifest = dir.join("ghor.toml");
    let manifest_out = assert_success(
        run_ghor(&[
            "manifest",
            "--manifest-path",
            manifest.to_str().expect("utf8 manifest"),
        ]),
        "ghor manifest asm",
    );
    assert!(manifest_out.contains("\"primary\": \"asm\""));
}

#[test]
fn ghor_pre_phase17_unknown_target_fail() {
    let dir = create_project("unknown_target");
    let manifest = dir.join("ghor.toml");
    assert_failure_contains(
        run_ghor(&[
            "cross",
            "--manifest-path",
            manifest.to_str().expect("utf8 manifest"),
            "--target",
            "mips64-unknown-none",
            "--json",
        ]),
        "ghor unknown target",
        "E_GHOR_TARGET_001",
    );
}

#[test]
fn ghor_locked_fails_when_outdated() {
    let dir = create_project("locked_outdated");
    let manifest = dir.join("ghor.toml");
    let manifest_arg = manifest.to_str().expect("utf8 manifest");
    assert_success(
        run_ghor(&["lock", "--manifest-path", manifest_arg]),
        "ghor lock",
    );
    let mut raw = std::fs::read_to_string(&manifest).expect("manifest");
    raw = raw.replace("version = \"0.1.0\"", "version = \"0.2.0\"");
    std::fs::write(&manifest, raw).expect("manifest write");
    assert_failure_contains(
        run_ghor(&["build", "--manifest-path", manifest_arg, "--locked"]),
        "ghor build --locked",
        "E_GHOR_LOCK_DRIFT_001",
    );
}

#[test]
fn ghor_frozen_forbids_lock_update() {
    let dir = create_project("frozen_lock");
    let manifest = dir.join("ghor.toml");
    assert_failure_contains(
        run_ghor(&[
            "update",
            "--manifest-path",
            manifest.to_str().expect("utf8 manifest"),
            "--frozen",
        ]),
        "ghor update --frozen",
        "E_GHOR_FROZEN_001",
    );
}

#[test]
fn ghor_machine_and_basal_passthrough_ok() {
    let machine = assert_success(run_ghor(&["machine", "doctor"]), "ghor machine doctor");
    assert!(machine.contains("machine doctor: OK"));
    let basal = assert_success(run_ghor(&["basal", "doctor"]), "ghor basal doctor");
    assert!(basal.contains("basal doctor"));
}

#[test]
fn ghor_registry_local_publish_search_info_fetch_yank_ok() {
    let package = create_project("registry_pkg");
    let manifest = package.join("ghor.toml");
    let registry = temp_dir("registry_local");
    let registry_arg = registry.to_str().expect("registry");
    let manifest_arg = manifest.to_str().expect("manifest");

    let publish = assert_success(
        run_ghor(&[
            "publish",
            "--manifest-path",
            manifest_arg,
            "--registry",
            registry_arg,
        ]),
        "ghor publish local registry",
    );
    assert!(publish.contains("\"status\": \"published\""));
    assert!(registry.join("config.json").exists());
    assert!(registry.join("index.json").exists());
    let package_index_path = registry.join(cargo_index_path("registry_pkg"));
    assert!(
        package_index_path.exists(),
        "Cargo-style package index must exist"
    );
    let package_index = fs::read_to_string(package_index_path).expect("package index");
    assert!(package_index.contains("\"vers\":\"0.1.0\""));
    assert!(package_index.contains("\"cksum\""));
    assert!(registry
        .join("packages/registry_pkg/0.1.0/registry_pkg-0.1.0.crate")
        .exists());

    let search = assert_success(
        run_ghor(&[
            "search",
            "registry_pkg",
            "--registry",
            registry_arg,
            "--json",
        ]),
        "ghor search local registry",
    );
    assert!(search.contains("\"registry_pkg\""));
    let info = assert_success(
        run_ghor(&["info", "registry_pkg", "--registry", registry_arg, "--json"]),
        "ghor info local registry",
    );
    assert!(info.contains("\"checksum\""));
    let owner_add = assert_success(
        run_ghor(&[
            "owner",
            "add",
            "registry_pkg",
            "release-team",
            "--registry",
            registry_arg,
            "--json",
        ]),
        "ghor owner add",
    );
    assert!(owner_add.contains("\"release-team\""));
    let owner_remove = assert_success(
        run_ghor(&[
            "owner",
            "remove",
            "registry_pkg",
            "release-team",
            "--registry",
            registry_arg,
            "--json",
        ]),
        "ghor owner remove",
    );
    assert!(owner_remove.contains("\"owners\": []"));

    let checksum = registry_index_package_checksum(&registry, "registry_pkg");
    let consumer = temp_dir("registry_consumer");
    let consumer_manifest = write_consumer_manifest(&consumer, "registry_pkg", &checksum);
    let fetch = assert_success(
        run_ghor(&[
            "fetch",
            "--manifest-path",
            consumer_manifest.to_str().expect("consumer manifest"),
            "--registry",
            registry_arg,
            "--json",
        ]),
        "ghor fetch local registry",
    );
    assert!(fetch.contains("\"fetched\""));
    assert!(consumer
        .join("target/ghor/registry-cache/registry_pkg/0.1.0/registry_pkg-0.1.0.crate")
        .exists());

    let yank = assert_success(
        run_ghor(&["yank", "registry_pkg", "--registry", registry_arg, "--json"]),
        "ghor yank local registry",
    );
    assert!(yank.contains("\"status\": \"ok\""));
    let yanked_info = assert_success(
        run_ghor(&["info", "registry_pkg", "--registry", registry_arg, "--json"]),
        "ghor info yanked",
    );
    assert!(yanked_info.contains("\"yanked\": true"));
    let yanked_package_index =
        fs::read_to_string(registry.join(cargo_index_path("registry_pkg"))).expect("package index");
    assert!(yanked_package_index.contains("\"yanked\":true"));
    let audit_path = registry.join("registry-audit.json");
    assert!(audit_path.exists(), "registry audit log must be written");
    let audit = fs::read_to_string(&audit_path).expect("registry audit");
    assert!(audit.contains("\"action\": \"yank\""));
    assert!(!audit.contains("secret-token"));

    let unyank = assert_success(
        run_ghor(&[
            "yank",
            "registry_pkg",
            "--undo",
            "--registry",
            registry_arg,
            "--json",
        ]),
        "ghor unyank local registry",
    );
    assert!(unyank.contains("\"yanked\": false"));
    let unyanked_info = assert_success(
        run_ghor(&["info", "registry_pkg", "--registry", registry_arg, "--json"]),
        "ghor info unyanked",
    );
    assert!(unyanked_info.contains("\"yanked\": false"));
}

#[test]
fn ghor_registry_http_search_info_fetch_ok() {
    let package = create_project("registry_http_pkg");
    let manifest = package.join("ghor.toml");
    let registry = temp_dir("registry_http_source");
    let registry_arg = registry.to_str().expect("registry");
    assert_success(
        run_ghor(&[
            "publish",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry_arg,
        ]),
        "ghor publish local registry for http",
    );
    let base = start_registry_http_server(registry.clone());

    let search = assert_success(
        run_ghor(&["search", "registry_http_pkg", "--registry", &base, "--json"]),
        "ghor search http registry",
    );
    assert!(search.contains("\"registry_transport\": \"http\""));
    let info = assert_success(
        run_ghor(&["info", "registry_http_pkg", "--registry", &base, "--json"]),
        "ghor info http registry",
    );
    assert!(info.contains("\"checksum\""));

    let checksum = registry_index_package_checksum(&registry, "registry_http_pkg");
    let consumer = temp_dir("registry_http_consumer");
    let consumer_manifest = write_consumer_manifest(&consumer, "registry_http_pkg", &checksum);
    let fetch = assert_success(
        run_ghor(&[
            "fetch",
            "--manifest-path",
            consumer_manifest.to_str().expect("consumer manifest"),
            "--registry",
            &base,
            "--json",
        ]),
        "ghor fetch http registry",
    );
    assert!(fetch.contains("\"network\": \"used\""));
    assert!(consumer
        .join("target/ghor/registry-cache/registry_http_pkg/0.1.0/registry_http_pkg-0.1.0.crate")
        .exists());
}

#[test]
fn ghor_registry_http_publish_puts_index_and_package_ok() {
    let package = create_project("registry_put_pkg");
    let manifest = package.join("ghor.toml");
    let registry = temp_dir("registry_http_put");
    fs::create_dir_all(&registry).expect("registry root");
    let base = start_registry_http_server(registry.clone());

    let publish = assert_success(
        run_ghor(&[
            "publish",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            &base,
            "--token",
            "test-token",
            "--json",
        ]),
        "ghor publish http registry",
    );
    assert!(publish.contains("\"registry_transport\": \"http\""));
    assert!(publish.contains("\"web_api_publish\""));
    assert!(publish.contains("\"response_hash\""));
    assert!(registry.join("config.json").exists());
    assert!(registry.join("index.json").exists());
    assert!(registry.join(cargo_index_path("registry_put_pkg")).exists());
    assert!(registry
        .join("packages/registry_put_pkg/0.1.0/registry_put_pkg-0.1.0.crate")
        .exists());

    let info = assert_success(
        run_ghor(&["info", "registry_put_pkg", "--registry", &base, "--json"]),
        "ghor info after http publish",
    );
    assert!(info.contains("\"registry_put_pkg\""));
}

#[test]
fn ghor_registry_http_web_api_yank_owner_login_ok() {
    let package = create_project("registry_api_pkg");
    let manifest = package.join("ghor.toml");
    let registry = temp_dir("registry_http_api");
    let home = temp_dir("registry_http_api_home");
    fs::create_dir_all(&registry).expect("registry root");
    fs::create_dir_all(&home).expect("isolated ghor home");
    let home_arg = home.to_str().expect("isolated ghor home path");
    let base = start_registry_http_server(registry.clone());

    let publish = assert_success(
        run_ghor_env(
            &[
                "publish",
                "--manifest-path",
                manifest.to_str().expect("manifest"),
                "--registry",
                &base,
                "--token",
                "test-token",
                "--json",
            ],
            &[("GHOR_HOME", home_arg)],
        ),
        "ghor api publish",
    );
    assert!(publish.contains("\"web_api_publish\""));
    assert!(publish.contains("\"response_hash\""));
    assert!(!publish.contains("test-token"));

    assert_failure_contains(
        run_ghor_env(
            &[
                "owner",
                "add",
                "registry_api_pkg",
                "no-token-team",
                "--registry",
                &base,
                "--json",
            ],
            &[("GHOR_HOME", home_arg)],
        ),
        "ghor owner add requires auth",
        "E_GHOR_AUTH_001",
    );

    let login = assert_success(
        run_ghor_env(
            &[
                "login",
                "--registry",
                &base,
                "--token",
                "test-token",
                "--json",
            ],
            &[("GHOR_HOME", home_arg)],
        ),
        "ghor login http",
    );
    assert!(login.contains("\"token_stored\""));
    assert!(login.contains("/me"));
    assert!(!login.contains("test-token"));

    let owner_add = assert_success(
        run_ghor_env(
            &[
                "owner",
                "add",
                "registry_api_pkg",
                "release-team",
                "--registry",
                &base,
                "--token",
                "test-token",
                "--json",
            ],
            &[("GHOR_HOME", home_arg)],
        ),
        "ghor api owner add",
    );
    assert!(owner_add.contains("\"web_api_owner\""));
    assert!(owner_add.contains("\"release-team\""));
    assert!(!owner_add.contains("test-token"));

    let owners = assert_success(
        run_ghor_env(
            &[
                "owner",
                "list",
                "registry_api_pkg",
                "--registry",
                &base,
                "--token",
                "test-token",
                "--json",
            ],
            &[("GHOR_HOME", home_arg)],
        ),
        "ghor api owner list",
    );
    assert!(owners.contains("\"release-team\""));

    let yank = assert_success(
        run_ghor_env(
            &[
                "yank",
                "registry_api_pkg",
                "--vers",
                "0.1.0",
                "--registry",
                &base,
                "--token",
                "test-token",
                "--json",
            ],
            &[("GHOR_HOME", home_arg)],
        ),
        "ghor api yank",
    );
    assert!(yank.contains("\"web_api_yank\""));
    assert!(yank.contains("\"response_hash\""));
    assert!(!yank.contains("test-token"));
    let package_index =
        fs::read_to_string(registry.join(cargo_index_path("registry_api_pkg"))).expect("index");
    assert!(package_index.contains("\"yanked\":true"));
}

#[test]
fn ghor_security_login_credentials_permissions_ok() {
    let home = temp_dir("security_credentials_home");
    let registry = temp_dir("security_credentials_registry");
    fs::create_dir_all(&registry).expect("registry");
    let home_arg = home.to_str().expect("home");
    let registry_arg = registry.to_str().expect("registry");

    let login = assert_success(
        run_ghor_env(
            &[
                "login",
                "--registry",
                registry_arg,
                "--token",
                "super-secret-token",
                "--json",
            ],
            &[("GHOR_HOME", home_arg)],
        ),
        "ghor login credentials permissions",
    );
    assert!(login.contains("\"token_stored\""));
    assert!(!login.contains("super-secret-token"));

    let credentials = home.join("credentials.json");
    assert!(credentials.exists(), "credentials file must be written");
    let raw = fs::read_to_string(&credentials).expect("credentials");
    assert!(raw.contains("super-secret-token"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let file_mode = fs::metadata(&credentials)
            .expect("credentials metadata")
            .permissions()
            .mode()
            & 0o777;
        let dir_mode = fs::metadata(&home)
            .expect("credentials home metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(file_mode, 0o600);
        assert_eq!(dir_mode, 0o700);
    }
}

#[cfg(unix)]
#[test]
fn ghor_security_insecure_credentials_do_not_unlock_remote_mutation_fail() {
    use std::os::unix::fs::PermissionsExt;

    let home = temp_dir("security_insecure_credentials_home");
    fs::create_dir_all(&home).expect("home");
    let registry = temp_dir("security_insecure_credentials_registry");
    fs::create_dir_all(&registry).expect("registry");
    let base = start_registry_http_server(registry);
    let credentials = home.join("credentials.json");
    let mut credentials_json = json!({
        "schema": "ghor.credentials.v1",
        "registries": {}
    });
    credentials_json["registries"][base.as_str()] = json!({
        "token": "insecure-token",
        "transport": "http"
    });
    fs::write(
        &credentials,
        serde_json::to_vec_pretty(&credentials_json).expect("credentials json"),
    )
    .expect("credentials write");
    fs::set_permissions(&credentials, fs::Permissions::from_mode(0o644))
        .expect("credentials chmod");

    assert_failure_contains(
        run_ghor_env(
            &[
                "owner",
                "add",
                "registry_api_pkg",
                "release-team",
                "--registry",
                &base,
                "--json",
            ],
            &[("GHOR_HOME", home.to_str().expect("home"))],
        ),
        "ghor owner add ignores insecure credentials",
        "E_GHOR_AUTH_001",
    );
}

#[test]
fn ghor_registry_cdn_governance_and_moderation_ok() {
    let package = create_project("registry_cdn_governed_pkg");
    let registry = temp_dir("registry_cdn_governed");
    let registry_arg = registry.to_str().expect("registry");
    assert_success(
        run_ghor(&[
            "publish",
            "--manifest-path",
            package.join("ghor.toml").to_str().expect("manifest"),
            "--registry",
            registry_arg,
            "--allow-dirty",
            "--json",
        ]),
        "publish governed package",
    );

    let cdn = temp_dir("registry_cdn_out");
    let cdn_out = assert_success(
        run_ghor(&[
            "cdn",
            "build",
            "--registry",
            registry_arg,
            "--out",
            cdn.to_str().expect("cdn"),
            "--json",
        ]),
        "ghor cdn build",
    );
    let cdn_json: Value = serde_json::from_str(&cdn_out).expect("cdn json");
    assert_eq!(cdn_json["status"], "ok");
    assert_eq!(cdn_json["manifest"]["content_addressed"], true);
    assert!(cdn.join("cdn-manifest.json").exists());
    assert!(cdn_json["manifest"]["entry_count"].as_u64().unwrap_or(0) > 0);

    let governance = assert_success(
        run_ghor(&[
            "governance",
            "--init",
            "--role",
            "alice:admin",
            "--registry",
            registry_arg,
            "--token",
            "secret-token",
            "--json",
        ]),
        "ghor governance init",
    );
    assert!(!governance.contains("secret-token"));
    let governance_json: Value = serde_json::from_str(&governance).expect("governance json");
    assert_eq!(governance_json["status"], "ok");
    assert_eq!(governance_json["policy"]["roles"]["alice"], "admin");
    assert!(registry.join("governance.json").exists());

    let moderation = assert_success(
        run_ghor(&[
            "moderation",
            "registry_cdn_governed_pkg",
            "--version",
            "0.1.0",
            "--action",
            "block",
            "--reason",
            "fixture policy violation",
            "--registry",
            registry_arg,
            "--token",
            "secret-token",
            "--json",
        ]),
        "ghor moderation block",
    );
    assert!(!moderation.contains("secret-token"));
    let moderation_json: Value = serde_json::from_str(&moderation).expect("moderation json");
    assert_eq!(moderation_json["moderation_status"], "blocked");
    assert!(registry.join("moderation.json").exists());
    let audit = fs::read_to_string(registry.join("registry-audit.json")).expect("registry audit");
    assert!(audit.contains("governance.update"));
    assert!(audit.contains("moderation.update"));
    assert!(!audit.contains("secret-token"));
    let package_index =
        fs::read_to_string(registry.join(cargo_index_path("registry_cdn_governed_pkg")))
            .expect("package index");
    assert!(package_index.contains("\"moderation_status\":\"blocked\""));
    assert!(package_index.contains("\"yanked\":true"));
}

#[test]
fn ghor_registry_productive_hardening_ok() {
    let package = create_project("registry_hardening_pkg");
    let manifest = package.join("ghor.toml");
    let registry = temp_dir("registry_hardening_source");
    let registry_arg = registry.to_str().expect("registry");
    assert_success(
        run_ghor(&[
            "publish",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry_arg,
            "--allow-dirty",
            "--json",
        ]),
        "ghor publish local registry for hardening",
    );
    let base = start_registry_http_server(registry.clone());
    let cdn = temp_dir("registry_hardening_cdn");
    assert_success(
        run_ghor(&[
            "cdn",
            "build",
            "--registry",
            &base,
            "--out",
            cdn.to_str().expect("cdn"),
            "--json",
        ]),
        "ghor cdn build for hardening",
    );
    assert_success(
        run_ghor(&[
            "governance",
            "--init",
            "--registry",
            &base,
            "--role",
            "release-team:admin",
            "--token",
            "registry-hardening-token",
            "--json",
        ]),
        "ghor governance init for hardening",
    );
    assert_success(
        run_ghor(&[
            "moderation",
            "registry_hardening_pkg",
            "--version",
            "0.1.0",
            "--action",
            "block",
            "--reason",
            "hardening smoke moderation state",
            "--registry",
            &base,
            "--token",
            "registry-hardening-token",
            "--json",
        ]),
        "ghor moderation state for hardening",
    );

    let registry_hardening = assert_success(
        run_ghor(&[
            "registry",
            "hardening",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            &base,
            "--token",
            "registry-hardening-token",
            "--cdn-out",
            cdn.to_str().expect("cdn"),
            "--json",
        ]),
        "ghor registry hardening",
    );
    assert!(!registry_hardening.contains("registry-hardening-token"));
    let value: Value = serde_json::from_str(&registry_hardening).expect("registry hardening json");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["summary"]["release_ready"], true);
    assert!(value["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .any(|check| check["id"] == "package_checksum" && check["status"] == "ok"));

    let release_hardening = assert_success(
        run_ghor(&[
            "release-hardening",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            &base,
            "--token",
            "registry-hardening-token",
            "--cdn-out",
            cdn.to_str().expect("cdn"),
            "--json",
        ]),
        "ghor release hardening",
    );
    assert!(!release_hardening.contains("registry-hardening-token"));
    let value: Value = serde_json::from_str(&release_hardening).expect("release hardening json");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["registry_hardening"]["status"], "ok");
}

#[test]
fn ghor_registry_productive_hardening_requires_remote_token_fail() {
    let package = create_project("registry_hardening_no_token_pkg");
    let manifest = package.join("ghor.toml");
    let registry = temp_dir("registry_hardening_no_token");
    let registry_arg = registry.to_str().expect("registry");
    assert_success(
        run_ghor(&[
            "publish",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry_arg,
            "--allow-dirty",
            "--json",
        ]),
        "ghor publish local registry before no-token hardening",
    );
    let base = start_registry_http_server(registry);
    assert_failure_contains(
        run_ghor(&[
            "registry",
            "hardening",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            &base,
            "--json",
        ]),
        "ghor registry hardening requires remote token",
        "E_GHOR_REGISTRY_HARDENING_001",
    );
}

#[test]
fn ghor_cargo_build_script_executes_real_build_rs_ok() {
    if !cargo_available() {
        eprintln!("cargo unavailable; skipping");
        return;
    }
    let dir = temp_dir("cargo_build_script_exec");
    fs::create_dir_all(dir.join("src")).expect("src");
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"cargo_build_script_exec\"\nversion = \"0.1.0\"\nedition = \"2021\"\nbuild = \"build.rs\"\n\n",
    )
    .expect("manifest");
    fs::write(
        dir.join("build.rs"),
        "fn main() { println!(\"cargo:warning=GHOR_BUILD_SCRIPT_EXECUTED\"); }\n",
    )
    .expect("build rs");
    fs::write(dir.join("src/lib.rs"), "pub fn ok() -> bool { true }\n").expect("lib");
    let out = assert_success(
        run_ghor(&[
            "build-script",
            "--manifest-path",
            dir.join("Cargo.toml").to_str().expect("manifest"),
            "--json",
        ]),
        "ghor build-script",
    );
    let value: Value = serde_json::from_str(&out).expect("build-script json");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["build_script_declared"], true);
    assert_eq!(value["build_script_execution"], "executed_by_cargo_check");
    assert_eq!(value["stdout_contains_build_script_warning"], true);
}

#[test]
fn ghor_cargo_proc_macro_executes_real_macro_ok() {
    if !cargo_available() {
        eprintln!("cargo unavailable; skipping");
        return;
    }
    let dir = temp_dir("cargo_proc_macro_exec");
    let macro_dir = dir.join("macro_crate");
    let consumer_dir = dir.join("consumer");
    fs::create_dir_all(macro_dir.join("src")).expect("macro src");
    fs::create_dir_all(consumer_dir.join("src")).expect("consumer src");
    fs::write(
        macro_dir.join("Cargo.toml"),
        "[package]\nname = \"ghor_proc_macro_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\nproc-macro = true\n\n",
    )
    .expect("macro manifest");
    fs::write(
        macro_dir.join("src/lib.rs"),
        "use proc_macro::TokenStream;\n#[proc_macro]\npub fn make_answer(_input: TokenStream) -> TokenStream { \"pub fn generated_answer() -> u32 { 42 }\".parse().unwrap() }\n",
    )
    .expect("macro lib");
    fs::write(
        consumer_dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"ghor_proc_macro_consumer\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nghor_proc_macro_fixture = {{ path = \"{}\" }}\n\n",
            macro_dir.to_string_lossy().replace('\\', "/")
        ),
    )
    .expect("consumer manifest");
    fs::write(
        consumer_dir.join("src/lib.rs"),
        "use ghor_proc_macro_fixture::make_answer;\nmake_answer!();\npub fn call() -> u32 { generated_answer() }\n",
    )
    .expect("consumer lib");
    let out = assert_success(
        run_ghor(&[
            "proc-macro",
            "--manifest-path",
            consumer_dir.join("Cargo.toml").to_str().expect("manifest"),
            "--json",
        ]),
        "ghor proc-macro",
    );
    let value: Value = serde_json::from_str(&out).expect("proc-macro json");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["cargo_check_executed"], true);
    assert_eq!(value["proc_macro_execution"], "validated_by_cargo_check");
}

#[test]
fn ghor_login_logout_with_credential_provider_ok() {
    let project = create_project("login_provider_pkg");
    let registry = temp_dir("login_provider_registry");
    fs::create_dir_all(&registry).expect("registry");
    let base = start_registry_http_server(registry);
    fs::create_dir_all(project.join(".cargo")).expect("cargo dir");
    fs::write(
        project.join(".cargo/config.toml"),
        format!(
            "[registries.secure]\nindex = \"{}\"\ncredential-provider = \"ghor-token\"\n\n[credential-alias]\nghor-token = [\"sh\", \"-c\", \"printf provider-token\"]\n",
            base
        ),
    )
    .expect("cargo config");
    let manifest = project.join("ghor.toml");
    let home = temp_dir("login_provider_home");
    let home_arg = home.to_str().expect("home");
    let login = assert_success(
        run_ghor_env(
            &[
                "login",
                "--manifest-path",
                manifest.to_str().expect("manifest"),
                "--registry",
                "secure",
                "--json",
            ],
            &[("GHOR_HOME", home_arg)],
        ),
        "ghor login provider",
    );
    assert!(login.contains("\"token_stored\""));
    let credentials = fs::read_to_string(home.join("credentials.json")).expect("credentials");
    assert!(credentials.contains("provider-token"));
    let logout = assert_success(
        run_ghor_env(
            &[
                "logout",
                "--manifest-path",
                manifest.to_str().expect("manifest"),
                "--registry",
                "secure",
                "--json",
            ],
            &[("GHOR_HOME", home_arg)],
        ),
        "ghor logout provider registry",
    );
    assert!(logout.contains("\"removed\": true"));
    let credentials = fs::read_to_string(home.join("credentials.json")).expect("credentials");
    assert!(!credentials.contains("provider-token"));
}

#[test]
fn ghor_registry_sparse_http_cache_etag_304_ok() {
    let package = create_project("registry_sparse_pkg");
    let manifest = package.join("ghor.toml");
    let registry = temp_dir("registry_sparse_source");
    assert_success(
        run_ghor(&[
            "publish",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
        ]),
        "ghor publish local registry for sparse",
    );
    let base = start_registry_http_server(registry.clone());
    let sparse = format!("sparse+{base}");
    let checksum = registry_index_package_checksum(&registry, "registry_sparse_pkg");
    let consumer = temp_dir("registry_sparse_consumer");
    let consumer_manifest = write_consumer_manifest(&consumer, "registry_sparse_pkg", &checksum);
    for label in ["first sparse fetch", "second sparse fetch"] {
        let fetch = assert_success(
            run_ghor(&[
                "fetch",
                "--manifest-path",
                consumer_manifest.to_str().expect("consumer manifest"),
                "--registry",
                &sparse,
                "--json",
            ]),
            label,
        );
        assert!(fetch.contains("\"registry_transport\": \"sparse+http\""));
    }
    let requests = fs::read_to_string(registry.join("requests.log")).expect("requests log");
    assert!(
        requests.contains(" 304"),
        "sparse registry should exercise HTTP 304 cache path\n{requests}"
    );
}

#[test]
fn ghor_registry_git_index_fetch_ok() {
    if !git_available() {
        return;
    }
    let package = create_project("registry_git_pkg");
    let manifest = package.join("ghor.toml");
    let registry = temp_dir("registry_git_source");
    assert_success(
        run_ghor(&[
            "publish",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
        ]),
        "ghor publish local registry for git",
    );
    assert_success(
        Command::new("git")
            .arg("-C")
            .arg(&registry)
            .arg("init")
            .output()
            .expect("git init"),
        "git init registry",
    );
    assert_success(
        Command::new("git")
            .arg("-C")
            .arg(&registry)
            .args(["add", "."])
            .output()
            .expect("git add"),
        "git add registry",
    );
    assert_success(
        Command::new("git")
            .arg("-C")
            .arg(&registry)
            .args([
                "-c",
                "user.name=ghor-test",
                "-c",
                "user.email=ghor-test@example.invalid",
                "commit",
                "-m",
                "registry fixture",
            ])
            .output()
            .expect("git commit"),
        "git commit registry",
    );
    let checksum = registry_index_package_checksum(&registry, "registry_git_pkg");
    let consumer = temp_dir("registry_git_consumer");
    let consumer_manifest = write_consumer_manifest(&consumer, "registry_git_pkg", &checksum);
    let registry_url = format!("git+file://{}", registry.display());
    let fetch = assert_success(
        run_ghor(&[
            "fetch",
            "--manifest-path",
            consumer_manifest.to_str().expect("consumer manifest"),
            "--registry",
            &registry_url,
            "--json",
        ]),
        "ghor fetch git registry",
    );
    assert!(fetch.contains("\"registry_transport\": \"git\""));
    assert!(consumer
        .join("target/ghor/registry-cache/registry_git_pkg/0.1.0/registry_git_pkg-0.1.0.crate")
        .exists());
}

#[test]
fn ghor_cargo_config_named_registry_default_fetch_ok() {
    let package = create_project("cargo_config_pkg");
    let manifest = package.join("ghor.toml");
    let registry = temp_dir("cargo_config_named_registry");
    assert_success(
        run_ghor(&[
            "publish",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--json",
        ]),
        "publish fixture for cargo config registry",
    );
    let checksum = registry_index_package_checksum(&registry, "cargo_config_pkg");
    let consumer = temp_dir("cargo_config_consumer");
    let consumer_manifest = write_consumer_manifest(&consumer, "cargo_config_pkg", &checksum);
    fs::create_dir_all(consumer.join(".cargo")).expect("cargo config dir");
    fs::write(
        consumer.join(".cargo/config.toml"),
        format!(
            "[registry]\ndefault = \"mirror\"\n\n[registries]\nmirror = {{ index = \"{}\" }}\n\n[net]\noffline = true\nretry = 4\ngit-fetch-with-cli = true\n\n[http]\ntimeout = 11\nproxy = \"http://127.0.0.1:9\"\ncainfo = \"certs/ca.pem\"\nproxy-cainfo = \"certs/proxy-ca.pem\"\ncheck-revoke = false\n\n[git]\nproxy = \"http://127.0.0.1:9\"\ncainfo = \"certs/git-ca.pem\"\n",
            registry.display()
        ),
    )
    .expect("cargo config");

    let config = assert_success(
        run_ghor(&[
            "registry",
            "cargo-config",
            "--manifest-path",
            consumer_manifest.to_str().expect("manifest"),
            "--json",
        ]),
        "ghor registry cargo-config",
    );
    let value: Value = serde_json::from_str(&config).expect("cargo config json");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["registry_transport"], "filesystem");
    assert_eq!(value["config"]["registry"]["default"], "mirror");
    assert_eq!(value["config"]["net"]["offline"], "true");
    assert_eq!(value["config"]["http"]["timeout"], "11");
    assert_eq!(value["config"]["git"]["proxy"], "http://127.0.0.1:9");

    let fetch = assert_success(
        run_ghor(&[
            "fetch",
            "--manifest-path",
            consumer_manifest.to_str().expect("manifest"),
            "--json",
        ]),
        "ghor fetch default named registry from cargo config",
    );
    assert!(fetch.contains("\"registry_transport\": \"filesystem\""));
    assert!(consumer
        .join("target/ghor/registry-cache/cargo_config_pkg/0.1.0/cargo_config_pkg-0.1.0.crate")
        .exists());
}

#[test]
fn ghor_cargo_config_credential_provider_alias_ok() {
    let project = create_project("cargo_config_provider");
    fs::create_dir_all(project.join(".cargo")).expect("cargo config dir");
    fs::write(
        project.join(".cargo/config.toml"),
        "[registries.secure]\nindex = \"https://example.invalid/index\"\ncredential-provider = \"ghor-token\"\n\n[credential-alias]\nghor-token = [\"sh\", \"-c\", \"printf provider-token\"]\n",
    )
    .expect("cargo config");
    let manifest = project.join("ghor.toml");
    let config = assert_success(
        run_ghor(&[
            "registry",
            "cargo-config",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            "secure",
            "--json",
        ]),
        "ghor credential provider alias",
    );
    let value: Value = serde_json::from_str(&config).expect("cargo config json");
    assert_eq!(value["credential_status"], "available");
    assert_eq!(value["registry_transport"], "https");
    assert_eq!(value["config"]["selected_registry_name"], "secure");
    assert_eq!(value["config"]["credential_aliases"][0], "ghor-token");
    assert!(
        !config.contains("provider-token"),
        "effective config output must not leak provider tokens"
    );
}

#[test]
fn ghor_cargo_config_offline_remote_deferred_fail() {
    let project = create_project("cargo_config_offline");
    fs::create_dir_all(project.join(".cargo")).expect("cargo config dir");
    fs::write(
        project.join(".cargo/config.toml"),
        "[registry]\ndefault = \"remote\"\n\n[registries.remote]\nindex = \"sparse+https://example.invalid/index\"\n\n[net]\noffline = true\n",
    )
    .expect("cargo config");
    let manifest = project.join("ghor.toml");
    assert_failure_contains(
        run_ghor(&[
            "registry",
            "index",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--json",
        ]),
        "ghor offline remote registry",
        "E_GHOR_NET_OFFLINE_001",
    );
}

#[test]
fn ghor_update_real_resolver_dry_run_and_precise_ok() {
    let registry = temp_dir("update_real_registry");
    publish_project_version(&registry, "update_dep", "1.0.0", "");
    publish_project_version(&registry, "update_dep", "1.2.0", "");
    let checksum = registry_index_package_checksum_version(&registry, "update_dep", "1.2.0");
    let consumer = temp_dir("update_real_consumer");
    let manifest = write_consumer_manifest(&consumer, "update_dep", &checksum);
    let mut raw = fs::read_to_string(&manifest).expect("consumer manifest");
    raw = raw.replace(
        "update_dep = { version = \"0.1.0\"",
        "update_dep = { version = \"1\"",
    );
    fs::write(&manifest, raw).expect("consumer manifest write");

    let dry = assert_success(
        run_ghor(&[
            "update",
            "--dry-run",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--json",
        ]),
        "ghor update dry-run",
    );
    assert!(dry.contains("\"status\": \"dry_run\""));
    assert!(!consumer.join("ghor.lock").exists());

    let update = assert_success(
        run_ghor(&[
            "update",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--json",
        ]),
        "ghor update writes lock",
    );
    assert!(update.contains("\"status\": \"updated\""));
    let lock = fs::read_to_string(consumer.join("ghor.lock")).expect("lock");
    assert!(lock.contains("version = 4"));
    assert!(lock.contains("version = \"1.2.0\""));
    assert!(consumer.join("ghor.lock.meta.json").exists());

    let precise = assert_success(
        run_ghor(&[
            "update",
            "-p",
            "update_dep",
            "--precise",
            "1.0.0",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--json",
        ]),
        "ghor update precise",
    );
    assert!(precise.contains("\"precise\": \"1.0.0\""));
    let lock = fs::read_to_string(consumer.join("ghor.lock")).expect("lock");
    assert!(lock.contains("version = \"1.0.0\""));
}

#[test]
fn ghor_lock_exports_cargo_compatible_lockfile_ok() {
    let registry = temp_dir("lock_cargo_compatible_registry");
    publish_project_version(&registry, "lock_dep", "0.1.0", "");
    let checksum = registry_index_package_checksum_version(&registry, "lock_dep", "0.1.0");
    let consumer = temp_dir("lock_cargo_compatible_consumer");
    let manifest = write_consumer_manifest(&consumer, "lock_dep", &checksum);
    let lock_out = assert_success(
        run_ghor(&[
            "lock",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--json",
        ]),
        "ghor lock cargo-compatible",
    );
    assert!(lock_out.contains("\"cargo_lock_format_version\": 4"));
    assert!(lock_out.contains("\"lock_format\": \"cargo_lock_toml_primary\""));
    let cargo_lock = fs::read_to_string(consumer.join("ghor.lock")).expect("lock");
    assert!(cargo_lock.starts_with("# This file is automatically @generated by ghor.\nversion = 4"));
    assert!(cargo_lock.contains("name = \"lock_dep\""));
    assert!(cargo_lock.contains("source = \"registry+"));
    assert!(cargo_lock.contains("checksum = \""));
    let lock_meta: Value = serde_json::from_slice(
        &fs::read(consumer.join("ghor.lock.meta.json")).expect("lock metadata"),
    )
    .expect("lock metadata json");
    assert_eq!(lock_meta["cargo_lock_format_version"], 4);
    assert_eq!(lock_meta["cargo_packages"][0]["name"], "lock_dep");
    assert_eq!(
        lock_meta["cargo_lock_toml"].as_str(),
        Some(cargo_lock.as_str())
    );
}

#[test]
fn ghor_git_dependency_exact_head_ok() {
    if !git_available() {
        return;
    }
    let dep = create_project("git_exact_dep");
    assert_success(
        Command::new("git")
            .arg("-C")
            .arg(&dep)
            .arg("init")
            .output()
            .expect("git init"),
        "git init dep",
    );
    assert_success(
        Command::new("git")
            .arg("-C")
            .arg(&dep)
            .args(["add", "."])
            .output()
            .expect("git add"),
        "git add dep",
    );
    assert_success(
        Command::new("git")
            .arg("-C")
            .arg(&dep)
            .args([
                "-c",
                "user.name=ghor-test",
                "-c",
                "user.email=ghor-test@example.invalid",
                "commit",
                "-m",
                "git dep fixture",
            ])
            .output()
            .expect("git commit"),
        "git commit dep",
    );
    let head = assert_success(
        Command::new("git")
            .arg("-C")
            .arg(&dep)
            .args(["rev-parse", "HEAD"])
            .output()
            .expect("git rev-parse"),
        "git head",
    )
    .trim()
    .to_string();
    let consumer = temp_dir("git_exact_consumer");
    fs::create_dir_all(consumer.join("src")).expect("consumer src");
    fs::write(
        consumer.join("src/main.n"),
        "fn main() -> i32 { return 0; }\n",
    )
    .expect("consumer src");
    let manifest = consumer.join("ghor.toml");
    fs::write(
        &manifest,
        format!(
            "[package]\nname = \"git_exact_consumer\"\nversion = \"0.1.0\"\nkind = \"bin\"\n\n[target.default]\ntriple = \"x86_64-unknown-linux-gnu\"\n\n[languages]\nprimary = \"nC\"\nadapters = []\n\n[dependencies]\ngit_exact_dep = {{ git = \"{}\", rev = \"{}\" }}\n",
            dep.display(),
            head
        ),
    )
    .expect("consumer manifest");
    let home = temp_dir("git_exact_home");
    let resolve = assert_success(
        run_ghor_env(
            &[
                "resolve",
                "--manifest-path",
                manifest.to_str().expect("manifest"),
                "--json",
            ],
            &[("GHOR_HOME", home.to_str().expect("home"))],
        ),
        "ghor resolve git dep exact-head",
    );
    let value: Value = serde_json::from_str(&resolve).expect("resolve json");
    let resolved = value["resolved"].as_array().expect("resolved");
    let git = resolved
        .iter()
        .find(|entry| entry["status"] == "resolved_git")
        .expect("git dependency");
    assert_eq!(git["source"]["resolved_head"].as_str(), Some(head.as_str()));
    assert!(git["source"]["source"]
        .as_str()
        .expect("source")
        .contains(&format!("#{head}")));
    assert!(!resolve.contains("E_GHOR_CHECKSUM_001"));
}

#[test]
fn ghor_vendor_registry_extracts_package_and_config_ok() {
    let package = create_project("vendor_registry_pkg");
    let manifest = package.join("ghor.toml");
    let registry = temp_dir("vendor_registry_source");
    assert_success(
        run_ghor(&[
            "publish",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--json",
        ]),
        "publish vendor fixture",
    );
    let checksum = registry_index_package_checksum(&registry, "vendor_registry_pkg");
    let consumer = temp_dir("vendor_registry_consumer");
    let consumer_manifest = write_consumer_manifest(&consumer, "vendor_registry_pkg", &checksum);
    let vendor_dir = consumer.join("third-party/vendor");
    let vendor = assert_success(
        run_ghor(&[
            "vendor",
            vendor_dir.to_str().expect("vendor"),
            "--versioned-dirs",
            "--manifest-path",
            consumer_manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--json",
        ]),
        "ghor vendor registry",
    );
    assert!(vendor.contains("\"schema\": \"ghor.vendor.v1\""));
    assert!(vendor.contains("vendored-sources"));
    assert!(vendor_dir
        .join("vendor_registry_pkg-0.1.0/ghor.toml")
        .exists());
    assert!(vendor_dir
        .join("vendor_registry_pkg-0.1.0/.cargo-checksum.json")
        .exists());
    let checksum_json: Value = serde_json::from_slice(
        &fs::read(vendor_dir.join("vendor_registry_pkg-0.1.0/.cargo-checksum.json"))
            .expect("vendor checksum"),
    )
    .expect("checksum json");
    assert_eq!(checksum_json["package"].as_str(), Some(checksum.as_str()));
    assert!(checksum_json["files"]["ghor.toml"].as_str().is_some());
    assert!(checksum_json["files"]["src/main.n"].as_str().is_some());
}

#[test]
fn ghor_tree_and_metadata_include_resolver_nodes_ok() {
    let registry = temp_dir("tree_metadata_registry");
    publish_project_version(&registry, "tree_dep", "1.0.0", "");
    let checksum = registry_index_package_checksum_version(&registry, "tree_dep", "1.0.0");
    let consumer = temp_dir("tree_metadata_consumer");
    let manifest = write_consumer_manifest(&consumer, "tree_dep", &checksum);
    let tree = assert_success(
        run_ghor(&[
            "tree",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--json",
        ]),
        "ghor tree resolver",
    );
    assert!(tree.contains("\"nodes\""));
    assert!(tree.contains("\"tree_dep\""));
    let metadata = assert_success(
        run_ghor(&[
            "metadata",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--json",
        ]),
        "ghor metadata cargo-compatible",
    );
    assert!(metadata.contains("\"cargo_metadata_format_version\": 1"));
    assert!(metadata.contains("\"workspace_members\""));
    assert!(metadata.contains("\"target_directory\""));
    assert!(metadata.contains("\"resolve\""));
    let value: Value = serde_json::from_str(&metadata).expect("metadata json");
    assert_eq!(value["cargo_metadata_format_version"], 1);
    assert!(value["packages"][0]["edition"].as_str().is_some());
    assert!(value["packages"][0]["authors"].as_array().is_some());
    assert!(value["cargo_compatible"]["resolve"]["nodes"][0]["deps"]
        .as_array()
        .is_some());
}

#[test]
fn ghor_install_from_registry_writes_launcher_and_source_ok() {
    let package = create_project("install_registry_pkg");
    let manifest = package.join("ghor.toml");
    let registry = temp_dir("install_registry_source");
    assert_success(
        run_ghor(&[
            "publish",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--json",
        ]),
        "publish install fixture",
    );
    let root = temp_dir("install_registry_root");
    let install = assert_success(
        run_ghor(&[
            "install",
            "install_registry_pkg",
            "--version",
            "0.1.0",
            "--registry",
            registry.to_str().expect("registry"),
            "--root",
            root.to_str().expect("root"),
            "--json",
        ]),
        "ghor install registry",
    );
    assert!(install.contains("\"source\": \"registry\""));
    assert!(root.join("bin/install_registry_pkg").exists());
    assert!(root
        .join("registry-src/install_registry_pkg-0.1.0/ghor.toml")
        .exists());
}

#[test]
fn ghor_resolver_semver_selects_highest_compatible_ok() {
    let registry = temp_dir("resolver_semver_registry");
    publish_project_version(&registry, "semver_dep", "1.0.0", "");
    publish_project_version(&registry, "semver_dep", "1.5.0", "");
    publish_project_version(&registry, "semver_dep", "2.0.0", "");
    let checksum = registry_index_package_checksum_version(&registry, "semver_dep", "1.5.0");
    let consumer = temp_dir("resolver_semver_consumer");
    let manifest = write_consumer_manifest(&consumer, "semver_dep", &checksum);
    let mut raw = fs::read_to_string(&manifest).expect("consumer manifest");
    raw = raw.replace(
        "semver_dep = { version = \"0.1.0\"",
        "semver_dep = { version = \"1\"",
    );
    fs::write(&manifest, raw).expect("consumer manifest write");
    let resolve = assert_success(
        run_ghor(&[
            "resolve",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--json",
        ]),
        "ghor resolve semver",
    );
    assert!(resolve.contains("\"schema\": \"ghor.resolve.v2\""));
    assert!(resolve.contains("\"version\": \"1.5.0\""));
    assert!(!resolve.contains("\"version\": \"2.0.0\""));
}

#[test]
fn ghor_resolver_features_v1_v2_build_dev_target_ok() {
    let registry = temp_dir("resolver_features_registry");
    publish_project_version(
        &registry,
        "feature_dep",
        "1.0.0",
        "default = [\"base\"]\nbase = []\nserde = []\nstd = []\nwinapi = []\n",
    );
    let checksum = registry_index_package_checksum_version(&registry, "feature_dep", "1.0.0");
    let consumer = temp_dir("resolver_features_consumer");
    fs::create_dir_all(consumer.join("src")).expect("src");
    fs::write(
        consumer.join("src/main.n"),
        "fn main() -> i32 { return 0; }\n",
    )
    .expect("src");
    let manifest = consumer.join("ghor.toml");
    fs::write(
        &manifest,
        format!(
            "[package]\nname = \"resolver_features_consumer\"\nversion = \"0.1.0\"\nkind = \"bin\"\nedition = \"2021\"\n\n[target.default]\ntriple = \"x86_64-unknown-linux-gnu\"\n\n[languages]\nprimary = \"nC\"\nadapters = []\n\n[dependencies]\nfeature_dep = {{ version = \"1\", checksum = \"{checksum}\", features = [\"serde\"], default-features = false }}\n\n[build-dependencies]\nfeature_dep = {{ version = \"1\", checksum = \"{checksum}\", features = [\"std\"], default-features = false }}\n\n[dev-dependencies]\nfeature_dep = {{ version = \"1\", checksum = \"{checksum}\", features = [\"base\"], default-features = false }}\n\n[target.x86_64-pc-windows-msvc.dependencies]\nfeature_dep = {{ version = \"1\", checksum = \"{checksum}\", features = [\"winapi\"], default-features = false }}\n"
        ),
    )
    .expect("manifest");
    let resolve_v1 = assert_success(
        run_ghor(&[
            "resolve",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--resolver",
            "1",
            "--json",
        ]),
        "ghor resolve features v1",
    );
    assert!(resolve_v1.contains("\"group\": \"feature_dep@1.0.0:unified\""));
    assert!(resolve_v1.contains("\"serde\""));
    assert!(resolve_v1.contains("\"std\""));
    assert!(resolve_v1.contains("\"winapi\""));

    let resolve_v2 = assert_success(
        run_ghor(&[
            "resolve",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--resolver",
            "2",
            "--json",
        ]),
        "ghor resolve features v2",
    );
    assert!(resolve_v2.contains("\"group\": \"feature_dep@1.0.0:normal\""));
    assert!(resolve_v2.contains("\"group\": \"feature_dep@1.0.0:build\""));
    assert!(resolve_v2.contains("\"status\": \"skipped_inactive_target\""));
}

#[test]
fn ghor_resolver_overrides_alternate_registry_source_replacement_ok() {
    let registry = temp_dir("resolver_alt_registry");
    publish_project_version(&registry, "mirror_dep", "1.0.0", "");
    let checksum = registry_index_package_checksum_version(&registry, "mirror_dep", "1.0.0");
    let consumer = temp_dir("resolver_source_replacement_consumer");
    fs::create_dir_all(consumer.join("src")).expect("src");
    fs::write(
        consumer.join("src/main.n"),
        "fn main() -> i32 { return 0; }\n",
    )
    .expect("src");
    let patch_dir = consumer.join("patched_dep");
    fs::create_dir_all(&patch_dir).expect("patch dir");
    let manifest = consumer.join("ghor.toml");
    fs::write(
        &manifest,
        format!(
            "[package]\nname = \"resolver_source_replacement_consumer\"\nversion = \"0.1.0\"\nkind = \"bin\"\n\n[target.default]\ntriple = \"x86_64-unknown-linux-gnu\"\n\n[languages]\nprimary = \"nC\"\nadapters = []\n\n[registries]\nmirror = \"{}\"\n\n[source.crates-io]\nreplace-with = \"mirror\"\n\n[dependencies]\nmirror_dep = {{ version = \"1\", checksum = \"{checksum}\", registry = \"crates-io\" }}\npatched_dep = {{ version = \"1\", checksum = \"{checksum}\" }}\nreplaced_dep = {{ version = \"1\", checksum = \"{checksum}\" }}\n\n[patch.crates-io]\npatched_dep = {{ path = \"{}\", version = \"9.0.0\" }}\n\n[replace]\nreplaced_dep = {{ path = \"{}\", version = \"8.0.0\" }}\n",
            registry.display(),
            patch_dir.display(),
            patch_dir.display()
        ),
    )
    .expect("manifest");
    let resolve = assert_success(
        run_ghor(&[
            "resolve",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--json",
        ]),
        "ghor resolve source replacement",
    );
    assert!(resolve.contains("\"replacement_kind\": \"alternate_registry\""));
    assert!(resolve.contains("\"override\": \"patch\""));
    assert!(resolve.contains("\"override\": \"replace\""));
    assert!(resolve.contains("\"resolved_path\""));
}

#[test]
fn ghor_workspace_virtual_manifest_metadata_ok() {
    let (root, workspace_manifest, _) = create_workspace_fixture("workspace_metadata");
    let stdout = assert_success(
        run_ghor(&[
            "workspace",
            "metadata",
            "--manifest-path",
            workspace_manifest.to_str().expect("manifest"),
            "--json",
        ]),
        "ghor workspace metadata",
    );
    let value: Value = serde_json::from_str(&stdout).expect("workspace metadata json");
    assert_eq!(value["schema"], "ghor.workspace.metadata.v2");
    assert_eq!(value["virtual_manifest"], true);
    assert_eq!(
        value["workspace_root"].as_str(),
        Some(root.to_string_lossy().as_ref())
    );
    assert_eq!(
        value["shared_lockfile"].as_str(),
        Some(root.join("ghor.lock").to_string_lossy().as_ref())
    );
    assert_eq!(
        value["shared_target"].as_str(),
        Some(root.join("target").join("ghor").to_string_lossy().as_ref())
    );
    let members = value["members"].as_array().expect("members");
    assert!(members
        .iter()
        .any(|member| member["package"] == "workspace_app"));
    assert!(members
        .iter()
        .any(|member| member["package"] == "workspace_tool"));
    assert!(members
        .iter()
        .any(|member| member["package"] == "shared_dep"));
    assert!(!members
        .iter()
        .any(|member| member["package"] == "workspace_skip"));
    let defaults = value["default_members"]
        .as_array()
        .expect("default members");
    assert_eq!(defaults.len(), 1);
    assert_eq!(defaults[0]["package"], "workspace_app");
}

#[test]
fn ghor_workspace_member_inherits_package_dependencies_lints_ok() {
    let (_root, _workspace_manifest, app_manifest) =
        create_workspace_fixture("workspace_inheritance");
    let stdout = assert_success(
        run_ghor(&[
            "manifest",
            "--manifest-path",
            app_manifest.to_str().expect("manifest"),
            "--json",
        ]),
        "ghor manifest workspace member",
    );
    let value: Value = serde_json::from_str(&stdout).expect("manifest json");
    assert_eq!(value["package"]["version"], "1.2.3");
    assert_eq!(value["package"]["edition"], "2026");
    assert_eq!(value["package"]["license"], "MIT");
    assert_eq!(value["workspace_is_root"], false);
    assert_eq!(value["dependencies"][0]["name"], "shared_dep");
    assert_eq!(value["dependencies"][0]["path"], "crates/shared_dep");
    assert_eq!(value["dependencies"][0]["workspace_inherited"], true);
    assert_eq!(value["lints"]["rust.unsafe_code"], "deny");
}

#[test]
fn ghor_workspace_shared_lock_target_and_default_members_ok() {
    let (root, workspace_manifest, app_manifest) = create_workspace_fixture("workspace_shared");
    assert_success(
        run_ghor(&[
            "lock",
            "--manifest-path",
            app_manifest.to_str().expect("manifest"),
            "--json",
        ]),
        "ghor lock workspace member",
    );
    assert!(root.join("ghor.lock").exists());
    assert!(root.join("ghor.lock.meta.json").exists());
    assert!(!root.join("crates/app/ghor.lock").exists());

    assert_success(
        run_ghor(&[
            "build",
            "--manifest-path",
            app_manifest.to_str().expect("manifest"),
            "--json",
        ]),
        "ghor build workspace member",
    );
    assert!(root
        .join("target/ghor/debug/workspace_app.build.json")
        .exists());
    assert!(!root
        .join("crates/app/target/ghor/debug/workspace_app.build.json")
        .exists());

    let workspace_check = assert_success(
        run_ghor(&[
            "workspace",
            "check",
            "--manifest-path",
            workspace_manifest.to_str().expect("manifest"),
            "--json",
        ]),
        "ghor workspace check",
    );
    assert!(workspace_check.contains("\"schema\": \"ghor.workspace.action.v1\""));
    assert!(workspace_check.contains("\"package\": \"workspace_app\""));
    assert!(!workspace_check.contains("\"package\": \"workspace_tool\""));
}

#[test]
fn ghor_workspace_cli_check_build_test_workspace_ok() {
    let (root, workspace_manifest, _) = create_workspace_fixture("workspace_cli");
    let manifest_arg = workspace_manifest.to_str().expect("manifest");
    let check = assert_success(
        run_ghor(&[
            "check",
            "--workspace",
            "--manifest-path",
            manifest_arg,
            "--json",
        ]),
        "ghor check --workspace",
    );
    let check: Value = serde_json::from_str(&check).expect("check json");
    assert_eq!(check["workspace"], true);
    assert!(check["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .any(|package| package["package"] == "workspace_app"));

    let build = assert_success(
        run_ghor(&[
            "build",
            "--workspace",
            "--manifest-path",
            manifest_arg,
            "--json",
        ]),
        "ghor build --workspace",
    );
    assert!(build.contains("\"schema\": \"ghor.build.v1\""));
    assert!(build.contains("\"workspace\": true"));
    assert!(root
        .join("target/ghor/debug/workspace_app.build.json")
        .exists());
    assert!(root
        .join("target/ghor/debug/workspace_tool.build.json")
        .exists());

    fs::create_dir_all(root.join("crates/app/tests")).expect("tests dir");
    fs::write(
        root.join("crates/app/tests/smoke.n"),
        "workspace_app_test {\n    to main {\n        return true;\n    }\n\n    fa:\n        return true;\n}\n",
    )
    .expect("test source");
    let test = assert_success(
        run_ghor(&[
            "test",
            "--workspace",
            "--manifest-path",
            manifest_arg,
            "--json",
        ]),
        "ghor test --workspace",
    );
    let test: Value = serde_json::from_str(&test).expect("test json");
    assert_eq!(test["workspace"], true);
    assert_eq!(test["status"], "ok");
}

#[test]
fn ghor_registry_checksum_mismatch_fail() {
    let package = create_project("registry_bad_checksum_pkg");
    let manifest = package.join("ghor.toml");
    let registry = temp_dir("registry_bad_checksum");
    assert_success(
        run_ghor(&[
            "publish",
            "--manifest-path",
            manifest.to_str().expect("manifest"),
            "--registry",
            registry.to_str().expect("registry"),
        ]),
        "ghor publish checksum registry",
    );
    let consumer = temp_dir("registry_bad_checksum_consumer");
    let consumer_manifest = write_consumer_manifest(
        &consumer,
        "registry_bad_checksum_pkg",
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
    assert_failure_contains(
        run_ghor(&[
            "fetch",
            "--manifest-path",
            consumer_manifest.to_str().expect("consumer manifest"),
            "--registry",
            registry.to_str().expect("registry"),
            "--json",
        ]),
        "ghor fetch checksum mismatch",
        "E_GHOR_CHECKSUM_001",
    );
}

#[cfg(unix)]
#[test]
fn ghor_detects_stale_installed_ncc_ok() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("stale_toolchain");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let stale_ncc = dir.join("ncc");
    std::fs::write(
        &stale_ncc,
        "#!/usr/bin/env sh\nprintf '{\"status\":\"basal_lowering_deferred_machine\",\"dependencies\":{\"machine\":\"blocked_by_machine\"}}\\n'\n",
    )
    .expect("stale ncc");
    let mut perms = std::fs::metadata(&stale_ncc)
        .expect("metadata")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&stale_ncc, perms).expect("chmod");

    let out = Command::new(ghor_bin())
        .args(["doctor", "--json"])
        .env("NC_NO_COLOR", "1")
        .env("GHOR_INSTALLED_NCC", stale_ncc)
        .output()
        .expect("ghor doctor");
    let stdout = assert_success(out, "ghor stale doctor");
    assert!(stdout.contains("E_GHOR_TOOLCHAIN_STALE_001"));
}
