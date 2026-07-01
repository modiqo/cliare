# Agent CLI Citations And Evidence

> **Scope:** Primary-source papers and benchmark evidence relevant to agents using CLIs, tool interfaces, and execution environments.
> **Status:** Research inventory for CLIARE scoring and evaluation design.

---

## Reading Lens

CLIARE is not trying to benchmark language models directly. It measures a CLI
so that:

1. maintainers can improve the CLI for agent harnesses, and
2. agent harnesses can consume an evidence-backed CLI shape before acting.

The cited work is therefore used for design evidence:

- agents need interactive execution feedback;
- interface design changes agent success;
- tool schemas and argument grammar matter;
- execution-based evaluation is more reliable than static text judgment;
- long-horizon agents fail through planning drift, missing state, bad tool
  selection, malformed arguments, weak recovery, and unsafe side effects;
- current frontier systems still fail many terminal and computer-use tasks,
  which makes a reliable CLI shape layer valuable.

---

## Terminal And CLI Agent Benchmarks

| Work | Primary Source | Evidence | CLIARE Implication |
|---|---|---|---|
| Terminal-Bench 2.0 | [Merrill et al., 2026](https://arxiv.org/abs/2601.11868) | Curates 89 hard command-line tasks, each with a unique environment, human-written solution, and tests; frontier agents score below 65%. | CLIARE should optimize for hard, realistic shell use, not only easy `--help` extraction. Shape confidence should reduce failed exploration in terminal tasks. |
| TerminalWorld | [Chu et al., 2026](https://arxiv.org/abs/2605.22535) | Builds 1,530 validated tasks from 80,870 real terminal recordings, spanning 18 categories and 1,280 commands; the verified subset shows frontier agents still top out near 62.5%, and weak correlation with Terminal-Bench. | Real terminal workflows are diverse and command-heavy. CLIARE's shape should capture command trees, output contracts, and preconditions across ordinary developer workflows, not just curated expert tasks. |
| TUA-Bench | [Chen et al., 2026](https://arxiv.org/abs/2606.28480) | Defines 120 real-world terminal-use tasks across routine digital activities and scientific/engineering workflows; strongest reported frontier agent reaches 65.8%. | Terminal use is broader than coding. CLIARE should support contexts, fixtures, network/runtime preconditions, and task families beyond software build tools. |
| InterCode | [Yang et al., 2023](https://arxiv.org/abs/2306.14898) | Frames coding as an interactive environment with execution feedback as observations; includes Bash, SQL, and Python action spaces in Docker environments. | CLIARE's evidence log and shape should preserve the action/observation loop so harnesses can reason about what was actually executed and observed. |

### Takeaway

Terminal-agent research consistently evaluates agents by execution outcomes,
not by whether they can summarize documentation. CLIARE should therefore treat
runtime evidence as the primary source of truth and make every emitted shape
claim trace back to probes, process results, and confidence.

---

## Software-Engineering Agent Interfaces

| Work | Primary Source | Evidence | CLIARE Implication |
|---|---|---|---|
| SWE-bench | [Jimenez et al., 2023](https://arxiv.org/abs/2310.06770) | Uses 2,294 real GitHub issues across 12 Python repositories; solving issues requires interacting with execution environments and long code contexts. | CLI quality matters because agents use build systems, package managers, test runners, formatters, and repo tools as part of solving real issues. |
| SWE-agent | [Yang et al., 2024](https://arxiv.org/abs/2405.15793) | Shows that a custom agent-computer interface materially improves software-agent performance on SWE-bench and HumanEvalFix. | Interface shape is not incidental. CLIARE should measure and expose the CLI interface features that let agents navigate, invoke, recover, and verify. |
| OpenHands | [Wang et al., 2024](https://arxiv.org/abs/2407.16741) | Presents an agent platform where agents write code, use the command line, and browse the web inside sandboxed environments. | Agent harnesses are already built around terminal execution. A CLI shape file can be a reusable input to these harnesses rather than a per-agent learned heuristic. |
| CodeAct | [Wang et al., 2024](https://arxiv.org/abs/2402.01030) | Uses executable Python code as a unified action space and reports up to 20% success-rate gains over common action formats on API/tool benchmarks. | Harnesses often compose tools through code. `shape.json` should be structured enough for programmatic planning, not just human-readable reporting. |
| MLE-bench | [Chan et al., 2024](https://arxiv.org/abs/2410.07095) | Evaluates ML engineering agents on 75 Kaggle competitions; best reported setup reaches at least bronze-medal performance in 16.9% of competitions. | Long-running engineering workflows depend on many CLIs and repeated verification. CLIARE should measure timeouts, output parseability, context blockers, and reproducibility. |

### Takeaway

Software-agent systems succeed or fail partly because of their interface to the
environment. CLIARE should treat CLI design as an agent-computer-interface
problem and emit a shape that harnesses can compile into concrete calls.

---

## Tool, API, And Function-Calling Research

| Work | Primary Source | Evidence | CLIARE Implication |
|---|---|---|---|
| ReAct | [Yao et al., 2022](https://arxiv.org/abs/2210.03629) | Interleaves reasoning and actions; improves interactive decision-making tasks by letting actions gather observations and reasoning handle exceptions. | CLIARE should expose recoverable observations and diagnostics so agents can plan, act, and repair rather than blindly retry. |
| MRKL Systems | [Karpas et al., 2022](https://arxiv.org/abs/2205.00445) | Argues for systems that combine language models with external modules and discrete tools. | A CLI shape file is a module interface description for terminal tools. |
| Toolformer | [Schick et al., 2023](https://arxiv.org/abs/2302.04761) | Trains models to decide which APIs to call, when to call them, what arguments to pass, and how to use results. | CLIARE shape fields should support tool selection, argument construction, and result interpretation. |
| Gorilla / APIBench | [Patil et al., 2023](https://arxiv.org/abs/2305.15334) | Shows API-call generation is limited by wrong arguments and hallucinated usage; retrieval over updated documentation reduces hallucination and handles version changes. | CLIARE can provide versioned, executable-evidence-backed retrieval input for CLI calls, reducing stale-doc and hallucinated-flag failures. |
| API-Bank | [Li et al., 2023](https://arxiv.org/abs/2304.08244) | Evaluates planning, retrieving, and calling 73 API tools across annotated dialogues. | CLI shape should include enough information for planning and tool retrieval, not only a flat command list. |
| ToolLLM / ToolBench | [Qin et al., 2023](https://arxiv.org/abs/2307.16789) | Builds a tool-use framework around 16,464 real-world APIs and multi-tool solution paths. | CLIARE should prepare CLI surfaces for multi-step harness plans and eventual cross-CLI composition. |
| tau-bench | [Yao et al., 2024](https://arxiv.org/abs/2406.12045) | Evaluates tool agents in dynamic user conversations with policy constraints; reports low and inconsistent pass rates, including pass^k reliability. | CLIARE should measure not just one-shot callability but repeated reliability, policy-following affordances, and state/context requirements. |
| AppWorld | [Trivedi et al., 2024](https://arxiv.org/abs/2407.18901) | Provides 9 apps, 457 APIs, 750 tasks, execution tests, and collateral-damage checks. | CLIARE's safety and side-effect evidence should become a first-class part of harness shape confidence. |
| ToolEmu | [Ruan et al., 2023](https://arxiv.org/abs/2309.15817) | Finds high-stakes tool-agent risks such as privacy leaks and financial loss; reports that even the safest tested agent shows severe failures in 23.9% of cases under the evaluator. | CLIARE should score hazardous affordances, credential-like side effects, destructive defaults, and lack of dry-run/preview support. |

### Takeaway

The API/tool literature shows that agents need precise interface schemas, valid
argument constraints, updated retrieval context, and risk controls. CLIARE's
CLI shape file should become the command-line analog of a verified tool schema,
with provenance and confidence.

---

## Computer-Use Benchmarks Adjacent To CLI Shape

| Work | Primary Source | Evidence | CLIARE Implication |
|---|---|---|---|
| AgentBench | [Liu et al., 2023](https://arxiv.org/abs/2308.03688) | Evaluates LLM agents across eight interactive environments and reports failure causes such as long-term reasoning, decision-making, and instruction following. | CLIARE should expose enough state and preconditions to reduce avoidable decision errors in terminal workflows. |
| WebArena | [Zhou et al., 2023](https://arxiv.org/abs/2307.13854) | Builds realistic websites and 812 tasks; best GPT-4 baseline reaches 14.41% end-to-end success versus 78.24% human performance. | Realistic environments reveal failure modes hidden by synthetic tasks. CLIARE evals should include real CLIs and real workflow tasks. |
| OSWorld | [Xie et al., 2024](https://arxiv.org/abs/2404.07972) | Provides 369 open-ended computer tasks with setup and execution-based evaluation; best model reaches 12.24% while humans reach 72.36%. | General computer-use agents still need structured environment affordances. Terminal shape can be a strong affordance when the task can be done through a CLI. |
| OSWorld 2.0 | [Yuan et al., 2026](https://arxiv.org/abs/2606.29537) | Focuses on long-horizon workflows with realistic hidden state, dynamic environments, and safety reports; reports low completion rates even with frontier agents. | CLIARE should model hidden-state blockers explicitly: auth, local context, fixtures, network, runtime dependencies, and dynamic output hazards. |

### Takeaway

Computer-use benchmarks show that realistic state and execution constraints
dominate agent outcomes. CLIARE's shape file should make hidden state explicit
and help agents avoid unsafe or low-confidence actions.

---

## Evidence Themes

### Execution-Based Truth

Terminal-Bench, TerminalWorld, TUA-Bench, InterCode, AppWorld, WebArena, and
OSWorld all rely on execution-based evaluation. CLIARE should continue treating
runtime evidence as stronger than documentation, static help text, or parser
assumptions.

### Interface Design Matters

SWE-agent shows that agent-computer interfaces affect success. Toolformer,
Gorilla, API-Bank, and ToolLLM show that tool schemas, retrieval, and argument
construction are central. CLIARE should make the CLI interface explicit and
machine-consumable.

### Shape Should Reduce Exploration Cost

Terminal agents still fail many tasks. A reliable command index can reduce:

- repeated help probing,
- hallucinated flags,
- malformed arguments,
- execution of commands needing fixtures,
- parsing of human-oriented output,
- unsafe side-effect exploration.

### Confidence And Provenance Are Product Features

A shape file without evidence is another documentation artifact. CLIARE's
distinct contribution is that shape entries can carry target fingerprint,
probe evidence, confidence, preconditions, and safety findings.

---

## Research Gaps For CLIARE To Fill

Most existing benchmarks evaluate agents, not the CLIs those agents operate.
CLIARE can occupy the missing layer:

```text
CLI runtime evidence
  -> executable shape
  -> harness planning input
  -> maintainer feedback
  -> calibrated agent-use improvement
```

The central research question is:

> Given a fixed agent harness and task distribution, how much does an
> evidence-backed CLI shape improve task success, reduce exploratory commands,
> reduce unsafe actions, and improve recovery compared with raw terminal access
> plus documentation?
