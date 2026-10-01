# 15 — Harden the assembled workflow

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done

**What to build:** A reproducible acceptance run proves the integrated features
work together, followed by a real terminal visual review and release evidence.

**Blocked by:** 04, 08, 10, 14 (transitively all preceding tickets).
**Priority:** Release gate. **Stories:** 1–22.

- [x] Run the complete supported Rust and shell/Python checks on the assembled
  revision, including preexisting lifecycle/status changes and compatibility paths.
- [x] Create a session, launch three named workers from one commit, preserve
  coordinator identity, independently navigate two clients, and recover an exited
  worker through Drudwyn actions using controlled agents.
- [x] Integrate independent commits with one shared-file conflict, hand off to
  the coordinator, Continue/Abort as applicable, and verify the assembled checkout.
- [x] Exercise direct-to-base and integration-branch workflows including explicit
  promotion, failed verification from an untracked input, rerun, and guarded cleanup.
- [x] Inspect actual terminal status/Cockpit layouts at all specified widths,
  icon modes, themes, redaction, 36-worker scale, and independent client actions.
- [x] Record discovery/rendering costs and responsiveness with many Git checkouts;
  investigate regressions rather than borrowing the browser spike's performance.
- [x] Confirm documentation/help/configuration, content-blind data flow, no implicit
  push/deployment, and crosscheck findings. Record limitations before calling it shipped.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.

## Assembled acceptance receipt — 2026-10-01

Implemented on `work/worktree-worker-workflow` from
`adb574093a80421caabee73945424246ed3dc568`; production features 01–14 were already
independently cleared. The [acceptance report](../../docs/specs/2026-10-01-worktree-worker-acceptance.md)
and spec Verification contain every story verbatim, Linux execution verdicts,
real terminal evidence, measurements, and explicit unverified release boundaries.

`tests/assembled_workflow_test.py` reuses the real Git/tmux/two-client fixtures
and exercises the release binary through the actual plugin/scripts. Both direct
and integration-branch flows passed through three named pinned workers,
multiline/reference delivery, stable coordinator, independent Cockpit Open,
retained exit/recovery, real conflict handoff/Abort/Continue, explicit checks
(untracked input failure then rerun), promotion and guarded cleanup.
`tests/run.sh` now includes the release build and assembled scenarios.

Only approved small cosmetics changed production code: empty plain-shell
activity text/separator removal and singular `1 project`. A resulting narrow
long-name shell fallback failure was independently observed, routed to a fresh
fix session, reproduced in five combinations, corrected, and verified against
20 real terminal role/width/redaction combinations. No hardening failure was
silently marked passed. Cockpit's existing width/icon/redaction matrix now runs
across all three supported themes (60 combinations).

Final frozen gates passed: fmt, 41 Rust tests, complete shell/Python suite (212 Python
cases; one existing optional Resurrect skip), release build, both assembled
scenarios (24.641s total), and diff whitespace checks. Logs:
`/tmp/drudwyn-ticket15-rust-final.log`,
`/tmp/drudwyn-ticket15-full-final.log`, and
`/tmp/drudwyn-ticket15-release-build.log`. The first suite was superseded after
the context correction; it is not counted as final evidence. Final source/tests
were frozen for the passing run; only receipts changed afterward.

36-worker/four-project/39-checkout evidence: 1167ms discovery, 1.181s CLI,
1.130s actual Cockpit startup, 6ms last-row inspection, 30ms inspection during
blocked refresh, 1.767s simultaneous two-client fresh status rendering, and
0.990s installed-row readiness. These match the scale of prior real fixtures;
no browser timing or universal performance claim is used.

All implementation tickets are complete. Native macOS/ARM64 runtime and manual
release gates remain unverified and are not accepted as shipped. The repository's
`main` remains `eaf24469290cbf77dd1d2a6176fbd54f7ace1868`; no push or deployment.
The final independent Standards/Spec receipt follows separately. Prior receipt
string and repeated shell-classification maintenance judgments remain nonblocking.

## Independent Northstar crosscheck — 2026-10-01

Reviewed `adb574093a80421caabee73945424246ed3dc568` through implementation
commit `d5571cbfb46747801a6e9e01475c7ab16d4ec62b`, running Standards and Spec
sequentially under the implement-all override. Sources were CONTRIBUTING,
CONTEXT, the privacy boundary, ADR0002/0003/0005/0006/0007, this ticket, the
approved breakdown and all 22 originating user stories. All seven production
and test file hashes match the independently reviewed corrected candidate.
The final report, spec Verification and queue accurately retain the evidence
and release limitations.

**Standards: clear — 0 severe, 0 new judgement calls.** The scoped rendering
changes and acceptance harness preserve the documented stateless, content-blind
boundary and reuse the real Git/tmux command and terminal seams. Earlier
nonblocking maintenance observations remain: ticket13 receipt strings
(possible Primitive Obsession), and ticket14 repeated shell classification
(possible Duplicated Code).

**Spec: clear — 0 unresolved severe findings.** Independent execution reproduced
the cosmetic regression that erased long selected-shell/coordinator context at
64/80 columns. The fresh fix session corrected the fallback without inventing
activity. The unchanged public reproduction now preserves COORD and the bounded
name; independently decoded ANSI also confirms ordinary-shell and redacted
coordinator behavior. Evidence is in
`/tmp/drudwyn-ticket15-independent-context-after.log` and the recorded
long-context red/green logs and terminal artifacts. No failing case was omitted
from the final report.

The independent direct-to-base release scenario passed in 12.104s; its log is
`/tmp/drudwyn-ticket15-independent-direct.log`. The harness selects the actual
release binary through `scripts/v2.sh`, loads the plugin, verifies controlled
agent executables, and uses disposable real Git repositories and two attached
tmux clients. Recovery and cleanup use observed exited panes, not invented
exit receipts. Both destination scenarios passed in the builder's final frozen
run (24.641s). Final logs confirm 41 Rust tests and the complete shell/Python,
privacy, packaging and release checks: 212 Python cases, 211 passed and one
existing optional Resurrect skip. Runtime hash equality required no duplicate
full-suite execution. The 22-story tables match the spec verbatim; recorded
terminal artifacts and 36-worker timings are real runtime evidence.

Acceptance remains scoped to Linux x86_64 / tmux 3.4. Native macOS/ARM64,
real third-party service sessions and manual release-artifact observations are
unverified, not accepted as shipped. The previously noted conservative
non-Linux source-shell Finish veto remains a portability limitation to validate.
This receipt changes only ticket15; no production edits, branch integration,
push or live user fixtures were part of the independent review. Main remains
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`.
