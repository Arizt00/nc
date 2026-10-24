use std::collections::BTreeMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Op {
    Create,
    Update,
    Delete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TxStatus {
    Pending,
    Committed,
}

#[derive(Clone, Debug)]
struct WalRecord {
    tx_id: u64,
    op: Op,
    key: String,
    value: String,
    status: TxStatus,
}

#[derive(Clone, Debug)]
struct TxRequest {
    op: Op,
    key: String,
    value: String,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", red_bold(&format!("error: {err}")));
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || is_help(&args[0]) {
        print_help();
        return Ok(());
    }

    match args[0].as_str() {
        "follower" | "serve-follower" => cmd_follower(&args[1..]),
        "tx" | "transaction" => cmd_tx(&args[1..]),
        "sync" => cmd_sync(&args[1..]),
        "status" => cmd_status(&args[1..]),
        "stop" => cmd_stop(&args[1..]),
        "e2e" | "verify-real" => cmd_e2e(&args[1..]),
        other => Err(format!("unknown nwal command '{other}'").into()),
    }
}

fn print_help() {
    println!(
        "{}",
        cyan_bold("nwal - real WAL + synchronous majority replication")
    );
    println!();
    println!("Usage:");
    println!("  nwal follower --id a --addr 127.0.0.1:7101 --data-dir .nwal");
    println!("  nwal follower --id b --addr 127.0.0.1:7102 --data-dir .nwal");
    println!("  nwal tx --data-dir .nwal --followers 127.0.0.1:7101,127.0.0.1:7102 --op create --key user:1 --value 100");
    println!("  nwal sync --data-dir .nwal --follower 127.0.0.1:7101");
    println!("  nwal status --data-dir .nwal");
    println!("  nwal stop --addr 127.0.0.1:7101");
    println!("  nwal e2e");
    println!();
    println!("What is real here:");
    println!("  - every node has an on-disk wal.log and data.tsv");
    println!("  - WAL writes are flushed with fsync before ACK/commit");
    println!("  - followers communicate through TCP");
    println!("  - leader commits only with majority: leader + one follower OK");
    println!("  - a recovered follower is synced from the leader WAL");
}

fn cmd_follower(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let id = opt_value(args, "--id").unwrap_or_else(|| "a".to_string());
    let addr = opt_value(args, "--addr").unwrap_or_else(|| "127.0.0.1:7101".to_string());
    let data_dir = PathBuf::from(opt_value(args, "--data-dir").unwrap_or_else(|| ".nwal".into()));
    let node_dir = data_dir.join(format!("follower_{id}"));
    ensure_node(&node_dir)?;

    title(&format!("FOLLOWER {id} listening at {addr}"), "magenta");
    println!("  wal  = {}", node_dir.join("wal.log").display());
    println!("  data = {}", node_dir.join("data.tsv").display());

    let listener = TcpListener::bind(&addr)?;
    serve_follower(listener, node_dir)
}

fn cmd_tx(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = PathBuf::from(opt_value(args, "--data-dir").unwrap_or_else(|| ".nwal".into()));
    let followers = parse_followers(
        &opt_value(args, "--followers").unwrap_or_else(|| "127.0.0.1:7101,127.0.0.1:7102".into()),
    );
    let request = TxRequest {
        op: parse_op(&opt_value(args, "--op").unwrap_or_else(|| "create".into()))?,
        key: opt_value(args, "--key").unwrap_or_else(|| "key:1".into()),
        value: opt_value(args, "--value").unwrap_or_else(|| "0".into()),
    };
    let result = submit_tx(&data_dir, &followers, request)?;
    if result {
        Ok(())
    } else {
        Err("transaction was not committed because majority was not reached".into())
    }
}

fn cmd_sync(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = PathBuf::from(opt_value(args, "--data-dir").unwrap_or_else(|| ".nwal".into()));
    let follower = opt_value(args, "--follower").unwrap_or_else(|| "127.0.0.1:7101".into());
    sync_follower(&data_dir, &follower)
}

fn cmd_status(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = PathBuf::from(opt_value(args, "--data-dir").unwrap_or_else(|| ".nwal".into()));
    title("CLUSTER STATUS", "cyan");
    print_node_status("leader", &leader_dir(&data_dir))?;
    for id in ["a", "b"] {
        let dir = data_dir.join(format!("follower_{id}"));
        if dir.exists() {
            print_node_status(&format!("follower_{id}"), &dir)?;
        }
    }
    Ok(())
}

fn cmd_stop(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let addr = opt_value(args, "--addr").ok_or("stop requires --addr <host:port>")?;
    let response = send_request(&addr, "STOP\n", Duration::from_millis(800))?;
    println!("{}", green(&format!("stop {addr}: {}", response.trim())));
    Ok(())
}

fn cmd_e2e(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = match opt_value(args, "--data-dir") {
        Some(raw) => PathBuf::from(raw),
        None => env::temp_dir().join(format!(
            "nwal-real-e2e-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|stamp| stamp.as_nanos())
                .unwrap_or_default()
        )),
    };
    if data_dir.exists() {
        safe_remove_e2e_dir(&data_dir)?;
    }
    fs::create_dir_all(&data_dir)?;

    title("E2E REAL: WAL + TCP + FSYNC", "cyan");
    println!("  data-dir = {}", data_dir.display());

    let follower_a = spawn_follower("a", &data_dir)?;
    let follower_b = spawn_follower("b", &data_dir)?;

    wait_tcp(&follower_a.addr)?;
    wait_tcp(&follower_b.addr)?;

    submit_tx(
        &data_dir,
        &[follower_a.addr.clone(), follower_b.addr.clone()],
        TxRequest {
            op: Op::Create,
            key: "account:1".into(),
            value: "100".into(),
        },
    )?;

    let follower_a_addr = follower_a.addr.clone();
    let follower_b_addr = follower_b.addr.clone();

    send_request(&follower_a.addr, "STOP\n", Duration::from_secs(1))?;
    follower_a.join()?;
    println!("{}", yellow("  follower_a stopped: real network failure"));

    submit_tx(
        &data_dir,
        &[follower_a_addr.clone(), follower_b_addr.clone()],
        TxRequest {
            op: Op::Update,
            key: "account:1".into(),
            value: "250".into(),
        },
    )?;

    let follower_a_restarted = spawn_follower("a", &data_dir)?;
    wait_tcp(&follower_a_restarted.addr)?;
    sync_follower(&data_dir, &follower_a_restarted.addr)?;

    send_request(&follower_a_restarted.addr, "STOP\n", Duration::from_secs(1))?;
    send_request(&follower_b_addr, "STOP\n", Duration::from_secs(1))?;
    follower_a_restarted.join()?;
    follower_b.join()?;

    assert_data_value(&leader_dir(&data_dir), "account:1", "250")?;
    assert_data_value(&data_dir.join("follower_a"), "account:1", "250")?;
    assert_data_value(&data_dir.join("follower_b"), "account:1", "250")?;

    cmd_status(&["--data-dir".into(), data_dir.to_string_lossy().to_string()])?;
    println!(
        "{}",
        green_bold("E2E REAL OK: majority commit and follower recovery completed")
    );
    Ok(())
}

struct FollowerHandle {
    addr: String,
    join: thread::JoinHandle<Result<(), String>>,
}

impl FollowerHandle {
    fn join(self) -> Result<(), Box<dyn std::error::Error>> {
        match self.join.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(err)) => Err(err.into()),
            Err(_) => Err("follower thread panicked".into()),
        }
    }
}

