"""Shared chunk-diff measurement helpers."""
import os

CHUNK_SIZES = [4096, 16384, 65536, 262144, 1048576]


def diff_files(path_a, path_b, chunk_size):
    """Return (changed_chunks, total_chunks_b, changed_bytes, size_a, size_b).

    Chunks are fixed-size at offset k*chunk_size. A chunk counts as changed if
    its bytes differ or it exists only in the newer file (growth/truncation).
    """
    size_a, size_b = os.path.getsize(path_a), os.path.getsize(path_b)
    total_b = (size_b + chunk_size - 1) // chunk_size
    changed = 0
    with open(path_a, "rb") as fa, open(path_b, "rb") as fb:
        for _ in range(total_b):
            ca = fa.read(chunk_size)
            cb = fb.read(chunk_size)
            if ca != cb:
                changed += 1
    return changed, total_b, changed * chunk_size, size_a, size_b


def report(label, path_a, path_b):
    rows = []
    for cs in CHUNK_SIZES:
        changed, total, cbytes, _, _ = diff_files(path_a, path_b, cs)
        pct = 100.0 * changed / total if total else 0.0
        rows.append((cs, changed, total, pct, cbytes / 1e6))
    sa, sb = os.path.getsize(path_a) / 1e6, os.path.getsize(path_b) / 1e6
    print(f"\n== {label}  (before {sa:.1f} MB -> after {sb:.1f} MB)")
    print(f"   {'chunk':>8} {'changed':>9} {'total':>8} {'%chunks':>8} {'upload MB':>10}")
    for cs, changed, total, pct, mb in rows:
        print(f"   {cs:>8} {changed:>9} {total:>8} {pct:>7.1f}% {mb:>10.2f}")
