# Cargo Workspace 重构手动指南

请按照以下步骤操作，这会引导你完成整个重构过程。

---

## 第 1 步：准备目录结构

1.  **重命名**: 将 `backend/Cargo.toml` 文件重命名为 `backend/Cargo.toml.old`，暂时保管。
2.  **创建目录**: 在 `backend/` 目录下创建一个新目录 `crates`。
3.  **移动 `src`**: 将 `backend/src` 整个目录移动到 `backend/crates/` 下，并**重命名**为 `server`。
    *   现在的路径应该是 `backend/crates/server`。
4.  **创建 `agent` 目录**: 在 `backend/crates/` 目录下创建一个新的空目录 `agent`。
5.  **创建 `agent/src`**: 在 `backend/crates/agent/` 目录下创建一个新的空目录 `src`。

**操作完成后，你的 `backend` 目录结构应该看起来像这样：**

```
backend/
├── Cargo.toml.old      (旧的配置文件)
├── crates/
│   ├── agent/
│   │   └── src/        (空的)
│   └── server/         (原有的 src 目录)
├── build.rs
├── proto/
├── migrations/
... (其他原有文件和目录)
```

---

## 第 2 步：创建新的 `Cargo.toml` 文件

现在，我们需要创建三个新的 `Cargo.toml` 文件来定义 Workspace 和新的 `agent`/`server` crates。

### 1. 创建 `backend/Cargo.toml` (Workspace 根)

在 `backend/` 目录下创建一个新的 `Cargo.toml` 文件，内容如下：

```toml
[workspace]
members = [
    "crates/agent",
    "crates/server",
]
resolver = "2"

# You can define shared profile settings here
[profile.release]
debug = 1
```

### 2. 创建 `backend/crates/server/Cargo.toml`

这个文件的大部分内容来自你之前备份的 `Cargo.toml.old`。在 `backend/crates/server/` 目录下创建一个新的 `Cargo.toml`，内容如下：

```toml
[package]
name = "nodenexus-server"
version = "0.1.1"
edition = "2021"

[dependencies]
# --- 从 Cargo.toml.old 复制几乎所有的依赖到这里 ---
# --- 除了 dhat, prost-build, tonic-build ---
bytes = "1.10"
prost = "0.13"
tokio = { version = "1.45", features = ["macros", "rt-multi-thread", "sync", "time", "process", "signal"] }
tonic = { version = "0.13", features = ["transport", "codegen", "prost", "tls-native-roots"] }
tokio-rustls = "0.26"
dashmap = "6.1"
rustls = "0.23"
chrono = { version = "0.4", features = ["serde"] }
futures-util = { version = "0.3" }
tokio-stream = "0.1"
uuid = { version = "1.17", features = ["v4"] }
sysinfo = "0.35"
netdev = "0.35"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sqlx = { version = "0.8", features = [ "runtime-tokio-rustls", "postgres", "macros", "chrono", "json" ] }
axum = { version = "0.8", features = ["ws", "macros"] }
jsonwebtoken = "9"
bcrypt = "0.17"
rustls-native-certs = "0.8"
thiserror = "2.0"
dotenv = "0.15"
toml = "0.8"
clap = { version = "4.5", features = ["derive"] }
tower-service = "0.3"
tower-http = { version = "0.6", features = ["cors"] }
futures = "0.3"
once_cell = "1.21"
tera = "1.20"
aes-gcm = "0.10"
async-trait = "0.1"
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
hex = "0.4"
rust-i18n = "3.1"
axum-extra = { version = "0.10", features = ["cookie"] }
sea-orm = { version = "1.1.12", features = ["sqlx-postgres", "runtime-tokio-rustls", "macros", "chrono", "with-json"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json", "registry"] }
tracing-appender = "0.2"
tracing-log = "0.2"
surge-ping = "0.8"
encoding_rs = "0.8"
lazy_static = "1.5"
rand = "0.9"
rust-embed = "8.7"
mime_guess = "2.0"
http-body-util = "0.1"
hyper = "1.6.0"
tower = "0.5.2"
time = "0.3"
urlencoding = "2.1.3"
tempfile = "3.20"
tokio-tungstenite = { version = "0.27", features = ["rustls-tls-native-roots"] }

[build-dependencies]
prost-build = "0.13"
tonic-build = { version = "0.13", features = ["prost"] }
```

### 3. 创建 `backend/crates/agent/Cargo.toml`

这是 `agent` 的专属配置文件，只包含必要的依赖。在 `backend/crates/agent/` 目录下创建一个新的 `Cargo.toml` 文件，内容如下：

```toml
[package]
name = "nodenexus-agent"
version = "0.1.1"
edition = "2021"

[dependencies]
# 核心 & 通信
tokio = { version = "1.45", features = ["macros", "rt-multi-thread", "sync", "time", "process", "signal"] }
tonic = { version = "0.13", features = ["transport", "codegen", "prost", "tls-native-roots"] }
prost = "0.13"
futures-util = "0.3"
tokio-rustls = "0.26"
rustls = "0.23"
rustls-native-certs = "0.8" # For establishing trusted connections
bytes = "1.10"

# 指标 & 系统
sysinfo = "0.35"
netdev = "0.35"

# 配置 & 日志 & 错误处理
clap = { version = "4.5", features = ["derive"] }
toml = "0.8"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"
thiserror = "2.0"

# 辅助
uuid = { version = "1.17", features = ["v4"] }
once_cell = "1.21"

# Windows 服务特定依赖
[target.'cfg(windows)'.dependencies]
windows-service = "0.8"
winapi = { version = "0.3", features = ["winnt", "winuser", "errhandlingapi"] }
windows-sys = { version = "0.60", features = ["Win32_Globalization"] }
codepage = "0.1"

# 可选的 dhat 性能分析
dhat = { version = "0.3", optional = true }

[features]
dhat-heap = ["dhat"]
```

---

## 第 3 步：迁移代码和调整路径

1.  **移动 `agent` 代码**: 将 `backend/crates/server/src/bin/agent.rs` 文件移动到 `backend/crates/agent/src/` 目录下，并**重命名**为 `main.rs`。
2.  **移动 `agent` 模块**: 将 `backend/crates/server/src/agent_modules` 整个目录移动到 `backend/crates/agent/src/` 目录下。
3.  **调整 `agent` 的 `main.rs`**:
    *   打开新的 `backend/crates/agent/src/main.rs` 文件。
    *   删除所有 `use backend::...` 这样的行。
    *   在文件顶部添加 `mod agent_modules;`。
    *   将所有 `use backend::agent_modules::...` 修改为 `use crate::agent_modules::...`。
    *   暂时将引用 `version` 的地方硬编码为 `"0.1.1"` 或者注释掉。

4.  **调整 `server` 的 `lib.rs` 和 `main.rs`**
    *   删除 `backend/crates/server/src/bin/agent.rs` (因为它已经被移动了)。
    *   检查 `backend/crates/server/src/lib.rs`，删除 `pub mod agent_modules;` 这一行。

---

完成以上所有步骤后，就可以尝试在 `backend` 目录下运行 `cargo build` 来检查是否有编译错误。