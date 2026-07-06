# Chunk-stability experiments

Measures how well SQLite and DuckDB database files lend themselves to
fixed-offset chunk diffing — the replication substrate proposed in
[`docs/design/clustering.md`](../../clustering.md) (§4–5). For each write
workload, the harness produces a before/after copy of the database file
using a given *producer* and reports, per chunk size, how many chunks
changed and how many bytes a delta ship would upload.

## Files

- `chunkdiff.py` — shared fixed-offset chunk comparison + report table.
- `sqlite_experiment.py` — SQLite workloads (scattered/clustered updates,
  appends, deletes, tiny updates) against a checkpoint-then-copy producer,
  plus `VACUUM INTO` stability under in-place updates and a no-op sanity
  check. Stdlib only.
- `sqlite_vacuum_ripple.py` — the `VACUUM INTO` failure case: deleting a
  few early rows re-packs everything downstream (~100% churn), vs the
  same deletion under checkpoint+copy (~0.1%).
- `duckdb_experiment.py` — DuckDB equivalents plus `COPY FROM DATABASE`
  (rebuild) determinism and checkpoint idempotence. Requires the `duckdb`
  Python package.

## Running

```bash
python3 sqlite_experiment.py
python3 sqlite_vacuum_ripple.py

python3 -m venv .venv && .venv/bin/pip install duckdb
.venv/bin/python duckdb_experiment.py
```

Each script creates (and re-creates) a `*_work/` scratch directory next to
itself; expect a few hundred MB of disk and a couple of minutes.

## Headline results (2026-07-06; Python sqlite3, DuckDB 1.5.4)

Full tables and interpretation live in the design doc (§5). In short:

- **Checkpoint-then-copy is byte-stable** (0% churn with no writes) for
  both engines and tracks the engine's real physical churn.
- **The rebuilders are not shippable producers**: SQLite `VACUUM INTO`
  ripples ~99.7% of the file after deleting 100 early rows; DuckDB
  `COPY FROM DATABASE` differs ~97.7% around a 10-row update (the rebuild
  is not deterministic).
- **SQLite churn floor = pages touched** (~10× the row fraction for
  scattered updates) → small chunks (64 KiB) for SQLite, and page-granular
  WAL segments as the eventual fix for hot scattered-update databases.
- **DuckDB churn floor = the touched row-group×column segment**
  (~0.5–1 MB each; deletes are nearly free via deletion vectors) → chunk
  size barely matters; 1 MiB is fine, and the shipping debounce should be
  longer than SQLite's.

Re-run this harness when adding secondary-index/blob/multi-GB cases, and
after DuckDB storage-format upgrades, before trusting the chunk-size
constants.
