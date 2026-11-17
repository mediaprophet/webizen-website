"""Assemble the static GitHub Pages bundle (_site) from checked-in website roots.

Matches the path routing of tooling/world_server.py and tooling/aaa_review_server.py:
- /             -> site/*
- /publication/ -> publication/generated/*
- /production/  -> production/*
- /data/        -> data/*
- /evidence/    -> evidence/*
- /plans/       -> plans/*
- /migration/   -> migration/*
- /recipes/     -> recipes/*
- /schemas/     -> schemas/*

Leaves all binary, JSON, Vibe, Q42, 10D, and WASM files byte-for-byte untouched so
runtime SHA-256 verification in native/world-host and copy_aaa_review.py passes.
Adds subpath resolution in generated HTML/JS so the bundle works both at a root
domain (e.g. webizen.au or localhost) and under the GitHub Pages project subpath
(/webizen-website/).
"""
from pathlib import Path
import os
import posixpath
import re
import shutil
import sys

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "_site"
# Cloudflare Workers static assets reject files over 25 MiB.
MAX_ASSET_BYTES = 25 * 1024 * 1024

MOUNT_DIRS = [
    ("production", "production"),
    ("data", "data"),
    ("evidence", "evidence"),
    ("plans", "plans"),
    ("migration", "migration"),
    ("recipes", "recipes"),
    ("schemas", "schemas"),
]

HEADERS = """\
/*
  X-Content-Type-Options: nosniff
  Cross-Origin-Opener-Policy: same-origin
  Cross-Origin-Embedder-Policy: require-corp
  Cross-Origin-Resource-Policy: same-origin
  Cache-Control: public, max-age=0, must-revalidate

/*.wasm
  Content-Type: application/wasm
  Cache-Control: public, max-age=31536000, immutable

/*.10d
  Content-Type: application/octet-stream
  Cache-Control: public, max-age=31536000, immutable

/*.q42
  Content-Type: application/octet-stream
  Cache-Control: public, max-age=31536000, immutable
"""

SUBPATH_SHIM = """<script>(function(){
var m=location.pathname.match(/^(\\/webizen-website)(?:\\/|$)/);
if(!m)return;
var base=m[1];
function mapPath(u){
  if(typeof u!=="string")return u;
  if(u==="/")return base+"/";
  if(u.indexOf("/?")===0||u.indexOf("/#")===0)return base+u;
  if(u.indexOf("/source/")===0)return "/qualiaDB/"+u.slice(8);
  if(u.charAt(0)==="/"&&u.charAt(1)!=="/"&&u.indexOf(base+"/")!==0&&u.indexOf("/qualiaDB/")!==0)return base+u;
  return u;
}
var origFetch=window.fetch.bind(window);
window.fetch=function(input,init){
  if(typeof input==="string")input=mapPath(input);
  else if(typeof URL==="function"&&input instanceof URL&&input.origin===location.origin)input=new URL(mapPath(input.pathname)+input.search+input.hash,location.origin);
  return origFetch(input,init);
};
var origSetAttr=Element.prototype.setAttribute;
Element.prototype.setAttribute=function(name,value){
  if((name==="href"||name==="src")&&typeof value==="string")value=mapPath(value);
  return origSetAttr.call(this,name,value);
};
function patchProp(proto,prop){
  if(!proto)return;
  var d=Object.getOwnPropertyDescriptor(proto,prop);
  if(!d||!d.set)return;
  Object.defineProperty(proto,prop,{
    configurable:true,
    enumerable:d.enumerable,
    get:d.get,
    set:function(v){return d.set.call(this,typeof v==="string"?mapPath(v):v);}
  });
}
patchProp(window.HTMLAnchorElement&&HTMLAnchorElement.prototype,"href");
patchProp(window.HTMLImageElement&&HTMLImageElement.prototype,"src");
patchProp(window.HTMLMediaElement&&HTMLMediaElement.prototype,"src");
patchProp(window.HTMLScriptElement&&HTMLScriptElement.prototype,"src");
patchProp(window.HTMLLinkElement&&HTMLLinkElement.prototype,"href");
})();</script>"""


def ignore_filter(directory: str, contents: list[str]) -> set[str]:
    ignored = set()
    rel_dir = Path(directory).resolve().relative_to(ROOT).as_posix()
    for name in contents:
        if name in ("__pycache__", ".git", ".preview", "runs") or name.startswith("rustc"):
            ignored.add(name)
            continue
        if name.endswith((".pyc", ".exe", ".pdb", ".o")):
            ignored.add(name)
            continue
        if rel_dir == "evidence" and name == "prior-local-files":
            ignored.add(name)
            continue
        if ("/artifacts" in f"/{rel_dir}") and name.endswith((".hmc", ".dxf", ".10d", ".html")):
            ignored.add(name)
            continue
    return ignored


