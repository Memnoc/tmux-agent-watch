"""Status A through CLI, real terminal cells and attached-client actions."""
import fcntl
import struct
import termios
import unicodedata
from terminal_cells import TerminalCells
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import time
import unittest
from independent_navigation_test import IndependentNavigation, BIN, ROOT

STYLE = re.compile(r'(?<!#)#\[[^\]]*\]')
def plain(value):
    return STYLE.sub('', value).replace('##', '#')

class StatusATest(IndependentNavigation):
    def render(self, width=160, row='both'):
        session, window = self.selection(self.clients[0]).split(':')
        return self.command('status-bar', '--session', session, '--window', window, '--width', str(width), '--row', row, client=self.clients[0]).stdout

    def exited_worker(self, event, code, managed=False):
        # Gate exit until hook evidence has been attributed to this fake child.
        fake = Path(self.tmp.name) / 'fake-worker/codex'
        gate = Path(self.tmp.name) / ('exit-' + str(time.monotonic_ns()))
        script = (shlex.quote(str(fake)) + ' 300 & agent=$!; '
                  'while [ ! -e ' + shlex.quote(str(gate)) + ' ]; do sleep .02; done; '
                  'kill "$agent"; wait "$agent"; ' +
                  ('sleep 300' if code is None else f'sleep 1 & exit {code}'))
        if managed:
            root = Path(self.tmp.name) / 'worktrees $literal'
            result = self.command('workspace', 'start', '--repo', str(self.repo),
                                  '--base', 'main', '--worktree-root', str(root),
                                  'status-worker', 'sh', '-c', script, client=self.clients[0])
            self.worker = re.search(r'window (@[0-9]+)', result.stderr)[1]
            checkout = Path(result.stdout.strip())
            self.assertIn('$literal', str(checkout))
            self.assertEqual(self.tmux('show', '-wqv', '-t', self.worker,
                                      '@drudwyn_launch_checkout'), os.fsencode(checkout).hex())
        else:
            self.tmux('set', '-w', '-t', self.worker, 'remain-on-exit', 'on')
            self.tmux('respawn-pane', '-k', '-t', self.worker, '-c', str(self.repo), 'sh', '-c', script)
        deadline = time.monotonic() + 4
        while True:
            try:
                self.worker_hook(event)
                break
            except subprocess.CalledProcessError:
                self.assertLess(time.monotonic(), deadline)
                time.sleep(.02)
        gate.touch()
        while True:
            native = self.tmux('display-message', '-p', '-t', self.worker, '#{pane_dead_status}')
            status = self.command('status', client=self.clients[0]).stdout
            worker = next((row for row in status.splitlines() if row.startswith(self.worker+'\t')), '')
            if 'Exited' in worker and (code is None or native == str(code)):
                break
            self.assertLess(time.monotonic(), deadline, worker)
            time.sleep(.02)
        self.assertIn('exit code ' + ('unknown' if code is None else str(code)), worker)
        self.command('navigate', '--window', self.worker, client=self.clients[0])
        self.tmux('set', '-g', '@drudwyn-visible-tabs', '1')

    def test_correction_retained_attention_and_exit_in_actual_rows(self):
        self.install()
        for event, label, compact, code in [('stop', 'REVIEW', 'REV', 23),
                                           ('permissionRequest', 'INPUT', 'IN', 0),
                                           ('permissionRequest', 'INPUT', 'IN', None)]:
            self.exited_worker(event, code)
            exit_label = '?' if code is None else str(code)
            for width in [160, 48]:
                self.resize_client(self.clients[0], width)
                screen, data = self.converged(self.clients[0], width)
                top, bottom = screen.lines()[-2:]
                self.assertIn(label + ' / EXIT ' + exit_label, top)
                self.assertRegex(top, r'  '+label+r' / EXIT '+re.escape(exit_label)+r'  ')
                if width == 160:
                    self.assertIn(label+' (hook) / EXIT '+exit_label, bottom)
                else:
                    self.assertIn(compact+'/X'+exit_label, bottom)
                self.assertNotIn('DONE', top+bottom)
                self.assertIn('FAIL '+('1' if code == 23 else '0'), bottom)
                path = Path(f'/tmp/drudwyn-ticket10-correction-{label}-{exit_label.replace("?", "unknown")}-{width}')
                path.with_suffix('.txt').write_text(top+'\n'+bottom+'\n')
                path.with_suffix('.ansi').write_bytes(data)

    def test_correction_stopped_branch_uses_bound_checkout_or_unknown(self):
        ordinary = self.worker
        self.exited_worker('stop', 23, managed=True)
        self.install()
        self.resize_client(self.clients[0], 160)
        screen, _ = self.converged(self.clients[0], 160)
        self.assertIn('· status-worker ·', screen.lines()[-1])
        Path('/tmp/drudwyn-ticket10-correction-bound-checkout.txt').write_text('\n'.join(screen.lines()[-2:])+'\n')
        pane = self.tmux('display-message', '-p', '-t', self.worker, '#{pane_id}')
        # Loss of checkout evidence must not expose the renderer's own branch.
        for value in ['', 'not-hex', os.fsencode('relative').hex()]:
            self.tmux('set', '-w', '-t', self.worker, '@drudwyn_launch_checkout', value)
            screen, _ = self.converged(self.clients[0], 160)
            self.assertIn('· ref ? ·', screen.lines()[-1])
            self.assertNotIn('work/worktree-worker-workflow', screen.lines()[-1])
        self.assertEqual(self.tmux('display-message', '-p', '-t', pane, '#{pane_current_path}'), '')
        # An ordinary stopped agent has no authoritative checkout association.
        self.worker = ordinary
        self.exited_worker('stop', 23)
        screen, _ = self.converged(self.clients[0], 160)
        self.assertIn('· ref ? ·', screen.lines()[-1])
        self.assertNotIn('work/worktree-worker-workflow', screen.lines()[-1])

    def test_caps_all_local_windows_and_global_hidden_attention(self):
        for i in range(5):
            self.tmux('new-window', '-d', '-t', 'project', '-n', f'shell-{i}', '-c', '/tmp', 'bash', '--noprofile', '--norc')
        self.command('navigate', '--window', self.worker, client=self.clients[0])
        for cap, maximum in [('1', 1), ('3', 3), ('4', 4), ('6', 6), ('auto', 7), ('malformed', 4)]:
            self.tmux('set', '-g', '@drudwyn-visible-tabs', cap)
            rows = self.render().splitlines()
            self.assertEqual(len(rows), 2)
            targets = re.findall(r'range=user\|window:([^\]]+)', rows[0])
            self.assertLessEqual(len(targets), maximum)
            self.assertGreater(len(targets), 0)
            self.assertIn(self.worker, [target.split(':')[-1] for target in targets])
            self.assertIn(f'+{7-len(targets)}', plain(rows[0])) if len(targets) < 7 else None
            self.assertIn('NEED YOU', plain(rows[1]))
            self.assertIn('REVIEW 1', plain(rows[1]))
            self.assertIn('● ', plain(rows[0]))
        self.tmux('set', '-g', '@drudwyn-visible-tabs', '1')
        before = self.selection(self.clients[1])
        result = self.command('cockpit', '--list', '--windows', '--local-session', self.selection(self.clients[0]).split(':')[0], client=self.clients[0])
        self.assertIn('MATCHING 7 windows', result.stdout)
        self.assertIn('shell-4', result.stdout)
        self.assertEqual(self.selection(self.clients[1]), before)

    def install(self):
        self.tmux('set', '-g', '@drudwyn-status-layout', 'tabs')
        self.tmux('set-environment', '-g', 'DRUDWYN_V2_BIN', str(BIN))
        self.tmux('set', '-g', 'mouse', 'on')
        subprocess.run(['bash', str(ROOT / 'tmux-drudwyn.tmux')], env={**self.env, 'DRUDWYN_V2_BIN': str(BIN)}, capture_output=True, check=True)
        self.tmux('set', '-g', 'status-interval', '1')

    def test_install_two_rows_and_keyboard_action_table(self):
        self.install()
        top = self.tmux('show', '-gqv', 'status-format[0]')
        bottom = self.tmux('show', '-gqv', 'status-format[1]')
        self.assertIn('status-a.sh', top)
        self.assertIn('tabs', top)
        self.assertIn('context', bottom)
        self.assertNotIn('separator', top + bottom)
        self.assertIn('drudwyn-status', self.tmux('list-keys', '-T', 'prefix', 'g'))
        for key in ['f', 'i', 'r', 'a', 'w']:
            self.assertIn('status-action', self.tmux('list-keys', '-T', 'drudwyn-status', key))

    def test_plain_shell_context_omits_empty_activity(self):
        self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
        self.install()
        self.resize_client(self.clients[0], 160)
        screen, data = self.converged(self.clients[0], 160)
        bottom = screen.lines()[-1]
        self.assertIn('COORD', bottom)
        self.assertIn(' · main', bottom)
        self.assertNotIn('(unknown)', bottom)
        self.assertEqual(bottom.split('NEED YOU')[0].count(' · '), 1)
        Path('/tmp/drudwyn-ticket15-shell.txt').write_text(bottom + '\n')
        Path('/tmp/drudwyn-ticket15-shell.ansi').write_bytes(data)

    def test_long_shell_context_retains_selected_identity_at_all_widths(self):
        self.install()
        selected = self.selection(self.clients[0])
        other = self.selection(self.clients[1])
        index = self.tmux('display-message', '-p', '-t', self.home, '#{window_index}')
        for role, name in [('SH', 'editing shell with a deliberately long name'),
                           ('COORD', 'planning coordinator with a deliberately long name')]:
            if role == 'COORD':
                self.command('coordinator', 'set', '--window', self.home, client=self.clients[0])
            self.tmux('rename-window', '-t', self.home, name)
            for redacted in [False, True]:
                self.tmux('set', '-g', '@drudwyn-redact-labels', 'on' if redacted else 'off')
                for width in [48, 64, 80, 120, 160]:
                    with self.subTest(role=role, redacted=redacted, width=width):
                        session, window = selected.split(':')
                        rows = plain(self.command('status-bar', '--projection', '--session', session,
                                     '--window', window, '--width', str(width), client=self.clients[0]).stdout).splitlines()
                        top, bottom = rows
                        attention_label = 'ATTN' if width < 64 else 'NEED YOU'
                        context = bottom.split(attention_label)[0].strip()
                        self.assertIn(role, context)
                        if width >= 64:
                            self.assertIn('Workspace' if redacted else name.split()[0], context)
                        self.assertNotIn('(unknown)', context)
                        self.assertFalse(context.endswith('·'), context)
                        self.assertIn('● ' + index + ' ', top)
                        for count in [attention_label + ' 1', 'FAIL 0', 'INPUT 0', 'REVIEW 1']:
                            self.assertIn(count, bottom)
                        for row in rows:
                            self.assertLessEqual(len(row), width)
                            if redacted:
                                self.assertNotIn(name.split()[0], row)
                                self.assertNotIn('main', row)
                        self.resize_client(self.clients[0], width)
                        screen, data = self.converged(self.clients[0], width)
                        self.assertEqual([line.rstrip() for line in screen.lines()[-2:]],
                                         [line.rstrip() for line in rows])
                        path = Path(f'/tmp/drudwyn-ticket15-long-{role}-{redacted}-{width}')
                        path.with_suffix('.txt').write_text('\n'.join(screen.lines()[-2:]) + '\n')
                        path.with_suffix('.ansi').write_bytes(data)
                        self.assertEqual(self.selection(self.clients[0]), selected)
                        self.assertEqual(self.selection(self.clients[1]), other)

    def resize_client(self, client, width):
        fcntl.ioctl(self.client_fds[client], termios.TIOCSWINSZ, struct.pack('HHHH',40,width,0,0))
        deadline=time.monotonic()+3
        while dict(row.split('␟') for row in self.tmux('list-clients','-F','#{client_name}␟#{client_width}').splitlines()).get(client) != str(width):
            self.assertLess(time.monotonic(),deadline)
            time.sleep(.02)

    def terminal(self, client, width, expected='NEED YOU', rows=None):
        if not hasattr(self,'screens'):self.screens={}
        if client not in self.screens or self.screens[client][0].width != width:
            self.screens[client]=(TerminalCells(width),len(self.client_output[client]))
        deadline=time.monotonic()+12
        data=bytearray()
        while True:
            screen,offset=self.screens[client]
            self.tmux('refresh-client','-t',client)
            time.sleep(.12)
            chunk=bytes(self.client_output[client][offset:]);data.extend(chunk)
            screen.feed(chunk)
            self.screens[client]=(screen,offset+len(chunk))
            if rows is not None:
                ready=[line.rstrip() for line in screen.lines()[-2:]]==[line.rstrip() for line in rows]
            else:ready=expected in '\n'.join(screen.lines())
            if ready:return screen,bytes(data)
            self.assertLess(time.monotonic(),deadline,'Missing '+expected+': '+repr(screen.lines()[-2:]))

    def converged(self, client, width):
        session,window=self.selection(client).split(':')
        expected=plain(self.command('status-bar','--projection','--session',session,'--window',window,'--width',str(width),client=client).stdout).splitlines()
        return self.terminal(client,width,rows=expected)

    def test_real_status_cells_two_clients_widths_roles_padding_and_redaction(self):
        self.command('coordinator','set','--window',self.home,client=self.clients[0])
        self.tmux('rename-window','-t',self.worker,'identical 世界 café worker name with # literal')
        for i in range(5):
            self.tmux('new-window','-d','-t','project','-n','identical 世界 café shell name', '-c','/tmp','bash','--noprofile','--norc')
        self.command('navigate','--window',self.worker,client=self.clients[0])
        self.install()
        for mode in ['safe','nerd']:
            self.tmux('set','-g','@drudwyn-icon-mode',mode)
            for theme in ['moon','dawn','rose-pine']:
                self.tmux('set','-g','@drudwyn-theme',theme)
                for width in [48,64,80,120,160]:
                    self.resize_client(self.clients[0],width)
                    self.resize_client(self.clients[1],80 if width==160 else 160)
                    screen,data=self.converged(self.clients[0],width)
                    top,bottom=screen.lines()[-2:]
                    self.assertIn('● ',top)
                    self.assertIn('REVIEW',top)
                    self.assertIn('ATTN' if width < 64 else 'NEED YOU',bottom)
                    self.assertIn('FAIL 0',bottom)
                    self.assertIn('INPUT 0',bottom)
                    self.assertIn('REVIEW 1',bottom)
                    palette={'moon':(239,204,216,67),'dawn':(253,132,179,24),'rose-pine':(238,204,216,66)}[theme]
                    # Accumulated terminal SGR state, not strings in generated formats.
                    marker=next(i for i,c in enumerate(screen.grid[-2]) if c=='●')
                    badge=top.index('REVIEW')
                    badge_cell=sum(2 if unicodedata.east_asian_width(c) in ('W','F') else 0 if unicodedata.combining(c) else 1 for c in top[:badge])
                    for cell in screen.styles[-2][marker-1:badge_cell-2]:
                        self.assertEqual(cell[1],('index',palette[0]),(theme,width,cell))
                        self.assertTrue(cell[2])
                    for label,color in zip(['FAIL','INPUT','REVIEW'],palette[1:]):
                        cell=sum(2 if unicodedata.east_asian_width(c) in ('W','F') else 0 if unicodedata.combining(c) else 1 for c in bottom[:bottom.rindex(label)])
                        self.assertEqual(screen.styles[-1][cell][1],('index',color),(theme,width,label))
                    self.assertRegex(top,r'● \d+ AGENT ')
                    self.assertRegex(top,r'  REVIEW  ')
                    self.assertRegex(top,r'  \+\d+')
                    self.assertNotIn('COORD',top) if width<=64 else None
                    path=Path(f'/tmp/drudwyn-ticket10-{mode}-{theme}-{width}')
                    path.with_suffix('.txt').write_text(top+'\n'+bottom+'\n')
                    path.with_suffix('.ansi').write_bytes(data)
                    other,_=self.converged(self.clients[1],80 if width==160 else 160)
                    self.assertIn('COORD',other.lines()[-2])
                    self.assertNotEqual(top,other.lines()[-2])
        self.tmux('set','-g','@drudwyn-redact-labels','on')
        self.resize_client(self.clients[0],120)
        time.sleep(1.1)
        screen,_=self.terminal(self.clients[0],120)
        self.assertNotIn('identical','\n'.join(screen.lines()[-2:]))
        self.assertIn('Workspace','\n'.join(screen.lines()[-2:]))

    def key(self, client, data):
        os.write(self.client_fds[client],data)

    def status_key(self, client, key):
        self.key(client,b'\x02g')
        deadline=time.monotonic()+3
        while dict(row.split('␟') for row in self.tmux('list-clients','-F','#{client_name}␟#{client_key_table}').splitlines()).get(client) != 'drudwyn-status':
            self.assertLess(time.monotonic(),deadline,'Status key table was not selected')
            time.sleep(.02)
        self.key(client,key)

    def close_popup(self, client):
        self.key(client,b'q')
        deadline=time.monotonic()+3
        while True:
            screen,_=self.terminal(client,120)
            if 'WORKSPACE COCKPIT' not in '\n'.join(screen.lines()):break
            self.assertLess(time.monotonic(),deadline,'Popup stayed open')

    def click(self, client, x, y):
        self.key(client,f'\x1b[<0;{x+1};{y+1}M\x1b[<0;{x+1};{y+1}m'.encode())

    def test_real_mouse_keyboard_and_stable_local_membership_routes(self):
        outsider=self.tmux('new-window','-d','-P','-F','#{window_id}','-t','project','-n','outside-shell','-c','/tmp','bash','--noprofile','--norc')
        mixed=Path(self.tmp.name)/'other-repo'
        subprocess.run(['git','init','-q',str(mixed)],check=True)
        self.tmux('new-window','-d','-t','project','-n','mixed-repo','-c',str(mixed),'bash','--noprofile','--norc')
        self.tmux('new-session','-d','-s','other','-c','/tmp','bash','--noprofile','--norc')
        self.tmux('link-window','-s',self.worker,'-t','other:17')
        other_session=self.tmux('display-message','-p','-t','other','#{session_id}')
        # The same window has a different local index in a non-grouped session.
        output=self.command('status-bar','--session',other_session,'--window',self.worker,'--width','160',client=self.clients[0]).stdout
        self.assertIn('● 17 AGENT',plain(output))
        local=self.command('cockpit','--list','--windows','--local-session',other_session,client=self.clients[0]).stdout
        self.assertIn('MATCHING 2 windows',local)
        self.assertNotIn('outside-shell',local)
        self.tmux('set','-g','@drudwyn-visible-tabs','1')
        self.command('navigate','--window',self.worker,client=self.clients[0])
        self.install()
        before=[self.selection(c) for c in self.clients]
        screen,_=self.terminal(self.clients[0],120)
        top,bottom=screen.lines()[-2:]
        self.click(self.clients[0],118,38)
        self.assertEqual([self.selection(c) for c in self.clients],before)
        # These are two independent clicks, not tmux's DoubleClick event.
        time.sleep(.6)
        self.click(self.clients[0],bottom.rindex('REVIEW'),39)
        popup,_=self.terminal(self.clients[0],120,'state review')
        self.assertIn('MATCHING 1 workers','\n'.join(popup.lines()))
        self.assertEqual([self.selection(c) for c in self.clients],before)
        self.close_popup(self.clients[0])
        for key,label in [(b'f','failed'),(b'i','input'),(b'r','review'),(b'a','attention')]:
            self.status_key(self.clients[0],key)
            self.terminal(self.clients[0],120,'state '+label)
            self.close_popup(self.clients[0])
        # Overflow and its keyboard route reach every local window, with no Git association assumption.
        screen,_=self.terminal(self.clients[0],120)
        self.click(self.clients[0],screen.lines()[-2].index('+'),38)
        popup,_=self.terminal(self.clients[0],120,'MATCHING 4 windows')
        self.assertIn('outside-shell','\n'.join(popup.lines()))
        self.close_popup(self.clients[0])
        self.status_key(self.clients[0],b'w')
        self.terminal(self.clients[0],120,'MATCHING 4 windows')
        self.key(self.clients[0],b'p')
        self.terminal(self.clients[0],120,'project project')
        self.close_popup(self.clients[0])
        self.status_key(self.clients[0],b'w')
        self.terminal(self.clients[0],120,'MATCHING 4 windows')
        self.key(self.clients[0],b'/outside-shell\r')
        self.terminal(self.clients[0],120,'MATCHING 1 windows')
        self.key(self.clients[0],b'\r')
        deadline=time.monotonic()+3
        while not self.selection(self.clients[0]).endswith(':'+outsider):
            self.assertLess(time.monotonic(),deadline)
            time.sleep(.02)
        self.assertEqual(self.selection(self.clients[1]),before[1])
        self.tmux('set','-g','@drudwyn-visible-tabs','4')
        deadline=time.monotonic()+4
        while True:
            screen,_=self.terminal(self.clients[0],120)
            if 'worker' in screen.lines()[-2]:break
            self.assertLess(time.monotonic(),deadline)
        self.click(self.clients[0],screen.lines()[-2].index('worker'),38)
        deadline=time.monotonic()+3
        while not self.selection(self.clients[0]).endswith(':'+self.worker):
            self.assertLess(time.monotonic(),deadline)
            time.sleep(.02)
        self.assertEqual(self.selection(self.clients[1]),before[1])
        # A captured target cannot select an index replacement after its window disappears.
        session=self.selection(self.clients[0]).split(':')[0]
        target='window:'+session+':'+outsider
        self.tmux('kill-window','-t',outsider)
        replacement=self.tmux('new-window','-d','-P','-F','#{window_id}','-t','project','-n','outside-shell')
        before=[self.selection(c) for c in self.clients]
        result=self.command('status-action',target,client=self.clients[0],check=False)
        self.assertNotEqual(result.returncode,0)
        self.assertEqual([self.selection(c) for c in self.clients],before)
        self.assertNotEqual(replacement,outsider)

    def test_overlap_counts_evidence_and_bounded_git_probes(self):
        import shutil
        fake=Path(self.tmp.name)/'fake-worker/codex'
        self.assertEqual(fake.resolve(),Path(shutil.which('sleep')).resolve())
        self.tmux('set','-g','@drudwyn-visible-tabs','1')
        second=self.tmux('new-session','-d','-P','-F','#{pane_id}','-s','elsewhere','-c',str(self.repo),str(fake),'300')
        subprocess.run([str(BIN),'hook','codex','permissionRequest'],env={**self.env,'TMUX_PANE':second},check=True,capture_output=True)
        self.tmux('set','-w','-t',self.worker,'remain-on-exit','on')
        pane=self.tmux('display-message','-p','-t',self.worker,'#{pane_id}')
        self.tmux('respawn-pane','-k','-t',pane,'sh','-c',shlex.quote(str(fake))+' 1; sleep 1 & exit 23')
        deadline=time.monotonic()+2
        while True:
            hook=subprocess.run([str(BIN),'hook','codex','stop'],env={**self.env,'TMUX_PANE':pane},capture_output=True)
            if hook.returncode==0:break
            self.assertLess(time.monotonic(),deadline)
            time.sleep(.01)
        deadline=time.monotonic()+3
        while self.tmux('display-message','-p','-t',pane,'#{pane_dead_status}')!='23':
            self.assertLess(time.monotonic(),deadline)
            time.sleep(.02)
        cockpit=self.command('cockpit','--list',client=self.clients[0]).stdout
        self.assertIn('2 attention (1 failed / 1 input / 1 review; categories overlap)',cockpit)
        rendered=plain(self.render(160))
        self.assertIn('NEED YOU 2 (overlap)',rendered)
        for label in ['FAIL 1','INPUT 1','REVIEW 1']:self.assertIn(label,rendered)
        self.assertNotIn('worker',rendered.splitlines()[0])
        self.assertIn('ATTN 2*',plain(self.render(48)))
        # Instrument only the public renderer process: no Git details for tabs,
        # and at most the selected checkout's branch for context, never all repos.
        gate=Path(self.tmp.name)/'gitgate';gate.mkdir()
        log=gate/'calls';real=shutil.which('git')
        wrapper=gate/'git'
        wrapper.write_text('#!/bin/sh\nprintf "%s\\n" "$*" >> '+shlex.quote(str(log))+'\nexec '+shlex.quote(real)+' "$@"\n');wrapper.chmod(0o755)
        self.env['PATH']=str(gate)+':'+self.env['PATH']
        self.render(row='tabs')
        self.assertFalse(log.exists())
        self.render(row='context')
        self.assertEqual(len(log.read_text().splitlines()),1)
        self.assertIn('branch --show-current',log.read_text())
        # Pure process presence remains RUN; a valid hook establishes WORK.
        self.tmux('respawn-pane','-k','-t',pane,str(fake),'300')
        self.command('navigate','--window',self.worker,client=self.clients[0])
        self.assertIn('  RUN  ',plain(self.render()).splitlines()[0])
        self.worker_hook('userPromptSubmit')
        self.assertIn('  WORK  ',plain(self.render()).splitlines()[0])
        self.install()
        screen,data=self.terminal(self.clients[0],120)
        self.assertIn('WORK',screen.lines()[-2])
        Path('/tmp/drudwyn-ticket10-working.ansi').write_bytes(data)

    def test_actual_caps_settings_and_literal_format_names(self):
        for i in range(5):self.tmux('new-window','-d','-t','project','-n','sh'+str(i),'bash','--noprofile','--norc')
        self.command('navigate','--window',self.worker,client=self.clients[0])
        self.install()
        self.resize_client(self.clients[0],160)
        for cap in ['1','3','4','6','auto','malformed']:
            self.tmux('set','-g','@drudwyn-visible-tabs',cap)
            # Jobs are refreshed by tmux; wait for the new bounded row to match
            # the independently rendered public command for this same metadata.
            expected=plain(self.render(160,row='tabs')).rstrip()
            deadline=time.monotonic()+4
            while True:
                screen,data=self.terminal(self.clients[0],160)
                if screen.lines()[-2].rstrip()==expected:break
                self.assertLess(time.monotonic(),deadline,(cap,expected,screen.lines()[-2]))
            Path('/tmp/drudwyn-ticket10-cap-'+cap+'.txt').write_text('\n'.join(screen.lines()[-2:]))
        pane=self.tmux('new-window','-d','-P','-F','#{pane_id}','-t','project',str(BIN),'settings')
        self.wait_pane(pane,'OPTIONS')
        self.tmux('send-keys','-t',pane,'l',*(['j']*9),'Enter')
        screen=self.wait_pane(pane,'Applied Visible status tabs')
        self.assertIn('Visible status tabs',screen)
        self.assertIn(self.tmux('show','-gqv','@drudwyn-visible-tabs'),['1','3','4','6','auto'])
        self.tmux('kill-pane','-t',pane)
        self.tmux('set','-g','@drudwyn-visible-tabs','1')
        self.tmux('rename-window','-t',self.worker,'#[bg=red] literal')
        deadline=time.monotonic()+4
        while True:
            screen,data=self.terminal(self.clients[0],160)
            if '#[bg=red] literal' in screen.lines()[-2]:break
            self.assertLess(time.monotonic(),deadline,screen.lines()[-2])
        Path('/tmp/drudwyn-ticket10-literal-format.ansi').write_bytes(data)
        self.assertIn('REVIEW',screen.lines()[-2])

    def test_many_checkouts_two_client_refresh_and_stale_ranges(self):
        import shutil
        from concurrent.futures import ThreadPoolExecutor
        fake=Path(self.tmp.name)/'fake-worker/codex'
        self.assertEqual(fake.resolve(),Path(shutil.which('sleep')).resolve())
        for p in range(4):
            repo=self.repo if p==0 else Path(self.tmp.name)/f'repo{p}'
            session='project' if p==0 else f'project{p}'
            if p:
                subprocess.run(['git','clone','-q',str(self.repo),str(repo)],check=True)
                self.tmux('new-session','-d','-s',session,'-c',str(repo),'bash','--noprofile','--norc')
            coordinator=self.tmux('display-message','-p','-t',session,'#{window_id}')
            sid=self.tmux('display-message','-p','-t',session,'#{session_id}')
            self.command('coordinator','set','--window',coordinator,'--session',sid,client=self.clients[0])
            for n in range(8 if p==0 else 9):
                checkout=Path(self.tmp.name)/f'checkout{p}-{n}'
                subprocess.run(['git','-C',str(repo),'worktree','add','-qb',f'work/{p}-{n}',str(checkout)],check=True)
                worker=self.tmux('new-window','-d','-P','-F','#{window_id}','-t',session,'-n','same worker','-c',str(checkout),str(fake),'300')
                # These are known live checkout identities, as managed launches retain.
                self.tmux('set','-w','-t',worker,'@drudwyn_worktree',str(checkout))
                self.tmux('set','-w','-t',worker,'@drudwyn_repo',str(repo))
        self.command('navigate','--window',self.worker,client=self.clients[0])
        self.resize_client(self.clients[0],64)
        self.resize_client(self.clients[1],160)
        requests=[(self.clients[0],64),(self.clients[1],160)]
        def render_for(item):
            client,width=item
            session,window=self.selection(client).split(':')
            start=time.monotonic()
            out=self.command('status-bar','--session',session,'--window',window,'--width',str(width),client=client).stdout
            return time.monotonic()-start,out
        before=time.monotonic()
        with ThreadPoolExecutor(max_workers=2) as pool:results=list(pool.map(render_for,requests))
        print('36-worker/4-project/39-checkout two-client render wall %.3fs, individual %s' % (time.monotonic()-before,[round(r[0],3) for r in results]),flush=True)
        for _,output in results:self.assertIn('NEED YOU 1',plain(output))
        before=time.monotonic()
        self.install()
        first,_=self.converged(self.clients[0],64)
        second,_=self.converged(self.clients[1],160)
        print('36-worker two-client installed rows ready %.3fs' % (time.monotonic()-before),flush=True)
        self.assertIn('NEED YOU 1',first.lines()[-1])
        self.assertIn('NEED YOU 1',second.lines()[-1])
        self.assertIn('WT',second.lines()[-2])
        Path('/tmp/drudwyn-ticket10-scale-64.txt').write_text('\n'.join(first.lines()[-2:]))
        Path('/tmp/drudwyn-ticket10-scale-160.txt').write_text('\n'.join(second.lines()[-2:]))
        # Membership changes after the metadata snapshot fail closed instead of
        # advertising an inaccurate hidden count or reusing another row's index.
        gate=Path(self.tmp.name)/'race';gate.mkdir()
        real=shutil.which('tmux');wrapper=gate/'tmux'
        wrapper.write_text('#!/bin/sh\nif [ "$1" = list-windows ] && [ "$2" = -t ]; then touch '+shlex.quote(str(gate/'ready'))+'; while [ ! -e '+shlex.quote(str(gate/'go'))+' ]; do sleep .02; done; fi\nexec '+shlex.quote(real)+' "$@"\n');wrapper.chmod(0o755)
        session,window=self.selection(self.clients[0]).split(':')
        proc=subprocess.Popen([str(BIN),'status-bar','--session',session,'--window',window,'--width','64','--row','tabs'],env={**self.env,'PATH':str(gate)+':'+self.env['PATH']},stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        self.addCleanup(lambda:(gate/'go').touch())
        deadline=time.monotonic()+3
        while not (gate/'ready').exists():
            self.assertLess(time.monotonic(),deadline)
            time.sleep(.02)
        self.tmux('new-window','-d','-t',session,'-n','racing-shell','bash','--noprofile','--norc')
        (gate/'go').touch()
        stdout,stderr=proc.communicate(timeout=5)
        self.assertNotEqual(proc.returncode,0)
        self.assertEqual(stdout,'')
        self.assertIn('changed during render',stderr)

    def test_selected_last_shrink_stale_observer_and_custom_style(self):
        fake=Path(self.tmp.name)/'fake-worker/codex'
        for i in range(4):self.tmux('new-window','-d','-t','project','-n','same long shell name','bash','--noprofile','--norc')
        last=self.tmux('new-window','-d','-P','-F','#{window_id}','-t','project','-n','last 世界 worker with long identity','-c',str(self.repo),str(fake),'300')
        pane=self.tmux('display-message','-p','-t',last,'#{pane_id}')
        subprocess.run([str(BIN),'hook','codex','permissionRequest'],env={**self.env,'TMUX_PANE':pane},check=True,capture_output=True)
        self.command('navigate','--window',last,client=self.clients[0])
        self.install()
        index=self.tmux('display-message','-p','-t',last,'#{window_index}')
        for width in [160,48,160,64,48]:
            self.resize_client(self.clients[0],width)
            screen,data=self.converged(self.clients[0],width)
            self.assertIn('● '+index+' ',screen.lines()[-2])
            self.assertIn('INPUT',screen.lines()[-2])
            self.assertIn('INPUT 1',screen.lines()[-1])
            Path(f'/tmp/drudwyn-ticket10-selected-last-{width}.txt').write_text('\n'.join(screen.lines()[-2:]))
        self.resize_client(self.clients[0],120)
        self.tmux('set','-g','@drudwyn-icon-mode','nerd')
        self.tmux('set','-g','@drudwyn-codex-icon','λ')
        self.tmux('set','-g','@drudwyn-needs-input-color','#123456')
        screen,_=self.converged(self.clients[0],120)
        self.assertIn('λ',screen.lines()[-2])
        input_cell=screen.lines()[-1].index('INPUT')
        self.assertNotEqual(screen.styles[-1][input_cell][1],('index',216))
        self.tmux('set','-g','@drudwyn-icon-mode','safe')
        screen,_=self.converged(self.clients[0],120)
        self.assertNotIn('λ',screen.lines()[-2])
        # Pause only this fixture's observer; do not spawn scans from bar jobs.
        pid=int(self.tmux('show','-gqv','@drudwyn_watcher_pid'))
        self.assertIn(str(ROOT/'scripts/start-watcher.sh').encode(),Path(f'/proc/{pid}/cmdline').read_bytes())
        os.kill(pid,19)
        self.addCleanup(lambda:os.kill(pid,18) if Path(f'/proc/{pid}').exists() else None)
        time.sleep(.2)
        self.tmux('set','-g','@drudwyn_scan_at','1')
        screen,data=self.converged(self.clients[0],120)
        self.assertIn('NEED YOU STALE',screen.lines()[-1])
        self.assertIn('INPUT 1',screen.lines()[-1])
        self.assertEqual(self.tmux('show','-gqv','@drudwyn_scan_at'),'1')
        Path('/tmp/drudwyn-ticket10-stale.txt').write_text('\n'.join(screen.lines()[-2:]))
        Path('/tmp/drudwyn-ticket10-stale.ansi').write_bytes(data)
        for timestamp in ['', 'malformed', str(int(time.time())+3600)]:
            self.tmux('set','-g','@drudwyn_scan_at',timestamp)
            session,window=self.selection(self.clients[0]).split(':')
            output=self.command('status-bar','--projection','--session',session,'--window',window,'--width','120',client=self.clients[0]).stdout
            self.assertIn('NEED YOU STALE',plain(output),timestamp)
        import shutil
        gate=Path(self.tmp.name)/'failed-scan';gate.mkdir()
        wrapper=gate/'tmux';real=shutil.which('tmux')
        wrapper.write_text('#!/bin/sh\nif [ "$1" = set-option ] && [ "$2" = -wq ]; then exit 1; fi\nexec '+shlex.quote(real)+' "$@"\n');wrapper.chmod(0o755)
        self.tmux('set','-g','@drudwyn_scan_at','1')
        failed=subprocess.run([str(BIN),'scan'],env={**self.env,'PATH':str(gate)+':'+self.env['PATH']},capture_output=True)
        self.assertNotEqual(failed.returncode,0)
        self.assertEqual(self.tmux('show','-gqv','@drudwyn_scan_at'),'1')
        self.command('scan')
        screen,_=self.converged(self.clients[0],120)
        self.assertNotIn('STALE',screen.lines()[-1])

    def test_status_shortcut_preserves_existing_binding_when_reconfigured(self):
        self.tmux('bind-key','g','display-message','existing user binding')
        self.install()
        self.assertIn('existing user binding',self.tmux('list-keys','-T','prefix','g'))
        pane=self.tmux('new-window','-d','-P','-F','#{pane_id}','-t','project',str(BIN),'settings')
        self.wait_pane(pane,'OPTIONS')
        self.tmux('send-keys','-t',pane,'l','l','l',*(['j']*12),'e',*(['BSpace']*10))
        self.tmux('send-keys','-t',pane,'-l','F7')
        self.tmux('send-keys','-t',pane,'Enter')
        self.wait_pane(pane,'Applied Status actions')
        self.assertIn('existing user binding',self.tmux('list-keys','-T','prefix','g'))
        self.assertIn('drudwyn-status',self.tmux('list-keys','-T','prefix','F7'))

if __name__ == '__main__':
    names = [n for n in StatusATest.__dict__ if n.startswith('test_') and (len(sys.argv)==1 or sys.argv[1] in n)]
    result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(StatusATest(n) for n in names))
    sys.exit(not result.wasSuccessful())
