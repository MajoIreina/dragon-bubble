"""Launch the final ZIP's app from an unrelated cwd and through LaunchServices."""
import argparse
import json
import os
from pathlib import Path
import plistlib
import subprocess


def run(command, *, env=None, timeout=120):
    print("Running:", command, flush=True)
    subprocess.run(command, check=True, cwd="/", env=env, timeout=timeout)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("package", type=Path)
    args = parser.parse_args()
    package = args.package.resolve()
    out = package.parent / "launch-check"
    extracted = out / "Path With Spaces"
    extracted.mkdir(parents=True, exist_ok=True)
    run(["ditto", "-x", "-k", str(package), str(extracted)])
    app = extracted / "My Dragon Bubble.app"
    executable = app / "Contents/MacOS/dragon-bubble"
    run(["codesign", "--verify", "--deep", "--strict", str(app)])
    with (app / "Contents/Info.plist").open("rb") as stream:
        assert plistlib.load(stream)["CFBundleShortVersionString"] == "0.1.2a"
    assert not list(app.rglob("*.glb")), "Shu must be embedded, not a separate model"
    results = []
    for mode in ["direct", "finder"]:
        data = out / mode
        data.mkdir(parents=True, exist_ok=True)
        if mode == "finder":
            preferences = json.loads((out / "direct/preferences.json").read_text())
            preferences["position"] = [999999, -999999]
            (data / "preferences.json").write_text(json.dumps(preferences))
            (data / "welcome_seen").touch()
        env = dict(os.environ, LPP_DATA_DIR=str(data), RUST_BACKTRACE="1", RUST_LOG="info")
        try:
            if mode == "direct":
                run([str(executable), "--startup-test"], env=env)
            else:
                run(["open", "-W", "-n", str(app), "--env", "LPP_DATA_DIR=" + str(data),
                     "--env", "RUST_BACKTRACE=1", "--env", "RUST_LOG=info", "--args", "--startup-test"], env=env)
            evidence = json.loads((data / "startup-pass.json").read_text())
            assert evidence["backend"] == "Metal" and evidence["frames"] >= 5
            assert evidence["embedded_shu"]
            assert (data / "startup-pet.png").stat().st_size > 1000
            results.append(dict(mode=mode, **evidence))
        finally:
            log = data / "logs/latest.log"
            if log.exists():
                print(log.read_text(errors="replace")[-24000:], flush=True)
    run(["codesign", "--verify", "--deep", "--strict", str(app)])
    (out / "result.json").write_text(json.dumps(results, indent=2))
    print("PACKAGED APP LAUNCH PASS", json.dumps(results), flush=True)


if __name__ == "__main__":
    main()
