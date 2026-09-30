# Worktree worker workflow: field report intake

Date: 2026-09-30. Owner and reporter: Memnoc.
Status: worker/session/integration decisions recorded in ADR-0005; status-bar
variant A, persistent attention, configurable density, and spacing requirements
recorded in ADR-0006. The
[specification](../specs/2026-09-30-worktree-worker-workflow.md) is written;
the [implementation breakdown](../specs/2026-09-30-worktree-worker-ticket-plan.md)
is approved and published as executable tickets.

A [global overview extension](2026-09-30-global-worker-overview.md) for dozens
of workers and its surface role split are approved in
[ADR-0007](../adr/0007-global-cockpit-and-navigation-roles.md), using the same
status vocabulary and a shared fixture in the visual spike.
Priorities and sequencing below are recommendations, not approved
implementation tickets.

## Sources and evidence limits

- **S1 — Direct feedback:** items 1–5 and the introductory paragraphs in
  [Worktree session notes](../QA-notes/Worktree-session-notes.md), supplied by
  Memnoc after using Drudwyn in `~/Code/memnoc-dev`.
- **S2 — Field report:** the article in that same file, dated 2026-09-30,
  drafted with agent assistance. References below use its section titles.
  The saved source includes two copies of the article; treat them as one
  account, not independent corroboration. Preserve the source as supplied.
- **S3 — Current implementation:** inspected in this working checkout on
  2026-09-30. Relevant source links accompany findings below. The checkout
  already has pending lifecycle, status-bar, test, and documentation edits.
  Those changes predate this intake and are not fixes delivered by it.
- **S4 — Existing product decisions:** [domain context](../../CONTEXT.md),
  [ADRs](../adr/README.md), [daily use](../usage.md), and the existing
  [unfinished-branch recovery agenda](../agenda.md).
