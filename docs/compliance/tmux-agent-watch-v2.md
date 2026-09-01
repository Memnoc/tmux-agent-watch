---
system: tmux-agent-watch v2
owner: Memnoc
last-reviewed: 2026-09-01
phase: shipping
verdict: ready
---

# Compliance: tmux-agent-watch v2

## Boundary

- Intended purpose: a deterministic, tmux-native supervisor for local coding-agent workspaces; it launches separately installed agents and presents operational lifecycle state. User-confirmed 2026-09-01.
- Users and affected people: terminal-native developers operating their own local tmux and Git environments. It is not intended for evaluating, ranking, or making decisions about people. User-confirmed 2026-09-01.
- Distribution and jurisdictions: public source and release artifacts distributed globally through GitHub, including the EU/EEA; there is no hosted service. User-confirmed 2026-09-01; repository remote: `git@github.com:Memnoc/tmux-agent-watch.git`.
- Builder/operator legal roles: Memnoc is the open-source maintainer; each user operates the software locally. User-confirmed 2026-09-01.
- Upstream providers and models: Codex, Claude Code, and OpenCode are independently selected and installed third-party tools. v2 supplies and operates no model. User-confirmed 2026-09-01.
- Training or fine-tuning: none. User-confirmed 2026-09-01.
- Data categories, sources, destinations, retention, deletion: v2 may transiently route a user-entered task to the chosen local agent and may inspect non-content tmux, process, and Git metadata. It must not read or retain prompts, messages, generated content, permission text, terminal scrollback, task history, telemetry, analytics, or crash reports. No project backend receives data. Runtime state is discarded with the process or tmux session. User-confirmed 2026-09-01.
- Outputs and decisions influenced: fixed operational labels and user-requested tmux/Git actions. No decisions about people. User-confirmed 2026-09-01.
- Behaviour configuration: typed local configuration derived from documented tmux options; deterministic rules only. No model, remote feature flags, or content inference. Confirmed architecture interview 2026-09-01.
- Release state and relevant dates: v2.0.0-alpha.1 is assembled locally on 2026-09-01 and is the default implementation in the working tree; `@agent-watch-v2 off` retains the legacy fallback. Shipping verification covers source based on `b7ef00fe2c09ac9df382f5ea02a8edec6fb43929` plus the documented uncommitted v2 implementation, and the locally built Linux x86_64 release binary. The public v1.0.0 Bash tag remains a separate legacy release.

## Trigger triage

| Trigger | Result | Evidence | Disposition |
|---------|--------|----------|-------------|
| AI inference, model distribution, training, or upstream AI service | yes | v2 launches independently installed coding agents but performs no inference; confirmed boundary and research note | EU AI Act pack; preserve the non-AI deterministic supervisor boundary |
| Personal or behavioural data | yes | repository paths, branch names, task text, and activity timestamps can identify a person; GDPR Article 4 research | GDPR/privacy pack; content-blind, local-only, ephemeral processing controls required |
| Regulated decisions about a person | no | intended purpose explicitly excludes ranking, employment decisions, and human performance monitoring | Out of scope; a change requires a new review |
| Direct AI interaction or synthetic content | no for the supervisor | the UI identifies and launches third-party agents but does not present their output as its own persona | Keep upstream identity visible; a combined conversational surface requires review |
| Biometrics, emotion inference, surveillance, manipulation, or social scoring | no | no such capability or intended purpose | Prohibit inferred productivity, intent, emotion, and worker scoring |
| Regulated product, profession, or sector | no | local developer workflow tool | A sector-specific intended purpose requires review |
| Foreseeable material harm or misuse | yes, limited | content capture or persistent employee monitoring could create privacy and rights impact | Enforce no content capture, telemetry, history, or cross-session monitoring |
| EU jurisdictional nexus | yes | global GitHub distribution includes EU/EEA users | EU AI Act and GDPR/privacy research completed |

## Distribution inventory

