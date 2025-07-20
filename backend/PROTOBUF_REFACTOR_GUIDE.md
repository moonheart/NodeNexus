# Protobuf 重构指南 (Workspace)

这是在 Workspace 中处理共享 Protobuf 文件的最佳实践。

---

## 第 1 步：创建 `common` Crate

1.  **创建目录**:
    *   在 `backend/crates/` 目录下，创建一个新目录 `common`。
    *   在 `backend/crates/common/` 目录下，创建一个新目录 `src`。

2.  **移动文件和目录**:
    *   将项目根目录的 `backend/build.rs` 文件移动到 `backend/crates/common/` 目录下。
    *   将项目根目录的 `backend/proto/` 整个目录移动到 `backend/crates/common/` 目录下。

**操作完成后，你的 `common` 目录结构应该如下：**
```
backend/crates/common/
├── build.rs      (从 backend/ 移动而来)
├── proto/        (从 backend/ 移动而来)
└── src/
```

---

## 第 2 步：配置 `common` Crate

1.  **创建 `backend/crates/common/Cargo.toml`**:
    内容如下。这个文件定义了编译 Protobuf 所需的依赖。

    ```toml
    [package]
    name = "nodenexus-common"
    version = "0.1.0"
    edition = "2021"

    [dependencies]
    prost = "0.13"
    tonic = { version = "0.13", features = ["prost"] }
    serde = { version = "1.0", features = ["derive"] }

    [build-dependencies]
    prost-build = "0.13"
    tonic-build = { version = "0.13", features = ["prost"] }
    ```

2.  **创建 `backend/crates/common/src/lib.rs`**:
    这个文件是 `common` 库的入口，它负责将 `build.rs` 生成的代码包含进来并公开。

    ```rust
    // 使用 serde 为所有生成的类型自动派生 Serialize 和 Deserialize
    fn add_serde_derive(builder: &mut tonic_build::Builder) -> &mut tonic_build::Builder {
        builder
            .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
    }

    // service.rs
    pub mod service {
        tonic::include_proto!("service");
    }

    // metrics.rs
    pub mod metrics {
        tonic::include_proto!("metrics");
    }

    // handshake.rs
    pub mod handshake {
        tonic::include_proto!("handshake");
    }

    // config.rs
    pub mod config {
        tonic::include_proto!("config");
    }
    
    // command.rs
    pub mod command {
        tonic::include_proto!("command");
    }

    // pty.rs
    pub mod pty {
        tonic::include_proto!("pty");
    }

    // messages.rs
    pub mod messages {
        tonic::include_proto!("messages");
    }

    // docker.rs
    pub mod docker {
        tonic::include_proto!("docker");
    }

    // generic_metrics.rs
    pub mod generic_metrics {
        tonic::include_proto!("generic_metrics");
    }

    // batch_command.rs
    pub mod batch_command {
        tonic::include_proto!("batch_command");
    }

    // common.rs
    pub mod common {
        tonic::include_proto!("common");
    }
    ```

3.  **修改 `backend/crates/common/build.rs`**:
    确保 `tonic_build` 能在当前 crate 内找到 `.proto` 文件。

    ```rust
    fn main() -> Result<(), Box<dyn std::error::Error>> {
        println!("cargo:rerun-if-changed=proto");
        tonic_build::configure()
            .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
            .compile(
                &[
                    "proto/service.proto",
                    "proto/metrics.proto",
                    "proto/handshake.proto",
                    "proto/config.proto",
                    "proto/command.proto",
                    "proto/pty.proto",
                    "proto/messages.proto",
                    "proto/docker.proto",
                    "proto/generic_metrics.proto",
                    "proto/batch_command.proto",
                    "proto/common.proto",
                ],
                &["proto"], // 指定 proto 文件的根目录
            )?;
        Ok(())
    }
    ```

---

## 第 3 步：更新 Workspace 和其他 Crates

1.  **更新根 `Cargo.toml`**:
    打开 `backend/Cargo.toml`，将 `common` 添加到 `members` 列表。

    ```toml
    [workspace]
    members = [
        "crates/agent",
        "crates/server",
        "crates/common", # <-- 添加这一行
    ]
    resolver = "2"
    # ...
    ```

2.  **更新 `server` Crate**:
    *   打开 `backend/crates/server/Cargo.toml`。
    *   **删除**整个 `[build-dependencies]` 部分。
    *   在 `[dependencies]` 部分，添加对 `common` 库的依赖：
        ```toml
        nodenexus-common = { path = "../common" }
        ```

3.  **更新 `agent` Crate**:
    *   打开 `backend/crates/agent/Cargo.toml`。
    *   在 `[dependencies]` 部分，添加对 `common` 库的依赖：
        ```toml
        nodenexus-common = { path = "../common" }
        ```

---

## 第 4 步：修改代码中的 `use` 路径

现在，你需要在 `agent` 和 `server` 的所有 Rust 代码中，将原来引用 Protobuf 生成代码的路径进行修改。

