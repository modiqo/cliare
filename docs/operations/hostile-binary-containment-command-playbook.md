# Hostile-Binary Containment Command Playbook

> **Scope:** Concrete operator commands for validating future hostile-binary containment backends and classifying observed actions.
> **Status:** Command reference. This is not current CLIARE runtime behavior.

---

## Rule

Contain first, classify second.

Do not run a binary on the host and then decide whether it was hostile. A hostile-binary profile must place the target inside a deny-by-default boundary before `exec`. Classification is then based on observed or denied actions inside that boundary.

Current CLIARE `isolated` mode is not this boundary. It is an artifact-local filesystem and environment isolation mode for safe probing and side-effect evidence. Current artifacts explicitly record `hostile_binary_containment: false`.

---

## Local Tool Preflight

Run these commands before using any recipe in this document:

```sh
command -v docker
docker version
docker run --help | sed -n '1,220p'
```

The Docker recipe below requires a running Docker daemon. On this development machine the Docker client is installed, but the daemon was not running when this document was written:

```text
failed to connect to the docker API at unix:///Users/chetanconikee/.docker/run/docker.sock
```

These macOS commands may exist, but they are not accepted as CLIARE hostile-binary containment backends:

```sh
command -v sandbox-exec
man sandbox-exec | sed -n '1,40p'
```

`sandbox-exec` is marked deprecated by its own man page on macOS. On this development machine even a permissive profile failed to apply:

```sh
sandbox-exec -p '(version 1) (allow default)' /usr/bin/true
```

Observed result:

```text
sandbox-exec: sandbox_apply: Operation not permitted
```

`dtruss` is tracing, not containment. On this development machine it also failed under the current system policy:

```sh
dtruss /usr/bin/true
```

Observed result:

```text
dtrace: system integrity protection is on, some features will not be available
dtrace: failed to initialize dtrace: Operation not permitted
```

---

## Docker Containment Candidate

This recipe is for Linux binaries that can execute inside the selected Linux container image. Docker Desktop on macOS runs Linux containers in a Linux VM; it does not execute macOS binaries inside the container.

Prepare a target bundle:

```sh
export TARGET="$PWD/target/x86_64-unknown-linux-musl/release/cliare"
test -x "$TARGET"

export RUNROOT="$(mktemp -d)"
mkdir -p "$RUNROOT/target" "$RUNROOT/work"
cp "$TARGET" "$RUNROOT/target/target"
chmod 0555 "$RUNROOT/target/target"
```

Ensure the image is available:

```sh
docker image inspect debian:bookworm-slim >/dev/null || docker pull debian:bookworm-slim
```

Run the target with a deny-by-default container posture:

```sh
docker run --rm \
  --network none \
  --read-only \
  --cap-drop=ALL \
  --security-opt=no-new-privileges \
  --pids-limit=64 \
  --memory=256m \
  --cpus=1 \
  --tmpfs /tmp:rw,noexec,nosuid,nodev,size=64m \
  --mount type=bind,src="$RUNROOT/target",dst=/target,readonly \
  --mount type=bind,src="$RUNROOT/work",dst=/work \
  --user 65534:65534 \
  --workdir /work \
  debian:bookworm-slim \
  /target/target --help
```

Capture stdout, stderr, and exit status:

```sh
docker run --rm \
  --network none \
  --read-only \
  --cap-drop=ALL \
  --security-opt=no-new-privileges \
  --pids-limit=64 \
  --memory=256m \
  --cpus=1 \
  --tmpfs /tmp:rw,noexec,nosuid,nodev,size=64m \
  --mount type=bind,src="$RUNROOT/target",dst=/target,readonly \
  --mount type=bind,src="$RUNROOT/work",dst=/work \
  --user 65534:65534 \
  --workdir /work \
  debian:bookworm-slim \
  /target/target --help \
  >"$RUNROOT/stdout" \
  2>"$RUNROOT/stderr"
printf '%s\n' "$?" > "$RUNROOT/status"
```

Inspect captured output:

```sh
cat "$RUNROOT/status"
sed -n '1,120p' "$RUNROOT/stdout"
sed -n '1,120p' "$RUNROOT/stderr"
find "$RUNROOT/work" -maxdepth 3 -type f -print
```

