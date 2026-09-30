"""Existing-checkout recovery through the CLI and real two-client Cockpit."""
import os
import shlex
import shutil
import subprocess
import sys
import time
import unittest
from pathlib import Path
from independent_navigation_test import IndependentNavigation, BIN

class RecoveryTest(IndependentNavigation):
    # Reuse the isolated two-client fixture, not its unrelated test cases.
    def test_inventory_distinguishes_survivor_from_live_checkout(self):
        survivor = Path(self.tmp.name) / 'surviving checkout'
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'add', '-qb', 'survivor', str(survivor)], check=True)
        result = self.command('workspace', 'recover-list', '--repo', str(self.repo), client=self.clients[0], check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('survivor', result.stdout)
        self.assertIn('Recoverable', result.stdout)
        self.assertIn('Live workspace', result.stdout)
        self.assertIn('Unknown', result.stdout)

    def test_open_shell_preserves_dirty_literal_checkout_and_client_selection(self):
        (self.repo / 'tracked').write_text('original')
        subprocess.run(['git', '-C', str(self.repo), 'add', 'tracked'], check=True)
        subprocess.run(['git', '-C', str(self.repo), '-c', 'user.name=Test', '-c', 'user.email=test@example.invalid', 'commit', '-qm', 'tracked'], check=True)
        survivor = Path(self.tmp.name) / 'survivor $literal #;'
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'add', '-qb', 'survivor', str(survivor)], check=True)
        (survivor / 'tracked').write_text('dirty keep me')
        (survivor / 'untracked').write_text('keep me')
        before = self.selection(self.clients[1])
        result = self.command('workspace', 'recover', '--repo', str(self.repo), '--path', str(survivor), '--shell', client=self.clients[0], check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        window = result.stdout.strip().splitlines()[0]
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + window))
        self.assertEqual(before, self.selection(self.clients[1]))
        self.command('scan', client=self.clients[0])
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', window, '@drudwyn_process'), '')
        self.assertEqual((survivor / 'untracked').read_text(), 'keep me')
        self.assertEqual((survivor / 'tracked').read_text(), 'dirty keep me')
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', window, '@drudwyn_git_status'), 'dirty')
        self.assertEqual(subprocess.check_output(['git', '-C', str(survivor), 'branch', '--show-current'], text=True).strip(), 'survivor')
        again = self.command('workspace', 'recover', '--repo', str(self.repo), '--path', str(survivor), '--shell', client=self.clients[0], check=False)
        self.assertNotEqual(again.returncode, 0)
        self.assertIn('Live workspace', again.stderr)

    def test_restart_requires_task_and_uses_exact_fake_agent(self):
        survivor = Path(self.tmp.name) / 'restart $literal;'
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'add', '-qb', 'restart', str(survivor)], check=True)
        fakebin = Path(self.tmp.name) / 'agents'
        fakebin.mkdir()
        shutil.copy('/bin/cat', fakebin / 'codex')
        self.env['PATH'] = str(fakebin) + os.pathsep + self.env['PATH']
        self.assertEqual(shutil.which('codex', path=self.env['PATH']), str(fakebin / 'codex'))
        args = ['workspace', 'recover', '--repo', str(self.repo), '--path', str(survivor), '--agent', 'codex', '--unassociated']
        missing = self.command(*args, client=self.clients[0], check=False)
        self.assertNotEqual(missing.returncode, 0)
        (survivor / 'task.md').write_text('private task content')
        result = self.command(*args, '--task-file', 'task.md', client=self.clients[0], check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('Fresh conversation', result.stdout)
        window = result.stdout.splitlines()[0]
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', window, '@drudwyn_delivery'), 'sent')
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', window, '@drudwyn_task_file'), 'task.md')
        pane = self.tmux('display-message', '-p', '-t', window, '#{pane_id}')
        pid = self.tmux('display-message', '-p', '-t', pane, '#{pane_pid}')
        self.assertEqual(os.readlink('/proc/' + pid + '/exe'), str(fakebin / 'codex'))
        self.assertNotIn('private task content', self.tmux('show-options', '-w', '-t', window))
        self.assertEqual(self.tmux('list-buffers', '-F', '#{buffer_name}'), '')
        self.command('workspace', 'deliver-task', window, '--task-file', 'task.md', '--retry', client=self.clients[0])

    def test_missing_coordinator_can_be_recovered_without_resetting_files(self):
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        (self.repo / 'untracked').write_text('coordinator planning')
        self.tmux('kill-window', '-t', self.home)
        self.tmux('new-window', '-d', '-t', 'project', '-c', '/tmp', 'sleep', '30')
        self.tmux('kill-window', '-t', self.worker)
        result = self.command('workspace', 'recover', '--repo', str(self.repo), '--path', str(self.repo), '--shell', '--coordinator', client=self.clients[0], check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        window = result.stdout.strip()
        self.assertEqual(self.tmux('show-option', '-qv', '-t', 'project', '@drudwyn_coordinator'), window)
        self.assertEqual((self.repo / 'untracked').read_text(), 'coordinator planning')
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', window, '@drudwyn_worktree'), '')
        self.command('coordinator', 'open', client=self.clients[0])
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + window))

    def test_cockpit_exposes_recovery_shell_and_cancel(self):
        survivor = Path(self.tmp.name) / 'ui-survivor'
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'add', '-qb', 'ui-survivor', str(survivor)], check=True)
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project', '-c', str(self.repo),
                         'env', 'DRUDWYN_CLIENT=' + self.clients[0], str(BIN), 'cockpit')
        self.wait_pane(pane, 'WORKSPACE')
        self.tmux('send-keys', '-t', pane, 'o')
        self.wait_pane(pane, 'RECOVER WORKTREE')
        self.wait_pane(pane, 'Open shell')
        self.wait_pane(pane, 'Restart with task')
        windows_before_cancel = self.tmux('list-windows', '-a', '-F', '#{window_id} #{pane_pid}')
        self.tmux('send-keys', '-t', pane, 't')
        self.wait_pane(pane, 'Fresh conversation')
        self.tmux('send-keys', '-t', pane, '-l', 'cancelled private task')
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'RECOVER WORKTREE')
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'WORKSPACE')
        self.assertEqual(self.tmux('list-windows', '-a', '-F', '#{window_id} #{pane_pid}'), windows_before_cancel)
        self.tmux('send-keys', '-t', pane, 'o', 'j', 's')
        for _ in range(200):
            if self.tmux('show-option', '-wqv', '-t', self.selection(self.clients[0]).split(':')[1], '@drudwyn_worktree') == str(survivor): break
            time.sleep(.025)
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', self.selection(self.clients[0]).split(':')[1], '@drudwyn_worktree'), str(survivor))

    def test_stopped_reference_is_deliberately_reused_and_metadata_loss_requires_selection(self):
        survivor = Path(self.tmp.name) / 'retained'
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'add', '-qb', 'retained', str(survivor)], check=True)
        (survivor / 'task $literal;.md').write_text('never read me')
        fakebin = Path(self.tmp.name) / 'fakebin'; fakebin.mkdir()
        shutil.copy('/bin/cat', fakebin / 'codex')
        self.env['PATH'] = str(fakebin) + os.pathsep + self.env['PATH']
        self.assertEqual(shutil.which('codex', path=self.env['PATH']), str(fakebin / 'codex'))
        args = ['workspace', 'recover', '--repo', str(self.repo), '--path', str(survivor), '--agent', 'codex', '--unassociated']
        result = self.command(*args, '--task-file', 'task $literal;.md', client=self.clients[0])
        old = result.stdout.splitlines()[0]
        pid = int(self.tmux('display-message', '-p', '-t', old, '#{pane_pid}'))
        os.kill(pid, 15)
        for _ in range(100):
            if self.tmux('display-message', '-p', '-t', old, '#{pane_dead}') == '1': break
            time.sleep(.02)
        listing = self.command('workspace', 'recover-list', '--repo', str(self.repo), client=self.clients[0]).stdout
        self.assertIn('Stopped workspace', listing)
        self.assertIn('task $literal;.md', listing)
        restarted = self.command(*args, '--use-task-reference', client=self.clients[0])
        new = restarted.stdout.splitlines()[0]
        self.assertNotEqual(old, new)
        self.assertEqual(self.tmux('display-message', '-p', '-t', old, '#{pane_dead}'), '1')
        self.assertEqual(bytes.fromhex(self.tmux('show-option', '-wqv', '-t', new, '@drudwyn_task_reference')).decode(), 'task $literal;.md')
        self.tmux('kill-window', '-t', old); self.tmux('kill-window', '-t', new)
        lost = self.command(*args, '--use-task-reference', client=self.clients[0], check=False)
        self.assertNotEqual(lost.returncode, 0)
        self.assertIn('explicitly select', lost.stderr)

    def test_unavailable_inputs_leave_git_and_windows_unchanged(self):
        survivor = Path(self.tmp.name) / 'unsafe'
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'add', '-qb', 'unsafe', str(survivor)], check=True)
        baseline = self.tmux('list-windows', '-a', '-F', '#{window_id} #{pane_pid}')
        args = ['workspace', 'recover', '--repo', str(self.repo), '--path', str(survivor)]
        def rejected(extra, needle):
            result = self.command(*args, *extra, client=self.clients[0], check=False)
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn(needle, result.stderr)
            self.assertEqual(baseline, self.tmux('list-windows', '-a', '-F', '#{window_id} #{pane_pid}'))
        rejected(['--agent', 'codex', '--unassociated', '--task-file', 'missing'], 'unavailable')
        rejected(['--agent', 'codex', '--batch', 'unknown', '--task-file', 'missing'], 'batch')
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'lock', str(survivor)], check=True)
        rejected(['--shell'], 'Locked')
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'unlock', str(survivor)], check=True)
        toolbin = Path(self.tmp.name) / 'toolbin'; toolbin.mkdir()
        for tool in ['git', 'tmux', 'sh']: (toolbin / tool).symlink_to(shutil.which(tool))
        original = self.env['PATH']; self.env['PATH'] = str(toolbin)
        (survivor / 'task.md').write_text('unread')
        rejected(['--agent', 'codex', '--unassociated', '--task-file', 'task.md'], 'unavailable')
        self.env['PATH'] = original
        shutil.rmtree(survivor)
        rejected(['--shell'], 'Prunable')
        self.assertIn('refs/heads/unsafe', subprocess.check_output(['git', '-C', str(self.repo), 'show-ref'], text=True))

    def test_concurrent_recovery_refuses_duplicate_window(self):
        survivor = Path(self.tmp.name) / 'concurrent'
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'add', '-qb', 'concurrent', str(survivor)], check=True)
        gate = Path(self.tmp.name) / 'gate'; gate.mkdir()
        ready, release = gate / 'ready', gate / 'release'
        real = shutil.which('tmux')
        shim = gate / 'tmux'
        shim.write_text('#!/bin/sh\nif [ "$1" = new-window ]; then touch ' + shlex.quote(str(ready)) + '; while [ ! -f ' + shlex.quote(str(release)) + ' ]; do sleep .02; done; fi\nexec ' + shlex.quote(real) + ' "$@"\n')
        shim.chmod(0o755)
        args = ['workspace', 'recover', '--repo', str(self.repo), '--path', str(survivor), '--shell']
        env = dict(self.env, DRUDWYN_CLIENT=self.clients[0], PATH=str(gate) + os.pathsep + self.env['PATH'])
        first = subprocess.Popen([str(BIN), *args], env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.addCleanup(lambda: first.kill() if first.poll() is None else None)
        for _ in range(200):
            if ready.exists(): break
            time.sleep(.02)
        self.assertTrue(ready.exists())
        second = self.command(*args, client=self.clients[0], check=False)
        self.assertNotEqual(second.returncode, 0)
        self.assertIn('already in progress', second.stderr)
        release.touch()
        stdout, stderr = first.communicate(timeout=15)
        self.assertEqual(first.returncode, 0, stderr)
        listing = self.command('workspace', 'recover-list', '--repo', str(self.repo), client=self.clients[0]).stdout
        self.assertIn('Live workspace | concurrent', listing)

    def test_cockpit_restart_and_narrow_redaction(self):
        survivor = Path(self.tmp.name) / 'private-survivor'
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'add', '-qb', 'private-survivor', str(survivor)], check=True)
        fakebin = Path(self.tmp.name) / 'fakebin'; fakebin.mkdir()
        shutil.copy('/bin/cat', fakebin / 'codex')
        fakepath = str(fakebin) + os.pathsep + self.env['PATH']
        self.assertEqual(shutil.which('codex', path=fakepath), str(fakebin / 'codex'))
        self.tmux('set-option', '-g', '@drudwyn-redact-labels', 'on')
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project', '-c', str(self.repo),
                         'env', 'DRUDWYN_CLIENT=' + self.clients[0], 'PATH=' + fakepath, str(BIN), 'cockpit')
        self.wait_pane(pane, 'WORKSPACE')
        self.tmux('send-keys', '-t', pane, 'o', 'j')
        for width in [48, 64, 80, 120, 160]:
            self.tmux('resize-window', '-t', pane, '-x', str(width), '-y', '24')
            screen = self.wait_pane(pane, 'Recover coordinator')
            self.assertIn('Open shell', screen)
            self.assertIn('Restart with task', screen)
            self.assertNotIn('private-survivor', screen)
            self.assertNotIn(str(self.repo), screen)
            Path('/tmp/drudwyn-ticket08-recovery-' + str(width) + '.txt').write_text(screen)
        self.tmux('send-keys', '-t', pane, 't')
        self.wait_pane(pane, 'Fresh conversation')
        self.tmux('send-keys', '-t', pane, '-l', 'private recovery task')
        screen = self.wait_pane(pane, '[redacted]')
        self.assertNotIn('private recovery task', screen)
        self.tmux('send-keys', '-t', pane, 'F6')
        for _ in range(200):
            window = self.selection(self.clients[0]).split(':')[1]
            if self.tmux('show-option', '-wqv', '-t', window, '@drudwyn_delivery') == 'sent': break
            time.sleep(.025)
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', window, '@drudwyn_delivery'), 'sent')
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', window, '@drudwyn_batch'), '')
        self.assertNotIn('private recovery task', self.tmux('show-options', '-w', '-t', window))

    def test_external_window_race_is_reported_without_destructive_cleanup(self):
        survivor = Path(self.tmp.name) / 'race'
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'add', '-qb', 'race', str(survivor)], check=True)
        (survivor / 'untracked').write_text('preserve')
        gate = Path(self.tmp.name) / 'gate'; gate.mkdir()
        real = shutil.which('tmux')
        shim = gate / 'tmux'
        shim.write_text('#!/bin/sh\nif [ "$1" = new-window ]; then ' + shlex.quote(real) + ' new-window -d -t project -c ' + shlex.quote(str(survivor)) + ' sleep 30; fi\nexec ' + shlex.quote(real) + ' "$@"\n')
        shim.chmod(0o755)
        self.env['PATH'] = str(gate) + os.pathsep + self.env['PATH']
        result = self.command('workspace', 'recover', '--repo', str(self.repo), '--path', str(survivor), '--shell', client=self.clients[0], check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Another window appeared', result.stderr)
        self.assertIn('no task sent', result.stderr)
        self.assertEqual((survivor / 'untracked').read_text(), 'preserve')
        self.assertEqual(len(self.tmux('list-windows', '-t', 'project', '-F', '#{window_id}').splitlines()), 4)

    def test_coordinator_recovery_uses_literal_checkout_identity(self):
        survivor = Path(self.tmp.name) / 'coordinator $literal;'
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'add', '-qb', 'planning', str(survivor)], check=True)
        result = self.command('workspace', 'recover', '--repo', str(self.repo), '--path', str(survivor), '--shell', '--coordinator', client=self.clients[0], check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.tmux('show-option', '-qv', '-t', 'project', '@drudwyn_coordinator'), result.stdout.strip())

    def test_failed_restart_retains_existing_commits_files_and_stopped_pane(self):
        survivor = Path(self.tmp.name) / 'early-exit'
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'add', '-qb', 'early-exit', str(survivor)], check=True)
        (survivor / 'keep').write_text('untracked work')
        (survivor / 'task.md').write_text('task file')
        before = subprocess.check_output(['git', '-C', str(self.repo), 'worktree', 'list', '--porcelain'])
        commit = subprocess.check_output(['git', '-C', str(survivor), 'rev-parse', 'HEAD'])
        fakebin = Path(self.tmp.name) / 'fakebin'; fakebin.mkdir()
        shutil.copy('/bin/true', fakebin / 'codex')
        self.env['PATH'] = str(fakebin) + os.pathsep + self.env['PATH']
        self.assertEqual(shutil.which('codex', path=self.env['PATH']), str(fakebin / 'codex'))
        result = self.command('workspace', 'recover', '--repo', str(self.repo), '--path', str(survivor), '--agent', 'codex', '--unassociated', '--task-file', 'task.md', client=self.clients[0], check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('retained worktree', result.stderr)
        self.assertEqual((survivor / 'keep').read_text(), 'untracked work')
        self.assertEqual(commit, subprocess.check_output(['git', '-C', str(survivor), 'rev-parse', 'HEAD']))
        self.assertEqual(before, subprocess.check_output(['git', '-C', str(self.repo), 'worktree', 'list', '--porcelain']))
        self.assertIn('1', self.tmux('list-panes', '-a', '-F', '#{pane_dead}'))

    def paused_coordinator_recovery(self, name):
        checkout = Path(self.tmp.name) / name
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'add', '-qb', name, str(checkout)], check=True)
        gate = Path(self.tmp.name) / ('gate-' + name); gate.mkdir()
        ready, release = gate / 'ready', gate / 'release'
        shim = gate / 'tmux'
        shim.write_text('#!/bin/sh\nif [ "$1" = new-window ]; then touch ' + shlex.quote(str(ready)) + '; while [ ! -f ' + shlex.quote(str(release)) + ' ]; do sleep .02; done; fi\nexec ' + shlex.quote(shutil.which('tmux')) + ' "$@"\n')
        shim.chmod(0o755)
        env = dict(self.env, DRUDWYN_CLIENT=self.clients[0], PATH=str(gate) + os.pathsep + self.env['PATH'])
        process = subprocess.Popen([str(BIN), 'workspace', 'recover', '--repo', str(self.repo), '--path', str(checkout), '--shell', '--coordinator'], env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        def cleanup():
            release.touch()
            if process.poll() is None: process.kill()
            process.communicate(timeout=10)
        self.addCleanup(cleanup)
        for _ in range(200):
            if ready.exists(): break
            self.assertIsNone(process.poll())
            time.sleep(.02)
        self.assertTrue(ready.exists())
        return process, release, checkout

    def test_coordinator_manual_selection_wins_over_pending_recovery(self):
        process, release, checkout = self.paused_coordinator_recovery('pending')
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        release.touch()
        stdout, stderr = process.communicate(timeout=15)
        self.assertNotEqual(process.returncode, 0, stdout)
        self.assertIn('Coordinator changed', stderr)
        self.assertIn('retained', stderr)
        self.assertEqual(self.tmux('show-option', '-qv', '-t', 'project', '@drudwyn_coordinator'), self.home)
        self.assertTrue(checkout.is_dir())
        self.assertEqual(len(self.tmux('list-windows', '-t', 'project', '-F', '#{window_id}').splitlines()), 3)

    def test_coordinator_recoveries_in_different_checkouts_do_not_overwrite(self):
        process, release, checkout = self.paused_coordinator_recovery('first')
        other = Path(self.tmp.name) / 'second'
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'add', '-qb', 'second', str(other)], check=True)
        selected = self.command('workspace', 'recover', '--repo', str(self.repo), '--path', str(other), '--shell', '--coordinator', client=self.clients[1]).stdout.strip()
        release.touch()
        stdout, stderr = process.communicate(timeout=15)
        self.assertNotEqual(process.returncode, 0, stdout)
        self.assertIn('Coordinator changed', stderr)
        self.assertEqual(self.tmux('show-option', '-qv', '-t', 'project', '@drudwyn_coordinator'), selected)
        self.assertTrue(checkout.is_dir() and other.is_dir())
        self.assertTrue(self.selection(self.clients[1]).endswith(':' + selected))
        self.assertEqual(len(self.tmux('list-windows', '-t', 'project', '-F', '#{window_id}').splitlines()), 4)

    def test_cockpit_recovers_selected_unmanaged_agent_in_literal_repository(self):
        repo = Path(self.tmp.name) / 'repo $literal'
        subprocess.run(['git', 'init', '-q', '-b', 'main', str(repo)], check=True)
        subprocess.run(['git', '-C', str(repo), '-c', 'user.name=Test', '-c', 'user.email=test@example.invalid', 'commit', '-qm', 'initial', '--allow-empty'], check=True)
        survivor = Path(self.tmp.name) / 'literal-survivor'
        subprocess.run(['git', '-C', str(repo), 'worktree', 'add', '-qb', 'literal-survivor', str(survivor)], check=True)
        fake = Path(self.tmp.name) / 'fake-worker' / 'codex'
        self.assertEqual(fake.resolve(), Path(shutil.which('sleep')).resolve())
        worker = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project', '-n', 'literal-worker', '-c', str(repo), str(fake), '300')
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', worker, '@drudwyn_launch_checkout'), '')
        self.assertIn('literal-survivor', self.command('workspace', 'recover-list', '--repo', str(repo), client=self.clients[0]).stdout)
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project', '-c', str(self.repo), 'env', 'DRUDWYN_CLIENT=' + self.clients[0], str(BIN), 'cockpit')
        self.wait_pane(pane, 'WORKSPACE')
        self.tmux('send-keys', '-t', pane, '-l', '/literal-worker')
        time.sleep(.1)
        self.tmux('send-keys', '-t', pane, 'Enter', 'o')
        self.wait_pane(pane, 'RECOVER WORKTREE')
        self.wait_pane(pane, 'literal-survivor')
        self.tmux('send-keys', '-t', pane, 'j', 's')
        for _ in range(200):
            window = self.selection(self.clients[0]).split(':')[1]
            if self.tmux('show-option', '-wqv', '-t', window, '@drudwyn_branch') == 'literal-survivor': break
            time.sleep(.02)
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', window, '@drudwyn_branch'), 'literal-survivor')

    def test_coordinator_mutation_keeps_guard_after_parent_death(self):
        gate = Path(self.tmp.name) / 'assignment-gate'; gate.mkdir()
        ready, release = gate / 'ready', gate / 'release'
        shim = gate / 'tmux'
        shim.write_text('#!/bin/sh\nif [ "$1" = set-option ] && [ "$4" = @drudwyn_coordinator ]; then touch ' + shlex.quote(str(ready)) + '; while [ ! -f ' + shlex.quote(str(release)) + ' ]; do sleep .02; done; fi\nexec ' + shlex.quote(shutil.which('tmux')) + ' "$@"\n')
        shim.chmod(0o755)
        env = dict(self.env, DRUDWYN_CLIENT=self.clients[0], PATH=str(gate) + os.pathsep + self.env['PATH'])
        process = subprocess.Popen([str(BIN), 'coordinator', 'set', '--window', self.home], env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        def cleanup():
            release.touch()
            if process.poll() is None: process.kill()
            process.communicate(timeout=10)
        self.addCleanup(cleanup)
        for _ in range(200):
            if ready.exists(): break
            self.assertIsNone(process.poll())
            time.sleep(.02)
        self.assertTrue(ready.exists())
        process.kill(); process.wait(timeout=5)
        result = self.command('coordinator', 'set', '--window', self.worker, client=self.clients[0], check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('another update is still in progress', result.stderr)
        release.touch()
        process.communicate(timeout=10)
        self.command('coordinator', 'set', '--window', self.worker, client=self.clients[0])
        self.assertEqual(self.tmux('show-option', '-qv', '-t', 'project', '@drudwyn_coordinator'), self.worker)

    def test_cockpit_missing_selected_pane_does_not_borrow_invoking_repo(self):
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project', '-c', str(self.repo), 'env', 'DRUDWYN_CLIENT=' + self.clients[0], str(BIN), 'cockpit')
        self.wait_pane(pane, 'WORKSPACE')
        self.tmux('kill-window', '-t', self.worker)
        self.tmux('send-keys', '-t', pane, 'o')
        screen = self.wait_pane(pane, 'Selected pane changed')
        self.assertNotIn('RECOVER WORKTREE', screen)
        self.assertIn('ERROR', screen)

if __name__ == '__main__':
    names = [n for n in RecoveryTest.__dict__ if n.startswith('test_') and (len(sys.argv) == 1 or sys.argv[1] in n)]
    result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(RecoveryTest(n) for n in names))
    sys.exit(not result.wasSuccessful())
