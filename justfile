set positional-arguments

# Override with `just cliare_bin=target/debug/cliare ...` when dogfooding a local build.
cliare_bin := "cliare"

# Default traversal budget for large CLI surfaces.
profile := "deep"
max_depth := "12"
max_probes := "5000"
concurrency := "8"
allowed_drop := "0"

# List available CLIARE command shortcuts.
default:
    @just --list

# Build this repository's CLIARE binary for local dogfooding.
build:
    cargo build --locked --bin cliare

# Print CLIARE's machine-readable command contract.
metadata format="json":
    {{cliare_bin}} metadata --format {{format}}

# Print a role-specific operational playbook.
playbook role="maintainer" cli="mycli" id=cli format="human":
    {{cliare_bin}} playbook {{role}} --target {{cli}} --out .cliare/{{id}} --format {{format}}

# Measure a local CLI in the foreground.
measure cli id=cli:
    {{cliare_bin}} measure {{cli}} \
      --out .cliare/{{id}} \
      --profile {{profile}} \
      --max-depth {{max_depth}} \
      --max-probes {{max_probes}} \
      --concurrency {{concurrency}} \
      --refresh

# Measure a local CLI in the background for large command surfaces.
measure-detached cli id=cli:
    {{cliare_bin}} measure {{cli}} \
      --out .cliare/{{id}} \
      --profile {{profile}} \
      --max-depth {{max_depth}} \
      --max-probes {{max_probes}} \
      --concurrency {{concurrency}} \
      --refresh \
      --detach

# Run a quick smoke measurement with a smaller traversal budget.
measure-quick cli id=cli:
    {{cliare_bin}} measure {{cli}} \
      --out .cliare/{{id}} \
      --profile quick \
      --refresh

# Run the standard maintainer-loop measurement.
measure-standard cli id=cli:
    {{cliare_bin}} measure {{cli}} \
      --out .cliare/{{id}} \
      --profile standard \
      --refresh

# Run a deep foreground measurement with the configured traversal budget.
measure-deep cli id=cli:
    @just profile=deep max_depth={{max_depth}} max_probes={{max_probes}} concurrency={{concurrency}} measure {{cli}} {{id}}

# Measure host/auth-specific behavior. Use only when the target needs local state.
measure-host cli id=cli:
    {{cliare_bin}} measure {{cli}} \
      --out .cliare/{{id}} \
      --profile {{profile}} \
      --execution-mode host \
      --max-depth {{max_depth}} \
      --max-probes {{max_probes}} \
      --concurrency {{concurrency}} \
      --refresh

# Measure an authenticated context into a context suite under `.cliare/<id>`.
measure-auth cli id=cli:
    {{cliare_bin}} measure {{cli}} \
      --out .cliare/{{id}} \
      --context authenticated \
      --auth-state present \
      --execution-mode host \
      --profile {{profile}} \
      --max-depth {{max_depth}} \
      --max-probes {{max_probes}} \
      --concurrency {{concurrency}} \
      --refresh

# Measure behavior inside a supplied local project/repository directory.
measure-local-context cli id=cli context_workdir=".":
    {{cliare_bin}} measure {{cli}} \
      --out .cliare/{{id}} \
      --context local-context \
      --local-context-state present \
      --context-workdir {{context_workdir}} \
      --profile {{profile}} \
      --max-depth {{max_depth}} \
      --max-probes {{max_probes}} \
      --concurrency {{concurrency}} \
      --refresh

# Inspect foreground or detached measurement progress for an artifact id.
jobs id:
    {{cliare_bin}} jobs status --out .cliare/{{id}}

# Inspect measurement progress for a named context in a context suite.
jobs-context id context:
    {{cliare_bin}} jobs status --out .cliare/{{id}} --context {{context}}

# Generate or print a persona report from an artifact id.
report id persona="maintainer" format="markdown":
    {{cliare_bin}} report {{persona}} --out .cliare/{{id}} --format {{format}}

