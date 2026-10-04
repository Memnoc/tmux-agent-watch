"""Explicit promotion and guarded cleanup through commands and real Cockpit keys."""
from pathlib import Path
import os
import re
import shlex
import subprocess
import sys
import time
import unittest
from independent_navigation_test import IndependentNavigation, BIN
from worker_integration_test import WorkerIntegrationTest

class PromotionCleanupTest(WorkerIntegrationTest):
    setUp = WorkerIntegrationTest.setUp
    git = WorkerIntegrationTest.git
    token = WorkerIntegrationTest.token
    integrate = WorkerIntegrationTest.integrate

    def test_promotion_requires_review_and_destination_own_checks(self):
        assembly = Path(self.tmp.name)/'assembly'
        self.git('worktree', 'add', '-b', 'assemble', str(assembly), self.commit)
        verified = subprocess.run([str(BIN), 'workspace', 'verify', '--path', str(assembly),
                                   '--check', 'assembly-check', '--command-stdin'], input='true\n',
                                  env=self.env, text=True, capture_output=True, check=True)
        self.assertIn('Passed', verified.stdout)
        args = ['workspace', 'promote', '--path', str(assembly), '--base', 'main']
        preview = self.command(*args, check=False)
        self.assertEqual(preview.returncode, 0, preview.stderr)
        self.assertIn('PROMOTE PREVIEW', preview.stdout)
        self.assertIn('refs/heads/assemble', preview.stdout)
        self.assertIn('Destination verification:', preview.stdout)
        self.assertIn('Not verified', preview.stdout)
        self.assertNotIn('Passed', preview.stdout)
        self.assertEqual(self.git('rev-parse', 'main'), self.base)
        self.assertIn('Integrated:', self.command(*args, '--apply', self.token(preview)).stdout)
        self.assertEqual(self.git('rev-parse', 'main'), self.commit)
        self.assertIn('Not verified', self.command('workspace', 'verify', '--path', str(self.repo)).stdout)
        self.assertTrue(assembly.is_dir())
        self.assertIn('No-op:', self.command(*args, '--apply', self.token(self.command(*args))).stdout)

    def test_finish_refuses_unrelated_primary_head_and_active_writer(self):
        self.git('switch', '-qc', 'planning')
        self.git('merge', '--ff-only', 'worker')
        result = self.command('workspace', 'finish', '--path', str(self.source), '--base', 'main', '--yes', check=False)
        self.assertNotEqual(result.returncode, 0, 'Unrelated primary HEAD must not authorize cleanup')
        self.assertTrue(self.source.is_dir())
        self.assertIn('not contained', result.stderr)
        self.worker_hook('stop')  # Review is not writer absence.
        result = self.command('workspace', 'finish', '--path', str(self.source), '--base', 'planning', '--yes', check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('active writer', result.stderr)
        self.assertTrue(self.source.is_dir())

    def test_cockpit_promotion_is_explicit_and_finish_confirms_selected_batch(self):
        assembly = Path(self.tmp.name)/'assembly'
        self.git('worktree', 'add', '-b', 'assemble', str(assembly), self.commit)
        result = self.command('batch', 'setup', '--repo', str(self.repo), '--session', self.batch.split('/')[0],
                              '--integration', 'assemble', '--reuse-existing', '--yes',
                              '--expect-source', self.base, '--expect-destination', self.commit,
                              client=self.clients[0])
        batch = re.search(r'Batch: (\S+)', result.stdout)[1]
        self.tmux('set-option', '-w', '-t', self.worker, '@drudwyn_batch', batch)
        before = [self.selection(c) for c in self.clients]
        pane = self.integration_ui()
        self.wait_pane(pane, 'INTEGRATE PREVIEW')
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'Snapshot: r refresh')
        self.tmux('send-keys', '-t', pane, 'P')
        output = self.wait_pane(pane, 'PROMOTE PREVIEW')
        self.assertIn('refs/heads/assemble', output)
        self.assertEqual(self.git('rev-parse', 'main'), self.base)
        self.tmux('send-keys', '-t', pane, 'y')
        self.wait_pane(pane, 'Integrated:')
        self.assertEqual(self.git('rev-parse', 'main'), self.commit)
        self.assertEqual([self.selection(c) for c in self.clients], before)
        self.tmux('send-keys', '-t', pane, 'Escape')
        self.wait_pane(pane, 'Snapshot: r refresh')
        self.stop_pane(self.worker)
        self.tmux('send-keys', '-t', pane, 'r')
        self.wait_pane(pane, 'No workspaces match')
        self.tmux('send-keys', '-t', pane, 'w')
        self.wait_pane(pane, 'MATCHING 1 windows')
        self.wait_pane(pane, 'Snapshot: r refresh')
        self.tmux('send-keys', '-t', pane, 'j')
        self.tmux('send-keys', '-t', pane, 'f')
        output = self.wait_pane(pane, 'FINISH PREVIEW')
        self.assertIn('refs/heads/assemble', output)
        self.assertIn('branch is retained', output)
        self.assertTrue(self.source.exists())
        self.tmux('send-keys', '-t', pane, 'y')
        self.wait_pane(pane, 'Removed worktree; branch retained')
        self.assertFalse(self.source.exists())
        self.assertEqual(self.git('rev-parse', 'worker'), self.commit)
        self.assertIn(self.home, self.tmux('list-windows', '-a', '-F', '#{window_id}'))

    def stop_pane(self, pane, source=None):
        source=source or self.source
        self.tmux('set-option','-w','-t',pane,'remain-on-exit','on')
        self.tmux('set-option','-p','-t',pane,'@drudwyn_recovery_checkout',os.fsencode(source).hex())
        self.tmux('respawn-pane','-k','-t',pane,'-c',str(source),'true')
        deadline=time.monotonic()+3
        while self.tmux('display-message','-p','-t',pane,'#{pane_dead}')!='1':
            self.assertLess(time.monotonic(),deadline);time.sleep(.01)

    def idle_integrated(self):
        # Preserve the helper seam used by the independent reproduction. The
        # eligible baseline now explicitly ends the source pane, never infers idle.
        self.integrate('--apply', self.token(self.integrate()))
        self.stop_pane(self.worker)

    def finish(self, *args, destination='main', check=True, env=None):
        result = subprocess.run([str(BIN), 'workspace', 'finish', '--path', str(self.source),
                               '--destination', destination, *args], env=env or self.env,
                              text=True, capture_output=True)
        if check: self.assertEqual(result.returncode,0,result.stderr)
        return result

    def test_finish_preserves_mixed_coordinator_linked_sessions_and_branch(self):
        self.idle_integrated()
        # The worker-only window can close across links; mixed/coordinator windows survive.
        mixed = self.tmux('new-window', '-d', '-P', '-F', '#{window_id}', '-t', 'project', '-c', str(self.source), 'bash --noprofile --norc')
        mixed_source_pane=self.tmux('display-message','-p','-t',mixed,'#{pane_id}')
        other = self.tmux('split-window', '-d', '-P', '-F', '#{pane_id}', '-t', mixed, '-c', str(self.repo), 'bash --noprofile --norc')
        self.tmux('new-session', '-d', '-s', 'other', '-c', str(self.repo), 'bash --noprofile --norc')
        untouched = self.tmux('display-message', '-p', '-t', 'other', '#{window_id}')
        self.tmux('link-window', '-s', self.worker, '-t', 'other:9')
        self.tmux('link-window', '-s', mixed, '-t', 'other:8')
        # Coordinator identity lives in a different session, still authoritative.
        coord = self.tmux('new-window', '-d', '-P', '-F', '#{window_id}', '-t', 'other', '-c', str(self.source), 'bash --noprofile --norc')
        self.tmux('set-option', '-t', 'other', '@drudwyn_coordinator', coord)
        self.stop_pane(mixed_source_pane)
        self.stop_pane(coord)
        preview = self.finish('--preview')
        self.assertTrue(self.source.exists())
        self.finish('--apply', self.token(preview), '--yes')
        self.assertFalse(self.source.exists())
        self.assertEqual(self.git('rev-parse', 'worker'), self.commit)
        windows = self.tmux('list-windows', '-a', '-F', '#{window_id}').splitlines()
        self.assertNotIn(self.worker, windows)
        for w in [mixed, coord, self.home, untouched]: self.assertIn(w, windows)
        self.assertEqual(self.tmux('display-message', '-p', '-t', other, '#{pane_id}'), other)
        self.assertIn('other', self.tmux('list-sessions', '-F', '#{session_name}'))
        self.assertNotEqual(self.finish('--yes', check=False).returncode, 0)
        self.assertEqual(self.git('rev-parse', 'worker'), self.commit)

    def test_finish_dirty_ignored_operations_detached_and_stale_previews(self):
        self.idle_integrated()
        for tracked in [False, True]:
            file = self.source/('common.txt' if tracked else 'untracked')
            previous=file.read_bytes() if tracked else None
            file.write_text('preserve me')
            self.assertNotEqual(self.finish('--yes', check=False).returncode, 0)
            self.assertEqual(file.read_text(), 'preserve me')
            if tracked: file.write_bytes(previous)
            else: file.unlink()
        ignore = self.repo/'.git/info/exclude'
        ignore.write_text('ignored-input\n')
        local=self.source/'ignored-input'
        local.write_text('ignored work')
        result=self.finish('--yes', check=False)
        self.assertNotEqual(result.returncode,0)
        self.assertEqual(local.read_text(),'ignored work')
        local.unlink()
        gitdir=Path(self.git('rev-parse','--absolute-git-dir',path=self.source))
        for name in ['MERGE_HEAD','CHERRY_PICK_HEAD','REVERT_HEAD','rebase-merge','rebase-apply','sequencer','BISECT_LOG','index.lock','HEAD.lock']:
            marker=gitdir/name
            marker.touch()
            result=self.finish('--yes',check=False)
            self.assertIn('operation or lock',result.stderr,name)
            self.assertTrue(self.source.exists())
            marker.unlink()
        self.git('checkout','--detach',path=self.source)
        self.assertIn('Detached',self.finish('--yes',check=False).stderr)
        self.git('switch','worker',path=self.source)
        token=self.token(self.finish('--preview'))
        self.git('commit','--allow-empty','-qm','destination moved')
        self.assertIn('changed after preview',self.finish('--apply',token,'--yes',check=False).stderr)
        token=self.token(self.finish('--preview'))
        self.git('commit','--allow-empty','-qm','source moved',path=self.source)
        self.git('merge','--no-edit','worker')
        self.assertIn('changed after preview',self.finish('--apply',token,'--yes',check=False).stderr)
        self.assertTrue(self.source.exists())

    def test_finish_background_process_and_nonshell_caller_are_writers(self):
        self.idle_integrated()
        writer=subprocess.Popen(['sleep','300'],cwd=self.source)
        try:
            self.assertIn('active writer',self.finish('--yes',check=False).stderr)
        finally:
            writer.terminate(); writer.wait()
        # An agent/editor process invoking Finish from the source is not exempt.
        script='import subprocess,sys; p=subprocess.run(sys.argv[1:],capture_output=True,text=True); print(p.stderr); sys.exit(p.returncode)'
        result=subprocess.run([sys.executable,'-c',script,str(BIN),'workspace','finish','--path',str(self.source),'--base','main','--yes'],cwd=self.source,env=self.env,text=True,capture_output=True)
        self.assertNotEqual(result.returncode,0)
        self.assertIn('active writer',result.stdout)
        self.assertTrue(self.source.exists())
        # The normal invocation shell itself is exempt, and no caller window is killed.
        result=subprocess.run(['bash','--noprofile','--norc','-c','"$@"', 'finish', str(BIN),'workspace','finish','--path',str(self.source),'--base','main','--yes'],cwd=self.source,env=self.env,text=True,capture_output=True)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertFalse(self.source.exists())

    def test_promotion_stale_dirty_conflict_abort_and_actual_base_checkout(self):
        assembly=Path(self.tmp.name)/'assembly'
        self.git('worktree','add','-b','assemble',str(assembly),self.commit)
        self.git('switch','-qc','planning')
        target=Path(self.tmp.name)/'actual base'
        self.git('worktree','add',str(target),'main')
        args=['workspace','promote','--path',str(assembly),'--base','main']
        preview=self.command(*args)
        self.assertIn(str(target),preview.stdout)
        self.git('commit','--allow-empty','-qm','assembly moved',path=assembly)
        self.assertIn('changed after preview',self.command(*args,'--apply',self.token(preview),check=False).stderr)
        local=target/'local'
        local.write_text('preserve')
        self.assertIn('dirty',self.command(*args,check=False).stderr)
        self.assertEqual(local.read_text(),'preserve'); local.unlink()
        (target/'worker.txt').write_text('base conflict')
        self.git('add','.',path=target);self.git('commit','-qm','conflict',path=target)
        before=self.git('rev-parse','HEAD',path=target)
        result=self.command(*args,'--apply',self.token(self.command(*args)),check=False)
        self.assertIn('merge retained',result.stderr)
        self.assertTrue(self.git('ls-files','--unmerged',path=target))
        self.assertEqual(self.git('rev-parse','HEAD'),self.base)
        self.command('workspace','conflict','--path',str(target),'--abort')
        self.assertEqual(self.git('rev-parse','HEAD',path=target),before)
        self.assertEqual(self.git('status','--porcelain',path=target),'')
        self.assertTrue(assembly.exists())

    def test_finish_cancel_removal_failure_and_orphan_child_guards(self):
        import shutil
        self.idle_integrated()
        args=[str(BIN),'workspace','finish','--path',str(self.source),'--destination','main']
        cancelled=subprocess.run(args,input='n\n',env=self.env,text=True,capture_output=True)
        self.assertIn('cancelled',cancelled.stderr)
        self.assertTrue(self.source.exists())
        wrapper_dir=Path(self.tmp.name)/'wrapper';wrapper_dir.mkdir()
        wrapper=wrapper_dir/'git'
        real_git=shutil.which('git')
        wrapper.write_text('#!/bin/sh\ncase " $* " in *" worktree remove "*) echo PRIVATE_REMOVE_OUTPUT; exit 17;; esac\nexec '+shlex.quote(real_git)+' "$@"\n')
        wrapper.chmod(0o755)
        env={**self.env,'PATH':str(wrapper_dir)+':'+os.environ['PATH']}
        result=self.finish('--yes',env=env,check=False)
        self.assertIn('removal failed',result.stderr)
        self.assertNotIn('PRIVATE_REMOVE_OUTPUT',result.stdout+result.stderr)
        self.assertTrue(self.source.exists())
        self.assertIn(self.worker,self.tmux('list-windows','-a','-F','#{window_id}'))
        arrived=Path(self.tmp.name)/'arrived';release=Path(self.tmp.name)/'release'
        wrapper.write_text('#!/bin/sh\ncase " $* " in *" worktree remove "*) touch '+shlex.quote(str(arrived))+'; while [ ! -e '+shlex.quote(str(release))+' ]; do sleep .01; done;; esac\nexec '+shlex.quote(real_git)+' "$@"\n')
        parent=subprocess.Popen(args+['--yes'],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        self.addCleanup(lambda:release.touch())
        try:
            deadline=time.monotonic()+5
            while not arrived.exists():
                self.assertIsNone(parent.poll());self.assertLess(time.monotonic(),deadline);time.sleep(.02)
            parent.kill();parent.wait()
            result=self.finish('--yes',check=False)
            self.assertIn('already in progress',result.stderr)
            self.assertTrue(self.source.exists())
            # The target guard also survives the supervisor, blocking verification there.
            result=subprocess.run([str(BIN),'workspace','verify','--path',str(self.repo),'--check','blocked','--command-stdin'],input='true',env=self.env,text=True,capture_output=True)
            self.assertNotEqual(result.returncode,0)
        finally:
            if parent.poll() is None:parent.kill();parent.wait()
            release.touch()
        deadline=time.monotonic()+5
        while self.source.exists():self.assertLess(time.monotonic(),deadline);time.sleep(.02)
        self.assertEqual(self.git('rev-parse','worker'),self.commit)
        self.assertIn(self.worker,self.tmux('list-windows','-a','-F','#{window_id}'))

    def test_unobservable_unrelated_process_is_not_an_associated_writer(self):
        self.idle_integrated()
        script='import ctypes,time; ctypes.CDLL(None).prctl(4,0,0,0,0); time.sleep(300)'
        unrelated=subprocess.Popen([sys.executable,'-c',script],cwd=self.tmp.name)
        try:
            for _ in range(100):
                try:os.readlink('/proc/'+str(unrelated.pid)+'/cwd')
                except PermissionError:break
                time.sleep(.01)
            else:self.fail('fixture did not become nondumpable')
            self.assertIn('FINISH PREVIEW',self.finish('--preview').stdout)
            # The same hidden cwd in an associated worker pane must refuse.
            self.tmux('respawn-pane','-k','-t',self.worker,'-c',str(self.source),shlex.join([sys.executable,'-c',script]))
            deadline=time.monotonic()+3
            while self.tmux('display-message','-p','-t',self.worker,'#{pane_current_command}')!='python3':
                self.assertLess(time.monotonic(),deadline);time.sleep(.01)
            result=self.finish('--yes',check=False)
            self.assertNotEqual(result.returncode,0)
            self.assertTrue(self.source.exists())
        finally:unrelated.terminate();unrelated.wait()

    def test_newline_checkout_and_alias_cannot_retarget_sibling(self):
        normal=Path(self.tmp.name)/'sentinel'
        unusual=Path(str(normal)+'\n')
        alias=Path(self.tmp.name)/'safe alias'
        self.git('worktree','add','-b','sentinel',str(normal),'main')
        self.git('worktree','add','-b','unusual',str(unusual),'main')
        alias.symlink_to(unusual,target_is_directory=True)
        for selected in [unusual,alias]:
            for action,tail in [('integrate',['--destination','main']),('promote',['--base','main']),('finish',['--base','main','--yes'])]:
                result=self.command('workspace',action,'--path',str(selected),*tail,check=False)
                self.assertNotEqual(result.returncode,0,(selected,action,result.stdout))
                self.assertTrue(normal.exists());self.assertTrue(unusual.exists())
                self.assertEqual(self.git('rev-parse','main'),self.base)

    def test_finish_batch_metadata_loss_never_borrows_another_target(self):
        self.idle_integrated()
        self.git('worktree','add','-b','assembly',str(Path(self.tmp.name)/'assembly'),self.base)
        result=self.command('batch','setup','--repo',str(self.repo),'--session',self.batch.split('/')[0],
                            '--integration','assembly','--reuse-existing','--yes','--expect-source',self.commit,
                            '--expect-destination',self.base,client=self.clients[0])
        other=re.search(r'Batch: (\S+)',result.stdout)[1]
        # The explicit older main batch remains authoritative even with a newer batch.
        args=['workspace','finish','--path',str(self.source),'--batch',self.batch,'--preview']
        self.assertIn('refs/heads/main',self.command(*args).stdout)
        self.assertIn('not contained',self.command('workspace','finish','--path',str(self.source),'--batch',other,'--yes',check=False).stderr)
        session,key=self.batch.split('/')
        self.tmux('set-option','-u','-t',session,'@drudwyn_batch_'+key)
        result=self.command(*args,check=False)
        self.assertNotEqual(result.returncode,0)
        self.assertIn('reselect',result.stderr)
        self.assertTrue(self.source.exists())
        self.assertNotEqual(self.command('workspace','finish','--path',str(self.source),'--yes',check=False).returncode,0)

    def test_cockpit_narrow_redacted_promotion_cancel_and_explicit_finish_destination(self):
        self.tmux('set','-g','@drudwyn-redact-labels','on')
        pane=self.integration_ui()
        self.wait_pane(pane,'INTEGRATE PREVIEW')
        self.tmux('send-keys','-t',pane,'Escape')
        self.wait_pane(pane,'Snapshot: r refresh')
        self.tmux('send-keys','-t',pane,'P')
        self.wait_pane(pane,'PROMOTE PREVIEW')
        self.tmux('resize-window','-t',pane,'-x','48','-y','24')
        output=self.wait_pane(pane,'[Esc] cancel')
        self.assertIn('[y] Promote reviewed changes',output)
        self.assertIn('[redacted]',output)
        self.assertNotIn(str(self.repo),output)
        self.assertNotIn(self.base,output)
        Path('/tmp/drudwyn-ticket14-narrow-promotion.txt').write_text(output)
        self.tmux('send-keys','-t',pane,'Escape')
        self.wait_pane(pane,'WORKSPACE COCKPIT')
        self.assertEqual(self.git('rev-parse','main'),self.base)
        self.tmux('kill-window','-t',pane)
        self.tmux('set','-g','@drudwyn-redact-labels','off')
        self.tmux('set-option','-wu','-t',self.worker,'@drudwyn_batch')
        pane=self.integration_ui()
        self.wait_pane(pane,'INTEGRATE · CHOOSE DESTINATION')
        self.tmux('send-keys','-t',pane,'Escape')
        self.wait_pane(pane,'Snapshot: r refresh')
        self.tmux('send-keys','-t',pane,'f')
        output=self.wait_pane(pane,'FINISH · CHOOSE DESTINATION')
        self.assertNotIn('Destination branch: main',output)
        self.tmux('send-keys','-t',pane,'e')
        self.tmux('send-keys','-t',pane,'Enter')
        self.wait_pane(pane,'Choose exactly one explicit destination')
        self.assertTrue(self.source.exists())

    def test_window_closure_rechecks_pane_set_coordinator_and_session_on_server(self):
        import shutil
        real_tmux=shutil.which('tmux')
        wrappers=Path(self.tmp.name)/'tmux-wrapper';wrappers.mkdir()
        wrapper=wrappers/'tmux'
        env={**self.env,'PATH':str(wrappers)+':'+os.environ['PATH']}
        for change in ['pane','respawn','coordinator','last-session']:
            with self.subTest(change=change):
                source=Path(self.tmp.name)/('close-'+change)
                self.git('worktree','add','-b','close-'+change,str(source),'main')
                window=self.tmux('new-window','-d','-P','-F','#{window_id}','-t','project','-c',str(source),'bash --noprofile --norc')
                self.stop_pane(window,source)
                if change=='pane':
                    injection=[real_tmux,'split-window','-d','-t',window,'-c',str(self.repo),'bash --noprofile --norc']
                elif change=='respawn':
                    injection=[real_tmux,'respawn-pane','-k','-t',window,'-c',str(self.repo),'bash --noprofile --norc']
                elif change=='coordinator':
                    injection=[real_tmux,'set-option','-t','project','@drudwyn_coordinator',window]
                else:
                    self.tmux('new-session','-d','-s','last-'+change,'-c',str(self.repo),'bash --noprofile --norc')
                    other=self.tmux('display-message','-p','-t','last-'+change,'#{window_id}')
                    self.tmux('link-window','-s',window,'-t','last-'+change+':9')
                    injection=[real_tmux,'kill-window','-t',other]
                wrapper.write_text('#!/bin/sh\nif [ "$1" = if-shell ] && [ "$4" = '+shlex.quote(window)+' ]; then '+shlex.join(injection)+' >/dev/null; fi\nexec '+shlex.quote(real_tmux)+' "$@"\n')
                wrapper.chmod(0o755)
                result=subprocess.run([str(BIN),'workspace','finish','--path',str(source),'--base','main','--yes'],env=env,text=True,capture_output=True)
                self.assertEqual(result.returncode,0,result.stderr)
                self.assertFalse(source.exists())
                self.assertIn(window,self.tmux('list-windows','-a','-F','#{window_id}'))
                self.assertEqual(self.git('rev-parse','close-'+change),self.base)
                self.tmux('set-option','-t','project','@drudwyn_coordinator',self.home)

    def test_builtin_shell_waiting_without_children_cannot_authorize_finish(self):
        self.idle_integrated()
        for interactive in [False,True]:
            with self.subTest(interactive=interactive):
                started=Path(self.tmp.name)/('shell-started-'+str(interactive))
                release=Path(self.tmp.name)/('shell-release-'+str(interactive))
                script='printf ready > '+shlex.quote(str(started))+'; while [[ ! -e '+shlex.quote(str(release))+' ]]; do read -r -t 0.1 -n 1 ignored; done; printf pending-work > worker.txt; read -r -t 300 ignored'
                command=['bash','--noprofile','--norc'] + ([] if interactive else ['-c',script])
                self.tmux('respawn-pane','-k','-t',self.worker,'-c',str(self.source),shlex.join(command))
                if interactive:
                    self.tmux('send-keys','-t',self.worker,'-l',script)
                    self.tmux('send-keys','-t',self.worker,'Enter')
                deadline=time.monotonic()+5
                while not started.exists():self.assertLess(time.monotonic(),deadline);time.sleep(.02)
                for action in ['--preview','--yes']:
                    result=self.finish(action,check=False)
                    self.assertNotEqual(result.returncode,0,(interactive,action,result.stdout))
                    self.assertIn('active writer',result.stderr)
                    self.assertTrue(self.source.exists())
                    self.assertIn(self.worker,self.tmux('list-windows','-a','-F','#{window_id}'))
                release.touch()
                deadline=time.monotonic()+3
                while (self.source/'worker.txt').read_text()!='pending-work':self.assertLess(time.monotonic(),deadline);time.sleep(.02)
                self.git('restore','worker.txt',path=self.source)

    def test_calling_shell_background_child_outside_checkout_is_still_writer(self):
        self.idle_integrated()
        script='(cd "$1"; exec sleep 300) & child=$!; "$2" workspace finish --path "$3" --base main --yes; result=$?; kill "$child"; wait "$child" 2>/dev/null; exit "$result"'
        result=subprocess.run(['bash','--noprofile','--norc','-c',script,'invoke',self.tmp.name,str(BIN),str(self.source)],cwd=self.source,env=self.env,text=True,capture_output=True)
        self.assertNotEqual(result.returncode,0,result.stdout)
        self.assertIn('active writer',result.stderr)
        self.assertTrue(self.source.exists())

if __name__ == '__main__':
    names = [n for n in PromotionCleanupTest.__dict__ if n.startswith('test_') and (len(sys.argv)==1 or sys.argv[1] in n)]
    result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(PromotionCleanupTest(n) for n in names))
    sys.exit(not result.wasSuccessful())
