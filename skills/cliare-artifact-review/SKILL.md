---
name: cliare-artifact-review
description: Use when reviewing a CLIARE measurement artifact directory, explaining score changes, triaging issues, finding evidence, or proposing CLI remediation work from artifact-map.json, scorecard.json, issues.json, command-index.json, shape.json, and evidence.jsonl.
---

# CLIARE Artifact Review

## What This Is

This skill is for reviewing one CLIARE measurement directory. It helps an agent connect the artifact map, scorecard, reviewable issues, persona reports, command index, raw command shape, and runtime evidence without overstating the data.

Use it when a maintainer, harness author, platform engineer, security reviewer, researcher, or release owner asks what a CLIARE run means and what should be fixed next.

Work from the artifact directory. Prefer evidence-backed explanations over raw JSON excerpts.

## Command Discipline

Prefer direct file reads and `jq` over ad hoc scripts. Do not use Python, `cd`, shell redirection, heredocs, or compound shell commands for routine artifact inspection. Keep shell commands simple and pass the artifact path directly to each file argument.

Good:

```sh
jq '{score:.summary.score,top_issues:[.top_issues[] | {id,severity,category,confidence,title}]}' /tmp/cliare-run/persona-harness.json
```

Avoid changing directories, shell redirection, and ad hoc scripts for basic artifact inspection.

## Workflow

1. Identify the persona. If the user does not specify one, default to `maintainer` for CLI implementation work, `harness` for agent routing, `security` for approval/policy, and `platform` for CI gates.

2. Orient to the folder before reading raw artifacts. If `artifact-map.json` is missing, generate it:

```sh
cliare describe <artifact-dir> --write
```

Then read the map:

```sh
jq '{kind:.artifact_kind,health:.health,navigation:.navigation,missing_required:.missing_required,summaries:.summaries}' <artifact-dir>/artifact-map.json
```

3. Read posture:

```sh
jq '{score:.score.total,status:.score.status,model:.score.model,coverage:{commands_discovered:.coverage.commands_discovered,commands_runtime_confirmed:.coverage.commands_runtime_confirmed,traversal_complete:.coverage.traversal_complete,budget_exhausted:.coverage.budget_exhausted,observed_max_depth:.coverage.observed_max_depth,max_depth:.coverage.max_depth,probes_completed:.coverage.probes_completed,max_probes:.coverage.max_probes}}' <artifact-dir>/scorecard.json
```

4. Generate the issue ledger and persona packet if they are missing:

```sh
cliare report maintainer --out <artifact-dir> --write
```

5. Start with the persona table, not a raw dump. Read the matching persona Markdown first, then use JSON only for drill-down:

```sh
jq '{persona:.persona,question:.primary_question,score:.summary.score,top_issues:[.top_issues[] | {id,severity,category,confidence,title,affected_commands:(.affected_commands|length),evidence:(.evidence|length),recommendation,verification:.verification.command}]}' <artifact-dir>/persona-maintainer.json
```

6. Use `issues.json` as the canonical review queue:

```sh
jq '.summary, [.issues[] | {id,severity,category,confidence,title,affected:(.affected_commands|length)}]' <artifact-dir>/issues.json
```

7. Deep dive one issue only after the user chooses a row:

```sh
jq --arg id "issue.output_mode_unprobed" '.issues[] | select(.id==$id) | {what:{id,title,impact,why_it_matters},severity,category,confidence,where:{affected_commands:.affected_commands[0:10],evidence:.evidence[0:5]},how:{recommendation,verification}}' <artifact-dir>/issues.json
```

8. Use `command-index.json` when a developer or harness needs command-level details:

```sh
jq --arg path "adapter new" '.commands[] | select(.path == ($path | split(" "))) | {command,summary,runtime_state,agent_suitability,suitability_reasons,parameters,preconditions,output_contracts,gaps,evidence}' <artifact-dir>/command-index.json
```

9. Use `shape.json` only when raw inference details are needed:

```sh
jq '.gaps[] | {kind,command_path,reason,evidence}' <artifact-dir>/shape.json
```

10. Resolve evidence references before making runtime claims. Strip suffixes after the first colon:

```sh
ref="e_000257:output mode layout row 17"
event="${ref%%:*}"
jq --arg id "$event" 'select(.event_id==$id)' <artifact-dir>/evidence.jsonl
```

## Persona Response Shape

When answering a persona-level question, use a table before details:

