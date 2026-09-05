"""Exercise real navigator key handling against a disposable tmux server.

Run after cargo build: python3 tests/navigator_test.py
"""

from pathlib import Path
import os
import shlex
import subprocess
import tempfile
import time
import unittest


class NavigatorTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="watch-nav-", dir="/tmp")
        self.addCleanup(self.directory.cleanup)
        self.socket = str(Path(self.directory.name) / "tmux.sock")
        self.addCleanup(lambda: self.tmux("kill-server", check=False))
        self.tmux("-f", "/dev/null", "new-session", "-d", "-s", "keep", "sleep 300")
        self.tmux("set-window-option", "-g", "remain-on-exit", "on")

    def tmux(self, *args, check=True):
        return subprocess.run(
            ["tmux", "-S", self.socket, *args], capture_output=True,
            text=True, check=check,
        ).stdout.strip()

    def start(self, surface):
        binary = Path(__file__).resolve().parents[1] / "target/debug/tmux-agent-watch"
        self.pane = self.tmux(
            "new-window", "-d", "-P", "-F", "#{pane_id}", "-t", "keep",
            "-n", "navigator-test", shlex.join([str(binary), surface]),
        )
        self.wait_text("NAVIGATOR")

    def wait_text(self, text):
        deadline = time.monotonic() + 5
        output = ""
        while time.monotonic() < deadline:
            output = self.tmux("capture-pane", "-p", "-t", self.pane)
            if text in output:
                return
            time.sleep(0.02)
        self.fail(f"Did not render {text!r}: {output!r}")

    def keys(self, text):
        self.tmux("send-keys", "-t", self.pane, "-l", "--", text)

    def assert_closed(self):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            status = self.tmux("display-message", "-p", "-t", self.pane,
                               "#{pane_dead}:#{pane_dead_status}")
            if status == "1:0":
                return
            time.sleep(0.02)
        self.fail(f"Navigator did not exit successfully: {status}")

    def confirm_target(self, name):
        self.keys("/" + name)
        self.wait_text("Filter")
        self.keys("\x1b")
        self.wait_text("kill")
        self.keys("x")
        self.wait_text("confirm")

    def wait_gone(self, command, target):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if target not in self.tmux(*command).splitlines():
                return
            time.sleep(0.02)
        self.fail(f"{target} still exists")

    def test_rename_selected_item_and_cancel_in_both_navigators(self):
        for surface, kind in [("navigator", "window"), ("sessions", "session")]:
            with self.subTest(surface=surface):
                if kind == "window":
                    target = self.tmux("new-window", "-d", "-P", "-F", "#{window_id}",
                                       "-t", "keep", "-n", "victim", "sleep 300")
                else:
                    target = self.tmux("new-session", "-d", "-P", "-F", "#{session_id}",
                                       "-s", "victim", "sleep 300")
                self.start(surface)
                self.keys("/victim")
                self.wait_text("Filter")
                self.keys("\x1b")
                self.wait_text("r rename")
                self.keys("r")
                self.wait_text("Rename " + kind)
                self.keys("cancelled")
                self.keys("\x1b")
                self.wait_text("r rename")
                self.assertEqual(self.tmux("display-message", "-p", "-t", target,
                                          "#{" + kind + "_name}"), "victim")
                self.keys("r")
                self.wait_text("Rename " + kind)
                self.tmux("send-keys", "-t", self.pane, *(["BSpace"] * 6))
                self.keys("-renamed space; literal")
                self.tmux("send-keys", "-t", self.pane, "Enter")
                self.wait_text("Renamed")
                self.assertEqual(self.tmux("display-message", "-p", "-t", target,
                                          "#{" + kind + "_name}"), "-renamed space; literal")
                self.keys("q")
                self.assert_closed()

    def test_rename_error_allows_correction_and_preserves_current_marker(self):
        self.tmux("new-session", "-d", "-s", "duplicate", "sleep 300")
        self.start("sessions")
        self.keys("r")
        self.wait_text("Rename session")
        self.tmux("send-keys", "-t", self.pane, *(["BSpace"] * 4))
        self.tmux("send-keys", "-t", self.pane, "Enter")
        self.wait_text("Rename session")
        self.keys("duplicate")
        self.tmux("send-keys", "-t", self.pane, "Enter")
        self.wait_text("Rename failed")
        self.tmux("send-keys", "-t", self.pane, *(["BSpace"] * 9))
        self.keys("renamed-current")
        self.tmux("send-keys", "-t", self.pane, "Enter")
        self.wait_text("Renamed")
        self.wait_text("current")
        self.assertEqual(self.tmux("display-message", "-p", "-t", self.pane,
                                  "#{session_name}"), "renamed-current")
        self.keys("q")
        self.assert_closed()

    def test_workspace_confirmation_cancellation_and_refresh(self):
        target = self.tmux("new-window", "-d", "-P", "-F", "#{window_id}",
                           "-t", "keep", "-n", "victim", "sleep 300")
        self.start("navigator")
        self.confirm_target("victim")
        self.assertIn(target, self.tmux("list-windows", "-a", "-F", "#{window_id}"))
        self.keys("n")
        self.wait_text("kill")
        self.assertIn(target, self.tmux("list-windows", "-a", "-F", "#{window_id}"))
        self.keys("x")
        self.wait_text("confirm")
        self.keys("y")
        self.wait_gone(["list-windows", "-a", "-F", "#{window_id}"], target)
        self.keys("q")
        self.assert_closed()

    def test_session_confirmation_survives_rename(self):
        target = self.tmux("new-session", "-d", "-P", "-F", "#{session_id}",
                           "-s", "victim", "sleep 300")
        self.start("sessions")
        self.confirm_target("victim")
        self.tmux("rename-session", "-t", target, "renamed")
        replacement = self.tmux("new-session", "-d", "-P", "-F", "#{session_id}",
                                "-s", "victim", "sleep 300")
        self.keys("y")
        self.wait_gone(["list-sessions", "-F", "#{session_id}"], target)
        self.assertIn(replacement, self.tmux("list-sessions", "-F", "#{session_id}"))
        self.keys("q")
        self.assert_closed()

    def test_stale_session_displays_failure_without_killing_replacement(self):
        target = self.tmux("new-session", "-d", "-P", "-F", "#{session_id}",
                           "-s", "victim", "sleep 300")
        self.start("sessions")
        self.confirm_target("victim")
        self.tmux("kill-session", "-t", target)
        replacement = self.tmux("new-session", "-d", "-P", "-F", "#{session_id}",
                                "-s", "victim", "sleep 300")
        self.keys("y")
        self.wait_text("Kill failed")
        self.assertIn(replacement, self.tmux("list-sessions", "-F", "#{session_id}"))
        self.keys("q")
        self.assert_closed()

    def test_killing_session_preserves_windows_linked_elsewhere(self):
        target = self.tmux("new-session", "-d", "-P", "-F", "#{session_id}",
                           "-s", "victim", "sleep 300")
        shared = self.tmux("new-window", "-d", "-P", "-F", "#{window_id}",
                           "-t", "keep", "-n", "shared", "sleep 300")
        self.tmux("link-window", "-s", shared, "-t", "victim")
        self.start("sessions")
        self.confirm_target("victim")
        self.keys("y")
        self.wait_gone(["list-sessions", "-F", "#{session_id}"], target)
        self.assertIn(shared, self.tmux("list-windows", "-a", "-F", "#{window_id}"))
        self.keys("q")
        self.assert_closed()

    def test_killing_the_last_session_closes_its_navigator(self):
        self.start("sessions")
        self.confirm_target("keep")
        self.keys("y")
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if not self.tmux("list-sessions", check=False):
                return
            time.sleep(0.02)
        self.fail("Last session still exists")

    def test_plugin_does_not_overwrite_resurrect_save_binding(self):
        self.tmux("bind-key", "C-s", "run-shell", "/test/resurrect/save.sh")
        self.tmux("set-option", "-g", "@agent-watch-hud", "off")
        self.tmux("set-option", "-g", "@agent_watch_watcher_pid", str(os.getpid()))
        plugin = Path(__file__).resolve().parents[1] / "tmux-agent-watch.tmux"
        self.tmux("run-shell", shlex.quote(str(plugin)))
        self.assertIn("/test/resurrect/save.sh", self.tmux("list-keys", "-T", "prefix", "C-s"))
        self.assertIn("choose-tree -Zs", self.tmux("list-keys", "-T", "prefix", "S"))

    def test_manual_save_uses_resurrect_and_keeps_both_navigators_open(self):
        marker = Path(self.directory.name) / "saved"
        script = Path(self.directory.name) / "save with spaces.sh"
        script.write_text("#!/bin/sh\n[ \"$1\" = quiet ] || exit 1\nprintf saved > "
                          + shlex.quote(str(marker)) + "\n")
        script.chmod(0o700)
        self.tmux("set-option", "-g", "@resurrect-save-script-path", str(script))
        for surface in ["navigator", "sessions"]:
            with self.subTest(surface=surface):
                self.start(surface)
                self.keys("s")
                self.wait_text("Saved via tmux-resurrect")
                self.assertEqual(marker.read_text(), "saved")
                marker.unlink()
                self.keys("q")
                self.assert_closed()

    def test_manual_save_reports_missing_plugin_and_script_failure(self):
        self.start("sessions")
        self.keys("s")
        self.wait_text("Save unavailable")
        script = Path(self.directory.name) / "fail.sh"
        script.write_text("#!/bin/sh\nexit 1\n")
        script.chmod(0o700)
        self.tmux("set-option", "-g", "@resurrect-save-script-path", str(script))
        self.keys("s")
        self.wait_text("Save failed")
        self.keys("q")
        self.assert_closed()

    @unittest.skipUnless(os.environ.get("TMUX_RESURRECT_SAVE_SCRIPT"),
                         "Set TMUX_RESURRECT_SAVE_SCRIPT to test real snapshots")
    def test_real_resurrect_snapshot_excludes_killed_workspace(self):
        snapshot_dir = Path(self.directory.name) / "snapshots"
        self.tmux("set-option", "-g", "@resurrect-dir", str(snapshot_dir))
        self.tmux("set-option", "-g", "@resurrect-capture-pane-contents", "off")
        self.tmux("set-option", "-g", "@resurrect-save-script-path",
                  os.environ["TMUX_RESURRECT_SAVE_SCRIPT"])
        target = self.tmux("new-window", "-d", "-P", "-F", "#{window_id}",
                           "-t", "keep", "-n", "victim", "sleep 300")
        self.start("navigator")
        self.keys("s")
        self.wait_text("Saved via tmux-resurrect")
        self.assertIn(":victim\t", (snapshot_dir / "last").read_text())
        self.confirm_target("victim")
        self.keys("y")
        self.wait_gone(["list-windows", "-a", "-F", "#{window_id}"], target)
        self.wait_text("press s to save cleanup")
        # Resurrect snapshot names have one-second precision.
        time.sleep(1.1)
        self.keys("s")
        self.wait_text("Saved via tmux-resurrect")
        self.assertNotIn(":victim\t", (snapshot_dir / "last").read_text())
        self.keys("q")
        self.assert_closed()


if __name__ == "__main__":
    unittest.main()
