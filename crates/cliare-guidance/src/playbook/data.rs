use super::ParameterGuide;

pub(super) fn artifact_layout() -> Vec<&'static str> {
    vec![
        "`--out` names one target's artifact root, not a global CLIARE database.",
        "Use `.cliare/<target-cli>` for normal project-local runs.",
        "Context runs write under `.cliare/<target-cli>/contexts/<context>`.",
        "If you use `--detach`, wait for `cliare jobs status --out <artifact-dir>` to report `complete` before reading reports or issues.",
    ]
}

pub(super) fn parameter_guide() -> Vec<ParameterGuide> {
    vec![
        ParameterGuide {
            name: "--profile quick",
            meaning: "Small local smoke pass.",
            use_when: "Editing help, diagnostics, or one output contract.",
        },
        ParameterGuide {
            name: "--profile standard",
            meaning: "Balanced default pass.",
            use_when: "Normal maintainer loop.",
        },
        ParameterGuide {
            name: "--profile deep",
            meaning: "Broader release-quality pass.",
            use_when: "CI baseline, release, or publishing agent surface.",
        },
        ParameterGuide {
            name: "--max-depth",
            meaning: "Recursive command-path depth.",
            use_when: "Nested command families are missing or observed_max_depth equals max_depth.",
        },
        ParameterGuide {
            name: "--max-probes",
            meaning: "Maximum runtime probes.",
            use_when: "budget_exhausted is true, frontier_remaining is greater than zero, or too many candidate commands remain.",
        },
        ParameterGuide {
            name: "--concurrency",
            meaning: "Probes run at the same time.",
            use_when: "Lower for rate limits, shared state, daemons, or flaky CLIs; raise only for stable local CLIs.",
        },
        ParameterGuide {
            name: "--timeout-ms",
            meaning: "Per-probe timeout.",
            use_when: "The CLI is slow, network-backed, daemon-backed, or package-manager-like.",
        },
        ParameterGuide {
            name: "--output-limit-bytes",
            meaning: "Retained stdout/stderr bytes per probe.",
            use_when: "Help or machine output is legitimately large.",
        },
        ParameterGuide {
            name: "--execution-mode isolated",
            meaning: "Default sandboxed profile.",
            use_when: "Safe local probing.",
        },
        ParameterGuide {
            name: "--execution-mode host",
            meaning: "Host config, auth, plugins, and local state are visible.",
            use_when: "Measuring authenticated or host-specific behavior.",
        },
    ]
}
