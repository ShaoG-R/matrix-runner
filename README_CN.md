# matrix-runner

一个功能强大的、由配置驱动的 Rust 项目测试执行器，用于在广泛的特性标志和环境矩阵中执行测试。

## 核心功能

- **矩阵测试**: 在一个简单的 TOML 文件中定义全面的测试矩阵。
- **并行执行**: 并发运行测试以获得更快的反馈，并可配置任务数。
- **快速失败策略**: 在“安全”测试用例首次失败时自动停止所有测试，节省时间和 CI 资源。
- **处理不稳定测试**: 优雅地处理那些在特定平台上预期会失败的测试 (`allow_failure`)。
- **分布式测试**: 原生支持在多个 CI 执行器之间拆分测试矩阵。
- **目标过滤**: 根据主机的体系结构（`x86`、`x86_64`、`aarch64` 等）自动选择要运行的测试。
- **失败产物**: 失败的测试运行会将其构建产物保留在 `target-errors` 目录中，以便于调试。
- **国际化 (i18n)**: 控制台输出支持多语言（当前支持英文和中文）。

## 为何选择 `matrix-runner`？

测试具有大量特性标志的 Rust 项目，特别是那些支持 `no_std` 环境或使用特定硬件优化的项目，可能会很复杂。开发者很容易错过某个损坏的功能组合。`matrix-runner` 会自动执行此过程，确保所有指定的配置都得到持续测试。它最初是为了测试复杂的加密库而构建的，但其通用性足以适用于任何 Rust 项目。

## 安装

```bash
cargo install matrix-runner
```

*(注意: 该项目尚未发布到 crates.io。您可以从其 git 仓库进行安装)*

## 使用方法

导航到您的 Rust 项目根目录并运行：

```bash
matrix-runner [OPTIONS]
```

### 主要选项:

- `-c, --config <PATH>`: 测试矩阵配置文件的路径。默认为 `TestMatrix.toml`。
- `-j, --jobs <NUMBER>`: 要运行的并行任务数。默认值为根据您的逻辑 CPU 核心数计算的合理值。
- `--project-dir <PATH>`: 要测试的项目的路径。默认为当前目录 (`.`)。
- `--total-runners <NUMBER>`: 用于拆分测试的并行执行器总数（用于 CI）。
- `--runner-index <NUMBER>`: 当前执行器的索引（从 0 开始）。

### 示例：在具有两台并行计算机的 CI 环境中运行测试

**机器 1:**
```bash
matrix-runner --total-runners 2 --runner-index 0
```

**机器 2:**
```bash
matrix-runner --total-runners 2 --runner-index 1
```

## 配置 (`TestMatrix.toml`)

`matrix-runner` 的行为由一个 TOML 文件（例如 `TestMatrix.toml`）控制。该文件包含全局设置和 `[[cases]]` 数组，其中每个 case 代表一个具有特定配置的 `cargo test` 调用。

### 全局设置

- `language` (字符串, 可选): 设置控制台的输出语言。支持 `"en"` 和 `"zh-CN"`。默认为 `"en"`。

### Case 参数:

- `name` (字符串, 必需): 测试用例的唯一的、人类可读的名称。
- `features` (字符串, 必需): 为此测试运行启用功能的逗号分隔列表。
- `no_default_features` (布尔值, 必需): 如果为 `true`，则将 `--no-default-features` 标志传递给 Cargo。
- `allow_failure` (字符串数组, 可选): 一个操作系统或体系结构标识符的列表（例如 `"windows"`、`"aarch64"`），在这些平台上，此用例允许失败而不会停止整个测试套件。
- `arch` (字符串数组, 可选): 此测试适用的体系结构列表。如果主机的体系结构不在此列表中，则跳过该测试。

### 配置示例:

```toml
# TestMatrix.toml

# 用于设置输出语言的全局配置
language = "zh-CN"

# 一个使用默认功能的基本用例
[[cases]]
name = "std-default"
features = ""
no_default_features = false

# 一个 no_std 用例，启用特定功能
[[cases]]
name = "no_std-minimal"
features = "some-feature"
no_default_features = true

# 一个仅在 x86_64 上运行且允许在 Windows 上失败的用例
[[cases]]
name = "x64-specific-optimized"
features = "avx2-optimizations"
no_default_features = false
arch = ["x86_64"]
allow_failure = ["windows"]
```

## 许可证

该项目尚未获得许可。请选择一个合适的开源许可证（例如 MIT 或 Apache-2.0）。

## 贡献

欢迎贡献！请随时提交拉取请求。 