fn spawn_follower(id: &str, data_dir: &Path) -> Result<FollowerHandle, Box<dyn std::error::Error>> {
    let node_dir = data_dir.join(format!("follower_{id}"));
    ensure_node(&node_dir)?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?.to_string();
    let join =
        thread::spawn(move || serve_follower(listener, node_dir).map_err(|err| err.to_string()));
    println!("{}", green(&format!("  follower_{id} listening at {addr}")));
    Ok(FollowerHandle { addr, join })
}

fn serve_follower(
    listener: TcpListener,
    node_dir: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut line = String::new();
        {
            let mut reader = BufReader::new(stream.try_clone()?);
            reader.read_line(&mut line)?;
        }
        let response = handle_follower_request(&node_dir, line.trim());
        stream.write_all(response.as_bytes())?;
        stream.write_all(b"\n")?;
        stream.flush()?;
        if line.trim() == "STOP" {
            break;
        }
    }
    Ok(())
}

fn handle_follower_request(node_dir: &Path, request: &str) -> String {
    match handle_follower_request_result(node_dir, request) {
        Ok(value) => value,
        Err(err) => format!("ERR|{err}"),
    }
}

fn handle_follower_request_result(
    node_dir: &Path,
    request: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let parts = request.split('|').collect::<Vec<_>>();
    match parts.first().copied().unwrap_or_default() {
        "PING" => Ok("OK|PONG".into()),
        "STOP" => Ok("OK|STOP".into()),
        "STATUS" => {
            let wal = read_wal(&wal_path(node_dir))?;
            let committed = wal
                .iter()
                .filter(|row| row.status == TxStatus::Committed)
                .count();
            Ok(format!(
                "OK|STATUS|records={}|committed={committed}",
                wal.len()
            ))
        }
        "APPEND" => {
            if parts.len() != 6 {
                return Err("APPEND requires 5 fields".into());
            }
            let record = WalRecord {
                tx_id: parts[1].parse()?,
                op: parse_op(parts[2])?,
                key: parts[3].to_string(),
                value: parts[4].to_string(),
                status: parse_status(parts[5])?,
            };
            append_or_update_wal(node_dir, record.clone())?;
            Ok(format!("OK|APPEND|{}", record.tx_id))
        }
        "COMMIT" => {
            if parts.len() != 2 {
                return Err("COMMIT requires tx id".into());
            }
            let tx_id = parts[1].parse()?;
            commit_on_node(node_dir, tx_id)?;
            Ok(format!("OK|COMMIT|{tx_id}"))
        }
        other => Err(format!("unknown follower request '{other}'").into()),
    }
}

