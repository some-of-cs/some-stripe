# Main Conversation End-of-Module Whole-Codebase Validation & Review

## 1. Mandate

At the conclusion of every module:
1. **Module Promotion**:
   - The Module Leader merges all user stories into the module branch (`feat/module-N`).
   - The Module Leader opens the Module PR to `main` and obtains Gatekeeper Council sign-off.
   - `Git Workflow Master` merges the Module PR into `main`.
2. **Main Conversation Comprehensive Validation**:
   - The Main Supervisor validates the entire workspace on `main`:
     - `cargo fmt --all -- --check`
     - `cargo clippy --workspace --all-targets -- -D warnings`
     - `cargo test --doc`
     - `cargo test --workspace`
3. **Whole-Codebase Independent Review**:
   - The Main Supervisor invokes the Federated Gatekeeper Council President (`Code Reviewer`) armed with `security-audit` to audit the entire repository across the current module AND all previous modules (Modules 1..N).
   - Any identified issues are remediated immediately via a dedicated patch implementer before moving to the next module.
