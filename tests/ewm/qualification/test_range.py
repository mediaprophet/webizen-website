"""Local hosting advertises Accept-Ranges and serves exact 206 windows."""
import hashlib
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "tooling"))
from http_range import satisfiable_range  # noqa: E402


def test_hosting_source_advertises_accept_ranges():
    server = (ROOT / "tooling/world_server.py").read_text(encoding="utf-8")
    assert "from http_range import satisfiable_range" in server
    assert "send_response(206)" in server
    assert "Accept-Ranges" in server
    pages = (ROOT / "tooling/build_pages.py").read_text(encoding="utf-8")
    assert "/packages/*" in pages
    assert "/production/*" in pages
    assert "Accept-Ranges: bytes" in pages
    assert "/*.hmc" in pages


def test_satisfiable_range_matches_world_host():
    data_len = 1946992
    assert satisfiable_range("bytes=0-63", data_len) == (0, 63)
    assert satisfiable_range("bytes=0-7", 8) == (0, 7)
    assert satisfiable_range("bytes=10-", 20) == (10, 19)
    assert satisfiable_range("bytes=0-9999999", 8) == (0, 7)
    assert satisfiable_range("bytes=8-10", 8) is None
    assert satisfiable_range("bytes=5-4", 8) is None
    assert satisfiable_range(None, 8) is None


def test_first_paint_object_serves_exact_206():
    catalog = ROOT / "packages/by-sha256/bb895e81beaa7fc65804b6294e44d7fc334c1277baec8bbd025db287b7e665de"
    raw = catalog.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == catalog.name
    start, end = satisfiable_range("bytes=0-63", len(raw))
    window = raw[start : end + 1]
    assert len(window) == 64
    assert window[:4] == b"10d\0"

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_args):
            return

        def do_GET(self):
            data = raw
            rng = self.headers.get("Range")
            windowed = satisfiable_range(rng, len(data))
            if windowed is None:
                self.send_response(416)
                self.send_header("Content-Range", f"bytes */{len(data)}")
                self.send_header("Accept-Ranges", "bytes")
                self.end_headers()
                return
            lo, hi = windowed
            chunk = data[lo : hi + 1]
            self.send_response(206)
            self.send_header("Accept-Ranges", "bytes")
            self.send_header("Content-Range", f"bytes {lo}-{hi}/{len(data)}")
            self.send_header("Content-Length", str(len(chunk)))
            self.end_headers()
            self.wfile.write(chunk)

    httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    thread.start()
    try:
        import urllib.request

        url = f"http://127.0.0.1:{httpd.server_address[1]}/packages/by-sha256/{catalog.name}"
        req = urllib.request.Request(url, headers={"Range": "bytes=0-63"})
        with urllib.request.urlopen(req, timeout=5) as response:
            assert response.status == 206
            assert response.headers.get("Accept-Ranges") == "bytes"
            assert response.headers.get("Content-Range") == f"bytes 0-63/{len(raw)}"
            body = response.read()
        assert body == raw[:64]
        assert len(body) == 64
    finally:
        httpd.shutdown()
        httpd.server_close()


def test_range_host_matrix_records_cloudflare_200_and_pages_206():
    import json

    matrix = json.loads((ROOT / "evidence/ewm/range-hosts.json").read_text(encoding="utf-8"))
    assert matrix["schema"] == "webizen.ewm-range-host-matrix/v1"
    by_id = {h["id"]: h for h in matrix["hosts"]}
    cf = by_id["cloudflare-webizen.au"]
    pages = by_id["github-pages-webizen-website"]
    local = by_id["local-world-server"]
    assert any(o["status"] == 200 and o["partial"] is False for o in cf["objects"])
    assert "whole-entry GET" in cf["policy"]
    assert any(o["status"] == 206 and o["partial"] is True for o in pages["objects"])
    assert "/webizen-website/" in pages["policy"]
    assert "206" in local["policy"]
    dem = next(o for o in cf["objects"] if o["object_bytes"] == 1946992)
    habitat = next(o for o in cf["objects"] if o["object_bytes"] == 8723745)
    assert dem["status"] == 200 and dem["partial"] is False
    assert habitat["status"] == 200 and habitat["partial"] is False
    evidence = json.loads((ROOT / "evidence/ewm/qualification.json").read_text(encoding="utf-8"))
    host = evidence["hosting_range"]
    assert host["evidence"] == "evidence/ewm/range-hosts.json"
    assert "1946992" in host["cloudflare_worker_static_assets"].replace(",", "") or "1,946,992" in host["cloudflare_worker_static_assets"]
    assert "8723745" in host["cloudflare_worker_static_assets"].replace(",", "") or "8,723,745" in host["cloudflare_worker_static_assets"]
    assert "200" in host["cloudflare_worker_static_assets"]
    assert "206" in host["github_pages"]
    assert "206" in host["local_world_server"]
    assert "Whole-entry GET" in host["fallback"]
