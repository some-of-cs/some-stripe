# Rule: Cyclomatic Complexity Minimization & Decomposition Standard (Gate G-Complexity)

## Core Objective
Ensure that every line of Rust code in `some-stripe` adheres to strict cognitive and structural simplicity. Code must be easily understandable, testable, and hardened against regression bugs by keeping cyclomatic branching complexity **as low as possible**.

All implementations, User Stories, and PRs MUST undergo automated or evidence-backed cyclomatic complexity analysis governed by [`.agents/skills/cyclomatic-complexity/SKILL.md`](../skills/cyclomatic-complexity/SKILL.md).

---

## 1. Cyclomatic Complexity Metrics & Scoring Invariants

Cyclomatic complexity ($CC$) counts the number of linearly independent execution paths through a function:

$$CC = 1 + \sum (\text{decision points})$$

Decision points include:
- Conditional branches: `if`, `else if`
- Pattern matching: each `match` arm
- Iteration loops: `for`, `while`, `loop`
- Boolean short-circuit operators: `&&`, `||`
- Fallible early-propagation operator: `?`

### Standard Thresholds & Enforcement Tiers

| CC Metric Range | Classification | Enforcement Action |
| :---: | :---: | :--- |
| **$CC \le 6$** | 🟢 **Ideal Target** | **Goal for all code.** Clean coordinator functions, linear pipelines, and concise endpoint dispatchers. |
| **$7 \le CC \le 10$** | 🟡 **Acceptable Upper Bound** | Permitted only for inherently multi-state domain state machines or parsers. Must be justified with clean naming and tests. |
| **$11 \le CC \le 15$** | 🟠 **Decomposition Required** | **Mandatory refactoring trigger.** Function MUST be decomposed into named helper functions or predicates before PR approval. |
| **$CC > 15$** | 🔴 **Hard Merge Blocker** | **Gate G-Complexity FAILS.** Merging via `gh pr merge` is strictly blocked. No exceptions. |

### Structural Constraints
- **Maximum Nesting Depth**: Deepest indentation within any function must be **$\le 2$ levels**.
- **Arrow Anti-pattern Prohibited**: Deeply indented cascading `if let` chains are forbidden. Use guard clauses with early returns (`if !valid { return Err(...); }`).

---

## 2. Mandatory Refactoring Strategies

When implementing or reviewing code, agents must apply the decomposition techniques from [`.agents/skills/cyclomatic-complexity/SKILL.md`](../skills/cyclomatic-complexity/SKILL.md):

1. **Extract Cohesive Branches into Named Functions**:
   - Separate validation, parameter extraction, serialization, and network dispatch into distinct, single-purpose helper functions.
   - Example: A 25-path resource method must be transformed into a 2-to-4-path coordinator calling focused helpers (each with $CC \le 5$).

2. **Replace Nesting with Early Guard Returns**:
   - Validate pre-conditions, authorization, and parameters at the beginning of the function and return immediately on failure.
   - Flatten the primary execution path so reader cognitive load remains constant.

3. **Collapse Compound Conditions into Named Predicates**:
   - Replace long boolean expressions like `if a && b && !c && d >= 10` with descriptive predicate helpers like `if is_eligible_for_payout(...)`.
   - Each predicate becomes independently unit-testable and documents its intent.

4. **Table Dispatch & Pattern Serialization**:
   - Replace repetitive parameter-mapping ladders with unified serialization utilities (e.g. `to_form_params(params)?` in `some_stripe_core::patterns::serializer`).
   - Replace multi-branch string parsers with hash maps, slice lookup tables, or match arms mapping directly to static tokens.

5. **Eliminate Redundant Idempotency Logic**:
   - Utilize the workspace-wide shared helper [`resolve_idempotency_key(params.idempotency_key.as_deref())`](file:///workspaces/some-stripe/src/resources/mod.rs) rather than rolling custom multi-line match blocks in each resource method.

---

## 3. Quality Gate Ownership & Review Protocol

- **Assigned Gatekeepers**:
  - **`Rust Refactoring Specialist`** (Skill: `cyclomatic-complexity`, `upgrade-stripe`): Audits AST branching complexity during pre-merge validation.
  - **`Code Reviewer`** (Skills: `cyclomatic-complexity`, `security-audit`, `stripe-best-practices`): Formally inspects and approves Gate G-Complexity in the GitHub PR review comment.
- **Audit Verification Scope**:
  - Measure cyclomatic complexity of all modified and newly added functions.
  - Verify that the target function has $CC \le 6$ (or $\le 10$ with documented domain justification).
  - Confirm **zero functions with $CC > 15$** exist anywhere in the PR diff or affected modules.
- **PR Sign-Off Requirement**:
  - The PR review comment MUST include an explicit Gate G-Complexity entry:
    ```markdown
    | **Gate G-Complexity** (Complexity Minimization) | Code Reviewer | `cyclomatic-complexity` | ✅ PASS | All refactored/new functions score CC <= 6; 0 functions with CC > 10. |
    ```
