# Expose common gates and delegate language work to the imported module.
set shell := ["bash", "-euo", "pipefail", "-c"]
set windows-shell := ["bash", "-euo", "pipefail", "-c"]

import 'just/lang.just'

default:
    just --list

setup:
    prek install --install-hooks

check: common-check lang-check polyglot-check

common-check:
    prek run --all-files --show-diff-on-failure

fmt: common-fmt lang-fmt

common-fmt:
    prek run --all-files taplo-format shfmt typos trailing-whitespace end-of-file-fixer mixed-line-ending

test: lang-test polyglot-check

# The two suites that are not Rust: the pi extension that posts a finished turn,
# and the Python speech services the router starts.
#
# They live here rather than in just/lang.just because that module is the Rust
# language tier. A suite wired to nothing is the failure AGENTS.md names, and
# both of these were previously runnable but ungated.
polyglot-check: extension-test services-test

# The pi extension, run by node's own test runner.
#
# Named by glob rather than by directory: `node --test extensions/` does not
# discover `.mts` files and reports a failure for the directory itself, which
# looks like a broken test rather than a runner that found nothing.
extension-test:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! command -v node >/dev/null 2>&1; then
        echo "extension-test: SKIPPED. node is not installed, so the extension"
        echo "extension-test: tests did not run. They are NOT passing; install"
        echo "extension-test: node to run them."
        exit 0
    fi
    node --test extensions/*.test.mts

# The speech services, on the system interpreter with no third-party packages.
services-test:
    #!/usr/bin/env bash
    set -euo pipefail
    python="${MAESTRO_SPEECH_TEST_PYTHON:-python3}"
    if ! command -v "${python}" >/dev/null 2>&1; then
        echo "services-test: SKIPPED. '${python}' is not installed, so the speech"
        echo "services-test: service tests did not run. They are NOT passing; set"
        echo "services-test: MAESTRO_SPEECH_TEST_PYTHON or install python3."
        exit 0
    fi
    ./services/run-tests

# Needs network access; remote links are a heavy-tier check.
links-remote:
    lychee --no-progress .

hooks-update:
    prek auto-update

doctor:
    @for tool in just prek git; do command -v "$tool"; "$tool" --version; done
