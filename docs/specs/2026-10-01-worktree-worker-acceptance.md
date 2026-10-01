# Assembled worktree-worker acceptance — 2026-10-01

Implementation branch: `work/worktree-worker-workflow`. Build baseline:
`adb574093a80421caabee73945424246ed3dc568` (tickets 01–14 independently cleared),
plus the ticket15 commit containing this report. The only production changes in
this gate remove an empty shell-activity separator and use singular `1 project`.

The assembled release scenarios and complete frozen-candidate suite pass on Linux.
Execution and review/fix receipts are recorded below; the final independent review
receipt follows in ticket15. This is implementation/verification
evidence, **not approval to ship**. Native macOS, Linux ARM64 and real third-party
agent service runs are not represented by these fixtures. Memnoc has not accepted
those missing native-release observations. The project repository's `main` remains
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`. No push, tag, publishing, deployment,
or live-user fixture is part of this gate; fixture branches are deliberately merged.

## Environment and reproducibility

Executed on Linux `7.1.5-76070105-generic`, x86_64; tmux 3.4; Git 2.43.0;
Python 3.12.3; Bash 5.2.21; Rust/Cargo 1.95.0. All runtime mutation is confined to
throwaway repositories and private tmux sockets under `/tmp`. Two real tmux
clients attach through PTYs. Initial/ad hoc assembled fixture shells explicitly use Bash without profiles or rc files. Assembled Git/tmux operations are real; individual failure probes inject only controlled failures/timing. Test actors make
known fixture commits and conflict edits; the supervisor never reads those
contents. Fake agents are verified copies of `cat` or existing controlled
sleep/Python receivers, never a PATH-selected installed Codex service.

From the repository root, with dependencies installed:

```sh
cargo fmt --check
cargo test --locked
bash tests/run.sh
git diff --check
```

`tests/run.sh` finishes by building `cargo build --release --locked` and running
`python3 tests/assembled_workflow_test.py`. To repeat only the combined flow:

```sh
cargo build --release --locked
python3 tests/assembled_workflow_test.py
```

The assembled harness uses the existing real-client fixture helpers. It confirms
`scripts/v2.sh` executes `target/release/tmux-drudwyn`, then loads the actual
`tmux-drudwyn.tmux` entrypoint on its private server without a binary override.
Installed status scripts are checked against actual client ANSI cells. Other
interaction suites deliberately select the freshly built debug binary through
the existing test-only override; they do not reload the user's server.

## User-story verdicts

Each story below is copied verbatim from the approved spec (continuation lines
joined). `pass (Linux)` means execution observed the stated behavior in this
environment; it is not a claim of native macOS acceptance. The final column
identifies the public command/rendered-input evidence in `tests/`.

| # | User story (verbatim) | Verdict | Execution evidence |
| --- | --- | --- | --- |
| 1 | As a user, I want a short worker name independent of its task, so that a detailed instruction does not become an unreadable branch or window name. | pass (Linux) | Worker launch form/task tests; assembled alpha/beta/gamma names and work/* branches. |
| 2 | As a user, I want to choose a source branch/ref and inspect its resolved commit, so that workers receive the intended instructions and shared base. | pass (Linux) | Batch source/ref previews, expected-commit confirmation and stale-source refusal; assembled pinned SHA. |
| 3 | As a user, I want sibling workers to retain that starting commit even after I navigate or integrate another worker, so that accidental dependencies do not appear between independent tasks. | pass (Linux) | Batch tests move source refs between launches; assembled three sibling HEADs equal the pinned SHA. |
| 4 | As a user, I want to choose direct-to-base integration or an integration branch once per live batch, so that both workflows are supported and the destination remains visible. | pass (Linux) | Both assembled runs; batch setup/reuse/collision and destination visibility tests. |
| 5 | As a user, I want a comfortable multiline task editor or an explicit task-file reference, so that launch requires no follow-up task paste in normal use. | pass (Linux) | Multiline launch UI and raw receiver tests; assembled multiline stdin and task-file launch/recovery. |
| 6 | As a user, I want launch, delivery, process exit, and agent activity to have distinct evidence, so that a closed window or successful exit cannot be mistaken for completed work. | pass (Linux) | Launch15, activity39 and status exit/attention cases; assembled retained exited beta and fresh receiver. |
| 7 | As a user, I want failed launches and recovery to preserve files and commits, so that partial work survives a terminal or agent failure. | pass (Linux) | Startup failure and recovery preservation checks; assembled beta commit survives stop/restart. |
| 8 | As a user, I want a stable, visibly named coordinator and identifiable worktree/ordinary-agent/shell windows, so that I can return to the right place. | pass (Linux) | Coordinator identity/navigation tests; assembled same coordinator window survives launch, conflict and cleanup. |
| 9 | As a user with two terminals, I want navigation and session creation in one to leave the other's selection alone, so that both terminals remain useful. | pass (Linux) | Two actual PTY-attached clients in navigation/status and both assembled runs; requester-only New Session and Cockpit Open. |
| 10 | As a user, I want a New Session command and session-navigator action with a name and directory, so that I can open a shell for editing or other work. | pass (Linux) | Session command and navigator keyboard tests cover directory/default, cancellation and invalid inputs; assembled New Session. |
| 11 | As a user, I want to reopen an existing worktree without creating a duplicate branch, and deliberately restart or resume its agent, so that recovery does not pretend to restore a lost task or conversation. | pass (Linux) | Recovery tests cover shell/restart/reference/resume availability and metadata loss; assembled deliberate task-file restart. |
| 12 | As a user, I want a two-row status bar with readable spacing and configurable tab density, so that selection, identity, and attention remain legible. | pass (Linux) | Installed two-row status decoded from real client ANSI at 48/64/80/120/160, 2 icon modes, 3 themes; cap/settings tests. |
| 13 | As a user, I want persistent attention counts for hidden workers and a route to their matching Cockpit view, so that a small bar does not hide urgent work. | pass (Linux) | Hidden attention, overlap, mouse and keyboard routes, off-screen workers and membership-race tests. |
| 14 | As a user supervising dozens of agents, I want a global inventory with project and attention grouping, search, filters, and full selected-worker details, so that every worker is reachable without relying on tabs. | pass (Linux) | 36 workers / 4 projects / 39 checkouts; global filters/search/grouping/details and full terminal matrix. |
| 15 | As a user, I want inspection to differ from opening a window, stable selection during refresh, and a clear unavailable state for a disappeared target, so that an update does not act on the wrong worker or lose my place. | pass (Linux) | Cockpit inspect/Open, selected-ID preservation and vanished-target tests; assembled inspect leaves both clients unchanged. |
| 16 | As the supervisor, I need to deduplicate linked windows and preserve evidence provenance, so that counts and activity labels remain truthful across views. | pass (Linux) | Linked-client inventory deduplication and lifecycle process-birth/provenance tests; global/status counts reconcile. |
| 17 | As a user, I want to inspect source and destination commits and initiate integration from Cockpit, so that merging reviewed work is a built-in action. | pass (Linux) | Integration20 includes actual Cockpit preview/cancel/apply and stale IDs/refs; assembled reviewed integration. |
| 18 | As a coordinator, I want conflicts handed to my agent in the destination checkout with visible Continue/Abort actions, so that resolution remains deliberate and does not require the user to type Git plumbing. | pass (Linux) | Conflict17 covers handoff delivery/retry/recovery, real UI Continue/Abort and replaced operations; both assembled conflicts. |
| 19 | As a user, I want reported worker checks separated from checks actually run on the assembled checkout, so that branch-level success is not presented as verified integration. | pass (Linux) | Verification10 covers actual visible terminal execution, failure/staleness/missing receipts; assembled untracked-input failure/rerun. |
| 20 | As a user, I want explicit promotion of an integration branch into the base and guarded cleanup, so that finishing workers does not silently ship or remove work. | pass (Linux) | Promotion/cleanup15, real Cockpit P/f, lock/writer/concurrency safeguards; assembled explicit promotion and branch-preserving cleanup. |
| 21 | As a user, I want understandable errors and cancellation for invalid names, collisions, dirty targets, stale refs, and unavailable tools, so that retrying does not reset an existing checkout or duplicate an action. | pass (Linux) | All command/UI suites include cancellation, collisions, dirty/stale/ambiguous/missing resources and bounded retry failures. |
| 22 | As a user, I want the existing content-blind and label-redaction behavior preserved, so that improved supervision does not create a conversation store. | pass (Linux) | Privacy and lifecycle suites, transient delivery checks, metadata-loss cases, full redaction matrix; no production content inspection added. |

## Boundaries crossed by the assembled scenarios

Both destination flows perform these steps in one disposable project/server:

1. Create an ad hoc shell session through Drudwyn, preserving the second client's
   selected window. Return to the original coordinator and confirm the batch's
   source and destination commits before setup.
2. Launch alpha, beta and gamma from the same pinned commit, with independent
   short names/branches and multiline task or deliberate task-file delivery.
   Verify their process executable identities, delivery receipts and coordinator
   identity. Make independent commits; alpha and beta change the same file.
3. Navigate both attached clients independently. Inspect beta in the real Cockpit
   without navigation, then Open it for only the requesting client. End beta's
   controlled process, retain its exited pane, and recover through Drudwyn with a
   deliberately selected task file. Assert the old pane and commit survive.
4. Review and integrate alpha and gamma. Integrating beta retains a real Git
   conflict and sends a transient handoff to the existing coordinator in the
   destination checkout. Abort through Drudwyn, preview/apply again, resolve as
   the controlled coordinator actor, then Continue. Assert all three source
   commits are ancestors of the selected destination and files are combined.
5. Select a visible check explicitly. An untracked application input makes it
   fail; remove the input and rerun successfully. The check script/output is not
   retained in live options. The integration-branch variant additionally proves
   main is unchanged until explicit promotion; main remains Not verified until
   its own selected check is run.
6. Finish refuses an integrated worker while its agent is live. Explicitly stop
   the source agents and review/apply guarded cleanup. Checkouts and their exited
   worker windows disappear; branches and coordinator remain. No remote exists.
   Capture the installed status rows from the actual client terminal.

This sequence complements failure-path suites spanning launch/delivery/process
identity, metadata loss/recovery, selected-client/linked-session routing,
review-token/Git mutation, conflict receipt/coordinator transmission,
check-runner/inode locks, and cleanup/process/window ownership. Their malicious
or stalled children and injected command failures remain local test actors.

## Final execution receipts

The final corrected-candidate full suite exited successfully. No production or
test edits followed this frozen run. The first full run exited
successfully but was superseded because a scoped cosmetic regression was found:
a long plain-shell/coordinator name could leave the selected context blank at
64/80 columns. Five combinations failed in
`/tmp/drudwyn-ticket15-long-context-red.log`. A fresh debug/fix session corrected
the bounded role/name fallback, then re-executed 20 role/width/redaction
combinations against both the public projection and installed terminal cells.
Both context tests passed (5.572s); the retained exit-attention test also passed
(2.350s). Green logs: `/tmp/drudwyn-ticket15-long-context-green.log` and
`/tmp/drudwyn-ticket15-context-exit.log`. Story12 is pass after that observed
correction, not silently reclassified. The original suite log is explicitly
superseded: `/tmp/drudwyn-ticket15-full-superseded.log`.

Final release assembly passed: 2 scenarios, 24.641s (direct 11.596s,
integration/promotion 11.906s). The earlier focused 2-scenario run passed in
24.618s. `cargo fmt --check`, `cargo test --locked` (41 passed),
`cargo build --release --locked`, and `git diff --check` passed.
The complete shell/Python suite passed: 212 Python cases, 211 passed and one
existing optional Resurrect skip. Case counts are batch UI: 1, launch: 15, recovery: 17, integration: 20,
conflict: 17, verification: 10, promotion/cleanup: 15, global Cockpit: 11,
status: 13, activity: 39, navigator: 13 (one skip), independent navigation: 29,
settings: 10, assembled: 2. Existing shell syntax, status, lifecycle,
worktree, legacy fallback, help, privacy, package and release-workflow gates
also passed.

Exercised local release-binary SHA-256:
`6b31b484e7ae62bfacd422ad9aa25e5abb4b9fd5e0666736b5758ccdd20ac79d`.
This identifies the tested local executable, not a published release archive. The cosmetic tests were first observed failing on the baseline,
then passed with the scoped fixes. Their logs are
`/tmp/drudwyn-ticket15-{shell,project}-{red,green}.log`.

Frozen-run logs:

- `/tmp/drudwyn-ticket15-rust-final.log`
- `/tmp/drudwyn-ticket15-full-final.log`
- `/tmp/drudwyn-ticket15-release-build.log`
- `/tmp/drudwyn-ticket15-assembled-focused.log`

The committed report retains verdicts and measurements; temporary raw logs and
terminal artifacts are reproducible fixtures, not product logs or a content
store. The full suite also executes existing shell syntax, lifecycle, legacy
v1 fallback, help/options, privacy, packaging and release-workflow checks.

## Actual terminal review and performance

Actual captures were reviewed across all 30 status combinations and 60 Cockpit
combinations, with full-frame samples at every width and narrow redacted/details
views. At 48/64 columns the selected tab remains visible with separated name,
role and padded attention badge; wider terminals retain two, three or four tabs
as space permits. GLOBAL failure/input/review labels remain visible in both
clients. Cockpit retains worker name/role/activity and moves optional fields into
scrollable details; label redaction preserves stable IDs and actions. The corrected
long-shell context retains SH/COORD and a bounded selected name, without an empty
separator or invented unknown activity. ANSI/SGR cell assertions validate colors,
selected backgrounds and Unicode width; these are actual tmux terminal outputs.
Corrected-candidate measurements (same disposable 36-worker / four-project /
39-checkout fixtures):

| Measurement | Observed time |
| --- | --- |
| Global metadata snapshot | 1167 ms |
| Global CLI wall time | 1.181 s |
| Actual Cockpit ready | 1.130 s |
| End-key inspection of last worker | 6 ms |
| Details inspection during deliberately gated refresh | 30 ms |
| Two simultaneous fresh status renders | 1.767 s wall (1.764 s / 0.989 s individually) |
| Both installed status rows ready | 0.990 s including plugin install |

These are local samples, not a production SLO or a universal responsiveness
claim. Ticket09's prior real fixture measured 1147 ms / 1.161 s CLI / 1.110 s UI;
the current discovery/UI result is comparable. Ticket10's prior two-client
fixture measured 1.775 s fresh render wall and 0.994 s installed-row readiness; the corrected candidate shows no material regression in this sample.
No browser timings are used. Installed rows reuse observer metadata and keep
per-cell Git probes bounded; the suite exercises stale/error cases rather than
accepting old snapshots as current. No new refresh tuning was introduced.

The status
matrix decodes attached-client ANSI/SGR into terminal cells for all 30
width/icon/theme combinations, checking selected backgrounds, badge spacing,
text labels and independent client views. Cockpit exercises the five widths,
both icon modes, all three themes and redaction both off/on (60 combinations),
with scrolled full details, rather than a browser simulation. The existing
36-worker fixtures cover four projects and 39 unique checkouts, duplicate names,
global totals, End inspection latency, and two simultaneous status projections.

Artifacts: `/tmp/drudwyn-ticket10-{safe,nerd}-{moon,dawn,rose-pine}-{48,64,80,120,160}.{txt,ansi}`;
`/tmp/drudwyn-ticket09-{safe,nerd}-{moon,dawn,rose-pine}-{48,64,80,120,160}-redact{0,1}.txt`
and corresponding `details` / `details-end` captures;
`/tmp/drudwyn-ticket09-global36-120.txt`;
`/tmp/drudwyn-ticket10-scale-{64,160}.txt`;
`/tmp/drudwyn-ticket15-{direct,integration}.{txt,ansi}`.
Additional shell fallback artifacts: `/tmp/drudwyn-ticket15-long-{SH,COORD}-{False,True}-{48,64,80,120,160}.{txt,ansi}`. The shell correction is visible in `/tmp/drudwyn-ticket15-shell.{txt,ansi}`:
`COORD zsh · main` has no empty activity segment.

## Documentation, privacy, compatibility, and release boundaries

Reviewed [usage](../usage.md), [configuration](../configuration.md),
[privacy](../privacy.md), [installation](../installation.md),
[contributing](../../CONTRIBUTING.md), CLI/help checks, and the
[release preflight](../preflight-checklist.md). Source/destination selection,
live-only metadata, transient tasks, explicit verification/promotion/Finish,
label redaction and the legacy content-reading exception remain documented.
The legacy mode is tested for compatibility, not claimed to be content-blind or
to have parity with the new workflow. No registry, task history, diff/scrollback
inspection, implicit push/deployment, or branch deletion was added by this gate.

| Boundary | Verdict | Reason / remaining evidence |
| --- | --- | --- |
| Linux x86_64 real command/plugin/tmux assembled behavior | pass | Executed as above, including release binary. |
| Native macOS x86_64 / ARM64 runtime | unverifiable | No native macOS host in this session. Platform-specific process/lsof/ps behavior has not been executed natively. |
| Linux ARM64 runtime | unverifiable | No ARM64 runtime host in this session. |
| Real Codex/Claude/OpenCode service interactions | unverifiable | Deterministic fake agent fixtures prove supervisor routing; no real third-party credentials/session was used. |
| GitHub native release artifacts/publication | unverifiable | Local packaging/workflow checks do not substitute for the manual four-platform workflow and artifact-install gate. Nothing was published. |
| Optional local Resurrect save integration | unverifiable | Existing optional test skips when its external fixture/plugin is unavailable; ordinary navigator tests still run. |

Content-blind process metadata can veto observable writers but cannot certify
absence of all external processes, unreadable cwd metadata, future mutations or
ignored/external build inputs. Verification is a receipt for selected checks at
an observed revision/environment, not deployment or eternal validity. On native
non-Linux hosts the previously recorded conservative source-shell Finish veto
remains unverified; no native behavior claim or new platform regression is inferred.

## Independent review

The root-coordinated independent audit reproduced the narrow shell-context
failure and independently executed the direct-to-base release scenario
(`/tmp/drudwyn-ticket15-independent-direct.log`). The builder executed both
release scenarios in the final suite. A fresh
fix session reproduced all five failing combinations and supplied the minimal
fallback correction and 20-combination regression. The final frozen suite above
re-executed that correction. The final independent Standards/Spec receipt follows
in [ticket15](../../.scratch/worktree-worker-workflow/15-assembled-hardening.md).
No known runtime failure remains in the executed Linux scope. Confirmed failures
were routed to a separate debug/fix session and retained in this report, not
silently fixed inline or relabelled. Prior review judgments carried forward:
ticket 13 receipt strings (possible Primitive Obsession) and ticket 14 repeated
shell classification (possible Duplicated Code), both nonblocking maintenance
observations. Ticket 15 completion and
shipment are separate: only explicit acceptance of the remaining unverifiables
or new native evidence can satisfy the release decision.
