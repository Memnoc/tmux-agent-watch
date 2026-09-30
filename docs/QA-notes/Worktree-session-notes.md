Drudwyn Worktree session notes

1. Worktrees are cluncky to work with overall
    A. not enough space to type in the Task field
    B. no visual indicator that a worker in a tree not just another agent. Maybe a symbol or a different color could work best? From prefix + w or s I cannot tell which process is a tree unless I remember the name

2. There should be something simple like: launch a new worker, select the branch, give it a task, start it and that's it, observe the progression from the Cockpit and if needed interact with it when it tells you too

3. I also seem to be constantly losing the original branch and tmux window when switching workers, not too sure why that happens

4. There should be a feature that inherits consistently the name of the task or branch or tree as the name of the session or window-sometimes all I can see is zsh zsh zsh and I have no idea what is going on until I get there

5. Another super annoying thing is that if I spawn a new terminal, and open tmux, when I switch a window or session there it mirrors the other terminal, so effectively, having two terminals is not good at all, does not help. Why is that?

Overall, it should be much easier to spawn, control, observe and interact with worktree agents/workers than it is now, so we need to work on improving that.

One thing that works well is the send off to the other agent when the worker is done, but again, we could also make that a visual thing in the session, work or cockpit.

The other super annoying thing is that I need to keep opening shells, for example to merge, I need to open a shell and run git -C .... merge --ff-only etc. That should not be a manual thing to do when you have an automation tool that supposedly handles it.

Here is a complete report of changes made in ~/Code/memnoc-dev where I have tested worktrees with three different features in parallel. That experience is where my feedback comes from, but don't be surprised if you see new things popping up from the agent sum up.


---
title: "Worktrees worked. Coordinating the agents was still clunky."
date: 2026-09-30
description: "A detailed field report from using Drudwyn and Codex to make three website changes: isolated branches, fragile handoffs, missing windows, shared test ports, and a manual merge."
tldr: "Three agents completed three isolated website changes, and we merged their commits locally. Getting there required repeated window recovery, manual task delivery, test-port coordination, and a hand-edited merge conflict. The worktrees did their job; this session exposed how much coordination still depended on me and the supervising agent."
draft: true
tags: [git, worktrees, drudwyn, agentic_workflows]
---

I built Drudwyn to make working with coding agents in tmux easier. So when I
wanted to make several changes to my website, this seemed like a reasonable
test: use my own tool, create separate worktrees, and let agents work on
independent tasks.

The changes were ordinary website maintenance. I was not trying to coordinate
a distributed rewrite or build something with dozens of dependencies. I wanted
three things:

- Change the navigation labels to **Home, About, and Writing**.
- Give blog articles a more interesting header, with color and an optional TL;DR.
- Show CodeAtlas and Drudwyn with synced project metadata, without letting a
  failure in either project's build prevent my website from deploying.

The agents eventually completed the changes. Their commits were merged into a
local integration branch. That part worked.

But the path from “I have three tasks” to “the three tasks are integrated” was
surprisingly manual. I copied commands between conversations and terminals,
recreated a planning window several times, recovered an agent that had received
no task, and manually resolved a predictable conflict in the test scripts.

This is a report of that session, including the awkward parts. It was drafted
with AI assistance from the conversation and checked against the local Git
history and ticket receipts. It is not a timed benchmark, and it is not a claim
that the assembled changes have already been deployed.

## The first friction was the explanation

I started by asking how to use Drudwyn and worktrees to make website changes in
parallel. The first answer was a complete workflow: planning, branches,
worktrees, dependencies, development ports, testing, review, integration, and
cleanup.

Most of it was relevant. It was also too much at once for the way I wanted to
learn. I had to interrupt:

> not like this, guide me step by step.

That matters because the assistant was part of the interface. The experience
was not just Drudwyn's terminal UI. It was Drudwyn, tmux, Git, Codex, the
repository's engineering workflow, and a coordinating conversation explaining
what to do next.

Once we switched to one step at a time, the instructions became easier to
follow. The cost was a long sequence of small handoffs: run a command, reply
“done,” receive the next command, switch windows, describe what happened.

For a guided first run, that can be useful. It also made the amount of human
coordination impossible to miss.

## We gave the workers a shared starting point

Before starting agents, we clarified what the three changes meant.

“Capital case” became title case: Home, About, and Writing. The article header
would use the site's existing Rosé Pine themes, rather than introducing a new
visual system. The TL;DR would be optional and author-controlled.

“Synced projects” needed the most clarification. We agreed on periodically
refreshing descriptions, latest releases, and last-update information from
GitHub, while retaining the last successful values if a refresh failed.
CodeAtlas was already on the homepage, so it needed updating rather than a
duplicate entry. Drudwyn needed adding.

The website also already had an independent live evidence audit. Keeping
external project failures outside normal deployment was partly an existing
property to preserve, not something we were inventing from scratch.

In the original checkout, I created the branch that would carry the shared
instructions and later receive the finished work:

```sh
git switch -c work/website-updates
```

The coordinating agent wrote a spec and three tickets. Each ticket included
acceptance criteria, its scope, verification expectations, and an agreement
about what the worker should avoid changing. Each worker was to commit its
result and leave merging and deployment to the coordinating session.

