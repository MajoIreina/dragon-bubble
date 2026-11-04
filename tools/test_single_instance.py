"""Exercise installed/portable copies, concurrent launches and crash recovery."""
from concurrent.futures import ThreadPoolExecutor
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time


def main():
    executable = Path(sys.argv[1]).resolve()
    with tempfile.TemporaryDirectory(prefix="dragon-instance-") as temporary:
        root = Path(temporary)
        copied = root / executable.name
        shutil.copy2(executable, copied)
        copied.chmod(0o755)
        env = dict(os.environ, LPP_DATA_DIR=str(root / "primary"), LPP_LOG_TO_STDERR="1")
        first = None
        with (root / "primary.log").open("w") as log:
            try:
                first = subprocess.Popen([str(executable), "--autostart"], env=env, stdout=log, stderr=log)
                time.sleep(3)
                assert first.poll() is None, "First instance exited during startup"

                def duplicate(index):
                    profile = root / f"duplicate-{index}"
                    result = subprocess.run([str(copied if index % 2 else executable), "--autostart"],
                                            env=dict(env, LPP_DATA_DIR=str(profile)),
                                            capture_output=True, timeout=10)
                    assert result.returncode == 0 and b"already running" in result.stderr, result.stderr
                    assert not profile.exists(), "Duplicate initialized a user profile"

                with ThreadPoolExecutor(max_workers=8) as pool:
                    list(pool.map(duplicate, range(8)))
                assert first.poll() is None, "Duplicate launch affected existing instance"
            finally:
                if first and first.poll() is None:
                    first.kill()
                    first.wait(timeout=20)
            # Simulate an unclean exit: the OS lock must be released without
            # removing lock files, polling stale PIDs or requiring a reboot.
            restarted = subprocess.Popen([str(executable), "--autostart"], env=env, stdout=log, stderr=log)
            try:
                time.sleep(3)
                assert restarted.poll() is None, "Lock remained after process termination"
            finally:
                if restarted.poll() is None:
                    restarted.kill()
                    restarted.wait(timeout=20)
        print("SINGLE INSTANCE PASS: 8 concurrent original/copied launches rejected before initialization; crash/restart recovered.")


if __name__ == "__main__":
    main()
