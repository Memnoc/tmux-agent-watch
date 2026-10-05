# 27 — Readable check output and workspace activity labels

Status: implemented

## Feedback

The October 4 screenshots at 18:43 and 18:44 show a monochrome check-output
screen and a selected Ready to review label crowded against its branch name.

## Changes

- Use the configured theme for Running, Passed and Failed headings, with text
  symbols as well as color. Separate command output and result with quiet rules.
- Show check name, folder and exit status; keep raw timestamps/revisions in the
  returning form's Details. Explain Enter to return and F2 for Details.
- Bound activity labels to a 20-cell column with trailing padding. Light text
  on selected rows preserves contrast when selection replaces badge backgrounds.
- Keep commands and child output transient; stdout/stderr inherit the terminal.
  Existing CLI receipts and check semantics are unchanged.

## Evidence

Before: a real navigator render had only one space between Ready to review and
main, overflowing its column. The command-output regression lacked CHECK OUTPUT.
After: real keyboard/render tests cover branch alignment at 200/160 columns,
selected-label foreground, and the label at 64 columns. Successful and failed
check screens cover semantic colors, redaction, exit codes and return to form.

Validation: 43 Rust tests; 15 screenshot tests; 11 verification tests; navigator
tests (one optional Resurrect skip); merge-result/check-return regression; privacy
checks. Release workflow validation recorded below after completion.

Both assembled release routes passed: direct main and separate integration branch,
with controlled workers, recovery, conflicts, checks and cleanup. Actual terminal
captures of Passed and Failed are at /tmp/drudwyn-feedback-ui/check-output.html
(ephemeral). Logs: /tmp/drudwyn-modal-screens-final.log,
/tmp/drudwyn-output-suite.log, /tmp/drudwyn-modal-assembled.log.


## October 5 follow-up — badge background reached the branch

The 15:06 screenshot shows that text padding alone did not create a visible gap:
the activity span painted its trailing spaces with the badge background all the
way to the branch column. The previous text-only gap assertion passed this case.

Extended the existing actual-terminal regression to check both whitespace and
Rosé Pine background cells before the branch at 200/160 columns. It failed with
pine (49,116,143) where the neutral base (25,23,36) was required. The renderer now
reserves the final two activity-column cells as an unstyled gutter, preserving
column positions, badge text padding and selected-row contrast.

Validation: all 15 screenshot regressions and 13 navigator tests passed (one
optional tmux-resurrect test skipped), plus formatting and whitespace checks.
Logs: /tmp/drudwyn-badge-gap-red.log, /tmp/drudwyn-badge-gap-green.log and
/tmp/drudwyn-badge-navigator.log. Checking rendered background cells, rather than
only spaces in captured text, prevents recurrence of this visual defect.