| Artifact or service | Revision/version | Public/deployed/supported | Evidence | Coverage |
|---------------------|------------------|---------------------------|----------|----------|
| GitHub source and TPM plugin | public `v1.0.0` and current `main` | public and supported legacy release | remote `git@github.com:Memnoc/tmux-agent-watch.git`, tag and README | legacy v1 recorded separately; not treated as v2 privacy evidence |
| v2 Rust source | `2.0.0-alpha.1`, working tree based on `b7ef00f` | local release candidate, opt-in | `Cargo.toml`, `Cargo.lock`, `src/`, full test suite | verified on Linux x86_64 |
| Linux x86_64 binary | `2.0.0-alpha.1` | locally built, not yet published | stripped ELF; SHA-256 `b99922702a4182ab237ca748b00121a0a162f4686c05b8e16dbca1f2feef7e3f` | build and runtime tests verified 2026-09-01 |
| Linux/macOS x86_64 and ARM64 release archives | workflow defined for `2.0.0-alpha.1` | not yet published | `.github/workflows/release.yml`, `scripts/package-release.sh`, `tests/package_test.sh` | local package/install seam verified; each hosted artifact remains subject to its tag workflow result |
| Hosted service/API | none | none | architecture, dependencies, source scan | verified absent |

## Foreseeable harm review

| Affected party | Capability, failure, or misuse | Reach and reversibility | Safeguard evidence | Disposition |
|----------------|--------------------------------|-------------------------|--------------------|-------------|
| Local developer | task or agent content retained by the supervisor | local but potentially sensitive; copies may be difficult to retract | content-blind discovery/hook tests and outer-seam transient delivery test | ready |
| Developer during screen sharing | repository, branch, task, or workspace labels exposed | audience of the share; disclosure may be irreversible | `@agent-watch-redact-labels`, unit tests, and rendered tmux integration test | ready |
| Employee or contractor | activity state repurposed for performance monitoring | potentially organisational and rights-affecting | unsupported intended purpose, no history/backend/identity, privacy notice requires deployer assessment | ready within stated purpose; new review required for monitoring support |
| Operator repository | unsafe worktree deletion | local code loss, partly reversible through Git | clean, linked, merged, and confirmation guards in v2 integration suite | ready |

## Legal obligations

| ID | Pack and primary source | Applies because | Required outcome | Owner | Evidence | Status |
|----|-------------------------|-----------------|------------------|-------|----------|--------|
| L-1 | GDPR Articles 4(1)-(2), 5, and 25 | transient consultation and use can be personal-data processing | Minimise local data touched and make privacy-preserving defaults observable in the released implementation | Memnoc | `docs/privacy.md`, `tests/privacy_test.sh`, `tests/v2_test.sh`, redacted render tests | ready |
| L-2 | AI Act Article 3(1) and Recital 12 | the non-AI classification depends on deterministic, human-defined rules | Keep v2 model-free and do not generate inferred content, recommendations, scores, or decisions | Memnoc | ADR-0002; deterministic `src/domain.rs`, `src/lifecycle.rs`, and dependency/source checks | ready |

## Northstar engineering policy

| ID | Trigger | Control | Owner | Evidence | Status |
|----|---------|---------|-------|----------|--------|
| P-1 | Local workspace metadata may be personal data | No backend, network calls, telemetry, analytics, update ping, crash upload, database, history, cache, or content-bearing tmux options | Memnoc | dependency inventory, `tests/privacy_test.sh`, `tests/v2_test.sh`, and `docs/privacy.md` | ready |
| P-2 | Agent prompts and output can contain sensitive content | Never inspect pane scrollback, prompts, responses, permission details, clipboard, diffs, file contents, environment values, or commit bodies | Memnoc | `src/lifecycle.rs`, `src/discovery.rs`, and `tests/v2_test.sh`, including task-delivery outer-seam coverage | ready |
| P-3 | A task is needed to start an agent | Route task text directly to the chosen agent without storing it in plugin state, environment variables, tmux options, or logs | Memnoc | `workspace::deliver_task` uses stdin and a uniquely named delete-on-paste tmux buffer; `tests/v2_test.sh` verifies pane delivery and absence from options, buffers, environment, and process arguments | ready |
| P-4 | Diagnostics can accidentally disclose metadata | Default to transient errors only; any future debug export requires a separate review, explicit preview, redaction, and user-chosen destination | Memnoc | no diagnostic/export command, logging framework, or crash uploader in source or dependencies; errors remain transient stderr/TUI state | ready |
| P-5 | Screen sharing can expose local metadata | Provide a display-redaction mode for repository, path, branch, task, session, and window labels | Memnoc | typed `@agent-watch-redact-labels`; cockpit/ambient unit tests and outer-seam sidebar test | ready |
| P-6 | Privacy boundary can regress | CI checks dependencies and source for network, analytics, crash, and database paths; document the local data-flow manifest | Memnoc | `tests/privacy_test.sh` is included by `tests/run.sh`; `docs/privacy.md` | ready |

