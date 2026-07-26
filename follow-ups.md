# Follow-ups

Work that is deliberately out of scope for the change that surfaced it,
recorded here so it isn't lost.

## Run snapshots inside the isolation boundary

**Problem.** ayb's multi-tenant isolation story is that tenant SQL runs
in `ayb_query_daemon`, a subprocess restricted with Landlock (filesystem
+ network) and setrlimit. Snapshots don't go through that boundary:
`server::snapshots::execution::snapshot_database` calls
`engine.create_snapshot(...)` directly, so the **server process** opens
and parses every tenant's database file on every snapshot interval, with
no sandbox and with the server's secrets (fernet key, S3 credentials,
SMTP password) in its address space.

Two distinct exposures:

1. *Parsing untrusted files.* A tenant fully controls the bytes of their
   database file. Any parser bug in SQLite or DuckDB reachable from
   opening a crafted file is reachable from privileged code, on a timer.
2. *Engine capabilities.* The snapshot path can't run with the query
   path's full safety perimeter, because `ATTACH`/`VACUUM INTO` need
   external file access. DuckDB with `enable_external_access` can read
   and write files (`COPY ... TO '<path>'`) as the server user.

Mitigations already in place (not a fix, just the floor): snapshot SQL
interpolates paths as escaped SQL string literals, entity/database slugs
are charset-validated at the API boundary, and the snapshot connection
disables extension autoload/autoinstall.

**Why it isn't trivially fixed.** The snapshot has to end up in S3, and
the sandboxed daemon is exactly the thing that must not have network
access or credentials. So "just do it in the daemon" trades a
file-parsing exposure for a much worse one.

**Concrete proposal — split the work at the file boundary.** The two
halves have different privilege needs, so give them different processes:

- *Produce* the snapshot file in a sandboxed child. The engine work is
  "read database at path A, write a consistent copy to path B" — no
  network, no credentials, only two paths. That fits the existing
  daemon's Landlock model: allow read-write on the database directory
  and the snapshot directory, deny network, keep the rlimits. It could
  be a new verb on the existing per-database daemon (`{"snapshot":
  {"destination": "..."}}` alongside the current query request) or a
  short-lived `ayb_query_daemon --snapshot` invocation. The per-database
  daemon is the better fit: it already holds the right Landlock scope,
  and routing through it removes the file-lock contention that the
  DuckDB `with_lock_retry` logic exists to paper over, since the daemon
  serializes access to that database.
- *Upload* the finished file from the server process, which is where the
  S3 credentials belong. The server never opens the tenant's database —
  it only reads a file the sandboxed child produced, hashes it, and PUTs
  it. `hash_db_directory` and `SnapshotStorage` already work this way.

Restore is the mirror image and has the same shape: the server downloads
and decompresses (needs credentials), and nothing needs to parse the
file in the server process — the current code already only moves files
and re-points the `current` symlink.

**Payoff.** The server process stops parsing tenant-controlled database
files entirely; the sandbox becomes the only place an engine ever touches
tenant data. It also lets the snapshot connection drop
`enable_external_access` back to a narrower posture, since Landlock —
not DuckDB's own setting — becomes the thing bounding file access.

### Unlocks: persistent connections with explicit handoff

Doing this also removes the reason the query path re-opens the database
on every statement, so the two changes belong together.

Today `query_duckdb` and `query_sqlite` open a connection, run one
statement, and drop it. The daemon amortizes process spawn and sandbox
setup, but *not* connection setup — and for DuckDB that cost is real,
since it probes the host (`/sys/devices/system/cpu/online`,
`/sys/fs/cgroup/...`, `/proc/self/*`) at instantiation to size its thread
pool and buffer pool.

That open/close cycle is currently load-bearing rather than accidental:
it is what leaves the file unlocked between queries so the snapshot job's
`ATTACH` can acquire it. A persistent read-write DuckDB connection would
hold the exclusive file lock for the daemon's entire lifetime and
snapshots would never succeed. **So do not make connections persistent
before moving snapshots into the daemon — in that order it breaks
backups.**

