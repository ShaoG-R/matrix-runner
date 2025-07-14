# matrix-runner

A powerful, configuration-driven test runner for Rust projects to execute tests across a wide matrix of feature flags and environments.

## Core Features

- **Matrix Testing**: Define a comprehensive test matrix in a simple TOML file.
- **Parallel Execution**: Run tests concurrently to get faster feedback, with configurable job counts.
- **Fail-Fast Strategy**: Automatically stops all tests on the first failure of a "safe" test case, saving time and CI resources.
- **Flaky Test Handling**: Gracefully handle tests that are expected to fail on certain platforms (`allow_failure`).
- **Distributed Testing**: Natively supports splitting the test matrix across multiple CI runners.
- **Target Filtering**: Automatically selects tests to run based on the host's architecture (`x86`, `x86_64`, `aarch64`, etc.).
- **Failure Artifacts**: Failed test runs leave their build artifacts in a `target-errors` directory for easy debugging.
- **Internationalization (i18n)**: Console output supports multiple languages (currently English and Chinese).

## Why `matrix-runner`?

Testing Rust projects with a large number of feature flags, especially those supporting `no_std` environments or using hardware-specific optimizations, can be complex. It's easy to miss a broken combination. `matrix-runner` automates this process, ensuring all specified configurations are continuously tested. It was built to test complex cryptographic libraries, but it's generic enough for any Rust project.

## Installation

```bash
cargo install matrix-runner
```

*(Note: The crate is not yet published to crates.io. You can install it from the git repository)*

## Usage

Navigate to your Rust project's root directory and run:

```bash
matrix-runner [OPTIONS]
```

### Key Options:

- `-c, --config <PATH>`: Path to the test matrix config file. Defaults to `TestMatrix.toml`.
- `-j, --jobs <NUMBER>`: Number of parallel jobs to run. Defaults to a sensible value based on your logical CPU cores.
- `--project-dir <PATH>`: Path to the project directory to test. Defaults to the current directory (`.`).
- `--total-runners <NUMBER>`: The total number of parallel runners you are splitting the tests across (for CI).
- `--runner-index <NUMBER>`: The 0-based index of the current runner.

### Example: Running tests in a CI environment with two parallel machines

**Machine 1:**
```bash
matrix-runner --total-runners 2 --runner-index 0
```

**Machine 2:**
```bash
matrix-runner --total-runners 2 --runner-index 1
```

## Configuration (`TestMatrix.toml`)

The behavior of `matrix-runner` is controlled by a TOML file (e.g., `TestMatrix.toml`). This file contains global settings and an array of `[[cases]]`, where each case represents a single `cargo test` invocation with a specific configuration.

### Global Settings

- `language` (String, optional): Sets the output language for the console. Supports `"en"` and `"zh-CN"`. Defaults to `"en"`.

### Case Parameters:

- `name` (String, required): A unique, human-readable name for the test case.
- `features` (String, required): A comma-separated list of features to enable for this test run.
- `no_default_features` (Boolean, required): If `true`, the `--no-default-features` flag is passed to Cargo.
- `allow_failure` (Array of Strings, optional): A list of OS or architecture identifiers (e.g., `"windows"`, `"aarch64"`) where this case is allowed to fail without stopping the entire test suite.
- `arch` (Array of Strings, optional): A list of architectures this test is valid for. If the host machine's architecture is not in this list, the test is skipped.

### Example Configuration:

```toml
# TestMatrix.toml

# Global setting for output language
language = "zh-CN"

# A basic case with default features
[[cases]]
name = "std-default"
features = ""
no_default_features = false

# A no_std case, enabling a specific feature
[[cases]]
name = "no_std-minimal"
features = "some-feature"
no_default_features = true

# A case that runs only on x86_64 and is allowed to fail on Windows
[[cases]]
name = "x64-specific-optimized"
features = "avx2-optimizations"
no_default_features = false
arch = ["x86_64"]
allow_failure = ["windows"]
```

## License

This project is not yet licensed. Please choose an appropriate open-source license (e.g., MIT or Apache-2.0).

## Contributing

Contributions are welcome! Please feel free to submit a pull request. 