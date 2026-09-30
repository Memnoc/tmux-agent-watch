"""Global inventory and terminal actions with disposable real Git/tmux clients."""
import os
from pathlib import Path
import shlex
import subprocess
import sys
import time
import unittest
from independent_navigation_test import IndependentNavigation, BIN

class GlobalCockpitTest(IndependentNavigation):
    def test_global_inventory_deduplicates_workers_and_includes_project_shells(self):
        fake = Path(self.tmp.name) / 'fake-worker/codex'
        self.assertEqual(fake.resolve(), Path('/usr/bin/sleep').resolve())
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        projects = ['project']
        workers = [self.worker]
        for p in range(4):
            session = 'project' if p == 0 else f'project-{p}'
            repo = self.repo if p == 0 else Path(self.tmp.name) / f'repo-{p}'
            if p:
                subprocess.run(['git', 'clone', '-q', str(self.repo), str(repo)], check=True)
                self.tmux('new-session', '-d', '-s', session, '-c', str(repo), 'bash', '--noprofile', '--norc')
                coord = self.tmux('display-message', '-p', '-t', session, '#{window_id}')
                self.command('coordinator', 'set', '--window', coord, '--session', self.tmux('display-message', '-p', '-t', session, '#{session_id}'), client=self.clients[0])
                projects.append(session)
            for i in range(8 if p == 0 else 9):
                checkout = Path(self.tmp.name) / f'checkout-{p}-{i}'
                subprocess.run(['git', '-C', str(repo), 'worktree', 'add', '-qb', f'work/{p}-{i}', str(checkout)], check=True)
                workers.append(self.tmux('new-window', '-d', '-P', '-F', '#{window_id}', '-t', session, '-n', 'identical long worker name', '-c', str(checkout), str(fake), '300'))
        self.command('navigate', '--window', self.worker, client=self.clients[0])
        before = time.monotonic()
        result = self.command('cockpit', '--list', client=self.clients[0], check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('GLOBAL 36 workers · 36 live · 4 projects', result.stdout)
        rows = [line.split('\t')[0] for line in result.stdout.splitlines() if line.startswith('@')]
        self.assertEqual(set(rows), set(workers))
        self.assertEqual(len(rows), 36)
        print(f'global fixture: 36 workers/4 projects, refresh wall {time.monotonic()-before:.3f}s\n{result.stdout.splitlines()[0]}\n{result.stdout.splitlines()[1]}', flush=True)
        outside = self.tmux('new-window', '-d', '-P', '-F', '#{window_id}', '-t', 'project', '-n', 'outside-git-shell', '-c', '/tmp', 'bash', '--noprofile', '--norc')
        shells = self.command('cockpit', '--list', '--project', 'project', '--windows', client=self.clients[0]).stdout
        self.assertIn(self.home + '\t', shells)
        self.assertIn('Coordinator shell', shells)
        self.assertIn('MATCHING 11 windows', shells)
        self.assertIn(outside + '\t', shells)
        self.assertIn('UNAVAILABLE ' + outside, shells)
        # Exercise the actual 36-worker terminal inventory, not only CLI counts.
        opened = time.monotonic()
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project', '-c', str(self.repo), 'env', 'DRUDWYN_CLIENT=' + self.clients[0], str(BIN), 'cockpit')
        screen = self.wait_pane(pane, 'MATCHING 36')
        print(f'36-worker UI ready {time.monotonic()-opened:.3f}s', flush=True)
        Path('/tmp/drudwyn-ticket09-global36-120.txt').write_text(screen)
        moved = time.monotonic()
        self.tmux('send-keys', '-t', pane, 'End')
        screen = self.wait_pane(pane, 'IDENTITY ' + rows[-1])
        print(f'36-worker End inspection {time.monotonic()-moved:.3f}s', flush=True)
        self.assertLess(time.monotonic()-moved, .8)
        self.tmux('kill-pane', '-t', pane)
        before = self.selection(self.clients[1])
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project', '-c', str(self.repo), 'env', 'DRUDWYN_CLIENT=' + self.clients[0], str(BIN), 'cockpit', '--project', 'project', '--windows', '--search', 'outside-git-shell')
        self.wait_pane(pane, 'MATCHING 1 windows')
        self.tmux('send-keys', '-t', pane, 'Enter')
        for _ in range(100):
            if self.selection(self.clients[0]).endswith(':' + outside): break
            time.sleep(.02)
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + outside))
        self.assertEqual(before, self.selection(self.clients[1]))
        ordinary = self.tmux('new-session', '-d', '-P', '-F', '#{window_id}', '-s', 'unassociated', '-c', str(self.repo), str(fake), '300')
        result = self.command('cockpit', '--list', '--search', 'unassociated', client=self.clients[0]).stdout
        self.assertIn('GLOBAL 37 workers · 37 live · 4 projects', result)
        self.assertIn(ordinary + '\t', result)
        self.assertIn('MATCHING 1 workers', result)

    def test_details_filters_and_inspection_preserve_identity_and_attention(self):
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        (self.repo / 'changed secret.txt').write_text('content must never be inspected')
        result = self.command('cockpit', '--list', '--search', 'main', client=self.clients[0]).stdout
        self.assertIn(self.worker + '\t', result)
        before = [self.selection(c) for c in self.clients]
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project', '-c', str(self.repo),
                         'env', 'DRUDWYN_CLIENT=' + self.clients[0], str(BIN), 'cockpit', '--state', 'review')
        screen = self.wait_pane(pane, 'MATCHING 1')
        self.assertIn('GLOBAL 1 workers', screen)
        self.assertIn('state review', screen)
        self.tmux('send-keys', '-t', pane, 'd')
        screen = self.wait_pane(pane, 'DETAILS')
        self.assertIn('Integration: unknown', screen)
        self.assertIn('Checks: unknown', screen)
        self.assertIn('changed secret.txt', screen)
        self.assertNotIn('content must never', screen)
        self.assertEqual([self.selection(c) for c in self.clients], before)
        self.assertEqual(self.tmux('show-option', '-wqv', '-t', self.worker, '@drudwyn_state'), 'done')
        self.tmux('send-keys', '-t', pane, 'd')
        self.wait_pane(pane, 'MATCHING 1')
        self.tmux('send-keys', '-t', pane, 'r')
        self.wait_pane(pane, 'REFRESHING')
        self.wait_pane(pane, 'Snapshot: r refresh')
        self.tmux('send-keys', '-t', pane, 'Enter')
        for _ in range(100):
            if self.selection(self.clients[0]).endswith(':' + self.worker): break
            time.sleep(.025)
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + self.worker))
        self.assertEqual(self.selection(self.clients[1]), before[1])

    def test_refresh_is_responsive_and_failure_keeps_snapshot_actions_stale(self):
        import shutil
        gate = Path(self.tmp.name) / 'refresh-gate'; gate.mkdir()
        self.addCleanup(lambda: (gate / 'release').touch())
        real = shutil.which('tmux')
        wrapper = gate / 'tmux'
        wrapper.write_text('#!/bin/sh\nif [ "$1" = list-panes ] && [ -e ' + shlex.quote(str(gate / 'pause')) + ' ]; then\n touch ' + shlex.quote(str(gate / 'ready')) + '\n while [ ! -e ' + shlex.quote(str(gate / 'release')) + ' ]; do sleep .02; done\n exit 1\nfi\nexec ' + shlex.quote(real) + ' "$@"\n')
        wrapper.chmod(0o755)
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project', '-c', str(self.repo),
                         'env', 'DRUDWYN_CLIENT=' + self.clients[0], 'PATH=' + str(gate) + ':' + self.env['PATH'], str(BIN), 'cockpit')
        self.wait_pane(pane, 'MATCHING 1')
        (gate / 'pause').touch()
        before = [self.selection(c) for c in self.clients]
        self.tmux('send-keys', '-t', pane, 'r')
        self.wait_pane(pane, 'REFRESHING')
        start = time.monotonic()
        self.tmux('send-keys', '-t', pane, 'd')
        self.wait_pane(pane, 'DETAILS')
        elapsed = time.monotonic()-start
        self.assertLess(elapsed, .8)
        print(f'inspect during gated refresh: {elapsed:.3f}s', flush=True)
        self.tmux('send-keys', '-t', pane, 'd')
        (gate / 'release').touch()
        stale_screen = self.wait_pane(pane, 'STALE')
        Path('/tmp/drudwyn-ticket09-stale.txt').write_text(stale_screen)
        self.tmux('send-keys', '-t', pane, 'Enter')
        self.wait_pane(pane, 'refresh successfully before action')
        self.assertEqual([self.selection(c) for c in self.clients], before)
        (gate / 'pause').unlink()
        self.tmux('send-keys', '-t', pane, 'r')
        self.wait_pane(pane, 'Snapshot: r refresh')

    def test_identity_survives_reordering_filters_and_vanished_target(self):
        fake = Path(self.tmp.name) / 'fake-worker/codex'
        second = self.tmux('new-window', '-d', '-P', '-F', '#{window_id}', '-t', 'project', '-n', 'same name', '-c', str(self.repo), str(fake), '300')
        self.tmux('rename-window', '-t', self.worker, 'same name')
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        self.command('navigate', '--window', second, client=self.clients[0])
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project', '-c', str(self.repo), 'env', 'DRUDWYN_CLIENT=' + self.clients[0], str(BIN), 'cockpit')
        screen = self.wait_pane(pane, 'MATCHING 2')
        self.assertIn('* ' + second, screen)
        self.assertIn('IDENTITY ' + self.worker, screen)
        Path('/tmp/drudwyn-ticket09-current-vs-selected.txt').write_text(screen)
        target = self.tmux('display-message', '-p', '-t', second, '#{pane_id}')
        subprocess.run([str(BIN), 'hook', 'codex', 'permissionRequest'], env={**self.env, 'TMUX_PANE': target}, check=True)
        self.tmux('send-keys', '-t', pane, 'r')
        self.wait_pane(pane, 'NEEDS INPUT')
        screen = self.wait_pane(pane, 'Snapshot: r refresh')
        self.assertIn('IDENTITY ' + self.worker, screen)
        self.tmux('send-keys', '-t', pane, 'g')
        screen = self.wait_pane(pane, 'REVIEW [REVIEW]')
        self.assertIn('IDENTITY ' + self.worker, screen)
        self.tmux('send-keys', '-t', pane, '-l', '/codex')
        self.wait_pane(pane, 'MATCHING 2')
        self.tmux('send-keys', '-t', pane, 'Enter')
        self.tmux('send-keys', '-t', pane, 's')
        self.wait_pane(pane, 'state attention')
        self.tmux('send-keys', '-t', pane, 'p')
        self.wait_pane(pane, 'project $0')
        # Pending Open resolves the selected stable ID, never the same-name row.
        before = [self.selection(c) for c in self.clients]
        self.tmux('kill-window', '-t', self.worker)
        self.tmux('send-keys', '-t', pane, 'Enter')
        self.wait_pane(pane, 'Target window has vanished')
        self.assertEqual([self.selection(c) for c in self.clients], before)
        self.tmux('send-keys', '-t', pane, 'r')
        self.wait_pane(pane, 'move or inspect a row')
        self.tmux('send-keys', '-t', pane, 'Enter')
        self.assertEqual([self.selection(c) for c in self.clients], before)
        self.tmux('send-keys', '-t', pane, 'x')
        self.tmux('send-keys', '-t', pane, '-l', '/absent-worker-name')
        empty = self.wait_pane(pane, 'No workspaces match')
        Path('/tmp/drudwyn-ticket09-empty.txt').write_text(empty)
        self.assertIn('GLOBAL 1 workers', empty)

    def test_state_filters_keep_global_totals_and_exits_truthful(self):
        for event, state in [('userPromptSubmit','working'),('permissionRequest','input'),('stop','review')]:
            self.worker_hook(event)
            output = self.command('cockpit', '--list', '--state', state, client=self.clients[0]).stdout
            self.assertIn('GLOBAL 1 workers · 1 live', output)
            self.assertIn('MATCHING 1 workers', output)
            empty = self.command('cockpit', '--list', '--state', 'failed', client=self.clients[0]).stdout
            self.assertIn('GLOBAL 1 workers · 1 live', empty)
            self.assertIn('MATCHING 0 workers', empty)
            self.assertIn('No workspaces match', empty)
        self.tmux('set-option', '-w', '-t', self.worker, 'remain-on-exit', 'on')
        pid = self.tmux('display-message', '-p', '-t', self.worker, '#{pane_pid}')
        os.kill(int(pid), 15)
        for _ in range(100):
            if self.tmux('display-message', '-p', '-t', self.worker, '#{pane_dead}') == '1': break
            time.sleep(.02)
        output = self.command('cockpit', '--list', '--state', 'exited', client=self.clients[0]).stdout
        self.assertIn('GLOBAL 1 workers · 0 live', output)
        self.assertIn('1 exited · 1 attention', output)
        self.assertIn('MATCHING 1 workers', output)
        self.assertIn('not task completion', output)
        bad = self.command('cockpit', '--list', '--project', 'missing-project', client=self.clients[0], check=False)
        self.assertNotEqual(bad.returncode, 0)
        self.assertIn('Project is unavailable', bad.stderr)

    def test_narrow_scrolling_complete_details_redaction_and_icon_modes(self):
        secret = 'private-worker-' + 'long-'*20
        self.tmux('rename-window', '-t', self.worker, secret)
        for i in range(45): (self.repo / ('private-file-%02d.txt' % i)).write_text('never read this content')
        for mode in ['safe', 'nerd']:
            self.tmux('set-option', '-g', '@drudwyn-icon-mode', mode)
            for redacted in [False, True]:
                self.tmux('set-option', '-g', '@drudwyn-redact-labels', 'on' if redacted else 'off')
                pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project', '-c', str(self.repo), 'env', 'DRUDWYN_CLIENT=' + self.clients[0], str(BIN), 'cockpit')
                self.wait_pane(pane, 'MATCHING 1')
                for width in [48,64,80,120,160]:
                    self.tmux('resize-window', '-t', pane, '-x', str(width), '-y', '24')
                    screen = self.wait_pane(pane, self.worker)
                    if redacted: self.assertNotIn('private-', screen)
                    Path(f'/tmp/drudwyn-ticket09-{mode}-{width}-redact{int(redacted)}.txt').write_text(screen)
                    self.tmux('send-keys', '-t', pane, 'd')
                    screen = self.wait_pane(pane, 'DETAILS')
                    Path(f'/tmp/drudwyn-ticket09-details-{mode}-{width}-redact{int(redacted)}.txt').write_text(screen)
                    frames = screen
                    for _ in range(25):
                        self.tmux('send-keys', '-t', pane, 'PageDown')
                        time.sleep(.015)
                        frames += self.tmux('capture-pane', '-p', '-t', pane)
                    self.assertIn('Checks: unknown', frames)
                    self.assertIn('primary checkout', self.tmux('capture-pane', '-p', '-t', pane))
                    Path(f'/tmp/drudwyn-ticket09-details-end-{mode}-{width}-redact{int(redacted)}.txt').write_text(self.tmux('capture-pane', '-p', '-t', pane))
                    if redacted:
                        self.assertNotIn('private-', frames)
                        self.assertNotIn(str(self.repo), frames)
                    else:
                        self.assertIn('private-file-44.txt', frames)
                        self.assertIn(secret, ''.join(c for c in frames if not c.isspace() and c != '│'))
                    self.assertNotIn('never read this content', frames)
                    self.tmux('send-keys', '-t', pane, 'd')
                    self.wait_pane(pane, 'MATCHING 1')
                self.tmux('kill-pane', '-t', pane)

    def test_known_batch_and_literal_task_reference_become_unknown_after_metadata_loss(self):
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        commit = subprocess.check_output(['git', '-C', str(self.repo), 'rev-parse', 'HEAD'], text=True).strip()
        output = self.command('batch', 'setup', '--repo', str(self.repo), '--yes', '--expect-source', commit, '--expect-destination', commit, client=self.clients[0]).stdout
        batch = next(line.removeprefix('Batch: ') for line in output.splitlines() if line.startswith('Batch: '))
        self.command('batch', 'select', batch, '--window', self.worker, client=self.clients[0])
        reference = 'task $literal.md'
        self.tmux('set-option', '-w', '-t', self.worker, '@drudwyn_task_reference', reference.encode().hex())
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project', '-c', str(self.repo), 'env', 'DRUDWYN_CLIENT=' + self.clients[0], str(BIN), 'cockpit', '--search', 'refs/heads/main')
        self.wait_pane(pane, 'MATCHING 1')
        self.tmux('send-keys', '-t', pane, 'd')
        screen = self.wait_pane(pane, 'DETAILS')
        self.assertIn(commit, screen)
        self.assertIn('Destination: main', screen)
        self.assertIn(reference, screen)
        self.assertIn('Integration: unknown', screen)
        self.assertIn('Checks: unknown', screen)
        self.tmux('set-option', '-u', '-t', 'project', '@drudwyn_batch_' + batch.split('/')[1])
        self.tmux('set-option', '-wu', '-t', self.worker, '@drudwyn_task_reference')
        self.tmux('send-keys', '-t', pane, 'd', 'x', 'r')
        self.wait_pane(pane, 'REFRESHING')
        self.wait_pane(pane, 'Snapshot: r refresh')
        self.tmux('send-keys', '-t', pane, 'd')
        screen = self.wait_pane(pane, 'Batch: unknown')
        self.assertIn('TASK REF unknown', screen)
        self.assertIn('metadata unknown or lost', screen)
        self.assertNotIn(reference, screen)

    def test_project_coordinator_action_and_supported_themes(self):
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        counts = self.command('cockpit', '--list', '--search', 'project', client=self.clients[0]).stdout
        self.assertIn('GLOBAL 1 workers · 1 live · 1 projects', counts)
        self.assertIn('MATCHING 1 workers · 1 coordinators', counts)
        self.command('navigate', '--window', self.worker, client=self.clients[0])
        other = self.selection(self.clients[1])
        for theme in ['rose-pine', 'moon', 'dawn']:
            pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project', '-c', str(self.repo), 'env', 'DRUDWYN_CLIENT=' + self.clients[0], str(BIN), 'cockpit', '--theme', theme)
            screen = self.wait_pane(pane, 'MATCHING 1')
            self.assertIn(self.home + ' · c return', screen)
            self.tmux('send-keys', '-t', pane, 'c')
            for _ in range(100):
                if self.selection(self.clients[0]).endswith(':' + self.home): break
                time.sleep(.02)
            self.assertTrue(self.selection(self.clients[0]).endswith(':' + self.home))
            self.assertEqual(self.selection(self.clients[1]), other)

    def test_pending_finish_cannot_retarget_same_name_replacement(self):
        checkout = Path(self.tmp.name) / 'pending-finish'
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'add', '-qb', 'pending-finish', str(checkout)], check=True)
        fake = Path(self.tmp.name) / 'fake-worker/codex'
        target = self.tmux('new-window', '-d', '-P', '-F', '#{window_id}', '-t', 'project', '-n', 'pending-worker', '-c', str(checkout), str(fake), '300')
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project', '-c', str(self.repo), 'env', 'DRUDWYN_CLIENT=' + self.clients[0], str(BIN), 'cockpit', '--search', 'pending-worker')
        self.wait_pane(pane, 'MATCHING 1')
        self.tmux('send-keys', '-t', pane, 'f')
        self.wait_pane(pane, 'FINISH WORKSPACE')
        self.tmux('kill-window', '-t', target)
        replacement = self.tmux('new-window', '-d', '-P', '-F', '#{window_id}', '-t', 'project', '-n', 'pending-worker', '-c', str(checkout), str(fake), '300')
        self.tmux('send-keys', '-t', pane, 'y')
        self.wait_pane(pane, 'Pending Finish target changed')
        self.assertTrue(checkout.exists())
        self.assertEqual(self.tmux('display-message', '-p', '-t', replacement, '#{window_id}'), replacement)
        subprocess.run(['git', '-C', str(self.repo), 'show-ref', '--verify', '--quiet', 'refs/heads/pending-finish'], check=True)

if __name__ == '__main__':
    names = [n for n in GlobalCockpitTest.__dict__ if n.startswith('test_') and (len(sys.argv) == 1 or sys.argv[1] in n)]
    result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(GlobalCockpitTest(n) for n in names))
    sys.exit(not result.wasSuccessful())
