# Repository Guidelines

## Project Structure & Module Organization
This repository is planning-first. [`implementation-plan.md`](./implementation-plan.md) is the source of truth for architecture, phasing, dependencies, and the intended Rust crate layout. Check it before introducing new files or changing terminology.

`local-notes/` is ignored scratch space for research and proposal work, not reviewable source. The planned implementation is a single Rust crate with modules under `src/` such as `rdf/`, `owl/`, `store/`, `compile/`, `engine/`, and `output/`.

## Build, Test, and Development Commands
There is no checked-in build pipeline yet. For current contributions, validate documentation changes with:

- `git diff --check` to catch whitespace problems and merge markers.
- `rg "Phase|Module Structure|Conventions" implementation-plan.md` to confirm references still match the plan.

When the Rust crate is scaffolded, use the commands already assumed by the plan:

- `cargo fmt` to format code.
- `cargo clippy --all-targets --all-features` for linting.
- `cargo test` for unit and integration tests.

## Coding Style & Naming Conventions
Keep Markdown concise, heading-driven, and specific. Prefer short paragraphs and flat bullet lists. Match the existing tone: technical, direct, and design-focused.

For planned Rust code, use 4-space indentation, `snake_case` for modules and functions, `CamelCase` for types, and `SCREAMING_SNAKE_CASE` for constants. This project values inference correctness, type safety, memory safety, and predictable behavior over short-term convenience. Prefer explicit types, checked error paths, and ownership-aware designs. Avoid `unwrap()` in library code, avoid `unsafe` unless it is clearly justified and reviewed, and do not reach for escape hatches that weaken safety or correctness guarantees. The plan explicitly favors `anyhow` at application boundaries and `thiserror` for internal errors.

## Testing Guidelines
No automated test suite is committed yet. Every change should preserve consistency with `implementation-plan.md` and avoid contradictory terminology across docs.

When code is added, place tests close to the affected module and favor focused coverage for parsing, storage, and inference behavior. Run `cargo test` before opening a PR once the crate exists.

## Commit & Pull Request Guidelines
History is minimal and currently uses short sentence-style subjects such as `Initial commit.` Keep future commit titles short, imperative, and scoped to one concern.

PRs should explain what changed, why it changed, and whether the update affects the implementation plan, research notes, or future module boundaries. Include command output for any new tooling or tests. Screenshots are only useful for rendered diagrams or generated reports.