# Write all persona reports and shared review artifacts.
report-write id persona="maintainer":
    {{cliare_bin}} report {{persona}} --out .cliare/{{id}} --write

# Print a focused persona report for one agent-readiness area.
report-area id area persona="maintainer" format="markdown":
    {{cliare_bin}} report {{persona}} --out .cliare/{{id}} --area {{area}} --format {{format}}

# Print a focused issue report with evidence attached.
report-issue id issue persona="maintainer" format="bundle":
    {{cliare_bin}} report {{persona}} --out .cliare/{{id}} --issue {{issue}} --with-evidence --format {{format}}

# List generated issues with maintainer dispositions.
issues id format="human":
    {{cliare_bin}} issues list --out .cliare/{{id}} --format {{format}}

# Mark an issue with a maintainer disposition.
issue-mark id issue status reason:
    {{cliare_bin}} issues mark {{issue}} --out .cliare/{{id}} --status {{status}} --reason {{quote(reason)}}

# Describe an artifact directory for humans or agents.
describe id format="markdown":
    {{cliare_bin}} describe .cliare/{{id}} --format {{format}}

# Write artifact-map.json and artifact-map.md into the artifact directory.
describe-write id:
    {{cliare_bin}} describe .cliare/{{id}} --write

# Compare two context measurement directories.
context-compare left right out=".cliare-context" format="markdown":
    {{cliare_bin}} context compare {{left}} {{right}} --out {{out}} --format {{format}}

# Compare two context measurement directories and write suite artifacts.
context-compare-write left right out=".cliare-context":
    {{cliare_bin}} context compare {{left}} {{right}} --out {{out}} --write

# Measure and compare against a baseline scorecard.
guard cli id baseline:
    {{cliare_bin}} guard {{cli}} \
      --baseline {{baseline}} \
      --out .cliare/{{id}} \
      --profile {{profile}} \
      --max-depth {{max_depth}} \
      --max-probes {{max_probes}} \
      --concurrency {{concurrency}} \
      --allowed-drop {{allowed_drop}} \
      --refresh

# Measure and compare against a baseline with a policy file.
guard-policy cli id baseline policy:
    {{cliare_bin}} guard {{cli}} \
      --baseline {{baseline}} \
      --policy {{policy}} \
      --out .cliare/{{id}} \
      --profile {{profile}} \
      --max-depth {{max_depth}} \
      --max-probes {{max_probes}} \
      --concurrency {{concurrency}} \
      --allowed-drop {{allowed_drop}} \
      --refresh

# Run a benchmark corpus.
benchmark manifest="benchmarks/local-corpus.json" out=".cliare-bench" target_concurrency="2":
    {{cliare_bin}} benchmark \
      --manifest {{manifest}} \
      --out {{out}} \
      --target-concurrency {{target_concurrency}} \
      --refresh

# List installable CLIARE skill targets.
skills-list format="text":
    {{cliare_bin}} skills list --format {{format}}

# Preview skill installation for this project.
skills-install-dry agent="all" scope="project" project_dir=".":
    {{cliare_bin}} skills install --agent {{agent}} --scope {{scope}} --project-dir {{project_dir}} --dry-run

# Install CLIARE skills into this project.
skills-install-project agent="all" project_dir=".":
    {{cliare_bin}} skills install --agent {{agent}} --scope project --project-dir {{project_dir}}

# Common post-measurement review loop.
review id:
    {{cliare_bin}} describe .cliare/{{id}} --format markdown
    {{cliare_bin}} issues list --out .cliare/{{id}} --format human
    {{cliare_bin}} report maintainer --out .cliare/{{id}} --format markdown

# Common agent-surface publishing loop.
agent-surface id:
    {{cliare_bin}} describe .cliare/{{id}} --write
    {{cliare_bin}} report harness --out .cliare/{{id}} --write
    {{cliare_bin}} metadata --format json
