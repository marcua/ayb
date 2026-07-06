"""Chunk-stability experiment for SQLite under realistic write patterns.

Production model being simulated: the ayb daemon holds the DB, quiesces
between queries, runs PRAGMA wal_checkpoint(TRUNCATE), and the shipper
diffs the main DB file against the last-shipped copy.
"""
import os
import random
import shutil
import sqlite3
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from chunkdiff import report

WORK = os.path.join(os.path.dirname(os.path.abspath(__file__)), "sqlite_work")
shutil.rmtree(WORK, ignore_errors=True)
os.makedirs(WORK)
DB = os.path.join(WORK, "db.sqlite")

random.seed(42)
N_ROWS = 300_000
PAYLOAD = "x" * 100  # ~120B rows -> ~25-30 rows per 4KB page


def snap(name):
    """Checkpoint-then-copy: the chunk-stable producer."""
    conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    dst = os.path.join(WORK, name)
    t0 = time.time()
    shutil.copyfile(DB, dst)
    return dst, time.time() - t0


print(f"SQLite {sqlite3.sqlite_version}")
conn = sqlite3.connect(DB)
conn.execute("PRAGMA journal_mode=WAL")
conn.execute("PRAGMA synchronous=FULL")
conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, k INTEGER, payload TEXT)")
conn.executemany(
    "INSERT INTO t VALUES (?, ?, ?)",
    ((i, random.randint(0, 10**9), PAYLOAD) for i in range(N_ROWS)),
)
conn.commit()
base, cp_time = snap("base.db")
print(f"base DB: {os.path.getsize(DB)/1e6:.1f} MB, {N_ROWS} rows; copy took {cp_time*1000:.0f} ms")

# --- W1: scattered UPDATE of 1% of rows (worst-case locality) ---
ids = random.sample(range(N_ROWS), N_ROWS // 100)
conn.executemany("UPDATE t SET k = k + 1 WHERE id = ?", ((i,) for i in ids))
conn.commit()
w1, _ = snap("w1.db")
report("SQLite W1: scattered UPDATE 1% of rows (3,000 rows)", base, w1)

# --- W2: scattered UPDATE of 0.1% of rows (drip writes) ---
ids = random.sample(range(N_ROWS), N_ROWS // 1000)
conn.executemany("UPDATE t SET k = k + 1 WHERE id = ?", ((i,) for i in ids))
conn.commit()
w2, _ = snap("w2.db")
report("SQLite W2: scattered UPDATE 0.1% (300 rows)", w1, w2)

# --- W3: clustered UPDATE of 1% (contiguous id range) ---
conn.execute("UPDATE t SET k = k + 1 WHERE id BETWEEN 100000 AND 102999")
conn.commit()
w3, _ = snap("w3.db")
report("SQLite W3: clustered UPDATE 1% (contiguous 3,000 rows)", w2, w3)

# --- W4: append INSERT 1% ---
conn.executemany(
    "INSERT INTO t VALUES (?, ?, ?)",
    ((N_ROWS + i, random.randint(0, 10**9), PAYLOAD) for i in range(N_ROWS // 100)),
)
conn.commit()
w4, _ = snap("w4.db")
report("SQLite W4: append INSERT 1% (3,000 rows)", w3, w4)

# --- W5: DELETE 1% scattered + reinsert (freelist churn) ---
ids = random.sample(range(N_ROWS), N_ROWS // 100)
conn.executemany("DELETE FROM t WHERE id = ?", ((i,) for i in ids))
conn.executemany(
    "INSERT INTO t VALUES (?, ?, ?)",
    ((10_000_000 + i, random.randint(0, 10**9), PAYLOAD) for i in range(N_ROWS // 100)),
)
conn.commit()
w5, _ = snap("w5.db")
report("SQLite W5: scattered DELETE 1% + reinsert 1%", w4, w5)

# --- W6: tiny UPDATE (10 rows) -- the churn floor ---
conn.executemany("UPDATE t SET k = k + 1 WHERE id = ?", ((i * 31,) for i in range(10)))
conn.commit()
w6, _ = snap("w6.db")
report("SQLite W6: UPDATE 10 rows", w5, w6)

# --- V: VACUUM INTO as producer around an in-place UPDATE. Deterministic
# here; see sqlite_vacuum_ripple.py for why it still can't be the producer.
v1 = os.path.join(WORK, "v1.db")
conn.execute(f"VACUUM INTO '{v1}'")
conn.executemany("UPDATE t SET k = k + 1 WHERE id = ?", ((i * 37,) for i in range(10)))
conn.commit()
v2 = os.path.join(WORK, "v2.db")
conn.execute(f"VACUUM INTO '{v2}'")
report("SQLite V: VACUUM INTO before/after UPDATE of 10 rows", v1, v2)

# --- Sanity: no-op checkpoint+copy is byte-stable ---
s1, _ = snap("s1.db")
s2, _ = snap("s2.db")
report("SQLite sanity: two copies, zero writes between", s1, s2)

conn.close()
