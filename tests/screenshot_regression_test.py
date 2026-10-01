"""Real terminal regressions from the 2026-10-01 unregistered-session screenshots."""
import re
import subprocess
import unittest
from pathlib import Path
from independent_navigation_test import IndependentNavigation, BIN


class ScreenshotRegression(IndependentNavigation):
    def four_workers(self, separate_repos=False):
        self.worker_hook('userPromptSubmit')
        workers = [self.worker]
        fake = Path(self.tmp.name) / 'fake-worker/codex'
        self.assertEqual(fake.resolve(), Path('/usr/bin/sleep').resolve())
        for index in range(3):
            repo = self.repo
            if separate_repos:
                repo = Path(self.tmp.name) / f'repo-{index}'
                subprocess.run(['git', 'clone', '-q', str(self.repo), str(repo)], check=True)
            workers.append(self.tmux('new-window', '-d', '-P', '-F', '#{window_id}',
                                     '-t', 'project', '-n', f'worker-{index}',
                                     '-c', str(repo), str(fake), '300'))
        return workers

    def test_four_workers_visible_at_screenshot_size_and_actions_discoverable(self):
        workers = self.four_workers()
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project',
                         '-c', str(self.repo), 'env', 'DRUDWYN_CLIENT=' + self.clients[0],
                         str(BIN), 'cockpit')
        self.tmux('resize-window', '-t', pane, '-x', '84', '-y', '27')
        screen = self.wait_pane(pane, 'MATCHING 4')
        Path('/tmp/drudwyn-screenshot-cockpit.txt').write_text(screen)
        for name in ['worker-0', 'worker-1', 'worker-2']:
            self.assertIn(name, screen)
        self.assertIn('WORKING', screen)
        self.assertIn('[?] Actions', screen)
        self.tmux('send-keys', '-t', pane, '?')
        screen = self.wait_pane(pane, 'COCKPIT ACTIONS')
        for action in ['Integrate', 'Promote', 'Conflict', 'Verify', 'Finish']:
            self.assertIn(action, screen)
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'MATCHING 4')
        self.tmux('send-keys', '-t', pane, 'End')
        self.tmux('send-keys', '-t', pane, 'd')
        screen = self.wait_pane(pane, 'DETAILS')
        self.assertIn('IDENTITY ' + self.worker, screen)
        self.assertIn('project unknown', screen)

    def test_wide_cockpit_uses_branch_space_and_complete_sidebar_values(self):
        branch='work/worktree-worker-workflow'
        subprocess.run(['git','-C',str(self.repo),'branch','-m',branch],check=True)
        self.worker_hook('userPromptSubmit')
        pane=self.tmux('new-window','-d','-P','-F','#{pane_id}','-t','project',
                       '-c',str(self.repo),'env','DRUDWYN_CLIENT='+self.clients[0],str(BIN),'cockpit')
        self.tmux('resize-window','-t',pane,'-x','200','-y','38')
        screen=self.wait_pane(pane,'MATCHING 1')
        worker_rows=[line for line in screen.splitlines() if 'WORKING' in line and 'AGENT' in line]
        self.assertTrue(worker_rows,screen)
        self.assertIn(branch,worker_rows[0])
        self.assertIn('Verification',screen)
        self.assertIn('Not verified',screen)
        self.assertIn('[d] Full details',screen)

    def test_branded_overview_hides_raw_ids_and_full_paths(self):
        self.four_workers(separate_repos=True)
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project',
                         '-c', str(self.repo), 'env', 'DRUDWYN_CLIENT=' + self.clients[0],
                         str(BIN), 'cockpit')
        self.tmux('resize-window', '-t', pane, '-x', '84', '-y', '27')
        screen = self.wait_pane(pane, 'MATCHING 4')
        self.assertIn('Drudwyn', screen)
        self.assertRegex(screen, '[█▀▄\u2800-\u28ff]')
        self.assertNotRegex(screen, r'@\d+')
        self.assertNotIn(str(self.repo), screen)
        for name in ['worker-0', 'worker-1', 'worker-2']:
            self.assertIn(name, screen)
        self.tmux('send-keys', '-t', pane, 'd')
        detail = self.wait_pane(pane, 'DETAILS')
        self.assertIn(str(self.repo), detail)
        self.assertRegex(detail, r'IDENTITY @\d+')
        self.tmux('kill-pane', '-t', pane)
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project',
                         'env', 'DRUDWYN_CLIENT=' + self.clients[0], str(BIN), 'navigator')
        self.tmux('resize-window', '-t', pane, '-x', '84', '-y', '27')
        screen = self.wait_pane(pane, 'WORKSPACE NAVIGATOR')
        self.assertNotRegex(screen, r'@\d+')
        self.assertIn('[Esc] Close', screen)

    def test_same_named_projects_keep_distinct_short_labels(self):
        fake = Path(self.tmp.name) / 'fake-worker/codex'
        for suffix in ['one', 'two']:
            repo = Path(self.tmp.name) / ('shared-long-parent-' * 3 + suffix) / 'same-repo'
            repo.parent.mkdir()
            subprocess.run(['git', 'clone', '-q', str(self.repo), str(repo)], check=True)
            self.tmux('new-window', '-d', '-t', 'project', '-n', 'same-worker',
                      '-c', str(repo), str(fake), '300')
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project',
                         '-c', str(self.repo), 'env', 'DRUDWYN_CLIENT=' + self.clients[0],
                         str(BIN), 'cockpit')
        self.tmux('resize-window', '-t', pane, '-x', '100', '-y', '27')
        screen = self.wait_pane(pane, 'MATCHING 3')
        self.assertIn('same-repo [1]', screen)
        self.assertIn('same-repo [2]', screen)

    def test_agent_navigator_identifies_same_window_number_in_two_sessions(self):
        fake = Path(self.tmp.name) / 'fake-worker/codex'
        self.tmux('rename-window', '-t', self.worker, 'same-worker')
        self.tmux('new-session', '-d', '-s', 'other', '-c', str(self.repo), 'bash', '--noprofile', '--norc')
        self.tmux('new-window', '-d', '-t', 'other:1', '-n', 'same-worker',
                  '-c', str(self.repo), str(fake), '300')
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project',
                         'env', 'DRUDWYN_CLIENT=' + self.clients[0], str(BIN), 'navigator')
        self.tmux('resize-window', '-t', pane, '-x', '84', '-y', '27')
        screen = self.wait_pane(pane, 'WORKSPACE NAVIGATOR')
        agents = screen.split('AGENTS', 1)[1]
        self.assertIn('session project', agents)
        self.assertIn('session other', agents)
        self.assertNotRegex(agents, r'@\d+')

    def test_approved_cockpit_table_keeps_columns_and_selected_summary(self):
        self.four_workers(separate_repos=True)
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project',
                         '-c', str(self.repo), 'env', 'DRUDWYN_CLIENT=' + self.clients[0],
                         str(BIN), 'cockpit')
        self.tmux('resize-window', '-t', pane, '-x', '160', '-y', '42')
        screen = self.wait_pane(pane, 'MATCHING 4')
        header = next((line for line in screen.splitlines() if 'WORKER' in line and 'ACTIVITY' in line), '')
        for column in ['WORKER', 'AGENT', 'ACTIVITY', 'BRANCH', 'GIT', 'INTEGRATION']:
            self.assertIn(column, header)
        self.assertIn('SELECTED WORKER', screen)
        for total in ['1 WORK', '3 RUN', '0 UNCONFIRMED']:
            self.assertIn(total, screen)
        self.assertIn('0 need you', screen)
        for name in ['worker-0', 'worker-1', 'worker-2']:
            self.assertIn(name, screen)
        self.assertNotRegex(screen, r'@\d+')

    def test_status_name_has_gutter_and_is_not_repeated_in_context(self):
        self.worker_hook('userPromptSubmit')
        self.tmux('rename-window', '-t', self.worker, 'codex-drudwyn')
        self.command('navigate', '--window', self.worker, client=self.clients[0])
        self.tmux('set', '-g', '@drudwyn-visible-tabs', '1')
        session, window = self.selection(self.clients[0]).split(':')
        result = self.command('status-bar', '--session', session, '--window', window,
                              '--width', '160', client=self.clients[0]).stdout
        top, bottom = result.splitlines()
        self.assertRegex(top, r'codex-drudwyn {2}#\[bg=')
        self.assertNotIn('codex-drudwyn', bottom)

    def test_known_repositories_count_without_inventing_coordination(self):
        self.four_workers()
        result = self.command('cockpit', '--list', client=self.clients[0]).stdout
        self.assertIn('4 live · 1 project', result)
        self.assertNotIn('\tunassociated\t', result)
        self.assertEqual(self.tmux('show', '-qv', '-t', 'project', '@drudwyn_coordinator'), '')
        # A linked checkout is the same repository, even without a managed launch.
        linked = Path(self.tmp.name) / 'linked'
        subprocess.run(['git', '-C', str(self.repo), 'worktree', 'add', '-qb', 'feature', str(linked)], check=True)
        self.tmux('new-window', '-d', '-t', 'project', '-c', str(linked),
                  str(Path(self.tmp.name) / 'fake-worker/codex'), '300')
        result = self.command('cockpit', '--list', client=self.clients[0]).stdout
        self.assertIn('5 live · 1 project', result)
        other = Path(self.tmp.name) / 'other-repo'
        subprocess.run(['git', 'clone', '-q', str(self.repo), str(other)], check=True)
        outside = self.tmux('new-window', '-d', '-P', '-F', '#{window_id}', '-t', 'project',
                            '-n', 'outside-repository', '-c', str(other),
                            str(Path(self.tmp.name) / 'fake-worker/codex'), '300')
        unknown = self.command('cockpit', '--list', '--project', 'unassociated', client=self.clients[0]).stdout
        self.assertIn('MATCHING 0 workers', unknown)
        filtered = self.command('cockpit', '--list', '--project', str(self.repo), client=self.clients[0]).stdout
        self.assertIn('MATCHING 5 workers', filtered)
        self.assertNotIn(outside + '\t', filtered)
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project',
                         '-c', str(self.repo), 'env', 'DRUDWYN_CLIENT=' + self.clients[0],
                         str(BIN), 'cockpit', '--project', str(self.repo))
        self.wait_pane(pane, 'MATCHING 5')
        self.tmux('send-keys', '-t', pane, 'w')
        screen = self.wait_pane(pane, 'MATCHING 7 windows')
        self.assertNotIn('outside-repository', screen)

    def test_attention_total_is_labelled_in_status(self):
        # No hook reports attention: live workers must not look like GLOBAL 0 workers.
        self.worker_hook('userPromptSubmit')
        session, window = self.selection(self.clients[0]).split(':')
        output = self.command('status-bar', '--session', session, '--window', window,
                              '--width', '160', '--row', 'context', client=self.clients[0],
                              check=False)
        self.assertEqual(output.returncode, 0, output.stderr)
        text = re.sub(r'(?<!#)#\[[^\]]*\]', '', output.stdout)
        self.assertIn('NEED YOU 0', text)

    def test_navigator_identifies_numeric_session_label(self):
        self.tmux('rename-session', '-t', 'project', '7')
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', '7',
                         'env', 'DRUDWYN_CLIENT=' + self.clients[0], str(BIN), 'navigator')
        screen = self.wait_pane(pane, 'WORKSPACE NAVIGATOR')
        self.assertIn('session 7', screen)


def load_tests(loader, tests, pattern):
    return unittest.TestSuite(ScreenshotRegression(name) for name in
                              ScreenshotRegression.__dict__ if name.startswith('test_'))


if __name__ == '__main__':
    unittest.main()
