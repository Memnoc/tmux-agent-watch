"""Explicit integration through public commands and actual tmux Cockpit actions."""
from pathlib import Path
import os
import re
import shlex
import subprocess
import sys
import time
import unittest
from independent_navigation_test import IndependentNavigation, BIN, ROOT

class WorkerIntegrationTest(IndependentNavigation):
    def setUp(self):
        super().setUp()
        self.git('config', 'user.name', 'Test')
        self.git('config', 'user.email', 'test@example.invalid')
        (self.repo/'common.txt').write_text('base content')
        self.git('add', '.')
        self.git('commit', '-qm', 'tracked baseline')
        self.base = self.git('rev-parse', 'HEAD')
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        session = self.selection(self.clients[0]).split(':')[0]
        batch = self.command('batch', 'setup', '--repo', str(self.repo), '--session', session,
                             '--yes', '--expect-source', self.base, '--expect-destination', self.base,
                             client=self.clients[0])
        self.batch = re.search(r'Batch: (\S+)', batch.stdout)[1]
        result = self.command('workspace', 'start', '--repo', str(self.repo), '--batch', self.batch,
                              '--name', 'integrate-worker', '--worktree-root', str(Path(self.tmp.name)/'trees $literal'),
                              'worker', str(Path(self.tmp.name)/'fake-worker/codex'), '300',
                              client=self.clients[0])
        self.source = Path(result.stdout.strip())
        self.worker = re.search(r'window (@[0-9]+)', result.stderr)[1]
        (self.source/'worker.txt').write_text('private implementation content\n')
        self.git('add', '.', path=self.source)
        self.git('commit', '-qm', 'worker private commit body', path=self.source)
        self.commit = self.git('rev-parse', 'HEAD', path=self.source)

    def git(self, *args, path=None, check=True):
        return subprocess.run(['git', '-C', str(path or self.repo), *args], text=True,
                              capture_output=True, check=check).stdout.strip()

    def integrate(self, *args, path=None, destination='main', check=True, client=None):
        return self.command('workspace', 'integrate', '--path', str(path or self.source),
                            '--destination', destination, *args, check=check, client=client or self.clients[0])

    def token(self, result):
        return re.search(r'Review token: ([0-9a-f]+)', result.stdout)[1]

    def test_preview_cancel_fast_forward_and_ancestry_noop(self):
        preview = self.integrate()
        self.assertIn('Source: refs/heads/worker', preview.stdout)
        self.assertIn('Destination: refs/heads/main', preview.stdout)
        self.assertIn(str(self.repo), preview.stdout)
        self.assertIn(self.commit, preview.stdout)
        self.assertIn(self.base, preview.stdout)
        self.assertIn('worker.txt', preview.stdout)
        self.assertIn('Checks: unknown', preview.stdout)
        self.assertIn('Fast-forward', preview.stdout)
        self.assertNotIn('private implementation content', preview.stdout)
        self.assertNotIn('private commit body', preview.stdout)
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.base)  # preview/cancel
        result = self.integrate('--apply', self.token(preview))
        self.assertIn('Integrated', result.stdout)
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.commit)
        self.assertTrue(self.source.is_dir())
        again = self.integrate()
        self.assertIn('Already contained', again.stdout)
        self.assertIn('No-op', self.integrate('--apply', self.token(again)).stdout)
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.commit)

    def test_divergent_merge_into_actual_elsewhere_checkout_and_batch(self):
        self.git('switch', '-qc', 'planning')
        target = Path(self.tmp.name)/'destination $literal ; space'
        self.git('worktree', 'add', str(target), 'main')
        (target/'target.txt').write_text('target work')
        self.git('add', '.', path=target)
        self.git('commit', '-qm', 'target change', path=target)
        previous = self.git('rev-parse', 'HEAD', path=target)
        preview = self.integrate()
        self.assertIn(str(target), preview.stdout)
        self.assertIn('Normal merge', preview.stdout)
        result = self.integrate('--apply', self.token(preview))
        self.assertIn('Integrated', result.stdout)
        self.assertEqual(self.git('branch', '--show-current'), 'planning')
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.base)
        parents = self.git('rev-list', '--parents', '-n', '1', 'HEAD', path=target).split()
        self.assertEqual(parents[1:], [previous, self.commit])
        # The old batch still points to the primary checkout; no silent retarget.
        failed = self.command('workspace', 'integrate', '--path', str(self.source), '--batch', self.batch, check=False)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn('reselect', failed.stderr)
        # A new explicit integration branch is independent of both main/planning.
        integration = Path(self.tmp.name)/'assembly'
        self.git('worktree', 'add', '-b', 'assemble', str(integration), self.base)
        result = self.command('batch', 'setup', '--repo', str(self.repo), '--session', self.batch.split('/')[0],
                              '--integration', 'assemble', '--reuse-existing', '--yes',
                              '--expect-source', self.git('rev-parse', 'main'), '--expect-destination', self.base,
                              client=self.clients[0])
        batch = re.search(r'Batch: (\S+)', result.stdout)[1]
        args = ['workspace', 'integrate', '--path', str(self.source), '--batch', batch]
        preview = self.command(*args)
        self.assertIn('refs/heads/assemble', preview.stdout)
        self.assertIn('Integrated', self.command(*args, '--apply', self.token(preview)).stdout)
        self.assertEqual(self.git('rev-parse', 'HEAD', path=integration), self.commit)

    def test_dirty_untracked_operations_detached_and_ambiguous_rejected(self):
        for path in [self.source, self.repo]:
            tracked = path/'common.txt'
            tracked.write_text('preserved tracked change')
            for staged in [False, True]:
                if staged: self.git('add', 'common.txt', path=path)
                result = self.integrate(check=False)
                self.assertIn('dirty', result.stderr)
                self.assertEqual(tracked.read_text(), 'preserved tracked change')
            self.git('restore', '--staged', 'common.txt', path=path)
            tracked.write_text('base content')
            marker = path/'untracked private.txt'
            marker.write_text('preserve this')
            result = self.integrate(check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('dirty', result.stderr)
            self.assertEqual(marker.read_text(), 'preserve this')
            marker.unlink()
            gitdir = Path(self.git('rev-parse', '--absolute-git-dir', path=path))
            for operation in ['MERGE_HEAD', 'CHERRY_PICK_HEAD', 'REVERT_HEAD', 'rebase-merge', 'rebase-apply', 'sequencer', 'index.lock']:
                marker = gitdir/operation
                marker.touch()
                result = self.integrate(check=False)
                self.assertNotEqual(result.returncode, 0, operation)
                self.assertIn('operation or lock', result.stderr)
                marker.unlink()
        self.git('checkout', '--detach')
        self.assertNotEqual(self.integrate(check=False).returncode, 0)
        self.git('switch', 'main')
        duplicate = Path(self.tmp.name)/'duplicate'
        self.git('worktree', 'add', '--force', str(duplicate), 'main')
        self.assertIn('exactly one', self.integrate(check=False).stderr)
        self.assertEqual(self.git('rev-parse', 'main'), self.base)

    def test_changed_source_target_branch_and_checkout_require_new_preview(self):
        reviewed = self.token(self.integrate())
        self.git('switch', '-qc', 'renamed', path=self.source)
        self.assertIn('changed after preview', self.integrate('--apply', reviewed, check=False).stderr)
        self.git('switch', 'worker', path=self.source)
        self.git('commit', '--allow-empty', '-qm', 'new source', path=self.source)
        self.assertIn('changed after preview', self.integrate('--apply', reviewed, check=False).stderr)
        reviewed = self.token(self.integrate())
        self.git('commit', '--allow-empty', '-qm', 'new target')
        self.assertIn('changed after preview', self.integrate('--apply', reviewed, check=False).stderr)
        reviewed = self.token(self.integrate())
        # Recreate the same pathname/refs with another directory inode.
        old = self.repo.with_name('old-repo')
        self.repo.rename(old)
        import shutil
        shutil.copytree(old, self.repo)
        self.assertIn('changed after preview', self.integrate('--apply', reviewed, check=False).stderr)
        self.assertNotEqual(self.git('rev-parse', 'HEAD'), self.commit)

    def test_conflict_and_hook_failure_retain_work_and_never_claim_success(self):
        (self.repo/'worker.txt').write_text('conflicting target content')
        self.git('add', '.')
        self.git('commit', '-qm', 'target conflict')
        before = self.git('rev-parse', 'HEAD')
        preview = self.integrate()
        result = self.integrate('--apply', self.token(preview), check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('merge retained', result.stderr)
        self.assertNotIn('Integrated:', result.stdout)
        self.assertEqual(self.git('rev-parse', 'HEAD'), before)
        self.assertEqual(self.git('rev-parse', 'MERGE_HEAD'), self.commit)
        self.assertTrue(self.git('ls-files', '--unmerged'))
        self.assertTrue(self.source.is_dir())
        self.assertIn('operation or lock', self.integrate(check=False).stderr)
        self.git('merge', '--abort')  # deliberate fixture cleanup, never supervisor fallback
        self.git('rm', 'worker.txt')
        self.git('commit', '-qm', 'remove conflicting file')
        hook = self.repo/'.git/hooks/pre-merge-commit'
        hook.write_text('#!/bin/sh\necho PRIVATE_HOOK_CONTENT\nexit 1\n')
        hook.chmod(0o755)
        preview = self.integrate()
        result = self.integrate('--apply', self.token(preview), check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('PRIVATE_HOOK_CONTENT', result.stdout+result.stderr)
        self.assertTrue((self.repo/'.git/MERGE_HEAD').exists())

    def test_missing_metadata_is_explicit_and_redaction_preserves_controls(self):
        result = self.command('workspace', 'integrate', '--path', str(self.source), check=False)
        self.assertNotEqual(result.returncode, 0)
        session, key = self.batch.split('/')
        self.tmux('set', '-u', '-t', session, '@drudwyn_batch_'+key)
        result = self.command('workspace', 'integrate', '--path', str(self.source), '--batch', self.batch, check=False)
        self.assertIn('reselect', result.stderr)
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.base)
        self.tmux('set', '-g', '@drudwyn-redact-labels', 'on')
        preview = self.integrate()
        self.assertIn('Review token:', preview.stdout)
        self.assertNotIn(str(self.source), preview.stdout)
        self.assertNotIn('worker.txt', preview.stdout)
        self.assertNotIn(self.commit, preview.stdout)
        self.assertIn('Integrated', self.integrate('--apply', self.token(preview)).stdout)

    def test_ignored_collision_is_not_overwritten(self):
        self.git('config', 'core.excludesFile', str(Path(self.tmp.name)/'ignore'))
        (Path(self.tmp.name)/'ignore').write_text('worker.txt\n')
        for divergent in (False, True):
            if divergent:
                self.git('commit', '--allow-empty', '-qm', 'divergent target')
            before = self.git('rev-parse', 'HEAD')
            for directory in (False, True):
                with self.subTest(divergent=divergent, directory=directory):
                    ignored = self.repo/'worker.txt'
                    if directory:
                        ignored.mkdir()
                        ignored = ignored/'private-cache'
                    ignored.write_text('irreplaceable ignored work')
                    self.assertEqual(self.git('status', '--porcelain'), '')
                    preview = self.integrate()
                    result = self.integrate('--apply', self.token(preview), check=False)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertEqual(ignored.read_text(), 'irreplaceable ignored work')
                    self.assertEqual(self.git('rev-parse', 'HEAD'), before)
                    ignored.unlink()
                    if directory:
                        ignored.parent.rmdir()

    def test_divergent_merge_allows_unrelated_ignored_paths_and_unchanged_source_paths(self):
        # Target deliberately deletes common.txt; its local ignored replacement
        # must survive because the source has not changed that path.
        self.git('rm', 'common.txt')
        self.git('commit', '-qm', 'target removes unchanged source file')
        (self.repo/'.git/info/exclude').write_text('common.txt\nnode_modules/\n')
        (self.repo/'common.txt').write_text('local ignored replacement')
        dependency = self.repo/'node_modules/dependency'
        dependency.mkdir(parents=True)
        (dependency/'cache').write_text('unrelated ignored dependency')
        self.assertEqual(self.git('status', '--porcelain'), '')
        result = self.integrate('--apply', self.token(self.integrate()))
        self.assertIn('Integrated', result.stdout)
        self.assertEqual((self.repo/'common.txt').read_text(), 'local ignored replacement')
        self.assertEqual((dependency/'cache').read_text(), 'unrelated ignored dependency')

    def test_ignored_file_cannot_become_incoming_directory(self):
        (self.source/'incoming').mkdir()
        (self.source/'incoming/file').write_text('source data')
        self.git('add', '.', path=self.source)
        self.git('commit', '-qm', 'incoming directory', path=self.source)
        (self.repo/'.git/info/exclude').write_text('incoming\n')
        for divergent in (False, True):
            if divergent:
                self.git('commit', '--allow-empty', '-qm', 'divergent target')
            (self.repo/'incoming').write_text('protected ignored file')
            before = self.git('rev-parse', 'HEAD')
            result = self.integrate('--apply', self.token(self.integrate()), check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual((self.repo/'incoming').read_text(), 'protected ignored file')
            self.assertEqual(self.git('rev-parse', 'HEAD'), before)
            (self.repo/'incoming').unlink()

    def test_concurrent_alias_and_orphan_git_child_keep_destination_guard(self):
        import shutil
        real_git = shutil.which('git')
        wrappers = Path(self.tmp.name)/'wrapper'
        wrappers.mkdir()
        arrived = Path(self.tmp.name)/'merge-arrived'
        release = Path(self.tmp.name)/'merge-release'
        wrapper = wrappers/'git'
        wrapper.write_text('#!/bin/sh\ncase " $* " in *" merge "*)\n'
                           'touch '+shlex.quote(str(arrived))+'\n'
                           'while [ ! -e '+shlex.quote(str(release))+' ]; do sleep .01; done;; esac\n'
                           'exec '+shlex.quote(real_git)+' "$@"\n')
        wrapper.chmod(0o755)
        reviewed = self.token(self.integrate())
        env = {**self.env, 'PATH': str(wrappers)+':'+os.environ['PATH']}
        parent = subprocess.Popen([str(BIN), 'workspace', 'integrate', '--path', str(self.source),
                                   '--destination', 'main', '--apply', reviewed], env=env,
                                  stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        self.addCleanup(parent.stderr.close)
        self.addCleanup(lambda: release.touch())
        self.addCleanup(lambda: parent.poll() is not None or parent.kill())
        deadline = time.monotonic()+5
        while not arrived.exists():
            self.assertIsNone(parent.poll(), parent.stderr.read() if parent.poll() is not None else "")
            self.assertLess(time.monotonic(), deadline)
            time.sleep(.02)
        alias = Path(self.tmp.name)/'source alias'
        alias.symlink_to(self.source, target_is_directory=True)
        second = self.integrate('--apply', reviewed, path=alias, client=self.clients[1], check=False)
        self.assertIn('already in progress', second.stderr)
        parent.kill(); parent.wait(timeout=3)
        second = self.integrate('--apply', reviewed, path=alias, client=self.clients[1], check=False)
        self.assertIn('already in progress', second.stderr)
        self.command('scan')  # no long-lived global lifecycle lock
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.base)
        release.touch()
        deadline = time.monotonic()+5
        while self.git('rev-parse', 'HEAD') != self.commit:
            self.assertLess(time.monotonic(), deadline)
            time.sleep(.02)
        final = self.integrate()
        self.assertIn('Already contained', final.stdout)
        self.assertIn('No-op', self.integrate('--apply', self.token(final)).stdout)

    def integration_ui(self, git_env=()):
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project',
                         shlex.join(['env', 'DRUDWYN_CLIENT='+self.clients[0], *git_env, str(BIN),
                                     'cockpit', '--search', 'integrate-worker', '--project', self.batch.split('/')[0]]))
        self.tmux('resize-window', '-t', pane, '-x', '120', '-y', '40')
        self.wait_pane(pane, 'WORKSPACE COCKPIT')
        self.tmux('send-keys', '-t', pane, 'i')
        return pane

    def test_cockpit_preview_cancel_and_apply_preserves_both_client_selections(self):
        before = [self.selection(c) for c in self.clients]
        pane = self.integration_ui()
        preview = self.wait_pane(pane, 'INTEGRATE PREVIEW')
        self.assertIn('refs/heads/main', preview)
        self.assertIn('Checks: unknown', preview)
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'WORKSPACE COCKPIT')
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.base)
        deadline = time.monotonic()+4
        while 'REFRESHING' in self.tmux('capture-pane', '-p', '-t', pane):
            self.assertLess(time.monotonic(), deadline)
            time.sleep(.02)
        self.tmux('send-keys', '-t', pane, 'i')
        self.wait_pane(pane, 'INTEGRATE PREVIEW')
        self.tmux('send-keys', '-t', pane, 'y')
        self.wait_pane(pane, 'Integrated:')
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.commit)
        self.assertEqual([self.selection(c) for c in self.clients], before)
        self.assertTrue(self.source.is_dir())

    def test_python_merge_hook_has_usable_stdin_and_checks_stay_unknown(self):
        self.git('commit', '--allow-empty', '-qm', 'divergent target')
        marker = Path(self.tmp.name)/'python-hook'
        hook = self.repo/'.git/hooks/pre-merge-commit'
        hook.write_text('#!/usr/bin/python3\nimport sys\nfrom pathlib import Path\n'
                        'assert sys.stdin.read() == ""\n'
                        f'Path({str(marker)!r}).touch()\n')
        hook.chmod(0o755)
        preview = self.integrate()
        result = self.integrate('--apply', self.token(preview))
        self.assertTrue(marker.exists())
        self.assertIn('Integrated:', result.stdout)
        self.assertIn('checks unknown', result.stdout)

    def test_cockpit_revalidates_stale_commit_and_disappeared_stable_pane(self):
        pane = self.integration_ui()
        self.wait_pane(pane, 'INTEGRATE PREVIEW')
        self.git('commit', '--allow-empty', '-qm', 'changed after UI review', path=self.source)
        self.tmux('send-keys', '-t', pane, 'y')
        self.wait_pane(pane, 'changed after preview')
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.base)
        self.tmux('send-keys', '-t', pane, 'r')
        self.wait_pane(pane, self.git('rev-parse', 'HEAD', path=self.source))
        index = self.tmux('display-message', '-p', '-t', self.worker, '#{window_index}')
        self.tmux('kill-window', '-t', self.worker)
        replacement = self.tmux('new-window', '-d', '-P', '-F', '#{window_id}', '-t', 'project:'+index,
                                '-n', 'integrate-worker', '-c', str(self.source),
                                str(Path(self.tmp.name)/'fake-worker/codex'), '300')
        self.assertNotEqual(replacement, self.worker)
        self.tmux('send-keys', '-t', pane, 'y')
        self.wait_pane(pane, 'target changed or disappeared')
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.base)

    def test_cockpit_missing_batch_requires_explicit_destination_and_narrow_scroll(self):
        self.tmux('set', '-wu', '-t', self.worker, '@drudwyn_batch')
        for i in range(32):
            (self.source/f'change-{i:02}.txt').write_text('private file body')
        self.git('add', '.', path=self.source)
        self.git('commit', '-qm', 'many files', path=self.source)
        pane = self.integration_ui()
        self.wait_pane(pane, 'CHOOSE DESTINATION')
        self.tmux('send-keys', '-t', pane, 'Enter')
        self.wait_pane(pane, 'Choose exactly one')
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.base)
        self.tmux('send-keys', '-t', pane, '-l', 'main')
        self.tmux('send-keys', '-t', pane, 'Enter')
        self.wait_pane(pane, 'INTEGRATE PREVIEW')
        self.tmux('resize-window', '-t', pane, '-x', '48', '-y', '24')
        for _ in range(16): self.tmux('send-keys', '-t', pane, 'PageDown')
        output = self.wait_pane(pane, '"worker.txt"')
        self.assertIn('Esc cancel', output)
        Path('/tmp/drudwyn-ticket11-narrow-bottom.txt').write_text(output)
        self.tmux('send-keys', '-t', pane, 'Home')
        output = self.wait_pane(pane, 'refs/heads/worker')
        Path('/tmp/drudwyn-ticket11-narrow-preview.txt').write_text(output)
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'WORKSPACE COCKPIT')
        self.assertEqual(self.git('rev-parse', 'HEAD'), self.base)
        self.tmux('kill-window', '-t', pane)
        self.tmux('set', '-g', '@drudwyn-redact-labels', 'on')
        pane = self.integration_ui()
        self.wait_pane(pane, 'CHOOSE DESTINATION')
        self.tmux('send-keys', '-t', pane, '-l', 'main')
        self.tmux('send-keys', '-t', pane, 'Enter')
        output = self.wait_pane(pane, 'INTEGRATE PREVIEW')
        self.assertIn('[redacted]', output)
        self.assertNotIn('change-00.txt', output)
        self.assertNotIn(str(self.source), output)
        self.assertNotIn(self.commit, output)
        Path('/tmp/drudwyn-ticket11-redacted-preview.txt').write_text(output)

    def test_details_follow_current_ancestry_not_review_or_old_success(self):
        self.worker_hook('stop')
        pane = self.integration_ui()
        self.wait_pane(pane, 'INTEGRATE PREVIEW')
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'WORKSPACE COCKPIT')
        self.tmux('send-keys', '-t', pane, 'd')
        self.wait_pane(pane, 'Not contained in selected destination')
        self.assertIn('REVIEW', self.command('status').stdout)
        self.git('merge', '--ff-only', 'worker')  # external integration, not cached action
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'WORKSPACE COCKPIT')
        self.tmux('send-keys', '-t', pane, 'r', 'd')
        self.wait_pane(pane, 'Contained in selected destination')
        self.assertIn('REVIEW', self.command('status').stdout)
        self.git('reset', '--hard', self.base)  # fixture-only reverse target movement
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'WORKSPACE COCKPIT')
        self.tmux('send-keys', '-t', pane, 'r', 'd')
        self.wait_pane(pane, 'Not contained in selected destination')
        self.tmux('set', '-wu', '-t', self.worker, '@drudwyn_batch')
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'WORKSPACE COCKPIT')
        self.tmux('send-keys', '-t', pane, 'r', 'd')
        self.wait_pane(pane, 'Integration: unknown')
        self.assertEqual(self.git('rev-parse', 'main'), self.base)

    def test_inherited_git_repository_environment_cannot_retarget_explicit_path(self):
        other = Path(self.tmp.name)/'unrelated'
        self.git('init', '-q', '-b', 'main', str(other))
        self.git('-c', 'user.name=Test', '-c', 'user.email=test@example.invalid',
                 'commit', '--allow-empty', '-qm', 'unrelated', path=other)
        env = {**self.env, 'GIT_DIR': str(other/'.git'), 'GIT_WORK_TREE': str(other)}
        result = subprocess.run([str(BIN), 'workspace', 'integrate', '--path', str(self.source),
                                 '--destination', 'main'], env=env, text=True, capture_output=True, check=True)
        self.assertIn('Source: refs/heads/worker', result.stdout)
        self.assertIn('Target checkout: '+str(self.repo), result.stdout)
        self.assertNotIn(str(other), result.stdout)

    def test_detail_source_commit_ignores_inherited_target_git_environment(self):
        pane = self.integration_ui(['GIT_DIR='+str(self.repo/'.git'), 'GIT_WORK_TREE='+str(self.repo)])
        self.wait_pane(pane, 'INTEGRATE PREVIEW')
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'WORKSPACE COCKPIT')
        self.tmux('send-keys', '-t', pane, 'd')
        output = self.wait_pane(pane, 'Not contained in selected destination')
        self.assertIn(self.commit, output)

    def test_advertised_merge_modes_override_branch_options_without_staged_alternates(self):
        for divergent in [False, True]:
            for i, options in enumerate(['--squash', '--no-commit', '--squash --no-commit',
                                         '--squash --no-commit '+('--ff-only' if divergent else '--no-ff')]):
                with self.subTest(divergent=divergent, options=options):
                    branch = f'target-{int(divergent)}-{i}'
                    target = Path(self.tmp.name)/branch
                    self.git('worktree', 'add', '-b', branch, str(target), self.base)
                    if divergent: self.git('commit', '--allow-empty', '-qm', 'target change', path=target)
                    before = self.git('rev-parse', 'HEAD', path=target)
                    self.git('config', f'branch.{branch}.mergeOptions', options)
                    marker = Path(self.tmp.name)/(branch+'-hook')
                    hook = self.repo/'.git/hooks/post-merge'
                    hook.write_text('#!/usr/bin/python3\nimport sys\nfrom pathlib import Path\n'
                                    'assert sys.stdin.read() == ""\n'
                                    f'Path({str(marker)!r}).write_text(sys.argv[1])\n')
                    hook.chmod(0o755)
                    preview = self.integrate(destination=branch)
                    self.assertIn('Normal merge' if divergent else 'Fast-forward', preview.stdout)
                    result = self.integrate('--apply', self.token(preview), destination=branch)
                    self.assertIn('Integrated:', result.stdout)
                    after = self.git('rev-parse', 'HEAD', path=target)
                    self.assertNotEqual(after, before)
                    if divergent:
                        parents = self.git('rev-list', '--parents', '-n', '1', 'HEAD', path=target).split()
                        self.assertEqual(parents[1:], [before, self.commit])
                    else:
                        self.assertEqual(after, self.commit)
                    self.assertEqual(self.git('status', '--porcelain', path=target), '')
                    self.assertEqual(self.git('diff', '--cached', '--name-only', path=target), '')
                    self.assertEqual(self.git('rev-parse', '--verify', 'MERGE_HEAD', path=target, check=False), '')
                    self.assertEqual(marker.read_text(), '0')  # Git says normal merge, not squash.
                    self.assertTrue(self.source.is_dir())

if __name__ == '__main__':
    names = [n for n in WorkerIntegrationTest.__dict__ if n.startswith('test_') and (len(sys.argv)==1 or sys.argv[1] in n)]
    result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(WorkerIntegrationTest(n) for n in names))
    sys.exit(not result.wasSuccessful())
