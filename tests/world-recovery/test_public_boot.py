"""WR-00: publication alias and bare-URL resident boot.

The well-known /publication/world.q42 file is untracked. A fresh tree built
without that file must still publish it from publication/generated/current.json.
"""
import hashlib
import json
import shutil
import sys
import tempfile
import threading
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tooling"))
import build_pages  # noqa: E402


READING_FALLBACK = (
    "The living world could not start on this device. The reading view is always available."
)
EWM_INSTRUCTION = "The woodland streams when you add ?ewm=1"


def test_world_q42_alias_without_untracked_file():
    pointer_path = ROOT / "publication" / "generated" / "current.json"
    pointer = json.loads(pointer_path.read_text(encoding="utf-8"))
    named = ROOT / "publication" / "generated" / pointer["q42"]
    assert named.is_file()
    named_bytes = named.read_bytes()
    assert hashlib.sha256(named_bytes).hexdigest() == pointer["sha256"]
    with tempfile.TemporaryDirectory() as raw:
        scratch = Path(raw)
        generated = scratch / "generated"
        publication = scratch / "_site" / "publication"
        generated.mkdir(parents=True)
        publication.mkdir(parents=True)
        shutil.copy(pointer_path, generated / "current.json")
        shutil.copy(named, generated / pointer["q42"])
        assert not (generated / "world.q42").exists()
        for item in generated.iterdir():
            shutil.copy(item, publication / item.name)
        assert not (publication / "world.q42").exists()
        written = build_pages.write_world_q42_alias(generated, publication)
        assert written.is_file()
        assert written.name == "world.q42"
        assert written.read_bytes() == named_bytes
        assert hashlib.sha256(written.read_bytes()).hexdigest() == pointer["sha256"]
    pages = (ROOT / "tooling" / "build_pages.py").read_text(encoding="utf-8")
    assert "write_world_q42_alias(" in pages


def test_static_alias_get_is_200():
    pointer = json.loads(
        (ROOT / "publication" / "generated" / "current.json").read_text(encoding="utf-8")
    )
    named = (ROOT / "publication" / "generated" / pointer["q42"]).read_bytes()
    with tempfile.TemporaryDirectory() as raw:
        scratch = Path(raw)
        publication = scratch / "publication"
        publication.mkdir(parents=True)
        build_pages.write_world_q42_alias(ROOT / "publication" / "generated", publication)

        class Handler(SimpleHTTPRequestHandler):
            def __init__(self, *args, **kwargs):
                super().__init__(*args, directory=str(scratch), **kwargs)

            def log_message(self, *_args):
                return

        httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=httpd.serve_forever, daemon=True)
        thread.start()
        try:
            import urllib.request

            url = f"http://127.0.0.1:{httpd.server_address[1]}/publication/world.q42"
            with urllib.request.urlopen(url, timeout=5) as response:
                body = response.read()
                assert response.status == 200
                assert body == named
                assert hashlib.sha256(body).hexdigest() == pointer["sha256"]
        finally:
            httpd.shutdown()


