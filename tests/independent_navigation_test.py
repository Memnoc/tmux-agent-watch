"""Drudwyn routing with two real terminal clients, isolated Git and tmux."""
from pathlib import Path
import os
import fcntl
import struct
import termios
import pty
import shlex
import subprocess
import sys
import tempfile
import threading
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / 'target/debug/tmux-drudwyn'

class IndependentNavigation(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='drudwyn-clients-', dir='/tmp')
        self.addCleanup(self.tmp.cleanup)
        self.socket = str(Path(self.tmp.name) / 'tmux.sock')
        self.repo = Path(self.tmp.name) / 'repo'
        subprocess.run(['git', 'init', '-q', '-b', 'main', str(self.repo)], check=True)
        subprocess.run(['git', '-C', str(self.repo), '-c', 'user.name=Test', '-c',
                        'user.email=test@example.invalid', 'commit', '-qm', 'initial', '--allow-empty'], check=True)
        self.tmux('-f', '/dev/null', 'new-session', '-d', '-s', 'project', '-c', str(self.repo))
        self.addCleanup(lambda: self.tmux('kill-server', check=False))
        self.home = self.tmux('display-message', '-p', '-t', 'project', '#{window_id}')
        self.worker = self.tmux('new-window', '-d', '-P', '-F', '#{window_id}', '-t', 'project', '-n', 'worker', '-c', str(self.repo))
        self.tmux('set-option', '-w', '-t', self.worker, '@drudwyn_state', 'review')
        self.env = {**os.environ, 'TMUX': f'{self.socket},{self.tmux("display-message", "-p", "#{pid}")},0'}
        self.env.pop('TMUX_PANE', None)
        self.client_output = {}
        self.client_fds = {}
        self.clients = [self.attach(), self.attach()]

    def tmux(self, *args, check=True):
        return subprocess.run(['tmux', '-S', self.socket, *args], text=True,
                              capture_output=True, check=check).stdout.strip()

    def attach(self):
        before = set(self.tmux('list-clients', '-F', '#{client_name}').splitlines())
        fd, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 120, 0, 0))
        env = {**os.environ, 'TERM': 'xterm-256color'}
        env.pop('TMUX', None)
        runner = ('import fcntl, os, sys, termios; '
                  'fcntl.ioctl(0, termios.TIOCSCTTY, 0); '
                  'os.execvp(sys.argv[1], sys.argv[1:])')
        process = subprocess.Popen([sys.executable, '-c', runner, 'tmux', '-S',
                                    self.socket, 'attach-session', '-t', 'project'],
                                   stdin=slave, stdout=slave, stderr=slave,
                                   env=env, start_new_session=True)
        os.close(slave)
        output = bytearray()
        def drain():
            try:
                while True:
                    chunk = os.read(fd, 65536)
                    if not chunk: break
                    output.extend(chunk)
            except OSError:
                pass
        threading.Thread(target=drain, daemon=True).start()
        def cleanup():
            process.terminate()
            process.wait(timeout=5)
            os.close(fd)
        self.addCleanup(cleanup)
        for _ in range(200):
            names = set(self.tmux('list-clients', '-F', '#{client_name}').splitlines()) - before
            if names:
                name = names.pop()
                self.client_fds[name] = fd
                self.client_output[name] = output
                return name
            time.sleep(.01)
        self.fail('client did not attach')

    def selection(self, client):
        rows = self.tmux('list-clients', '-F', '#{client_name}␟#{session_id}:#{window_id}')
        return dict(row.split('␟') for row in rows.splitlines())[client]

    def command(self, *args, client=None, check=True):
        env = self.env.copy()
        if client: env['DRUDWYN_CLIENT'] = client
        else: env.pop('DRUDWYN_CLIENT', None)
        return subprocess.run([str(BIN), *args], env=env, capture_output=True, text=True, check=check)

    def ui(self, surface, query):
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project',
                         shlex.join(['env', f'DRUDWYN_CLIENT={self.clients[0]}', str(BIN), surface]))
        def wait(text):
            for _ in range(250):
                output = self.tmux('capture-pane', '-p', '-t', pane)
                if text in output: return
                time.sleep(.02)
            self.fail(f'no {text!r}: {output}')
        wait('NAVIGATOR' if surface != 'cockpit' else 'WORKSPACE')
        self.tmux('send-keys', '-t', pane, '-l', '/' + query)
        time.sleep(.1)
        self.tmux('send-keys', '-t', pane, 'Escape')
        time.sleep(.1)
        self.tmux('send-keys', '-t', pane, 'Enter')
        time.sleep(.3)

    def test_normal_tmux_selection_is_shared(self):
        self.tmux('select-window', '-t', self.worker)
        self.assertTrue(all(self.selection(c).endswith(':' + self.worker) for c in self.clients))

    def test_linked_views_count_worker_once_and_destroy_only_unused_view(self):
        other = self.selection(self.clients[1])
        panes = self.tmux('list-panes', '-a', '-F', '#{pane_id} #{pane_pid}')
        self.command('navigate', '--window', self.worker, client=self.clients[0])
        view = self.selection(self.clients[0]).split(':')[0]
        self.assertNotEqual(view, other.split(':')[0])
        self.assertEqual(len(self.command('status').stdout.splitlines()), 1)
        self.assertEqual(set(self.tmux('list-panes', '-a', '-F', '#{pane_id} #{pane_pid}').splitlines()), set(panes.splitlines()))
        self.command('navigate', '--window', self.home, client=self.clients[0])
        self.assertEqual(self.selection(self.clients[0]), view + ':' + self.home)
        self.tmux('detach-client', '-t', self.clients[0])
        self.assertNotIn(view, self.tmux('list-sessions', '-F', '#{session_id}').splitlines())
        self.assertEqual(self.selection(self.clients[1]), other)
        self.assertIn(self.worker, self.tmux('list-windows', '-a', '-F', '#{window_id}'))

    def test_session_navigator_and_cockpit_move_only_requester(self):
        other = self.selection(self.clients[1])
        self.ui('cockpit', 'worker')
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + self.worker))
        self.assertEqual(self.selection(self.clients[1]), other)
        self.tmux('new-session', '-d', '-s', 'elsewhere')
        self.ui('sessions', 'elsewhere')
        self.assertEqual(self.selection(self.clients[1]), other)
        self.assertEqual(self.selection(self.clients[0]).split(':')[0],
                         self.tmux('display-message', '-p', '-t', 'elsewhere', '#{session_id}'))
        self.ui('sessions', 'project')
        self.assertEqual(self.selection(self.clients[1]), other)

    def test_invalid_routes_preserve_both_clients_and_sessions(self):
        before = [self.selection(client) for client in self.clients]
        sessions = self.tmux('list-sessions', '-F', '#{session_id}')
        for args, client in [(['--window', self.worker], None),
                             (['--window', self.worker], '/missing/client'),
                             (['--window', '@999999'], self.clients[0]),
                             (['--window', '1'], self.clients[0]),
                             (['--session', '$999999'], self.clients[0])]:
            result = self.command('navigate', *args, client=client, check=False)
            self.assertNotEqual(result.returncode, 0, result)
            self.assertTrue(result.stderr.strip())
            self.assertEqual([self.selection(client) for client in self.clients], before)
            self.assertEqual(self.tmux('list-sessions', '-F', '#{session_id}'), sessions)
        self.tmux('kill-window', '-t', self.worker)
        replacement = self.tmux('new-window', '-d', '-P', '-F', '#{window_id}', '-t', 'project:1')
        result = self.command('navigate', '--window', self.worker, client=self.clients[0], check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual([self.selection(client) for client in self.clients], before)
        self.assertNotEqual(replacement, self.worker)

    def test_status_mouse_click_and_attention_shortcut_route_requester(self):
        self.tmux('set-option', '-g', '@drudwyn_watcher_pid', str(os.getpid()))
        self.tmux('set-environment', '-g', 'DRUDWYN_V2_BIN', str(BIN))
        self.tmux('run-shell', shlex.quote(str(ROOT / 'tmux-drudwyn.tmux')))
        self.tmux('set-option', '-g', 'mouse', 'on')
        # Deterministic columns, using the installed range-click binding.
        self.tmux('set-option', '-g', 'status', 'on')
        self.tmux('set-option', '-g', 'status-position', 'top')
        self.tmux('set-option', '-g', 'status-format[0]',
                  '#[range=user|' + self.worker + '] WORKER #[norange]')
        before = self.selection(self.clients[1])
        time.sleep(.2)
        os.write(self.client_fds[self.clients[0]], b'\x1b[<0;3;1M\x1b[<0;3;1m')
        for _ in range(100):
            if self.selection(self.clients[0]).endswith(':' + self.worker): break
            time.sleep(.02)
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + self.worker))
        self.assertEqual(self.selection(self.clients[1]), before)
        self.command('navigate', '--window', self.home, client=self.clients[0])
        self.tmux('set-option', '-w', '-t', self.worker, '@drudwyn_attention_since', '1')
        os.write(self.client_fds[self.clients[0]], b'\x02a')
        time.sleep(.3)
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + self.worker))
        self.assertEqual(self.selection(self.clients[1]), before)

    def test_installed_popup_routes_its_invoking_client(self):
        self.tmux('set-option', '-g', '@drudwyn_watcher_pid', str(os.getpid()))
        self.tmux('set-environment', '-g', 'DRUDWYN_V2_BIN', str(BIN))
        self.tmux('run-shell', shlex.quote(str(ROOT / 'tmux-drudwyn.tmux')))
        before = self.selection(self.clients[1])
        fd = self.client_fds[self.clients[0]]
        output = self.client_output[self.clients[0]]
        output.clear()
        os.write(fd, b'\x02w')
        for _ in range(200):
            if b'NAVIGATOR' in output: break
            time.sleep(.02)
        self.assertIn(b'NAVIGATOR', output)
        os.write(fd, b'/worker')
        time.sleep(.1)
        os.write(fd, b'\x1b')
        time.sleep(.1)
        os.write(fd, b'\r')
        for _ in range(100):
            if self.selection(self.clients[0]).endswith(':' + self.worker): break
            time.sleep(.02)
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + self.worker), repr(output[-300:]))
        self.assertEqual(self.selection(self.clients[1]), before)

    def test_user_grouped_session_survives_repeated_switches(self):
        user_session = self.tmux('new-session', '-d', '-P', '-F', '#{session_id}', '-s', 'my-view', '-t', 'project')
        elsewhere = self.tmux('new-session', '-d', '-P', '-F', '#{session_id}', '-s', 'elsewhere')
        before = self.selection(self.clients[1])
        for _ in range(3):
            self.command('navigate', '--window', self.worker, '--session', '$0', client=self.clients[0])
            view = self.selection(self.clients[0]).split(':')[0]
            self.command('navigate', '--session', elsewhere, client=self.clients[0])
            self.assertNotIn(view, self.tmux('list-sessions', '-F', '#{session_id}').splitlines())
            self.assertIn(user_session, self.tmux('list-sessions', '-F', '#{session_id}').splitlines())
            self.assertEqual(self.selection(self.clients[1]), before)
        self.assertEqual(subprocess.check_output(['git', '-C', str(self.repo), 'status', '--porcelain'], text=True), '')
        self.assertEqual(subprocess.check_output(['git', '-C', str(self.repo), 'branch', '--show-current'], text=True).strip(), 'main')

    def test_shell_in_linked_window_cannot_guess_between_clients(self):
        self.command('navigate', '--window', self.worker, client=self.clients[0])
        before = [self.selection(client) for client in self.clients]
        self.env['TMUX_PANE'] = self.tmux('display-message', '-p', '-t', self.worker, '#{pane_id}')
        result = self.command('navigate', '--window', self.home, check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('ambiguous', result.stderr)
        self.assertEqual([self.selection(client) for client in self.clients], before)

    def test_workspace_navigator_moves_only_requester(self):
        other = self.selection(self.clients[1])
        self.ui('navigator', 'worker')
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + self.worker))
        self.assertEqual(self.selection(self.clients[1]), other)

if __name__ == '__main__': unittest.main()
