# Working style

- Implement the minimum needed for the agreed change. Do not add unused helpers, APIs, or other functionality in anticipation of future use; add them when a concrete need arises.

# Code style

- Use `foo/mod.rs` for modules with child files; do not pair `foo.rs` with a `foo/` directory containing its children.
- Consider the fully qualified name (FQN) when naming items. Avoid repeating context already supplied by the module path; prefer `light::Field` over `light::LightField`.
- Define items before they are used when reading a file from top to bottom. If function A calls function B in the same file, define B above A; apply the same ordering to methods and test helpers.
- Give each helper function a terse comment above its definition describing its purpose. Helpers should perform meaningful work; prefer inlining trivial one-line helpers that see little reuse.
- Prefer named, file-local constants declared at the top of the file over inline literal values, especially property keys and classname values such as `"classname"` and `"worldspawn"`.
- Keep tests in a separate `test.rs` file alongside the module they test, such as `src/map/test.rs`.
- Do not define tests inline in implementation files. Include the separate test module with `#[cfg(test)] mod test;`.

## Visibility

- Only use `pub` and private visibility; do not use restricted visibility such as `pub(crate)` or `pub(super)`.
- Default to private; make items `pub` only when they must cross a module boundary.
- If a child-module item is needed by the parent, make the item and its parent-called methods `pub` while keeping the child module private.
- Prevent cross-module mutation of internal state unless the mutation happens through an explicit API, or the struct is a transient data carrier owned by the caller.
