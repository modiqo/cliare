# 05 - Evidence and Command Shape Spec

> **Scope:** Durable artifacts, evidence schema, normalized command-shape IR, confidence model, and export surfaces.
> **Status:** Draft

---

## Summary

CLIARE has two primary technical artifacts:

1. **Evidence Log**
   - append-only observations from probing
   - replayable
   - raw or redacted
   - source of truth for inference

2. **Command Shape Catalog**
   - normalized inferred model of the CLI
   - probabilistic
   - suitable for agents, CI, reports, and tool generation

The evidence log is analogous to source code. The shape catalog is analogous to compiled IR.

---

## Design Rule

Every nontrivial shape field must answer:

```text
How do you know?
How confident are you?
Which observations support or contradict it?
Which model produced this claim?
```

Without that, black-box CLI inference becomes brittle.

---

## Evidence Log

File:

```
.cliare/evidence.jsonl
```

Each line is a JSON object.

Top-level fields:

```json
{
  "schema_version": "cliare.evidence.v1",
  "event_id": "e_000001",
  "run_id": "run_abc",
  "timestamp": "2026-06-13T00:00:00Z",
  "kind": "process_completed",
  "probe_id": "p_000001",
  "payload": {}
}
```

Kinds:

- `run_started`
- `run_checkpoint`
- `probe_scheduled`
- `process_started`
- `process_completed`
- `stdout_artifact`
- `stderr_artifact`
- `filesystem_diff`
- `network_event`
- `process_event`
- `classifier_annotation`
- `redaction_event`
- `run_finished`

---

## Probe Event

```json
{
  "schema_version": "cliare.evidence.v1",
  "event_id": "e_000010",
  "kind": "probe_scheduled",
  "probe_id": "p_000004",
  "payload": {
    "argv": ["mycli", "project", "list", "--help"],
    "profile": "safe",
    "intent": "help_for_candidate_command",
    "risk": {
      "metadata": 0.95,
      "read": 0.04,
      "write": 0.01
    },
    "reason": "candidate command discovered from root help"
  }
}
```

---

## Process Completion Event

```json
{
  "schema_version": "cliare.evidence.v1",
  "event_id": "e_000011",
  "kind": "process_completed",
  "probe_id": "p_000004",
  "payload": {
    "argv": ["mycli", "project", "list", "--help"],
    "exit_code": 0,
    "signal": null,
    "duration_ms": 43,
    "timed_out": false,
    "stdout": {
      "inline": "Usage: mycli project list [--format <FORMAT>]...",
      "sha256": "...",
      "truncated": false,
      "redacted": false
    },
    "stderr": {
      "inline": "",
      "sha256": "...",
      "truncated": false,
      "redacted": false
    }
  }
}
```

Large outputs should be stored by artifact reference:

```json
{
  "artifact_ref": "artifacts/stdout/p_000004.txt",
  "sha256": "...",
  "bytes": 89231
}
```

---

## Filesystem Diff Event

```json
{
  "schema_version": "cliare.evidence.v1",
  "event_id": "e_000012",
  "kind": "filesystem_diff",
  "probe_id": "p_000004",
  "payload": {
    "created": [],
    "modified": [],
    "deleted": [],
    "outside_sandbox": []
  }
}
```

For a config-writing command:

```json
{
  "created": [
    {
      "path": "$HOME/.mycli/config.json",
      "class": "home_config",
      "size": 241,
      "sha256": "...",
      "content_stored": false
    }
  ]
}
```

---

## Network Event

```json
{
  "schema_version": "cliare.evidence.v1",
  "event_id": "e_000013",
  "kind": "network_event",
  "probe_id": "p_000021",
  "payload": {
    "event": "connect_attempt",
    "destination_host": "api.example.com",
    "destination_port": 443,
    "protocol": "tcp",
    "allowed": false,
    "policy": "deny"
  }
}
```

---

## Classifier Annotation

Classifier annotations are derived evidence, not raw observations.

```json
{
  "schema_version": "cliare.evidence.v1",
  "event_id": "e_000040",
  "kind": "classifier_annotation",
  "probe_id": "p_000004",
  "payload": {
    "classifier": "help_parser.clap_like.v1",
    "annotations": [
      {
        "type": "flag_mention",
        "command_path": ["mycli", "project", "list"],
        "flag": "--format",
        "value_hint": "FORMAT",
        "description": "Output format"
      }
    ]
  }
}
```

Annotations must reference the raw probe event. If the classifier changes, annotations can be regenerated.

---

## Command Shape Catalog

