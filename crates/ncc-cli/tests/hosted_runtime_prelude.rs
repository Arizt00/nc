use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static FIXTURE_ID: AtomicU64 = AtomicU64::new(0);

fn ncc_bin() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_ncc")
        .map(PathBuf::from)
        .expect("CARGO_BIN_EXE_ncc must be set")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must resolve")
}

fn write_fixture(label: &str, source: &str) -> PathBuf {
    let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "ncc_hosted_runtime_prelude_{}_{}_{}",
        std::process::id(),
        id,
        label
    ));
    fs::create_dir_all(&root).expect("temporary nC project must be created");
    let path = root.join(format!("{label}.n"));
    fs::write(&path, source).expect("temporary nC fixture must be written");
    path
}

fn remove_fixture(path: &std::path::Path) {
    let root = path.parent().expect("temporary fixture parent");
    let _ = fs::remove_dir_all(root);
}

fn run_ncc(args: &[String]) -> Output {
    let current_dir = args
        .iter()
        .find(|arg| arg.ends_with(".n"))
        .and_then(|arg| std::path::Path::new(arg).parent())
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(workspace_root);
    Command::new(ncc_bin())
        .current_dir(current_dir)
        .args(args)
        .env("NC_NO_COLOR", "1")
        .output()
        .expect("ncc command must execute")
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn reserve_loopback_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback port must bind");
    listener
        .local_addr()
        .expect("loopback address must resolve")
        .port()
}

#[test]
fn requested_io_and_sys_prelude_executes_without_visible_use() {
    let id = FIXTURE_ID.fetch_add(1, Ordering::Relaxed);
    let fixture_root = std::env::temp_dir().join(format!(
        "ncc_hosted_runtime_material_{}_{}",
        std::process::id(),
        id
    ));
    fs::create_dir_all(&fixture_root).expect("material runtime project must be created");
    let source_path = fixture_root.join("hostedPreludeMaterial.n");
    let first_path = fixture_root.join("material.txt");
    let second_path = first_path.with_extension("renamed.txt");
    let nc_path = |path: &std::path::Path| path.to_string_lossy().replace('\\', "\\\\");
    let registry_key = format!("backend_material_{}_{}", std::process::id(), id);
    let source = format!(
        r#"
hostedPreludeMaterial {{
    run() -> bool {{
        if (!fswrite("{}", "uno\ndos")) return false;
        lines >> array<str> = fread_lines("{}");
        if (lines.len() != 2) return false;
        readable >> bool = permissions::canRead("{}");
        writable >> bool = permissions::canWrite("{}");
        executable >> bool = permissions::canExecute("{}");
        required = permissions::requirePermission("{}");
        if (!readable || !writable || !isOk(required)) return false;
        if (!rename_file("{}", "{}")) return false;
        if (!registry_write("{}", "material-ok")) return false;
        stored >> str = registry_read("{}");
        load >> ft64 = cpu_load();
        now >> int = get_current_time();
        sleep(0);
        wifi >> bool = network::isWiFiConnected();
        if (!delete_file("{}")) return false;
        return stored == "material-ok" && load >= -1.0 && load <= 100.0 && now > 0 &&
            (wifi || !wifi) && (executable || !executable);
    }}

    ref main ->> @fn(run);

    fa:
        return true;
}}
"#,
        nc_path(&first_path),
        nc_path(&first_path),
        nc_path(&first_path),
        nc_path(&first_path),
        nc_path(&first_path),
        nc_path(&first_path),
        nc_path(&first_path),
        nc_path(&second_path),
        registry_key,
        registry_key,
        nc_path(&second_path),
    );
    fs::write(&source_path, source).expect("material runtime fixture must be written");

    let output = run_ncc(&["run".to_string(), source_path.to_string_lossy().to_string()]);
    let rendered = combined(&output);
    let _ = fs::remove_dir_all(&fixture_root);

    assert!(
        output.status.success(),
        "material runtime prelude\n{rendered}"
    );
}

