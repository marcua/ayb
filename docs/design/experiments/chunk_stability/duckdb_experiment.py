"""Chunk-stability experiment for DuckDB under realistic write patterns.

Production model: daemon quiesces, runs CHECKPOINT, shipper diffs the
.duckdb file against the last-shipped copy. Connection is closed before
each copy here for experimental safety; production holds it open.

Requires: pip install duckdb
"""
import os
import shutil
import sys
import time

import duckdb

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from chunkdiff import report

WORK = os.path.join(os.path.dirname(os.path.abspath(__file__)), "duckdb_work")
shutil.rmtree(WORK, ignore_errors=True)
os.makedirs(WORK)
DB = os.path.join(WORK, "db.duckdb")

N_ROWS = 2_000_000


def run(sql):
    conn = duckdb.connect(DB)
    t0 = time.time()
    for stmt in sql:
        conn.execute(stmt)
    t1 = time.time()
    conn.execute("CHECKPOINT")
    t2 = time.time()
    conn.close()
    return t1 - t0, t2 - t1


def snap(name):
    dst = os.path.join(WORK, name)
    t0 = time.time()
    shutil.copyfile(DB, dst)
    return dst, time.time() - t0


print(f"DuckDB {duckdb.__version__}")
_, ckpt = run([
    "CREATE TABLE t AS "
    "SELECT i AS id, random() AS k, md5(i::VARCHAR) AS payload "
    f"FROM generate_series(0, {N_ROWS - 1}) s(i)",
])
base, cp_time = snap("base.duckdb")
print(f"base DB: {os.path.getsize(DB)/1e6:.1f} MB, {N_ROWS} rows "
      f"(~{N_ROWS//122880 + 1} row groups); checkpoint {ckpt*1000:.0f} ms, copy {cp_time*1000:.0f} ms")

# --- W1: scattered UPDATE 1% (every row group touched) ---
w, c = run(["UPDATE t SET k = k + 1 WHERE id % 100 = 7"])
w1, _ = snap("w1.duckdb")
report(f"DuckDB W1: scattered UPDATE 1% (20k rows; write {w:.2f}s ckpt {c:.2f}s)", base, w1)

# --- W2: clustered UPDATE 1% (one row-group range) ---
w, c = run(["UPDATE t SET k = k + 1 WHERE id BETWEEN 1000000 AND 1019999"])
w2, _ = snap("w2.duckdb")
report(f"DuckDB W2: clustered UPDATE 1% (write {w:.2f}s ckpt {c:.2f}s)", w1, w2)

# --- W3: UPDATE 10 rows (churn floor) ---
w, c = run(["UPDATE t SET k = k + 1 WHERE id IN (3,1003,20003,300003,400003,500003,600003,700003,800003,900003)"])
w3, _ = snap("w3.duckdb")
report(f"DuckDB W3: UPDATE 10 rows (write {w:.2f}s ckpt {c:.2f}s)", w2, w3)

# --- W4: append INSERT 1% ---
w, c = run([
    f"INSERT INTO t SELECT i, random(), md5(i::VARCHAR) "
    f"FROM generate_series({N_ROWS}, {N_ROWS + N_ROWS // 100 - 1}) s(i)",
])
w4, _ = snap("w4.duckdb")
report(f"DuckDB W4: append INSERT 1% (write {w:.2f}s ckpt {c:.2f}s)", w3, w4)

# --- W5: clustered DELETE 1% ---
w, c = run(["DELETE FROM t WHERE id BETWEEN 500000 AND 519999"])
w5, _ = snap("w5.duckdb")
report(f"DuckDB W5: clustered DELETE 1% (write {w:.2f}s ckpt {c:.2f}s)", w4, w5)

# --- Idempotence: CHECKPOINT with zero writes ---
run([])
s1, _ = snap("s1.duckdb")
run([])
s2, _ = snap("s2.duckdb")
report("DuckDB sanity: two checkpoint+copy cycles, zero writes between", s1, s2)

# --- COPY FROM DATABASE (rebuild) determinism around a 10-row change ---
conn = duckdb.connect(DB)
alias = conn.execute("SELECT current_database()").fetchone()[0]
c1 = os.path.join(WORK, "c1.duckdb")
conn.execute(f"ATTACH '{c1}' AS c1")
conn.execute(f"COPY FROM DATABASE {alias} TO c1")
conn.execute("DETACH c1")
conn.execute("UPDATE t SET k = k + 1 WHERE id IN (7,1007,20007,300007,400007,500007,600007,700007,800007,900007)")
c2 = os.path.join(WORK, "c2.duckdb")
conn.execute(f"ATTACH '{c2}' AS c2")
conn.execute(f"COPY FROM DATABASE {alias} TO c2")
conn.execute("DETACH c2")
conn.close()
report("DuckDB C: COPY FROM DATABASE rebuilds before/after 10-row UPDATE", c1, c2)
