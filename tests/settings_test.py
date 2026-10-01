"""Exercise customization effects through the real menu in isolated tmux servers."""

from pathlib import Path
import os
import shlex
import shutil
import subprocess
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]


class SettingsTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="drudwyn-settings-", dir="/tmp")
        self.addCleanup(self.directory.cleanup)
        self.socket = str(Path(self.directory.name) / "tmux.sock")
        self.addCleanup(lambda: self.tmux("kill-server", check=False))
        self.tmux("-f", "/dev/null", "new-session", "-d", "-s", "test",
                  "-x", "94", "-y", "28", "sleep 300")
        self.tmux("set-option", "-g", "status-format[0]", "original status")
        self.tmux("set-environment", "-g", "DRUDWYN_V2_BIN",
                  str(ROOT / "target/debug/tmux-drudwyn"))
        self.tmux("set-environment", "-gu", "NO_COLOR")
        self.env = dict(os.environ, TMUX=f"{self.socket},0,0")
        self.reload()

    def tmux(self, *args, check=True):
        return subprocess.run(["tmux", "-S", self.socket, *args],
                              text=True, capture_output=True, check=check).stdout.strip()

    def reload(self):
        subprocess.run(["bash", str(ROOT / "tmux-drudwyn.tmux")],
                       env=self.env, check=True, capture_output=True)

    def launch(self):
        self.pane = self.tmux("new-window", "-d", "-P", "-F", "#{pane_id}",
                              "-t", "test", shlex.quote(str(ROOT / "scripts/settings.sh")))
        self.wait_text("OPTIONS")

    def wait_text(self, text):
        for _ in range(150):
            output = self.tmux("capture-pane", "-p", "-t", self.pane)
            if text in output:
                return output
            time.sleep(0.02)
        self.fail(f"Missing {text!r} in menu: {output}")

    def keys(self, *keys):
        self.tmux("send-keys", "-t", self.pane, *keys)

    def edit(self, category, row, value):
        self.keys(*(["l"] * category + ["j"] * row + ["e"] + ["BSpace"] * 40))
        self.tmux("send-keys", "-t", self.pane, "-l", "--", value)
        self.keys("Enter")

    def agent(self):
        window = self.tmux("display-message", "-p", "-t", "test:0", "#{window_id}")
        fake = Path(self.directory.name) / "codex"
        fake.symlink_to(shutil.which("sleep"))
        self.assertEqual(fake.resolve(), Path(shutil.which("sleep")).resolve())
        self.tmux("respawn-pane", "-k", "-t", window, str(fake), "300")
        pane = self.tmux("display-message", "-p", "-t", window, "#{pane_id}")
        pid = self.tmux("display-message", "-p", "-t", pane, "#{pane_pid}")
        executable = Path('/proc') / pid / 'exe'
        if executable.exists():
            self.assertEqual(executable.resolve(), Path(shutil.which('sleep')).resolve())
        subprocess.run([str(ROOT / "target/debug/tmux-drudwyn"), "hook", "codex", "permissionRequest"], env={**self.env, "TMUX_PANE": pane}, check=True, capture_output=True)
        self.state_since = self.tmux("show-option", "-wqv", "-t", window, "@drudwyn_since")
        return window

    def bar(self, window, width=160):
        return subprocess.run([str(ROOT / "scripts/status-bar.sh"), "test", window, str(width)],
                              env=self.env, text=True, capture_output=True, check=True).stdout

    def test_hud_disable_restores_previous_status(self):
        self.launch()
        self.keys("l", "l", "j", "Enter")
        self.wait_text("Applied")
        self.assertEqual(self.tmux("show-option", "-gqv", "@drudwyn-hud"), "off")
        self.assertEqual(self.tmux("show-option", "-gqv", "status"), "on")
        self.assertEqual(self.tmux("show-option", "-gqv", "status-format[0]"),
                         "original status")

    def test_shortcut_is_bound_before_menu_closes(self):
        self.launch()
        self.keys("l", "l", "l", "Enter", "BSpace")
        self.tmux("send-keys", "-t", self.pane, "-l", "F8")
        self.keys("Enter")
        self.wait_text("Applied")
        binding = self.tmux("list-keys", "-T", "prefix", "F8", check=False)
        self.assertIn("next-attention.sh", binding)
        self.assertEqual(self.tmux("list-keys", "-T", "prefix", "a", check=False), "")
        help_text = subprocess.run([str(ROOT / "scripts/help.sh")], input="q", env=self.env,
                                   capture_output=True, text=True, check=True).stdout
        self.assertIn("F8     jump", help_text)

    def test_every_shortcut_can_be_rebound_live(self):
        targets = [
            ("a", "next-attention.sh"), ("Space", "sidebar-resize.sh"),
            ("A", "sidebar-restart.sh"), ("W", "cockpit --start"),
            ("X", "worktree-finish.sh"), ("P", "navigation-popup.sh #{q:client_name} cockpit"),
            ("w", "navigation-popup.sh #{q:client_name} navigator"), ("C-w", "choose-tree -Zw"),
            ("s", "navigation-popup.sh #{q:client_name} sessions"), ("S", "choose-tree -Zs"),
            ("H", "help.sh"), ("O", "settings.sh"),
        ]
        self.launch()
        self.keys("l", "l", "l")
        for row, (previous, command) in enumerate(targets):
            key = f"C-F{row + 1}"
            self.keys("e", *(["BSpace"] * 10))
            self.tmux("send-keys", "-t", self.pane, "-l", key)
            self.keys("Enter")
            self.wait_text("= " + key)
            self.assertIn(command, self.tmux("list-keys", "-T", "prefix", key))
            self.assertEqual(self.tmux("list-keys", "-T", "prefix", previous, check=False), "")
            self.keys("j")

    def test_lifecycle_overrides_repaint_existing_chat_without_changing_state(self):
        window = self.agent()
        self.launch()
        self.edit(4, 3, "#123456")
        self.wait_text("Applied Waiting colour")
        self.assertIn("bg=#123456", self.bar(window))
        self.assertIn("bg=#123456", self.bar(window, 64))
        self.keys("k", "e", "BSpace")
        self.tmux("send-keys", "-t", self.pane, "-l", "!")
        self.keys("Enter")
        self.wait_text("Applied Waiting symbol")
        self.assertEqual(self.tmux("show-option", "-wqv", "-t", window, "@drudwyn_marker"),
                         "#[fg=#123456]!#[default]")
        self.assertEqual(self.tmux("show-option", "-wqv", "-t", window, "@drudwyn_since"), self.state_since)
        self.assertEqual(self.tmux("show-option", "-wqv", "-t", window, "@drudwyn_source"), "hook")

    def test_status_layout_switches_live_and_restores_single_row(self):
        self.launch()
        self.keys("l", "Enter")
        self.wait_text("Applied Status layout = tabs")
        self.assertEqual(self.tmux("show-option", "-gqv", "status"), "2")
        self.assertIn("context", self.tmux("show-option", "-gqv", "status-format[1]"))
        self.keys("Enter")
        self.wait_text("Applied Status layout = focus")
        self.assertEqual(self.tmux("show-option", "-gqv", "status"), "on")
        self.assertIn("focus", self.tmux("show-option", "-gqv", "status-format[0]"))
        self.assertEqual(self.tmux("show-option", "-gqv", "status-format[1]"), "")

    def test_theme_previews_and_descriptions_follow_selection(self):
        self.launch()
        output = self.wait_text("Agent preselected")
        self.assertGreater(output.index("Agent preselected"), output.index("[Esc] Close"))
        self.keys("l", "j", "Enter", "Enter")
        self.wait_text("Applied Theme = dawn")
        capture = self.tmux("capture-pane", "-e", "-p", "-t", self.pane)
        self.assertIn("48;2;250;244;237", capture)
        self.wait_text("Previews here immediately")
        self.keys("j")
        output = self.wait_text("Auto detects installed fonts")
        self.assertGreater(output.index("Auto detects installed fonts"), output.index("[Esc] Close"))
        self.assertIn("[Esc] Close", self.tmux("capture-pane", "-p", "-t", self.pane))

    def test_invalid_choice_and_occupied_key_keep_previous_values(self):
        self.launch()
        self.edit(0, 0, "invalid-agent")
        self.wait_text("Choose one of:")
        self.assertEqual(self.tmux("show-option", "-gqv", "@drudwyn-agent"), "")
        self.keys("Escape")
        self.edit(3, 0, "P")
        self.wait_text("already bound")
        self.assertIn("cockpit", self.tmux("list-keys", "-T", "prefix", "P"))
        self.assertIn("next-attention", self.tmux("list-keys", "-T", "prefix", "a"))
        self.keys("BSpace")
        self.tmux("send-keys", "-t", self.pane, "-l", "not-a-tmux-key")
        self.keys("Enter")
        self.wait_text("unknown key")
        self.assertIn("next-attention", self.tmux("list-keys", "-T", "prefix", "a"))

    def test_redaction_hides_labels_at_wide_and_narrow_widths(self):
        window = self.agent()
        self.tmux("rename-window", "-t", window, "secret-client")
        self.tmux("set-option", "-w", "-t", window, "@drudwyn_branch", "private-branch")
        self.launch()
        self.keys("j", "j", "j", "Enter")
        self.wait_text("Applied Redact labels")
        for width in (64, 96, 160):
            bar = self.bar(window, width)
            self.assertNotIn("secret", bar)
            self.assertNotIn("private-branch", bar)
            self.assertNotIn("tmux-agent", bar)
        for surface in ("navigator", "sessions"):
            self.pane = self.tmux("new-window", "-d", "-P", "-F", "#{pane_id}",
                                  "-t", "test", "-n", "secret-navigation",
                                  shlex.join([str(ROOT / "target/debug/tmux-drudwyn"), surface]))
            output = self.wait_text("NAVIGATOR")
            self.assertNotIn("secret", output)
            self.assertNotIn("private-branch", output)

    def test_sidebar_enable_resize_disable(self):
        self.agent()
        self.launch()
        self.keys("l", "l", "j", "j", "j", "Enter")
        self.wait_text("Applied Legacy sidebar = on")
        sidebar = self.tmux("show-option", "-qv", "-t", "test", "@drudwyn_sidebar_pane")
        self.assertTrue(sidebar.startswith("%"))
        self.keys("j", "j", "j", "e", "BSpace")
        self.tmux("send-keys", "-t", self.pane, "-l", "5")
        self.keys("Enter")
        self.wait_text("Applied Sidebar width")
        self.assertEqual(self.tmux("display-message", "-p", "-t", sidebar, "#{pane_width}"), "5")
        self.keys("k", "k", "k", "Enter")
        self.wait_text("Applied Legacy sidebar = off")
        self.assertNotIn(sidebar, self.tmux("list-panes", "-a", "-F", "#{pane_id}").splitlines())

    def test_sidebar_toggle_and_expanded_width_keep_renderer_alive(self):
        window = self.agent()
        self.launch()
        self.keys("l", "l", "j", "j", "j", "Enter")
        self.wait_text("Applied Legacy sidebar = on")
        pane = self.tmux("display-message", "-p", "-t", window, "#{pane_id}")
        subprocess.run([str(ROOT / "scripts/sidebar-resize.sh")],
                       env=dict(self.env, TMUX_PANE=pane), check=True, capture_output=True)
        sidebar = self.tmux("show-option", "-qv", "-t", "test", "@drudwyn_sidebar_pane")
        self.keys("j", "j", "j", "j", "e", "BSpace", "BSpace")
        self.tmux("send-keys", "-t", self.pane, "-l", "25")
        self.keys("Enter")
        self.wait_text("Applied Expanded sidebar width")
        self.assertEqual(self.tmux("display-message", "-p", "-t", sidebar, "#{pane_width}"), "25")
        self.assertEqual(self.tmux("display-message", "-p", "-t", sidebar, "#{pane_dead}"), "0")

    def test_interval_change_restarts_the_running_watcher(self):
        previous = self.tmux("show-option", "-gqv", "@drudwyn_watcher_pid")
        self.launch()
        self.edit(2, 4, "1")
        self.wait_text("Applied Scan interval")
        current = self.tmux("show-option", "-gqv", "@drudwyn_watcher_pid")
        self.assertNotEqual(previous, current)
        # Inspect only our isolated watcher's sleep process, not agent content.
        for _ in range(100):
            children = subprocess.run(["ps", "--ppid", current, "-o", "args="],
                                      text=True, capture_output=True).stdout
            if "sleep 1" in children:
                break
            time.sleep(0.02)
        self.assertIn("sleep 1", children)


if __name__ == "__main__":
    unittest.main()