fn wait_child_output(mut child: Child, timeout: Duration) -> Output {
    let deadline = Instant::now() + timeout;
    loop {
        if child
            .try_wait()
            .expect("ncc child status must be readable")
            .is_some()
        {
            return child
                .wait_with_output()
                .expect("completed ncc child output must be readable");
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .expect("timed-out ncc child output must be readable");
            panic!("ncc child exceeded {timeout:?}\n{}", combined(&output));
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn connect_loopback_while_child_runs(child: &mut Child, port: u16) -> Result<TcpStream, String> {
    let deadline = Instant::now() + Duration::from_secs(40);
    loop {
        match TcpStream::connect(("localhost", port)) {
            Ok(stream) => return Ok(stream),
            Err(error) => {
                if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
                    return Err(format!(
                        "nC server exited before accepting loopback traffic: {status}"
                    ));
                }
                if Instant::now() >= deadline {
                    return Err(format!("nC server did not bind localhost:{port}: {error}"));
                }
                thread::sleep(Duration::from_millis(25));
            }
        }
    }
}

#[test]
fn mobile_and_sys_backend_prelude_runs_without_visible_use() {
    let source = workspace_root().join("examples/12_apps/mobile_backend_smoke_no_imports.n");
    let output = Command::new(ncc_bin())
        .current_dir(workspace_root())
        .args(["run", source.to_str().expect("UTF-8 mobile fixture")])
        .env("NC_NO_COLOR", "1")
        .env("NCC_MOBILE_ALLOW_PERMISSIONS", "1")
        .env("NCC_MOBILE_TTS_READY", "1")
        .env("NCC_MOBILE_CALL_STATE", "INCOMING")
        .env("NCC_MOBILE_INCOMING_NUMBER", "+34600000000")
        .env("NCC_MOBILE_HTTP_STATUS", "200")
        .env(
            "NCC_MOBILE_HTTP_JSON",
            r#"{"city":"Las Palmas","country":"Espana","type":"MOBILE"}"#,
        )
        .output()
        .expect("mobile backend prelude must execute");
    let rendered = combined(&output);

    assert!(
        output.status.success(),
        "mobile backend prelude\n{rendered}"
    );
    assert!(
        rendered.lines().any(|line| line.trim() == "Las Palmas"),
        "{rendered}"
    );
}

#[test]
fn network_backend_prelude_runs_server_send_receive_close_over_real_loopback() {
    let port = reserve_loopback_port();
    let path = write_fixture(
        "network_server_loopback",
        &format!(
            r#"
networkServerLoopback {{
    run() -> bool {{
        var endpoint = network::server_open({port});
        if (endpoint == null) return false;
        var request = endpoint::receive();
        var sent = endpoint::send("pong");
        var closed = endpoint::close();
        return request == "ping" && sent == 4 && closed;
    }}

    ref main ->> @fn(run);

    fa:
        return true;
}}
"#
        ),
    );

    let mut child = Command::new(ncc_bin())
        .current_dir(path.parent().expect("network fixture project root"))
        .args(["run", path.to_str().expect("UTF-8 network fixture")])
        .env("NC_NO_COLOR", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("nC loopback server must start");

    let interaction = (|| -> Result<[u8; 4], String> {
        let mut stream = connect_loopback_while_child_runs(&mut child, port)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|error| error.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|error| error.to_string())?;
        stream
            .write_all(b"ping")
            .map_err(|error| error.to_string())?;
        let mut response = [0_u8; 4];
        stream
            .read_exact(&mut response)
            .map_err(|error| error.to_string())?;
        Ok(response)
    })();

    if interaction.is_err() {
        let _ = child.kill();
    }
    let output = wait_child_output(child, Duration::from_secs(10));
    let rendered = combined(&output);
    remove_fixture(&path);

    assert!(output.status.success(), "nC server loopback\n{rendered}");
    assert_eq!(
        interaction.expect("loopback server exchange must pass"),
        *b"pong"
    );
}

#[test]
fn network_backend_prelude_runs_client_send_receive_close_over_real_loopback() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("loopback server must bind");
    listener
        .set_nonblocking(true)
        .expect("loopback server must support nonblocking accept");
    let port = listener
        .local_addr()
        .expect("loopback server address must resolve")
        .port();
    let server = thread::spawn(move || -> std::io::Result<[u8; 4]> {
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "nC client did not connect to loopback server",
                        ));
                    }
                    thread::sleep(Duration::from_millis(20));
                }
                Err(error) => return Err(error),
            }
        };
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        let mut request = [0_u8; 4];
        stream.read_exact(&mut request)?;
        stream.write_all(b"pong")?;
        Ok(request)
    });
    let path = write_fixture(
        "network_client_loopback",
        &format!(
            r#"
networkClientLoopback {{
    run() -> bool {{
        var endpoint = network::client_open("127.0.0.1", {port});
        if (endpoint == null) return false;
        var sent = endpoint::send("ping");
        var response = endpoint::receive();
        var closed = endpoint::close();
        return sent == 4 && response == "pong" && closed;
    }}

    ref main ->> @fn(run);

    fa:
        return true;
}}
"#
        ),
    );

    let output = run_ncc(&["run".into(), path.to_string_lossy().into_owned()]);
    let rendered = combined(&output);
    let request = server
        .join()
        .expect("loopback server thread must not panic")
        .expect("loopback server exchange must pass");
    remove_fixture(&path);

    assert!(output.status.success(), "nC client loopback\n{rendered}");
    assert_eq!(request, *b"ping");
}

