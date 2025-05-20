
**核心原则：**

*   **MVP (Minimum Viable Product) 优先**: 尽快发布一个包含核心功能的可用的版本。
*   **迭代开发**: 每个阶段都有可交付的成果，并根据反馈进行调整。
*   **模块化**: 各个功能模块尽可能独立开发和测试。

---

**开发阶段划分及内容：**

**阶段 0: 准备与基础架构搭建 (预计 1-2 周)**

1.  **需求细化与技术选型最终确认**:
    *   再次梳理功能清单的优先级。
    *   最终确定核心技术栈 (Rust 框架, 前端框架, 数据库等)。
2.  **项目初始化**:
    *   创建 Git 仓库。
    *   搭建 Rust 后端项目结构 (e.g., using `cargo new --lib` for core logic and `cargo new` for binaries)。
    *   搭建 React 前端项目结构 (e.g., using Vite + TypeScript)。
3.  **数据库设计与初始化**:
    *   设计核心数据表结构 (VPS 信息, 用户, 基础性能指标表)。
    *   选择并安装数据库 (TimescaleDB/PostgreSQL)。
    *   编写初始数据库迁移脚本 (e.g., using `sqlx-cli` or a dedicated migration tool)。
4.  **Agent-Server 基础通信**:
    *   定义 Agent 与 Server 之间最基础的通信协议 (e.g., gRPC or HTTPS + JSON/MessagePack)。
    *   实现 Agent 注册/心跳机制。
    *   Server 端能接收并记录 Agent 状态。
5.  **CI/CD 初步搭建 (可选，但推荐)**:
    *   配置基本的自动化构建和测试流程 (e.g., GitHub Actions)。

**阶段 1: MVP - 核心监控与展示 (预计 4-6 周)**

1.  **Agent - 核心性能数据采集**:
    *   实现 CPU 使用率采集。
    *   实现内存使用率采集。
    *   实现磁盘 I/O 采集 (读/写速率)。
    *   实现网络流量采集 (总流量，速率)。
    *   实现磁盘空间使用率采集。
    *   Agent 将采集数据发送给 Server。
2.  **Server - 数据接收与存储**:
    *   API 接收 Agent 上报的性能数据。
    *   数据验证与清洗。
    *   将性能数据存入 TimescaleDB。
3.  **Frontend - VPS 管理与实时数据显示**:
    *   用户登录/注册 (简单实现，后续可加强)。
    *   添加/编辑/删除 VPS 信息 (IP, 名称等)。
    *   列表展示已添加的 VPS 及其在线状态。
    *   选择单个 VPS，实时图表展示其核心性能数据 (CPU, 内存, 网络, 磁盘IO)。
    *   (可选) 最简单的 WebSocket 推送实现实时数据。
4.  **Server - 基础 API**:
    *   提供管理 VPS 的 CRUD API。
    *   提供查询实时/短时历史性能数据的 API。

**阶段 2: 增强监控与用户体验 (预计 4-6 周)**

1.  **Agent - Docker 监控**:
    *   采集 VPS 上的 Docker 容器列表。
    *   采集各容器的 CPU, 内存使用情况。
    *   采集容器状态 (running, stopped, etc.)。
    *   Agent 将 Docker 数据上报 Server。
2.  **Server - Docker 数据处理与 API**:
    *   存储 Docker 容器信息和性能数据。
    *   提供查询和管理 Docker 容器的 API (列出, 启动, 停止, 重启 - 先实现 API，Agent 端后续实现执行)。
3.  **Frontend - Docker 监控与管理界面**:
    *   展示 VPS 上的 Docker 容器列表及其状态和资源使用。
    *   提供操作按钮 (启动, 停止, 重启容器)。
4.  **Agent - 执行 Docker 管理命令**:
    *   Agent 接收 Server 下发的 Docker 操作指令并执行。
5.  **历史性能数据查看**:
    *   Server 端 API 支持按时间范围查询历史性能数据。
    *   Frontend 实现历史数据图表展示 (支持选择时间范围，如最近1小时, 24小时, 7天等)。
6.  **VPS 上下线提醒 (基础告警)**:
    *   Server 检测 Agent 心跳超时。
    *   实现简单的邮件通知功能，当 VPS 离线/上线时发送邮件。