I committed those instructions:

```sh
git add docs/specs/2026-09-30-website-updates.md .scratch/website-updates/
git commit -m "Plan parallel website updates"
```

That produced the common starting commit, `1d533a4`.

| Worker | Task | Intended scope |
| --- | --- | --- |
| A | Title-case navigation | Navigation markup and affected tests |
| B | Article header and TL;DR | Article presentation, content metadata, focused tests |
| C | Synced projects | Saved metadata, homepage display, refresh automation, tests |

All three tickets had no blockers. The repository's Northstar workflow called
for a fresh session per ticket, TDD, and a crosscheck review. Separate worktrees
gave those sessions separate working files while retaining shared Git history.

That separation was useful. But preparing and committing the instructions was
still something the coordinating agent and I had to arrange explicitly.

## Choosing the correct base was a real decision

In Drudwyn, I opened the worktree creation form with `prefix + Shift+W`.
The form showed a task, a derived branch name, a base, and an agent selection.

The base was important. The behavior documented in the checkout we inspected
preferred the configured base branch's locally available upstream reference.
For this repository, that would normally lead toward `origin/main`.

But our new spec and tickets existed on the local planning branch. Starting
from the default base would have left the worker without those instructions.

I had to press **F2** to select **Continue from current branch**, then verify
that the form showed:

```text
refs/heads/work/website-updates (1d533a4158ff)
```

Showing the ref and commit was helpful. It made the starting point inspectable.
Still, the correct choice depended on remembering why this task needed the
planning branch, which window I was in, and which branch that window's
directory had checked out.

The same check mattered for later workers. Starting B from A's worktree would
have introduced a dependency we did not intend. Every worker needed the
shared planning commit, not whichever checkout happened to be in front of me.

## The first launch did not produce a completed task

For worker A, the assistant told me to put the full instruction into the Task
field:

```text
Implement only .scratch/website-updates/01-title-case-navigation.md. Read its spec and AGENTS.md, and follow the ticket's parallel work agreement.
```

Drudwyn derived the branch name from that text. The resulting directory name
was enormous:

```text
work-implement-only-scratch-website-updates-01-title-case-navigation-md-read-its-spec-and-agents-md-and-follow-the-ticket-s-para-e-work-agreement
```

The assistant later acknowledged that giving me a long instruction for a field
that also generated the branch name was a poor choice. That was a guidance
mistake as well as a useful product observation: a readable workspace name and
a complete agent prompt have different requirements.

Then a new terminal opened and closed. My interpretation was tentative:

> a new terminal session started, and it concluded with success I guess and then closed itself

The coordinating agent inspected the worktree and Git history. The worktree
existed, but it was still at the planning commit. There were no file changes,
no new commits, and the ticket was still marked `ready`.

The worktree had been created. The implementation had not happened.

We did not establish why the agent exited. We had no captured error or exit
record in the evidence used for this report. It would be misleading to label
this a confirmed Codex defect or a confirmed Drudwyn launch bug.

What we did establish was an operational gap: from what I saw, I could not
reliably distinguish “workspace created” from “task completed.” The absence of
a window looked enough like completion that I had to ask.

## Recovering the workspace did not recover the task

The recovery was manual. We opened a persistent tmux shell in the existing
worktree and ran:

```sh
codex
```

That opened a session, but nothing started implementing the navigation change.
I reported that Codex was not picking up any task.

The assistant then explained that starting Codex without a prompt created a
fresh session; it did not recover Drudwyn's earlier task delivery. I had to
paste the instruction again, this time explicitly invoking the implementation
skill and pointing it at ticket 01.

The assistant also acknowledged that this should have been explained before
asking me to restart it.

This was a small recovery sequence with several separate responsibilities:

1. Discover that the first attempt had not implemented anything.
2. Locate the worktree that had survived the closed window.
3. Open a shell that would remain available if the agent exited.
4. Start a new agent session.
5. Deliver the task again.

The source files and ticket were durable. The active session and the knowledge
that it had actually received the task were much less visible to me.

## I repeatedly lost the planning window

After A was running, we moved on to B. The instruction was to return to the
original planning window and create another worktree from the same branch.

I could not find that window through Drudwyn.

We recreated it with:

```sh
tmux new-window -n website-plan -c /home/memnoc/Code/memnoc-dev
```

Then I checked the branch:

```sh
git branch --show-current
```

It needed to print `work/website-updates`.

The same recovery came up again while preparing to start C. At another point
I simply said:

> I once again lost the window

The conversation contains three reports of difficulty finding the planning
window. We repeatedly fell back to a raw tmux command, followed by a Git branch
check, before resuming the cockpit workflow.

Again, that is an observation about this session, not a proven root cause. We
did not establish whether a window had closed, been repurposed, or was merely
hard to identify among the existing windows. No files or branches were lost.

But the user experience was still clunky. I was using a tool intended to keep
agent workspaces within reach, and repeatedly needed an external conversation
to tell me how to reconstruct the place from which I was coordinating them.

