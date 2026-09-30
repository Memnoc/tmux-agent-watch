"""Real Cockpit keys against a disposable Git repository and tmux server."""
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target/debug/tmux-drudwyn"


class BatchUiTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="drudwyn-batch-ui-", dir="/tmp")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.socket = str(self.root / "socket")
        self.addCleanup(lambda: self.tmux("kill-server", check=False))
        self.repo = self.root / "repo"
        self.git("init", "-q", "-b", "trunk", str(self.repo), cwd=self.root)
        self.git("config", "user.name", "Test")
        self.git("config", "user.email", "test@example.invalid")
        self.git("commit", "-qm", "initial", "--allow-empty")
        self.initial = self.git("rev-parse", "HEAD")
        self.git("switch", "-qc", "planning")
        self.tmux("-f", "/dev/null", "new-session", "-d", "-s", "batch", "-x", "120", "-y", "36", "-c", str(self.repo))
        self.env = dict(os.environ, TMUX=f"{self.socket},0,0")
        self.env.pop("DRUDWYN_CLIENT", None)
        self.env["TMUX_PANE"] = self.tmux("display-message", "-p", "#{pane_id}")
        self.session = self.tmux("display-message", "-p", "#{session_id}")
        self.coordinator = self.tmux("display-message", "-p", "#{window_id}")
        self.cli("coordinator", "set", "--window", self.coordinator, "--session", self.session)
        self.tmux("set-option", "-g", "@drudwyn-base-branch", "trunk")
        self.pane = self.tmux("new-window", "-d", "-P", "-F", "#{pane_id}", "-t", self.session, "-c", str(self.repo), "env", "-u", "DRUDWYN_CLIENT", str(BIN), "cockpit", "--start")
        self.wait("BATCH SETUP")

    def git(self, *args, cwd=None):
        return subprocess.run(["git", *args], cwd=cwd or self.repo, text=True, capture_output=True, check=True).stdout.strip()

    def tmux(self, *args, check=True):
        return subprocess.run(["tmux", "-S", self.socket, *args], text=True, capture_output=True, check=check).stdout.strip()

    def cli(self, *args):
        return subprocess.run([str(BIN), *args], env=self.env, text=True, capture_output=True, check=True).stdout

    def keys(self, *args):
        self.tmux("send-keys", "-t", self.pane, *args)

    def wait(self, needle):
        for _ in range(150):
            output = self.tmux("capture-pane", "-p", "-t", self.pane)
            if needle in output:
                return output
            time.sleep(.02)
        self.fail(f"Missing {needle!r}: {output}")

    def test_setup_cancel_confirm_and_reopen_keeps_destination_and_pin(self):
        self.keys("Tab")
        self.tmux("send-keys", "-t", self.pane, "-l", "assemble")
        self.keys("Tab", "Tab")
        self.tmux("send-keys", "-t", self.pane, "-l", str(self.root / "assembled"))
        self.keys("Enter")
        preview = self.wait("Enter: confirm this source and destination")
        self.assertIn("Source: refs/heads/trunk", preview)
        self.assertIn("Destination: assemble", preview)
        self.assertFalse((self.root / "assembled").exists())
        self.keys("Escape")
        self.wait("START WORKSPACE")
        self.assertFalse((self.root / "assembled").exists())
        self.keys("F3", "Tab")
        self.tmux("send-keys", "-t", self.pane, "-l", "assemble")
        self.keys("Tab", "Tab")
        self.tmux("send-keys", "-t", self.pane, "-l", str(self.root / "assembled"))
        self.keys("Enter")
        self.wait("Enter: confirm this source and destination")
        self.keys("Enter")
        self.wait("Batch pinned · Destination assemble")
        self.assertEqual(self.git("branch", "--show-current"), "planning")
        self.assertEqual(self.git("-C", str(self.root / "assembled"), "rev-parse", "HEAD"), self.initial)
        self.git("update-ref", "refs/heads/trunk", self.initial)
        self.keys("Escape", "n")
        self.wait("Batch pinned · Destination assemble")


if __name__ == "__main__":
    unittest.main()
