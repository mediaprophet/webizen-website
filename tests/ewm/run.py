"""Run EWM qualification and rollback tests without pytest."""
import importlib.util
import sys
import traceback
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def load(path):
    spec = importlib.util.spec_from_file_location(path.stem, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def main():
    failed = 0
    ran = 0
    for path in sorted(ROOT.rglob("test_*.py")):
        mod = load(path)
        for name in sorted(dir(mod)):
            if not name.startswith("test_"):
                continue
            ran += 1
            try:
                getattr(mod, name)()
                print(f"PASS {path.name}::{name}")
            except Exception:
                failed += 1
                print(f"FAIL {path.name}::{name}")
                traceback.print_exc()
    print(f"ran={ran} failed={failed}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
