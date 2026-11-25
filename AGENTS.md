# MindsWall contributor guide

MindsWall is an early-stage, local-first wallpaper manager targeting Windows 11.
The goal is a lightweight desktop utility that works offline and without
administrator privileges. Keep these goals in mind when making changes.

## Communication

- Reply in the language the user writes in.
- Write code, comments, commit messages, and repository documentation in English.
- Describe planned functionality as planned; do not claim it is implemented or
  verified before it exists and has been checked.

## Project context

- `docs/requirements/requirement.md`: feature priorities, planned architecture,
  technology stack, roadmap, and performance targets.
- `src/main.rs`: current Rust entry point and early recursive image scanner
  example. Its folder path is currently hard-coded for local experimentation.
- `Cargo.toml` and `Cargo.lock`: Rust package metadata and dependencies.
- `README.md`: project overview and current development instructions.
- `LICENSE`: MIT license.

The source code describes current behavior. The requirements describe goals,
including a Tauri 2 desktop app with a React and TypeScript frontend; they do
not imply that those components or the proposed project structure exist yet.

## Development guidelines

- Keep image processing and configuration local. Do not introduce network
  dependencies or administrator requirements without an explicit decision.
- Prefer clear, idiomatic Rust and handle filesystem and Windows API errors
  explicitly. Avoid assuming paths are valid Unicode.
- Keep Windows-specific code separate from reusable logic as the project grows.
- Preserve support for JPG, JPEG, PNG, and BMP when changing image discovery.
- For slideshow and shuffle work, follow the requirements: avoid repeats until
  the current set has been exhausted.
- Keep background resource use low; the requirements list performance targets,
  which should be measured before being reported as achieved.
- Update the README when implemented behavior or development instructions change.

## Commands and verification

Run commands from the repository root with Rust and Cargo installed:

```powershell
cargo run          # Run the current scanner example after setting its folder path
cargo fmt --check  # Check Rust formatting
cargo check        # Check compilation
cargo test         # Run Rust tests
cargo clippy       # Run Rust lints when the component is installed
```

Choose checks appropriate to the change. Add meaningful tests for behavior
when needed, particularly for scanning, shuffle, scheduling, configuration,
and error handling. Windows-specific behavior must be verified on Windows;
do not claim it works based only on platform-independent tests. Report checks
run and any relevant checks not run.

## Git and releases

- Do not create commits or tags unless the user explicitly requests them.
- Do not push or create a GitHub Release. Prepare local work and provide the
  user with the relevant push or release command.
- Do not add AI or agent attribution to commits, tags, pull requests, or release
  notes.

## After code changes

Report the important changes, checks run and not run, remaining concerns, and a
suggested Conventional Commit message. Base the suggested message on staged
changes if any exist; otherwise use the working-tree diff. Include a relevant
scope and any Jira key present in the diff, and keep the subject within 100
characters.

Commit message rules:

1. Base it on staged changes (`git diff --cached`) if any exist; otherwise on
   `git diff`. Never mix the two.
2. Use Conventional Commits: `<type>(<scope>): <summary>`, with a scope when
   one is identifiable (for example `sql`, `images`, `remove-background`).
3. Include Jira keys after the scope if they appear in the diff.
4. Keep the subject at 100 characters or fewer and focus on user or business
   impact, not syntax noise. Ignore whitespace-only changes.

Example:

```
fix(remove-background): reset mounted flag on mount so results are not dropped under StrictMode
```
