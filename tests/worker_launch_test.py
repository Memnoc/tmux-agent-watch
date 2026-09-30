"""Named worker launch and transient task delivery through real CLI/tmux seams."""
import os
import hashlib
import sys
from pathlib import Path
import subprocess
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / 'target/debug/tmux-drudwyn'

class WorkerLaunchTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='drudwyn-launch-', dir='/tmp')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.socket = str(self.root / 'socket')
        self.addCleanup(lambda: self.tmux('kill-server', check=False))
        self.repo = self.root / 'repo'
        self.git('init', '-q', '-b', 'main', str(self.repo), cwd=self.root)
        self.git('config', 'user.name', 'Test')
        self.git('config', 'user.email', 'test@example.invalid')
        (self.repo / 'task.md').write_text('repository owned task\n')
        self.git('add', '.'); self.git('commit', '-qm', 'initial')
        self.initial = self.git('rev-parse', 'HEAD')
        self.tmux('-f', '/dev/null', 'new-session', '-d', '-s', 'launch', '-x', '100', '-y', '24', '-c', str(self.repo))
        self.env = dict(os.environ, TMUX=f'{self.socket},0,0')
        self.env.pop('DRUDWYN_CLIENT', None)
        self.env['TMUX_PANE'] = self.tmux('display-message', '-p', '#{pane_id}')
        self.session = self.tmux('display-message', '-p', '#{session_id}')
        self.coordinator = self.tmux('display-message', '-p', '#{window_id}')
        self.cli('coordinator', 'set', '--window', self.coordinator, '--session', self.session)
        result = self.cli('batch', 'setup', '--repo', str(self.repo), '--session', self.session, '--yes', '--expect-source', self.initial, '--expect-destination', self.initial)
        self.batch = result.stdout.split('Batch: ')[1].splitlines()[0]
        self.cli('batch', 'select', self.batch, '--window', self.coordinator)

    def git(self, *args, cwd=None):
        return subprocess.run(['git', *args], cwd=cwd or self.repo, text=True, capture_output=True, check=True).stdout.strip()
    def tmux(self, *args, check=True):
        return subprocess.run(['tmux', '-S', self.socket, *args], env=getattr(self, 'env', None), text=True, capture_output=True, check=check).stdout.strip()
    def cli(self, *args, check=True, input=None):
        return subprocess.run([str(BIN), *args], env=self.env, text=True, input=input, capture_output=True, check=check)
    def wait(self, pane, needle):
        for _ in range(160):
            output = self.tmux('capture-pane', '-p', '-t', pane)
            if needle in output: return output
            time.sleep(.025)
        self.fail(f'Missing {needle!r}: {output}')

    def test_task_file_must_exist_at_pinned_source_before_allocation(self):
        (self.repo / 'planning-only.md').write_text('uncommitted plan')
        result = self.cli('workspace', 'start', '--repo', str(self.repo), '--batch', self.batch, '--task-file', 'planning-only.md', 'short', 'sleep', '30', check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('not available at the pinned source', result.stderr)
        self.assertFalse((self.root / 'repo-worktrees' / 'short').exists())
        self.assertNotIn('refs/heads/short', self.git('show-ref'))

    def test_form_separates_name_branch_and_multiline_text(self):
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', self.session, '-c', str(self.repo), str(BIN), 'cockpit', '--start')
        self.wait(pane, 'BATCH SETUP')
        window = self.tmux('display-message', '-p', '-t', pane, '#{window_id}')
        self.cli('batch', 'select', self.batch, '--window', window)
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait(pane, 'START WORKSPACE')
        self.tmux('send-keys', '-t', pane, 'Escape')
        time.sleep(.15)
        self.tmux('send-keys', '-t', pane, 'n')
        self.wait(pane, 'Short name')
        self.tmux('send-keys', '-t', pane, '-l', 'small')
        self.tmux('send-keys', '-t', pane, 'Tab')
        self.tmux('send-keys', '-t', pane, 'End', 'BSpace')
        self.tmux('send-keys', '-t', pane, '-l', 'X')
        self.tmux('send-keys', '-t', pane, 'Tab')
        self.tmux('send-keys', '-t', pane, '-l', 'first line')
        self.tmux('send-keys', '-t', pane, 'Enter')
        self.tmux('send-keys', '-t', pane, '-l', 'second line')
        screen = self.wait(pane, 'second line')
        self.assertIn('work/smalX', screen)
        self.assertIn('first line', screen)
        self.assertIn('F6 start', screen)
        self.assertEqual(self.tmux('list-windows', '-t', self.session, '-F', '#{window_id}').count('\n'), 1)

    def start_receiver(self, branch='short'):
        receiver = self.root / 'receiver'
        receiver.write_text('#!/bin/sh\nIFS= read -r line\nprintf "RECEIVED\\n"\nexec sleep 30\n')
        receiver.chmod(0o755)
        self.cli('workspace', 'start', '--repo', str(self.repo), '--batch', self.batch, branch, str(receiver))
        window = self.tmux('list-windows', '-t', self.session, '-F', '#{window_id}').splitlines()[-1]
        pane = self.tmux('display-message', '-p', '-t', window, '#{pane_id}')
        return window, pane

    def test_failed_paste_or_submission_is_uncertain_without_resend(self):
        for operation in ['paste-buffer', 'send-keys', 'load-buffer']:
            with self.subTest(operation=operation):
                window, pane = self.start_receiver('fail-' + operation)
                shimdir = self.root / operation
                shimdir.mkdir()
                shim = shimdir / 'tmux'
                shim.write_text('#!/bin/sh\nif [ "$1" = "' + operation + '" ]; then\n' + ('/usr/bin/tmux "$@"\n' if operation == 'load-buffer' else '') + 'exit 1\nfi\nexec /usr/bin/tmux "$@"\n')
                shim.chmod(0o755)
                original_path = self.env['PATH']
                self.env['PATH'] = str(shimdir) + ':' + original_path
                result = self.cli('workspace', 'deliver-task', window, input='private failure instruction', check=False)
                self.env['PATH'] = original_path
                self.assertNotEqual(result.returncode, 0)
                self.assertTrue((self.root / 'repo-worktrees' / ('fail-' + operation)).is_dir())
                self.assertEqual(self.tmux('display-message', '-p', '-t', pane, '#{pane_dead}'), '0')
                self.assertEqual(self.tmux('list-buffers', '-F', '#{buffer_name}'), '')
                expected = 'not_sent' if operation == 'load-buffer' else 'uncertain'
                self.assertEqual(self.tmux('show-option', '-wqv', '-t', window, '@drudwyn_delivery'), expected)
                if operation != 'load-buffer':
                    again = self.cli('workspace', 'deliver-task', window, input='private failure instruction', check=False)
                    self.assertNotEqual(again.returncode, 0)
                    self.assertIn('Nothing resent', again.stderr)
                self.assertNotIn('private failure instruction', self.tmux('show-options', '-w', '-t', window))

    def raw_receiver(self):
        script = self.root / 'raw_receiver.py'
        script.write_text("""import os, sys, tty, hashlib, time
# Controlled agent: delay models a slow startup; no task is written to disk.
time.sleep(float(sys.argv[1]) if len(sys.argv) > 1 else 0)
tty.setraw(0)
os.write(1, b'\\x1b[?2004hREADY\\r\\n')
data = b''
while True:
    data += os.read(0, 1)
    if data.endswith(b'\\r'):
        value = data[:-1].replace(b'\\x1b[200~', b'').replace(b'\\x1b[201~', b'')
        os.write(1, b'HASH:' + hashlib.sha256(value).hexdigest().encode() + b'\\r\\n')
        data = b''
""")
        agent = self.root / 'codex'
        if not agent.exists(): agent.symlink_to(sys.executable)
        return agent, script

    def test_single_start_sends_multiline_stdin_once_without_content_metadata(self):
        agent, script = self.raw_receiver()
        task = 'first private line\nsecond line with $literal and unicode café'
        result = self.cli('workspace', 'start', '--repo', str(self.repo), '--batch', self.batch, '--name', 'tiny', '--task-stdin', 'work/custom', str(agent), str(script), input=task)
        self.assertIn('Task sent; acceptance and implementation unknown', result.stdout)
        window = self.tmux('list-windows', '-t', self.session, '-F', '#{window_id}').splitlines()[-1]
        pane = self.tmux('display-message', '-p', '-t', window, '#{pane_id}')
        screen = self.wait(pane, 'HASH:')
        self.assertIn(hashlib.sha256(task.encode()).hexdigest(), screen.replace('\n', ''))
        self.assertEqual(self.tmux('display-message', '-p', '-t', pane, '#{window_name}'), 'tiny')
        self.assertEqual(self.git('-C', str(self.root / 'repo-worktrees/work-custom'), 'rev-parse', 'HEAD'), self.initial)
        self.assertNotIn(task, result.stdout + result.stderr)
        self.assertNotIn('first private line', self.tmux('show-options', '-w', '-t', window))
        self.assertNotIn('first private line', self.tmux('show-environment', '-g'))
        self.assertNotIn('first private line', subprocess.run(['ps', '-eo', 'args='], text=True, capture_output=True, check=True).stdout)
        self.assertEqual(self.tmux('list-buffers', '-F', '#{buffer_name}'), '')
        self.assertEqual(self.git('-C', str(self.root / 'repo-worktrees/work-custom'), 'status', '--porcelain', '--untracked-files=all', '--ignored'), '')
        again = self.cli('workspace', 'deliver-task', window, input=task, check=False)
        self.assertNotEqual(again.returncode, 0)
        self.assertIn('Nothing resent', again.stderr)

    def test_file_reference_start_sends_reference_and_retains_only_reference(self):
        agent, script = self.raw_receiver()
        result = self.cli('workspace', 'start', '--repo', str(self.repo), '--batch', self.batch, '--task-file', 'task.md', 'file-worker', str(agent), str(script))
        self.assertIn('Task sent;', result.stdout)
        window = self.tmux('list-windows', '-t', self.session, '-F', '#{window_id}').splitlines()[-1]
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', window, '@drudwyn_task_file'), 'task.md')
        reference_prompt = 'Read the repository task file "task.md" and carry out its instructions.'
        screen = self.wait(window, 'HASH:')
        self.assertIn(hashlib.sha256(reference_prompt.encode()).hexdigest(), ''.join(screen.split()))
        self.assertNotIn('repository owned task', self.tmux('show-options', '-w', '-t', window))
        (self.root / 'repo-worktrees/file-worker/task.md').unlink()
        result = self.cli('workspace', 'deliver-task', window, '--task-file', 'task.md', '--retry', check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('unavailable in the worker checkout', result.stderr)

    def test_split_during_start_cannot_replace_the_launch_pane(self):
        receiver = self.root / 'split-receiver'
        receiver.write_text('#!/bin/sh\ntmux split-window -t "$TMUX_PANE" sleep 30\nIFS= read -r line\nprintf "ORIGINAL_RECEIVED\\n"\nexec sleep 30\n')
        receiver.chmod(0o755)
        result = self.cli('workspace', 'start', '--repo', str(self.repo), '--batch', self.batch, '--task-stdin', 'splitting', str(receiver), input='task for original')
        window = self.tmux('list-windows', '-t', self.session, '-F', '#{window_id}').splitlines()[-1]
        original = self.tmux('list-panes', '-t', window, '-F', '#{pane_id}').splitlines()[0]
        self.wait(original, 'ORIGINAL_RECEIVED')
        self.assertIn('Task sent;', result.stdout)

    def open_form(self, width=100):
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', self.session, '-c', str(self.repo), str(BIN), 'cockpit', '--start')
        self.wait(pane, 'BATCH SETUP')
        window = self.tmux('display-message', '-p', '-t', pane, '#{window_id}')
        self.cli('batch', 'select', self.batch, '--window', window)
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait(pane, 'START WORKSPACE')
        self.tmux('send-keys', '-t', pane, 'Escape')
        time.sleep(.15)
        self.tmux('send-keys', '-t', pane, 'n')
        self.wait(pane, 'Short name')
        self.tmux('resize-window', '-t', window, '-x', str(width), '-y', '24')
        return pane

    def test_bracketed_paste_review_and_single_start_at_supported_widths(self):
        agent, script = self.raw_receiver()
        fakebin = self.root / 'fakebin'
        fakebin.mkdir()
        wrapper = fakebin / 'codex'
        wrapper.write_text('#!/bin/sh\nexec "' + str(agent) + '" "' + str(script) + '"\n')
        wrapper.chmod(0o755)
        self.env['PATH'] = str(fakebin) + ':' + self.env['PATH']
        self.tmux('set-environment', '-t', self.session, 'PATH', self.env['PATH'])
        resolved = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', self.session, 'sh', '-c', 'command -v codex; sleep 30')
        self.wait(resolved, str(wrapper))
        self.tmux('kill-pane', '-t', resolved)
        for width in [48, 64, 80, 120, 160]:
            with self.subTest(width=width):
                pane = self.open_form(width)
                self.tmux('send-keys', '-t', pane, '-l', 'tiny' + str(width))
                self.tmux('send-keys', '-t', pane, 'Tab', 'Tab')
                task = '\n'.join(f'line {n:02d}: private café 界\tinstructions for width {width}' for n in range(20))
                subprocess.run(['tmux', '-S', self.socket, 'load-buffer', '-b', 'test-input', '-'], input=task, text=True, check=True)
                self.tmux('paste-buffer', '-d', '-p', '-b', 'test-input', '-t', pane)
                self.wait(pane, 'line 19:')
                self.tmux('send-keys', '-t', pane, 'PPage')
                self.wait(pane, 'line 00:')
                self.tmux('send-keys', '-t', pane, 'NPage')
                self.wait(pane, 'line 19:')
                self.tmux('send-keys', '-t', pane, 'F6')
                self.wait(pane, 'task sent;')
                worker = self.tmux('list-windows', '-t', self.session, '-F', '#{window_id}').splitlines()[-1]
                worker_pane = self.tmux('display-message', '-p', '-t', worker, '#{pane_id}')
                screen = self.wait(worker_pane, 'HASH:')
                compact = ''.join(screen.split())
                self.assertIn(hashlib.sha256(task.encode()).hexdigest(), compact)
                self.assertTrue((self.root / 'repo-worktrees' / ('work-tiny' + str(width))).is_dir())
                self.assertEqual(self.git('rev-parse', 'HEAD'), self.initial)
                self.assertEqual(self.tmux('show-option', '-t', self.session, '-qv', '@drudwyn_coordinator'), self.coordinator)
                self.assertEqual(self.tmux('list-buffers', '-F', '#{buffer_name}'), '')

    def test_delayed_start_waits_boundedly_and_requires_deliberate_delivery_after_timeout(self):
        agent, script = self.raw_receiver()
        for delay, sent in [(0.6, True), (4.5, False)]:
            with self.subTest(delay=delay):
                bootdir = self.root / ('boot-' + str(delay))
                bootdir.mkdir()
                boot = bootdir / 'codex'
                boot.write_text('#!/bin/sh\nsleep ' + str(delay) + '\nexec "' + str(agent) + '" "' + str(script) + '"\n')
                boot.chmod(0o755)
                task = 'delayed synthetic task'
                branch = 'delay-' + str(delay)
                result = self.cli('workspace', 'start', '--repo', str(self.repo), '--batch', self.batch, '--task-stdin', branch, str(boot), input=task, check=False)
                window = self.tmux('list-windows', '-t', self.session, '-F', '#{window_id}').splitlines()[-1]
                pane = self.tmux('display-message', '-p', '-t', window, '#{pane_id}')
                self.assertTrue((self.root / 'repo-worktrees' / branch).is_dir())
                if sent:
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.wait(pane, 'HASH:')
                else:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn('task NOT sent', result.stderr)
                    self.assertEqual(self.tmux('show-option', '-wqv', '-t', window, '@drudwyn_delivery'), 'not_sent')
                    screen = self.wait(pane, 'READY')
                    self.assertNotIn('HASH:', screen)
                    self.assertEqual(self.tmux('list-buffers', '-F', '#{buffer_name}'), '')
                    self.cli('workspace', 'deliver-task', window, input=task)
                    self.wait(pane, 'HASH:')

    def test_ui_task_file_launch_keeps_selected_reference(self):
        agent, script = self.raw_receiver()
        fakebin = self.root / 'file-fakebin'; fakebin.mkdir()
        wrapper = fakebin / 'codex'
        wrapper.write_text('#!/bin/sh\nexec "' + str(agent) + '" "' + str(script) + '"\n'); wrapper.chmod(0o755)
        self.env['PATH'] = str(fakebin) + ':' + self.env['PATH']
        probe = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', self.session, 'sh', '-c', 'command -v codex; sleep 30')
        self.wait(probe, str(wrapper)); self.tmux('kill-pane', '-t', probe)
        pane = self.open_form(64)
        self.tmux('send-keys', '-t', pane, '-l', 'file-form')
        self.tmux('send-keys', '-t', pane, 'Tab', 'Tab', 'F5')
        self.tmux('send-keys', '-t', pane, '-l', 'task.md')
        self.wait(pane, 'Repository task file')
        self.tmux('send-keys', '-t', pane, 'F6')
        self.wait(pane, 'task sent;')
        window = self.tmux('list-windows', '-t', self.session, '-F', '#{window_id}').splitlines()[-1]
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', window, '@drudwyn_task_file'), 'task.md')

    def test_delivery_stays_with_created_pane_when_active_split_changes(self):
        window, pane = self.start_receiver()
        other = self.tmux('split-window', '-P', '-F', '#{pane_id}', '-t', pane, 'sleep', '30')
        result = self.cli('workspace', 'deliver-task', window, input='private instructions')
        self.assertIn('sent; acceptance and implementation unknown', result.stdout)
        self.wait(pane, 'RECEIVED')
        self.assertNotIn('private instructions', self.tmux('capture-pane', '-p', '-t', other))
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', window, '@drudwyn_delivery'), 'sent')
        self.assertEqual(self.tmux('list-buffers', '-F', '#{buffer_name}'), '')

    def test_replaced_launch_process_is_not_instructed(self):
        window, pane = self.start_receiver()
        self.tmux('respawn-pane', '-k', '-t', pane, 'sleep', '30')
        result = self.cli('workspace', 'deliver-task', window, input='private instructions', check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('process changed', result.stderr)
        self.assertNotIn('private instructions', self.tmux('capture-pane', '-p', '-t', pane))

if __name__ == '__main__': unittest.main()
