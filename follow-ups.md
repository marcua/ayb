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

**Cost.** The daemon protocol grows a second request type, snapshot
errors have to travel back over that protocol, and the Landlock ruleset
must be widened to include the snapshot destination directory (today the
daemon only gets the database directory).
