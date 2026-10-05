"""Approved three-region A through public CLI and installed tmux status cells."""
import subprocess
import unittest
from independent_navigation_test import ROOT, BIN
from status_a_test import plain, StatusATest

class BalancedStatus(StatusATest):
    def test_current_workspace_centered_git_global_attention_and_separator(self):
        self.worker_hook('permissionRequest')
        path = self.repo / 'lines.txt'
        path.write_text('one\ntwo\nthree\n')
        subprocess.run(['git', '-C', str(self.repo), 'add', '.'], check=True)
        subprocess.run(['git', '-C', str(self.repo), 'commit', '-qm', 'Fixture'], check=True)
        path.write_text('one\nfour\n')
        self.tmux('new-window', '-d', '-t', 'project', '-n', 'hidden-shell', 'sleep', '300')
        self.command('navigate', '--window', self.worker, client=self.clients[0])
        self.tmux('set', '-g', 'mouse', 'on')
        self.tmux('set-environment', '-g', 'DRUDWYN_V2_BIN', str(BIN))
        installed = subprocess.run(['bash', str(ROOT / 'tmux-drudwyn.tmux')],
            env={**self.env, 'DRUDWYN_V2_BIN': str(BIN)}, capture_output=True, text=True)
        self.assertEqual(installed.returncode, 0, installed.stderr)
        self.assertEqual(self.tmux('show', '-gqv', 'status'), '2')
        self.assertIn('separator', self.tmux('show', '-gqv', 'status-format[0]'))
        self.assertIn('balanced', self.tmux('show', '-gqv', 'status-format[1]'))
        session, window = self.selection(self.clients[0]).split(':')
        for width in [48, 64, 80, 120, 160]:
            output = self.command('status-bar', '--projection', '--session', session,
                '--window', window, '--width', str(width), '--row', 'balanced',
                client=self.clients[0]).stdout
            row = plain(output).rstrip('\n')
            self.assertEqual(len(row), width)
            self.assertNotIn('hidden-shell', row)
            for expected in ['+1 -2', '1 NEED', 'worker']:
                self.assertIn(expected, row)
            if width >= 64:
                middle = 'main +1 -2'
                self.assertIn(middle, row)
                self.assertLessEqual(abs(row.index(middle) + len(middle)/2 - width/2), .5)
            self.assertIn('#[bg=#191724,', output)
        before = [self.selection(c) for c in self.clients]
        self.resize_client(self.clients[0], 160)
        screen, _ = self.terminal(self.clients[0], 160, '1 NEED')
        self.assertIn('─' * 80, screen.lines()[-2])
        row = screen.lines()[-1]
        self.assertIn('1 AGENTS', row)
        self.click(self.clients[0], row.index('NEED'), 39)
        self.terminal(self.clients[0], 160, 'state attention')
        self.assertEqual([self.selection(c) for c in self.clients], before)
        self.key(self.clients[0], b'q')
        screen, _ = self.terminal(self.clients[0], 160, '1 AGENTS')
        self.click(self.clients[0], screen.lines()[-1].index('AGENTS'), 39)
        self.terminal(self.clients[0], 160, 'state all')
        self.assertEqual([self.selection(c) for c in self.clients], before)
        self.key(self.clients[0], b'q')
        self.tmux('set', '-g', '@drudwyn-redact-labels', 'on')
        output = self.command('status-bar', '--session', session, '--window', window,
                             '--width', '120', '--row', 'balanced', client=self.clients[0]).stdout
        self.assertIn('Workspace', plain(output))
        self.assertNotIn('worker', plain(output))
        self.assertNotIn('main', plain(output))

    def test_long_worktree_exit_stale_evidence_is_bounded_in_both_clients(self):
        self.exited_worker('stop', 23, managed=True)
        self.tmux('rename-window', '-t', self.worker, 'very-long-worker-#[fg=red]-界界界-name')
        self.tmux('set', '-g', '@drudwyn_scan_at', '1')
        session, window = self.selection(self.clients[0]).split(':')
        for width in [20, 40, 48, 64, 80, 120, 160]:
            output = self.command('status-bar', '--projection', '--session', session,
                '--window', self.worker, '--width', str(width), '--row', 'balanced',
                client=self.clients[0]).stdout
            row = plain(output).rstrip('\n')
            self.assertLessEqual(len(row), width)
            self.assertIn('STALE', row)
            if width >= 48:
                self.assertIn('REV/X23', row)
                self.assertIn('WT', row)
                self.assertIn('2 NEED*', row)
        self.tmux('set', '-g', '@drudwyn-status-layout', 'balanced')
        self.tmux('set-environment', '-g', 'DRUDWYN_V2_BIN', str(BIN))
        subprocess.run(['bash', str(ROOT / 'scripts/hud-install.sh')], env=self.env, check=True)
        for client, target in [(self.clients[0], self.worker), (self.clients[1], self.home)]:
            self.command('navigate', '--window', target, client=client)
        self.tmux('set', '-g', '@drudwyn-icon-mode', 'safe')
        for client, text in [(self.clients[0], 'REV/X23'), (self.clients[1], '>_')]:
            self.resize_client(client, 80)
            screen, _ = self.terminal(client, 80, text)
            self.assertIn(text, screen.lines()[-1])
            self.assertIn('NEED', screen.lines()[-1])

    def test_branch_shortens_before_global_total_disappears_and_icons_follow_settings(self):
        self.worker_hook('permissionRequest')
        subprocess.run(['git', '-C', str(self.repo), 'switch', '-qc',
                        'work/a-very-long-branch-for-the-selected-workspace'], check=True)
        fake = self.repo.parent / 'fake-worker/codex'
        for i in range(11):
            self.tmux('new-window', '-d', '-t', 'project', '-n', f'other-{i}', str(fake), '300')
        self.command('status')
        self.tmux('set', '-g', '@drudwyn_scan_at', '1')
        self.tmux('set', '-g', '@drudwyn-icon-mode', 'nerd')
        self.tmux('set', '-g', '@drudwyn-agent-icon', 'auto')
        self.tmux('set', '-g', '@drudwyn-codex-icon', 'Z')
        row = plain(self.command('status-bar', '--projection', '--session', '$0',
                    '--window', self.worker, '--width', '64', '--row', 'balanced').stdout)
        self.assertIn('12 AGENTS', row)
        self.assertIn('+0 -0', row)
        self.assertIn('STALE', row)
        self.assertIn('Z worker', row)
        self.assertLessEqual(len(row.rstrip('\n')), 64)

    def test_themes_top_position_and_restore(self):
        self.tmux('set', '-g', 'status', 'on')
        self.tmux('set', '-g', 'status-format[0]', 'original bar')
        original_second = self.tmux('show', '-gqv', 'status-format[1]')
        self.tmux('set', '-g', '@drudwyn-status-layout', 'balanced')
        self.tmux('set', '-g', 'status-position', 'top')
        self.tmux('set-environment', '-g', 'DRUDWYN_V2_BIN', str(BIN))
        for theme, base, surface in [('rose-pine', '#191724', '#26233a'),
                                     ('moon', '#232136', '#393552'),
                                     ('dawn', '#faf4ed', '#f2e9e1')]:
            self.tmux('set', '-g', '@drudwyn-theme', theme)
            subprocess.run(['bash', str(ROOT / 'scripts/hud-install.sh')], env=self.env, check=True)
            self.assertIn('balanced', self.tmux('show', '-gqv', 'status-format[0]'))
            self.assertIn('separator', self.tmux('show', '-gqv', 'status-format[1]'))
            separator = subprocess.run(['bash', str(ROOT / 'scripts/status-separator.sh'), '80'],
                env=self.env, capture_output=True, text=True, check=True).stdout
            self.assertIn(base, separator)
            self.assertEqual(plain(separator), '─' * 80)
            output = self.command('status-bar', '--session', '$0', '--window', self.home,
                                  '--width', '80', '--row', 'balanced', client=self.clients[0]).stdout
            self.assertIn(f'#[bg={base},', output)
            self.assertNotIn(f'bg={surface}', output)
        subprocess.run(['bash', str(ROOT / 'scripts/hud-install.sh'), '--disable'], env=self.env, check=True)
        self.assertEqual(self.tmux('show', '-gqv', 'status'), 'on')
        self.assertEqual(self.tmux('show', '-gqv', 'status-format[0]'), 'original bar')
        self.assertEqual(self.tmux('show', '-gqv', 'status-format[1]'), original_second)

def load_tests(loader, tests, pattern):
    return unittest.TestSuite(BalancedStatus(n) for n in BalancedStatus.__dict__ if n.startswith('test_'))
if __name__ == '__main__': unittest.main()
