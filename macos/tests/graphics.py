"""Exercise all graphics modes and physical rebounds in the signed release app."""
import argparse
import json
import os
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("app", type=Path)
    args = parser.parse_args()
    app = args.app.resolve()
    out = app.parent / "graphics-check"
    results = []
    for name, flag, marker, timeout in [
        ("smoke", "--smoke-test", "SMOKE PASS:", 240),
        ("bounce", "--bounce-test", "BOUNCE PASS:", 60),
        ("menu", "--menu-test", "MENU PASS:", 60),
        ("resources", "--resource-test", "RESOURCE PASS:", 240),
    ]:
        folder = out / name
        folder.mkdir(parents=True, exist_ok=True)
        env = dict(os.environ, LPP_DATA_DIR=str(folder), LPP_LOG_TO_STDERR="1", RUST_BACKTRACE="1")
        with (folder / "run.log").open("w") as log:
            result = subprocess.run([str(app / "Contents/MacOS/dragon-bubble"), flag],
                                    env=env, cwd="/", stdout=log,
                                    stderr=subprocess.STDOUT, timeout=timeout)
        text = (folder / "run.log").read_text(errors="replace")
        print(text[-24000:], flush=True)
        assert result.returncode == 0 and marker in text, (name, result.returncode)
        results.append(dict(name=name, passed=True,
                            checks=[line for line in text.splitlines() if line.startswith(("SMOKE", "BOUNCE"))]))
    subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
    (out / "result.json").write_text(json.dumps(results, indent=2))
    print("GRAPHICS TESTS PASS", flush=True)


if __name__ == "__main__":
    main()
