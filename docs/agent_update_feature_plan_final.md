# Agent 自动更新功能设计方案 (最终版)

## 1. 核心思想

本方案旨在为 Agent 实现一个安全、可靠、全自动的更新机制。其核心思想是将更新的决策权和执行权完全下放给 Agent 自身，使得系统更加去中心化、解耦和健壮。

- **Server (后端)**: 角色简化为“传话筒”，仅负责向 Agent 转发由前端触发的“立即检查更新”指令。
- **Agent (代理)**: 承担更新的全部责任。它既能按计划定时自检更新，也能响应 Server 的指令立即检查。

## 2. 核心流程

Agent 的更新流程是整个功能的核心，设计上强调安全性和原子性。

```mermaid
graph TD
    subgraph "Agent 更新流程"
        direction LR
        A[触发: 定时器或手动指令] --> B{获取更新锁};
        B -- "成功 (未在更新)" --> C[获取最新版本信息];
        B -- "失败 (正在更新)" --> D[跳过, 记录日志];
        C --> E{版本 > 当前版本?};
        E -- "是" --> F[下载新版本];
        E -- "否" --> G[流程正常结束];
        F --> H{下载成功?};
        H -- "是" --> I[启动新版本健康检查];
        H -- "否" --> J[记录错误, 结束];
        I --> K{检查成功?};
        K -- "是" --> L[替换旧版本];
        K -- "否" --> J;
        L --> M{替换成功?};
        M -- "是" --> N[退出当前进程 (由外部管理器重启)];
        M -- "否" --> J;
    end

    subgraph "最终状态"
        direction LR
        D --> Z[End]
        G --> Z
        J --> Z
        N --> Z
    end

    linkStyle 2 stroke-width:2px,fill:none,stroke:green;
    linkStyle 3 stroke-width:2px,fill:none,stroke:red;
    linkStyle 5 stroke-width:2px,fill:none,stroke:green;
    linkStyle 6 stroke-width:2px,fill:none,stroke:gray;
    linkStyle 8 stroke-width:2px,fill:none,stroke:green;
    linkStyle 10 stroke-width:2px,fill:none,stroke:green;
    linkStyle 12 stroke-width:2px,fill:none,stroke:green;
```

## 3. 模块改造详情

### 3.1. 后端 (Server)

- **数据模型**:
  - 在 `vps` 表中保留 `agent_version` 字段，用于 UI 展示。
- **通信协议**:
  - 定义一个无参数的指令 `TriggerUpdateCheckCommand`，用于触发 Agent 检查更新。
- **API**:
  - 提供 `POST /api/agents/{agent_id}/trigger-update-check` 接口，供前端调用。

### 3.2. Agent (核心)

- **版本信息**:
  - 在编译时通过 `build.rs` 将 `CARGO_PKG_VERSION` 嵌入二进制文件。
- **并发控制 (重要)**:
  - 在 Agent 的内存状态中，增加一个原子锁（如 `Arc<Mutex<bool>>`），命名为 `is_updating`。
  - 在更新流程开始时，必须先获取锁。如果获取失败，则说明已有更新在进行，本次触发将被跳过。
  - 在流程的**所有**退出点（无论成功、失败或无需更新），都必须确保释放该锁。推荐使用 RAII Guard 模式来自动管理锁的生命周期。
- **更新检查模块**:
  - Agent 启动后，作为一个独立的后台任务运行。
  - 内置定时器，周期性地（可配置，如每6小时）执行检查。
  - 检查逻辑：访问 GitHub API -> 比较版本号 -> 触发更新。
- **指令响应**:
  - 在主消息循环中，响应 `TriggerUpdateCheckCommand` 指令，立即执行一次更新检查（同样需要遵守并发控制）。
- **安全更新流程**:
  1.  **下载**: 下载新版二进制文件到临时路径。
  2.  **校验 (推荐)**: 对比下载文件的 SHA256 哈希值。
  3.  **健康检查**: 使用特殊参数（如 `--health-check`）启动新版 Agent。该模式下 Agent 会完成初始化和连接测试，然后正常退出。
  4.  **替换**: 如果健康检查成功，使用原子性的 `rename` 操作覆盖旧文件。
  5.  **重启**: 替换成功后，当前进程退出，由外部进程管理器（`systemd`, `supervisor` 等）负责重启。

### 3.3. 前端 (Web UI)

- **UI 变更**:
  - 在 Agent 列表页，将“更新”按钮改为“**检查更新**”。
- **交互逻辑**:
  - 点击按钮后，调用后端的 `trigger-update-check` API。
  - 向用户显示操作已发出的提示信息。

## 4. 依赖项

- **Agent**:
  - `tokio` (已有)
  - `reqwest` (用于 HTTP 请求)
  - `serde` (用于解析 API 响应)
  - `semver` (用于版本号比较)