def rel_url(from_dir_posix: str, target_root_path: str) -> str:
    """Convert a root-relative URL like '/world/native.css' or '/?chapter=x#y' to a relative URL."""
    if target_root_path.startswith("/source/"):
        return "/qualiaDB/" + target_root_path[len("/source/"):]
    # Split query/hash
    match = re.match(r"^([^?#]*)(.*)$", target_root_path)
    path_part, suffix = match.group(1), match.group(2)
    trailing_slash = path_part.endswith("/")
    clean_target = path_part.lstrip("/")
    from_clean = from_dir_posix.strip("/") or "."
    target_clean = clean_target.rstrip("/") or "."
    rel = posixpath.relpath(target_clean, from_clean)
    if rel == ".":
        result = "./"
    else:
        if not rel.startswith("."):
            rel = "./" + rel
        if trailing_slash and not rel.endswith("/"):
            rel += "/"
        result = rel
    if suffix and result == "./":
        return "./" + suffix
    return result + suffix


def rewrite_html(html_path: Path) -> None:
    rel_dir = html_path.parent.relative_to(OUT).as_posix()
    text = html_path.read_text(encoding="utf-8")

    if "<head>" in text:
        text = text.replace("<head>", "<head>" + SUBPATH_SHIM, 1)
    elif "<head " in text:
        text = re.sub(r"(<head\b[^>]*>)", r"\1" + SUBPATH_SHIM, text, count=1)
    else:
        text = SUBPATH_SHIM + text

    def replace_attr(m: re.Match) -> str:
        attr, quote, val = m.group(1), m.group(2), m.group(3)
        if val.startswith("//"):
            return m.group(0)
        return f"{attr}={quote}{rel_url(rel_dir, val)}{quote}"

    text = re.sub(r'\b(href|src)=(["\'])(/[^"\']*)\2', replace_attr, text)

    def replace_import(m: re.Match) -> str:
        prefix, quote, val = m.group(1), m.group(2), m.group(3)
        if val.startswith("//"):
            return m.group(0)
        return f"{prefix}{quote}{rel_url(rel_dir, val)}{quote}"

    text = re.sub(r'(\bfrom\s+)(["\'])(/[^"\']+)\2', replace_import, text)
    html_path.write_text(text, encoding="utf-8")


def rewrite_js_imports(js_path: Path) -> None:
    rel_dir = js_path.parent.relative_to(OUT).as_posix()
    text = js_path.read_text(encoding="utf-8")

    def replace_import(m: re.Match) -> str:
        prefix, quote, val = m.group(1), m.group(2), m.group(3)
        if val.startswith("//"):
            return m.group(0)
        return f"{prefix}{quote}{rel_url(rel_dir, val)}{quote}"

    updated = re.sub(r'(\bfrom\s+)(["\'])(/[^"\']+)\2', replace_import, text)
    if updated != text:
        js_path.write_text(updated, encoding="utf-8")


def main() -> int:
    if OUT.exists():
        shutil.rmtree(OUT)
    shutil.copytree(ROOT / "site", OUT, ignore=ignore_filter)
    bg_gitignore = OUT / "pkg" / "native-world" / ".gitignore"
    if bg_gitignore.exists():
        bg_gitignore.unlink()

    shutil.copytree(
        ROOT / "publication" / "generated",
        OUT / "publication",
        dirs_exist_ok=True,
        ignore=ignore_filter,
    )
    for src_name, dst_name in MOUNT_DIRS:
        src_dir = ROOT / src_name
        if src_dir.exists():
            shutil.copytree(
                src_dir,
                OUT / dst_name,
                dirs_exist_ok=True,
                ignore=ignore_filter,
            )

    (OUT / ".nojekyll").write_text("", encoding="utf-8")
    (OUT / "_headers").write_text(HEADERS, encoding="utf-8")
    (OUT / "404.html").write_text(
        "<!doctype html><title>Not found</title><p>This path is not published on webizen.au.</p>\n",
        encoding="utf-8",
    )

    skipped = []
    for path in OUT.rglob("*"):
        if path.is_file() and path.stat().st_size > MAX_ASSET_BYTES:
            skipped.append((path.stat().st_size, path.relative_to(OUT).as_posix()))
            path.unlink()
    for size, rel in sorted(skipped, reverse=True):
        print(f"Skipped {size / (1024 * 1024):.2f} MiB over Cloudflare 25 MiB limit: {rel}")

    for html_file in OUT.rglob("*.html"):
        rewrite_html(html_file)

    for js_file in (OUT / "lab").rglob("*.js"):
        rewrite_js_imports(js_file)

    total_files = sum(1 for _ in OUT.rglob("*") if _.is_file())
    total_bytes = sum(f.stat().st_size for f in OUT.rglob("*") if f.is_file())
    print(f"Built _site: {total_files} files, {total_bytes / (1024 * 1024):.2f} MiB")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