File:

```
.cliare/shape.json
```

Top-level:

```json
{
  "schema_version": "cliare.command-shape.v1",
  "target": {
    "name": "mycli",
    "binary_path": "./dist/mycli",
    "reported_version": "1.2.3"
  },
  "fingerprint": {},
  "commands": [],
  "global_flags": [],
  "environment": [],
  "outputs": [],
  "contradictions": [],
  "coverage": {},
  "models": {
    "inference": "cliare-infer-v1"
  }
}
```

---

## Command Object

```json
{
  "id": "mycli.project.list",
  "argv": ["mycli", "project", "list"],
  "display": "mycli project list",
  "summary": {
    "text": "List projects",
    "confidence": 0.82,
    "evidence": ["e_000004"]
  },
  "visibility": {
    "class": "public",
    "confidence": 0.91,
    "evidence": ["e_000001", "e_000004"]
  },
  "existence": {
    "probability": 0.98,
    "evidence": ["e_000001", "e_000004", "e_000020"]
  },
  "aliases": ["ls"],
  "flags": [],
  "positionals": [
    {
      "name": "project_id",
      "required": true,
      "variadic": false,
      "evidence": ["e_000004:usage line 2"]
    }
  ],
  "usage_observed": true,
  "stdin": {},
  "stdout": {},
  "stderr": {},
  "exit_codes": [],
  "side_effects": {},
  "examples": [],
  "scores": {}
}
```

---

## Flag Object

```json
{
  "name": "--format",
  "short": "-f",
  "description": {
    "text": "Output format",
    "confidence": 0.78,
    "evidence": ["e_000004"]
  },
  "existence": {
    "probability": 0.96,
    "evidence": ["e_000004", "e_000031", "e_000032"]
  },
  "arity": {
    "class": "one",
    "probability": 0.94,
    "evidence": ["e_000004", "e_000033"]
  },
  "value_kind": "required",
  "value_name": "kind",
  "value_schema": {
    "schema": {
      "type": "string",
      "enum": ["json", "table", "yaml"]
    },
    "confidence": 0.87,
    "evidence": ["e_000034", "e_000035"]
  },
  "required": false,
  "repeatable": false,
  "placement": {
    "supports_equals": true,
    "supports_space": true,
    "after_positionals": "unknown"
  }
}
```

The MVP shape emitted by the current reference implementation uses compact flag grammar fields:

| Field | Meaning |
|---|---|
| `value_kind` | `boolean`, `required`, or `optional` |
| `value_name` | normalized placeholder such as `file`, `kind`, or `target` |
| `required` | true when help text marks the flag as required |
| `repeatable` | true when help text marks repeated values with `...`, "repeatable", or similar wording |

The richer probabilistic `arity`, `value_schema`, `required`, `repeatable`, and placement objects remain the target standard form. The compact fields are the MVP bridge from black-box help text to useful agent call contracts.

---

## Positional Object

```json
{
  "name": "project_id",
  "index": 0,
  "required": {
    "value": true,
    "probability": 0.92,
    "evidence": ["e_000051", "e_000052"]
  },
  "arity": {
    "class": "one",
    "probability": 0.89,
    "evidence": ["e_000053"]
  },
  "value_schema": {
    "schema": {
      "type": "string",
      "pattern": "^[A-Za-z0-9_-]+$"
    },
    "confidence": 0.42,
    "evidence": ["e_000054"]
  },
  "semantic_type": {
    "class": "resource_id",
    "probability": 0.61,
    "evidence": ["e_000051"]
  }
}
```

---

## Output Contract

```json
{
  "stdout": {
    "kind": {
      "class": "json",
      "probability": 0.91,
      "evidence": ["e_000061", "e_000062"]
    },
    "schema": {
      "schema": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "id": { "type": "string" },
            "name": { "type": "string" }
          }
        }
      },
      "confidence": 0.74,
      "evidence": ["e_000061"]
    },
    "clean_non_tty": {
      "value": true,
      "probability": 0.96,
      "evidence": ["e_000061"]
    }
  },
  "stderr": {
    "usage": "diagnostics",
    "confidence": 0.67
  }
}
```

Output kinds:

- `json`
- `ndjson`
- `yaml`
- `toml`
- `csv`
- `table`
- `plain_text`
- `mixed`
- `binary`
- `empty`
- `unknown`

---

## Exit Codes

```json
{
  "code": 2,
  "meaning": {
    "class": "usage_error",
    "probability": 0.84,
    "evidence": ["e_000071", "e_000072"]
  },
  "stable": {
    "value": true,
    "probability": 0.81,
    "evidence": ["e_000071", "e_000073"]
  }
}
```

