"""Pending startup tasks stay bound to their worker, folder, and helper lifetime."""
import os
from pathlib import Path
import shlex
import signal
import time
import unittest
from worker_launch_test import WorkerLaunchTest

class StartupWaitTest(WorkerLaunchTest):
    def pending(self, change_directory=False):
        agent, script = self.raw_receiver()
        script.write_text('''import os,tty,time
from pathlib import Path
tty.setraw(0)
os.write(1,b'\\x1b[?25lSETUP\\r\\n')
while not Path(''' + repr(str(self.root/'open-editor')) + ''').exists(): time.sleep(.02)
''' + ("os.chdir(" + repr(str(self.root)) + ")\n" if change_directory else '') + '''os.write(1,b'\\x1b[?25hEDITOR\\r\\n')
while True:
 data=os.read(0,65536)
 os.write(1,b'INPUT RECEIVED\\r\\n')
''')
        result=self.cli('workspace','start','--repo',str(self.repo),'--batch',self.batch,
                        '--task-stdin','work/pending',str(agent),str(script),input='never send elsewhere')
        self.assertIn('Task waiting',result.stdout)
        return self.tmux('list-windows','-t',self.session,'-F','#{window_id}').splitlines()[-1]

    def assert_discarded(self, window):
        time.sleep(.7)
        self.assertNotIn('INPUT RECEIVED',self.tmux('capture-pane','-p','-t',window))
        self.assertEqual(self.tmux('list-buffers','-F','#{buffer_name}'),'')
        observed=self.cli('cockpit','--list').stdout
        self.assertIn('pending task discarded',observed)
        self.assertNotIn('task starts afterward',observed)

    def test_worker_leaving_checkout_discards_pending_task(self):
        window=self.pending(change_directory=True)
        (self.root/'open-editor').touch()
        self.wait(window,'EDITOR')
        self.assert_discarded(window)

    def test_checkout_replaced_while_waiting_discards_pending_task(self):
        window=self.pending()
        checkout=self.root/'repo-worktrees/work-pending'
        checkout.rename(self.root/'moved-checkout')
        checkout.mkdir()
        (self.root/'open-editor').touch()
        self.wait(window,'EDITOR')
        self.assert_discarded(window)

    def test_killed_helper_reports_interruption_and_never_submits(self):
        window=self.pending()
        record=self.tmux('show-option','-wqv','-t',window,'@drudwyn_delivery_wait')
        helper=int(record.split(':')[0])
        os.kill(helper,signal.SIGKILL)
        time.sleep(.15)
        observed=self.cli('cockpit','--list').stdout
        self.assertIn('Setup wait interrupted; task not sent. Restart with the task.',observed)
        self.assertNotIn('task starts afterward',observed)
        (self.root/'open-editor').touch()
        self.wait(window,'EDITOR')
        time.sleep(.5)
        self.assertNotIn('INPUT RECEIVED',self.tmux('capture-pane','-p','-t',window))

    def test_transport_stall_never_becomes_a_definitely_unsent_wait(self):
        gate=self.root/'transport-shim';gate.mkdir()
        blocked=self.root/'transport-blocked'; release=self.root/'transport-release'
        wrapper=gate/'tmux'
        wrapper.write_text('#!/bin/sh\nif [ "$1" = load-buffer ]; then\n'
                           'touch '+shlex.quote(str(blocked))+'\n'
                           'while [ ! -e '+shlex.quote(str(release))+' ]; do sleep .02; done\nfi\n'
                           'exec /usr/bin/tmux "$@"\n')
        wrapper.chmod(0o755)
        original=self.env['PATH']; self.env['PATH']=str(gate)+':'+original
        window=self.pending()
        self.env['PATH']=original
        helper=int(self.tmux('show-option','-wqv','-t',window,'@drudwyn_delivery_wait').split(':')[0])
        def cleanup():
            release.touch()
            try: os.killpg(helper,signal.SIGKILL)
            except ProcessLookupError: pass
        self.addCleanup(cleanup)
        (self.root/'open-editor').touch()
        deadline=time.monotonic()+4
        while not blocked.exists():
            self.assertLess(time.monotonic(),deadline)
            time.sleep(.05)
        self.tmux('set-option','-w','-t',window,'@drudwyn_delivery_wait',f'{helper}:unused:0')
        self.assertEqual(self.tmux('show-option','-wqv','-t',window,'@drudwyn_delivery'),'uncertain')
        observed=self.cli('cockpit','--list').stdout
        self.assertNotIn('task not sent',observed)
        result=self.cli('workspace','deliver-task',window,'--retry',input='no overlapping retry',check=False)
        self.assertNotEqual(result.returncode,0)
        release.touch()
        deadline=time.monotonic()+6
        while self.tmux('show-option','-wqv','-t',window,'@drudwyn_delivery')!='sent':
            self.assertLess(time.monotonic(),deadline)
            time.sleep(.05)
        self.assertEqual(self.tmux('list-buffers','-F','#{buffer_name}'),'')

    def test_live_helper_is_not_reported_definitely_unsent_by_wall_clock(self):
        window=self.pending()
        record=self.tmux('show-option','-wqv','-t',window,'@drudwyn_delivery_wait')
        pid,birth,_=record.split(':')
        self.tmux('set-option','-w','-t',window,'@drudwyn_delivery_wait',f'{pid}:{birth}:0')
        observed=self.cli('cockpit','--list').stdout
        self.assertNotIn('task not sent',observed)
        self.assertIn('task starts afterward',observed)
        (self.root/'open-editor').touch()
        deadline=time.monotonic()+6
        while self.tmux('show-option','-wqv','-t',window,'@drudwyn_delivery')!='sent':
            self.assertLess(time.monotonic(),deadline)
            time.sleep(.05)

    def test_stalled_metadata_cannot_extend_setup_deadline(self):
        # Stall only after the helper has acknowledged, then wait for the real
        # production deadline. No timing override or persistent task fixture.
        gate=self.root/'shim';gate.mkdir()
        blocked=self.root/'blocked'; block=self.root/'block'
        wrapper=gate/'tmux'
        wrapper.write_text('#!/bin/sh\nif [ -e '+shlex.quote(str(block))+' ]; then\n'
                           'touch '+shlex.quote(str(blocked))+'\nexec sleep 300\nfi\n'
                           'exec /usr/bin/tmux "$@"\n')
        wrapper.chmod(0o755)
        original=self.env['PATH']
        self.env['PATH']=str(gate)+':'+original
        window=self.pending()
        self.env['PATH']=original
        record=self.tmux('show-option','-wqv','-t',window,'@drudwyn_delivery_wait')
        helper=int(record.split(':')[0]); expires=int(record.split(':')[2])
        block.touch()
        deadline=time.monotonic()+3
        while not blocked.exists():
            self.assertLess(time.monotonic(),deadline)
            time.sleep(.05)
        deadline=time.monotonic()+124
        while Path(f'/proc/{helper}/stat').exists():
            try:
                if Path(f'/proc/{helper}/stat').read_text().rsplit(') ',1)[1].startswith('Z'): break
            except FileNotFoundError: break
            self.assertLess(time.monotonic(),deadline,'Setup helper outlived its deadline')
            time.sleep(.5)
        self.assertLessEqual(time.time(),expires+5)
        observed=self.cli('cockpit','--list').stdout
        self.assertIn('Setup wait expired; task not sent. Restart with the task.',observed)
        self.assertEqual(self.tmux('list-buffers','-F','#{buffer_name}'),'')
        (self.root/'open-editor').touch()
        self.wait(window,'EDITOR')
        time.sleep(.5)
        self.assertNotIn('INPUT RECEIVED',self.tmux('capture-pane','-p','-t',window))


def load_tests(loader, tests, pattern):
    return unittest.TestSuite(StartupWaitTest(name) for name in StartupWaitTest.__dict__ if name.startswith('test_'))

if __name__=='__main__': unittest.main(verbosity=2)