fn submit_tx(
    data_dir: &Path,
    followers: &[String],
    request: TxRequest,
) -> Result<bool, Box<dyn std::error::Error>> {
    let leader = leader_dir(data_dir);
    ensure_node(&leader)?;
    let tx_id = next_tx_id(&leader)?;
    let pending = WalRecord {
        tx_id,
        op: request.op,
        key: request.key,
        value: request.value,
        status: TxStatus::Pending,
    };

    title(&format!("TX {tx_id} {}", op_to_str(&pending.op)), "cyan");
    phase("1. write leader WAL=PENDING and fsync");
    append_or_update_wal(&leader, pending.clone())?;
    println!("{}", green("  leader WAL persisted"));

    phase("2. send APPEND to followers over TCP");
    let mut acked = Vec::new();
    for follower in followers {
        let msg = encode_append(&pending);
        match send_request(follower, &msg, Duration::from_millis(900)) {
            Ok(response) if response.starts_with("OK|APPEND|") => {
                println!("{}", green(&format!("  follower {follower} ACK")));
                acked.push(follower.clone());
            }
            Ok(response) => {
                println!(
                    "{}",
                    yellow(&format!(
                        "  follower {follower} rejected: {}",
                        response.trim()
                    ))
                );
            }
            Err(err) => {
                println!(
                    "{}",
                    red(&format!("  follower {follower} unavailable: {err}"))
                );
            }
        }
    }

    phase("3. validate majority");
    if acked.is_empty() {
        println!(
            "{}",
            red_bold("  NO COMMITTED: leader + 0 followers is not majority")
        );
        return Ok(false);
    }

    println!(
        "{}",
        green(&format!(
            "  majority reached: leader + {} follower(s)",
            acked.len()
        ))
    );
    commit_on_node(&leader, tx_id)?;
    println!("{}", green("  leader COMMITTED and data applied"));

    phase("4. notify committed followers");
    for follower in &acked {
        match send_request(
            follower,
            &format!("COMMIT|{tx_id}\n"),
            Duration::from_millis(900),
        ) {
            Ok(response) if response.starts_with("OK|COMMIT|") => {
                println!("{}", green(&format!("  follower {follower} COMMITTED")));
            }
            Ok(response) => println!(
                "{}",
                yellow(&format!(
                    "  follower {follower} commit response: {}",
                    response.trim()
                ))
            ),
            Err(err) => println!(
                "{}",
                yellow(&format!(
                    "  follower {follower} commit notice failed: {err}"
                ))
            ),
        }
    }

    println!("{}", green_bold("COMMITTED"));
    Ok(true)
}

