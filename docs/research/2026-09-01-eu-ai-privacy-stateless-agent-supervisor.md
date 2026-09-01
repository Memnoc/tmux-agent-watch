# EU AI Act and GDPR considerations for a stateless agent supervisor

Date: 2026-09-01  
Status: architecture research, not legal advice

## Question and assumptions

This note considers `tmux-agent-watch` v2 as a local, open-source Rust CLI/tmux plugin which:

- discovers and supervises user-operated Codex, Claude, and OpenCode processes;
- performs no model inference, training, fine-tuning, evaluation, or content generation itself;
- supplies no hosted service and sends no data to the project maintainer;
- retains no prompts, messages, generated content, task history, usage analytics, crash reports, or telemetry; and
- derives a live UI from tmux/Git/process state, with only the configuration needed to operate the plugin.

Changing any of these facts can change the analysis. In particular, cloud sync, remote diagnostics, prompt capture, workforce analytics, output ranking, autonomous task allocation, or marketing the combined product under the project's own name should trigger a fresh review.

## Bottom line

On these assumptions, the better view is that the supervisor is ordinary deterministic software around separately supplied AI systems, not itself an “AI system”: it does not infer how to generate predictions, content, recommendations, or decisions. That conclusion follows from the Act's functional definition, but it is an interpretation rather than a regulator ruling about this product. The definition appears in Article 3(1) of the [consolidated AI Act](https://eur-lex.europa.eu/eli/reg/2024/1689/2026-07-27/eng).

The project therefore likely is not an AI-system provider merely by publishing this supervisor. A user or organisation may nevertheless be a “deployer” of the underlying Codex/Claude/OpenCode AI system in professional use; the personal, non-professional use exclusion is expressly built into the deployer definition in Article 3(4). Which entity is provider/deployer of a particular underlying CLI is product- and deployment-specific.

“No retention” is a sound conservative architecture policy, but it is not a complete exemption from privacy law. Reading or displaying an identifiable person's name, path, branch, prompt, terminal content, or activity can itself be GDPR “processing” even when nothing is written to disk: Article 4(2) includes consultation and use, while Article 4(1) defines personal data broadly. See the [GDPR, Articles 4(1)–(2)](https://eur-lex.europa.eu/eli/reg/2016/679/art_4/oj). The decisive questions are what data the program touches and who determines why and how, not just whether the data persists.

## AI Act applicability

### Why the supervisor likely falls outside the AI-system definition

