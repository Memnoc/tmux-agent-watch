"""Explicit assembled checks at the command and real terminal seams."""
from pathlib import Path
import os
import subprocess
import sys
import time
import unittest
from worker_integration_test import WorkerIntegrationTest
from independent_navigation_test import BIN

class VerificationTest(WorkerIntegrationTest):
    def verify(self, command=None, path=None, label='selected-check', env=None):
        args = [str(BIN), 'workspace', 'verify', '--path', str(path or self.repo)]
        if command is not None: args += ['--check', label, '--command-stdin']
        return subprocess.run(args, input=command, text=True, capture_output=True,
                              env=env or self.env)

    def test_explicit_visible_check_receipt_and_untracked_failure(self):
        self.integrate('--apply', self.token(self.integrate()))
        self.assertIn('Not verified', self.verify().stdout)
        (self.repo/'application-input').write_text('fail')
        command = 'printf "VISIBLE_CHECK_OUTPUT\\n"; test ! -e application-input'
        result = self.verify(command)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('VISIBLE_CHECK_OUTPUT', result.stdout)
        self.assertIn('Failed', result.stdout)
        status = self.verify().stdout
        for value in ['selected-check', str(self.repo), self.commit, 'Started:', 'Ended:', 'Exit: 1', 'point-in-time', 'ignored']:
            self.assertIn(value, status)
        self.assertNotIn('VISIBLE_CHECK_OUTPUT', status)
        self.assertEqual(self.git('status', '--porcelain', path=self.source), '')
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.commit)
        (self.repo/'application-input').unlink()
        self.assertIn('Passed', self.verify(command).stdout)
        self.assertIn('Passed', self.verify().stdout)
        self.assertIn('Not verified', self.verify(path=self.source).stdout)
        metadata = self.tmux('show-options', '-s')
        self.assertNotIn('VISIBLE_CHECK_OUTPUT', metadata)
        self.assertNotIn('test ! -e', metadata)

    def test_cockpit_selects_batch_destination_and_runs_visibly(self):
        self.tmux("set-environment", "-gu", "NO_COLOR")
        before = [self.selection(c) for c in self.clients]
        pane = self.integration_ui()
        self.wait_pane(pane, 'MERGE CHANGES')
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'WORKSPACE COCKPIT')
        self.wait_pane(pane, 'Snapshot: r refresh')
        self.tmux('send-keys', '-t', pane, 'V')
        output = self.wait_pane(pane, 'CHECK MERGED CHANGES')
        self.assertIn(str(self.repo), output)
        self.tmux('send-keys', '-t', pane, 'Tab')
        self.tmux('send-keys', '-t', pane, '-l', 'ui-check')
        self.tmux('send-keys', '-t', pane, 'Tab')
        self.tmux('send-keys', '-t', pane, '-l', 'printf "UI_VISIBLE_OUTPUT"; test -f common.txt')
        self.tmux('send-keys', '-t', pane, 'F5')
        output = self.wait_pane(pane, '[Enter] Back to checks')
        self.assertIn('UI_VISIBLE_OUTPUT', output)
        self.assertIn('Passed', output)
        self.assertIn('CHECK OUTPUT', output)
        self.assertIn('[OK] Passed', output)
        self.assertNotIn('Tested revision:', output)
        self.assertNotIn('Started:', output)
        self.assertNotIn('Reported worker checks:', output)
        colored = self.tmux('capture-pane', '-p', '-e', '-t', pane)
        self.assertIn('38;2;156;207;216', colored)
        self.tmux('send-keys', '-t', pane, 'Enter')
        screen = self.wait_pane(pane, 'CHECK MERGED CHANGES')
        self.assertNotIn('PassedCheck', screen)
        self.assertTrue(any(line.strip().startswith('Check:') for line in screen.splitlines()), screen)
        self.assertEqual([self.selection(c) for c in self.clients], before)
        self.assertIn('Passed', self.verify().stdout)
        self.assertIn('Not verified', self.verify(path=self.source).stdout)

    def test_visible_failed_check_has_distinct_result_and_redacts_metadata(self):
        self.tmux('set-environment', '-gu', 'NO_COLOR')
        self.tmux('set-option', '-g', '@drudwyn-redact-labels', 'on')
        pane = self.integration_ui()
        self.wait_pane(pane, 'MERGE CHANGES')
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'Snapshot: r refresh')
        self.tmux('send-keys', '-t', pane, 'V')
        self.wait_pane(pane, 'CHECK MERGED CHANGES')
        self.tmux('send-keys', '-t', pane, 'Tab')
        self.tmux('send-keys', '-t', pane, '-l', 'private-failure-check')
        self.tmux('send-keys', '-t', pane, 'Tab')
        self.tmux('send-keys', '-t', pane, '-l', 'printf "FAILURE_OUTPUT"; false')
        self.tmux('send-keys', '-t', pane, 'F5')
        screen = self.wait_pane(pane, '[Enter] Back to checks')
        self.assertIn('FAILURE_OUTPUT', screen)
        self.assertIn('[!] Failed', screen)
        self.assertIn('Exit: 1', screen)
        self.assertNotIn('[OK]', screen)
        self.assertNotIn('private-failure-check', screen)
        self.assertNotIn(str(self.repo), screen)
        self.assertIn('[redacted]', screen)
        self.assertIn('38;2;235;111;146', self.tmux('capture-pane', '-p', '-e', '-t', pane))
        self.tmux('send-keys', '-t', pane, 'Enter')
        self.wait_pane(pane, 'CHECK MERGED CHANGES')
        self.assertIn('Failed', self.verify().stdout)

    def test_observed_dirty_edits_revision_and_metadata_loss(self):
        self.integrate('--apply', self.token(self.integrate()))
        tracked = self.repo/'common.txt'
        tracked.write_text('dirty one')
        self.assertIn('Passed', self.verify('true').stdout)
        tracked.write_text('dirty two')  # same length and same porcelain state
        self.assertIn('Stale', self.verify().stdout)
        tracked.write_text('dirty one')
        self.assertIn('Stale', self.verify().stdout)
        self.assertIn('Passed', self.verify('true').stdout)
        self.git('add', '.')
        self.git('commit', '-qm', 'later revision')
        self.assertIn('Stale', self.verify().stdout)
        self.assertIn('Passed', self.verify('true').stdout)
        keys = [line.split()[0] for line in self.tmux('show-options', '-s').splitlines() if line.startswith('@drudwyn_verify_')]
        for key in keys: self.tmux('set-option', '-su', key)
        self.assertIn('Not verified', self.verify().stdout)
        self.assertIn('Already contained', self.integrate().stdout)

    def start_check(self, command):
        process = subprocess.Popen([str(BIN), 'workspace', 'verify', '--path', str(self.repo),
                                    '--check', 'long-check', '--command-stdin'],
                                   stdin=subprocess.PIPE, stdout=subprocess.DEVNULL,
                                   stderr=subprocess.DEVNULL, text=True, env=self.env)
        process.stdin.write(command)
        process.stdin.close()
        self.addCleanup(lambda: process.poll() is None and process.kill())
        deadline = time.monotonic() + 5
        while 'Running' not in self.verify().stdout:
            self.assertLess(time.monotonic(), deadline)
            time.sleep(.03)
        return process

    def test_running_interruption_orphan_lock_and_deliberate_rerun(self):
        process = self.start_check('sleep 2; false')
        self.assertIn('Running', self.verify().stdout)
        blocked = self.verify('true')
        self.assertNotEqual(blocked.returncode, 0)
        process.kill()
        process.wait(timeout=5)
        report = self.verify().stdout
        self.assertIn('Failed', report)
        self.assertIn('interrupted', report)
        blocked = self.verify('true')
        self.assertNotEqual(blocked.returncode, 0)
        self.assertIn('still running', blocked.stderr)
        time.sleep(2.1)
        self.assertIn('Passed', self.verify('true').stdout)
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.base)

    def test_change_during_check_lost_receipt_and_redaction(self):
        process = self.start_check('sleep 1; true')
        (self.repo/'untracked-input').write_text('new input')
        self.assertIn('Stale', self.verify().stdout)
        (self.repo/'untracked-input').unlink()
        process.wait(timeout=5)
        self.assertIn('Stale', self.verify().stdout)
        self.assertIn('Passed', self.verify('true', label='private-check-identity').stdout)
        self.tmux('set', '-g', '@drudwyn-redact-labels', 'on')
        status = self.verify().stdout
        self.assertIn('Passed', status)
        self.assertIn('[redacted]', status)
        for value in [str(self.repo), 'private-check-identity', self.base]:
            self.assertNotIn(value, status)
        process = self.start_check('sleep 1; true')
        for line in self.tmux('show-options', '-s').splitlines():
            if line.startswith('@drudwyn_verify_'): self.tmux('set-option', '-su', line.split()[0])
        process.wait(timeout=5)
        self.assertIn('Not verified', self.verify().stdout)

    def test_actual_elsewhere_checkout_git_environment_and_shell_startup(self):
        self.git('switch', '-qc', 'planning')
        target = Path(self.tmp.name)/'elsewhere $literal destination'
        self.git('worktree', 'add', str(target), 'main')
        marker = Path(self.tmp.name)/'startup-ran'
        startup = Path(self.tmp.name)/'bash-startup'
        startup.write_text('touch '+str(marker))
        env = {**self.env, 'GIT_DIR': str(self.source/'.git'), 'GIT_WORK_TREE': str(self.source),
               'GIT_INDEX_FILE': str(Path(self.tmp.name)/'wrong-index'), 'BASH_ENV': str(startup)}
        result = self.verify('test "$(git branch --show-current)" = main; git rev-parse HEAD', path=target, env=env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('Passed', result.stdout)
        self.assertIn(str(target), result.stdout)
        self.assertIn(self.base, result.stdout)
        self.assertFalse(marker.exists())
        self.assertIn('Not verified', self.verify(path=self.repo).stdout)
        self.assertIn('Not verified', self.verify(path=self.source).stdout)

    def test_cockpit_redaction_narrow_cancel_and_missing_batch(self):
        self.tmux('set', '-g', '@drudwyn-redact-labels', 'on')
        pane = self.integration_ui()
        self.wait_pane(pane, 'MERGE CHANGES')
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'WORKSPACE COCKPIT')
        self.wait_pane(pane, 'Snapshot: r refresh')
        self.tmux('send-keys', '-t', pane, 'V')
        self.wait_pane(pane, 'CHECK MERGED CHANGES')
        self.tmux('resize-window', '-t', pane, '-x', '48', '-y', '24')
        output = self.wait_pane(pane, 'Refresh result')
        self.assertIn('[redacted]', output)
        self.assertNotIn(str(self.repo), output)
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'WORKSPACE COCKPIT')
        self.assertIn('Not verified', self.verify().stdout)
        self.tmux('set', '-g', '@drudwyn-redact-labels', 'off')
        self.tmux('set-option', '-wu', '-t', self.worker, '@drudwyn_batch')
        self.tmux('kill-window', '-t', pane)
        pane = self.integration_ui()
        self.wait_pane(pane, 'CHOOSE MERGE BRANCH')
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'Snapshot: r refresh')
        self.tmux('send-keys', '-t', pane, 'V')
        output = self.wait_pane(pane, 'CHECK MERGED CHANGES')
        self.assertNotIn(str(self.source), output)
        self.assertNotIn(str(self.repo), output)
        self.tmux('send-keys', '-t', pane, 'F5')
        output = self.wait_pane(pane, '[Enter] Back to checks')
        self.assertIn('Select a short check identity', output)
        self.assertIn('Not verified', self.verify().stdout)

    def test_malformed_or_replaced_runner_metadata_cannot_claim_success(self):
        self.assertIn('Passed', self.verify('true').stdout)
        key = next(line.split()[0] for line in self.tmux('show-options', '-s').splitlines() if line.startswith('@drudwyn_verify_'))
        original = self.tmux('show-option', '-sqv', key).split('|')
        for index, value in [(3, 'not-a-revision'), (8, ':'), (8, '0:123'),
                             (8, '123:garbage'), (9, 'garbage'), (9, ''),
                             (5, '999999999999999999999.000000000'),
                             (6, '1.000000000'), (7, 'invalid-exit')]:
            with self.subTest(field=index, value=value):
                receipt = original.copy()
                receipt[index] = value
                self.tmux('set-option', '-s', key, '|'.join(receipt))
                self.assertIn('Not verified', self.verify().stdout)
        process = self.start_check('sleep 1; true')
        receipt = self.tmux('show-option', '-sqv', key).split('|')
        receipt[8] = '999999999:1'
        self.tmux('set-option', '-s', key, '|'.join(receipt))
        status = self.verify().stdout
        self.assertIn('Failed', status)
        self.assertIn('interrupted', status)
        process.wait(timeout=5)
        self.assertNotEqual(process.returncode, 0)
        self.assertNotIn('Passed', self.verify().stdout)
        self.assertIn('Passed', self.verify('true').stdout)

    def test_checkout_replacement_keeps_execution_in_original_directory(self):
        process = self.start_check('sleep 1; printf preserved > check-output.txt')
        original = self.repo.with_name('original-checkout')
        self.repo.rename(original)
        subprocess.run(['git', 'clone', '-q', str(original), str(self.repo)], check=True)
        self.assertIn('Stale', self.verify().stdout)
        process.wait(timeout=5)
        self.assertNotEqual(process.returncode, 0)
        self.assertEqual((original/'check-output.txt').read_text(), 'preserved')
        self.assertFalse((self.repo/'check-output.txt').exists())
        self.assertIn('Stale', self.verify().stdout)

    def test_control_character_checkout_and_alias_never_retarget_sibling(self):
        unusual = Path(str(self.repo) + '\n')
        subprocess.run(['git', 'clone', '-q', str(self.repo), str(unusual)], check=True)
        alias = self.repo.with_name('newline-checkout-alias')
        alias.symlink_to(unusual, target_is_directory=True)
        marker = self.repo/'must-not-run'
        for selected in [unusual, alias]:
            marker.unlink(missing_ok=True)
            with self.subTest(selected=str(selected)):
                result = self.verify('printf touched > must-not-run', path=selected)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(marker.exists(), 'command ran in the unrelated ordinary sibling')
                self.assertIn('control characters', result.stderr)
                self.assertFalse((unusual/'must-not-run').exists())
        marker.unlink(missing_ok=True)

if __name__ == '__main__':
    names = [n for n in VerificationTest.__dict__ if n.startswith('test_') and (len(sys.argv)==1 or sys.argv[1] in n)]
    result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(VerificationTest(n) for n in names))
    sys.exit(not result.wasSuccessful())
