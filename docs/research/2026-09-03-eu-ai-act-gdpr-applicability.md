# EU AI Act and GDPR applicability: `tmux-agent-watch`

Date: 2026-09-03
Reviewed revision: `77cf87d9aa7944bb820459f3f3a813f094650823` (`ux/statusline-tabs`)
Status: primary-source research for a repository compliance review; not legal advice

## Scope and factual boundary

This note assesses the current v2/default product as a local tmux plugin and
Rust executable that:

- observes explicit tmux, process, lifecycle, working-directory, and Git
  metadata for separately installed coding-agent programs;
- renders deterministic status labels and provides user-invoked navigation and
  guarded Git-worktree operations;
- does not contain, train, fine-tune, host, proxy, or run an AI model;
- does not infer predictions, content, recommendations, scores, or decisions;
- does not read agent prompts, responses, permission text, or terminal
  scrollback in the v2 path;
- retains no task history and sends no telemetry or workspace data to the
  maintainer; and
- does not make or materially support decisions about natural persons.

Repository evidence includes `README.md`, `docs/privacy.md`, `src/discovery.rs`,
`src/lifecycle.rs`, `src/workspace.rs`, and `tests/privacy_test.sh`. The optional
legacy Bash path (`@agent-watch-v2 off`) is not evidence for the content-blind
v2 claim: its observation fallback includes `capture-pane` and some legacy
hooks inspect event payload fields. Any continued distribution of that mode
must be described separately rather than covered by the narrower v2 boundary.

## Executive finding