fn sync_follower(data_dir: &Path, follower: &str) -> Result<(), Box<dyn std::error::Error>> {
    let leader = leader_dir(data_dir);
    ensure_node(&leader)?;
    let records = read_wal(&wal_path(&leader))?;
    title(
        &format!("SYNC follower {follower} from leader WAL"),
        "magenta",
    );
    for record in records {
        let response = send_request(
            follower,
            &encode_append(&record),
            Duration::from_millis(900),
        )?;
        if !response.starts_with("OK|APPEND|") {
            return Err(format!("append sync failed: {}", response.trim()).into());
        }
        if record.status == TxStatus::Committed {
            let response = send_request(
                follower,
                &format!("COMMIT|{}\n", record.tx_id),
                Duration::from_millis(900),
            )?;
            if !response.starts_with("OK|COMMIT|") {
                return Err(format!("commit sync failed: {}", response.trim()).into());
            }
        }
        println!("{}", green(&format!("  synced tx {}", record.tx_id)));
    }
    println!("{}", green_bold("SYNC OK"));
    Ok(())
}

fn send_request(
    addr: &str,
    request: &str,
    timeout: Duration,
) -> Result<String, Box<dyn std::error::Error>> {
    let socket = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| format!("cannot resolve address '{addr}'"))?;
    let mut stream = TcpStream::connect_timeout(&socket, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    stream.write_all(request.as_bytes())?;
    stream.flush()?;
    let mut response = String::new();
    let mut reader = BufReader::new(stream);
    reader.read_line(&mut response)?;
    Ok(response)
}

fn wait_tcp(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        if send_request(addr, "PING\n", Duration::from_millis(120)).is_ok() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(30));
    }
    Err(format!("timeout waiting for {addr}").into())
}

fn ensure_node(node_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(node_dir)?;
    if !wal_path(node_dir).exists() {
        File::create(wal_path(node_dir))?.sync_all()?;
    }
    if !data_path(node_dir).exists() {
        File::create(data_path(node_dir))?.sync_all()?;
    }
    Ok(())
}

fn append_or_update_wal(
    node_dir: &Path,
    record: WalRecord,
) -> Result<(), Box<dyn std::error::Error>> {
    ensure_node(node_dir)?;
    let path = wal_path(node_dir);
    let mut records = read_wal(&path)?;
    let mut found = false;
    for row in &mut records {
        if row.tx_id == record.tx_id {
            if row.status == TxStatus::Committed && record.status == TxStatus::Pending {
                found = true;
                break;
            }
            *row = record.clone();
            found = true;
            break;
        }
    }
    if !found {
        records.push(record);
    }
    records.sort_by_key(|row| row.tx_id);
    write_wal_atomic(&path, &records)
}

fn commit_on_node(node_dir: &Path, tx_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    ensure_node(node_dir)?;
    let path = wal_path(node_dir);
    let mut records = read_wal(&path)?;
    let mut target: Option<WalRecord> = None;
    for row in &mut records {
        if row.tx_id == tx_id {
            row.status = TxStatus::Committed;
            target = Some(row.clone());
            break;
        }
    }
    let record = target.ok_or_else(|| format!("tx {tx_id} not found in WAL"))?;
    write_wal_atomic(&path, &records)?;
    apply_data(node_dir, &record)
}