Giving the replacement window the explicit name `website-plan` helped make its
role clearer. A future version of the workflow should make that role visible
from the beginning.

## Worktrees isolated files, but the test server was shared

The site had a production-browser test setup using port `4321`. Creating three
worktrees did not create three independent copies of that port.

We assigned separate ports for development previews:

| Worker | Development preview |
| --- | --- |
| A | `4322` |
| B | `4323` |
| C | `4324` |

Production-browser tests still needed sequential access to `4321`.

Workers explicitly requested a reservation from the coordinating session.
The coordinator tracked who could use the port and passed it to the next
worker after the previous one reported releasing it. B reserved it for its
article tests; C later reserved it for project verification and kept the
reservation while final review fixes were possible.

That coordination prevented the workers from deliberately launching their
test servers on the same port at the same time. It was also another small
piece of infrastructure implemented through conversation.

Nothing in our arrangement made the reservation an enforced lock. It depended
on the workers following their instructions and the coordinator keeping track.
The shared resource was outside Git, so the isolation provided by worktrees
did not address it.

## Independent tasks still touched the same file

The tickets were independent from a product perspective. Navigation,
article presentation, and project metadata could be implemented separately.

They were not guaranteed to have completely disjoint file changes.

B needed focused article tests, including an article without a TL;DR. It added
an isolated production fixture and extended the browser command in
`package.json`:

```json
"test:browser": "playwright test && playwright test --config tests/article-headers/playwright.config.ts"
```

C needed commands for project-refresh tests, an isolated project-browser test,
and the refresh operation itself.

B flagged this overlap before integration. The coordinator relayed it to C,
which kept its script edits limited and reported the overlap in its handoff.
That was good behavior: the eventual conflict was expected rather than a
surprise.

It also demonstrates a limit of dividing work by feature. Features that look
independent can still meet in package scripts, shared configuration, or CI.
Someone has to preserve the combined behavior when those changes meet.

## The workers did produce useful results

The friction was mainly around orchestration. The workers' handoffs contained
actual implementation, tests, review findings, and commits.

| Worker | Final commit | Reported verification on its branch |
| --- | --- | --- |
| A | `f235ca3` | Typecheck and build, 9 evidence tests, 37 browser tests, crosscheck with no remaining findings |
| B | `5a5eb80` | Typecheck and build, 9 evidence tests, 36 existing browser tests plus 8 focused article tests, crosscheck with no remaining findings |
| C | `e1e91ca` | Typecheck and build, 9 evidence tests, 22 refresh tests, 40 browser tests, 1 isolated fallback/offline browser test, crosscheck with no remaining findings |

These are branch-level receipts. They should not be added together and
presented as a count of unique tests for the assembled site: each worker
started from the planning commit, and many tests overlap.

B's checks caught contrast and wrapping issues, which it addressed within the
article presentation scope. C's review found an assumption that the saved
metadata snapshot would always contain populated values. The worker corrected
that assumption and verified a sparse snapshot before its final handoff.

That is meaningful engineering work. The agents did more than change code and
declare success. They also reported test results, review outcomes, shared-file
edits, and resource release.

The project-sync implementation used a saved snapshot for the static website
and a separate daily or manually invoked workflow that proposes updated data
through a pull request. A successful metadata refresh is therefore not the
same event as an accepted website update.

C documented the remote permissions and review steps needed for that workflow.
It did not change repository settings or execute the remote workflow during
this session. Local implementation and local tests are the evidence we have.

## Integration was a sequence of manual Git operations

Once the workers had finished, I returned to the integration branch.

First, the coordinator inspected A's change. Its implementation changed the
three navigation labels without changing their routes or active states.
I merged it with:

```sh
git -C /home/memnoc/Code/memnoc-dev merge --ff-only f235ca3
```

That fast-forwarded the planning branch to A's commit.

Then I merged B:

```sh
git -C /home/memnoc/Code/memnoc-dev merge --no-ff 5a5eb80 -m "Merge blog header and TLDR"
```

Git combined those branches cleanly and created merge commit `0ee74ba`.

Then came C:

```sh
git -C /home/memnoc/Code/memnoc-dev merge --no-ff e1e91ca -m "Merge synced project metadata"
```

This time Git reported the expected conflict in `package.json`.

The coordinator inspected the conflict and told me exactly which lines to
keep. I opened the file and replaced the conflict block with the combined
commands:

```json
"test:projects": "node --test tests/projects/*.test.mjs",
"test:projects:browser": "playwright test --config tests/projects/playwright.config.ts",
"refresh:projects": "node scripts/refresh-projects.mjs",
"test:browser": "playwright test && playwright test --config tests/article-headers/playwright.config.ts",
```

The resolution retained C's three commands and B's expanded browser command.
Choosing either side wholesale would have discarded part of the intended
test setup.

After I saved it, the coordinating agent parsed the JSON, checked that all
four commands had the expected values, and checked for whitespace errors or
remaining conflict markers. Then I completed the merge:

```sh
git -C /home/memnoc/Code/memnoc-dev add package.json
git -C /home/memnoc/Code/memnoc-dev commit --no-edit
```

That created `38b197e`, the local merge containing all three workers' changes.