Article 3(1) requires a machine-based system which, for explicit or implicit objectives, **infers from input how to generate outputs** such as predictions, content, recommendations, or decisions. A rules-based process scanner, state machine, and terminal renderer which maps explicit observations to predefined status labels lacks that inference characteristic. Recital 12 also says the definition should distinguish AI from simpler traditional software and should not cover systems based solely on human-defined rules that automatically execute operations. See the [official AI Act text](https://eur-lex.europa.eu/eli/reg/2024/1689/oj/eng).

Architecture should preserve that boundary:

- implement lifecycle classification as explicit, inspectable rules;
- do not embed a model to summarise prompts, infer worker intent, score productivity, prioritise people, or recommend employment-related actions;
- describe the product as a supervisor/integration for third-party AI tools, not as its own AI assistant or model; and
- show which third-party tool is being launched and avoid relabelling its generated output as the project's own.

These are boundary-preserving design choices, not independent legal duties imposed on non-AI software.

### Open source is useful but not the primary conclusion

Article 2(12) says the Act does not apply to AI systems released under free and open-source licences unless they are placed on the market or put into service as high-risk systems, prohibited-practice systems, or systems covered by Article 50. The exception and its limits appear in the [consolidated Act, Article 2(12)](https://eur-lex.europa.eu/eli/reg/2024/1689/2026-07-27/eng). Recitals 102–103 explain that monetisation and personal-data use beyond security, compatibility, or interoperability can defeat relevant open-source treatment; merely publishing in an open repository is not monetisation. See the [official Act, recitals 102–103](https://eur-lex.europa.eu/eli/reg/2024/1689/oj/eng).

Do not rely on this exception as the first line of analysis. If the supervisor is not an AI system at all, it does not need an AI-system exemption. If later versions do become AI systems, counsel should check the exact licence, monetisation/support model, Article 50 exposure, intended purpose, and risk classification.

### The underlying AI systems and role boundaries

The Act defines a provider as the party developing (or having developed) an AI system and placing it on the market or putting it into service under its own name/trademark, including free supply; a deployer is generally the professional entity using it under its authority. See [Article 3(3)–(4)](https://eur-lex.europa.eu/eli/reg/2024/1689/2026-07-27/eng).

Launching a separately installed CLI, passing a user-authored initial prompt, and observing coarse lifecycle state should not by itself make this project provider of the underlying system. But Article 25 can transfer provider obligations for a **high-risk** system where a party rebrands it, substantially modifies it, or changes its intended purpose so it becomes high-risk. See [Article 25](https://eur-lex.europa.eu/eli/reg/2024/1689/2026-07-27/eng). Consequently:

- integrations should remain thin adapters, not forks or rebranded combined AI products;
- documentation should identify third-party ownership and keep their terms/instructions visible;
- the plugin should not override safeguards or repurpose an agent for Annex III decisions; and
- adapters should declare capabilities and data flows rather than normalising away meaningful provider warnings.

The present developer-workspace use is not one of the enumerated high-risk use cases merely because AI is involved. That could change if functionality is sold or intended for employment decisions, worker management/evaluation, education admissions/assessment, biometrics, essential services, law enforcement, migration, justice, or other Article 6/Annex III contexts. The project should explicitly exclude employee scoring, performance monitoring, hiring/firing recommendations, and autonomous allocation of work to people from its intended purpose.

Article 50 transparency duties principally concern providers/deployers of specified AI systems and synthetic content. A terminal cockpit that clearly says “Codex”, “Claude”, or “OpenCode” and merely exposes the user's existing interaction is unlikely to create a separate chatbot-deception problem. If the product later presents generated output through its own conversational persona or creates/edits synthetic media, obtain advice on Article 50 and its exceptions. The relevant obligations are in [AI Act Article 50](https://eur-lex.europa.eu/eli/reg/2024/1689/2026-07-27/eng).

## AI Act dates as at 2026-09-01

The Commission's current official timeline says:

- the Act entered into force on 1 August 2024;
- prohibited practices and the original AI-literacy provisions applied from 2 February 2025;
- governance and general-purpose-model obligations applied from 2 August 2025;
- most remaining provisions and enforcement powers applied from 2 August 2026;
- Annex III high-risk rules apply from 2 December 2027; and
- Annex I product-integrated high-risk rules apply from 2 August 2028.

See the Commission's [AI Act application timeline](https://digital-strategy.ec.europa.eu/en/policies/regulatory-framework-ai) and [enforcement framework](https://digital-strategy.ec.europa.eu/en/policies/enforcement-ai-act). The consolidated legislation reflects amendments effective 27 July 2026; use the [current consolidated text](https://eur-lex.europa.eu/eli/reg/2024/1689/2026-07-27/eng), not only the original 2024 version. Dates and Commission guidance can still change, so re-check before a release that adds AI functionality or approaches the deferred high-risk dates.

## GDPR and privacy applicability

### Roles and scope

If the maintainer receives no user data and cannot access local runtime state, publishing software alone normally does not make the maintainer a processor handling each user's workspace data. Controller/processor status is functional: the controller determines the purposes and essential means of processing, and a processor handles data on a controller's behalf. The EDPB explains this factual role analysis in its [Guidelines 07/2020](https://www.edpb.europa.eu/documents/guideline/guidelines-072020-on-the-concepts-of-controller-and-processor-in-the-gdpr_en).

For professional use, the user or employer will more plausibly be controller for personal data it chooses to expose through the tool. A purely personal/household user's processing may fall under GDPR Article 2(2)(c), but workplace and professional use should not assume that exception. See [GDPR Article 2](https://eur-lex.europa.eu/eli/reg/2016/679/art_2/oj). The project's software design can help users comply, but cannot choose their lawful basis or satisfy their transparency/employment-law duties for them.

The GDPR has applied since 25 May 2018 under [Article 99](https://eur-lex.europa.eu/eli/reg/2016/679/art_99/oj); there is no new 2026 transition for this product.

### What “zero retention” should mean technically

Adopt a precise promise: **no project-controlled collection and no durable storage of content or behavioural history by default**. Avoid claiming “we process no data” if the binary momentarily reads identifiers or terminal state.

The following should be architectural invariants:

1. No project backend, accounts, remote API, telemetry, analytics SDK, advertising identifier, update ping, crash upload, or remote feature flag.
2. Never read pane scrollback, prompts, messages, generated content, clipboard contents, files, diffs, commit bodies, environment-variable values, or command arguments unless a narrowly defined user action strictly requires it. Prefer process identity, tmux IDs, exit/lifecycle hooks, working directory, Git ref, and clean/dirty booleans.
3. Hold derived runtime state only in process memory; discard it on exit. Do not create databases, history files, debug transcripts, shell-history entries containing prompts, or persistent caches. Avoid core dumps containing runtime data where practicable.
4. Store configuration only when required, document every field and location, and never put prompts/task descriptions into tmux global options or environment variables. A user-authored task prompt should be handed directly to the selected agent without copying it into plugin state.
5. Logs default off. An explicit diagnostic mode should redact paths, usernames, branch names, pane contents, prompt arguments, tokens, and environment data; write to a user-chosen destination; show exactly what will be captured; and provide deterministic deletion instructions.
6. No stable cross-session workspace/user identifier. tmux pane/window IDs may be used transiently for current-session routing, not retained as behavioural history.
7. Perform all discovery locally with least privilege. Do not inspect processes owned by other OS users. Bound subprocess execution, avoid secrets in command-line arguments, clear sensitive buffers where feasible, and prevent sensitive values from reaching panic/error output.
8. Treat exported support bundles as a separate, explicit feature with preview, redaction, consent, and local-only output. Do not implement automatic submission.

These invariants operationalise GDPR Article 5's purpose limitation, minimisation, storage limitation, integrity/confidentiality, and accountability principles and Article 25's data protection by design/default requirements. See [GDPR Article 5](https://eur-lex.europa.eu/eli/reg/2016/679/art_5/oj), [Article 25](https://eur-lex.europa.eu/eli/reg/2016/679/art_25/oj), and the EDPB's final [Guidelines 4/2019](https://www.edpb.europa.eu/documents/guideline/guidelines-42019-on-article-25-data-protection-by-design-and-by-default_en). Security controls should be proportionate to risk under [Article 32](https://eur-lex.europa.eu/eli/reg/2016/679/art_32/oj).

### Documentation and user controls

Ship a short privacy/data-flow statement that is testable against the implementation:

- enumerate each local datum read, its immediate purpose, lifetime, and display surface;
- state that the project has no server and receives no workspace data;
- distinguish plugin behaviour from the separate data practices of Codex, Claude, OpenCode, Git hosting, package registries, and update/download channels;
- state that launching an agent may cause that third party to process prompts under its own configuration and terms;
- expose a diagnostics command proving telemetry is absent and listing enabled integrations;
- allow display redaction of repository/path/branch labels for screen sharing; and
- warn organisational deployers that pane labels, branch names, task descriptions, and activity states may be personal data and that they must assess lawful basis, employee notices/consultation, access control, retention, and any DPIA need.

A DPIA is required under GDPR Article 35 only where processing is likely to result in high risk, including specified systematic evaluation/large-scale cases; this local developer tool is not automatically subject to one. An employer that expands it into systematic employee monitoring may reach a different conclusion and should assess national supervisory-authority lists and employment law. See [GDPR Article 35](https://eur-lex.europa.eu/eli/reg/2016/679/art_35/oj).

## Legal requirements versus conservative policy

| Topic | Likely legal position on stated facts | Conservative project policy |
|---|---|---|
| Is v2 an AI system? | Likely no, because deterministic supervision does not infer outputs; fact-sensitive and untested for this product. | Keep classification and orchestration rules explicit and model-free. |
| Open-source AI Act exception | Relevant only if the product/feature is an AI system; exceptions have high-risk, prohibited-practice, and Article 50 limits. | Use a genuine FOSS licence, avoid personal-data monetisation, but never treat FOSS as blanket immunity. |
| Maintainer as GDPR controller/processor | Likely neither for local workspace processing if no data is received or remotely accessible; facts and product governance control. | Operate no backend and build no data-return path. |
| Local transient data | Can still be GDPR processing when personal data is consulted/used. | Read only coarse metadata, minimise in memory, discard immediately. |
| Retention | GDPR permits necessary, lawful retention; it does not require zero retention in all software. | Retain no prompts, messages, outputs, history, telemetry, or diagnostic content by default. |
| Compliance records | Organisational controllers may have accountability/documentation duties even if the plugin stores no content. | Provide a static data-flow manifest and architecture decision record, not user-event logs. |
| AI transparency | Article 50 is unlikely to target a clearly labelled launcher/status UI as a separate conversational AI. | Always name the underlying agent; do not impersonate a human or obscure generated provenance. |

## Matters requiring counsel or a new review

Obtain EU counsel before release if any of the following becomes true:

- the project bundles, fine-tunes, hosts, proxies, evaluates, or markets an AI model/system under its own name;
- it ranks agents or people using inferred quality, emotion, intent, productivity, risk, or performance;
- it is positioned for hiring, worker management, education, essential services, biometrics, law enforcement, migration, justice, safety-critical products, or another Article 6/Annex III setting;
- telemetry, hosted updates, crash collection, cloud sync, shared dashboards, remote administration, or support-bundle upload is introduced;
- the maintainer can access customer installations or determines purposes for their personal-data processing;
- prompts/messages are cached, indexed, summarised, or used for analytics/security beyond momentary forwarding;
- a commercial support/platform model may affect FOSS treatment;
- the UI presents third-party AI output as the project's own conversational service; or
- the product monitors employees across sessions or employers ask to use it for performance decisions.

Also seek local advice on Irish/EU employment and ePrivacy rules if remote communications or workplace surveillance enter scope. The AI Act does not displace GDPR or other privacy law: Article 2 expressly preserves those regimes in the [consolidated Act](https://eur-lex.europa.eu/eli/reg/2024/1689/2026-07-27/eng).

## Recommended decision record

Record “stateless, local-only, content-blind supervision” as a product constraint, not merely a v2 implementation preference. CI can enforce parts of it: dependency allowlists, tests that no network sockets are opened, scans for analytics/crash SDKs, snapshot tests for redaction, and integration tests proving that normal operation creates no files beyond documented configuration. This evidence demonstrates the engineering policy without retaining user activity.