fn apply_data(node_dir: &Path, record: &WalRecord) -> Result<(), Box<dyn std::error::Error>> {
    let mut data = read_data(&data_path(node_dir))?;
    match record.op {
        Op::Create | Op::Update => {
            data.insert(record.key.clone(), record.value.clone());
        }
        Op::Delete => {
            data.remove(&record.key);
        }
    }
    write_data_atomic(&data_path(node_dir), &data)
}

fn read_wal(path: &Path) -> Result<Vec<WalRecord>, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path)?;
    let mut out = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let parts = line.split('|').collect::<Vec<_>>();
        if parts.len() != 5 {
            return Err(format!("invalid WAL line {} in {}", idx + 1, path.display()).into());
        }
        out.push(WalRecord {
            tx_id: parts[0].parse()?,
            op: parse_op(parts[1])?,
            key: parts[2].to_string(),
            value: parts[3].to_string(),
            status: parse_status(parts[4])?,
        });
    }
    Ok(out)
}

fn write_wal_atomic(path: &Path, records: &[WalRecord]) -> Result<(), Box<dyn std::error::Error>> {
    let parent = path.parent().ok_or("WAL path has no parent")?;
    fs::create_dir_all(parent)?;
    let tmp = path.with_extension("log.tmp");
    {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)?;
        for row in records {
            writeln!(
                file,
                "{}|{}|{}|{}|{}",
                row.tx_id,
                op_to_str(&row.op),
                row.key,
                row.value,
                status_to_str(&row.status)
            )?;
        }
        file.flush()?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    sync_existing_file(path)?;
    Ok(())
}

fn read_data(path: &Path) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let mut out = BTreeMap::new();
    if !path.exists() {
        return Ok(out);
    }
    for line in fs::read_to_string(path)?.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, '\t');
        let key = parts.next().unwrap_or_default();
        let value = parts.next().unwrap_or_default();
        out.insert(key.to_string(), value.to_string());
    }
    Ok(out)
}

fn write_data_atomic(
    path: &Path,
    data: &BTreeMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let parent = path.parent().ok_or("data path has no parent")?;
    fs::create_dir_all(parent)?;
    let tmp = path.with_extension("tsv.tmp");
    {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)?;
        for (key, value) in data {
            writeln!(file, "{key}\t{value}")?;
        }
        file.flush()?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    sync_existing_file(path)?;
    Ok(())
}

fn sync_existing_file(path: &Path) -> io::Result<()> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?
        .sync_all()
}

fn print_node_status(name: &str, node_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    ensure_node(node_dir)?;
    let wal = read_wal(&wal_path(node_dir))?;
    let data = read_data(&data_path(node_dir))?;
    let pending = wal
        .iter()
        .filter(|row| row.status == TxStatus::Pending)
        .count();
    let committed = wal
        .iter()
        .filter(|row| row.status == TxStatus::Committed)
        .count();
    println!(
        "  {} wal={} committed={} pending={} data_keys={}",
        cyan_bold(name),
        wal.len(),
        green(&committed.to_string()),
        yellow(&pending.to_string()),
        data.len()
    );
    for row in wal {
        println!(
            "    [{}] tx={} op={} key={} value={} status={}",
            gray("wal"),
            row.tx_id,
            magenta(op_to_str(&row.op)),
            yellow(&row.key),
            row.value,
            color_status(&row.status)
        );
    }
    for (key, value) in data {
        println!(
            "    [{}] {} -> {}",
            gray("data"),
            yellow(&key),
            green(&value)
        );
    }
    Ok(())
}

fn assert_data_value(
    node_dir: &Path,
    key: &str,
    expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = read_data(&data_path(node_dir))?;
    let actual = data
        .get(key)
        .ok_or_else(|| format!("missing data key '{key}'"))?;
    if actual != expected {
        return Err(format!("expected {key}={expected}, got {actual}").into());
    }
    Ok(())
}

fn next_tx_id(node_dir: &Path) -> Result<u64, Box<dyn std::error::Error>> {
    let max = read_wal(&wal_path(node_dir))?
        .iter()
        .map(|row| row.tx_id)
        .max()
        .unwrap_or(0);
    Ok(max + 1)
}