Git did not lose anyone's work. The branch structure was understandable, and
the conflict was small. Still, I was the person copying each merge command,
reporting its output, opening the conflicted file, editing it, and confirming
the result.

Drudwyn had helped create and navigate the workspaces. In this run, integration
was performed through the coordinating conversation and the Git CLI.

## Even the notes became part of the build environment

While doing all this, I had started the Markdown file that became this article.
It lived in the website's blog content directory and was untracked.

After the merges, I noticed it in Git's status output. The coordinator explained
that it had intentionally been left out of the feature commits.

There was another detail: the initial notes had no frontmatter. The blog's
content schema required a title, date, and description. Astro would discover
the file regardless of whether Git tracked it.

So “untracked” did not mean “irrelevant to the build.” My working notes were
inside an input directory consumed by the application.

This draft now has valid metadata and `draft: true`. The draft flag keeps it
out of published writing; valid metadata still matters because content is
loaded and validated before publication filtering.

It was a small issue, but a fitting final example. The workers' clean
worktrees and their passing checks did not cover every file in my original
checkout. Integration verification still had to account for that environment.

## How parallel was this, really?

We created three independent worktrees from a common commit. The tickets had
no implementation dependencies, and the arrangement allowed independent agent
work.

But this particular session does not demonstrate a three-way parallel speedup.

By the time B requested its browser-test reservation, A had already committed
its completed work. C was launched after B's completion had been reported.
Most visible launches, recoveries, and handoffs happened sequentially.

We also have no reliable timing measurements for the full run, no measured
amount of overlapping implementation, and no comparison against doing the
three changes in one session.

The fair claim is that we exercised isolated worker branches and integrated
their results. We prepared a workflow capable of parallel execution, but our
first guided use was largely serial in practice.

That is an important distinction for me. Calling something a parallel agent
workflow because it contains multiple agents and worktrees does not establish
that it saved time.

## Where the manual work accumulated

Looking back, the effort was spread across several small boundaries rather
than concentrated in one dramatic failure.

| Boundary | What I had to do | What remained unclear or manual |
| --- | --- | --- |
| Intent to tickets | Clarify the features and commit shared instructions | The launch UI did not establish the shared task contract for us |
| Planning to worker | Select the current branch and inspect the starting commit | Correct task inheritance depended on my location and base choice |
| Task to workspace name | Enter prose that also became a branch name | The first branch and directory were unwieldy |
| Launch to execution | Notice a closed terminal, inspect the worktree, restart Codex, paste the task | Workspace creation did not establish task completion or explain the exit |
| Navigation to coordination | Recreate the planning window and check its branch repeatedly | The durable project and its temporary terminal window were easy to confuse |
| Independent work to shared tests | Reserve and release port 4321 through messages | There was no enforced resource reservation in our setup |
| Finished branches to integration | Run three merges and resolve package scripts by hand | Finished agent tasks still needed a person to assemble their results |
| Local integration to shipping | Keep track of verification and remote setup still outstanding | Passing worker tests did not establish an assembled, deployed result |

Some of this was appropriate human decision-making: deciding what “synced”
meant, choosing the scope, and reviewing the intended behavior.

Some of it was repetitive plumbing: recovering a known workspace, delivering
the same task again, and communicating which worker could use a port.

The assistant also contributed avoidable friction. It began with too much
information, suggested an excessively long task name, and initially omitted
the fact that restarting Codex required delivering the task again. Those
problems should not all be attributed to Drudwyn.

Nor should the explicit confirmations be mistaken for an inherent requirement
of worktrees. I requested one-step-at-a-time guidance, and some of the repeated
“done” exchanges were a consequence of that teaching format.

Even accounting for those factors, the operation required more supervision
than I wanted for three fairly modest website changes.

## What this makes me want to improve in Drudwyn

These are product ideas from the session, not features implemented by the
website work.

First, I want a short workspace name to be separate from the instruction sent
to the agent. A workspace called `nav-labels` can still carry a detailed task
reference. It should not need a paragraph as its branch name.

Second, I want the distinction between repository, worktree, and live terminal
to be easier to act on. If the terminal disappears but the worktree exists,
reopening that worktree should be an obvious action. A designated planning or
integration workspace should also be easy to return to.

Third, a launch should leave enough operational evidence to understand its
outcome: did the process start, did it exit, and is there a session I can reopen?
That does not require storing the agent's conversation. Exit state and clear
recovery actions would already have helped with our first attempt.

Fourth, task recovery needs an explicit design. If a fresh session does not
inherit a prior prompt, the UI should make that clear. A deliberate ticket
reference could provide a recoverable starting point without pretending to
restore conversational state.

Fifth, I want common collisions to be visible earlier. For this repository,
the browser-test port and package scripts were shared resources. An explicit
convention, configurable ports, or a real reservation mechanism would reduce
the need to coordinate those facts through chat.

Finally, the review and integration handoff should make the important facts
easy to inspect together: starting commit, finished commit, changed files,
reported checks, and whether the branch has been integrated. The handoff should
also keep “worker finished,” “changes merged,” and “assembled site verified”
as separate states.

I do not want to remove decisions that benefit from judgment. I want to stop
reconstructing the same operational context every time I move between tools.

