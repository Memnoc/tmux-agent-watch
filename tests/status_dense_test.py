"""Dense single-row bar: bounded six-tab navigation, identity and attention."""
import re
import subprocess
import unittest
from independent_navigation_test import ROOT, BIN
from status_a_test import StatusATest, plain

class DenseStatus(StatusATest):
    def test_six_tabs_fit_and_keep_selected_window_and_global_attention(self):
        for i in range(6):
            self.tmux('new-window','-d','-t','project','-n',f'edit-{i}','-c',str(self.repo),'sleep','300')
        self.worker_hook('permissionRequest')
        self.command('navigate','--window',self.worker,client=self.clients[0])
        self.install()
        self.tmux('set','-g','@drudwyn-status-layout','dense')
        self.tmux('set','-g','@drudwyn-visible-tabs','6')
        result=subprocess.run(['bash',str(ROOT/'scripts/hud-install.sh')],env=self.env,capture_output=True,text=True)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertIn('dense',self.tmux('show','-gqv','status-format[0]'))
        self.assertEqual(self.tmux('show','-gqv','status'),'on')
        session,window=self.selection(self.clients[0]).split(':')
        for width in [48,64,80,120,160]:
            output=self.command('status-bar','--projection','--session',session,'--window',window,'--width',str(width),'--row','dense',client=self.clients[0]).stdout
            row=plain(output).rstrip()
            self.assertLessEqual(len(row),width)
            targets=re.findall(r'range=user\|window:([^\]]+)',output)
            self.assertLessEqual(len(targets),6)
            self.assertIn(self.worker,[t.split(':')[-1] for t in targets])
            self.assertIn('NEED 1',row)
            if width==160:
                self.assertEqual(len(targets),6)
                self.assertIn('+0 -0',row)
        self.resize_client(self.clients[0],160)
        for mode in ['safe','nerd']:
            self.tmux('set','-g','@drudwyn-icon-mode',mode)
            self.tmux('set','-g','@drudwyn-agent-icon','bot')
            screen,_=self.terminal(self.clients[0],160,'A 1' if mode=='safe' else '󰚩')
            self.assertIn('NEED 1',screen.lines()[-1])
            self.assertIn('worker',screen.lines()[-1])
        before=[self.selection(c) for c in self.clients]
        self.click(self.clients[0],screen.lines()[-1].index('NEED'),39)
        self.terminal(self.clients[0],160,'state attention')
        self.assertEqual([self.selection(c) for c in self.clients],before)
        self.key(self.clients[0],b'q')
        self.terminal(self.clients[0],160,'NEED 1')
        screen,_=self.terminal(self.clients[0],160,'edit-0')
        self.click(self.clients[0],screen.lines()[-1].index('edit-0'),39)
        import time
        for _ in range(100):
            if self.selection(self.clients[0]) != before[0]:break
            time.sleep(.02)
        self.assertNotEqual(self.selection(self.clients[0]),before[0])
        self.assertEqual(self.selection(self.clients[1]),before[1])
        self.command('navigate','--window',self.worker,client=self.clients[0])
        self.tmux('set','-g','@drudwyn-redact-labels','on')
        row=plain(self.command('status-bar','--session',session,'--window',window,'--width','160','--row','dense',client=self.clients[0]).stdout)
        self.assertNotIn('edit-',row)
        self.assertNotIn('worker',row)

    def test_extreme_narrow_stale_overlap_is_bounded(self):
        for i in range(15): self.tmux('new-window','-d','-t','project','-n',f'shell-{i}','sleep','300')
        self.exited_worker('stop',23,managed=True)
        self.tmux('set','-g','@drudwyn_scan_at','1')
        session,window=self.selection(self.clients[0]).split(':')
        output=self.command('status-bar','--projection','--session',session,'--window',window,'--width','20','--row','dense',client=self.clients[0]).stdout
        self.assertLessEqual(len(plain(output).rstrip('\n')),20)
        self.assertIn('STALE',plain(output))

def load_tests(loader,tests,pattern):
    return unittest.TestSuite(DenseStatus(n) for n in DenseStatus.__dict__ if n.startswith('test_'))
if __name__=='__main__':unittest.main()
