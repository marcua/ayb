"""Does VACUUM INTO stay chunk-stable when rows are inserted/deleted
(shifting page packing), rather than updated in place?"""
import os
import random
import shutil
import sqlite3
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from chunkdiff import report

WORK = os.path.join(os.path.dirname(os.path.abspath(__file__)), "sqlite_work2")
shutil.rmtree(WORK, ignore_errors=True)
os.makedirs(WORK)
DB = os.path.join(WORK, "db.sqlite")

random.seed(7)
N_ROWS = 300_000
PAYLOAD = "x" * 100

conn = sqlite3.connect(DB)
conn.execute("PRAGMA journal_mode=WAL")
conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, k INTEGER, payload TEXT)")
conn.executemany(
    "INSERT INTO t VALUES (?, ?, ?)",
    ((i, random.randint(0, 10**9), PAYLOAD) for i in range(N_ROWS)),
)
conn.commit()

v1 = os.path.join(WORK, "v1.db")
conn.execute(f"VACUUM INTO '{v1}'")

# Delete 100 rows near the START of the table: under VACUUM's re-packing,
# everything after them shifts.
conn.executemany("DELETE FROM t WHERE id = ?", ((i,) for i in range(1000, 1100)))
conn.commit()
v2 = os.path.join(WORK, "v2.db")
conn.execute(f"VACUUM INTO '{v2}'")
report("VACUUM INTO before/after deleting 100 EARLY rows", v1, v2)

# Same tiny deletion measured with checkpoint+copy (in-place producer):
conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
cp1 = os.path.join(WORK, "cp1.db")
shutil.copyfile(DB, cp1)
conn.executemany("DELETE FROM t WHERE id = ?", ((i,) for i in range(2000, 2100)))
conn.commit()
conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
cp2 = os.path.join(WORK, "cp2.db")
shutil.copyfile(DB, cp2)
report("checkpoint+copy before/after deleting 100 early rows", cp1, cp2)
conn.close()
