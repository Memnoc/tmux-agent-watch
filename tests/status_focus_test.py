"""Approved single-row B through the installed plugin and public CLI."""
import os
from pathlib import Path
import subprocess
import unittest
from independent_navigation_test import ROOT, BIN
from status_a_test import plain, StatusATest

class FocusStatus(StatusATest):
    def test_explicit_focus_is_one_row_with_git_counts_and_attention(self):
        self.tmux('set', '-g', '@drudwyn-status-layout', 'focus')
        self.worker_hook('permissionRequest')
        path=self.repo/'lines.txt'
        path.write_text('one\ntwo\nthree\n')
        subprocess.run(['git','-C',str(self.repo),'add','.'],check=True)
        subprocess.run(['git','-C',str(self.repo),'commit','-qm','Fixture'],check=True)
        path.write_text('one\nfour\n')
        self.command('navigate','--window',self.worker,client=self.clients[0])
        self.tmux('set','-g','mouse','on')
        self.tmux('set-environment','-g','DRUDWYN_V2_BIN',str(BIN))
        installed=subprocess.run(['bash',str(ROOT/'tmux-drudwyn.tmux')],env={**self.env,'DRUDWYN_V2_BIN':str(BIN)},capture_output=True,text=True)
        self.assertEqual(installed.returncode,0,installed.stderr)
        self.assertEqual(self.tmux('show','-gqv','status'),'on')
        self.assertIn('focus',self.tmux('show','-gqv','status-format[0]'))
        self.assertEqual(self.tmux('show','-gqv','status-format[1]'),'')
        session,window=self.selection(self.clients[0]).split(':')
        for width in [48,64,80,120,160]:
            output=self.command('status-bar','--projection','--session',session,'--window',window,'--width',str(width),'--row','focus',client=self.clients[0]).stdout
            row=plain(output).strip()
            self.assertEqual(len(output.splitlines()),1)
            self.assertLessEqual(len(row),width)
            for expected in ['worker','+1','-2','NEED 1']:
                self.assertIn(expected,row)
            self.assertIn('#191724',output)
        before=[self.selection(c) for c in self.clients]
        screen,_=self.terminal(self.clients[0],120,'+1')
        self.assertIn('NEED 1',screen.lines()[-1])
        self.assertNotIn('NEED',screen.lines()[-2])
        self.click(self.clients[0],screen.lines()[-1].index('NEED'),39)
        popup,_=self.terminal(self.clients[0],120,'state attention')
        self.assertIn('MATCHING 1 workers','\n'.join(popup.lines()))
        self.assertEqual([self.selection(c) for c in self.clients],before)
        self.key(self.clients[0], b'q')
        self.terminal(self.clients[0], 120, '+1')
        self.tmux('set','-g','@drudwyn-redact-labels','on')
        row=plain(self.command('status-bar','--projection','--session',session,'--window',window,'--width','120','--row','focus',client=self.clients[0]).stdout)
        self.assertIn('Workspace',row)
        self.assertNotIn('worker',row)
        self.assertNotIn('main',row)

    def test_exit_overlap_and_stale_attention_remain_distinct(self):
        self.exited_worker('stop', 23, managed=True)
        self.tmux('set', '-g', '@drudwyn_scan_at', '1')
        session,window=self.selection(self.clients[0]).split(':')
        for width in [48,80,160]:
            result=self.command('status-bar','--projection','--session',session,'--window',window,'--width',str(width),'--row','focus',client=self.clients[0])
            row=plain(result.stdout).strip()
            self.assertIn('NEED 2* STALE',row)
            self.assertIn('REV/X23' if width<100 else 'REVIEW (hook) / EXIT 23',row)
            self.assertIn('WT',row)
            self.assertLessEqual(len(row),width)

    def test_git_namespace_and_external_diff_cannot_redirect_selected_counts(self):
        path=self.repo/'lines.txt'
        path.write_text('one\ntwo\nthree\n')
        subprocess.run(['git','-C',str(self.repo),'add','.'],check=True)
        subprocess.run(['git','-C',str(self.repo),'commit','-qm','Fixture'],check=True)
        path.write_text('one\nfour\n')
        self.command('navigate','--window',self.worker,client=self.clients[0])
        marker=Path(self.tmp.name)/'external-diff-ran'
        hook=Path(self.tmp.name)/'diff-hook'
        hook.write_text('#!/bin/sh\ntouch '+str(marker)+'\nexit 1\n')
        hook.chmod(0o755)
        self.env.update(GIT_DIR='/not-a-repository',GIT_WORK_TREE='/tmp',GIT_EXTERNAL_DIFF=str(hook),GIT_DIFF_OPTS='--stat-count=0')
        session,window=self.selection(self.clients[0]).split(':')
        result=self.command('status-bar','--projection','--session',session,'--window',window,'--width','120','--row','focus',client=self.clients[0])
        self.assertIn('+1 -2',plain(result.stdout))
        self.assertFalse(marker.exists())

    def test_missing_selected_checkout_never_reports_clean_git(self):
        session,window=self.selection(self.clients[0]).split(':')
        self.tmux('respawn-pane','-k','-t',self.home,'-c','/tmp','sleep','300')
        result=self.command('status-bar','--session',session,'--window',self.home,'--width','80','--row','focus',client=self.clients[0])
        self.assertIn('Git ?',plain(result.stdout))
        self.assertNotIn('+0',plain(result.stdout))

def load_tests(loader,tests,pattern):
    return unittest.TestSuite(FocusStatus(name) for name in FocusStatus.__dict__ if name.startswith('test_'))

if __name__=='__main__':unittest.main()
