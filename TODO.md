<!-- Hold short-term reminders too small to justify an issue. -->
# TODO

Prune this list every release. Anything older than one release becomes an
issue or is deleted. Larger work belongs in issues; direction belongs in
[ROADMAP.md](ROADMAP.md).

- [ ] Qualify `similarity-rs` against a real duplicate, then wire duplication
      into `lang-check`. The profile lists it under `qualification_required`, so
      until it is proved the duplication gate is unqualified rather than passing.
- [ ] Bind test reporting. The `cargo` runner emits no JUnit XML, so either
      select a verified adapter or switch `runner` to nextest for its native
      output. Recorded as unresolved in ADR 0002.
- [ ] Pin the `mise.toml` tool versions with `mise use --pin`. They resolve to
      `latest` today, so an unpinned copy moves with upstream.
- [ ] Re-add the heavy `fuzz` job when the first hand-rolled parser lands, with
      harnesses under `fuzz/` and a budget bound in `just/lang.just`.
- [ ] Wire `lang-auditable` into a release workflow and verify the shipped binary
      retains its dependency inventory, for example with `cargo audit bin`.