**查找**: `use backend::...` (例如 `use backend::agent_service::...`)
**替换为**: `use nodenexus_common::...` (例如 `use nodenexus_common::service::...`)

你需要仔细检查 `agent` 和 `server` 两个项目中的所有 `.rs` 文件，特别是 `main.rs`, `lib.rs` 以及所有子模块。

完成以上步骤后，再次尝试在 `backend` 根目录运行 `cargo build`。

---

## 第 5 步：修改 `build.rs`

### `common` Crate 的 `build.rs`

你的 `backend/crates/common/build.rs` 文件需要被简化，让它只负责编译 Protobuf 文件。

**请将 `backend/crates/common/build.rs` 的内容完全替换为以下代码：**

```rust
fn main() -> Result&lt;(), Box&lt;dyn std::error::Error&gt;&gt; {
    // 当 proto 目录下的任何文件发生变化时，重新运行此构建脚本
    println!("cargo:rerun-if-changed=proto");

    tonic_build::configure()
        // 为所有生成的类型统一添加 serde 支持，这比原先的写法更简洁、更易于维护
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile(
            &[
                "proto/common.proto",
                "proto/handshake.proto",
                "proto/config.proto",
                "proto/metrics.proto",
                "proto/docker.proto",
                "proto/generic_metrics.proto",
                "proto/command.proto",
                "proto/pty.proto",
                "proto/messages.proto",
                "proto/service.proto",
                "proto/batch_command.proto",
            ],
            &["proto"], // 指定 .proto 文件的根目录
        )?;

    Ok(())
}
```

### `server` Crate 的 `build.rs`

之前 `build.rs` 中复制 `locales` 目录的逻辑只跟 `server` 有关，我们需要把它移到 `server` crate 中。

**请在 `backend/crates/server/` 目录下创建一个新的 `build.rs` 文件，并将以下内容粘贴进去：**

```rust
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result&lt;(), Box&lt;dyn std::error::Error&gt;&gt; {
    // --- locale copying logic ---
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    
    // 从 target/debug/build/nodenexus-server-xxxx/out 导航到 target/debug
    let dest_path = out_dir.join("../../../"); 

    // `locales` 目录在 workspace 根目录，位于 server crate 目录的两级之上
    let source_path = Path::new("../../locales");

    if source_path.exists() {
        let target_locales_path = dest_path.join("locales");

        if !target_locales_path.exists() {
            fs::create_dir_all(&target_locales_path)?;
        }

        for entry in fs::read_dir(source_path)? {
            let entry = entry?;
            let source_file = entry.path();
            if source_file.is_file() {
                 let dest_file = target_locales_path.join(entry.file_name());
                 fs::copy(&source_file, &dest_file)?;
            }
        }
    }
    
    println!("cargo:rerun-if-changed=../../locales");
    // --- End of locale copying logic ---

    Ok(())
}
```

---

## 第 6 步：修复 `use` 引用路径

在 `agent` 和 `server` 的代码中，你需要更新 `use` 语句来正确引用 `common` crate 中的共享代码。

### 1. 修复 `common/src/lib.rs`

首先，确保 `backend/crates/common/src/lib.rs` 的内容是完整的。请用以下内容**完全覆盖**该文件：

```rust
// service.rs
pub mod service {
    tonic::include_proto!("service");
}

// metrics.rs
pub mod metrics {
    tonic::include_proto!("metrics");
}

// handshake.rs
pub mod handshake {
    tonic::include_proto!("handshake");
}

// config.rs
pub mod config {
    tonic::include_proto!("config");
}

// command.rs
pub mod command {
    tonic::include_proto!("command");
}

// pty.rs
pub mod pty {
    tonic::include_proto!("pty");
}

// messages.rs
pub mod messages {
    tonic::include_proto!("messages");
}

// docker.rs
pub mod docker {
    tonic::include_proto!("docker");
}

// generic_metrics.rs
pub mod generic_metrics {
    tonic::include_proto!("generic_metrics");
}

// batch_command.rs
pub mod batch_command {
    tonic::include_proto!("batch_command");
}

// common.rs
pub mod common {
    tonic::include_proto!("common");
}
```

### 2. 修改 `agent/src/main.rs`

现在，打开 `backend/crates/agent/src/main.rs` 并进行以下修改：

*   **查找:** `use crate::agent_service::AgentConfig;`
*   **替换为:** `use nodenexus_common::config::AgentConfig;`

*   **查找:** `use crate::version::VERSION;`
*   **替换为:** (直接删除这一行)

*   **查找所有使用 `VERSION` 的地方** (例如 `info!(version = VERSION, ...)` )
*   **将 `VERSION` 替换为** 硬编码的字符串 `"0.1.1"`。

*   **查找所有** `backend::agent_modules::...`
*   **替换为** `crate::agent_modules::...`

对 `agent` crate 中的其他文件（例如 `agent_modules` 里的文件）也进行类似的检查和替换。所有对 Protobuf 生成的代码的引用都应该以 `use nodenexus_common::...` 开头。