## Where we actually stopped

At the end of the implementation and merge sequence, all three worker commits
were present on the local `work/website-updates` branch. The final merge was
`38b197e`. The package-script conflict had been resolved and the combined JSON
validated.

The workers had reported passing checks and completed crosschecks on their
own branches. The assembled-system hardening pass had not yet been completed.
We had not merged the result into `main`, pushed it, deployed it, or exercised
the scheduled refresh workflow remotely. The worker worktrees had not been
cleaned up either.

Instead, I paused to finish this report while the friction was still fresh.

The next useful test of Drudwyn is concrete: can I run a similar set of tasks
with short names, recoverable sessions, a stable planning workspace, and fewer
copy-and-paste handoffs? This session gives me actual failure points to work
on, and a local Git history against which to check what really happened.---
title: "Worktrees worked. Coordinating the agents was still clunky."
date: 2026-09-30
description: "A detailed field report from using Drudwyn and Codex to make three website changes: isolated branches, fragile handoffs, missing windows, shared test ports, and a manual merge."
tldr: "Three agents completed three isolated website changes, and we merged their commits locally. Getting there required repeated window recovery, manual task delivery, test-port coordination, and a hand-edited merge conflict. The worktrees did their job; this session exposed how much coordination still depended on me and the supervising agent."
draft: true
tags: [git, worktrees, drudwyn, agentic_workflows]
---

I built Drudwyn to make working with coding agents in tmux easier. So when I
wanted to make several changes to my website, this seemed like a reasonable
test: use my own tool, create separate worktrees, and let agents work on
independent tasks.

The changes were ordinary website maintenance. I was not trying to coordinate
a distributed rewrite or build something with dozens of dependencies. I wanted
three things:

- Change the navigation labels to **Home, About, and Writing**.
- Give blog articles a more interesting header, with color and an optional TL;DR.
- Show CodeAtlas and Drudwyn with synced project metadata, without letting a
  failure in either project's build prevent my website from deploying.

The agents eventually completed the changes. Their commits were merged into a
local integration branch. That part worked.

But the path from “I have three tasks” to “the three tasks are integrated” was
surprisingly manual. I copied commands between conversations and terminals,
recreated a planning window several times, recovered an agent that had received
no task, and manually resolved a predictable conflict in the test scripts.

This is a report of that session, including the awkward parts. It was drafted
with AI assistance from the conversation and checked against the local Git
history and ticket receipts. It is not a timed benchmark, and it is not a claim
that the assembled changes have already been deployed.

## The first friction was the explanation

I started by asking how to use Drudwyn and worktrees to make website changes in
parallel. The first answer was a complete workflow: planning, branches,
worktrees, dependencies, development ports, testing, review, integration, and
cleanup.

Most of it was relevant. It was also too much at once for the way I wanted to
learn. I had to interrupt:

> not like this, guide me step by step.

That matters because the assistant was part of the interface. The experience
was not just Drudwyn's terminal UI. It was Drudwyn, tmux, Git, Codex, the
repository's engineering workflow, and a coordinating conversation explaining
what to do next.

Once we switched to one step at a time, the instructions became easier to
follow. The cost was a long sequence of small handoffs: run a command, reply
“done,” receive the next command, switch windows, describe what happened.

For a guided first run, that can be useful. It also made the amount of human
coordination impossible to miss.

## We gave the workers a shared starting point

Before starting agents, we clarified what the three changes meant.

“Capital case” became title case: Home, About, and Writing. The article header
would use the site's existing Rosé Pine themes, rather than introducing a new
visual system. The TL;DR would be optional and author-controlled.

“Synced projects” needed the most clarification. We agreed on periodically
refreshing descriptions, latest releases, and last-update information from
GitHub, while retaining the last successful values if a refresh failed.
CodeAtlas was already on the homepage, so it needed updating rather than a
duplicate entry. Drudwyn needed adding.

The website also already had an independent live evidence audit. Keeping
external project failures outside normal deployment was partly an existing
property to preserve, not something we were inventing from scratch.

In the original checkout, I created the branch that would carry the shared
instructions and later receive the finished work:

```sh
git switch -c work/website-updates
```

The coordinating agent wrote a spec and three tickets. Each ticket included
acceptance criteria, its scope, verification expectations, and an agreement
about what the worker should avoid changing. Each worker was to commit its
result and leave merging and deployment to the coordinating session.

I committed those instructions:

```sh
git add docs/specs/2026-09-30-website-updates.md .scratch/website-updates/
git commit -m "Plan parallel website updates"
```

That produced the common starting commit, `1d533a4`.

| Worker | Task | Intended scope |
| --- | --- | --- |
| A | Title-case navigation | Navigation markup and affected tests |
| B | Article header and TL;DR | Article presentation, content metadata, focused tests |
| C | Synced projects | Saved metadata, homepage display, refresh automation, tests |

All three tickets had no blockers. The repository's Northstar workflow called
for a fresh session per ticket, TDD, and a crosscheck review. Separate worktrees
gave those sessions separate working files while retaining shared Git history.

That separation was useful. But preparing and committing the instructions was
still something the coordinating agent and I had to arrange explicitly.

