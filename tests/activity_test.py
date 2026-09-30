#!/usr/bin/env python3
"""Activity provenance through the public CLI on disposable real tmux panes."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / 'target/debug/tmux-drudwyn'


class Activity(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.path = Path(self.temp.name)
        self.socket = f'drudwyn-activity-{os.getpid()}-{self._testMethodName}'
        self.env = os.environ.copy()
        self.env.pop('TMUX', None)
        self.env.pop('TMUX_PANE', None)
        self.env.pop('DRUDWYN_CLIENT', None)
        self.fake = self.path / 'codex'
        self.fake.symlink_to(shutil.which('sleep'))
        self.assertEqual(self.fake.resolve(), Path(shutil.which('sleep')).resolve())
        self.tmux('new-session', '-d', '-s', 'activity', '-n', 'worker', str(self.fake), '300')
        self.pane = self.tmux('display-message', '-p', '#{pane_id}')
        self.window = self.tmux('display-message', '-p', '#{window_id}')
        self.env['TMUX'] = self.tmux('display-message', '-p', '#{socket_path}') + ',0,0'
        self.env['TMUX_PANE'] = self.pane
        self.tmux('set-option', '-w', 'remain-on-exit', 'on')
        time.sleep(.08)
        worker_pid = self.tmux('display-message', '-p', '-t', self.pane, '#{pane_pid}')
        executable = Path('/proc') / worker_pid / 'exe'
        if executable.exists():
            self.assertEqual(executable.resolve(), Path(shutil.which('sleep')).resolve())

    def tearDown(self):
        self.tmux('kill-server')
        self.temp.cleanup()

    def tmux(self, *args):
        return subprocess.check_output(['tmux', '-L', self.socket, '-f', '/dev/null', *args], env=self.env, text=True).strip()

    def cli(self, *args, **kwargs):
        return subprocess.check_output([str(BIN), *args], env=self.env, text=True, **kwargs)

    def option(self, name):
        return self.tmux('show-option', '-wqv', '-t', self.window, '@drudwyn_' + name)

    def test_same_pane_replacement_loses_old_attention(self):
        self.cli('hook', 'codex', 'permissionRequest')
        self.tmux('respawn-pane', '-k', '-t', self.pane, str(self.fake), '300')
        time.sleep(.08)
        self.cli('scan')
        self.assertEqual(self.option('state'), 'running')
        self.assertEqual(self.option('source'), 'process')
        self.assertEqual(self.option('attention_since'), '')

    def test_exit_is_separate_from_handoff_and_zero_is_not_completion(self):
        self.cli('hook', 'codex', 'stop')
        self.tmux('send-keys', '-t', self.pane, 'C-c')
        time.sleep(.5)
        self.cli('scan')
        self.assertEqual(self.option('state'), 'done')
        self.assertEqual(self.option('process'), 'exited')
        status = self.cli('status')
        self.assertIn('REVIEW', status)
        self.assertIn('Exited', status)
        self.assertIn('hook', status)
        self.tmux('respawn-pane', '-k', '-t', self.pane, str(self.fake), '.2')
        time.sleep(.08)
        self.cli('scan')
        time.sleep(.25)
        self.cli('scan')
        self.assertEqual(self.option('exit_code'), '0')
        self.assertNotEqual(self.option('exit_time'), '')
        self.assertEqual(self.option('state'), 'unknown')
        self.assertIn('exit code 0', self.cli('status'))

    def test_active_split_and_ambiguous_agents(self):
        self.cli('hook', 'codex', 'permissionRequest')
        helper = self.tmux('split-window', '-P', '-F', '#{pane_id}', '-t', self.pane, 'sleep', '300')
        self.cli('scan')
        status = self.cli('status')
        self.assertIn('NEEDS INPUT', status)
        self.assertIn('Codex', status)
        self.tmux('respawn-pane', '-k', '-t', helper, str(self.fake), '300')
        time.sleep(.08)
        self.cli('scan')
        self.assertEqual(self.option('state'), 'unknown')
        self.assertEqual(self.option('process'), 'ambiguous')
        self.assertIn('Ambiguous', self.cli('status'))
        self.tmux('kill-pane', '-t', helper)
        self.cli('scan')
        self.assertEqual(self.option('state'), 'needs_input')

    def test_child_replacement_under_same_shell_loses_attention(self):
        self.tmux('respawn-pane', '-k', '-t', self.pane, 'bash', '--noprofile', '--norc')
        self.tmux('send-keys', '-t', self.pane, str(self.fake) + ' 300', 'Enter')
        time.sleep(.15)
        parent = self.tmux('display-message', '-p', '-t', self.pane, '#{pane_pid}')
        self.cli('hook', 'codex', 'permissionRequest')
        self.tmux('send-keys', '-t', self.pane, 'C-c')
        time.sleep(.1)
        self.tmux('send-keys', '-t', self.pane, str(self.fake) + ' 300', 'Enter')
        time.sleep(.15)
        self.assertEqual(self.tmux('display-message', '-p', '-t', self.pane, '#{pane_pid}'), parent)
        self.cli('scan')
        self.assertEqual(self.option('state'), 'running')

    def test_managed_early_exits_retain_inspectable_receipts(self):
        repo = self.path / 'repo'
        subprocess.run(['git', 'init', '-q', '-b', 'main', str(repo)], check=True)
        subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Test', '-c', 'user.email=test@example.invalid', 'commit', '--allow-empty', '-qm', 'initial'], check=True)
        for code in (0, 23):
            result = subprocess.run([str(BIN), 'workspace', 'start', '--repo', str(repo), '--name', f'exit{code}', f'work/exit{code}', 'sh', '-c', f'exit {code}'], env=self.env, text=True, capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('agent exited during startup', result.stderr)
            pane = self.tmux('display-message', '-p', '-t', f'activity:exit{code}', '#{pane_id}')
            self.assertEqual(self.tmux('display-message', '-p', '-t', pane, '#{pane_dead}'), '1')
            self.assertTrue((self.path / 'repo-worktrees' / f'work-exit{code}').is_dir())
            # Some tmux 3.4 immediate exits have no native receipt even after
            # reaping. Unknown is required; do not invent the fixture's code.
            native = ''
            for _ in range(20):
                native = self.tmux('display-message', '-p', '-t', pane, '#{pane_dead_status}')
                if native:
                    break
                time.sleep(.05)
            status = self.cli('status')
            row = next(line for line in status.splitlines() if f'exit{code}' in line)
            if native:
                self.assertEqual(native, str(code))
                self.assertIn(f'exit code {code}', row)
                self.assertIn('UNKNOWN' if code == 0 else 'FAILED', row)
            else:
                self.assertIn('exit code unknown', row)
                self.assertIn('UNKNOWN', row)
            self.assertIn('not task completion', row)

    def test_attention_survives_real_cockpit_and_navigator_inspection(self):
        self.cli('hook', 'codex', 'permissionRequest')
        since = self.option('attention_since')
        self.tmux('resize-window', '-t', self.window, '-x', '160', '-y', '40')
        for command in ('cockpit', 'navigator'):
            ui = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', str(BIN), command)
            self.tmux('resize-window', '-t', ui, '-x', '160', '-y', '40')
            for _ in range(100):
                output = self.tmux('capture-pane', '-p', '-t', ui)
                if 'NEEDS INPUT' in output and 'hook' in output:
                    break
                time.sleep(.03)
            self.assertIn('NEEDS INPUT', output)
            self.assertIn('hook', output)
            self.tmux('send-keys', '-t', ui, 'j', 'k')
            self.tmux('send-keys', '-t', ui, 'Escape')
            self.cli('status')
            self.assertEqual(self.option('state'), 'needs_input')
            self.assertEqual(self.option('attention_since'), since)
        self.cli('hook', 'codex', 'userPromptSubmit')
        self.assertEqual(self.option('state'), 'working')
        self.assertEqual(self.option('attention_since'), '')
        self.assertIn('WORKING', self.cli('hud', 'fleet', 'activity', self.window))

    def test_hook_cannot_be_routed_from_unrelated_shell(self):
        helper = self.tmux('split-window', '-P', '-F', '#{pane_id}', '-t', self.pane, 'sleep', '300')
        env = dict(self.env, TMUX_PANE=helper)
        result = subprocess.run([str(BIN), 'hook', 'codex', 'stop'], env=env, text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('originating pane', result.stderr)
        self.cli('scan')
        self.assertEqual(self.option('state'), 'running')

    def test_nonzero_exit_keeps_handoff_and_failure_visible(self):
        self.tmux('respawn-pane', '-k', '-t', self.pane, 'sh', '-c', f'"{self.fake}" 1; exit 23')
        time.sleep(.1)
        self.cli('hook', 'codex', 'stop')
        time.sleep(1)
        status = self.cli('status')
        self.assertIn('REVIEW', status)
        self.assertIn('Exited (FAILED)', status)
        self.assertIn('exit code 23', status)
        self.assertEqual(self.option('source'), 'hook')

    def test_unobserved_exited_agent_has_no_invented_handoff(self):
        self.tmux('respawn-pane', '-k', '-t', self.pane, str(self.fake), '.1')
        time.sleep(.2)
        status = self.cli('status')
        self.assertIn('UNKNOWN', status)
        self.assertIn('exit code 0', status)
        self.assertNotIn('REVIEW', status)

    def test_starting_is_launch_evidence_and_failure_hook_is_authoritative(self):
        repo = self.path / 'repo'
        subprocess.run(['git', 'init', '-q', '-b', 'main', str(repo)], check=True)
        subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Test', '-c', 'user.email=test@example.invalid', 'commit', '--allow-empty', '-qm', 'initial'], check=True)
        launch = subprocess.Popen([str(BIN), 'workspace', 'start', '--repo', str(repo), '--name', 'slow', 'work/slow', 'sh', '-c', f'sleep 2; exec "{self.fake}" 300'], env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        seen = False
        while launch.poll() is None:
            if 'starting|launch' in self.tmux('list-windows', '-a', '-F', '#{@drudwyn_state}|#{@drudwyn_source}'):
                seen = True
            time.sleep(.01)
        out, err = launch.communicate(timeout=5)
        self.assertEqual(launch.returncode, 0, err)
        self.assertTrue(seen, 'launch did not publish Starting with launch evidence')
        fake = self.path / 'claude'
        fake.symlink_to(shutil.which('sleep'))
        self.assertEqual(fake.resolve(), Path(shutil.which('sleep')).resolve())
        self.tmux('respawn-pane', '-k', '-t', self.pane, str(fake), '300')
        time.sleep(.08)
        self.cli('hook', 'claude', 'StopFailure')
        self.assertEqual(self.option('state'), 'failed')
        self.assertEqual(self.option('source'), 'hook')
        self.cli('status')
        self.assertEqual(self.option('state'), 'failed')
        self.cli('hook', 'claude', 'UserPromptSubmit')
        self.assertEqual(self.option('state'), 'working')

    def test_linked_views_do_not_create_ambiguous_agents(self):
        self.cli('hook', 'codex', 'permissionRequest')
        self.tmux('new-session', '-d', '-s', 'linked', '-t', 'activity')
        clients = []
        terminals = []
        try:
            for session in ('activity', 'linked'):
                master, slave = os.openpty()
                terminals.append(master)
                clients.append(subprocess.Popen(['tmux', '-L', self.socket, 'attach-session', '-t', '=' + session], env=dict(self.env, TERM='xterm-256color'), stdin=slave, stdout=slave, stderr=slave))
                os.close(slave)
            for _ in range(100):
                if len(self.tmux('list-clients', '-F', '#{client_name}').splitlines()) == 2:
                    break
                time.sleep(.02)
            self.assertEqual(len(self.tmux('list-clients', '-F', '#{client_name}').splitlines()), 2)
            status = self.cli('status')
            self.assertEqual(len(status.splitlines()), 1)
            self.assertIn('NEEDS INPUT', status)
            self.assertNotIn('Ambiguous', status)
            self.tmux('split-window', '-d', '-t', self.pane, str(self.fake), '300')
            time.sleep(.08)
            status = self.cli('status')
            self.assertEqual(len(status.splitlines()), 1)
            self.assertIn('Ambiguous', status)
        finally:
            for client in clients:
                client.terminate()
                client.wait(timeout=5)
            for terminal in terminals:
                os.close(terminal)


if __name__ == '__main__':
    unittest.main()
