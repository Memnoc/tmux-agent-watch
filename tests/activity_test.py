#!/usr/bin/env python3
"""Activity provenance through the public CLI on disposable real tmux panes."""
import os
from contextlib import contextmanager
from pathlib import Path
import shutil
import shlex
import signal
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

    @contextmanager
    def queued_hook(self, event='permissionRequest'):
        """Observe entry to real guard acquisition while another CLI owns it."""
        gate = self.path / ('hook-' + event)
        gate.mkdir()
        ready = gate / 'ready'
        wrapper = gate / 'tmux'
        wrapper.write_text('#!/bin/sh\n'
                           'if [ "$1" = display-message ] && [ "$3" = "#{socket_path}" ]; then\n'
                           f'    : > {shlex.quote(str(ready))}\nfi\n'
                           f'exec {shlex.quote(shutil.which("tmux"))} "$@"\n')
        wrapper.chmod(0o755)
        process = subprocess.Popen([str(BIN), 'hook', 'codex', event], env=dict(self.env, PATH=str(gate) + ':' + self.env['PATH']), stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            deadline = time.monotonic() + 5
            while not ready.exists():
                self.assertIsNone(process.poll(), 'hook exited before guard acquisition')
                self.assertLess(time.monotonic(), deadline, 'hook did not reach guard acquisition')
                time.sleep(.01)
            yield process
        finally:
            if process.poll() is None:
                process.kill()
            process.communicate(timeout=5)

    def wait_fake_agents(self, count):
        root = self.tmux('display-message', '-p', '-t', self.pane, '#{pane_pid}')
        deadline = time.monotonic() + 3
        while True:
            rows = subprocess.check_output(['ps', '-eo', 'pid=,ppid=,stat=,comm='], text=True)
            agents = {pid for pid, parent, state, name in (row.split(maxsplit=3) for row in rows.splitlines())
                      if (pid == root or parent == root) and not state.startswith('Z') and Path(name).name == 'codex'}
            if len(agents) == count:
                return agents
            self.assertLess(time.monotonic(), deadline, 'fake worker population did not settle')
            time.sleep(.01)

    def unreaped_workers(self, count=1):
        parent = self.path / 'unreaped-parent.py'
        parent.write_text('import os, time\n'
                          f'for _ in range({count}):\n'
                          '    if os.fork() == 0:\n'
                          f'        os.execl({str(self.fake)!r}, "codex", "300")\n'
                          'time.sleep(300)\n')
        self.tmux('respawn-pane', '-k', '-t', self.pane, '/usr/bin/python3', str(parent))
        return self.wait_fake_agents(count)

    def make_zombie(self, pid):
        os.kill(int(pid), signal.SIGTERM)
        deadline = time.monotonic() + 3
        while True:
            state = subprocess.check_output(['ps', '-p', pid, '-o', 'stat='], text=True).strip()
            if state.startswith('Z'):
                return
            self.assertLess(time.monotonic(), deadline, 'fake worker did not become a zombie')
            time.sleep(.01)

    def test_zombie_is_exited_and_cannot_own_hook(self):
        pid, = self.unreaped_workers()
        self.make_zombie(pid)
        status = self.cli('status')
        self.assertNotIn('RUNNING', status)
        self.assertIn('Exited', status)
        self.assertIn('exit code unknown', status)
        self.assertEqual(self.option('state'), 'unknown')
        self.assertEqual(self.option('exit_time'), '')
        result = subprocess.run([str(BIN), 'hook', 'codex', 'userPromptSubmit'], env=self.env, capture_output=True, text=True, timeout=5)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('originating pane', result.stderr)
        self.assertNotIn('WORKING', self.cli('status'))

    def test_queued_hook_rejects_zombie_and_keeps_historical_review(self):
        pid, = self.unreaped_workers()
        self.cli('hook', 'codex', 'stop')
        since = self.option('attention_since')
        with self.paused_cli('scan') as (scan, release):
            with self.queued_hook() as hook:
                self.make_zombie(pid)
                release.touch()
                _, error = scan.communicate(timeout=5)
                self.assertEqual(scan.returncode, 0, error)
                _, error = hook.communicate(timeout=5)
                self.assertNotEqual(hook.returncode, 0)
                self.assertIn('originating pane', error)
        status = self.cli('status')
        self.assertIn('REVIEW', status)
        self.assertIn('Exited', status)
        self.assertNotIn('Running process', status)
        self.assertEqual(self.option('source'), 'hook')
        self.assertEqual(self.option('attention_since'), since)
        for name in ('exit_code', 'exit_signal', 'exit_time'):
            self.assertEqual(self.option(name), '')

    def test_zombie_does_not_make_live_worker_ambiguous(self):
        zombie, live = sorted(self.unreaped_workers(2))
        self.make_zombie(zombie)
        self.cli('hook', 'codex', 'stop')
        since = self.option('attention_since')
        binding = self.tmux('show-option', '-pqv', '-t', self.pane, '@drudwyn_p_identity')
        self.assertEqual(binding.split(':')[1], live)
        status = self.cli('status')
        self.assertIn('REVIEW', status)
        self.assertIn('Running process', status)
        self.assertNotIn('Ambiguous', status)
        self.assertEqual(self.option('source'), 'hook')
        self.assertEqual(self.option('attention_since'), since)
        # Once both have exited, retain only the receipt bound to the last
        # observed worker, even with an older zombie still present.
        self.make_zombie(live)
        status = self.cli('status')
        self.assertIn('REVIEW', status)
        self.assertIn('Exited', status)
        self.assertNotIn('Running process', status)
        self.assertEqual(self.tmux('show-option', '-pqv', '-t', self.pane, '@drudwyn_p_identity'), binding)
        self.assertEqual(self.option('attention_since'), since)

    def test_queued_hook_rejects_same_pane_replacement(self):
        self.cli('scan')
        before = self.wait_fake_agents(1)
        with self.paused_cli('scan') as (scan, release):
            with self.queued_hook() as hook:
                self.tmux('respawn-pane', '-k', '-t', self.pane, str(self.fake), '300')
                self.assertNotEqual(self.wait_fake_agents(1), before)
                release.touch()
                _, error = scan.communicate(timeout=5)
                self.assertEqual(scan.returncode, 0, error)
                _, error = hook.communicate(timeout=5)
                self.assertNotEqual(hook.returncode, 0, 'queued hook was rebound to replacement')
                self.assertIn('originating pane', error)
        self.assertIn('RUNNING', self.cli('status', timeout=5))
        self.assertEqual(self.option('source'), 'process')
        self.assertEqual(self.option('attention_since'), '')

    def test_queued_hook_keeps_unchanged_worker_attention(self):
        self.cli('hook', 'codex', 'permissionRequest')
        since = self.option('attention_since')
        with self.paused_cli('scan') as (scan, release):
            with self.queued_hook() as hook:
                release.touch()
                _, error = scan.communicate(timeout=5)
                self.assertEqual(scan.returncode, 0, error)
                _, error = hook.communicate(timeout=5)
                self.assertEqual(hook.returncode, 0, error)
        self.assertEqual(self.option('state'), 'needs_input')
        self.assertEqual(self.option('source'), 'hook')
        self.assertEqual(self.option('attention_since'), since)

    def test_queued_child_replacement_preserves_new_hook(self):
        self.tmux('respawn-pane', '-k', '-t', self.pane, 'bash', '--noprofile', '--norc')
        self.tmux('send-keys', '-t', self.pane, str(self.fake) + ' 300', 'Enter')
        before = self.wait_fake_agents(1)
        root = self.tmux('display-message', '-p', '-t', self.pane, '#{pane_pid}')
        self.cli('scan')
        with self.paused_cli('scan') as (scan, release):
            with self.queued_hook() as old_hook:
                self.tmux('send-keys', '-t', self.pane, 'C-c')
                self.wait_fake_agents(0)
                self.tmux('send-keys', '-t', self.pane, str(self.fake) + ' 300', 'Enter')
                self.assertNotEqual(self.wait_fake_agents(1), before)
                self.assertEqual(self.tmux('display-message', '-p', '-t', self.pane, '#{pane_pid}'), root)
                with self.queued_hook('userPromptSubmit') as new_hook:
                    release.touch()
                    _, error = scan.communicate(timeout=5)
                    self.assertEqual(scan.returncode, 0, error)
                    _, error = old_hook.communicate(timeout=5)
                    self.assertNotEqual(old_hook.returncode, 0, 'old event was rebound to the new child')
                    self.assertIn('originating pane', error)
                    _, error = new_hook.communicate(timeout=5)
                    self.assertEqual(new_hook.returncode, 0, error)
        # Neither possible acquisition order may erase the new child's event.
        self.assertEqual(self.option('state'), 'working')
        self.assertEqual(self.option('source'), 'hook')
        self.assertEqual(self.option('attention_since'), '')
        self.assertIn('WORKING', self.cli('status', timeout=5))

    def test_queued_hook_rejects_worker_that_exited(self):
        self.cli('scan')
        with self.paused_cli('scan') as (scan, release):
            with self.queued_hook() as hook:
                self.tmux('send-keys', '-t', self.pane, 'C-c')
                self.wait_fake_agents(0)
                release.touch()
                _, error = scan.communicate(timeout=5)
                self.assertEqual(scan.returncode, 0, error)
                _, error = hook.communicate(timeout=5)
                self.assertNotEqual(hook.returncode, 0)
                self.assertIn('originating pane', error)
        self.assertNotIn('NEEDS INPUT', self.cli('status', timeout=5))
        self.assertNotEqual(self.option('source'), 'hook')

    def test_queued_hook_rejects_ambiguous_pane_ownership(self):
        self.tmux('respawn-pane', '-k', '-t', self.pane, 'bash', '--noprofile', '--norc')
        self.tmux('send-keys', '-t', self.pane, str(self.fake) + ' 300 &', 'Enter')
        self.wait_fake_agents(1)
        self.cli('scan')
        with self.paused_cli('scan') as (scan, release):
            with self.queued_hook() as hook:
                self.tmux('send-keys', '-t', self.pane, str(self.fake) + ' 300 &', 'Enter')
                self.wait_fake_agents(2)
                release.touch()
                _, error = scan.communicate(timeout=5)
                self.assertEqual(scan.returncode, 0, error)
                _, error = hook.communicate(timeout=5)
                self.assertNotEqual(hook.returncode, 0)
                self.assertIn('originating pane', error)
        self.assertEqual(self.option('state'), 'unknown')
        self.assertEqual(self.option('process'), 'ambiguous')
        self.assertNotEqual(self.tmux('show-option', '-pqv', '-t', self.pane, '@drudwyn_p_source'), 'hook')

    @contextmanager
    def paused_cli(self, *args, phase='snapshot', fail=False):
        """Pause a real tmux call at its boundary, without faking its result."""
        gate = self.path / 'gate'
        gate.mkdir()
        wrapper = gate / 'tmux'
        # Python refuses directory stdin at startup. Preserve any inherited
        # descriptor through startup, then restore it before the real command.
        wrapper.write_text('#!/bin/sh\nexec /usr/bin/python3 "$(dirname "$0")/gate.py" "$@" 3<&0 </dev/null\n')
        (gate / 'gate.py').write_text('''
import os, pathlib, subprocess, sys, time
os.dup2(3, 0)
os.close(3)
gate = pathlib.Path(__file__).parent
real = REAL
args = sys.argv[1:]
state_option = '@drudwyn_state' if PHASE == 'projection' else '@drudwyn_p_state'
pause = (args[:1] == ['list-panes'] if PHASE == 'snapshot' else
         args[:1] == ['set-option'] and state_option in args and
         args[args.index(state_option) + 1] == 'running')
if pause and not (gate / 'ready').exists():
    result = subprocess.run([real, *args], capture_output=True) if PHASE == 'snapshot' else None
    (gate / 'ready').touch()
    deadline = time.monotonic() + 15
    while not (gate / 'release').exists():
        if time.monotonic() > deadline: sys.exit(97)
        time.sleep(.01)
    if FAIL == 'spawn':
        # The next mutation cannot exec; the snapshot itself remains real.
        (gate / 'tmux').write_bytes(bytes([127]) + b'ELF-invalid-executable')
    if FAIL is True: sys.exit(1)
    if result is not None:
        sys.stdout.buffer.write(result.stdout)
        sys.stderr.buffer.write(result.stderr)
        sys.exit(result.returncode)
os.execv(real, [real, *args])
'''.replace('REAL', repr(shutil.which('tmux'))).replace('PHASE', repr(phase)).replace('FAIL', repr(fail)))
        wrapper.chmod(0o755)
        process = subprocess.Popen([str(BIN), *args], env=dict(self.env, PATH=str(gate) + ':' + self.env['PATH']), stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            deadline = time.monotonic() + 5
            while not (gate / 'ready').exists():
                self.assertIsNone(process.poll(), 'CLI exited before reaching the timing gate')
                self.assertLess(time.monotonic(), deadline, 'CLI did not reach the timing gate')
                time.sleep(.01)
            yield process, gate / 'release'
        finally:
            (gate / 'release').touch()
            if process.poll() is None:
                process.kill()
            process.communicate(timeout=5)

    def attention_during_scan(self, phase):
        with self.paused_cli('scan', phase=phase) as (scan, release):
            hook = subprocess.Popen([str(BIN), 'hook', 'codex', 'permissionRequest'], env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                # A serialized implementation may queue the hook; an atomic
                # stale-write rejection may let it finish before the scan.
                try:
                    hook.wait(timeout=.4)
                except subprocess.TimeoutExpired:
                    pass
                release.touch()
                _, error = scan.communicate(timeout=5)
                self.assertEqual(scan.returncode, 0, error)
                _, error = hook.communicate(timeout=5)
                self.assertEqual(hook.returncode, 0, error)
            finally:
                if hook.poll() is None:
                    hook.kill()
                hook.communicate(timeout=5)
        self.assertEqual(self.option('state'), 'needs_input')
        self.assertEqual(self.option('source'), 'hook')
        since = self.option('attention_since')
        self.assertNotEqual(since, '')
        self.assertIn('NEEDS INPUT', self.cli('status', timeout=5))
        self.assertEqual(self.option('attention_since'), since)

    def test_initial_scan_cannot_erase_concurrent_attention(self):
        self.attention_during_scan('snapshot')

    def test_replacement_scan_cannot_erase_concurrent_attention_at_write(self):
        self.cli('hook', 'codex', 'stop')
        self.tmux('respawn-pane', '-k', '-t', self.pane, str(self.fake), '300')
        time.sleep(.08)
        # Pause at mutation, after any protective reread could have happened.
        self.attention_during_scan('write')
        self.tmux('respawn-pane', '-k', '-t', self.pane, str(self.fake), '300')
        time.sleep(.08)
        self.assertIn('RUNNING', self.cli('status', timeout=5))
        self.assertEqual(self.option('attention_since'), '')

    def test_simultaneous_scans_and_hook_keep_one_attention_receipt(self):
        with self.paused_cli('scan') as (first, release):
            commands = [('scan',), ('hook', 'codex', 'permissionRequest'), ('scan',)]
            pending = [subprocess.Popen([str(BIN), *args], env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True) for args in commands]
            try:
                time.sleep(.2)
                release.touch()
                for process in [first, *pending]:
                    _, error = process.communicate(timeout=5)
                    self.assertEqual(process.returncode, 0, error)
            finally:
                for process in pending:
                    if process.poll() is None:
                        process.kill()
                    process.communicate(timeout=5)
        self.assertIn('NEEDS INPUT', self.cli('status', timeout=5))
        self.assertEqual(self.option('source'), 'hook')
        self.assertNotEqual(self.option('attention_since'), '')

    def test_killed_scan_releases_guard_for_hook(self):
        with self.paused_cli('scan') as (scan, release):
            scan.kill()
            # Its paused tmux child is still alive: it must not inherit the lock.
            self.cli('hook', 'codex', 'permissionRequest', timeout=5)
            release.touch()
            scan.communicate(timeout=5)
        self.assertIn('NEEDS INPUT', self.cli('status', timeout=5))

    def orphaned_write_preserves_attention(self, replace, phase='write'):
        if replace:
            self.cli('hook', 'codex', 'stop')
            self.tmux('respawn-pane', '-k', '-t', self.pane, str(self.fake), '300')
            time.sleep(.08)
        with self.paused_cli('scan', phase=phase) as (scan, release):
            scan.kill()
            scan.wait(timeout=2)
            hook = subprocess.Popen([str(BIN), 'hook', 'codex', 'permissionRequest'], env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                try:
                    hook.wait(timeout=.4)
                except subprocess.TimeoutExpired:
                    pass
                release.touch()
                scan.communicate(timeout=5)
                _, error = hook.communicate(timeout=5)
                self.assertEqual(hook.returncode, 0, error)
            finally:
                if hook.poll() is None:
                    hook.kill()
                hook.communicate(timeout=5)
        # Check the completed writers before a fresh scan can repair projection.
        self.assertEqual(self.option('state'), 'needs_input')
        self.assertEqual(self.option('source'), 'hook')
        since = self.option('attention_since')
        self.assertNotEqual(since, '')
        self.assertIn('NEEDS INPUT', self.cli('status', timeout=5))
        self.cli('scan', timeout=5)
        self.assertEqual(self.option('attention_since'), since)
        self.tmux('respawn-pane', '-k', '-t', self.pane, str(self.fake), '300')
        time.sleep(.08)
        self.assertIn('RUNNING', self.cli('status', timeout=5))
        self.assertEqual(self.option('source'), 'process')
        self.assertEqual(self.option('attention_since'), '')

    def test_orphaned_initial_write_cannot_erase_newer_hook(self):
        self.orphaned_write_preserves_attention(replace=False)

    def test_orphaned_replacement_write_cannot_erase_newer_hook(self):
        self.orphaned_write_preserves_attention(replace=True)

    def test_orphaned_window_projection_cannot_erase_newer_hook(self):
        self.cli('scan')
        self.orphaned_write_preserves_attention(replace=False, phase='projection')

    def test_stalled_orphaned_mutation_returns_bounded_retry(self):
        with self.paused_cli('scan', phase='write') as (scan, release):
            scan.kill()
            scan.wait(timeout=2)
            result = subprocess.run([str(BIN), 'hook', 'codex', 'permissionRequest'], env=self.env, capture_output=True, text=True, timeout=7)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('another update is still in progress; retry', result.stderr)
            release.touch()
            scan.communicate(timeout=5)
        self.cli('hook', 'codex', 'permissionRequest', timeout=5)
        self.assertIn('NEEDS INPUT', self.cli('status', timeout=5))
        self.assertEqual(self.option('source'), 'hook')
        self.assertNotEqual(self.option('attention_since'), '')

    def test_failed_scan_releases_guard_for_hook(self):
        with self.paused_cli('scan', fail=True) as (scan, release):
            release.touch()
            scan.communicate(timeout=5)
            self.assertNotEqual(scan.returncode, 0)
        self.cli('hook', 'codex', 'permissionRequest', timeout=5)
        self.assertIn('NEEDS INPUT', self.cli('status', timeout=5))

    def test_mutation_spawn_failure_releases_guard_for_hook(self):
        with self.paused_cli('scan', fail='spawn') as (scan, release):
            release.touch()
            _, error = scan.communicate(timeout=5)
            self.assertNotEqual(scan.returncode, 0)
            self.assertIn('could not invoke tmux', error)
        self.cli('hook', 'codex', 'permissionRequest', timeout=5)
        self.assertIn('NEEDS INPUT', self.cli('status', timeout=5))
        self.assertEqual(self.option('source'), 'hook')

    def test_failed_replacement_update_cannot_inherit_old_handoff(self):
        self.cli('hook', 'codex', 'stop')
        self.tmux('respawn-pane', '-k', '-t', self.pane, str(self.fake), '300')
        time.sleep(.08)
        with self.paused_cli('scan', phase='write', fail=True) as (scan, release):
            release.touch()
            scan.communicate(timeout=5)
            self.assertNotEqual(scan.returncode, 0)
        self.assertIn('RUNNING', self.cli('status', timeout=5))
        self.assertEqual(self.option('source'), 'process')
        self.assertEqual(self.option('attention_since'), '')

    def test_stalled_scan_returns_bounded_retry_without_losing_attention(self):
        self.cli('hook', 'codex', 'permissionRequest')
        since = self.option('attention_since')
        with self.paused_cli('scan') as (scan, release):
            result = subprocess.run([str(BIN), 'hook', 'codex', 'stop'], env=self.env, capture_output=True, text=True, timeout=7)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('another update is still in progress; retry', result.stderr)
            self.assertEqual(self.option('state'), 'needs_input')
            self.assertEqual(self.option('attention_since'), since)
            release.touch()
            scan.communicate(timeout=5)
            self.assertEqual(scan.returncode, 0)
        self.cli('hook', 'codex', 'stop', timeout=5)
        self.assertIn('REVIEW', self.cli('status', timeout=5))

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