Once snapshots run *through* the daemon, the lock conflict disappears:
the daemon is the single writer for that database and can sequence the
work itself — keep a connection open for queries, close it (or quiesce
it) around the snapshot, reopen after.

**But weigh this against losing engine-enforced read-only.** Today
`query_duckdb` opens with `AccessMode::ReadOnly` for a read-only
request, so such a query physically cannot write: the engine refuses it
(this is what `map_duckdb_error`'s `NoWriteAccessError` and
`test_duckdb_read_only_prevents_writes` encode). That is defense in
depth *behind* the permission layer — a permission bug alone would not
be enough to let a read-only caller write. A single persistent
connection has a single access mode, and to serve writes it must be
read-write, so read-only requests would run on a read-write connection
and permission checks would become the only barrier.

Separate read and write connections do **not** solve this, at least for
DuckDB: `access_mode` is a property of the database *instance*
(it is set on `Config`, passed to `duckdb_open_ext`), `Connection::
try_clone` shares that instance, and opening the file a second time to
get a second instance is itself a lock conflict — including within a
single process, as `test_lock_conflict_is_recognized` demonstrates.
SQLite would allow it (rusqlite sets read-only/read-write per
connection), but building it there alone means an engine-asymmetric
mechanism inside an abstraction meant to hide such differences.

Note also that connection reuse buys no *concurrency* on its own:
`DaemonRegistry::execute_query` holds the per-daemon mutex for the whole
round trip and the daemon is a single-threaded stdin loop, so requests
to one database are serialized regardless of how many connections exist.

Net: treat the security boundary and the deletion of the lock-retry code
as the reasons to move snapshots into the daemon. Persistent connections
are a separate, smaller optimization with a real safety cost — decide it
on its own merits rather than assuming it comes along for free.

It also deletes code rather than adding it. `with_lock_retry`,
`LOCK_RETRY_TIMEOUT`/`LOCK_RETRY_INTERVAL`, and `is_lock_conflict` in
`hosted_db/duckdb.rs` exist only to wait out cross-process lock
conflicts. With one process owning the file they are unnecessary — which
also retires `is_lock_conflict`'s matching on DuckDB's English error text
(the crate exposes no structured code for this; see the unit tests
pinning that behavior).

A narrower alternative was considered and rejected: keep a persistent
*read-only* connection (DuckDB's read lock is shared) and open
read-write connections transiently. It helps the common case without
requiring the snapshot move, but it adds a second connection-state
machine for a partial win, and the handoff above supersedes it.

**Cost.** The daemon protocol grows a second request type, snapshot
errors have to travel back over that protocol, and the Landlock ruleset
must be widened to include the snapshot destination directory (today the
daemon only gets the database directory).

## Lint test code too (`cargo clippy --all-targets`)

**Problem.** `make lint` and the CI "Ensure clippy finds no issues" step
both run `cargo clippy -- -D warnings`, which lints the library and
binaries but *not* test code. Everything under `tests/` (and any
`#[cfg(test)]` module) is therefore unlinted, and lint debt has
accumulated there unnoticed.

**Known instances**, all in `tests/browser_e2e_tests/oauth_flow.rs`:

- `clippy::needless_borrows_for_generic_args` — five occurrences of
  `.post(&format!(...))`, which should be `.post(format!(...))`.
- `clippy::let_and_return` — a `let c = if ... ;` block whose binding is
  returned immediately.

**Proposal.** Change both the Makefile target and the CI step to
`cargo clippy --all-targets -- -D warnings`, and fix the findings above
in the same change. Do it on `main` rather than inside a feature branch:
the findings predate any one feature, and widening the gate will surface
them for whoever's branch happens to run first otherwise.

**Cost.** Slightly longer lint runs (test targets get compiled), and a
one-time cleanup pass. Worth it: test code is where several of ayb's
recent timing and correctness bugs lived, so it benefits from the same
scrutiny as the library.
