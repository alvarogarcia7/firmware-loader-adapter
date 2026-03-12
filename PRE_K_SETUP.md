# Prek Git Hooks Setup

This project uses [prek](https://prek.j178.dev/) to manage git hooks that automatically run quality checks before commits and pushes.

## Installation

### Option 1: Using cargo (Recommended for Rust projects)

```bash
cargo install prek
```

### Option 2: Using Homebrew (macOS/Linux)

```bash
brew install j178/tap/prek
```

### Option 3: Using prebuilt binaries

Download the latest release from [prek releases](https://github.com/j178/prek/releases) and place it in your PATH.

## Setup

After installing prek, run the following command in the project root to install the git hooks:

```bash
prek install
```

This will create git hooks in `.git/hooks/` that will execute the commands defined in `.prek.yaml`.

## Configuration

The `.prek.yaml` file uses pre-commit compatible YAML format and defines the following hooks:

### Pre-commit hooks
- Runs `make fmt-check` to verify code formatting
- Runs `make clippy` to catch common mistakes and improve code quality

### Pre-push hooks
- Runs `make fmt-check` to verify code formatting
- Runs `make lint` to run all linting checks
- Runs `make test` to execute the full test suite

## Usage

Once installed, the hooks will automatically run at the appropriate git lifecycle events:

- Before each commit, format and clippy checks will run
- Before each push, format checks, linting, and tests will run

If any check fails, the commit or push will be aborted, and you'll need to fix the issues before proceeding.

## Bypassing Hooks (Not Recommended)

In exceptional cases, you can bypass the hooks with:

```bash
git commit --no-verify
git push --no-verify
```

However, this should be avoided as it defeats the purpose of automated quality checks.

## Uninstalling

To remove the prek hooks:

```bash
prek uninstall
```

## Updating Configuration

If you modify `.prek.yaml`, you don't need to reinstall. The hooks will automatically use the updated configuration on the next run.
