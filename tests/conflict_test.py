"""Conflict handoff and guarded resolution through real Git/tmux command seams."""
import os
import shutil
import time
from pathlib import Path
import subprocess
import sys
import unittest
from worker_integration_test import WorkerIntegrationTest
from independent_navigation_test import BIN

class ConflictTest(WorkerIntegrationTest):
    def conflict(self):
        (self.repo/'worker.txt').write_text('conflicting target content')
        self.git('add', '.')
        self.git('commit', '-qm', 'target conflict')
        self.target_before = self.git('rev-parse', 'HEAD')
        result = self.integrate('--apply', self.token(self.integrate()), check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.git('rev-parse', 'MERGE_HEAD'), self.commit)
        return result

    def action(self, *args, check=True):
        return self.command('workspace', 'conflict', '--path', str(self.repo), *args,
                            client=self.clients[0], check=check)

    def test_conflict_is_inspectable_and_abort_is_repeatable(self):
        self.conflict()
        status = self.action().stdout
        self.assertIn('Conflict retained', status)
        self.assertIn(self.commit, status)
        self.assertIn(self.target_before, status)
        self.assertIn(str(self.repo), status)
        self.assertIn('checks unknown', status)
        self.assertIn('Aborted', self.action('--abort').stdout)
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.target_before)
        self.assertEqual(self.git('status', '--porcelain'), '')
        self.assertIn('Aborted', self.action('--abort').stdout)
        self.assertTrue(self.source.is_dir())

    def coordinator_agent(self):
        agent = Path(self.tmp.name)/'receiver-codex'
        agent.mkdir()
        executable = agent/'codex'
        executable.symlink_to(sys.executable)
        self.assertEqual(executable.resolve(), Path(sys.executable).resolve())
        script = agent/'receive.py'
        script.write_text('import sys, time\nfor line in sys.stdin:\n print("HANDOFF_RECEIVED", flush=True)\n')
        pane = self.tmux('display-message', '-p', '-t', self.home, '#{pane_id}')
        self.tmux('respawn-pane', '-k', '-t', pane, '-c', str(self.repo), str(executable), str(script))
        self.assertEqual(Path('/proc/'+self.tmux('display-message', '-p', '-t', pane, '#{pane_pid}')+'/exe').resolve(), Path(sys.executable).resolve())
        return pane

    def test_ordinary_coordinator_receives_one_transient_handoff(self):
        pane = self.coordinator_agent()
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', self.home, '@drudwyn_launch_checkout'), '')
        self.conflict()
        status = self.action().stdout
        self.assertIn('Handoff: sent', status)
        output = self.tmux('capture-pane', '-p', '-t', pane)
        self.assertEqual(output.count('HANDOFF_RECEIVED'), 1)
        for _ in range(3):
            self.action()
        self.assertEqual(self.tmux('capture-pane', '-p', '-t', pane).count('HANDOFF_RECEIVED'), 1)
        self.assertEqual(self.tmux('list-buffers'), '')
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', self.home, '@drudwyn_launch_checkout'), '')
        self.assertIn('Handoff: sent', self.action('--retry').stdout)
        self.assertEqual(self.tmux('capture-pane', '-p', '-t', pane).count('HANDOFF_RECEIVED'), 2)

    def test_cockpit_conflict_continue_abort_and_narrow_redaction(self):
        before = [self.selection(c) for c in self.clients]
        (self.repo/'worker.txt').write_text('conflicting target content')
        self.git('add', '.')
        self.git('commit', '-qm', 'target conflict')
        self.target_before = self.git('rev-parse', 'HEAD')
        pane = self.integration_ui()
        self.wait_pane(pane, 'MERGE CHANGES')
        self.tmux('send-keys', '-t', pane, 'y')
        output = self.wait_pane(pane, 'INTEGRATION CONFLICT')
        self.assertIn('Handoff: not_sent', output)
        self.tmux('send-keys', '-t', pane, 'c')
        self.wait_pane(pane, 'Unmerged entries')
        self.resolve()
        self.tmux('send-keys', '-t', pane, 'r')
        self.wait_pane(pane, 'Continue available')
        self.tmux('send-keys', '-t', pane, 'c')
        self.wait_pane(pane, 'Completed')
        merged = self.git('rev-parse', 'HEAD')
        self.tmux('send-keys', '-t', pane, 'c')
        self.assertEqual(self.git('rev-parse', 'HEAD'), merged)
        self.assertEqual([self.selection(c) for c in self.clients], before)
        self.tmux('kill-window', '-t', pane)
        self.git('reset', '--hard', self.target_before)  # isolated fixture
        self.integrate('--apply', self.token(self.integrate()), check=False)
        self.tmux('set', '-g', '@drudwyn-redact-labels', 'on')
        pane = self.integration_ui()
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'WORKSPACE COCKPIT')
        time.sleep(.8)
        self.tmux('send-keys', '-t', pane, 'C')
        self.wait_pane(pane, 'INTEGRATION CONFLICT')
        self.tmux('resize-window', '-t', pane, '-x', '48', '-y', '24')
        # Resize can temporarily crop the old frame; wait for the new footer.
        output = self.wait_pane(pane, '[c] Continue')
        self.assertIn('[redacted]', output)
        self.assertNotIn(str(self.repo), output)
        self.assertNotIn(self.commit, output)
        self.assertIn('[c] Continue', output)
        self.assertIn('[a] Abort', output)
        Path('/tmp/drudwyn-ticket12-narrow-redacted.txt').write_text(output)
        self.tmux('send-keys', '-t', pane, 'a')
        self.wait_pane(pane, 'Aborted')
        self.assertEqual(self.git('status', '--porcelain'), '')
        self.assertEqual([self.selection(c) for c in self.clients], before)

    def test_continue_ignores_inherited_editor_and_abort_failure_preserves_edits(self):
        self.conflict()
        self.resolve()
        marker = Path(self.tmp.name)/'editor-ran'
        editor = Path(self.tmp.name)/'editor'
        editor.write_text(f'#!/bin/sh\ntouch {marker}\nexit 1\n')
        editor.chmod(0o755)
        result = subprocess.run([str(BIN), 'workspace', 'conflict', '--path', str(self.repo), '--continue'],
                                env={**self.env, 'GIT_EDITOR': str(editor)}, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(marker.exists())
        self.git('reset', '--hard', self.target_before)  # isolated fixture
        self.integrate('--apply', self.token(self.integrate()), check=False)
        self.resolve()
        (self.repo/'worker.txt').write_text('unstaged resolution to preserve')
        result = self.action('--abort', check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('No reset or fallback', result.stderr)
        self.assertEqual((self.repo/'worker.txt').read_text(), 'unstaged resolution to preserve')
        self.assertEqual(self.git('rev-parse', 'MERGE_HEAD'), self.commit)

    def test_missing_wrong_checkout_and_ambiguous_coordinator_require_deliberate_retry(self):
        self.conflict()
        self.assertIn('Handoff: not_sent', self.action().stdout)
        pane = self.coordinator_agent()
        self.action()  # inspect never retries even after recovery
        self.assertNotIn('HANDOFF_RECEIVED', self.tmux('capture-pane', '-p', '-t', pane))
        before = self.selection(self.clients[1])
        self.action('--open')
        self.assertEqual(self.selection(self.clients[0]).split(':')[-1], self.home)
        self.assertEqual(self.selection(self.clients[1]), before)
        other = self.tmux('split-window', '-d', '-P', '-F', '#{pane_id}', '-t', self.home,
                          '-c', str(self.repo), str(Path(self.tmp.name)/'fake-worker/codex'), '300')
        self.assertNotEqual(self.action('--retry', check=False).returncode, 0)
        self.tmux('kill-pane', '-t', other)
        self.tmux('respawn-pane', '-k', '-t', pane, '-c', str(self.source),
                  str(Path(self.tmp.name)/'receiver-codex/codex'), str(Path(self.tmp.name)/'receiver-codex/receive.py'))
        self.assertIn('another checkout', self.action('--retry', check=False).stderr)
        self.assertNotIn('HANDOFF_RECEIVED', self.tmux('capture-pane', '-p', '-t', pane))
        self.tmux('respawn-pane', '-k', '-t', pane, '-c', str(self.repo),
                  str(Path(self.tmp.name)/'receiver-codex/codex'), str(Path(self.tmp.name)/'receiver-codex/receive.py'))
        self.assertIn('Handoff: sent', self.action('--retry').stdout)

    def test_uncertain_delivery_cleans_buffer_and_never_resends_on_inspect(self):
        pane = self.coordinator_agent()
        self.conflict()
        shim = Path(self.tmp.name)/'shim'
        shim.mkdir()
        wrapper = shim/'tmux'
        wrapper.write_text('#!/bin/sh\nif [ "$1" = send-keys ]; then exit 1; fi\nexec '+shutil.which('tmux')+' "$@"\n')
        wrapper.chmod(0o755)
        result = subprocess.run([str(BIN), 'workspace', 'conflict', '--path', str(self.repo), '--retry'],
                                env={**self.env, 'PATH': str(shim)+os.pathsep+os.environ['PATH']}, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('uncertain', result.stderr)
        self.assertEqual(self.tmux('list-buffers'), '')
        for _ in range(3):
            self.assertIn('Handoff: uncertain', self.action().stdout)
        self.assertEqual(self.tmux('capture-pane', '-p', '-t', pane).count('HANDOFF_RECEIVED'), 1)
        for args in [('show-options', '-s'), ('show-options', '-w', '-t', self.home)]:
            self.assertNotIn('Resolve the retained Git merge', self.tmux(*args))

    def test_missing_project_can_be_explicitly_bound_after_recovery(self):
        project = self.batch.split('/')[0]
        self.tmux('set', '-u', '-t', project, '@drudwyn_project_repo')
        self.conflict()
        self.assertIn('not_sent', self.action().stdout)
        pane = self.coordinator_agent()
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        self.assertIn('association unknown', self.action('--retry', check=False).stderr)
        result = self.action('--project', project, '--retry')
        self.assertIn('Handoff: sent', result.stdout)
        self.assertEqual(self.tmux('capture-pane', '-p', '-t', pane).count('HANDOFF_RECEIVED'), 1)

    def recovery_agent(self):
        directory = Path(self.tmp.name)/'recovery-agent'
        directory.mkdir()
        source = directory/'receiver.rs'
        source.write_text('use std::io::{self, BufRead}; fn main() { for line in io::stdin().lock().lines() { if line.is_err() { break; } println!("RECOVERED_HANDOFF"); } }')
        subprocess.run(['rustc', str(source), '-o', str(directory/'codex')], check=True)
        self.assertTrue((directory/'codex').is_file())
        return directory

    def test_recover_coordinator_agent_then_retry_without_manual_task_or_replacing_original(self):
        self.tmux("kill-window", "-t", "project:worker")
        self.conflict()
        directory = self.recovery_agent()
        before_other = self.selection(self.clients[1])
        env = {**self.env, 'DRUDWYN_CLIENT': self.clients[0], 'PATH': str(directory)+os.pathsep+os.environ['PATH']}
        result = subprocess.run([str(BIN), 'workspace', 'conflict', '--path', str(self.repo), '--recover-agent', 'codex'],
                                env=env, text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('not sent', result.stdout)
        project = self.batch.split('/')[0]
        coordinator = self.tmux('show-option', '-qv', '-t', project, '@drudwyn_coordinator')
        self.assertNotEqual(coordinator, self.home)
        self.assertIn(self.home, self.tmux('list-windows', '-t', project, '-F', '#{window_id}'))
        pane = self.tmux('display-message', '-p', '-t', coordinator, '#{pane_id}')
        pid = self.tmux('display-message', '-p', '-t', pane, '#{pane_pid}')
        self.assertEqual(Path('/proc/'+pid+'/exe').resolve(), (directory/'codex').resolve())
        self.assertNotIn('RECOVERED_HANDOFF', self.tmux('capture-pane', '-p', '-t', pane))
        self.assertIn('Handoff: sent', self.action('--retry').stdout)
        self.assertEqual(self.tmux('capture-pane', '-p', '-t', pane).count('RECOVERED_HANDOFF'), 1)
        self.assertEqual(self.selection(self.clients[1]), before_other)
        self.assertEqual(self.git('rev-parse', 'MERGE_HEAD'), self.commit)
        self.assertEqual(self.git('branch', '--show-current'), 'main')
        again = subprocess.run([str(BIN), 'workspace', 'conflict', '--path', str(self.repo), '--recover-agent', 'codex'],
                               env=env, text=True, capture_output=True)
        self.assertNotEqual(again.returncode, 0)
        self.assertIn('live agent', again.stderr)
        self.assertEqual(self.tmux('show-option', '-qv', '-t', project, '@drudwyn_coordinator'), coordinator)

    def test_process_replacement_after_paste_is_uncertain_and_not_submitted(self):
        pane = self.coordinator_agent()
        self.conflict()
        shim = Path(self.tmp.name)/'replace-shim'
        shim.mkdir()
        wrapper = shim/'tmux'
        import shlex
        actual = shutil.which('tmux')
        replacement = shlex.join([actual, 'respawn-pane', '-k', '-t', pane, '-c', str(self.repo),
                                  str(Path(self.tmp.name)/'receiver-codex/codex'), str(Path(self.tmp.name)/'receiver-codex/receive.py')])
        wrapper.write_text('#!/bin/sh\n'+actual+' "$@"\nresult=$?\nif [ "$1" = paste-buffer ]; then '+replacement+'; fi\nexit "$result"\n')
        wrapper.chmod(0o755)
        result = subprocess.run([str(BIN), 'workspace', 'conflict', '--path', str(self.repo), '--retry'],
                                env={**self.env, 'PATH': str(shim)+os.pathsep+os.environ['PATH']}, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('uncertain', result.stderr)
        self.assertIn('Handoff: uncertain', self.action().stdout)
        self.assertNotIn('HANDOFF_RECEIVED', self.tmux('capture-pane', '-p', '-t', pane))
        self.assertEqual(self.tmux('list-buffers'), '')
        self.assertIn('Handoff: sent', self.action('--retry').stdout)
        self.assertEqual(self.tmux('capture-pane', '-p', '-t', pane).count('HANDOFF_RECEIVED'), 1)

    def test_lost_receipt_and_inherited_git_addressing_never_adopt_an_operation(self):
        self.conflict()
        self.resolve()
        other = Path(self.tmp.name)/'unrelated'
        self.git('init', '-q', '-b', 'main', str(other))
        self.git('-c', 'user.name=Test', '-c', 'user.email=test@example.invalid', 'commit', '--allow-empty', '-qm', 'other', path=other)
        env = {**self.env, 'GIT_DIR': str(other/'.git'), 'GIT_WORK_TREE': str(other)}
        result = subprocess.run([str(BIN), 'workspace', 'conflict', '--path', str(self.repo), '--continue'], env=env, text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.git('reset', '--hard', self.target_before)  # fixture only
        self.integrate('--apply', self.token(self.integrate()), check=False)
        self.resolve()
        for line in self.tmux('show-options', '-s').splitlines():
            if line.startswith('@drudwyn_conflict_'):
                self.tmux('set-option', '-su', line.split()[0])
        for action in ('--continue', '--abort', '--retry'):
            result = self.action(action, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('No live conflict receipt', result.stderr)
            self.assertEqual(self.git('rev-parse', 'MERGE_HEAD'), self.commit)
        self.assertEqual((self.repo/'worker.txt').read_text(), 'deliberate coordinator resolution')

    def test_cockpit_recovers_agent_then_explicitly_retries_and_opens_client_locally(self):
        self.tmux('kill-window', '-t', 'project:worker')
        self.conflict()
        directory = self.recovery_agent()
        self.tmux('set', '-g', '@drudwyn-default-agent', 'codex')
        pane = self.integration_ui(['PATH='+str(directory)+os.pathsep+os.environ['PATH']])
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'WORKSPACE COCKPIT')
        time.sleep(.8)
        self.tmux('send-keys', '-t', pane, 'C')
        self.wait_pane(pane, 'INTEGRATION CONFLICT')
        before_other = self.selection(self.clients[1])
        self.tmux('send-keys', '-t', pane, 'v')
        output = self.wait_pane(pane, 'New conversation; task not sent')
        Path('/tmp/drudwyn-ticket12-recovered-agent.txt').write_text(output)
        project = self.batch.split('/')[0]
        coordinator = self.tmux('show-option', '-qv', '-t', project, '@drudwyn_coordinator')
        target = self.tmux('display-message', '-p', '-t', coordinator, '#{pane_id}')
        self.assertNotIn('RECOVERED_HANDOFF', self.tmux('capture-pane', '-p', '-t', target))
        self.tmux('send-keys', '-t', pane, 't')
        self.wait_pane(pane, 'Handoff: sent')
        self.assertEqual(self.tmux('capture-pane', '-p', '-t', target).count('RECOVERED_HANDOFF'), 1)
        self.tmux('send-keys', '-t', pane, 'r')
        self.wait_pane(pane, 'Handoff: sent')
        self.assertEqual(self.tmux('capture-pane', '-p', '-t', target).count('RECOVERED_HANDOFF'), 1)
        self.tmux('send-keys', '-t', pane, 'o')
        self.assertEqual(self.selection(self.clients[1]), before_other)
        self.assertEqual(self.git('rev-parse', 'MERGE_HEAD'), self.commit)

    def test_continue_lock_survives_parent_death_until_git_hook_finishes(self):
        self.conflict()
        self.resolve()
        arrived = Path(self.tmp.name)/'hook-arrived'
        release = Path(self.tmp.name)/'release-hook'
        hook = self.repo/'.git/hooks/pre-commit'
        hook.write_text(f'#!/bin/sh\ntouch {arrived}\nwhile [ ! -e {release} ]; do sleep .02; done\n')
        hook.chmod(0o755)
        parent = subprocess.Popen([str(BIN), 'workspace', 'conflict', '--path', str(self.repo), '--continue'],
                                  env=self.env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.addCleanup(lambda: release.touch())
        self.addCleanup(lambda: parent.poll() is not None or parent.kill())
        deadline = time.monotonic()+5
        while not arrived.exists():
            self.assertIsNone(parent.poll())
            self.assertLess(time.monotonic(), deadline)
            time.sleep(.02)
        self.assertIn('already in progress', self.action('--abort', check=False).stderr)
        parent.kill()
        parent.wait(timeout=3)
        self.assertIn('already in progress', self.action('--abort', check=False).stderr)
        self.command('scan')
        release.touch()
        deadline = time.monotonic()+5
        while self.git('rev-parse', '--verify', 'MERGE_HEAD', check=False):
            self.assertLess(time.monotonic(), deadline)
            time.sleep(.02)
        self.assertIn('Completed', self.action('--continue').stdout)

    def test_stale_completed_receipt_does_not_hide_new_merge_failure(self):
        self.conflict()
        self.resolve()
        self.action('--continue')
        self.git('merge', '--ff-only', 'main', path=self.source)
        (self.source/'collision.txt').write_text('new worker commit')
        self.git('add', '.', path=self.source)
        self.git('commit', '-qm', 'new worker change', path=self.source)
        (self.repo/'.git/info/exclude').write_text('collision.txt\n')
        (self.repo/'collision.txt').write_text('ignored user work')
        pane = self.integration_ui()
        self.wait_pane(pane, 'MERGE CHANGES')
        self.tmux('send-keys', '-t', pane, 'y')
        self.wait_pane(pane, 'Git merge')
        self.assertEqual((self.repo/'collision.txt').read_text(), 'ignored user work')

    def resolve(self):
        (self.repo/'worker.txt').write_text('deliberate coordinator resolution')
        self.git('add', 'worker.txt')

    def test_continue_requires_resolution_and_never_duplicates_commit(self):
        self.conflict()
        failed = self.action('--continue', check=False)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn('Unmerged entries', failed.stderr)
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.target_before)
        self.resolve()
        self.assertIn('Continue available', self.action().stdout)
        result = self.action('--continue')
        self.assertIn('Completed', result.stdout)
        self.assertIn('checks unknown', result.stdout)
        merged = self.git('rev-parse', 'HEAD')
        self.assertEqual(self.git('rev-list', '--parents', '-n', '1', 'HEAD').split()[1:], [self.target_before, self.commit])
        self.assertIn('Completed', self.action('--continue').stdout)
        self.assertEqual(self.git('rev-parse', 'HEAD'), merged)
        self.assertEqual((self.repo/'worker.txt').read_text(), 'deliberate coordinator resolution')

    def test_abort_restart_same_refs_is_not_the_expected_operation(self):
        self.conflict()
        self.git('merge', '--abort')
        self.git('merge', '--no-ff', '--no-edit', self.commit, check=False)
        self.assertEqual(self.git('rev-parse', 'MERGE_HEAD'), self.commit)
        self.resolve()
        for action in ('--continue', '--abort', '--retry'):
            result = self.action(action, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('operation changed or was replaced', result.stderr)
            self.assertEqual(self.git('rev-parse', 'HEAD'), self.target_before)
            self.assertEqual(self.git('rev-parse', 'MERGE_HEAD'), self.commit)
            self.assertEqual((self.repo/'worker.txt').read_text(), 'deliberate coordinator resolution')

    def test_external_completion_and_abort_reconcile_without_repeating(self):
        self.conflict()
        self.resolve()
        self.git('commit', '--no-edit')
        merged = self.git('rev-parse', 'HEAD')
        for action in ('--continue', '--abort', '--retry'):
            self.assertIn('Completed', self.action(action).stdout)
            self.assertEqual(self.git('rev-parse', 'HEAD'), merged)
        # A new conflict receives a fresh live receipt.
        self.git('reset', '--hard', self.target_before)  # disposable test fixture only
        result = self.integrate('--apply', self.token(self.integrate()), check=False)
        self.assertNotEqual(result.returncode, 0)
        self.git('merge', '--abort')
        for action in ('--continue', '--abort', '--retry'):
            self.assertIn('Aborted', self.action(action).stdout)
            self.assertEqual(self.git('rev-parse', 'HEAD'), self.target_before)

    def test_failed_continue_preserves_resolutions_and_private_hook_output(self):
        self.conflict()
        self.resolve()
        hook = self.repo/'.git/hooks/pre-commit'
        hook.write_text('#!/bin/sh\necho PRIVATE_RESOLUTION_HOOK\nexit 1\n')
        hook.chmod(0o755)
        result = self.action('--continue', check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('work retained', result.stderr)
        self.assertNotIn('PRIVATE_RESOLUTION_HOOK', result.stdout + result.stderr)
        self.assertEqual(self.git('rev-parse', 'MERGE_HEAD'), self.commit)
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.target_before)
        self.assertEqual((self.repo/'worker.txt').read_text(), 'deliberate coordinator resolution')

if __name__ == '__main__':
    names = [n for n in ConflictTest.__dict__ if n.startswith('test_') and (len(sys.argv)==1 or sys.argv[1] in n)]
    result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(ConflictTest(n) for n in names))
    sys.exit(not result.wasSuccessful())