#[test]
fn raw_socket_backend_executes_direct_send_recv_over_real_loopback() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("raw loopback server must bind");
    let port = listener.local_addr().expect("raw loopback address").port();
    let server = thread::spawn(move || -> std::io::Result<[u8; 4]> {
        let (mut stream, _) = listener.accept()?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        let mut request = [0_u8; 4];
        stream.read_exact(&mut request)?;
        stream.write_all(b"PONG")?;
        Ok(request)
    });
    let path = write_fixture(
        "raw_socket_loopback",
        &format!(
            r#"
use <memory:stack>;
use <net:net, socket>;

rawSocketMaterial {{
    fn:
        run() -> bool {{
            mark >> p<byte> <- stackMark();
            receive >> p<byte> <- stackAlloc(16, 8);
            started >> int <- netStartup();
            client >> addr <- socketCreate(2, 1, 6);
            connected >> int <- (started == 0 && client != null)
                ? socketConnect(client, "127.0.0.1", {port}, 5000)
                : -1;
            sent >> int <- connected == 0
                ? socketSend(client, adr(b"PING"), 4, 0)
                : -1;
            received >> int <- sent == 4
                ? socketRecv(client, adr(receive), 16, 0)
                : -1;
            ok >> bool <-
                received == 4 &&
                rd(off(receive, 0)) == 80 &&
                rd(off(receive, 1)) == 79 &&
                rd(off(receive, 2)) == 78 &&
                rd(off(receive, 3)) == 71;
            if (client != null) {{
                socketShutdown(client, 2);
                socketClose(client);
            }}
            if (started == 0) {{ netShutdown(); }}
            stackRestore(mark);
            return ok;
        }}

    ref main ->> run;

    fa:
        return true;
}}
"#
        ),
    );

    let output = run_ncc(&["run".into(), path.to_string_lossy().into_owned()]);
    let rendered = combined(&output);
    let request = server
        .join()
        .expect("raw loopback thread must not panic")
        .expect("raw loopback exchange must pass");
    remove_fixture(&path);

    assert!(output.status.success(), "raw socket loopback\n{rendered}");
    assert_eq!(request, *b"PING");
}

#[test]
fn hosted_string_json_prelude_checks_and_runs_without_visible_use() {
    let path = write_fixture(
        "positive",
        r#"
hostedPreludeCli {
    fn:
        run() -> bool {
            public:
                @root >> int : jsonParse("{\"element\":\"Cm\"}");
                @node >> int : jsonObjectGet(@root, "element");
                @upper >> str : strToUpper(jsonGetString(@node));
                println(@upper);
                @matches >> bool : strEq(@upper, "CM");
                @upperReleased >> bool : strRelease(@upper);
                @childReleased >> bool : jsonRelease(@node);
                @rootReleased >> bool : jsonRelease(@root);
                return @matches && @upperReleased && @childReleased && @rootReleased;
        }

    ref main ->> @fn(run);

    fa:
        return true;
}

"#,
    );

    let check = run_ncc(&["check".into(), path.to_string_lossy().into_owned()]);
    let check_rendered = combined(&check);
    assert!(
        check.status.success(),
        "hidden prelude check\n{check_rendered}"
    );

    let run = run_ncc(&["run".into(), path.to_string_lossy().into_owned()]);
    let run_rendered = combined(&run);
    assert!(run.status.success(), "hidden prelude run\n{run_rendered}");
    assert!(
        run_rendered.lines().any(|line| line.trim() == "CM"),
        "{run_rendered}"
    );

    remove_fixture(&path);
}

#[test]
fn compact_backend_prelude_materializes_string_time_and_fs_runtime() {
    let path = write_fixture(
        "compact_backend",
        r#"
compactBackendPrelude {
    showFirst(limit: int) -> void {
        println(strFromInt(limit));
        return 'x';
    }

    boolFromInt() -> bool {
        return 7;
    }

    run() -> bool {
        showFirst(3);
        return boolFromInt() && monotonicMs() >= 0 && exists(".");
    }

    main: run;
}
"#,
    );

    let run = run_ncc(&["run".into(), path.to_string_lossy().into_owned()]);
    let rendered = combined(&run);
    assert!(
        run.status.success(),
        "compact backend prelude run\n{rendered}"
    );
    assert!(
        rendered.lines().any(|line| line.trim() == "3"),
        "{rendered}"
    );

    let manifest = workspace_root().join(".ncc/run").join(format!(
        "{}.native-run.json",
        path.file_stem()
            .and_then(|value| value.to_str())
            .expect("fixture stem")
    ));
    let payload: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).expect("native run manifest must be retained"))
            .expect("native run manifest must be valid JSON");
    assert_eq!(
        payload.get("route").and_then(serde_json::Value::as_str),
        Some("mct-llvm-build-native-fallback")
    );

    remove_fixture(&path);
}

#[test]
fn hosted_root_receives_backend_prelude_without_visible_use() {
    let path = write_fixture(
        "root_negative",
        r#"
rootPreludeCli[@] {
    fn:
        run() -> int {
            public:
                return jsonParse("{}");
        }

    ref main ->> @fn(run);

    fa:
        return true;
}
"#,
    );

    let output = run_ncc(&["check".into(), path.to_string_lossy().into_owned()]);
    let rendered = combined(&output);
    assert!(
        output.status.success(),
        "hosted root backend prelude must resolve\n{rendered}"
    );

    remove_fixture(&path);
}