## Choosing the correct base was a real decision

In Drudwyn, I opened the worktree creation form with `prefix + Shift+W`.
The form showed a task, a derived branch name, a base, and an agent selection.

The base was important. The behavior documented in the checkout we inspected
preferred the configured base branch's locally available upstream reference.
For this repository, that would normally lead toward `origin/main`.

But our new spec and tickets existed on the local planning branch. Starting
from the default base would have left the worker without those instructions.

I had to press **F2** to select **Continue from current branch**, then verify
that the form showed:

```text
refs/heads/work/website-updates (1d533a4158ff)
```

Showing the ref and commit was helpful. It made the starting point inspectable.
Still, the correct choice depended on remembering why this task needed the
planning branch, which window I was in, and which branch that window's
directory had checked out.

The same check mattered for later workers. Starting B from A's worktree would
have introduced a dependency we did not intend. Every worker needed the
shared planning commit, not whichever checkout happened to be in front of me.

## The first launch did not produce a completed task

For worker A, the assistant told me to put the full instruction into the Task
field:

```text
Implement only .scratch/website-updates/01-title-case-navigation.md. Read its spec and AGENTS.md, and follow the ticket's parallel work agreement.
```

Drudwyn derived the branch name from that text. The resulting directory name
was enormous:

```text
work-implement-only-scratch-website-updates-01-title-case-navigation-md-read-its-spec-and-agents-md-and-follow-the-ticket-s-para-e-work-agreement
```

The assistant later acknowledged that giving me a long instruction for a field
that also generated the branch name was a poor choice. That was a guidance
mistake as well as a useful product observation: a readable workspace name and
a complete agent prompt have different requirements.

Then a new terminal opened and closed. My interpretation was tentative:

> a new terminal session started, and it concluded with success I guess and then closed itself

The coordinating agent inspected the worktree and Git history. The worktree
existed, but it was still at the planning commit. There were no file changes,
no new commits, and the ticket was still marked `ready`.

The worktree had been created. The implementation had not happened.

We did not establish why the agent exited. We had no captured error or exit
record in the evidence used for this report. It would be misleading to label
this a confirmed Codex defect or a confirmed Drudwyn launch bug.

What we did establish was an operational gap: from what I saw, I could not
reliably distinguish “workspace created” from “task completed.” The absence of
a window looked enough like completion that I had to ask.

## Recovering the workspace did not recover the task

The recovery was manual. We opened a persistent tmux shell in the existing
worktree and ran:

```sh
codex
```

That opened a session, but nothing started implementing the navigation change.
I reported that Codex was not picking up any task.

The assistant then explained that starting Codex without a prompt created a
fresh session; it did not recover Drudwyn's earlier task delivery. I had to
paste the instruction again, this time explicitly invoking the implementation
skill and pointing it at ticket 01.

The assistant also acknowledged that this should have been explained before
asking me to restart it.

This was a small recovery sequence with several separate responsibilities:

1. Discover that the first attempt had not implemented anything.
2. Locate the worktree that had survived the closed window.
3. Open a shell that would remain available if the agent exited.
4. Start a new agent session.
5. Deliver the task again.

The source files and ticket were durable. The active session and the knowledge
that it had actually received the task were much less visible to me.

## I repeatedly lost the planning window

After A was running, we moved on to B. The instruction was to return to the
original planning window and create another worktree from the same branch.

I could not find that window through Drudwyn.

We recreated it with:

```sh
tmux new-window -n website-plan -c /home/memnoc/Code/memnoc-dev
```

Then I checked the branch:

```sh
git branch --show-current
```

It needed to print `work/website-updates`.

The same recovery came up again while preparing to start C. At another point
I simply said:

> I once again lost the window

The conversation contains three reports of difficulty finding the planning
window. We repeatedly fell back to a raw tmux command, followed by a Git branch
check, before resuming the cockpit workflow.

Again, that is an observation about this session, not a proven root cause. We
did not establish whether a window had closed, been repurposed, or was merely
hard to identify among the existing windows. No files or branches were lost.

But the user experience was still clunky. I was using a tool intended to keep
agent workspaces within reach, and repeatedly needed an external conversation
to tell me how to reconstruct the place from which I was coordinating them.

Giving the replacement window the explicit name `website-plan` helped make its
role clearer. A future version of the workflow should make that role visible
from the beginning.

## Worktrees isolated files, but the test server was shared

The site had a production-browser test setup using port `4321`. Creating three
worktrees did not create three independent copies of that port.

We assigned separate ports for development previews:

| Worker | Development preview |
| --- | --- |
| A | `4322` |
| B | `4323` |
| C | `4324` |

Production-browser tests still needed sequential access to `4321`.

Workers explicitly requested a reservation from the coordinating session.
The coordinator tracked who could use the port and passed it to the next
worker after the previous one reported releasing it. B reserved it for its
article tests; C later reserved it for project verification and kept the
reservation while final review fixes were possible.

That coordination prevented the workers from deliberately launching their
test servers on the same port at the same time. It was also another small
piece of infrastructure implemented through conversation.

