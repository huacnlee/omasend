#!/usr/bin/env python3
"""Smoke-test the standalone TUI in a real Unix pseudo-terminal."""
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import struct
import subprocess
import sys
import termios
import time


def screen(text):
    """`text` without escape sequences: what the terminal shows, in order."""
    return re.sub(r"\x1b\[[0-9;?]*[A-Za-z]", " ", text)


def check(binary):
    master, slave = pty.openpty()
    process = None
    try:
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
        before = termios.tcgetattr(slave)

        def session():
            os.setsid()
            fcntl.ioctl(0, termios.TIOCSCTTY, 0)

        process = subprocess.Popen([str(binary.resolve())], stdin=slave, stdout=slave,
                                   stderr=slave, preexec_fn=session)
        output = bytearray()
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            if select.select([master], [], [], 0.1)[0]:
                output.extend(os.read(master, 65536))
            text = output.decode("utf-8", errors="replace")
            if "Drop something here" in screen(text):
                break
        os.write(master, b"\x03")
        process.wait(timeout=10)
        while select.select([master], [], [], 0.1)[0]:
            output.extend(os.read(master, 65536))
        text = output.decode("utf-8", errors="replace")
        assert process.returncode == 0, f"TUI exit code: {process.returncode}"
        assert any("\u2800" <= c <= "\u28ff" for c in text), "Missing the logo"
        assert "Drop something here" in screen(text), "Missing the Outbox"
        assert "Omasend session started" not in text, "Logs corrupted the screen"
        assert termios.tcgetattr(slave) == before, "Terminal modes were not restored"
        assert "\x1b[?1049l" in text, "Alternate screen was not restored"
        print("Standalone TUI: toolbar, Outbox, clean logs, Ctrl+C and terminal restoration passed")
    finally:
        if process is not None and process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)


if __name__ == "__main__":
    check(Path(sys.argv[1]))