This is a containment candidate, not a complete hostile-action classifier. Docker `--network none`, `--read-only`, `--cap-drop=ALL`, `--security-opt=no-new-privileges`, PID limits, memory limits, CPU limits, tmpfs, bind mounts, and non-root user execution reduce available capabilities. They do not by themselves produce structured denied-syscall evidence for every attempted action.

---

## Current Evidence Classification Commands

These commands operate on CLIARE's current `evidence.jsonl` format. They classify observed process outcomes and persistent filesystem side effects. They do not observe denied syscalls that current CLIARE does not record.

Set the evidence path:

```sh
export EVIDENCE="$PWD/.cliare/evidence.jsonl"
test -f "$EVIDENCE"
```

List probes with any persistent filesystem side effect:

```sh
jq -r '
  select(.kind=="process_completed")
  | .payload
  | select((.side_effects.total // 0) > 0)
  | [.probe_id, (.side_effects.total|tostring), (.argv|join(" "))]
  | @tsv
' "$EVIDENCE"
```

Classify credential-like filesystem side effects as hostile-policy violations:

```sh
jq -r '
  select(.kind=="process_completed")
  | .payload as $p
  | $p.side_effects.changes[]?
  | select((.path // "") | test("(^|/)(\\.ssh|\\.aws|credentials|id_rsa|token|secret)($|/)"))
  | ["hostile_policy_violation", "credential_path_side_effect", $p.probe_id, .kind, .region, .path]
  | @tsv
' "$EVIDENCE"
```

Classify truncated side-effect scans as unmeasured safety:

```sh
jq -r '
  select(.kind=="process_completed")
  | .payload
  | select(.side_effects.truncated == true)
  | ["unmeasured_safety", "side_effect_scan_truncated", .probe_id, (.side_effects.truncation_reason // "unknown")]
  | @tsv
' "$EVIDENCE"
```

Classify timed-out probes as bounded execution failures:

```sh
jq -r '
  select(.kind=="process_completed")
  | .payload
  | select(.status.state == "timed_out")
  | ["bounded_execution_failure", "probe_timeout", .probe_id, (.argv|join(" "))]
  | @tsv
' "$EVIDENCE"
```

Classify network-looking diagnostics as diagnostics only, not syscall evidence:

```sh
jq -r '
  select(.kind=="process_completed")
  | .payload
  | select(
      ((.stdout.text // "") | test("(?i)(network|connect|dns|tls|http|socket|timeout)"))
      or
      ((.stderr.text // "") | test("(?i)(network|connect|dns|tls|http|socket|timeout)"))
    )
  | ["diagnostic_only", "network_or_connectivity_text", .probe_id, (.argv|join(" "))]
  | @tsv
' "$EVIDENCE"
```

---

## Required Future Evidence For Real Hostile Classification

The current evidence format can classify persistent side effects and target diagnostics. A real hostile-binary backend needs structured attempted-action evidence from the containment layer.

The minimum future event fields are:

```json
{
  "action": "network_connect",
  "target": "203.0.113.10:443",
  "verdict": "denied",
  "policy": "network=deny",
  "classification": "hostile_policy_violation"
}
```

Concrete action classes to record:

- `network_connect`
- `file_read`
- `file_write`
- `process_spawn`
- `process_ptrace`
- `device_access`
- `procfs_access`
- `resource_limit_exceeded`

Concrete verdicts:

- `allowed`
- `denied`
- `truncated`
- `unobserved`

Concrete classifications:

- `allowed`
- `suspicious`
- `hostile_policy_violation`
- `unmeasured`

---

## Do Not Claim

Do not claim hostile-binary containment from any of these alone:

```sh
cliare measure ./target --execution-mode isolated
```

```sh
sandbox-exec -p '(version 1) (allow default)' ./target
```

```sh
dtruss ./target
```

Current valid wording is:

> CLIARE isolated mode reduces accidental local writes and records side effects in configured sandbox regions. It is not hostile-binary containment.

Future valid wording requires a backend that actually enforces the boundary before `exec` and records attempted denied actions as evidence.