fn encode_append(record: &WalRecord) -> String {
    format!(
        "APPEND|{}|{}|{}|{}|{}\n",
        record.tx_id,
        op_to_str(&record.op),
        record.key,
        record.value,
        status_to_str(&record.status)
    )
}

fn parse_followers(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_op(raw: &str) -> Result<Op, Box<dyn std::error::Error>> {
    match raw.to_ascii_lowercase().as_str() {
        "create" | "crear" | "1" => Ok(Op::Create),
        "update" | "actualizar" | "2" => Ok(Op::Update),
        "delete" | "borrar" | "3" => Ok(Op::Delete),
        other => Err(format!("unknown operation '{other}'").into()),
    }
}

fn op_to_str(op: &Op) -> &'static str {
    match op {
        Op::Create => "CREATE",
        Op::Update => "UPDATE",
        Op::Delete => "DELETE",
    }
}

fn parse_status(raw: &str) -> Result<TxStatus, Box<dyn std::error::Error>> {
    match raw.to_ascii_uppercase().as_str() {
        "PENDING" => Ok(TxStatus::Pending),
        "COMMITTED" => Ok(TxStatus::Committed),
        other => Err(format!("unknown tx status '{other}'").into()),
    }
}

fn status_to_str(status: &TxStatus) -> &'static str {
    match status {
        TxStatus::Pending => "PENDING",
        TxStatus::Committed => "COMMITTED",
    }
}

fn color_status(status: &TxStatus) -> String {
    match status {
        TxStatus::Pending => yellow_bold("PENDING"),
        TxStatus::Committed => green_bold("COMMITTED"),
    }
}

fn leader_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("leader")
}

fn wal_path(node_dir: &Path) -> PathBuf {
    node_dir.join("wal.log")
}

fn data_path(node_dir: &Path) -> PathBuf {
    node_dir.join("data.tsv")
}

fn opt_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn is_help(raw: &str) -> bool {
    matches!(raw, "-h" | "--help" | "help")
}

fn safe_remove_e2e_dir(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let raw = path.to_string_lossy();
    if !raw.contains("nwal-real-e2e") {
        return Err(format!("refusing to remove non-e2e directory '{}'", path.display()).into());
    }
    fs::remove_dir_all(path)?;
    Ok(())
}

fn title(text: &str, tone: &str) {
    println!();
    println!(
        "{}",
        color(
            "------------------------------------------------------------",
            tone,
            false
        )
    );
    println!("{}", color(&format!(" {text}"), tone, true));
    println!(
        "{}",
        color(
            "------------------------------------------------------------",
            tone,
            false
        )
    );
}

fn phase(text: &str) {
    println!("{}", cyan_bold(&format!("  > {text}")));
}

fn color(text: &str, tone: &str, bold: bool) -> String {
    if env::var_os("NO_COLOR").is_some() || env::var_os("NC_NO_COLOR").is_some() {
        return text.to_string();
    }
    let code = match tone {
        "red" => 31,
        "green" => 32,
        "yellow" => 33,
        "blue" => 34,
        "magenta" => 35,
        "cyan" => 36,
        "gray" => 90,
        _ => 37,
    };
    if bold {
        format!("\x1b[1;{code}m{text}\x1b[0m")
    } else {
        format!("\x1b[{code}m{text}\x1b[0m")
    }
}

fn red(text: &str) -> String {
    color(text, "red", false)
}

fn red_bold(text: &str) -> String {
    color(text, "red", true)
}

fn green(text: &str) -> String {
    color(text, "green", false)
}

fn green_bold(text: &str) -> String {
    color(text, "green", true)
}

fn yellow(text: &str) -> String {
    color(text, "yellow", false)
}

fn yellow_bold(text: &str) -> String {
    color(text, "yellow", true)
}

fn magenta(text: &str) -> String {
    color(text, "magenta", false)
}

fn cyan_bold(text: &str) -> String {
    color(text, "cyan", true)
}

fn gray(text: &str) -> String {
    color(text, "gray", false)
}
