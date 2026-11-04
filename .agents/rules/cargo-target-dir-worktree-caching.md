# Rule: Mandatory Shared Cargo Build Cache (`CARGO_TARGET_DIR`) in Git Worktrees

## 1. Core Architectural Invariant
All agents, subagents, and automated workflows operating inside any git worktree (`/workspaces/some-stripe/.worktrees/*`) **MUST ALWAYS** export and use the shared workspace target directory:

```bash
export CARGO_TARGET_DIR=/workspaces/some-stripe/target
```

Executing `cargo` (`test`, `check`, `build`, `clippy`, `doc`) without this environment variable is **strictly prohibited**.

---

## 2. Rationale & Historical Incident Post-Mortem

### The Failure Mode
- **Symptom**: Story leads and unit agents stalled for 10–15+ minutes with high CPU/memory consumption attempting to set up and compile worktrees.
- **Root Cause**: By default, Cargo places build artifacts in `<current_working_directory>/target`. When an agent switches to a new worktree (e.g. `.worktrees/feat-us-310-payment-intents`) and runs `cargo test`, Cargo perceives it as a fresh, empty workspace and recompiles all 193 workspace dependencies from scratch—including heavy C/C++ packages like `openssl-src`, `aws-lc-sys`, and `cmake`.
- **Solution**: Setting `export CARGO_TARGET_DIR=/workspaces/some-stripe/target` forces Cargo in all worktrees to reuse the pre-compiled, cached artifacts from the root workspace. Builds finish in **1 to 3 seconds** instead of 10+ minutes.

---

## 3. Mandatory Execution Template for All Agents

Every bash command running `cargo` inside `.worktrees/` MUST follow this exact template:

```bash
source $HOME/.cargo/env
export CARGO_TARGET_DIR=/workspaces/some-stripe/target
cd /workspaces/some-stripe/.worktrees/<assigned-worktree>
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

---

## 4. Zero-Delay Worktree Creation Protocol
- **Do NOT compile before creating worktrees**: Never block worktree creation on a baseline `cargo test` run.
- **Instant Provisioning**: Create all child unit worktrees immediately via `git worktree add`.
- **Parallel Dispatch**: Dispatch unit agents into their respective worktrees immediately without waiting for pre-flight test suites.
