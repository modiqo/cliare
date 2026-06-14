# 04 - Probe Sandbox Runtime

> **Scope:** How CLIARE executes arbitrary CLIs safely and reproducibly while collecting meaningful side-effect evidence.
> **Status:** Reference Design

---

## Summary

CLIARE exercises target CLIs in a controlled runtime after install or build, then tears down that runtime after measurement. The sandbox is part of the measurement environment, not an implementation detail.

The sandbox serves four purposes:

1. Protect the host from accidental or malicious side effects.
2. Make measurements reproducible.
3. Observe filesystem, network, process, and environment behavior.
4. Give the scoring model evidence about safety and automation readiness.

Default rule:

> Every target command is run as `execve(argv)` or equivalent, never through shell interpolation.

---

## Probe Profiles

CLIARE supports multiple probing profiles because different measurement contexts have different risk and coverage tradeoffs.

### `safe`

Only probes expected to be non-mutating:

- `--help`
- `-h`
- `help`
- `--version`
- `version`
- completion generation
- unknown command
- unknown flag
- missing value
- invalid enum

No ordinary business command is run unless it is clearly help/version/completion.

Use cases:

- first audit
- untrusted vendor CLI
- public leaderboard base tier
- local developer exploration

### `read`

Allows likely read-only commands:

- `list`
- `get`
- `show`
- `status`
- `describe`
- `inspect`
- `config get`
- `whoami`

Still no mutating commands unless dry-run is proven.

Use cases:

- CI measurement for project-owned CLI
- agent harness catalog generation
- platform CLI audit

### `fixture`

Allows commands to operate on generated local fixtures:

- temp directories
- fake config files
- dummy project files
- local stub services
- fake credentials

Use cases:

- build tools
- formatters
- linters
- project generators
- CLIs whose shape requires a valid workspace

### `auth`

Allows user-provided scoped credentials through explicit env allowlist.

Use cases:

- SaaS CLIs
- cloud CLIs
- enterprise internal tools

This profile should never be used for public certification without strong redaction and provenance controls.

### `full`

Allows broader execution. This is an explicit unsafe mode.

Use cases:

- maintainers testing their own CLI deeply
- local private analysis
- research harnesses

Public leaderboard should not accept `full` as a comparable certified profile.

---

## Default Environment

Every probe starts with a minimal environment:

```text
CI=1
NO_COLOR=1
TERM=dumb
HOME=<sandbox>/home
PWD=<sandbox>/work
TMPDIR=<sandbox>/tmp
XDG_CONFIG_HOME=<sandbox>/xdg/config
XDG_CACHE_HOME=<sandbox>/xdg/cache
XDG_DATA_HOME=<sandbox>/xdg/data
```

PATH should be controlled:

```text
PATH=<directory-containing-target>:<minimal-system-path>
```

Environment variables inherited from the host should be denied by default, especially:

- tokens
- cloud credentials
- SSH agent sockets
- GitHub tokens
- npm tokens
- kubeconfig
- AWS env vars
- GCP env vars
- Azure env vars

Allowlisting is explicit:

```yaml
environment:
  allow:
    - MYCLI_TEST_TOKEN
```

---

## Filesystem Layout

Each run gets a root:

```
<run-root>/
  home/
  work/
  tmp/
  xdg/
    config/
    cache/
    data/
  fixtures/
  artifacts/
```

Each probe can either share the run root or receive a probe-specific overlay.

Shared run root is useful for discovering config behavior across commands.

Probe-specific overlay is useful for isolation and reproducibility.

Recommended default:

- bootstrap/help probes can share one sandbox root
- side-effect probes should use fresh probe roots
- deterministic repeats should use fresh roots unless testing statefulness

---

## Filesystem Diffing

Before each probe:

1. Snapshot sandbox files.
2. Run target.
3. Snapshot sandbox files again.
4. Compute diff.

Diff records:

- created files
- modified files
- deleted files
- file sizes
- content hashes
- path classes
- whether path is inside allowed root

Example:

```json
{
  "created": [
    {
      "path": "$HOME/.mycli/config.json",
      "size": 128,
      "sha256": "...",
      "class": "config"
    }
  ],
  "modified": [],
  "deleted": []
}
```

Path classes:

| Class | Meaning |
|-------|---------|
| `cwd` | inside working directory |
| `home` | inside sandbox HOME |
| `xdg_config` | inside XDG config |
| `xdg_cache` | inside XDG cache |
| `tmp` | inside sandbox temp |
| `fixture` | inside generated fixture |
| `outside` | attempted outside sandbox or observed outside write |

Writes inside sandbox are not automatically bad. They are evidence.

For example:

- `--help` writing config is suspicious.
- `login` writing config is expected.
- `format` modifying input file is mutating.
- `list` writing cache may be acceptable but should be disclosed.

---

## Network Policy

Certified mode should default to network denied.

Network evidence states:

| State | Meaning |
|-------|---------|
| `denied` | network was blocked and attempts were recorded if possible |
| `stubbed` | calls routed to local fake services |
| `allowed` | network allowed by user policy |
| `unknown` | backend cannot observe or enforce network |

Network event:

```json
{
  "kind": "connect_attempt",
  "destination": "api.vendor.example:443",
  "protocol": "tcp",
  "allowed": false,
  "probe_id": "p_0012"
}
```

Network denial can reveal useful facts:

- command requires remote service
- command attempts auth refresh
- help command phones home
- completion command calls network

This affects safety and determinism scores.

---

## Process Policy

CLIARE should record subprocess behavior where supported.

Fields:

- spawned process path
- argv hash
- exit code
- duration
- whether process escaped sandbox policy

Some CLIs legitimately spawn:

- git
- pager
- browser opener
- editor
- language runtime
- package manager

But agent readiness improves when noninteractive mode avoids pagers, browsers, and editors.

Findings:

```text
Command opened pager despite CI=1.
Command attempted to open browser for auth.
Command spawned editor because required flag was missing.
```

---

## Timeout and Output Limits

Every probe needs:

- wall-clock timeout
- stdout byte limit
- stderr byte limit
- combined output limit
- process tree kill behavior

Defaults:

```yaml
sandbox:
  timeout_ms: 5000
  output_limit_bytes: 1048576
  kill_grace_ms: 500
```

Timeout is evidence. A command that hangs when missing input is less agent-ready than one that exits with a parseable error.

---

## TTY and Non-TTY Behavior

Agents usually run in noninteractive contexts. CLIARE should test non-TTY behavior by default.

Optional probes can compare:

- non-TTY
- pseudo-TTY
- `CI=1`
- `NO_COLOR=1`
- `TERM=dumb`
- `TERM=xterm-256color`

Findings:

- prints ANSI color despite `NO_COLOR=1`
- prompts interactively in CI
- uses pager in non-TTY
- progress spinner appears in stdout
- JSON output polluted by progress text

These are direct output and execution score inputs.

---

## Probe Risk Classification

Before running a candidate command, classify risk.

Risk classes:

| Class | Examples | Default Action |
|-------|----------|----------------|
| `metadata` | help, version, completion | run |
| `parse_negative` | unknown flag, missing value | run |
| `read_likely` | list, get, show, status | run in read profile |
| `write_likely` | create, update, delete, deploy | require dry-run/fixture/full |
| `destructive_likely` | delete, destroy, revoke, reset | skip unless explicit |
| `auth` | login, logout, token | skip unless auth profile |
| `unknown` | unclear command | metadata probes only |

Risk should be probabilistic, not absolute:

```json
{
  "read": 0.72,
  "write": 0.18,
  "destructive": 0.05,
  "auth": 0.05
}
```

---

## Dry-Run Discovery

For mutating commands, CLIARE should search for mitigation flags:

- `--dry-run`
- `--dry-run=client`
- `--check`
- `--plan`
- `--preview`
- `--validate`
- `--no-op`
- `--what-if`
- `--diff`
- `--confirm`
- `--yes`
- `--force`

Positive safety evidence:

- help advertises dry-run
- command accepts dry-run
- dry-run avoids filesystem/network side effects
- dry-run output clearly describes planned action

Negative evidence:

- destructive verb has no dry-run
- command accepts `--yes` but no preview mode
- dry-run still performs side effects

---

## Sandbox Backends

CLIARE should support multiple backends.

### Portable Backend

Works everywhere:

- temp dirs
- env isolation
- timeout
- process execution
- output capture
- file diffing inside sandbox

Does not strongly prevent outside writes or network calls.

### Container Backend

Uses Docker/Podman:

- stronger filesystem isolation
- network deny
- reproducible image
- resource limits

Useful in CI.

### Platform Native Backend

Possible advanced backends:

- macOS sandbox-exec or EndpointSecurity-based tracing
- Linux namespaces/seccomp/bubblewrap
- Windows Job Objects/AppContainer

These can be added later.

Certified leaderboard profiles should specify which backend class was used.

---

## Cleanup Semantics

After run:

1. Flush evidence log.
2. Save selected artifacts.
3. Redact sensitive values.
4. Remove sandbox roots unless `--keep-sandbox` is set.

Debug option:

```sh
cliare measure ./mycli --keep-sandbox on-failure
```

Modes:

| Mode | Behavior |
|------|----------|
| `never` | always delete sandbox |
| `on-failure` | keep only if run fails |
| `always` | keep for debugging |

CI default should be `never`.

---

## Why Post-Install Execution Is Correct

The user asked whether CLIARE should exercise the CLI post-install and kill the sandbox after computation.

Yes.

Reasons:

- installed binary includes packaging behavior
- generated completions may differ from source
- runtime dependencies matter
- path resolution matters
- version reporting matters
- CI environment matters
- agents use installed artifacts, not source trees

CLIARE should test what agents will actually run.

---

## What Not To Run

Even in deeper profiles, CLIARE should avoid:

- irreversible destructive commands without dry-run
- commands requiring real payments
- commands that send email/SMS/webhooks
- commands that publish artifacts publicly
- commands that delete remote state
- commands that rotate secrets
- commands that alter production resources

If a user wants these tested, they should provide fixtures or stubs.

---

## Safety Finding Examples

### Good

```text
deploy supports --dry-run and produces JSON plan output.
```

### Warning

```text
config get wrote cache file under $XDG_CACHE_HOME.
```

### Failure

```text
delete accepted resource id and --yes but no dry-run or confirmation-free preview was discovered.
```

### Critical

```text
status attempted outbound network during safe profile with no user-provided credentials.
```

---

## Initial Sandbox Scope

The implemented sandbox currently provides:

- temp HOME/cwd/XDG config-cache-data
- env deny-by-default
- no shell interpolation
- timeout
- output limit
- evidence recording
- scorecard/report/cache metadata

The current executor creates a deterministic sandbox under the artifact directory, clears inherited environment variables, restores only a small allowlist needed for portable execution, sets `CLIARE=1`, and runs probes from the sandbox cwd. It records sandbox metadata in `evidence.jsonl`, `scorecard.json`, `report.md`, and `measure-cache.json`.

Still planned:

- file diffing inside sandbox
- cleanup policy controls
- network blocking
- syscall or native sandbox tracing

Network blocking and syscall tracing can come later, but the schema should have fields for them from the beginning.
