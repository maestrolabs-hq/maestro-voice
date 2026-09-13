# Rust language module: fast gates in lang-check, heavy gates in the lang-* recipes below.
# CI runs exactly these recipes; the justfile shell is bash on every platform.

# Select exactly one: cargo or nextest (profile testing.unit_and_integration_runner).
runner := "cargo"

# nextest cannot run doctests, so Cargo runs them after it; any other runner value aborts.
test_command := if runner == "nextest" {
    "cargo nextest run --all-targets && cargo test --doc"
} else if runner == "cargo" {
    "cargo test --all-targets"
} else {
    error("runner must be cargo or nextest")
}

# Fuzz budget in seconds per target: a required project binding (profile fuzzing.targets_and_budget).
# Targets are the harnesses under fuzz/; an empty budget stays unresolved and lang-fuzz fails.
fuzz_max_total_time := ""

lang-check:
    cargo fmt --all --check
    cargo clippy --all-targets --all-features -- -D warnings
    just lang-test
    cargo machete
    cargo deny check

lang-fmt:
    cargo fmt --all

# The selected runner; with cargo, --all-targets leaves doctests to the heavy lang-docs recipe.
lang-test:
    {{ test_command }}

# Heavy gates below: evidence, never required checks.

# Profile floor: 90 percent of lines on stable; the LCOV report lands in reports/lcov.info.
lang-coverage:
    mkdir -p reports
    cargo llvm-cov --all-targets --lcov --output-path reports/lcov.info --fail-under-lines 90
    cargo llvm-cov report --summary-only

lang-audit:
    cargo audit --deny warnings

# No score floor is bound: a missed mutant makes this heavy recipe fail, which is evidence, not a gate.
lang-mutants:
    mkdir -p reports
    cargo mutants --no-shuffle --output reports/mutants

lang-docs:
    RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features
    cargo test --doc

# Declared features, not a powerset; the MSRV is rust-version in Cargo.toml and cargo-hack installs that toolchain.
# The lines differ on purpose: --no-dev-deps strips the dev-dependencies test targets need, so --all-targets stays on the MSRV line only.
lang-features:
    cargo hack check --each-feature --no-dev-deps
    cargo hack check --rust-version --all-targets

# Only for published libraries with a previous release; the baseline is a project binding. Delete the heavy job otherwise.
lang-semver:
    cargo semver-checks

# Nightly and Unix x86-64/AArch64 only; targets and budget are project bindings (profile fuzzing.targets_and_budget).
lang-fuzz:
    @test -d fuzz || { echo "lang-fuzz: no fuzz/ directory; bind fuzz targets under fuzz/ or delete the heavy fuzz job"; exit 1; }
    @test -n "{{ fuzz_max_total_time }}" || { echo "lang-fuzz: bind fuzz_max_total_time in just/lang.just before running lang-fuzz"; exit 1; }
    targets="$(cargo +nightly fuzz list)"; \
    test -n "$targets" || { echo "lang-fuzz: fuzz/ declares no targets"; exit 1; }; \
    for target in $targets; do cargo +nightly fuzz run "$target" -- -max_total_time={{ fuzz_max_total_time }}; done

# Shipped binaries only; verifying that the binary retains its dependency metadata is a project binding.
lang-auditable:
    cargo auditable build --release

# Qualification required by the profile (qualification_required: similarity-rs); not part of lang-check.
lang-duplication:
    similarity-rs .