On the stated v2 facts, `tmux-agent-watch` is likely **not an “AI system”**
under the EU AI Act. Article 3(1) requires a machine-based system that infers
from inputs how to generate outputs such as predictions, content,
recommendations, or decisions. This tool maps explicit operational facts to
human-defined UI states and actions; it performs no such inference. Recital 12
supports distinguishing AI from simpler software based solely on rules defined
by natural persons. See the current [consolidated AI Act, Article 3(1) and
Recital 12](https://eur-lex.europa.eu/eli/reg/2024/1689/2026-07-27/eng).

Consequently, the maintainer is unlikely to be an AI-system **provider** merely
by publishing this plugin, and the plugin does not independently trigger the
Act's high-risk or Article 50 transparency regimes. The separately installed
coding agents remain the relevant AI systems. Their suppliers may be providers,
and an organisation using them professionally may be their deployer. This is a
fact-sensitive interpretation, not a regulator's product-specific ruling.

GDPR is different: local, transient handling of identifiers or workspace
activity can be “processing” even without retention. On the stated architecture,
the maintainer receives no such data and is unlikely to be controller or
processor for an operator's local use. A workplace operator may nevertheless be
controller for personal data it chooses to expose or use through the tool.

## EU AI Act

### Regulated object and territorial scope

The Act applies to providers placing AI systems or general-purpose AI models on
the Union market, EU deployers, certain third-country providers/deployers whose
AI output is used in the Union, and other listed operators and affected people.
See [Article 2(1)](https://eur-lex.europa.eu/eli/reg/2024/1689/2026-07-27/eng).
An EU nexus therefore matters only after identifying a regulated object and
role; global GitHub availability alone does not turn deterministic software
into an AI system.

The current product lacks the defining inference function in Article 3(1).
Launching or displaying the state of another supplier's AI CLI does not, on
these facts, integrate a model into this plugin or make the plugin a
general-purpose AI system. Preserve this boundary by keeping lifecycle
classification explicit and model-free and by naming the third-party agent.

Article 2(12)'s free/open-source exclusion is secondary protection, not the
primary conclusion: it excludes FOSS AI systems unless they are placed on the
market or put into service as high-risk systems or systems covered by Articles
5 or 50. See [Article 2(12)](https://eur-lex.europa.eu/eli/reg/2024/1689/2026-07-27/eng).
If a future version becomes an AI system, the exact licence, commercial model,
classification, and exceptions would need a fresh analysis.

### Roles around the separately installed agents

Article 3 defines:

- a **provider** as the party that develops or has developed an AI system/model
  and places it on the market or puts it into service under its own name or
  trademark, whether paid or free; and
- a **deployer** as a person or body using an AI system under its authority,
  except use in a personal non-professional activity.

See [Article 3(3)-(4)](https://eur-lex.europa.eu/eli/reg/2024/1689/2026-07-27/eng).
Accordingly, the coding-agent supplier is the likely provider, while a business
using the agent may be a deployer. The open-source plugin maintainer is not, on
the current facts, provider of those separately supplied systems.

For a **high-risk** AI system, Article 25 can transfer provider status to a
third party that rebrands it, substantially modifies it, or changes its
intended purpose so that it becomes high-risk. See [Article
25](https://eur-lex.europa.eu/eli/reg/2024/1689/2026-07-27/eng). A new review is
therefore required before bundling/rebranding an agent, overriding safeguards,
or directing it toward Annex III decisions such as employment evaluation.

### Risk classification and transparency

The present intended purpose—supervising developer-controlled coding-agent
processes—is not itself an Annex III high-risk use. The result would change if
the tool inferred employee performance, ranked people, allocated work to
people, or supported decisions in employment, education, essential services,
law enforcement, migration, justice, biometrics, or another Article 6/Annex III
setting.

Article 50 applies specific transparency duties to certain AI systems, including
direct human interaction and synthetic-content/deepfake cases. Because this
plugin does not generate content or present itself as the conversational AI,
those duties are unlikely to attach independently to it. Clear labels such as
“Codex”, “Claude Code”, and “OpenCode” remain a sensible boundary safeguard.
See [Article 50](https://eur-lex.europa.eu/eli/reg/2024/1689/2026-07-27/eng).

### AI literacy

Current Article 4 requires **providers and deployers of AI systems** to take
measures supporting AI literacy for staff and others operating or using AI on
their behalf, proportionate to their knowledge, experience, training, context,
and affected people; it does not require guaranteeing a particular individual's
level. See [Article 4](https://eur-lex.europa.eu/eli/reg/2024/1689/2026-07-27/eng).

Because the plugin is likely not an AI system and its maintainer is not the
provider/deployer of the separately installed agent, Article 4 does not appear
to impose a standalone literacy duty on the plugin maintainer. An organisational
operator may have Article 4 duties as deployer of its coding agents. The README
can help by accurately naming agents, distinguishing lifecycle metadata from
model output, and linking to each agent supplier's instructions, but it should
not imply that this alone discharges an operator's context-specific duty.

### Exact application dates current on 2026-09-03

Regulation (EU) 2026/1744 amended the schedule and entered into force on 27 July
2026. Under current Article 113:

| Date | Application milestone |
| --- | --- |
| 1 August 2024 | AI Act entered into force. |
| 2 February 2025 | Chapters I and II, including definitions, Article 4 AI literacy, and most prohibited practices, began applying. |
| 2 August 2025 | Governance, GPAI, penalties (except Article 101), and Article 78 began applying. |
| 27 July 2026 | Amending Regulation (EU) 2026/1744 entered into force; Articles 102-110 began applying. |
| 2 August 2026 | The Act's general application date; enforcement powers for provisions then applicable began. |
| 2 December 2026 | Newly added prohibited-practice provisions in Article 5(1)(ba)-(bb), (1a)-(1b) apply; the transition in Article 111(4) for pre-2-August-2026 synthetic-content systems also ends. |
| 2 December 2027 | Chapter III Sections 1-3 apply to Annex III high-risk systems under Article 6(2). |
| 2 August 2028 | Chapter III Sections 1-3 apply to Annex I product-related high-risk systems under Article 6(1). |

Primary sources: [consolidated Article
113](https://eur-lex.europa.eu/eli/reg/2024/1689/2026-07-27/eng), [Regulation
(EU) 2026/1744](https://eur-lex.europa.eu/eli/reg/2026/1744/oj/eng), and the
Commission's current [AI Act application
timeline](https://digital-strategy.ec.europa.eu/en/policies/regulatory-framework-ai).

## GDPR and privacy

### Applicability and roles

GDPR has applied since **25 May 2018**. See [Article
99](https://eur-lex.europa.eu/eli/reg/2016/679/oj/eng). It applies to processing
of personal data by automated means within its material and territorial scope;
purely personal or household activity is excluded. See [Articles 2 and
3](https://eur-lex.europa.eu/eli/reg/2016/679/oj/eng).

“Personal data” includes information relating to an identified or identifiable
natural person. “Processing” includes collection, consultation, use, disclosure,
and erasure. Thus a username, user-bearing path, branch/task label, or activity
state may be personal data, and transient display can be processing even when
nothing is retained. See [Article
4(1)-(2)](https://eur-lex.europa.eu/eli/reg/2016/679/oj/eng).

Controller and processor roles follow actual purposes and means. On the stated
local-only facts, the maintainer cannot access operator data and does not decide
why it is processed, so publishing the software alone is unlikely to make the
maintainer controller or processor for each installation. A professional user
or employer choosing how local metadata is used is the more plausible
controller. Personal/non-professional users may fall within the household
exception; workplace users should not assume it.

### Data minimisation and privacy by design

Where GDPR applies to an operator, Article 5 requires lawfulness, purpose
limitation, data minimisation, accuracy, storage limitation, security, and
accountability. Article 25 requires the controller to implement appropriate
data-protection-by-design/default measures proportionate to the processing and
risk. See [Article 5](https://eur-lex.europa.eu/eli/reg/2016/679/oj/eng) and
[Article 25](https://eur-lex.europa.eu/eli/reg/2016/679/oj/eng).

The current v2 choices—local-only operation, no backend/telemetry/history,
content-blind discovery, transient state, explicit labels, and screen-sharing
redaction—are strong minimisation safeguards. They do not establish a universal
lawful basis for every operator. Organisational users remain responsible for
their purpose, lawful basis, employee notices/consultation, access controls,
and retention decisions.

A DPIA is required only where the planned processing is likely to result in a
high risk to natural persons, including certain systematic evaluations and
large-scale monitoring. Ordinary individual developer use does not obviously
meet that threshold; systematic employee monitoring or performance use could.
See [Article 35](https://eur-lex.europa.eu/eli/reg/2016/679/oj/eng).

### Necessary public claims

Use precise, testable language:

- “No project-controlled collection or durable content storage in v2” is safer
  than “no data processing.”
- State which local metadata is read, why, where it appears, and its lifetime.
- Distinguish the plugin from the coding agents and their providers' own data
  flows and terms.
- State that launching an agent can cause the separately configured provider to
  process the user's prompt.
- Keep workforce monitoring, employee scoring, and decisions about people
  explicitly outside the intended purpose.
- Describe the optional legacy mode separately if it remains distributed,
  because its content-observation behaviour falls outside the v2 promise.

## Change triggers requiring a new review

Reassess before releasing any feature that:

1. embeds, bundles, fine-tunes, hosts, proxies, evaluates, or rebrands a model or
   AI system;
2. infers task quality, intent, risk, emotion, productivity, or recommendations;
3. scores/ranks people or supports employment or other Annex III decisions;
4. reads, stores, indexes, summarises, or uploads prompts, output, pane content,
   files, diffs, or terminal history;
5. adds telemetry, analytics, crash upload, cloud sync, accounts, shared
   dashboards, remote administration, or maintainer access;
6. makes synthetic content or AI interaction appear to originate from this
   project rather than the named third-party agent; or
7. changes licensing, monetisation, support, distribution, or intended-purpose
   facts relevant to the open-source exception or operator roles.

## Conclusion

For the default v2 boundary, the evidence supports a **likely-not-an-AI-system**
classification and no independent provider, high-risk, Article 50, or Article 4
duty for the plugin maintainer. GDPR can still apply to an operator's transient
local handling of personal metadata, but the maintainer is unlikely to be that
processing's controller or processor when no data leaves the installation.

The main hardening issue exposed by this review is documentation scope: do not
extend v2's “content-blind” statement to the optional legacy mode while that
mode retains pane/payload inspection. Preserve the model-free, local-only,
content-blind, no-history design as an explicit release boundary and repeat this
review whenever one of the change triggers occurs.