Exit-code classes:

- `success`
- `usage_error`
- `auth_error`
- `not_found`
- `network_error`
- `permission_error`
- `partial_success`
- `internal_error`
- `unknown_error`

---

## Side-Effect Object

```json
{
  "class": {
    "value": "read",
    "probability": 0.86,
    "evidence": ["e_000081", "e_000082"]
  },
  "risk": {
    "read": 0.86,
    "write": 0.08,
    "destructive": 0.01,
    "auth": 0.02,
    "network": 0.03
  },
  "mitigations": [
    {
      "kind": "dry_run",
      "supported": true,
      "probability": 0.78,
      "evidence": ["e_000083"]
    }
  ],
  "filesystem": {
    "writes": [],
    "confidence": 0.71
  },
  "network": {
    "attempts": [],
    "confidence": 0.64
  }
}
```

Side-effect classes:

- `metadata`
- `read`
- `cache_write`
- `local_write`
- `remote_write`
- `destructive`
- `auth`
- `network`
- `interactive`
- `unknown`

---

## Contradictions

Contradictions are first-class because they are highly actionable.

```json
{
  "id": "contradiction.flag.help_runtime.001",
  "kind": "help_runtime_mismatch",
  "severity": "high",
  "subject": "mycli project list --format",
  "claims": [
    {
      "claim": "help says --format exists",
      "probability": 0.92,
      "evidence": ["e_000004"]
    },
    {
      "claim": "runtime rejects --format",
      "probability": 0.89,
      "evidence": ["e_000031"]
    }
  ],
  "impact": "Agents are likely to attempt a documented flag that fails at runtime."
}
```

Contradiction kinds:

- `help_runtime_mismatch`
- `completion_runtime_mismatch`
- `help_completion_mismatch`
- `version_inconsistent`
- `output_contract_mismatch`
- `exit_code_unstable`
- `safety_claim_mismatch`

---

## Evidence Strength

Evidence sources have different reliability.

Initial source reliability priors:

| Source | Reliability |
|--------|-------------|
| runtime accepts valid invocation | very high |
| runtime rejects invalid invocation as expected | high |
| completion suggests command or flag | high |
| help usage advertises syntax | medium-high |
| help prose mentions behavior | medium |
| unknown-command suggestions | medium |
| docs/man pages | low-medium |
| name heuristic | low |

Reliability should be calibrated over benchmark data.

---

## Tool Export

CLIARE should be able to export safe, high-confidence commands as tools.

Command:

```sh
cliare tools .cliare/shape.json --safe-only --min-confidence 0.85
```

Tool output:

```json
{
  "name": "mycli_project_list",
  "description": "List projects",
  "input_schema": {
    "type": "object",
    "properties": {
      "format": {
        "type": "string",
        "enum": ["json", "table", "yaml"],
        "default": "json"
      }
    }
  },
  "execution": {
    "argv_template": ["mycli", "project", "list", "--format", "{format}"],
    "safety": "read",
    "confidence": 0.89
  },
  "output": {
    "kind": "json",
    "confidence": 0.91
  }
}
```

The exporter should not include low-confidence or high-risk commands by default.

---

## Redaction

Evidence may contain secrets. The evidence model must support redaction.

Redaction event:

```json
{
  "kind": "redaction_event",
  "payload": {
    "target_event": "e_000011",
    "field": "stdout.inline",
    "redactor": "token-pattern-v1",
    "replacement": "[REDACTED:token]",
    "original_sha256": "..."
  }
}
```

Redacted evidence can still be used for many claims:

- command exists
- exit code
- output kind
- file write occurred
- network attempt occurred

But some claims may lose confidence:

- exact output schema
- enum extraction
- error wording

---

## Schema Versioning

All artifacts need explicit schema versions.

Version policy:

- additive fields are minor-compatible
- removed or retyped fields require new major schema
- score model version is separate from artifact schema
- evidence logs should be forward-preserved even if old inference cannot read every field

Recommended:

```text
cliare.evidence.v1
cliare.command-shape.v1
cliare.scorecard.v1
cliare.score-model.v1
cliare.infer-model.v1
```

---

## MVP Spec

The MVP command shape can support:

- target metadata
- binary fingerprint
- commands
- command existence confidence
- flags
- flag arity
- positionals
- output kind
- side-effect class
- contradictions
- score references

Do not delay MVP for perfect schema coverage. But do require evidence references and confidence from day one.
