"""Opt-in real Codex input test; only a local dummy provider, no credentials/model calls.

DRUDWYN_CODEX_TEST_BIN=/absolute/path/to/codex python3 tests/codex_startup_test.py
Every terminal, config, Git checkout and request belongs to this synthetic test.
"""
import json
import os
import signal
from pathlib import Path
import threading
import time
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from worker_launch_test import WorkerLaunchTest

CODEX = os.environ.get('DRUDWYN_CODEX_TEST_BIN')
TASK = 'Local submission probe.\nReply with OK only.'

@unittest.skipUnless(CODEX, 'Set DRUDWYN_CODEX_TEST_BIN for the installed Codex input test')
class CodexStartupTest(WorkerLaunchTest):
    def setUp(self):
        super().setUp()
        self.submissions = []
        submissions = self.submissions
        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args): pass
            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
                if any(item.get('role') == 'user' and any(
                    content.get('text') == TASK for content in item.get('content', [])
                ) for item in body.get('input', [])):
                    submissions.append(time.monotonic())
                response = {'id':'resp_local', 'object':'response', 'status':'completed',
                            'output':[], 'usage':{'input_tokens':1,'output_tokens':1,'total_tokens':2}}
                data = ('event: response.completed\ndata: ' + json.dumps(
                    {'type':'response.completed','response':response}) + '\n\n').encode()
                self.send_response(200)
                self.send_header('Content-Type','text/event-stream')
                self.send_header('Content-Length',str(len(data)))
                self.end_headers(); self.wfile.write(data)
            def do_GET(self):
                data = b'{"data":[]}'
                self.send_response(200); self.send_header('Content-Length',str(len(data)))
                self.end_headers(); self.wfile.write(data)
        server = ThreadingHTTPServer(('127.0.0.1',0), Handler)
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        threading.Thread(target=server.serve_forever,daemon=True).start()
        self.home = self.root/'isolated-codex'
        self.home.mkdir()
        self.config = f'''model = "gpt-4.1"
model_provider = "local_test"
approval_policy = "never"
sandbox_mode = "read-only"
check_for_update_on_startup = false
[model_providers.local_test]
name = "Local submission test"
base_url = "http://127.0.0.1:{server.server_port}/v1"
wire_api = "responses"
requires_openai_auth = false
'''
        self.tmux('set-environment','-g','CODEX_HOME',str(self.home))
        # Never copy user credentials/config/hooks into this test home.
        for name in ('OPENAI_API_KEY','CODEX_API_KEY','OPENAI_BASE_URL'):
            self.tmux('set-environment','-gu',name,check=False)

    def launch(self, trusted):
        config = self.config
        if trusted:
            config += f'\n[projects."{self.repo}"]\ntrust_level = "trusted"\n'
        (self.home/'config.toml').write_text(config)
        result = self.cli('workspace','start','--repo',str(self.repo),'--batch',self.batch,
                          '--task-stdin','work/probe',CODEX,'--no-daemon','--no-alt-screen',input=TASK,check=False)
        self.assertEqual(result.returncode,0,result.stderr)
        window = self.tmux('list-windows','-t',self.session,'-F','#{window_id}').splitlines()[-1]
        pid = int(self.tmux('display-message','-p','-t',window,'#{pane_pid}'))
        def stop_codex():
            self.tmux('kill-window','-t',window,check=False)
            for _ in range(100):
                try:
                    if (Path('/proc')/str(pid)/'stat').read_text().split(') ',1)[1].startswith('Z'):
                        return
                except FileNotFoundError:
                    return
                time.sleep(.02)
            try: os.kill(pid,signal.SIGTERM)
            except ProcessLookupError: pass
        self.addCleanup(stop_codex)
        return result, window

    def assert_submitted_once(self, window):
        for _ in range(120):
            if self.submissions: break
            time.sleep(.1)
        self.assertEqual(len(self.submissions),1, self.tmux('capture-pane','-p','-t',window))
        time.sleep(.4)
        self.assertEqual(len(self.submissions),1)
        self.assertEqual(self.tmux('show-option','-wqv','-t',window,'@drudwyn_delivery'),'sent')
        self.assertEqual(self.tmux('list-buffers','-F','#{buffer_name}'),'')

    def test_trusted_repository_submits_without_manual_enter(self):
        result, window = self.launch(True)
        self.assertIn('Task sent', result.stdout)
        self.assert_submitted_once(window)

    def test_first_run_waits_for_user_setup_then_submits_without_second_enter(self):
        result, window = self.launch(False)
        self.assertIn('Task waiting', result.stdout)
        screen = self.wait(window,'Trust this folder?')
        self.assertNotIn('Local submission probe',screen)
        self.assertEqual(self.submissions,[])
        self.assertNotIn('trust_level = "trusted"',(self.home/'config.toml').read_text())
        # This explicit simulated USER action is confined to our disposable repo.
        self.tmux('send-keys','-t',window,'Enter')
        self.assert_submitted_once(window)

def load_tests(loader, tests, pattern):
    return unittest.TestSuite(CodexStartupTest(n) for n in CodexStartupTest.__dict__ if n.startswith('test_'))
if __name__ == '__main__': unittest.main()
