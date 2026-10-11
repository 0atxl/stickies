#!/usr/bin/env python3
"""Run in an isolated Wayland display and private session bus after a release build."""

import argparse
import json
import os
from pathlib import Path
import signal
import sqlite3
import subprocess
import tempfile
import time


def read_note(database):
    connection = sqlite3.connect(database)
    try:
        return connection.execute(
            "SELECT title, body FROM notes ORDER BY id DESC LIMIT 1"
        ).fetchone()
    finally:
        connection.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--signal", choices=["SIGTERM", "SIGHUP", "SIGINT"], default="SIGTERM")
    args = parser.parse_args()
    if os.environ.get("STICKIES_TEST_SANDBOX") != "1":
        parser.error("run inside an isolated Wayland session with STICKIES_TEST_SANDBOX=1")
    owner = subprocess.run(
        ["gdbus", "call", "--session", "--dest", "org.freedesktop.DBus",
         "--object-path", "/org/freedesktop/DBus", "--method",
         "org.freedesktop.DBus.GetNameOwner", "dev.stickies.Stickies"],
        capture_output=True,
    )
    if owner.returncode == 0:
        parser.error("the private session bus already has a Stickies instance")

    binary = Path(__file__).resolve().parents[1] / "target/release/stickies"
    directory = Path(tempfile.mkdtemp(prefix="stickies-shutdown-"))
    print(f"Probe data and stderr: {directory}", flush=True)
    environment = os.environ.copy()
    environment.update(
        GDK_BACKEND="wayland",
        XDG_DATA_HOME=str(directory / "data"),
        XDG_CONFIG_HOME=str(directory / "config"),
        XDG_CACHE_HOME=str(directory / "cache"),
    )
    database = directory / "data/stickies/notes.db"
    title = f"Pending {args.signal} title"
    body = f"Pending {args.signal} body"
    with (directory / "stderr.log").open("w") as log:
        process = subprocess.Popen(
            [str(binary), "--new-note"], env=environment, stdout=log, stderr=log
        )
        try:
            deadline = time.monotonic() + 10
            while True:
                assert process.poll() is None, "application exited before opening the note"
                assert time.monotonic() < deadline, "note creation timed out"
                try:
                    if database.exists() and read_note(database) is not None:
                        break
                except sqlite3.OperationalError:
                    pass  # The new database may still be applying its schema.
                time.sleep(0.02)
            time.sleep(0.8)
            # Two Tabs move from the new note's title past Close into its body.
            # Establish virtual-keyboard focus before sending editable text.
            # Keep the virtual keyboard alive for 100 ms after the final key.
            subprocess.run(
                ["wtype", "-s", "500", "-k", "Left", "-s", "200", "-d", "1",
                 title, "-k", "Tab", "-k", "Tab", body,
                 "-s", "100"], env=environment, check=True,
            )
            typed_at = time.monotonic()
            assert read_note(database) == ("", ""), "autosave ran before the signal"
            process.send_signal(getattr(signal, args.signal))
            delay_ms = (time.monotonic() - typed_at) * 1000
            assert delay_ms < 100, "signal missed the 200 ms typing deadline"
            assert process.wait(timeout=5) == 0, "application did not exit cleanly"
            assert read_note(database) == (title, body), "pending title/body were lost"
            print(json.dumps({"signal": args.signal, "saved": True,
                              "signal_after_typing_tool_ms": delay_ms,
                              "typing_tool_final_wait_ms": 100}), flush=True)
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()


if __name__ == "__main__":
    main()