Nothing in our arrangement made the reservation an enforced lock. It depended
on the workers following their instructions and the coordinator keeping track.
The shared resource was outside Git, so the isolation provided by worktrees
did not address it.

## Independent tasks still touched the same file

The tickets were independent from a product perspective. Navigation,
article presentation, and project metadata could be implemented separately.

They were not guaranteed to have completely disjoint file changes.

B needed focused article tests, including an article without a TL;DR. It added
an isolated production fixture and extended the browser command in
`package.json`:

```json
"test:browser": "playwright test && playwright test --config tests/article-headers/playwright.config.ts"
```

C needed commands for project-refresh tests, an isolated project-browser test,
and the refresh operation itself.

B flagged this overlap before integration. The coordinator relayed it to C,
which kept its script edits limited and reported the overlap in its handoff.
That was good behavior: the eventual conflict was expected rather than a
surprise.

It also demonstrates a limit of dividing work by feature. Features that look
independent can still meet in package scripts, shared configuration, or CI.
Someone has to preserve the combined behavior when those changes meet.

## The workers did produce useful results

The friction was mainly around orchestration. The workers' handoffs contained
actual implementation, tests, review findings, and commits.

| Worker | Final commit | Reported verification on its branch |
| --- | --- | --- |
| A | `f235ca3` | Typecheck and build, 9 evidence tests, 37 browser tests, crosscheck with no remaining findings |
| B | `5a5eb80` | Typecheck and build, 9 evidence tests, 36 existing browser tests plus 8 focused article tests, crosscheck with no remaining findings |
| C | `e1e91ca` | Typecheck and build, 9 evidence tests, 22 refresh tests, 40 browser tests, 1 isolated fallback/offline browser test, crosscheck with no remaining findings |

These are branch-level receipts. They should not be added together and
presented as a count of unique tests for the assembled site: each worker
started from the planning commit, and many tests overlap.

B's checks caught contrast and wrapping issues, which it addressed within the
article presentation scope. C's review found an assumption that the saved
metadata snapshot would always contain populated values. The worker corrected
that assumption and verified a sparse snapshot before its final handoff.

That is meaningful engineering work. The agents did more than change code and
declare success. They also reported test results, review outcomes, shared-file
edits, and resource release.

The project-sync implementation used a saved snapshot for the static website
and a separate daily or manually invoked workflow that proposes updated data
through a pull request. A successful metadata refresh is therefore not the
same event as an accepted website update.

C documented the remote permissions and review steps needed for that workflow.
It did not change repository settings or execute the remote workflow during
this session. Local implementation and local tests are the evidence we have.

## Integration was a sequence of manual Git operations

Once the workers had finished, I returned to the integration branch.

First, the coordinator inspected A's change. Its implementation changed the
three navigation labels without changing their routes or active states.
I merged it with:

```sh
git -C /home/memnoc/Code/memnoc-dev merge --ff-only f235ca3
```

That fast-forwarded the planning branch to A's commit.

Then I merged B:

```sh
git -C /home/memnoc/Code/memnoc-dev merge --no-ff 5a5eb80 -m "Merge blog header and TLDR"
```

Git combined those branches cleanly and created merge commit `0ee74ba`.

Then came C:

```sh
git -C /home/memnoc/Code/memnoc-dev merge --no-ff e1e91ca -m "Merge synced project metadata"
```

This time Git reported the expected conflict in `package.json`.

The coordinator inspected the conflict and told me exactly which lines to
keep. I opened the file and replaced the conflict block with the combined
commands:

```json
"test:projects": "node --test tests/projects/*.test.mjs",
"test:projects:browser": "playwright test --config tests/projects/playwright.config.ts",
"refresh:projects": "node scripts/refresh-projects.mjs",
"test:browser": "playwright test && playwright test --config tests/article-headers/playwright.config.ts",
```

The resolution retained C's three commands and B's expanded browser command.
Choosing either side wholesale would have discarded part of the intended
test setup.

After I saved it, the coordinating agent parsed the JSON, checked that all
four commands had the expected values, and checked for whitespace errors or
remaining conflict markers. Then I completed the merge:

```sh
git -C /home/memnoc/Code/memnoc-dev add package.json
git -C /home/memnoc/Code/memnoc-dev commit --no-edit
```

That created `38b197e`, the local merge containing all three workers' changes.

Git did not lose anyone's work. The branch structure was understandable, and
the conflict was small. Still, I was the person copying each merge command,
reporting its output, opening the conflicted file, editing it, and confirming
the result.

Drudwyn had helped create and navigate the workspaces. In this run, integration
was performed through the coordinating conversation and the Git CLI.

## Even the notes became part of the build environment

While doing all this, I had started the Markdown file that became this article.
It lived in the website's blog content directory and was untracked.

After the merges, I noticed it in Git's status output. The coordinator explained
that it had intentionally been left out of the feature commits.

There was another detail: the initial notes had no frontmatter. The blog's
content schema required a title, date, and description. Astro would discover
the file regardless of whether Git tracked it.

So “untracked” did not mean “irrelevant to the build.” My working notes were
inside an input directory consumed by the application.

