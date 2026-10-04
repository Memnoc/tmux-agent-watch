"""Assembled release acceptance: real Git, plugin, tmux and two attached clients.

Run after `cargo build --release --locked`. Only the agent is controlled: a
verified private copy of cat receives synthetic tasks. Git and tmux are real.
Artifacts contain synthetic fixture data only and are written under /tmp.
"""
from pathlib import Path
import hashlib
import os
import re
import shutil
import shlex
import signal
import subprocess
import sys
import time
import unittest

from independent_navigation_test import IndependentNavigation, ROOT
from status_a_test import StatusATest, plain
from worker_integration_test import WorkerIntegrationTest

BIN = ROOT / 'target/release/tmux-drudwyn'


class AssembledWorkflowTest(IndependentNavigation):
    git = WorkerIntegrationTest.git
    token = WorkerIntegrationTest.token
    resize_client = StatusATest.resize_client
    terminal = StatusATest.terminal
    def converged(self, client, width):
        session,window=self.selection(client).split(':')
        expected=plain(self.command('status-bar','--projection','--session',session,
                                   '--window',window,'--width',str(width),'--row','balanced',
                                   client=client).stdout).strip()
        deadline=time.monotonic()+4
        while True:
            screen,data=self.terminal(client,width,'0 NEED')
            if screen.lines()[-1].strip()==expected:return screen,data
            self.assertLess(time.monotonic(),deadline,(expected,screen.lines()[-1]))


    def tmux(self, *args, check=True):
        if args[:3] == ('-f', '/dev/null', 'new-session'):
            args = (*args, 'bash', '--noprofile', '--norc')
        return super().tmux(*args, check=check)

    def command(self, *args, client=None, check=True, input=None):
        env = self.env.copy()
        env.pop('DRUDWYN_CLIENT', None)
        if client:
            env['DRUDWYN_CLIENT'] = client
        result = subprocess.run([str(BIN), *args], env=env, input=input,
                                capture_output=True, text=True)
        if check:
            self.assertEqual(result.returncode, 0, f'{args}: {result.stdout}\n{result.stderr}')
        return result

    def worker_hook(self, event):
        pane = self.tmux('display-message', '-p', '-t', self.worker, '#{pane_id}')
        subprocess.run([str(BIN), 'hook', 'codex', event],
                       env={**self.env, 'TMUX_PANE': pane}, capture_output=True, check=True)

    def setUp(self):
        self.assertTrue(BIN.is_file(), 'Build the release binary before assembled acceptance')
        super().setUp()
        self.tmux('kill-window', '-t', self.worker)
        self.root = Path(self.tmp.name)
        shell = self.root / 'controlled-shell'
        shell.write_text('#!/bin/sh\nexec bash --noprofile --norc "$@"\n')
        shell.chmod(0o755)
        self.tmux('set', '-g', 'default-shell', str(shell))
        self.git('config', 'user.name', 'Acceptance fixture')
        self.git('config', 'user.email', 'acceptance@example.invalid')
        (self.repo / 'shared.txt').write_text('base\n')
        (self.repo / 'task.md').write_text('Synthetic repository-owned task\n')
        self.git('add', '.')
        self.git('commit', '-qm', 'acceptance baseline')
        self.base = self.git('rev-parse', 'HEAD')
        self.session = self.selection(self.clients[0]).split(':')[0]
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        self.agents = self.root / 'agents'
        self.agents.mkdir()
        self.agent = self.agents / 'codex'
        native = self.agents / 'native'; native.mkdir()
        self.native_agent = native / 'codex'
        shutil.copy(shutil.which('cat'), self.native_agent)
        self.agent.write_text('#!/bin/sh\nstty raw -echo\nexec ' + shlex.quote(str(self.native_agent)) + '\n')
        self.agent.chmod(0o755)
        self.assertEqual(hashlib.sha256(self.native_agent.read_bytes()).digest(),
                         hashlib.sha256(Path(shutil.which('cat')).read_bytes()).digest())
        self.env['PATH'] = str(self.agents) + os.pathsep + self.env['PATH']
        self.assertEqual(shutil.which('codex', path=self.env['PATH']), str(self.agent))
        self.tmux('set-environment', '-g', 'PATH', self.env['PATH'])
        self.env.pop('DRUDWYN_V2_BIN', None)
        self.tmux('set-environment', '-gu', 'DRUDWYN_V2_BIN')
        # Exercise normal plugin binary selection, which must choose release.
        resolved = subprocess.run(['bash', '-x', str(ROOT / 'scripts/v2.sh'), 'status'],
                                  env=self.env, text=True, capture_output=True)
        self.assertEqual(resolved.returncode, 0, resolved.stderr)
        self.assertIn('+ exec ' + str(BIN) + ' status', resolved.stderr)
        subprocess.run(['bash', str(ROOT / 'tmux-drudwyn.tmux')], env=self.env,
                       capture_output=True, check=True)
        self.tmux('set', '-g', 'status-interval', '1')

    def assert_agent(self, window):
        pid = self.tmux('display-message', '-p', '-t', window, '#{pane_pid}')
        self.assertEqual(Path('/proc', pid, 'exe').resolve(), self.native_agent.resolve())

    def stop_agent(self, window):
        self.tmux('set', '-w', '-t', window, 'remain-on-exit', 'on')
        pid = int(self.tmux('display-message', '-p', '-t', window, '#{pane_pid}'))
        os.kill(pid, signal.SIGTERM)
        deadline = time.monotonic() + 4
        while self.tmux('display-message', '-p', '-t', window, '#{pane_dead}') != '1':
            self.assertLess(time.monotonic(), deadline)
            time.sleep(.02)

    def reviewed(self, action, path, destination):
        args = ['workspace', action, '--path', str(path), '--destination', destination]
        preview = self.command(*args, client=self.clients[0])
        return self.command(*args, '--apply', self.token(preview),
                            client=self.clients[0], check=False)

    def verify(self, path, script=None):
        args = ['workspace', 'verify', '--path', str(path)]
        if script is not None:
            args += ['--check', 'assembled-input-check', '--command-stdin']
        return self.command(*args, input=script, check=False)

    def test_direct_to_base(self):
        self.scenario(integration=False)

    def test_integration_branch_and_promotion(self):
        self.scenario(integration=True)

    def scenario(self, integration):
        flow = 'integration' if integration else 'direct'
        began = time.monotonic()
        other = self.selection(self.clients[1])
        created = self.command('session', 'new', '--name', 'acceptance editing',
                               '--directory', str(self.repo), client=self.clients[0])
        self.assertTrue(self.selection(self.clients[0]).startswith(created.stdout.strip() + ':'))
        self.assertEqual(other, self.selection(self.clients[1]))
        self.command('navigate', '--window', self.home, '--session', self.session,
                     client=self.clients[0])
        target = self.root / 'assembled' if integration else self.repo
        destination = 'assemble' if integration else 'main'
        args = ['batch', 'setup', '--repo', str(self.repo), '--session', self.session]
        if integration:
            args += ['--integration', destination, '--checkout', str(target)]
        preview = self.command(*args, client=self.clients[0])
        self.assertIn(self.base, preview.stdout)
        batch = self.command(*args, '--yes', '--expect-source', self.base,
                             '--expect-destination', self.base, client=self.clients[0])
        batch_id = re.search(r'Batch: (\S+)', batch.stdout)[1]
        self.command('batch', 'select', batch_id, '--window', self.home)
        # Keep the stable coordinator window; its controlled receiver runs where
        # conflict instructions must be delivered, in the chosen destination.
        self.tmux('respawn-pane', '-k', '-t', self.home, '-c', str(target), str(self.agent))
        self.assert_agent(self.home)
        sources, windows, commits = {}, {}, {}
        for name in ['alpha', 'beta', 'gamma']:
            task = None if name == 'alpha' else f'SYNTHETIC_{name}_TASK\nSecond line'
            task_args = ['--task-file', 'task.md'] if task is None else ['--task-stdin']
            result = self.command('workspace', 'start', '--repo', str(self.repo),
                                  '--batch', batch_id, '--name', name, *task_args,
                                  f'work/{name}', str(self.agent), input=task,
                                  client=self.clients[0])
            sources[name] = Path(result.stdout.splitlines()[0])
            windows[name] = re.search(r'window (@\d+)', result.stderr)[1]
            self.assert_agent(windows[name])
            self.assertEqual(self.git('rev-parse', 'HEAD', path=sources[name]), self.base)
            self.assertEqual(self.tmux('show', '-wqv', '-t', windows[name], '@drudwyn_delivery'), 'sent')
            self.assertEqual(self.tmux('display', '-p', '-t', windows[name], '#{window_name}'), name)
            self.assertEqual(self.tmux('show', '-qv', '-t', self.session, '@drudwyn_coordinator'), self.home)
            self.assertEqual(other, self.selection(self.clients[1]))
            file = 'gamma.txt' if name == 'gamma' else 'shared.txt'
            (sources[name] / file).write_text(name + '\n')
            self.git('add', '.', path=sources[name])
            self.git('commit', '-qm', f'{name} independent change', path=sources[name])
            commits[name] = self.git('rev-parse', 'HEAD', path=sources[name])

        # Real clients independently select siblings; inspection never moves either.
        self.command('navigate', '--window', windows['alpha'], client=self.clients[0])
        first = self.selection(self.clients[0])
        self.command('navigate', '--window', windows['gamma'], client=self.clients[1])
        self.assertEqual(first, self.selection(self.clients[0]))
        other = self.selection(self.clients[1])
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', self.session,
                         '-c', str(self.repo), 'env', 'DRUDWYN_CLIENT=' + self.clients[0],
                         str(BIN), 'cockpit', '--search', 'beta')
        screen = self.wait_pane(pane, 'Snapshot: r refresh')
        self.assertIn('MATCHING 1', screen)
        self.assertEqual(first, self.selection(self.clients[0]))
        self.assertEqual(other, self.selection(self.clients[1]))
        self.tmux('send-keys', '-t', pane, 'Enter')
        deadline = time.monotonic() + 4
        while not self.selection(self.clients[0]).endswith(':' + windows['beta']):
            self.assertLess(time.monotonic(), deadline)
            time.sleep(.02)
        self.assertEqual(other, self.selection(self.clients[1]))
        self.stop_agent(windows['beta'])
        listing = self.command('workspace', 'recover-list', '--repo', str(self.repo)).stdout
        self.assertIn('Stopped workspace', listing)
        old_beta = windows['beta']
        recovered = self.command('workspace', 'recover', '--repo', str(self.repo),
                                 '--path', str(sources['beta']), '--agent', 'codex',
                                 '--batch', batch_id, '--task-file', 'task.md', client=self.clients[0])
        windows['beta'] = recovered.stdout.splitlines()[0]
        self.assertIn('Fresh conversation', recovered.stdout)
        self.assertNotEqual(windows['beta'], old_beta)
        self.assert_agent(windows['beta'])
        self.assertEqual(self.git('rev-parse', 'HEAD', path=sources['beta']), commits['beta'])
        self.assertEqual(self.tmux('display', '-p', '-t', old_beta, '#{pane_dead}'), '1')
        self.assertEqual(other, self.selection(self.clients[1]))

        self.assertIn('Integrated', self.reviewed('integrate', sources['alpha'], destination).stdout)
        self.assertIn('Integrated', self.reviewed('integrate', sources['gamma'], destination).stdout)
        conflict = self.reviewed('integrate', sources['beta'], destination)
        self.assertNotEqual(conflict.returncode, 0)
        conflict_args = ['workspace', 'conflict', '--path', str(target)]
        status = self.command(*conflict_args).stdout
        self.assertIn('Handoff: sent', status)
        self.assertEqual(self.git('rev-parse', 'MERGE_HEAD', path=target), commits['beta'])
        self.assertEqual(self.tmux('list-buffers', '-F', '#{buffer_name}'), '')
        # Abort is deliberate; a fresh reviewed merge can then be resolved.
        self.assertIn('Aborted', self.command(*conflict_args, '--abort').stdout)
        self.assertEqual(self.git('status', '--porcelain', path=target), '')
        self.assertNotEqual(self.reviewed('integrate', sources['beta'], destination).returncode, 0)
        self.assertIn('Handoff: sent', self.command(*conflict_args).stdout)
        # Test actor stands in for the coordinator editing; Drudwyn never parses it.
        (target / 'shared.txt').write_text('alpha\nbeta\n')
        self.git('add', 'shared.txt', path=target)
        self.assertIn('Completed', self.command(*conflict_args, '--continue').stdout)
        for commit in commits.values():
            self.git('merge-base', '--is-ancestor', commit, destination)
        self.assertEqual((target / 'gamma.txt').read_text(), 'gamma\n')
        self.assertIn('Not verified', self.verify(target).stdout)
        (target / 'application-input').write_text('untracked failing input\n')
        script = 'printf "ASSEMBLED_VISIBLE_CHECK\\n"; test ! -e application-input\n'
        failed = self.verify(target, script)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn('ASSEMBLED_VISIBLE_CHECK', failed.stdout)
        self.assertIn('Failed', failed.stdout)
        (target / 'application-input').unlink()
        self.assertIn('Passed', self.verify(target, script).stdout)
        self.assertIn('Passed', self.verify(target).stdout)
        if integration:
            self.assertEqual(self.git('rev-parse', 'main'), self.base)
            self.assertIn('Not verified', self.verify(self.repo).stdout)
            promoted = self.command('workspace', 'promote', '--path', str(target), '--base', 'main')
            self.assertIn('MERGE COMBINED BRANCH', promoted.stdout)
            self.assertIn('Integrated', self.command('workspace', 'promote', '--path', str(target),
                                                    '--base', 'main', '--apply', self.token(promoted)).stdout)
            self.assertIn('Not verified', self.verify(self.repo).stdout)
            self.assertIn('Passed', self.verify(self.repo, script).stdout)

        # Live agents must be stopped explicitly even after verified integration.
        blocked = self.command('workspace', 'finish', '--path', str(sources['alpha']),
                               '--destination', destination, '--yes', check=False)
        self.assertIn('active writer', blocked.stderr)
        self.command('coordinator', 'open', client=self.clients[0])
        self.command('coordinator', 'open', client=self.clients[1])
        for name, window in windows.items():
            self.stop_agent(window)
            args = ['workspace', 'finish', '--path', str(sources[name]), '--destination', destination]
            preview = self.command(*args, '--preview')
            self.command(*args, '--apply', self.token(preview), '--yes')
            self.assertFalse(sources[name].exists())
            self.assertEqual(self.git('rev-parse', f'work/{name}'), commits[name])
        live_windows = self.tmux('list-windows', '-a', '-F', '#{window_id}').splitlines()
        self.assertIn(self.home, live_windows)
        self.assertNotIn(old_beta, live_windows)
        self.assertEqual(self.tmux('show', '-qv', '-t', self.session, '@drudwyn_coordinator'), self.home)
        self.assertEqual(self.git('branch', '--show-current'), 'main')
        self.assertEqual(self.git('remote'), '')
        metadata = self.tmux('show-options', '-s') + self.tmux('show-options', '-g')
        for window in set(live_windows):
            metadata += self.tmux('show-options', '-w', '-t', window)
        for secret in ['SYNTHETIC_beta_TASK', 'SYNTHETIC_gamma_TASK', 'ASSEMBLED_VISIBLE_CHECK', 'test ! -e']:
            self.assertNotIn(secret, metadata)
        self.resize_client(self.clients[0], 160)
        screen, ansi = self.converged(self.clients[0], 160)
        self.assertIn('COORD', screen.lines()[-1])
        self.assertEqual(self.tmux('show','-gqv','status'),'2')
        artifact = Path(f'/tmp/drudwyn-ticket15-{flow}')
        artifact.with_suffix('.txt').write_text('\n'.join(screen.lines()[-1:]) + '\n')
        artifact.with_suffix('.ansi').write_bytes(ansi)
        print(f'ASSEMBLED {flow}: two clients, three pinned workers, recovery, conflict Abort/Continue, '
              f'checks, promotion={integration}, guarded cleanup: {time.monotonic() - began:.3f}s', flush=True)


if __name__ == '__main__':
    names = [n for n in AssembledWorkflowTest.__dict__ if n.startswith('test_')
             and (len(sys.argv) == 1 or sys.argv[1] in n)]
    result = unittest.TextTestRunner(verbosity=2).run(
        unittest.TestSuite(AssembledWorkflowTest(n) for n in names))
    sys.exit(not result.wasSuccessful())
