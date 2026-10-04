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
        self.wait("SET UP TASKS")

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
        preview = self.wait("Enter: confirm setup and continue")
        self.assertIn("Start from: trunk", preview)
        self.assertIn("Merge into: assemble", preview)
        self.assertFalse((self.root / "assembled").exists())
        self.keys("Escape")
        self.wait("START A TASK")
        self.assertFalse((self.root / "assembled").exists())
        self.keys("F3", "Tab")
        self.tmux("send-keys", "-t", self.pane, "-l", "assemble")
        self.keys("Tab", "Tab")
        self.tmux("send-keys", "-t", self.pane, "-l", str(self.root / "assembled"))
        self.keys("Enter")
        self.wait("Enter: confirm setup and continue")
        self.keys("Enter")
        self.wait("Merge into assemble")
        self.assertEqual(self.git("branch", "--show-current"), "planning")
        self.assertEqual(self.git("-C", str(self.root / "assembled"), "rev-parse", "HEAD"), self.initial)
        self.git("update-ref", "refs/heads/trunk", self.initial)
        self.keys("Escape", "n")
        self.wait("Merge into assemble")

    def test_help_keeps_input_and_merge_choice_is_explicit(self):
        self.keys("F7")
        self.wait("(*) separate branch")
        self.keys("Enter")
        self.wait("Name the separate merge branch")
        self.keys("-l", "work/combined")
        for width in (160, 48):
            self.tmux("resize-window", "-t", self.pane, "-x", str(width), "-y", "24")
            self.keys("F1")
            self.wait("Where changes go")
            self.keys("NPage")
            self.wait("F1/Esc")
            self.keys("Escape")
            self.wait("work/combined")
        self.keys("F7")
        self.wait("(*) trunk")
        self.keys("Tab", "Tab")
        self.keys("-l", str(self.root / "trunk-folder"))
        self.keys("Enter")
        self.wait("Merge into: trunk")
        self.keys("NPage", "NPage")
        self.wait("Enter: confirm setup and continue")
        self.keys("Home")
        self.wait("Start from:")
        self.keys("F8")
        self.wait(self.initial)
        self.keys("Escape")
        self.assertFalse((self.root / "assembled").exists())



if __name__ == "__main__":
    unittest.main()
