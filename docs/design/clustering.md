# Design: clustering, placement, and replication

**Status:** draft for discussion
**Date:** 2026-07-06
**Code references:** written against `main` at `70898f6`; updated after
merging `main` at `0dcd201`, which includes DuckDB support (#776: the
`DbEngine` trait in `src/hosted_db/engine.rs`, a db-type-aware daemon, and
unified 256 MB/256 MB/32-fd daemon rlimits). Line numbers will drift;
function/file names are the stable pointers.

This document proposes how to scale ayb from one stateful server to many
nodes: how databases are placed on nodes, how requests are routed, how a
node failure is detected and recovered from, how much data a failure can
lose, and how the same machinery serves both SQLite and DuckDB. It also
records the results of chunk-stability experiments (see
`docs/design/experiments/chunk_stability/`) that ground several of the
design choices, and ends with a phased implementation plan.

## Summary

- **Placement groups** (by default, one per entity) are the unit of
  scheduling, failover, and resource limits. A group is pinned to exactly
  one node at a time by a **lease with a fencing epoch stored in the
  existing metadata database** (Postgres, which the roadmap already
  designates for multi-node coordination).
- **Every node can serve any request**: non-owners transparently proxy to
  the owner using a cached routing table. There is no raft, no gossip, no
  external coordination service — cluster mode requires exactly the
  dependencies ayb already has (Postgres + S3).
- **Durability comes from shipping deltas to S3 from the query daemon**,
  which already executes every write and therefore knows every commit and
  every quiescent moment. Databases are stored in S3 as **content-addressed
  chunks + manifests**; every ship is logically a full snapshot and
  physically an increment. SQLite and DuckDB ride the same mechanism.
- **Failover** = lease expiry → any node claims the group via one
  conditional `UPDATE` (epoch++) → rehydrates from S3 lazily. Crash RTO ≈
  lease TTL + seconds; planned-drain RTO ≈ 0. RPO is a per-database
  durability tier: snapshot-interval → seconds (default) → zero (opt-in
  commit gate).
- Correctness does not depend on the lease: a deposed owner is **fenced at
  the data layer** by compare-and-swap on a tiny manifest-pointer object in
  S3 (conditional writes are GA on AWS S3, MinIO, and R2).

This is the shape the industry converged on in 2024–2026 — Litestream v0.5
(S3-lease + page shipping), Turso's diskless cloud, and Cloudflare Durable
Objects all landed on "single writer + fencing token + object storage,"
not shared volumes or per-database consensus.

## 1. Where ayb is stateful today

An inventory of the current single-node coupling (verified against `main`
at `70898f6`):

**Already node-agnostic (no work needed):**

- Auth and sessions. API tokens validate against the metadata DB plus the
  shared fernet key (`src/server/tokens.rs`); the web "session" cookie is
  just `entity:token` (`src/server/ui_endpoints/auth.rs`). Any node with
  the same `database_url` and `fernet_key` validates any request.
- Every API/UI endpoint except three touches only the metadata DB and/or
  S3 (details, permissions, tokens, sharing, OAuth, registration).
- Snapshots in S3 are keyed `{prefix}/{entity}/{database}/{blake3-hash}`
  with no node identity (`src/server/snapshots/storage.rs`), and
  `restore_snapshot` already restores onto a node that has never seen the
  database. S3 restore is the ready-made cross-node data-movement
  primitive.
- The web UI is an HTTP client of the local API
  (`init_ayb_client` → `local_base_url`, `src/server/ui_endpoints/auth.rs`),
  so once the API layer routes across nodes, the UI inherits it.

**Node-local state and single-node assumptions (the work):**

1. Database files live under
   `{data_path}/databases/{entity}/{database}/{uuidv7}/` with a `current`
   symlink (`src/hosted_db/paths.rs`). Created locally by
   `create_database`; resolved locally by `query`; a missing file is a
   hard error (`fs::canonicalize`), with no rehydration from S3.
2. One long-lived sandboxed `ayb_query_daemon` process per database file,
   tracked in an in-memory registry keyed by path
   (`src/hosted_db/daemon_registry.rs`). Queries to a database are fully
   serialized by a mutex held across the stdin/stdout round-trip. There is
   no idle reaping, no query/IPC timeout, and shutdown is best-effort
   `try_lock` — no forced drain.
3. The snapshot scheduler discovers databases by walking the local
   filesystem, not the metadata DB (`src/server/snapshots/execution.rs`),
   deduplicates by listing S3 per database per interval, runs `VACUUM INTO`
   through a second, unsandboxed connection that bypasses the daemon, and
   prunes based on its own view of the S3 listing (races across nodes).
4. Config mixes per-node values (`host`, `port`, `data_path`) with
   cluster-wide values (`public_url`, `database_url`, `fernet_key`, `email`,
   `snapshots`) in one flat file (`src/server/config.rs`).

