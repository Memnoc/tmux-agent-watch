"""Drudwyn routing with two real terminal clients, isolated Git and tmux."""
from pathlib import Path
import os
import fcntl
import struct
import termios
import pty
import shlex
import shutil
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
        # Keep the requested cwd stable while testing newly created shells.
        self.tmux('set-option', '-g', 'default-command', 'bash --noprofile --norc')
        self.addCleanup(lambda: self.tmux('kill-server', check=False))
        self.home = self.tmux('display-message', '-p', '-t', 'project', '#{window_id}')
        fake_bin = Path(self.tmp.name) / 'fake-worker'
        fake_bin.mkdir()
        fake_agent = fake_bin / 'codex'
        fake_agent.symlink_to(shutil.which('sleep'))
        self.assertEqual(fake_agent.resolve(), Path(shutil.which('sleep')).resolve())
        self.worker = self.tmux('new-window', '-d', '-P', '-F', '#{window_id}', '-t', 'project', '-n', 'worker', '-c', str(self.repo), str(fake_agent), '300')
        self.env = {**os.environ, 'TMUX': f'{self.socket},{self.tmux("display-message", "-p", "#{pid}")},0'}
        self.env.pop('TMUX_PANE', None)
        worker_pid = self.tmux('display-message', '-p', '-t', self.worker, '#{pane_pid}')
        executable = Path('/proc') / worker_pid / 'exe'
        if executable.exists():
            self.assertEqual(executable.resolve(), Path(shutil.which('sleep')).resolve())
        self.worker_hook('stop')
        self.client_output = {}
        self.client_fds = {}
        self.clients = [self.attach(), self.attach()]

    def test_coordinator_toggle_returns_to_worker_in_requesting_client(self):
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        session = self.tmux('display-message', '-p', '-t', self.home, '#{session_id}')
        self.tmux('set-option', '-w', '-t', self.worker, '@drudwyn_project', session)
        self.command('navigate', '--window', self.worker, client=self.clients[0])
        other = self.selection(self.clients[1])
        self.command('coordinator', 'toggle', client=self.clients[0])
        self.assertTrue(self.selection(self.clients[0]).endswith(':'+self.home))
        self.command('coordinator', 'toggle', client=self.clients[0])
        self.assertTrue(self.selection(self.clients[0]).endswith(':'+self.worker))
        self.assertEqual(self.selection(self.clients[1]), other)

    def worker_hook(self, event):
        pane = self.tmux('display-message', '-p', '-t', self.worker, '#{pane_id}')
        subprocess.run([str(BIN), 'hook', 'codex', event], env={**self.env, 'TMUX_PANE': pane}, text=True, capture_output=True, check=True)

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

    def ui(self, surface, query, action="Enter", expected=None):
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
        if expected:
            if surface == 'cockpit':
                self.tmux('send-keys', '-t', pane, 'd')
                self.wait_pane(pane, 'DETAILS')
            self.assertIn(expected, self.wait_pane(pane, expected))
            if surface == 'cockpit':
                self.tmux('send-keys', '-t', pane, 'd')
                self.wait_pane(pane, 'MATCHING')
        self.tmux('send-keys', '-t', pane, action)
        time.sleep(.3)

    def wait_pane(self, pane, text):
        previous = None
        for _ in range(250):
            output = self.tmux('capture-pane', '-p', '-t', pane)
            if text in output and output == previous:
                return output
            previous = output
            time.sleep(.02)
        self.fail(f'no {text!r}: {output}')

    def session_form(self, surface="sessions"):
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project',
                         shlex.join(['env', f'DRUDWYN_CLIENT={self.clients[0]}', str(BIN), surface]))
        self.wait_pane(pane, 'NAVIGATOR')
        self.tmux('send-keys', '-t', pane, 'n')
        self.wait_pane(pane, 'NEW SESSION')
        return pane

    def test_workspace_new_shell_cancel_and_create_preserves_other_client(self):
        requester=self.selection(self.clients[0]); other=self.selection(self.clients[1])
        pane=self.session_form('navigator')
        self.tmux('send-keys','-t',pane,'Escape')
        self.wait_pane(pane,'WORKSPACE NAVIGATOR')
        self.assertEqual(self.selection(self.clients[0]),requester)
        self.tmux('send-keys','-t',pane,'n')
        self.wait_pane(pane,'NEW SESSION')
        self.tmux('send-keys','-t',pane,'-l','workspace shell')
        self.tmux('send-keys','-t',pane,'Tab','Enter')
        for _ in range(250):
            if self.selection(self.clients[0]) != requester:break
            time.sleep(.02)
        session=self.selection(self.clients[0]).split(':')[0]
        self.assertEqual(self.tmux('display-message','-p','-t',session,'#{session_name}'),'workspace shell')
        self.assertEqual(self.selection(self.clients[1]),other)

    def test_new_session_form_cancel_and_create(self):
        other = self.selection(self.clients[1])
        requester = self.selection(self.clients[0])
        sessions = self.tmux('list-sessions', '-F', '#{session_id}')
        pane = self.session_form()
        self.assertIn(str(self.repo), self.tmux('capture-pane', '-p', '-t', pane))
        self.tmux('send-keys', '-t', pane, '-l', 'cancelled session')
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'SESSION NAVIGATOR')
        self.assertEqual(self.tmux('list-sessions', '-F', '#{session_id}'), sessions)
        self.assertEqual(self.selection(self.clients[0]), requester)
        self.tmux('send-keys', '-t', pane, 'n')
        self.wait_pane(pane, 'NEW SESSION')
        self.tmux('send-keys', '-t', pane, '-l', 'form shell')
        self.tmux('send-keys', '-t', pane, 'Tab', 'Enter')
        for _ in range(250):
            if self.selection(self.clients[0]) != requester:
                break
            time.sleep(.02)
        session = self.selection(self.clients[0]).split(':')[0]
        self.assertEqual(self.tmux('display-message', '-p', '-t', session, '#{session_name}'), 'form shell')
        self.assertEqual(self.tmux('display-message', '-p', '-t', session, '#{pane_current_path}'), str(self.repo))
        self.assertEqual(self.selection(self.clients[1]), other)
        # The resulting named shell appears in both navigation surfaces.
        for surface in ['sessions', 'navigator']:
            ui = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project',
                           shlex.join(['env', f'DRUDWYN_CLIENT={self.clients[0]}', str(BIN), surface]))
            self.wait_pane(ui, 'form shell')
            self.tmux('send-keys', '-t', ui, 'Escape')

    def test_new_session_validates_before_mutation_and_allows_retry(self):
        original = self.tmux('list-sessions', '-F', '#{session_id}␟#{session_name}')
        selections = [self.selection(c) for c in self.clients]
        cases = [(['--name', 'project'], self.clients[0]),
                 (['--name', ''], self.clients[0]),
                 (['--name', 'invalid.name'], self.clients[0]),
                 (['--name', 'invalid:name'], self.clients[0]),
                 (['--name', 'invalid\nname'], self.clients[0]),
                 (['--name', 'new', '--directory', str(self.repo / 'missing')], self.clients[0]),
                 (['--name', 'new', '--directory', str(self.repo / '.git/HEAD')], self.clients[0]),
                 (['--name', 'new'], '/dev/missing-client'),
                 (['--name', 'new'], None)]
        for args, client in cases:
            with self.subTest(args=args, client=client):
                result = self.command('session', 'new', *args, client=client, check=False)
                self.assertNotEqual(result.returncode, 0)
                self.assertTrue(result.stderr.strip())
                self.assertEqual(self.tmux('list-sessions', '-F', '#{session_id}␟#{session_name}'), original)
                self.assertEqual([self.selection(c) for c in self.clients], selections)
        self.command('session', 'new', '--name', 'corrected', client=self.clients[0])
        self.assertEqual(self.selection(self.clients[1]), selections[1])

    def test_new_session_names_and_paths_are_literal_data(self):
        other = self.selection(self.clients[1])
        directory = Path(self.tmp.name) / 'space $(touch SENTINEL) `touch SENTINEL` #{session_name};'
        directory.mkdir()
        for name in ['notes $(touch SENTINEL) `touch SENTINEL`', '-notes', '#{session_name}', 'semi;', ';']:
            with self.subTest(name=name):
                result = self.command('session', 'new', '--name=' + name,
                                      '--directory', str(directory), client=self.clients[0], check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
                session = result.stdout.strip()
                self.assertEqual(self.tmux('display-message', '-p', '-t', session, '#{session_name}'), name)
                self.assertEqual(self.tmux('display-message', '-p', '-t', session, '#{pane_current_path}'), str(directory))
                self.assertEqual(self.selection(self.clients[1]), other)
                self.assertFalse((directory / 'SENTINEL').exists())
                self.assertFalse((self.repo / 'SENTINEL').exists())

    def test_new_session_ignores_default_command_and_survives_inherited_cleanup(self):
        self.tmux('set-option', '-g', 'destroy-unattached', 'on')
        marker = Path(self.tmp.name) / 'agent-started'
        self.tmux('set-option', '-g', 'default-command', 'touch ' + shlex.quote(str(marker)))
        other = self.selection(self.clients[1])
        result = self.command('session', 'new', '--name', 'ad hoc', client=self.clients[0], check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(self.selection(self.clients[0]).startswith(result.stdout.strip() + ':'))
        self.assertEqual(self.selection(self.clients[1]), other)
        self.assertFalse(marker.exists())
        self.assertEqual(self.tmux('show-option', '-gv', 'destroy-unattached'), 'on')
        self.assertEqual(self.tmux('show-option', '-gv', 'default-command'), 'touch ' + shlex.quote(str(marker)))

    def test_new_session_backslash_names_preserve_literal_input_and_cleanup_policy(self):
        other = self.selection(self.clients[1])
        windows = self.tmux('list-windows', '-a', '-F', '#{window_id}␟#{pane_id}␟#{pane_pid}')
        directory = Path(self.tmp.name) / 'literal\\;'
        directory.mkdir()
        for cleanup in ['off', 'on']:
            self.tmux('set-option', '-g', 'destroy-unattached', cleanup)
            # These are tmux's native stored spellings, also produced by
            # `tmux new-session -s` with the literal input names below.
            for name, stored_name in [(r'slash\word', r'slash\\word'),
                                      ('slash\\word;', 'slash\\\\word;')]:
                with self.subTest(cleanup=cleanup, name=name):
                    before = set(self.tmux('list-sessions', '-F', '#{session_id}').splitlines())
                    result = self.command('session', 'new', '--name', name,
                                          '--directory', str(directory), client=self.clients[0], check=False)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    session = result.stdout.strip()
                    self.assertEqual(set(self.tmux('list-sessions', '-F', '#{session_id}').splitlines()),
                                     before | {session})
                    self.assertEqual(self.tmux('display-message', '-p', '-t', session, '#{session_name}'), stored_name)
                    self.assertEqual(self.tmux('display-message', '-p', '-t', session, '#{pane_current_path}'), str(directory))
                    self.assertTrue(self.selection(self.clients[0]).startswith(session + ':'))
                    self.assertEqual(self.selection(self.clients[1]), other)
                    self.assertEqual(self.tmux('show-option', '-gv', 'destroy-unattached'), cleanup)
                    self.assertEqual(self.tmux('show-option', '-v', '-t', session, 'destroy-unattached'), '')
                    duplicate = self.command('session', 'new', '--name', name,
                                             client=self.clients[0], check=False)
                    self.assertNotEqual(duplicate.returncode, 0)
                    self.assertIn('duplicate session', duplicate.stderr)
                    self.assertEqual(set(self.tmux('list-sessions', '-F', '#{session_id}').splitlines()),
                                     before | {session})
                    self.assertTrue(self.selection(self.clients[0]).startswith(session + ':'))
                    self.assertEqual(self.selection(self.clients[1]), other)
                    for window in windows.splitlines():
                        self.assertIn(window, self.tmux('list-windows', '-a', '-F', '#{window_id}␟#{pane_id}␟#{pane_pid}'))
                    self.tmux('switch-client', '-c', self.clients[0], '-t', 'project')
                    self.tmux('kill-session', '-t', session, check=False)

    def test_new_session_form_corrects_duplicate_name_and_bad_directory(self):
        pane = self.session_form()
        original = self.tmux('list-sessions', '-F', '#{session_id}')
        selections = [self.selection(c) for c in self.clients]
        self.tmux('send-keys', '-t', pane, '-l', 'project')
        self.tmux('send-keys', '-t', pane, 'Tab', 'Enter')
        self.wait_pane(pane, 'Create failed')
        self.assertEqual(self.tmux('list-sessions', '-F', '#{session_id}'), original)
        self.tmux('send-keys', '-t', pane, 'Tab', *(['BSpace'] * len('project')))
        self.tmux('send-keys', '-t', pane, '-l', 'corrected shell')
        self.tmux('send-keys', '-t', pane, 'Tab', *(['BSpace'] * len(str(self.repo))))
        self.tmux('send-keys', '-t', pane, '-l', '/missing-directory')
        self.tmux('send-keys', '-t', pane, 'Enter')
        self.wait_pane(pane, 'Invalid starting directory')
        self.assertEqual([self.selection(c) for c in self.clients], selections)
        self.assertEqual(self.tmux('list-sessions', '-F', '#{session_id}'), original)
        directory = Path(self.tmp.name) / 'directory with spaces; data'
        directory.mkdir()
        self.tmux('send-keys', '-t', pane, *(['BSpace'] * len('/missing-directory')))
        self.tmux('send-keys', '-t', pane, '-l', str(directory))
        self.tmux('send-keys', '-t', pane, 'Enter')
        for _ in range(250):
            if self.selection(self.clients[0]) != selections[0]: break
            time.sleep(.02)
        session = self.selection(self.clients[0]).split(':')[0]
        self.assertEqual(self.tmux('display-message', '-p', '-t', session, '#{session_name}'), 'corrected shell')
        self.assertEqual(self.tmux('display-message', '-p', '-t', session, '#{pane_current_path}'), str(directory))
        self.assertEqual(self.selection(self.clients[1]), selections[1])

    def test_new_session_form_redacts_labels_and_errors(self):
        self.tmux('set-option', '-g', '@drudwyn-redact-labels', 'on')
        pane = self.session_form()
        self.tmux('send-keys', '-t', pane, '-l', 'project')
        self.tmux('send-keys', '-t', pane, 'Tab', 'Enter')
        self.wait_pane(pane, 'Action failed')
        for width in [120, 48]:
            self.tmux('resize-window', '-t', pane, '-x', str(width), '-y', '24')
            output = self.wait_pane(pane, '[Esc] Cancel')
            self.assertIn('[redacted]', output)
            self.assertNotIn(str(self.repo), output)
            self.assertNotIn('project', output)
            self.assertIn('Name', output)
            self.assertIn('Directory', output)
            self.assertIn('[Esc] Cancel', output)
        self.tmux('send-keys', '-t', pane, 'Escape')

    def test_new_session_command_failures_preserve_existing_sessions_and_retry(self):
        original = self.tmux('list-sessions', '-F', '#{session_id}␟#{session_name}')
        windows = self.tmux('list-windows', '-a', '-F', '#{window_id}␟#{pane_id}␟#{pane_pid}')
        selections = [self.selection(c) for c in self.clients]
        wrapper_dir = Path(self.tmp.name) / 'bin'
        wrapper_dir.mkdir()
        wrapper = wrapper_dir / 'tmux'
        wrapper.write_text('#!' + sys.executable + '\n'
                           'import os, sys\n'
                           'if sys.argv[1] == os.environ.get("TEST_FAIL_COMMAND"):\n'
                           '    sys.stderr.write("injected command failure\\n")\n'
                           '    sys.exit(1)\n'
                           'os.execv(os.environ["TEST_REAL_TMUX"], ["tmux", *sys.argv[1:]])\n')
        wrapper.chmod(0o755)
        self.env['TEST_REAL_TMUX'] = shutil.which('tmux')
        self.env['PATH'] = str(wrapper_dir) + os.pathsep + self.env['PATH']
        for command in ['new-session', 'switch-client']:
            self.env['TEST_FAIL_COMMAND'] = command
            result = self.command('session', 'new', '--name', 'retry', client=self.clients[0], check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('injected command failure', result.stderr)
            self.assertEqual(self.tmux('list-sessions', '-F', '#{session_id}␟#{session_name}'), original)
            self.assertEqual(self.tmux('list-windows', '-a', '-F', '#{window_id}␟#{pane_id}␟#{pane_pid}'), windows)
            self.assertEqual([self.selection(c) for c in self.clients], selections)
        self.env.pop('TEST_FAIL_COMMAND')
        self.command('--client', self.clients[0], 'session', 'new', '--name', 'retry')
        self.assertEqual(self.selection(self.clients[1]), selections[1])

    def test_new_session_uncertain_creation_keeps_existing_sessions(self):
        windows = self.tmux('list-windows', '-a', '-F', '#{window_id}␟#{pane_id}␟#{pane_pid}').splitlines()
        selections = [self.selection(c) for c in self.clients]
        wrapper_dir = Path(self.tmp.name) / 'bin'
        wrapper_dir.mkdir()
        wrapper = wrapper_dir / 'tmux'
        real_tmux = shutil.which('tmux')
        wrapper.write_text('#!' + sys.executable + '\n'
                           'import os, subprocess, sys\n'
                           'if sys.argv[1] == "new-session":\n'
                           '    sys.exit(subprocess.run([os.environ["TEST_REAL_TMUX"], *sys.argv[1:]], stdout=subprocess.DEVNULL).returncode)\n'
                           'os.execv(os.environ["TEST_REAL_TMUX"], ["tmux", *sys.argv[1:]])\n')
        wrapper.chmod(0o755)
        self.env['TEST_REAL_TMUX'] = real_tmux
        self.env['PATH'] = str(wrapper_dir) + os.pathsep + self.env['PATH']
        result = self.command('session', 'new', '--name', 'inspect retained', client=self.clients[0], check=False)
        self.assertNotEqual(result.returncode, 0)
        for window in windows:
            self.assertIn(window, self.tmux('list-windows', '-a', '-F', '#{window_id}␟#{pane_id}␟#{pane_pid}').splitlines())
        self.assertEqual([self.selection(c) for c in self.clients], selections)
        self.assertIn('inspect retained', self.tmux('list-sessions', '-F', '#{session_name}'))
        self.assertIn('inspect', result.stderr.lower())

    def test_new_session_without_attached_client_creates_nothing(self):
        sessions = self.tmux('list-sessions', '-F', '#{session_id}')
        for client in self.clients:
            self.tmux('detach-client', '-t', client)
        result = self.command('session', 'new', '--name', 'unattached', check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('missing or detached', result.stderr)
        self.assertEqual(self.tmux('list-sessions', '-F', '#{session_id}'), sessions)

    def test_session_management_shortcuts_remain_visible_at_narrow_width(self):
        pane = self.tmux('new-window', '-d', '-P', '-F', '#{pane_id}', '-t', 'project',
                         shlex.join(['env', f'DRUDWYN_CLIENT={self.clients[0]}', str(BIN), 'sessions']))
        self.wait_pane(pane, 'NAVIGATOR')
        self.tmux('resize-window', '-t', pane, '-x', '48', '-y', '24')
        for shortcut in ['[n] New Session', '[x] Kill', '[s] Save', 'Filter', 'Close']:
            self.wait_pane(pane, shortcut)
        self.tmux('send-keys', '-t', pane, 'Escape')

    def test_new_session_default_directory_preserves_literal_suffix(self):
        directory = Path(self.tmp.name) / 'directory ␟'
        directory.mkdir()
        self.command('session', 'new', '--name', 'literal directory', '--directory', str(directory), client=self.clients[0])
        result = self.command('session', 'new', '--name', 'same directory', client=self.clients[0], check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.tmux('display-message', '-p', '-t', result.stdout.strip(), '#{pane_current_path}'), str(directory))

    def test_new_session_creates_only_shell_and_switches_requester(self):
        other = self.selection(self.clients[1])
        windows = self.tmux('list-windows', '-a', '-F', '#{window_id}␟#{pane_id}␟#{pane_pid}')
        worktrees = subprocess.check_output(['git', '-C', str(self.repo), 'worktree', 'list', '--porcelain'])
        result = self.command('session', 'new', '--name', 'editing room',
                              client=self.clients[0], check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        session = result.stdout.strip()
        self.assertTrue(self.selection(self.clients[0]).startswith(session + ':'))
        self.assertEqual(self.selection(self.clients[1]), other)
        self.assertEqual(self.tmux('display-message', '-p', '-t', session, '#{session_name}'), 'editing room')
        self.assertEqual(self.tmux('display-message', '-p', '-t', session, '#{pane_current_path}'), str(self.repo))
        self.assertEqual(self.tmux('display-message', '-p', '-t', session, '#{session_windows}'), '1')
        self.assertIn(self.tmux('display-message', '-p', '-t', session, '#{pane_current_command}'), ['zsh', 'bash', 'sh', 'fish'])
        for window in windows.splitlines():
            self.assertIn(window, self.tmux('list-windows', '-a', '-F', '#{window_id}␟#{pane_id}␟#{pane_pid}'))
        self.assertEqual(subprocess.check_output(['git', '-C', str(self.repo), 'worktree', 'list', '--porcelain']), worktrees)

    def test_coordinator_association_and_return_preserve_identity(self):
        other = self.selection(self.clients[1])
        result = self.command('coordinator', 'set', '--window', self.home,
                              client=self.clients[0], check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.command('navigate', '--window', self.worker, client=self.clients[0])
        self.command('coordinator', 'open', client=self.clients[0])
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + self.home))
        self.assertEqual(self.selection(self.clients[1]), other)
        self.assertIn('Coordinator shell', self.command('status').stdout)
        self.assertEqual(subprocess.check_output(['git', '-C', str(self.repo), 'branch',
                                                 '--show-current'], text=True).strip(), 'main')

    def test_three_named_workers_preserve_coordinator_and_user_names(self):
        self.tmux('set-option', '-g', 'allow-rename', 'on')
        self.tmux('set-option', '-g', 'automatic-rename', 'on')
        self.tmux('rename-window', '-t', self.home, 'planning')
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        self.tmux('send-keys', '-t', self.home, r"printf '\033kzsh\033\\'", 'Enter')
        time.sleep(.1)
        other = self.selection(self.clients[1])
        for number in range(3):
            name = 'worker-' + str(number)
            result = self.command('workspace', 'start', '--repo', str(self.repo),
                                  '--name', name, 'work/' + name, '--', 'sh', '-c',
                                  r"printf '\033kzsh\033\\'; exec sleep 300",
                                  client=self.clients[0], check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            windows = self.tmux('list-windows', '-t', 'project', '-F', '#{window_name}␟#{window_id}')
            worker = dict(row.split('␟') for row in windows.splitlines())[name]
            self.command('navigate', '--window', worker, client=self.clients[0])
            self.assertIn('Worktree worker', self.command('status').stdout)
            self.tmux('rename-window', '-t', worker, 'my-' + name)
            self.command('scan')
            self.assertEqual(self.tmux('display-message', '-p', '-t', worker, '#{window_name}'), 'my-' + name)
            self.assertEqual(self.tmux('show-options', '-wqv', '-t', worker, 'allow-rename'), 'off')
            self.command('coordinator', 'open', client=self.clients[0])
            self.assertTrue(self.selection(self.clients[0]).endswith(':' + self.home))
        self.assertEqual(self.selection(self.clients[1]), other)
        self.assertEqual(self.tmux('display-message', '-p', '-t', self.home, '#{window_name}'), 'planning')
        self.assertEqual(subprocess.check_output(['git', '-C', str(self.repo), 'branch', '--show-current'], text=True).strip(), 'main')
        self.tmux('kill-window', '-t', self.home)
        before = self.selection(self.clients[0])
        result = self.command('coordinator', 'open', client=self.clients[0], check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Coordinator unavailable', result.stderr)
        self.assertIn('recover', result.stderr)
        self.assertEqual(self.selection(self.clients[0]), before)
        replacement = self.tmux('new-window', '-d', '-P', '-F', '#{window_id}', '-t', 'project:0', '-n', 'planning', '-c', str(self.repo))
        result = self.command('coordinator', 'open', client=self.clients[0], check=False)
        self.assertIn('Coordinator unavailable', result.stderr)
        self.ui('cockpit', 'my-worker-0', 'c', 'unavailable')
        self.assertNotEqual(replacement, self.home)
        self.command('coordinator', 'set', '--window', replacement, client=self.clients[0])
        self.command('coordinator', 'open', client=self.clients[0])
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + replacement))

    def test_coordinator_return_from_navigators_and_cockpit(self):
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        result = self.command('workspace', 'start', '--repo', str(self.repo), 'work/example',
                              '--', 'sleep', '300', client=self.clients[0])
        worker = self.tmux('list-windows', '-t', 'project', '-f', '#{==:#{window_name},work/example}', '-F', '#{window_id}')
        other = self.selection(self.clients[1])
        for surface, query, expected in [('navigator', 'work/example', 'Worktree worker'),
                                         ('cockpit', 'work/example', 'Worktree worker'),
                                         ('sessions', 'project', 'Coordinator')]:
            self.command('navigate', '--window', worker, client=self.clients[0])
            self.ui(surface, query, 'c', expected)
            self.assertTrue(self.selection(self.clients[0]).endswith(':' + self.home), surface)
            self.assertEqual(self.selection(self.clients[1]), other)
        self.ui('cockpit', self.tmux('display-message', '-p', '-t', self.home, '#{window_name}'),
                expected='Coordinator shell')
        self.worker_hook('userPromptSubmit')
        self.assertIn('WORKING 1', self.command('hud', 'fleet', 'project', self.home).stdout)

    def test_same_named_projects_redaction_and_external_agents(self):
        # Same basename and display labels cannot establish a project association.
        twin = Path(self.tmp.name) / 'another' / 'repo'
        subprocess.run(['git', 'init', '-q', '-b', 'main', str(twin)], check=True)
        subprocess.run(['git', '-C', str(twin), '-c', 'user.name=Test', '-c',
                        'user.email=test@example.invalid', 'commit', '-qm', 'initial', '--allow-empty'], check=True)
        session = self.tmux('new-session', '-d', '-P', '-F', '#{session_id}', '-s', 'twin', '-c', str(twin))
        twin_home = self.tmux('display-message', '-p', '-t', session, '#{window_id}')
        for window in [self.home, twin_home]: self.tmux('rename-window', '-t', window, 'same-name')
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        self.command('coordinator', 'set', '--window', twin_home, '--session', session)
        self.tmux('set-option', '-g', '@drudwyn-redact-labels', 'on')
        status = self.command('status').stdout
        self.assertNotIn('same-name', status)
        self.assertNotIn(str(self.repo), status)
        self.assertIn(self.home + '\t', status)
        self.assertIn(twin_home + '\t', status)
        other = self.selection(self.clients[1])
        self.command('coordinator', 'open', '--session', session, client=self.clients[0])
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + twin_home))
        self.command('coordinator', 'open', '--session', '$0', client=self.clients[0])
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + self.home))
        self.assertEqual(self.selection(self.clients[1]), other)
        self.ui('cockpit', 'worker', 'c', 'unknown')
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + self.home))
        self.ui('navigator', 'same-name', expected='Coordinator shell')
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + self.home))

    def test_coordinator_agent_and_linked_checkout_association(self):
        agent = Path(self.tmp.name) / 'codex'
        agent.symlink_to('/bin/sleep')
        self.tmux('respawn-pane', '-k', '-t', self.home, str(agent), '300')
        time.sleep(.1)
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        self.assertIn('Coordinator agent', self.command('status').stdout)
        self.command('workspace', 'start', '--repo', str(self.repo), 'work/first', '--',
                     'sleep', '300', client=self.clients[0])
        linked = self.repo.parent / 'repo-worktrees' / 'work-first'
        worker = self.tmux('list-windows', '-t', 'project', '-f', '#{==:#{window_name},work/first}', '-F', '#{window_id}')
        self.command('navigate', '--window', worker, client=self.clients[0])
        self.command('workspace', 'start', '--repo', str(linked), 'work/second', '--',
                     'sleep', '300', client=self.clients[0])
        self.ui('cockpit', 'work/second', 'c', 'c return')
        self.assertTrue(self.selection(self.clients[0]).endswith(':' + self.home))

    def test_coordinator_moved_out_of_project_is_unavailable(self):
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        self.command('workspace', 'start', '--repo', str(self.repo), 'work/lost', '--',
                     'sleep', '300', client=self.clients[0])
        self.tmux('new-session', '-d', '-s', 'elsewhere')
        self.tmux('move-window', '-s', self.home, '-t', 'elsewhere:')
        self.ui('cockpit', 'work/lost', 'c', 'unavailable')

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

    def test_global_destroy_unattached_allows_navigation_and_view_detach_cleanup(self):
        other = self.selection(self.clients[1])
        panes = set(self.tmux('list-panes', '-a', '-F', '#{pane_id} #{pane_pid}').splitlines())
        self.tmux('set-option', '-g', 'destroy-unattached', 'on')
        result = self.command('navigate', '--client', self.clients[0], '--window', self.worker, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        view = self.selection(self.clients[0]).split(':')[0]
        self.assertNotEqual(view, other.split(':')[0])
        self.assertEqual(self.selection(self.clients[0]), view + ':' + self.worker)
        self.assertEqual(self.selection(self.clients[1]), other)
        self.assertEqual(self.tmux('show-options', '-gv', 'destroy-unattached'), 'on')
        self.command('navigate', '--client', self.clients[0], '--window', self.home)
        self.assertEqual(self.selection(self.clients[0]), view + ':' + self.home)
        self.tmux('detach-client', '-t', self.clients[0])
        self.assertNotIn(view, self.tmux('list-sessions', '-F', '#{session_id}').splitlines())
        self.assertEqual(self.selection(self.clients[1]), other)
        self.assertIn(self.worker, self.tmux('list-windows', '-a', '-F', '#{window_id}'))
        self.assertEqual(set(self.tmux('list-panes', '-a', '-F', '#{pane_id} #{pane_pid}').splitlines()), panes)
        self.assertEqual(self.tmux('show-options', '-gv', 'destroy-unattached'), 'on')

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
