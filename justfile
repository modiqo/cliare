set positional-arguments

# Override with `just cliare_bin=target/debug/cliare ...` when dogfooding a local build.
cliare_bin := "cliare"

# Default traversal budget for large CLI surfaces.
profile := "deep"
max_depth := "12"
max_probes := "5000"
concurrency := "8"
allowed_drop := "0"

# Print the ordered CLIARE workflow cheatsheet.
default:
    @just cheatsheet

# Print the ordered CLIARE workflow cheatsheet.
cheatsheet:
    @printf '%s\n' \
      "CLIARE workflow cheatsheet" \
      "" \
      "Use <cli> for the executable to measure." \
      "Use <run-name> for the results folder under .cliare/." \
      "Example: <run-name> = cliare-self writes .cliare/cliare-self." \
      "" \
      "1. Local dogfood check" \
      "   just build" \
      "   just cliare_bin=target/debug/cliare measure-quick target/debug/cliare cliare" \
      "   just review cliare" \
      "" \
      "2. Standard maintainer review" \
      "   just measure-standard <cli> <run-name>" \
      "   just review <run-name>" \
      "   just issues <run-name>" \
      "   just report-area <run-name> output-contracts" \
      "   just report-issue <run-name> <issue-id>" \
      "" \
      "3. Deep measurement for large CLI surfaces" \
      "   just measure-deep <cli> <run-name>" \
      "   just jobs <run-name>" \
      "   just review <run-name>" \
      "" \
      "4. Security or host-state review" \
      "   just measure-host <cli> <run-name>" \
      "   just report <run-name> security" \
      "   just issues <run-name>" \
      "" \
      "5. Authenticated or local-context comparison" \
      "   just measure-auth <cli> <run-name>" \
      "   just measure-local-context <cli> <run-name> <project-dir>" \
      "   just context-compare .cliare/<run-name>/contexts/authenticated .cliare/<run-name>/contexts/local-context" \
      "" \
      "6. CI guard against a baseline" \
      "   just guard <cli> <run-name> .cliare/<baseline-run-name>/scorecard.json" \
      "   just guard-policy <cli> <run-name> .cliare/<baseline-run-name>/scorecard.json policy.json" \
      "" \
      "7. Publish an agent surface" \
      "   just describe-write <run-name>" \
      "   just report-write <run-name> harness" \
      "   just agent-surface <run-name>" \
      "   just surface-query <run-name> 'check job status'" \
      "   just surface-explain <run-name> 'jobs status'" \
      "" \
      "8. Skills and corpus work" \
      "   just skills-list" \
      "   just skills-install-dry" \
      "   just benchmark" \
      "" \
      "Lookup commands" \
      "   just recipes        Full raw recipe index" \
      "   just --summary      Compact recipe names" \
      "   just --show <name>   Show the exact command for one recipe"

# Print the ordered CLIARE workflow cheatsheet.
help:
    @just cheatsheet

# Print the full raw recipe index in source order.
recipes:
    @just --list --unsorted

# Build this repository's CLIARE binary for local dogfooding.
build:
    cargo build --locked --bin cliare

# Print CLIARE's machine-readable command contract.
metadata format="json":
    {{ cliare_bin }} metadata --format {{ format }}

# Print a role-specific operational playbook.
playbook role="maintainer" cli="mycli" run=cli format="human":
    {{ cliare_bin }} playbook {{ role }} --target {{ cli }} --out .cliare/{{ run }} --format {{ format }}

# Measure a local CLI in the foreground.
measure cli run=cli:
    {{ cliare_bin }} measure {{ cli }} \
      --out .cliare/{{ run }} \
      --profile {{ profile }} \
      --max-depth {{ max_depth }} \
      --max-probes {{ max_probes }} \
      --concurrency {{ concurrency }} \
      --refresh

# Measure a local CLI in the background for large command surfaces.
measure-detached cli run=cli:
    {{ cliare_bin }} measure {{ cli }} \
      --out .cliare/{{ run }} \
      --profile {{ profile }} \
      --max-depth {{ max_depth }} \
      --max-probes {{ max_probes }} \
      --concurrency {{ concurrency }} \
      --refresh \
      --detach

# Run a quick smoke measurement with a smaller traversal budget.
measure-quick cli run=cli:
    {{ cliare_bin }} measure {{ cli }} \
      --out .cliare/{{ run }} \
      --profile quick \
      --refresh

# Run the standard maintainer-loop measurement.
measure-standard cli run=cli:
    {{ cliare_bin }} measure {{ cli }} \
      --out .cliare/{{ run }} \
      --profile standard \
      --refresh

# Run a deep foreground measurement with the configured traversal budget.
measure-deep cli run=cli:
    @just profile=deep max_depth={{ max_depth }} max_probes={{ max_probes }} concurrency={{ concurrency }} measure {{ cli }} {{ run }}

# Measure host/auth-specific behavior. Use only when the target needs local state.
measure-host cli run=cli:
    {{ cliare_bin }} measure {{ cli }} \
      --out .cliare/{{ run }} \
      --profile {{ profile }} \
      --execution-mode host \
      --max-depth {{ max_depth }} \
      --max-probes {{ max_probes }} \
      --concurrency {{ concurrency }} \
      --refresh

# Measure an authenticated context into a context suite under `.cliare/<run>`.
measure-auth cli run=cli:
    {{ cliare_bin }} measure {{ cli }} \
      --out .cliare/{{ run }} \
      --context authenticated \
      --auth-state present \
      --execution-mode host \
      --profile {{ profile }} \
      --max-depth {{ max_depth }} \
      --max-probes {{ max_probes }} \
      --concurrency {{ concurrency }} \
      --refresh

