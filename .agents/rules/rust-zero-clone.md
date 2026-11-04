# Rule: Rust Zero-Clone Borrowing Discipline (Gate R1 / G-Borrow)

## Core Objective
Enforce strict compile-time Rust memory safety, zero data races, explicit lifetimes, and zero deep heap cloning across all `some-stripe` crates.

## Mandatory Invariants
1. **Zero Deep Clones on Domain Models**:
   - Calling `.clone()` on request parameter structs, domain entities (`PaymentIntent`, `Customer`, `Subscription`, `Refund`), or large collections is strictly prohibited.
   - The only permitted `.clone()` calls are pointer increments on reference-counted primitives: `Arc::clone(&client)` and `Rc::clone(&inner)`.
2. **Strict Borrowing & Lifetimes**:
   - Resource service methods and request builders MUST accept parameters strictly by reference (`&Params` or `&str`).
   - Wire serializers must stream directly from borrowed references rather than copying strings into intermediate heap vectors.
3. **Zero-Copy Byte Slices**:
   - Use `bytes::Bytes` or borrowed slices (`&[u8]`) for raw webhook payloads and HTTP response streaming.
   - Use stack-allocated small-vector buffers (`smallvec` / `tinyvec`) for bracket key segment accumulation.
4. **Verification & Audit**:
   - Any pull request introducing deep `.clone()` calls without explicit architectural justification will be rejected at Gate G-Borrow.