7.  **VPS 附加信息管理**:
    *   Frontend 和 Server 支持添加/编辑 VPS 的商家信息, 购买地址, 线路, 到期日等。

**阶段 3: 告警与任务系统 (预计 5-7 周)**

1.  **高级告警系统**:
    *   Server 端实现告警规则配置 (CPU 阈值, 内存阈值, 离线等)。
    *   AlertManager 逻辑：根据规则分析实时数据并触发告警。
    *   Frontend 实现告警规则配置界面。
    *   支持多种告警渠道 (先实现 Email，后续可扩展 Slack, Telegram 等)。
    *   Frontend 展示告警历史。
2.  **定时/一次性任务系统 (基础 - 非 Ansible)**:
    *   Agent 端实现执行简单命令/脚本的功能 (如 `ping 指定域名`)。
    *   Server 端设计任务调度逻辑 (存储任务定义, 触发执行)。
    *   Frontend 实现创建/管理定时任务和一次性任务 (如定时 Ping)。
    *   查看任务执行历史和结果。
3.  **每月流量监控与告警**:
    *   Agent 持续监控总流量。
    *   Server 聚合月流量数据。
    *   Frontend 展示月流量使用情况。
    *   配置流量阈值告警。

**阶段 4: 高级功能与集成 (预计 6-8 周)**

1.  **Ansible 集成 (用于任务系统)**:
    *   Server 端集成调用 Ansible CLI 的能力。
    *   设计如何管理 Ansible Inventory (动态生成或用户提供)。
    *   任务系统支持选择 Ansible Playbook 执行。
    *   处理 Ansible 执行结果。
    *   Frontend 界面适配 Ansible 任务。
2.  **Webshell**:
    *   Agent 端实现 PTY 交互逻辑。
    *   Server 端实现 WebSocket 代理，中继 Agent 和 Frontend 的数据。
    *   Frontend (Xterm.js) 实现终端界面。
    *   (可选) 批量 Webshell (基于 Ansible 或多路复用 Agent 连接)。
3.  **文件管理**:
    *   Agent 端实现列出目录、上传、下载文件的接口。
    *   Server 端代理文件操作请求。
    *   Frontend 实现文件浏览器界面。
4.  **流媒体解锁与 IP 风险检测**:
    *   Agent 端执行预定义的检测脚本。
    *   或 Server 端集成第三方 API 调用。
    *   Server 存储检测结果。
    *   Frontend 展示检测状态。
5.  **VPS 初始化脚本**:
    *   集成到任务系统，作为一种特殊任务类型。
    *   Server 端管理不同操作系统的初始化脚本模板。
    *   用户选择 VPS 和脚本模板执行。

**阶段 5: AI 交互与外部对接 (预计 3-5 周)**

1.  **MCP Server 实现**:
    *   设计 AI 客户端交互的 API (gRPC 或专用协议)。
    *   实现 MCP Server 逻辑，连接到主 Server API 获取信息和执行操作。
    *   编写示例 AI 客户端或测试工具。
2.  **对接商家后台 (WHMCS 等)**:
    *   研究目标商家后台的 API。
    *   实现 API 对接，同步 VPS 信息、账单、流量 (如果支持)。
    *   Frontend 展示从商家后台同步的信息。
3.  **Agent 设计参考 Alloy (深入)**:
    *   如果初期 Agent 设计较为简单，此阶段可以重构 Agent，使其更插件化、配置更灵活。
    *   实现 Agent 配置的热加载/远程更新。

**阶段 6: 优化、测试与文档 (持续进行，但此阶段重点投入)**

1.  **性能优化**:
    *   Agent 资源占用优化 (CPU, 内存, 二进制大小)。
    *   Server 端 API 响应速度优化。
    *   数据库查询优化。
2.  **安全性加固**:
    *   代码审计，依赖项安全扫描。
    *   加强认证授权机制。
    *   防止常见 Web 攻击 (XSS, CSRF, SQL注入等)。
3.  **全面测试**:
    *   单元测试、集成测试、端到端测试。
    *   压力测试。
4.  **用户文档编写**:
    *   安装部署指南。
    *   用户手册。
    *   API 文档 (如果需要对外开放)。
5.  **完善 CI/CD**:
    *   自动化测试、自动化部署。
6.  **用户体验 (UX) 改进**:
    *   根据内测或早期用户反馈，优化前端界面和交互流程。
