"""HTTP Range window used by the local world host.

Mirrors tooling/world_server.py: missing start is 0, missing end is size-1,
and only the first comma-separated spec is honoured. Suffix-byte ranges are
not a separate RFC 7233 form here.
"""


def satisfiable_range(range_header, size):
    """Return (start, end_inclusive) or None when the range cannot be served."""
    if not range_header or not range_header.startswith("bytes="):
        return None
    spec = range_header.split("=", 1)[1].split(",", 1)[0]
    start_s, end_s = spec.split("-", 1)
    start = int(start_s or 0)
    end = int(end_s) if end_s else size - 1
    if size <= 0:
        return None
    end = min(end, size - 1)
    if start < 0 or start > end or start >= size:
        return None
    return start, end