This draft now has valid metadata and `draft: true`. The draft flag keeps it
out of published writing; valid metadata still matters because content is
loaded and validated before publication filtering.

It was a small issue, but a fitting final example. The workers' clean
worktrees and their passing checks did not cover every file in my original
checkout. Integration verification still had to account for that environment.

## How parallel was this, really?

We created three independent worktrees from a common commit. The tickets had
no implementation dependencies, and the arrangement allowed independent agent
work.

But this particular session does not demonstrate a three-way parallel speedup.

By the time B requested its browser-test reservation, A had already committed
its completed work. C was launched after B's completion had been reported.
Most visible launches, recoveries, and handoffs happened sequentially.

We also have no reliable timing measurements for the full run, no measured
amount of overlapping implementation, and no comparison against doing the
three changes in one session.

The fair claim is that we exercised isolated worker branches and integrated
their results. We prepared a workflow capable of parallel execution, but our
first guided use was largely serial in practice.

That is an important distinction for me. Calling something a parallel agent
workflow because it contains multiple agents and worktrees does not establish
that it saved time.

## Where the manual work accumulated

Looking back, the effort was spread across several small boundaries rather
than concentrated in one dramatic failure.

| Boundary | What I had to do | What remained unclear or manual |
| --- | --- | --- |
| Intent to tickets | Clarify the features and commit shared instructions | The launch UI did not establish the shared task contract for us |
| Planning to worker | Select the current branch and inspect the starting commit | Correct task inheritance depended on my location and base choice |
| Task to workspace name | Enter prose that also became a branch name | The first branch and directory were unwieldy |
| Launch to execution | Notice a closed terminal, inspect the worktree, restart Codex, paste the task | Workspace creation did not establish task completion or explain the exit |
| Navigation to coordination | Recreate the planning window and check its branch repeatedly | The durable project and its temporary terminal window were easy to confuse |
| Independent work to shared tests | Reserve and release port 4321 through messages | There was no enforced resource reservation in our setup |
| Finished branches to integration | Run three merges and resolve package scripts by hand | Finished agent tasks still needed a person to assemble their results |
| Local integration to shipping | Keep track of verification and remote setup still outstanding | Passing worker tests did not establish an assembled, deployed result |

Some of this was appropriate human decision-making: deciding what “synced”
meant, choosing the scope, and reviewing the intended behavior.

Some of it was repetitive plumbing: recovering a known workspace, delivering
the same task again, and communicating which worker could use a port.

The assistant also contributed avoidable friction. It began with too much
information, suggested an excessively long task name, and initially omitted
the fact that restarting Codex required delivering the task again. Those
problems should not all be attributed to Drudwyn.

Nor should the explicit confirmations be mistaken for an inherent requirement
of worktrees. I requested one-step-at-a-time guidance, and some of the repeated
“done” exchanges were a consequence of that teaching format.

Even accounting for those factors, the operation required more supervision
than I wanted for three fairly modest website changes.

## What this makes me want to improve in Drudwyn

These are product ideas from the session, not features implemented by the
website work.

First, I want a short workspace name to be separate from the instruction sent
to the agent. A workspace called `nav-labels` can still carry a detailed task
reference. It should not need a paragraph as its branch name.

Second, I want the distinction between repository, worktree, and live terminal
to be easier to act on. If the terminal disappears but the worktree exists,
reopening that worktree should be an obvious action. A designated planning or
integration workspace should also be easy to return to.

Third, a launch should leave enough operational evidence to understand its
outcome: did the process start, did it exit, and is there a session I can reopen?
That does not require storing the agent's conversation. Exit state and clear
recovery actions would already have helped with our first attempt.

Fourth, task recovery needs an explicit design. If a fresh session does not
inherit a prior prompt, the UI should make that clear. A deliberate ticket
reference could provide a recoverable starting point without pretending to
restore conversational state.

Fifth, I want common collisions to be visible earlier. For this repository,
the browser-test port and package scripts were shared resources. An explicit
convention, configurable ports, or a real reservation mechanism would reduce
the need to coordinate those facts through chat.

Finally, the review and integration handoff should make the important facts
easy to inspect together: starting commit, finished commit, changed files,
reported checks, and whether the branch has been integrated. The handoff should
also keep “worker finished,” “changes merged,” and “assembled site verified”
as separate states.

I do not want to remove decisions that benefit from judgment. I want to stop
reconstructing the same operational context every time I move between tools.

## Where we actually stopped

At the end of the implementation and merge sequence, all three worker commits
were present on the local `work/website-updates` branch. The final merge was
`38b197e`. The package-script conflict had been resolved and the combined JSON
validated.

The workers had reported passing checks and completed crosschecks on their
own branches. The assembled-system hardening pass had not yet been completed.
We had not merged the result into `main`, pushed it, deployed it, or exercised
the scheduled refresh workflow remotely. The worker worktrees had not been
cleaned up either.

Instead, I paused to finish this report while the friction was still fresh.

The next useful test of Drudwyn is concrete: can I run a similar set of tasks
with short names, recoverable sessions, a stable planning workspace, and fewer
copy-and-paste handoffs? This session gives me actual failure points to work
on, and a local Git history against which to check what really happened.

