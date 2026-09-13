# Expose common gates and delegate language work to the imported module.
set shell := ["bash", "-euo", "pipefail", "-c"]
set windows-shell := ["bash", "-euo", "pipefail", "-c"]

import 'just/lang.just'

default:
    just --list

setup:
    prek install --install-hooks

check: common-check lang-check

common-check:
    prek run --all-files --show-diff-on-failure

fmt: common-fmt lang-fmt

common-fmt:
    prek run --all-files taplo-format shfmt typos trailing-whitespace end-of-file-fixer mixed-line-ending

test: lang-test

# Needs network access; remote links are a heavy-tier check.
links-remote:
    lychee --no-progress .

hooks-update:
    prek auto-update

doctor:
    @for tool in just prek git; do command -v "$tool"; "$tool" --version; done