| Column | Meaning |
|---|---|
| Priority | Persona-specific rank, usually `P1`, `P2`, and so on. |
| Severity | Release or routing impact. |
| Category | Discovery, grammar, execution, output, safety, recovery, coverage, policy, publishing, or calibration. |
| Confidence | `observed`, `blocked`, `needs_fixture`, `inferred`, or `advisory`. |
| Affected | Number of commands or events in the ledger. |
| Issue | Issue id and short title. |
| Persona action | What this persona should do next. |

After the table, ask which priority row to drill into unless the user already named an issue. Drill-down output should include what the issue means, where it appears, evidence ids, how to address it, and how to verify the fix.

## Large Issue Lists

Use this section when the user asks for all commands behind a large issue such as `issue.help_unavailable`.

Rules:

- Do not use Python or exploratory scripts.
- Do not infer false positives, root causes, or design intent from command names.
- Do not say a command is real or fake unless the `state`, `confidence`, and evidence support that statement.
- Use CLIARE terms from the artifact: `runtime_confirmed`, `precondition_blocked`, `unconfirmed`, and `not_in_shape_catalog`.
- For more than 50 affected commands, first show counts by state. If the user explicitly asked to list all commands, list all in a compact grouped format. Otherwise show samples and provide the exact query for the full list.
- Keep commentary short. A large list is an index, not a diagnosis.

Count affected commands by runtime state:

```sh
jq --arg id "issue.help_unavailable" '[.issues[] | select(.id==$id) | .affected_commands[]] | group_by(.state) | map({state:.[0].state,count:length})' <artifact-dir>/issues.json
```

List every affected command in compact form:

```sh
jq -r --arg id "issue.help_unavailable" '.issues[] | select(.id==$id) | .affected_commands | sort_by(.state, .path)[] | [.state, ((.confidence // 0) | tostring), (.path | join(" ")), .reason] | @tsv' <artifact-dir>/issues.json
```

List one command prefix:

```sh
jq -r --arg id "issue.help_unavailable" --arg prefix "registry" '.issues[] | select(.id==$id) | .affected_commands[] | select((.path | join(" ")) | startswith($prefix)) | [.state, ((.confidence // 0) | tostring), (.path | join(" ")), .reason] | @tsv' <artifact-dir>/issues.json
```

Recommended answer shape for a large list:

- Issue id, title, severity, confidence.
- Counts by `state`.
- Compact list grouped by `state` if explicitly requested.
- One sentence explaining that `inferred` issue confidence means this is a review queue, not proof that every command is defective.
- No additional claims unless backed by cited evidence ids.

## Persona Lenses

- `maintainer`: explain concrete CLI contract changes, fixture additions, and help/output improvements.
- `harness`: separate commands that are ready for routing from commands that need policy, fixtures, or manual review.
- `security`: foreground side effects, credential-like paths, auth/profile gates, and approval constraints.
- `platform`: turn findings into CI thresholds, warnings, exceptions, and guard policy.
- `oss`: decide what can be published credibly with caveats and reproducible artifacts.
- `devrel`: translate findings into public guidance, examples, and roadmap language.
- `research`: preserve evidence IDs, labels, score model, binary fingerprint, and calibration caveats.

## Interpretation Rules

- `observed` means the behavior was directly measured.
- `blocked` means runtime state prevented confirmation; identify the precondition and decide whether help/catalog behavior should bypass it.
- `needs_fixture` means the CLI may be correct, but CLIARE needs safe operands or fixture data to validate the advertised contract.
- `inferred` means the finding is lower confidence and should be resolved through clearer help output, additional traversal, or direct evidence.
- `advisory` means quality guidance, not a release blocker by itself.

## Remediation Output

When presenting high-level findings, use this structure:

- Current score and traversal status.
- Persona-specific table of pressing issues.
- Persona decision: what should happen before routing, approving, publishing, or gating.
- Biggest uncertainty: blocked preconditions, missing fixtures, incomplete traversal, or inferred candidates.

When deep-diving one issue, use this structure:

- Issue id, severity, category, confidence.
- What the issue means in plain language.
- Where it appears: affected commands, command paths, or evidence-only runtime events.
- Evidence event ids with exact argv/status and side effects when relevant.
- How to address it: CLI change, fixture addition, documentation, policy, or traversal rerun.
- Verification command from the issue ledger and the expected score/ledger change.

Avoid dumping entire JSON arrays. Summarize the pattern, cite the minimal evidence needed to reproduce or dismiss it, and keep confidence separate from severity.