## Uncertainty and decisions

| Question | Why material | Decision owner | Resolution or due condition | Release consequence |
|----------|--------------|----------------|-----------------------------|---------------------|
| Does a future feature make the project an AI-system provider or a controller/processor? | Bundling, rebranding, hosting, telemetry, remote access, or inferred scoring can change legal roles | Memnoc with qualified EU counsel where material | New compliance review before design approval | Do not release the changed boundary without resolution |
| Will an organisation use the tool for employee monitoring or performance decisions? | Workplace use may create different GDPR, employment-law, DPIA, and AI Act consequences | Organisational deployer; project owner for intended-purpose changes | Keep this purpose explicitly unsupported; review any proposed product support | Do not market or implement the use case without review |

## Research

- [EU AI Act and GDPR considerations for a stateless agent supervisor](../research/2026-09-01-eu-ai-privacy-stateless-agent-supervisor.md)

## Review history

### 2026-09-01 — shipping

- Boundary changes: none. V2 remains local-only, deterministic, content-blind, and stateless; agent selection and transient task routing do not change the upstream-provider boundary.
- Evidence exercised: 14 Rust unit tests; complete v1/v2 tmux suite; rendered redaction with navigation preserved; task delivery with absence checks across tmux options, buffers, server environment, and process arguments; dependency/source privacy checks; offline release build; Linux ELF linkage and SHA-256 capture.
- Accepted unverifiable evidence, approver, and rationale: macOS and ARM64 artifacts do not yet exist and are outside the current distributed artifact set; they require the same verification before publication. No hosted GitHub release settings or unpublished artifact provenance were asserted.
- Verdict and reason: `ready` for the recorded v2 source and local Linux x86_64 artifact because all applicable privacy and deterministic-boundary outcomes have executable evidence and no release-blocking issue was identified within this coverage. This is not a legal opinion and does not cover future binaries until verified.
- Migration verification: an unset `@agent-watch-v2` selects Rust cockpit, scanning, hooks, ambient surfaces, and workspace lifecycle actions; an explicit `off` restores the legacy Bash path. Both paths pass the assembled suite.
- Distribution verification: the tag workflow requires an exact Cargo version match, builds on four native GitHub-hosted runner architectures, publishes immutable per-target artifacts plus `SHA256SUMS`, and grants write permission only to the publish job. The local package/install seam verifies archive contents, checksum enforcement, installation, and execution. Hosted runner outputs remain unverified until the first tag workflow completes.
- Preflight gate: `workflow_dispatch` builds and verifies the same four-platform matrix but the publish job is guarded by a version-tag reference. The combined checksum-bearing bundle is retained for 14 days for the acceptance work in `docs/preflight-checklist.md`; `tests/release_workflow_test.sh` protects the matrix and publication guards.

### 2026-09-01 — planning

- Boundary changes: established v2 as local-only, deterministic, content-blind, and stateless by design.
- Evidence exercised: repository README, scripts, current lifecycle storage, confirmed architecture interview, and current official EU primary-source research.
- Accepted unverifiable evidence, approver, and rationale: intended purpose, distribution, and future implementation behaviour are owner-confirmed planning facts; Memnoc approved them on 2026-09-01.
- Verdict and reason: `action-required` because the design may proceed, but the recorded privacy and deterministic-boundary controls require implementation and verification before release.