- **S5 — tmux behavior:** installed tmux 3.4 manual and the
  [upstream manual](https://man.openbsd.org/tmux.1#new-session). A disposable
  server experiment on 2026-09-30 verified that two grouped sessions share
  window IDs while selecting `plan` and `worker` independently. This verifies
  a building block, not Drudwyn's navigation with two attached clients.
- **S6 — Follow-up request:** Memnoc, 2026-09-30, in the planning conversation:
  “Add a task to include the spawining of a new session as a command in Drudwyn,
  much like we are able to kill we should also be able to spawn.”
- **S7 — First design round:** Memnoc, 2026-09-30, accepted a common session
  “as long as there is way to tell that” and an action to spawn another session
  for shell work; selected Integrate; and preferred “preserving the stateless
  design for now.” See [ADR-0005](../adr/0005-project-sessions-and-explicit-integration.md).
- **S8 — Second design round:** Memnoc, 2026-09-30, confirmed “the coordinator
  agent steps in and handles the conflicts” and agreed to defer enforced port
  reservations. Memnoc requested an explanation of planning branches versus
  `main`; S9 resolves that choice.
- **S9 — Integration destination choice:** Memnoc, 2026-09-30, requested that
  Drudwyn ask whether to create a merging branch or integrate into `main`,
  because both use cases should be supported. The canonical term for the
  optional merging branch is Integration branch.
- **S10 — Status feedback follow-up:** Memnoc, 2026-09-30, requested a redesign
  of status-bar appearance, worktree identity, alerts, and progress feedback.
  See the [status feedback intake](2026-09-30-status-feedback.md) for the quoted
  feedback, implementation findings, and open design questions.

The website's history and worker receipts are reported evidence, not independently
audited here. S2 reports local integration at `38b197e`, with assembled-system
hardening, main-branch integration, push, deployment, remote workflow execution,
and cleanup still outstanding. Worker checks are not assembled-system checks.
No measured parallel speedup was established. No branch or file loss was
established; window disappearance and early agent exit have unknown causes.

## Stated requirements

| ID | Requirement in the reporter's words | Source |
| --- | --- | --- |
| R1 | “not enough space to type in the Task field” | S1, 1A |
| R2 | “no visual indicator that a worker in a tree not just another agent” | S1, 1B; explicitly names `prefix + w` and `prefix + s` |
| R3 | “launch a new worker, select the branch, give it a task, start it and that's it” | S1, 2 |
| R4 | “observe the progression from the Cockpit and if needed interact with it when it tells you too” | S1, 2 |
| R5 | “constantly losing the original branch and tmux window when switching workers” | S1, 3; S2, “I repeatedly lost the planning window” |
| R6 | “inherits consistently the name of the task or branch or tree as the name of the session or window” | S1, 4; repeated `zsh` labels are the symptom |
| R7 | “when I switch a window or session there it mirrors the other terminal” | S1, 5 |
| R8 | “make that a visual thing in the session, work or cockpit” | S1, positive feedback about the completion handoff |
| R9 | “That should not be a manual thing to do” | S1, opening shells to issue Git merge commands |
| R10 | “a short workspace name to be separate from the instruction sent to the agent” | S2, “What this makes me want to improve in Drudwyn” |
| R11 | “reopening that worktree should be an obvious action” | S2, same section |
| R12 | “did the process start, did it exit, and is there a session I can reopen?” | S2, same section |
| R13 | “A deliberate ticket reference could provide a recoverable starting point” | S2, same section; a proposed solution, not an approved storage design |
| R14 | “common collisions to be visible earlier” | S2, same section; shared test port and package scripts |
| R15 | “keep ‘worker finished,’ ‘changes merged,’ and ‘assembled site verified’ as separate states” | S2, same section |
| R16 | “much like we are able to kill we should also be able to spawn” | S6; a new tmux session command in Drudwyn |
| R17 | “do you want to spawn a merging branch or let's do it in main?” | S9; explicitly offer both integration destinations |
| R18 | “the alert system and the progress system representation in the status bar does not satisfy the eye nor the functional feedback” | S10; redesign identity, activity, and attention presentation |

## Implied requirements

These are inferences to check during design, not additional user commitments.

- **Stable project context:** launching another worker should use the chosen
  project and planning ref without requiring navigation back to the original
  checkout first. S2, “Choosing the correct base was a real decision.”
- **Explicit launch outcome:** worktree creation, process startup, task
  submission, and successful work are different events. A paste operation
  succeeding alone cannot prove an agent accepted or completed a task.
  S2, “The first launch did not produce a completed task.”
- **Recovery preserves work:** an absent terminal must not imply that its
  worktree should be discarded. Reopening should distinguish a fresh agent
  from a resumed conversation. S2, “Recovering the workspace did not recover
  the task.”
- **Usable identity without color:** a text or symbol marker should distinguish
  a linked worktree while lifecycle colors retain their existing meaning.
  S1, 1B; S4, ADR-0004.
- **Integration has a destination:** workers may integrate into a selected
  planning branch, not necessarily `main`; launch ref and integration target
  are separate concepts. S2, planning and integration sections.
- **Conflicts are a supported state:** expected overlaps need a resolution
  handoff that preserves both features, followed by assembled verification.
  S2, “Independent tasks still touched the same file” and integration section.
- **Local environment participates in verification:** untracked build inputs
  and shared ports must not disappear from the acceptance scenario merely
  because worker branches passed. S2, shared-port and notes/build sections.

## Triage and recommended order

Priority expresses user impact, not a finding of data loss. Items are work
areas for the interview; a spec must precede executable tickets.

| ID | Priority and kind | Evidence and current status | Verifiable outcome to design toward |
| --- | --- | --- | --- |
| W1 | P0: investigate planning-window loss | S1, 3; S2 reports three recovery episodes. Cockpit discovery excludes ordinary shells, while the workspace navigator has a separate all-window inventory. Filtering is confirmed; the historical cause is not. | Starting and switching among three workers preserves the original window ID, checkout branch, and a visible return route. If the terminal closes, its checkout remains recoverable. |
| W2 | P0: investigate launch/exit and task delivery | S2 reports a surviving worktree with no implementation after the agent window closed. Current launch checks existence after 150 ms; delivery pastes a transient buffer and sends Enter after a delay. Neither establishes task completion. | Delayed startup, early exit, submission failure, and recovery each produce a truthful visible outcome and preserve user work. |
| W3 | P1: independent terminal navigation | S1, 5. Shared selected windows are normal for clients on one tmux session. Grouped-session independence verified in isolation; session-switch mirroring and Drudwyn client targeting remain unverified. | Two attached clients can select different workers without moving the other client, including through both navigators and cockpit actions. |
| W4 | P1: simple worker launch | S1, 1A/2; S2, base choice and long branch name. Task-to-slug coupling and single-line ellipsized input confirmed in the cockpit. | Enter a short name, choose the starting branch/ref, edit a full task comfortably, and launch from a preview of the exact starting commit. Three workers can share the intended planning commit. |
| W5 | P1: identity and return path | S1, 1B/4; S2, repeated planning-window recovery. Navigator records have branch metadata but no linked-worktree field. Session navigator has only session-level counts and identity. | Worker and coordinator identities remain recognizable across both navigators, cockpit, and status; explicit user names remain usable. |
| W6 | P1: recover an existing worktree | S2, recovery sections. Current workspace discovery starts from live tmux windows, not a repository's complete worktree inventory. | Find an existing worktree with no live terminal, reopen it without creating a duplicate branch, and deliberately provide a task/reference or select a supported resume path. |
| W7 | P1: review, integrate, verify, clean up | S1, completion handoff and merge complaint; S2, integration and stopping point. Current Finish removes a clean, integrated worktree; it does not perform integration. | Inspect source and destination, initiate integration from the cockpit, handle conflicts through a visible handoff, distinguish reported checks from assembled verification, and clean up only eligible work. |
| W8 | P2: shared-resource coordination | S2, port 4321 reservations and package-script overlap. The session used a conversational convention, not an enforced lock. Product mechanism and initial scope remain open. | Make declared shared resources and overlap visible; if locking is selected, define contention, release, and abandoned-worker behavior. |
| W9 | P1: create a tmux session from Drudwyn | S6 explicitly requests creation alongside existing kill functionality. The session navigator currently offers switch, rename, kill, save, and filter. | Expose a New Session command/action from the session navigator. Proposed defaults: enter a name and starting directory, open a shell, and switch only the requesting terminal. Handle duplicate names and invalid directories visibly; cancel without creating anything or changing existing sessions. |
| W10 | P1: redesign status and alerts | S10; expands W5. The current bar lacks worktree identity and explicit counts for hidden attention; process presence alone is presented as working. | Visually distinguish selected window, worktree identity, reported activity, and actionable attention at every supported width; keep integration and verification distinct from agent progress. |
| W11 | P1: global worker overview | Memnoc's follow-up requests a global view for dozens of agents; see the global overview intake. Existing Cockpit reads across sessions but needs grouping and clearer scope. | Reach every worker from a searchable global inventory, preserve status-bar identity and state labels, expose project and attention filters, show full details and coordinator actions, and deduplicate linked windows. |

Recommended sequence: reproduce W1–W3 while designing the common worker model;
then launch, identity/status/global overview, and session creation (W4–W5/W9–W11), recovery (W2/W6), and
integration (W7).
Independent terminal navigation must inform the model early, rather than be
bolted onto session targeting later. Enforced port reservations in W8 are
deferred to a follow-up by S8. The
existing unfinished-branch recovery agenda remains valid and should reuse the
chosen branch selection, integration, and recovery concepts.

## Implementation evidence

- [Cockpit](../../src/cockpit.rs): the task input handler calls `slug` on the
  whole task, resolves configured-base/current-branch choices, and calls
  `start` then `deliver_task`. Rendering ellipsizes the task on one line.
- [Workspace lifecycle](../../src/workspace.rs): launch supplies an explicit
  window name derived from the branch. A process is the window command;
  startup failure invokes rollback. Task delivery uses a transient tmux buffer.
  Finish checks integration and removes the worktree; it has no merge action.
  Launch cleanup deserves reproduction with a process that writes before
  exiting, since rollback uses forced removal. No loss is claimed from S2.
- [Discovery](../../src/discovery.rs): `parse_windows` retains recognized
  agents or known lifecycle states, excluding ordinary shell windows. Git
  reconciliation already derives linked-worktree status for retained entries.
- [Workspace navigator](../../src/navigator.rs): inventories all tmux windows,
  groups by agent/lifecycle classification, and switches via a session name
  followed by a window ID. It does not carry linked-worktree identity.
- [Session navigator](../../src/session_navigator.rs): inventories session
  names, window counts, attached-client counts, and IDs. It switches clients
  to session names. Grouped sessions would require deliberate presentation and
  targeting to avoid duplicate worker entries or reunifying independent views.

The pending [split-pane regression test](../../tests/lifecycle_test.sh) and
status-badge edits address related lifecycle visibility. They need their own
verification/review; do not infer that they resolve W1 or W2.

## Decisions already made and constraints

- Memnoc wants the basic launch → observe → interact → integrate workflow
  inside Drudwyn, with less shell plumbing. S1. Exact interactions remain open.
- A New Session command belongs in the planned work. S6. Name/directory input
  and switching only the requesting terminal are recommended behavior. S7
  confirms the need for ad hoc shell/editing sessions alongside the shared
  project session. Creating a session does not by itself require creating a
  Git worktree or starting an agent.
- [ADR-0005](../adr/0005-project-sessions-and-explicit-integration.md) records
  S7: clearly identify the common project session, use an explicit Integrate
  action, and retain stateless task associations for now. Durable associations
  may be reconsidered later; they are not part of this design.
- The existing completion handoff is useful and should remain accessible.
  S1. Its mechanism was not independently identified in this intake.
- Preserve Git worktree isolation and inspectable starting refs/commits.
  S2 reports that these worked, while selection and coordination were awkward.
- [ADR-0002](../adr/0002-rust-stateless-v2-architecture.md) establishes a
  local, stateless, content-blind supervisor. Prompt history, persistent task
  storage, and inferred progress are not silently authorized by this report.
- [ADR-0003](../adr/0003-keep-review-tool-agnostic.md) leaves review tools to
  the user. A cockpit integration action need not prescribe a Git UI.
- [ADR-0004](../adr/0004-clustered-status-and-grouped-navigation.md) separates
  agent identity and lifecycle state. A worktree marker adds a different fact.

## Glossary candidates

Project session, coordinator workspace, and Integrate are now recorded in
`CONTEXT.md` following S7. The remaining distinctions below still need design
work; the coordinator's exact ownership and destination are not yet settled.

| Term | Proposed meaning | Ambiguity to resolve |
| --- | --- | --- |
| Worker | An agent assigned a task in a selected checkout, usually a linked worktree | Can a worker exist without a live terminal? Is a primary-checkout agent also a worker? |
| Coordinator workspace | The place used to plan, receive handoffs, and integrate workers | One per repository or one per batch of tasks? Must it be the primary checkout? |
| Starting point | The exact commit a new worker starts from | Separate the source branch from the newly created worker branch |
| Integration target | The branch/checkout that receives worker commits | May differ from both the launch base and configured `main` |
| Terminal view | One client's independently selected view of shared workers | A tmux session group is a possible implementation, not a decided product concept |
| Ready for review | An explicit handoff or agent event requiring review | A turn ending is not evidence that acceptance criteria passed |
| Integrated / verified | Commits included in the chosen target / assembled checks passed | Verification must be associated with the actual revision and environment |

## Contradictions and open questions

1. **Live workspace versus recoverable worker:** the glossary currently defines
   a Workspace as an active tmux window. Recovery requires exposing a checkout
   whose window has gone. Decide whether to extend Workspace or represent a
   Worker and its optional live terminal separately. S2; S4.
2. **Naming versus instructions:** the current task field supplies both prompt
   text and the branch slug. R10 explicitly asks to separate these. The chosen
   short name, editable branch name, and user rename precedence need a policy.
3. **Recovery versus retention — resolved:** task associations survive only
   within live tmux metadata; after its loss, the task/reference is reselected.
   No persistent task registry or prompt history is added. S7; ADR-0005.
4. **Exit versus success:** neither a vanished window, exit code zero, nor a
   finished agent turn proves a ticket is complete. Define the visible states
   and evidence required for handoff and verification. S2; ADR-0002.
5. **Integrated automation versus judgment — owner resolved:** the coordinator
   agent handles merge conflicts, with Continue/Abort in Drudwyn and assembled
   verification separate from completing the merge. S8. Detailed handoff and
   verification behavior remains to be specified.
6. **Window versus session mirroring:** normal shared-session window selection
   explains one part of R7. Switching the wrong client or moving both clients
   between sessions requires a separate reproduction. S1; S5.
7. **Shared resources versus product scope — release scope resolved:** enforced
   port reservations are a follow-up, not part of the first release. S8.
   Shared-file conflicts remain in scope for coordinator-led integration;
   they are not solved by port reservations.

## First decision round — accepted

- **Organization:** one clearly identified project session with coordinator
  and worker windows; a New Session action also supports ad hoc shells and
  editing. Rationale: keep related work together without limiting other work.
- **Integration:** an explicit Integrate action after review. Rationale: remove
  manual shell plumbing while leaving the decision to merge with the user.
- **Task recovery:** repository-owned task files and live associations; no
  durable task registry or prompt history. Rationale: preserve the stateless
  design for now, with the option to reconsider it later.

Memnoc accepted these choices in S7. ADR-0005 is the decision receipt.

## Second decision round — accepted

- **Conflict resolution — accepted:** the coordinator agent handles conflicts
  in the destination checkout, with visible Continue/Abort actions in Drudwyn.
  Continue checks that conflicts are resolved before finishing the merge;
  assembled verification is separate. The supervisor does not inspect
  conversation content to infer success. S8.
- **First-release scope — accepted:** complete launch, identity, independent
  terminals, session creation, recovery, and integration first. Enforced
  shared-port reservations remain a follow-up. S8. Assigning preview ports
  alone does not provide mutual exclusion.
- **Integration destination — accepted:** ask the user whether to integrate
  directly into the configured base (normally `main`) or create an integration
  branch. S9. Both paths are first-class choices; a coordinator window and task
  files do not require a separate planning branch.

### Destination choice in the workflow

Present the choice when setting up a batch of workers:

1. **Merge into main:** reviewed workers integrate directly into the base.
   Display the configured branch's actual name when it is not `main`.
2. **Create an integration branch:** choose a short branch name and inspect
   its starting ref/commit. Workers integrate there; after combined verification,
   a separate explicit action merges that branch into the base.

Keep the choice visible and retain it in live tmux metadata for the batch;
do not ask again for each sibling worker. After metadata is lost, ask again
rather than guessing a previous choice. An explicit destination change must
be visible before it affects a merge. Existing branch-name collisions should
produce an actionable error, not overwrite or reset a branch.

Launch ref/commit and integration destination remain distinct selections. Keep
a batch's chosen source commit stable even as siblings integrate, unless the
user explicitly chooses another source. Creating an integration branch must
preserve existing checkouts and branches. Neither workflow implies pushing or
deploying automatically. Exact UI and command contracts belong in the spec.

## Verification agenda and handoff

Use Northstar `debug` for reported defects: establish a symptom-specific failing
reproduction before assigning a cause or implementing a fix. Build disposable
Git repositories, tmux servers, and controllable fake worker processes; avoid
using the website's real branches as fixtures.

The eventual acceptance scenario should create a named session through Drudwyn,
launch three named workers from one planning commit, preserve the coordinator,
allow two clients to navigate independently, recover an exited worker, and
integrate independent commits with one controlled conflict. Exercise both the
direct-to-base route and the optional integration-branch route, including the
explicit final merge into the base. Then verify the assembled checkout, including an
untracked build input, before cleanup. Include narrow terminals and both icon
modes; preserve content-blind operation throughout. Port-reservation contention
tests belong to the deferred W8 follow-up.

Current result: source and code triage complete; isolated tmux grouped-session
experiment passed. Historical missing-window and launch incidents are not yet
reproduced. No application behavior changed and no fixes are claimed.

Next: implement the [approved ticket breakdown](../specs/2026-09-30-worktree-worker-ticket-plan.md)
on the separate implementation branch. Implementation
uses fresh sessions, `tdd`/`debug`, and `crosscheck`. `harden` must verify the
assembled workflow before it is described as shipped.
