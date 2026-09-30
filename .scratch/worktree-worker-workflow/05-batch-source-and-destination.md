# 05 — Choose the batch source and integration destination

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done

**What to build:** Batch setup pins a worker source commit and asks once whether
to integrate directly into the configured base or a named integration branch.

**Blocked by:** 03.
**Priority:** P1. **Stories:** 2, 3, 4, 21, 22.

- [x] Preview the source ref and resolved commit, including configured-base,
  current-branch, and explicit locally available ref choices; do not fetch implicitly.
- [x] Retain the pinned source for sibling launches despite navigation, target
  merges, or later source-ref movement; changing it requires deliberate selection.
- [x] Show the base's actual configured name, or collect an integration-branch
  name and starting point; keep the selected destination visible for the live batch.
- [x] Locate an appropriate destination checkout or offer a dedicated one;
  preserve the coordinator's original branch and never reset an existing branch.
- [x] Invalid refs, name/path collisions, dirty source expectations, cancellation,
  and metadata loss have explicit outcomes; source and destination are distinct.
- [x] Multiple batches for one repository retain independent associations;
  metadata is live-only and a missing association is unknown rather than guessed.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.


## Implementation receipt — 2026-09-30

Implemented from `20016c7` on `work/worktree-worker-workflow`.

- Added `batch setup`, `batch show`, and `batch select` commands. Setup previews
  local source ref/commit, destination branch/starting commit, and checkout;
  confirmation requires the two reviewed commits and revalidates the preview.
  Configured-base, current-branch, and explicit local revision choices remain
  separate from the destination's starting point. No fetch is performed.
- Added Cockpit batch setup (`b`, first New workspace, or F3 from the start
  form), with separate source, integration branch, destination start, and
  dedicated checkout fields. Enter previews, a second Enter confirms, and Esc
  cancels without allocating. Existing integration branches require explicit
  reuse selection. Pinned source and selected destination remain visible in
  the start form and selected-workspace details, including at 100x24.
- Stored independent immutable batch records in project-session tmux options;
  window associations select an explicit record. Sibling launches reuse the
  pinned commit after source movement, target merges, and navigation into a
  sibling. Missing metadata is unknown, never inferred from the repository.
  Existing unassociated command launches remain available for compatibility.
- Reused appropriate destination checkouts or created explicitly requested
  dedicated checkouts without switching the coordinator branch or resetting
  existing branches. Invalid refs/names, collisions, stale previews, dirty
  targets, and dirty-source expectations have explicit outcomes. Labels and
  setup errors honor redaction. No task content or durable registry was added.
- Updated usage and privacy documentation. Task editing/delivery expansion,
  integration/merging, promotion, verification, and recovery remain later tickets.

Validation:

- Red command regression: `tests/batch_test.sh` failed at the missing `batch`
  command before implementation. Green against disposable real Git and tmux:
  direct and integration destinations; independent destination start; three
  pinned siblings after ref movement and a real target merge; launch from a
  sibling checkout; independent batches; explicit local tag; dirty-source
  warning/file non-inheritance; invalid refs/names; occupied paths/branches;
  stale commit confirmation; existing checkout selection; metadata loss; and
  redaction. Fixtures did not use the user's live tmux server.
- UI regression red before implementation, then green at the rendered/key
  interface. `tests/batch_ui_test.py` exercised real Cockpit keystrokes against
  an isolated tmux server: preview, cancellation, confirmation, preservation
  of the planning branch, and retained destination on reopening the form.
- Final review added a failing regression for the old misleading base-choice
  wording on a pinned batch. Corrected the label and available actions; the
  normal-height destination visibility regression also passes.
- Final `cargo fmt --check`, `cargo test --locked` (38 tests), and full
  `bash tests/run.sh` passed. The full suite includes the new batch checks,
  29 real-client navigation/session tests, 13 navigator tests (one existing
  skip), 10 settings tests, lifecycle/start-failure/privacy/packaging checks.
  A sandbox-only first attempt could not create a local tmux socket; authorized
  isolated-server runs passed. The full suite was repeated only after the final
  pinned-source wording correction. `git diff --check` passed.

Builder Northstar check:

- Standards: reviewed against CONTRIBUTING, privacy boundary, CONTEXT, and
  ADRs 0002/0003/0005/0007. The new module owns batch preview/selection and
  live records; Git/tmux remain authoritative. No external service, content
  inspection, persistent registry, branch reset, or new dependency was added.
- Spec: checked all six ticket criteria against command, rendered UI, and real
  tmux evidence. Source and destination are distinct, batch IDs are independent,
  coordinator branch and client-local navigation are preserved, and missing
  associations stay unknown. Destination commits are labelled at setup rather
  than claimed as current merge or verification evidence.
- Per the coordinating session's authorized workflow, the independent Northstar
  crosscheck follows this atomic implementation commit. This receipt does not
  claim that independent review is already complete.

Limits: batches are live-only and scoped to the connected tmux project session.
The configured direct destination must exist locally; setup explains missing
refs/checkouts rather than fetching or guessing. Creating a new batch is the
explicit route to change source/destination. Setup does not integrate or verify
work. `main` remains `eaf2446`.
