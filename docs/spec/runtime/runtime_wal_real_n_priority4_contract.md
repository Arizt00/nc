# Runtime WAL real in nC - Priority 4

Status: implemented as a real multi-process `.n` WAL cluster over the real
runtime filesystem, process and TCP loopback primitives.

Baseical examples:

```bash
ncc run examples/runtime/walLeader.n
ncc run examples/runtime/negative/walLeaderNoMajority.n
```

Follower entrypoints can also be launched directly when debugging:

```bash
ncc run-fn examples/runtime/walFollower.n serveA
ncc run-fn examples/runtime/walFollower.n serveB
```

## Architecture

- `examples/runtime/walLeader.n` is the leader.
- `examples/runtime/walFollower.n` is a long-lived follower server.
- The leader starts two real follower processes with `process.spawn`.
- Followers listen on real TCP loopback sockets:
  - follower A: `127.0.0.1:42111`
  - follower B: `127.0.0.1:42112`
- Every node persists real files under `target/tmp/nc_wal_real_cluster`:
  - `leader/wal.log`
  - `leader/data.tsv`
  - `follower_a/wal.log`
  - `follower_a/data.tsv`
  - `follower_b/wal.log`
  - `follower_b/data.tsv`

## Protocol

The leader writes the transaction to its local WAL as `PENDING` before mutating
data. It then sends the transaction command (`TX1PENDING`) to follower A over
TCP. The follower receives the payload through `tcpRecv`, stores it as
`Result<str,str>`, unwraps it as `str`, and dispatches the WAL action from the
actual message. Follower B is killed through `process.kill` before replication to
prove that the cluster can still commit with majority: leader + follower A.

Once follower A ACKs, the leader appends `COMMITTED`, writes the data file and
fsyncs both files. It then sends `TX1COMMIT` to follower A, restarts follower B
with `process.spawn`, sends `SYNC1` over TCP and follower B rebuilds its WAL/data
files before rejoining. `STOP` is also a real TCP command, not an implicit
connection-order state.

The negative fixture starts no followers, writes only the leader `PENDING` record
and proves that the transaction is not committed without majority.

## Runtime Surface Used

- `process.spawn`
- `process.kill`
- `process.wait`
- `threadSleep`
- `fswrite`
- `fsappend`
- `fsread`
- `fsync`
- `exists`
- `tcpListen`
- `tcpConnect`
- `tcpAccept`
- `tcpSend`
- `tcpRecv`
- `tcpClose`

## Verification

Required checks:

```bash
ncc check examples/runtime/walFollower.n
ncc check examples/runtime/walLeader.n
ncc check examples/runtime/negative/walLeaderNoMajority.n
ncc check examples/runtime/runtime_real_tcp_recv_typed_result.n
ncc run examples/runtime/walLeader.n
ncc run examples/runtime/negative/walLeaderNoMajority.n
ncc run examples/runtime/runtime_real_tcp_recv_typed_result.n
cargo test -p ncc-cli --test wal_runtime_cluster -- --nocapture
```

The older `examples/runtime/wal_real_n_runtime.n` remains as a compact
single-process regression fixture, but it is no longer the final Priority 4
artifact.