def test_build_pages_main_writes_alias_without_untracked_file():
    """A fresh _site built with no untracked world.q42 still publishes the alias."""
    pointer_path = ROOT / "publication" / "generated" / "current.json"
    pointer = json.loads(pointer_path.read_text(encoding="utf-8"))
    named = (ROOT / "publication" / "generated" / pointer["q42"]).read_bytes()
    saved_root = build_pages.ROOT
    saved_out = build_pages.OUT
    with tempfile.TemporaryDirectory() as raw:
        tmp_path = Path(raw)
        generated = tmp_path / "publication" / "generated"
        generated.mkdir(parents=True)
        (generated / "current.json").write_text(
            pointer_path.read_text(encoding="utf-8"), encoding="utf-8"
        )
        (generated / pointer["q42"]).write_bytes(named)
        assert not (generated / "world.q42").exists()
        site = tmp_path / "site"
        site.mkdir()
        (site / "index.html").write_text(
            (ROOT / "site" / "index.html").read_text(encoding="utf-8"),
            encoding="utf-8",
        )
        out = tmp_path / "_site"
        build_pages.ROOT = tmp_path
        build_pages.OUT = out
        try:
            assert build_pages.main() == 0
        finally:
            build_pages.ROOT = saved_root
            build_pages.OUT = saved_out
        alias = out / "publication" / "world.q42"
        assert alias.is_file()
        assert alias.read_bytes() == named
        assert hashlib.sha256(alias.read_bytes()).hexdigest() == pointer["sha256"]
        html = (out / "index.html").read_text(encoding="utf-8")
        assert "/intenteco/" not in html
        assert "githubusercontent.com" not in html
        assert EWM_INSTRUCTION not in html
        assert READING_FALLBACK not in html

        class Handler(SimpleHTTPRequestHandler):
            def __init__(self, *args, **kwargs):
                super().__init__(*args, directory=str(out), **kwargs)

            def log_message(self, *_args):
                return

        httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=httpd.serve_forever, daemon=True)
        thread.start()
        try:
            import urllib.request

            base = f"http://127.0.0.1:{httpd.server_address[1]}"
            with urllib.request.urlopen(base + "/", timeout=5) as response:
                document = response.read().decode("utf-8")
                assert response.status == 200
            with urllib.request.urlopen(base + "/publication/world.q42", timeout=5) as response:
                body = response.read()
                assert response.status == 200
                assert body == named
            requested = []
            for attr in ("href", "src"):
                start = 0
                needle = attr + "="
                while True:
                    at = document.find(needle, start)
                    if at < 0:
                        break
                    quote = document[at + len(needle)]
                    end = document.find(quote, at + len(needle) + 1)
                    requested.append(document[at + len(needle) + 1 : end])
                    start = end + 1
            assert requested
            assert all("/intenteco/" not in url and not url.startswith("http") for url in requested)
            assert EWM_INSTRUCTION not in document
            assert READING_FALLBACK not in document
        finally:
            httpd.shutdown()
            thread.join(timeout=5)


def test_resident_homepage_does_not_require_ewm_query():
    boot = (ROOT / "native" / "world-host" / "src" / "runtime" / "boot.rs").read_text(
        encoding="utf-8"
    )
    html = (ROOT / "site" / "index.html").read_text(encoding="utf-8")
    assert 'data-scene="/production/lush-woodland/scene.q42"' in html
    assert EWM_INSTRUCTION not in html
    assert READING_FALLBACK not in html
    assert "/intenteco/" not in html
    assert "/publication/world.q42" in boot
    assert "/intenteco/" not in boot
    runtime = (ROOT / "native" / "world-host" / "src" / "runtime.rs").read_text(encoding="utf-8")
    assert "Could not load {path}" in runtime
    assert 'query_has_token(&query, "ewm=1")' in boot
    assert "extra_streaming" in boot
    assert "resident_scene" in boot
    assert "preview_scene != WOODLAND_SCENE_VOLUME" in boot
    assert "!resident_scene && !extra_streaming" in boot
    assert 'preview_scene.ends_with(".q42") && !ewm_streaming' not in boot
    spawn = boot.index("spawn_catchment_streaming_loop")
    assert "if extra_streaming" in boot[spawn - 200 : spawn]
    assert '"default":false' in boot
    main_src = (ROOT / "tooling" / "build_pages.py").read_text(encoding="utf-8")
    assert "write_world_q42_alias(" in main_src.split("def main", 1)[1]
    pack = json.loads((ROOT / "packages" / "current.json").read_text(encoding="utf-8"))
    assert pack["streaming"]["default"] is False
    paint = json.loads((ROOT / "data" / "ewm" / "first-paint.json").read_text(encoding="utf-8"))
    assert paint["streaming"]["default"] is False
    catalog = (ROOT / "native" / "world-host" / "src" / "runtime" / "catalog.rs").read_text(
        encoding="utf-8"
    )
    assert "must keep streaming.default false" in catalog