# Measure behavior inside a supplied local project/repository directory.
measure-local-context cli run=cli context_workdir=".":
    {{ cliare_bin }} measure {{ cli }} \
      --out .cliare/{{ run }} \
      --context local-context \
      --local-context-state present \
      --context-workdir {{ context_workdir }} \
      --profile {{ profile }} \
      --max-depth {{ max_depth }} \
      --max-probes {{ max_probes }} \
      --concurrency {{ concurrency }} \
      --refresh

# Inspect foreground or detached measurement progress for a run.
jobs run:
    {{ cliare_bin }} jobs status --out .cliare/{{ run }}

# Inspect measurement progress for a named context in a context suite.
jobs-context run context:
    {{ cliare_bin }} jobs status --out .cliare/{{ run }} --context {{ context }}

# Generate or print a persona report from a run.
report run persona="maintainer" format="markdown":
    {{ cliare_bin }} report {{ persona }} --out .cliare/{{ run }} --format {{ format }}

# Write one persona report and shared review artifacts.
report-write run persona="maintainer":
    {{ cliare_bin }} report {{ persona }} --out .cliare/{{ run }} --write

# Print a focused persona report for one agent-readiness area.
report-area run area persona="maintainer" format="markdown":
    {{ cliare_bin }} report {{ persona }} --out .cliare/{{ run }} --area {{ area }} --format {{ format }}

# Print a focused issue report with evidence attached.
report-issue run issue persona="maintainer" format="bundle":
    {{ cliare_bin }} report {{ persona }} --out .cliare/{{ run }} --issue {{ issue }} --with-evidence --format {{ format }}

# List generated issues with maintainer dispositions.
issues run format="human":
    {{ cliare_bin }} issues list --out .cliare/{{ run }} --format {{ format }}

# Query the measured command surface for a harness intent.
surface-query run intent format="human":
    {{ cliare_bin }} surface query {{ quote(intent) }} --out .cliare/{{ run }} --format {{ format }}

# Query commands with JSON output for a harness intent.
surface-query-json run intent:
    {{ cliare_bin }} surface query {{ quote(intent) }} --out .cliare/{{ run }} --require-output json --format json

# Explain one measured command path for harness routing.
surface-explain run command_path format="human":
    {{ cliare_bin }} surface explain {{ quote(command_path) }} --out .cliare/{{ run }} --format {{ format }}

# List measured commands by readiness state.
surface-list run state="ready" format="human" limit="50":
    {{ cliare_bin }} surface list --out .cliare/{{ run }} --state {{ state }} --limit {{ limit }} --format {{ format }}

# Mark an issue with a maintainer disposition.
issue-mark run issue status reason:
    {{ cliare_bin }} issues mark {{ issue }} --out .cliare/{{ run }} --status {{ status }} --reason {{ quote(reason) }}

# Describe an artifact directory for humans or agents.
describe run format="markdown":
    {{ cliare_bin }} describe .cliare/{{ run }} --format {{ format }}

# Write artifact-map.json and artifact-map.md into the artifact directory.
describe-write run:
    {{ cliare_bin }} describe .cliare/{{ run }} --write

# Compare two context measurement directories.
context-compare left right out=".cliare-context" format="markdown":
    {{ cliare_bin }} context compare {{ left }} {{ right }} --out {{ out }} --format {{ format }}

# Compare two context measurement directories and write suite artifacts.
context-compare-write left right out=".cliare-context":
    {{ cliare_bin }} context compare {{ left }} {{ right }} --out {{ out }} --write

# Measure and compare against a baseline scorecard.
guard cli run baseline:
    {{ cliare_bin }} guard {{ cli }} \
      --baseline {{ baseline }} \
      --out .cliare/{{ run }} \
      --profile {{ profile }} \
      --max-depth {{ max_depth }} \
      --max-probes {{ max_probes }} \
      --concurrency {{ concurrency }} \
      --allowed-drop {{ allowed_drop }} \
      --refresh

# Measure and compare against a baseline with a policy file.
guard-policy cli run baseline policy:
    {{ cliare_bin }} guard {{ cli }} \
      --baseline {{ baseline }} \
      --policy {{ policy }} \
      --out .cliare/{{ run }} \
      --profile {{ profile }} \
      --max-depth {{ max_depth }} \
      --max-probes {{ max_probes }} \
      --concurrency {{ concurrency }} \
      --allowed-drop {{ allowed_drop }} \
      --refresh

# Run a benchmark corpus.
benchmark manifest="benchmarks/local-corpus.json" out=".cliare-bench" target_concurrency="2":
    {{ cliare_bin }} benchmark \
      --manifest {{ manifest }} \
      --out {{ out }} \
      --target-concurrency {{ target_concurrency }} \
      --refresh

# List installable CLIARE skill targets.
skills-list format="text":
    {{ cliare_bin }} skills list --format {{ format }}

# Preview skill installation for this project.
skills-install-dry agent="all" scope="project" project_dir=".":
    {{ cliare_bin }} skills install --agent {{ agent }} --scope {{ scope }} --project-dir {{ project_dir }} --dry-run

# Install CLIARE skills into this project.
skills-install-project agent="all" project_dir=".":
    {{ cliare_bin }} skills install --agent {{ agent }} --scope project --project-dir {{ project_dir }}

# Common post-measurement maintainer review.
review run:
    {{ cliare_bin }} report maintainer --out .cliare/{{ run }} --format markdown

# Common agent-surface publishing loop.
agent-surface run:
    {{ cliare_bin }} describe .cliare/{{ run }} --write
    {{ cliare_bin }} report harness --out .cliare/{{ run }} --write
    {{ cliare_bin }} metadata --format json