So "shard ayb" decomposes into: **placement + routing + a durability story
tighter than interval snapshots**, plus mechanical fixes to items 2–4.

## 2. Goals and non-goals

Goals:

- Single binary; cluster mode adds **zero dependencies beyond Postgres and
  S3** (both already supported).
- One database has one writer at a time; a database never spans machines
  (matches the roadmap's clustering item).
- SQLite and DuckDB are served by the same placement/replication machinery;
  DuckDB is not a second-class citizen.
- Placement groups: multiple databases for one entity share a daemon and a
  fixed resource envelope.
- Design headroom to tens of thousands–low millions of placement groups.
- Bounded, configurable data loss (RPO) and failover time (RTO), including
  an opt-in zero-data-loss mode.

Non-goals:

- Multi-writer databases or cross-node write scaling for a single database
  (rqlite/dqlite/marmot territory; different consistency model).
- Building or embedding a consensus protocol. Postgres arbitrates the
  control plane; S3 conditional writes fence the data plane.
- A replicated low-latency log service (Neon safekeepers / Cloudflare SRS).
  It is the only way to get RPO=0 *without* paying S3 latency on the commit
  path, and it is explicitly out of scope; the opt-in RPO=0 tier pays S3
  latency instead.
- Shared/network volumes (ZeroFS, NFS, EBS multi-attach) as the core
  mechanism. See §7.

## 3. Architecture

### 3.1 Placement groups

A **placement group** is the unit of placement, failover, resource limits,
and (eventually) billing.

- Default: one group per entity. The schema allows more than one group per
  entity later (hot databases split out; paid tiers with dedicated groups).
- `placement_groups(id, entity_id, node_id, epoch, state, resource_class, …)`;
  `databases.placement_group_id`.
- **One daemon per group**, serving all databases in the group. The daemon
  protocol gains a request id and a database field (today the database path
  is `argv[1]`, one process per file). Landlock scope widens from the
  single database's directory to the entity directory — the on-disk layout
  already matches. The daemon's rlimits (later: cgroup) become the group's
  resource envelope: "pay for a fixed bundle of resources, collocate N
  databases in it."
- Group daemons are spawned lazily and reaped when idle (scale-to-zero per
  tenant). This is also what makes mass failover cheap: a failed-over cold
  group is just a cold group.

### 3.2 Control plane (Postgres)

All coordination state lives in the metadata DB. Cluster mode requires it
to be Postgres; single-node SQLite-metadata deployments keep working
unchanged with clustering off.

- **`nodes`**: id, advertise address, capacity, version, `last_heartbeat`.
  Each node updates its row every ~5s. This is the entire failure-detection
  mechanism — no gossip, no node-to-node health mesh.
- **Leases are per node, not per group** (the Kubernetes node-lease trick):
  renewal traffic is O(nodes), not O(groups). A group assignment is valid
  iff the owning node's lease is live. Group rows change only on movement.
- **Fencing epoch**: each group row carries a monotonically increasing
  `epoch`, bumped on every reassignment. Takeover is one statement:

  ```sql
  UPDATE placement_groups
     SET node_id = $me, epoch = epoch + 1
   WHERE id = $group
     AND node_id = $dead_node
     AND (SELECT last_heartbeat FROM nodes WHERE id = $dead_node)
         < now() - $ttl
  ```

  Postgres serializes racers; exactly one wins. Lease expiry is evaluated
  against the *database server's* clock, sidestepping node clock skew.
  (Deliberately not Postgres advisory locks: those are tied to session
  lifetime, which interacts badly with poolers and half-open connections.)
- **Routing table** = the group assignment rows, cached in each node with a
  short TTL plus `LISTEN/NOTIFY` invalidation. Misrouted request →
  re-read authoritative row → retry once.
- **Manager is a janitor, not a router**: a singleton lease (same
  primitive) for reaping dead nodes' groups, rebalancing, snapshot
  retention/GC, and capacity-aware placement of new groups. Every node can
  be manager; at most one is, briefly. Failover does not depend on the
  manager: any node fielding a request for an expired-lease group may take
  it over (traffic-driven takeover).
- **Postgres outage behavior** (documented semantics): owners keep serving
  on their current lease until TTL, then stop accepting writes
  (fail-stop); cached routing and metadata caches (stale-if-error) keep
  reads flowing. An outage longer than the TTL is a cluster-wide write
  pause until Postgres returns. This is the standard price of a control
  plane (LiteFS paid it to Consul); managed-HA Postgres makes it rare.

### 3.3 Fencing: leases for liveness, S3 CAS for safety

A lease anywhere only provides *liveness* ("who should be writing"). The
*safety* property — a deposed owner's writes can never land — is enforced
where the bytes go:

- Every database has a tiny **manifest pointer** object in S3. Publishing a
  new manifest requires `If-Match` on the pointer's ETag and carries the
  writer's epoch; a zombie's CAS fails no matter what it believes about its
  lease (Kleppmann's fencing-token rule, applied to the data path).
- **Chunks need no fencing at all.** They are content-addressed and
  immutable: a zombie uploading chunks produces harmless, unreferenced
  garbage. Only the pointer object is guarded. This keeps the hot upload
  path fence-free and makes the fencing surface one CAS per ship.
- S3 conditional writes (`If-None-Match` create, Aug 2024; `If-Match` ETag
  CAS, Nov 2024) are GA on AWS and supported by MinIO (ayb's test stack)
  and Cloudflare R2.

### 3.4 Data plane: chunks + manifests

Databases are replicated to S3 as **fixed-size, offset-aligned chunks**
plus **manifests**:

- A manifest lists the chunk hashes composing the database file(s) at a
  quiescent point, with enough metadata to reassemble (chunk size, file
  sizes, engine, epoch, timestamp).
- **Chunk pool is scoped per database** (`{prefix}/{entity}/{db}/chunks/{hash}`),
  not global: GC blast radius stays contained, and there is no cross-tenant
  dedup side channel. Manifests reference chunks by **pure content hash**,
  with hash→key resolution as a separate policy layer — this makes pool
  scoping (per-database now; per-fork-family later; see §4.6) a reversible
  policy decision, not a storage-format commitment.
- **Chunk sizes per engine** (from the experiments in §5): 64 KiB for
  SQLite, 1 MiB for DuckDB. Files at or below one chunk inline their chunk
  list into the pointer object — the small-file path costs one PUT, like
  today.
- The chunk pool is **generation-agnostic**; only manifests/pointers are
  epoch-scoped. A new owner after failover typically uploads ~nothing.
- Per-chunk zstd compression (slightly worse ratio than whole-file; buys
  random access and dedup-compatibility).

**The shipping loop** (per group, supervised next to the daemon):

1. The daemon marks a database dirty when a write transaction completes
   (it executes every write — no filesystem watching, no WAL polling
   daemons).
2. A debounce elapses (default ~2–5s for SQLite, ~10–30s or a
   bytes-since-checkpoint threshold for DuckDB — frequent checkpoints
   amplify DuckDB's row-group rewrites; §5).
3. Under the group's serialization (a guaranteed-quiescent instant —
   queries are already serialized): checkpoint
   (`PRAGMA wal_checkpoint(TRUNCATE)` / `CHECKPOINT`), identify changed
   chunks (blake3 full-file hash: measured 4.7 GB/s single-thread, so
   ~30–60ms at a 256 MB cap; or, for SQLite, read the WAL's frame headers
   *before* checkpointing — the dirty page list in microseconds), stage
   the changed bytes.
4. Off-lock: upload missing chunks, then CAS the manifest pointer.

There is **no "full vs incremental" decision**: every ship runs the same
path; upload size is emergent (churn since last ship). The only
full-upload events are the first ship and a deliberate defrag/rebaseline.

**Producers must be checkpoint-then-copy, not rebuilders.** `VACUUM INTO`
(SQLite) and `COPY FROM DATABASE` (DuckDB) — the shipped snapshot
methods behind the `DbEngine` trait since #776 — rewrite/repack the file and are chunk-catastrophic
(measured: 99.7% / 97.7% churn around 100-row / 10-row changes; §5). Both
are kept as rare, deliberate maintenance operations (defrag + fresh
baseline + integrity re-verification), not as the shipped artifact.

**Stop-the-world budget** (measured, §5): checkpoint (ms-scale SQLite,
0–50ms DuckDB) + hash (~30–60ms at 256 MB) + staging memcpy of changed
bytes. Well under ~100ms per debounce cycle typical; uploads never hold
the lock. On reflink filesystems the copy/freeze is ~1ms and hashing moves
off-lock too.

### 3.5 Snapshots: retained manifests

Snapshots stop being *the backup mechanism* and become *retained
manifests* — but they remain load-bearing:

- **Restore bound:** failover restores "manifest + chunks," O(database
  size), not O(write history).
- **Storage bound:** retention policy decides which manifests are kept;
  GC (mark manifests → sweep unreferenced chunks, per-database pool) is a
  manager job. Generation-scoped manifest prefixes make retiring a
  superseded generation a prefix delete.
- **Oops/corruption firewall:** replication faithfully ships `DROP TABLE`
  and page corruption; history is the undo. Retained manifests are the
  user-visible snapshot/versioning/PITR surface — the existing
  `list_snapshots`/`restore_snapshot` UX maps onto "list/restore retained
  manifests."
- **Verification anchor:** periodically reassemble from chunks and run
  `PRAGMA integrity_check` (as the snapshot path does today) to catch
  replication drift. Never trust the delta chain alone.
- Scheduling flips from wall-clock cron over all databases to
  **activity-driven retention**: idle databases (most of a large fleet)
  cost zero; the current walk + per-database S3 LIST + unconditional
  VACUUM disappears.

### 3.6 Durability tiers

Per-database (or per-group) setting:

| Tier | Mechanism | RPO | Commit latency added | Phase |
|---|---|---|---|---|
| T0 | retained manifests on long interval | snapshot interval | none | exists (recast) |
| T1 (default) | debounced chunk shipping | seconds | none | 2 |
| T2 | SQLite WAL-segment shipping (page deltas, TXIDs) | ~1s, PITR to txn | none | 3 |
| T3 (opt-in) | commit gate: ack after S3 CAS append | 0 | ~50–100ms std S3 / single-digit ms S3 Express | 4 |

Honest caveat for T1/T2 (same trade LiteFS and Litestream async make): in
a network partition, a not-yet-fenced owner can acknowledge writes that
never ship before takeover. The window is bounded by debounce + lease TTL;
a write-path lease-margin check shrinks it; T3 eliminates it (an unshipped
write is an unacked write — the zombie's CAS fails, so the client never
got a 200).

### 3.7 Request routing

- Any load balancer in front (round-robin is fine); every node routes.
- Non-owner nodes proxy to the owner over keep-alive connections (one
  in-DC hop). A redirect/`Fly-Replay`-style header for smart clients can
  come later.
- The query path checks a cached "am I still owner?" with a safety margin
  against the lease; enforcement remains the S3 CAS (§3.3). Reads on a
  zombie can be stale up to ~TTL during a partition; writes are never
  forked.
- `create_database` becomes: metadata row + placement decision
  (least-loaded node for the entity's group, or the group's current owner);
  the file is instantiated on the owner, not the receiving node.

### 3.8 Failover walkthroughs

**Crash:** heartbeats stop → TTL lapses (~15–30s) → next request (or
manager sweep) claims the group (epoch++) → owner rehydrates: fetch
pointer + manifest, fetch chunks in parallel (≤256 MB rlimit cap ⇒ ~1–3s),
replay any T2 segments → serve. **RTO ≈ TTL + seconds; RPO per tier.**
Groups nobody asks for stay cold — a node death with 30k groups is not
30k restores, it is lease markers plus lazy rehydration with jittered,
rate-limited background warm-up for recently-active groups.

**Planned drain (deploy/rebalance):** mark node draining → per group:
stop accepting, finish in-flight with a bounded drain (requires the
query timeout and forced-shutdown fixes from Phase 0), final ship,
release lease → successor claims immediately. **RTO ≈ 0, RPO = 0.**

The roadmap's sessions/transactions item raises the stakes on drain
(open transactions die with the daemon) — the daemon protocol should be
designed with session pinning in mind (§9).

### 3.9 Concurrency model (now and later)

Today's one-query-at-a-time-per-database is an implementation artifact of
the lockstep daemon protocol, not a design necessity:

1. Multiplex the daemon protocol (request ids) → N read connections + 1
   write connection per SQLite database (WAL readers coexist with the
   writer natively). Read concurrency is purely a protocol change.
2. DuckDB's in-process MVCC then also lifts *write* serialization within a
   daemon.
3. SQLite writes stay serialized per database indefinitely (SQLite's
   model; `BEGIN CONCURRENT` never landed in mainline).
4. Cross-node, single-writer-per-database is the invariant the lease/
   fencing design protects. Read scale-out without violating it: any node
   can serve stale reads from manifest + lazily fetched chunks
   (Litestream-VFS pattern) — future work.

## 4. Replication substrate details

### 4.1 S3 layout

```
{prefix}/{entity}/{database}/
  pointer                      # tiny; CAS-guarded; epoch + manifest ref
                               # (inlines chunk list for ≤1-chunk files)
  manifests/{epoch}/{txid-or-seq}.manifest
  chunks/{content-hash}        # immutable, fence-free, per-DB pool
  wal/{epoch}/{txid}.seg       # T2/T3 only (SQLite page-delta segments)
```

Per-database prefixes spread S3 request-rate limits (~3,500 PUT/s,
5,500 GET/s per prefix, auto-scaling) by construction.

### 4.2 Chunk-size choice (from §5 data)

- DuckDB is nearly chunk-size-insensitive — its churn unit is the
  ~0.5–1 MB row-group×column segment (23% churn at 4 KiB vs 26% at 1 MiB
  for a scattered update). **Use 1 MiB.**
- SQLite is highly sensitive under scattered row updates (~29 rows/page ⇒
  1% of rows dirties ~30% of pages; 0.1%-of-rows update: 3% churn at
  4 KiB, 42% at 64 KiB, 100% at 1 MiB). **Use 64 KiB**, and treat the T2
  page-delta path as the real fix for hot scattered-update databases.
- Manifests record their chunk size, so constants can change per engine or
  per database without a format break.

### 4.3 Costs (orders of magnitude)

- Full 256 MB baseline at 64 KiB chunks: 4,096 PUTs ≈ $0.02; restore is
  4,096 parallel GETs ≈ $0.002 + a few seconds.
- Steady-state T1 ship: a handful of PUTs per debounce window per *hot*
  database; idle databases cost zero (contrast: today's scheduler LISTs S3
  per database per interval — ~$120/day at 1M databases on an hourly
  cadence, before the wasted VACUUM I/O).
- T3 on S3 Express One Zone: PUT ≈ $1.13/M after the April 2025 price
  cuts; 100 commits/s ≈ $10/day. Standard S3: $5/M PUTs and 50–100ms
  commit latency.

### 4.4 GC and retention

- Retention policy chooses which manifests survive (e.g., every ship for
  1h, hourly for 24h, daily for 30d, plus user-pinned snapshots).
- GC = mark chunks referenced by retained manifests of that database
  (bounded, per-pool), sweep the rest. Runs as a manager job with the
  usual "in-flight ship" grace window. This is the genuinely fiddly part
  of CAS storage; budget review time accordingly.

### 4.5 Defrag/rebaseline

Occasional deliberate `VACUUM INTO` / `COPY FROM DATABASE` (compaction,
fragmentation, format upgrades) intentionally rewrites the file; the next
ship uploads a mostly-fresh chunk set and older chunks age out via
retention. Schedule rarely and off-peak.

### 4.6 Forking and versioning synergies

- Fork = copy or share a manifest. With per-database pools, fork is a
  burst of server-side `CopyObject`s (256 MB @ 64 KiB ≈ 4,096 copies ≈
  $0.02, seconds, no bytes through nodes) and duplicated storage.
- If forking of large public datasets becomes first-class (the
  GitHub-for-data vision), introduce **fork-family pools** scoped to the
  lineage root: forks share chunks, GC marks across the family, and chunk
  sharing exactly mirrors provenance (everyone in the family had read
  access at fork time), so no cross-tenant dedup oracle appears. A lazy
  variant (fork references the parent pool read-only, copy-on-write into
  its own) gives instant forks. Reversible precisely because manifests
  use pure content hashes (§3.4).
- Global pools are ruled out: cross-stranger dedup wins are negligible,
  and global GC's failure mode is everyone's problem.
- Documented semantic (same as git): a fork retains chunks even if the
  upstream database is later deleted or made private.
- The roadmap's import/export item falls out too: export = reassemble
  from a manifest; import = ship a provided file as a first manifest.

## 5. Experimental results: chunk stability

Reproducible harness: `docs/design/experiments/chunk_stability/`
(measured 2026-07-06; Python `sqlite3`, DuckDB 1.5.4; SQLite 35 MB /
300k rows, ~29 rows per 4 KiB page; DuckDB 50–64 MB / 2M rows ≈ 17 row
groups; fixed-offset chunk compare; producer = checkpoint-then-copy
unless noted).

| Workload (logical Δ) | 4 KiB | 64 KiB | 256 KiB | 1 MiB |
|---|---|---|---|---|
| SQLite scattered UPDATE 1% of rows (~0.4 MB) | 10.4 MB / 30% | 35 MB / 100% | 35 MB / 100% | 36 MB / 100% |
| SQLite scattered UPDATE 0.1% | 1.2 MB / 3% | 14.8 MB / 42% | 30.7 MB / 87% | 35.7 MB / 100% |
| SQLite clustered UPDATE 1% | 0.35 MB / 1% | 0.39 MB / 1% | 0.79 MB / 2% | 1.1 MB / 3% |
| SQLite append INSERT 1% | 0.38 MB | 0.66 MB | 1.6 MB | 5.2 MB |
| SQLite scattered DELETE 1% + reinsert | 10.8 MB / 30% | 35.5 MB / 99% | 35.9 MB / 100% | 36.7 MB / 100% |
| SQLite UPDATE 10 rows | 0.03 MB | 0.07 MB | 0.26 MB | 1.1 MB |
| SQLite DELETE 100 early rows | 0.03 MB | 0.26 MB | 0.52 MB | 2.1 MB |
| DuckDB scattered UPDATE 1% (one column) | 14.5 MB / 23% | 14.7 MB / 23% | 15.2 MB / 24% | 16.8 MB / 26% |
| DuckDB clustered UPDATE 1% | 1.3 MB / 2% | 1.4 MB / 2% | 1.6 MB / 3% | 2.1 MB / 3% |
| DuckDB UPDATE 10 scattered rows | 6.6 MB / 10% | 6.9 MB / 11% | 7.9 MB / 12% | 10.5 MB / 16% |
| DuckDB append INSERT 1% | 0.55 MB | 0.66 MB | 1.1 MB | 3.2 MB |
| DuckDB clustered DELETE 1% (20k rows) | 0.03 MB | 0.13 MB | 0.52 MB | 2.1 MB |

Producer stability:

| Producer test | Changed |
|---|---|
| checkpoint+copy, zero writes between copies (both engines) | **0%** |
| SQLite `VACUUM INTO` before/after UPDATE of 10 rows | 0.1% (deterministic under in-place updates) |
| SQLite `VACUUM INTO` before/after DELETE of 100 early rows | **99.7%** (repacking ripples everything downstream) |
| DuckDB `COPY FROM DATABASE` before/after 10-row UPDATE | **97.7%** (rebuild is not even deterministic) |

Timings: SQLite checkpoint+copy 141ms at 35 MB (copy-bound); DuckDB
checkpoint 0–50ms, copy 22ms at 64 MB; blake3 4.7 GB/s single-thread /
9.2 GB/s multi (⇒ ~30–60ms to hash a 256 MB file under the lock).

Findings the design relies on:

1. **Checkpoint-then-copy is byte-stable and tracks physical churn**; the
   rebuilders (`VACUUM INTO`, `COPY FROM DATABASE`) are unusable as
   shipping producers. (The `DbEngine::create_snapshot` implementations
   from #776 need this swap.)
2. **SQLite churn floor = pages touched**, ~10× the fraction of rows for
   scattered updates. Small chunks (64 KiB) or page-granular T2 segments
   are the mitigations; clustered/append/tiny writes diff beautifully at
   any chunk size.
3. **DuckDB is friendlier than feared**: columnar storage rewrites only
   the touched column's segments; deletes are nearly free (deletion
   vectors); appends are clean; checkpoint+copy is idempotent. Its wart
   is the coarse churn floor (~1 MB per touched row-group×column — 10
   scattered single-row updates cost 6.6 MB), which is engine-inherent
   write amplification, not a chunking artifact; it argues for DuckDB's
   longer debounce.
4. DuckDB's file grew once under update load (49→64 MB) then stabilized
   (free-block reuse) — watch at larger scale.

Experiments still to run before freezing constants: tables with secondary
indexes (index-page scatter will raise SQLite churn; the test table was
PK-only), blob/overflow pages, multi-GB files, DuckDB file-size steady
state under sustained updates, and DuckDB storage determinism across
version upgrades (re-run the harness per release while their format
evolves).

## 6. Scale analysis (10⁴–10⁶ groups)

Rules that keep the design from tipping over, and what breaks without
them:

- **No per-group periodic work.** Heartbeat per node; leases per node;
  shipping and retention driven by write activity. (1M groups ×
  10s-renewals would be 100k writes/s; per-node it is hundreds.)
- **Kill the FS-walk scheduler** (§3.5): today's design does per-database
  S3 LIST + VACUUM + full-file hash per interval regardless of activity —
  ~$120/day of LISTs alone at 1M databases hourly, plus wasted I/O.
  Dirty-tracking makes idle databases free; the snapshot index moves to
  Postgres (also fixing cross-node prune races).
- **Failover storms:** lazy activation + jittered warm-up + restore
  admission control (cap concurrent S3 GETs); prioritize by recency from
  shipping metadata.
- **Postgres hot path:** per-request `get_database` + permission lookups
  bottleneck before anything else at high QPS — short-TTL read-through
  caches keyed off the routing epoch (permission changes tolerate seconds
  of staleness); watch pool counts (20/node today; hundreds of nodes ⇒
  pgbouncer or smaller pools). 1M placement rows are trivial as data.
- **S3:** per-database prefixes spread rate limits; object-count growth is
  handled by retention/GC; batch WAL segments per *group* (not per
  database) if T2 write fan-out grows.
- **Daemon/process counts:** group daemons + idle reaping (a node hosting
  30k groups cannot hold 30k live processes; it can hold the ~hundreds
  that are active).
- **Manager scans:** index group rows by node; reap incrementally; the
  manager role shards by hash range if a single janitor ever lags —
  same lease primitive, N janitor leases.

## 7. Alternatives considered

**ZeroFS / SlateDB-backed volumes (S3-backed NBD/NFS; ZFS on top).**
Genuinely interesting and correctly fenced (SlateDB writer-epoch CAS;
explicit leader/standby handoff in seconds). Rejected as the core
mechanism: (a) physics — a durably-acked fsync must reach S3 whatever
layer does it, so either commits pay 50–300ms (NBD FLUSH to standard S3)
or sync is relaxed and the loss window returns, now with a filesystem +
NBD + ZFS between ayb and its data; (b) failover granularity is the
volume (node-grain, cold caches), not the placement group; (c) a young
storage engine plus kernel-adjacent ops under every tenant byte
contradicts the single-binary ethos. Nothing prevents an operator from
putting `data_path` on it. ([ZeroFS](https://github.com/Barre/ZeroFS),
[SlateDB manifest RFC](https://github.com/slatedb/slatedb/blob/main/rfcs/0001-manifest.md))

**S3-only coordination (no Postgres).** Conditional-write leases on S3
are now a proven pattern (Litestream v0.5.8's `lock.json`;
[Morling's write-up](https://www.morling.dev/blog/leader-election-with-s3-conditional-writes/)).
But clustering already requires a shared metadata DB for auth/entities,
and the routing table is a query workload (point lookups, scans by node,
transactions coupling epoch+assignment, notify) that S3 cannot serve.
S3-CAS remains in the design where it is strongest — data-path fencing —
and an S3 lease store stays a clean swap-in if a Postgres-free cluster
mode ever matters.

**Litestream as a sidecar.** Closest off-the-shelf option for SQLite
(LTX page shipping, S3 leases, PITR), but it is a Go daemon (embeddable
only from Go), SQLite-only, and would put the critical replication loop
outside the binary. Its *design* is the blueprint for T2; Rust prior art
to mine: [walrust](https://github.com/russellromney/walrust),
[haqlite](https://github.com/russellromney/haqlite),
[verneuil](https://github.com/backtrace-labs/verneuil) (all experimental).

**Raft-replicated SQLite (rqlite/dqlite) or per-group consensus.** Mature
for one database/cluster; a poor fit for fleets of thousands–millions of
small single-writer databases (per-tenant raft groups are exactly what
they don't give), and it adds a consensus implementation to the binary.

**DuckLake as the DuckDB story** (catalog in SQL DB + Parquet on S3;
v1.0 April 2026). Attractive future *database type* — catalog in ayb's
Postgres, data in ayb's bucket, near-stateless nodes — but it hosts
DuckLake tables, not arbitrary `.duckdb` files, so it complements rather
than replaces file-level replication. ([DuckLake 1.0](https://ducklake.select/2026/04/13/ducklake-10/))

**Ecosystem reference points** informing this design:
[Litestream revamped](https://fly.io/blog/litestream-revamped/) /
[v0.5.0](https://fly.io/blog/litestream-v050-is-here/) (and the LiteFS →
Litestream trajectory away from FUSE+Consul),
[Turso's diskless cloud](https://turso.tech/blog/turso-cloud-goes-diskless)
(S3 Express on the commit path),
[SQLite in Durable Objects](https://blog.cloudflare.com/sqlite-in-durable-objects/)
(placement + quorum log + output gates; the closest analog to "millions
of tiny single-writer DBs"), Neon's WAL/pages separation,
[MotherDuck differential storage](https://motherduck.com/blog/differential-storage-building-block-for-data-warehouse/),
[Cloud Backed SQLite](https://sqlite.org/cloudsqlite) (the SQLite team's
own block+manifest design), [Graft](https://github.com/orbitinghail/graft),
[S3 conditional writes](https://aws.amazon.com/about-aws/whats-new/2024/11/amazon-s3-functionality-conditional-writes/).

## 8. Plan

Each phase is independently shippable; none is throwaway. Later phases
refine RPO/RTO and scale; earlier phases already improve single-node ayb.

### Phase 0 — single-node groundwork

1. **Placement groups in metadata**: `placement_groups` table,
   `databases.placement_group_id`, default group per entity on creation
   (backfill migration for existing databases).
2. **Group daemon**: protocol v2 (request id + database field, JSON lines
   as today), registry keyed by group, Landlock scoped to the entity
   directory, per-group rlimits; lazy spawn (exists) + idle reaping;
   design the protocol with future multiplexing and session pinning in
   mind.
3. **Operational hardening**: query/IPC timeout (today `read_line` has no
   deadline and a hung query wedges the database forever); bounded drain
   replacing best-effort `try_lock` shutdown.
4. **Persistent per-database connections in the daemon** with owned
   checkpointing (`wal_autocheckpoint=0`; checkpoint on the shipping
   cadence). Perf win now; prerequisite for WAL capture later (per-query
   connections let SQLite delete the WAL between queries).
5. **Rehydrate-on-miss**: a query for a database whose local files are
   missing restores the latest snapshot from S3 instead of erroring.
   Node replacement becomes: start a fresh node with the same config.
6. **Snapshot pipeline fixes**: discovery from the metadata DB + daemon
   dirty flags (not FS walks); snapshot index in Postgres (not S3 LIST
   dedup); producer switched to checkpoint+copy (coordinate with
   the `DbEngine::create_snapshot` implementations,
   `src/hosted_db/engine.rs`); `VACUUM INTO` /
   `COPY FROM DATABASE` demoted to maintenance ops.

*Exit test:* kill a single-node server, delete `data_path`, start a fresh
server with the same config → all databases serve (from S3) with no
manual restore; a hung query times out without wedging its database.

### Phase 1 — multi-node, snapshot-fidelity failover

1. `[cluster]` config section (requires Postgres metadata; `node_id`,
   advertise address; per-node vs cluster-wide config split).
2. `nodes` table + heartbeats; per-node leases; group assignment rows
   with fencing epochs; takeover via conditional `UPDATE`.
3. Routing: cached routing table (TTL + `LISTEN/NOTIFY`), transparent
   proxy to owner, single re-resolve on misroute; `create_database`
   places the group and instantiates on the owner.
4. Manager lease (janitor): reap, rebalance, retention; traffic-driven
   takeover works without it.
5. Fencing foundation on S3 even before chunking: per-database pointer
   object updated by CAS with epoch; uploads keyed under
   `manifests/{epoch}/…`.
6. Planned-drain flow (deploys): drain → final snapshot → lease release →
   instant claim.

*Exit test:* 3-node cluster; `kill -9` one node → its groups serve from
surviving nodes within lease TTL + restore, RPO = snapshot interval; a
partitioned zombie node cannot overwrite S3 state (CAS rejected) and
stops accepting writes within TTL; rolling deploy loses zero
acknowledged writes.

### Phase 2 — chunk/manifest delta shipping (both engines)

1. Chunk store + manifests (per-database pool; content-hash addressing
   with resolver indirection; 64 KiB SQLite / 1 MiB DuckDB; ≤1-chunk
   inline path; per-chunk zstd).
2. Shipper in the group supervisor: dirty flag → debounce (per-engine) →
   quiesce/checkpoint → changed-chunk identification (hash-under-lock;
   SQLite WAL frame-header page list as the cheap path) → stage → upload
   off-lock → pointer CAS.
3. Restore = pointer → manifest → parallel chunk fetch; legacy whole-file
   snapshots remain readable.
4. GC job (manager) with retention policy; user-visible snapshots =
   retained/pinned manifests via the existing endpoints.
5. Durability-tier plumbing (per database), T1 default.

*Exit tests:* RPO ≤ debounce under `kill -9` (write, wait one debounce,
kill, fail over, verify); crash mid-ship never publishes a torn state
(CAS + immutable chunks); scheduled verification reassembles and passes
`integrity_check`; idle databases generate zero S3 traffic.

### Phase 3 — SQLite WAL fast path (T2)

1. Capture WAL frames from the daemon's persistent connection; encode
   page-delta segments (LTX-shaped: page list + monotonic TXID) between
   manifest checkpoints; compaction folds segments into manifests.
2. PITR to a TXID (roadmap "versioning" synergy); scattered-update
   efficiency (ship ≈ page churn without full-file hashing).

*Exit test:* restore to an arbitrary TXID; scattered-update workload
ships ~page-churn bytes (compare against the §5 harness numbers).

### Phase 4 — RPO=0 opt-in (T3)

1. Commit gate: acknowledge a write only after its segment is durable and
   the commit head CAS succeeds; group commit batching for concurrent
   writers; optional separate prefix/bucket class (e.g., S3 Express) for
   hot segments.

*Exit test:* acked-write-then-`kill -9`-then-failover loses nothing, ever
(loop it in CI); measured commit latency within budget.

### Phase 5 — scale hardening and extensions

Storm controls (lazy activation, admission control), metadata caches +
invalidation, connection pooling posture, retention economics, fork-family
pools when forking lands, read replicas from the chunk store, DuckLake as
an additional database type, cgroup resource envelopes.

## 9. Open questions

1. Group granularity: is one-group-per-entity the right default forever,
   and what triggers splitting (hot database, size, paid tier)?
2. Proxy-only vs redirect hints for API clients long-term.
3. Sessions/transactions (roadmap) vs failover: session pinning in the
   daemon protocol; what do clients observe on drain?
4. DuckDB concurrent writers (in-process MVCC) vs the shipping quiesce —
   checkpoint scheduling under a multiplexed daemon.
5. Chunk encryption at rest (per-database keys?) — interacts with dedup
   and fork sharing.
6. Postgres HA guidance to ship with cluster mode; exact documented
   semantics of a control-plane outage.
7. Resource classes: rlimits now, cgroups when — and how they price.
8. When a hot scattered-update SQLite database should auto-promote from
   T1 chunking to T2 segments (heuristic on churn ratio?).
9. DuckDB storage-format stability across releases: policy for re-running
   the §5 harness and re-baselining after engine upgrades.
