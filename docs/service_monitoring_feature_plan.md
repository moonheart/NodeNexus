# 服务监控功能 - 功能设计与技术方案 (V4 - 最终确认版)

**目标:** 实现一个灵活、稳定、可扩展的服务监控功能，允许用户通过指定的Agent对HTTP、TCP、Ping等服务进行定期探测，并记录和展示结果。

---

## 1. 核心流程

```mermaid
graph TD
    subgraph "用户操作 (Frontend)"
        A["用户在UI上创建/编辑监控任务"] --> B{"通过API将任务及分配规则保存到数据库"};
    end

    subgraph "配置下发 (Server -> Agent)"
        C["Server根据分配规则计算每个Agent的任务列表"] --> D["通过gRPC将任务配置下发给指定的Agent"];
    end

    subgraph "任务执行 (Agent)"
        E["Agent接收并维护一个监控任务列表"] --> F["根据任务配置, 定期执行HTTP/TCP/Ping探测"];
        F --> G["收集探测结果 (如: 状态码, 延迟)"];
        G --> H["将结果包装成'GenericMetric'通用指标"];
        H --> I["通过gRPC流上报给Server"];
    end

    subgraph "数据处理与展示 (Server & Frontend)"
        J["Server接收通用指标"] --> K["解析并存入'service_monitor_results'数据表"];
        L["Frontend通过API从Server查询监控结果"] --> M["在UI上展示服务状态列表和延迟图表"];
    end

    B --> C;
    I --> J;
```

---

## 2. 数据库模型

### 2.1. 实体关系图 (ERD)

```mermaid
erDiagram
    service_monitors {
        int id PK
        string name
        string monitor_type
        string target
        int frequency_seconds
        int timeout_seconds
        jsonb monitor_config
    }
    vps {
        int id PK
        string name
    }
    tags {
        int id PK
        string name
    }
    service_monitor_agents {
        int monitor_id FK
        int vps_id FK
    }
    service_monitor_tags {
        int monitor_id FK
        int tag_id FK
    }
    service_monitor_results {
        timestamp time PK
        int monitor_id FK
        int agent_id FK
        boolean is_up
        int latency_ms
    }

    service_monitors ||--o{ service_monitor_agents : "直接分配"
    vps ||--o{ service_monitor_agents : "执行"
    service_monitors ||--o{ service_monitor_tags : "按标签分配"
    tags ||--o{ service_monitor_tags : "应用"
    service_monitors ||--o{ service_monitor_results : "产生"
    vps ||--o{ service_monitor_results : "来自"
```

### 2.2. 表结构定义

*   **`service_monitors`**: 存储监控任务的配置。
    *   `id` (PK, serial)
    *   `user_id` (FK, integer, not null)
    *   `name` (varchar, not null)
    *   `monitor_type` (varchar, not null) - 'http', 'ping', 'tcp'
    *   `target` (varchar, not null)
    *   `frequency_seconds` (integer, not null, default: 60)
    *   `timeout_seconds` (integer, not null, default: 10)
    *   `is_active` (boolean, not null, default: true)
    *   `monitor_config` (jsonb, nullable) - 存储特定类型的配置
    *   `created_at`, `updated_at` (timestamptz)

*   **`service_monitor_agents`** (关联表):
    *   `monitor_id` (FK to `service_monitors.id`, PK)
    *   `vps_id` (FK to `vps.id`, PK)

*   **`service_monitor_tags`** (关联表):
    *   `monitor_id` (FK to `service_monitors.id`, PK)
    *   `tag_id` (FK to `tags.id`, PK)

*   **`service_monitor_results`** (TimescaleDB Hypertable):
    *   `time` (TIMESTAMPTZ, not null)
    *   `monitor_id` (FK to `service_monitors.id`, not null)
    *   `agent_id` (FK to `vps.id`, not null)
    *   `is_up` (boolean, not null)
    *   `latency_ms` (integer, not null)
    *   `details` (jsonb, nullable) - e.g., `{"status_code": 200, "error": ""}`

### 2.3. `monitor_config` 示例

```json
{
  "http": {
    "method": "GET",
    "expected_status_codes": [200],
    "request_headers": {},
    "request_body": null,
    "response_body_match": "",
    "ignore_tls_errors": false
  },
  "ping": {
    "packet_count": 4
  },
  "tcp": {}
}
```

---

## 3. Protobuf 扩展 (`proto/service.proto`)

```protobuf
// In message AgentConfig
message AgentConfig {
  // ... existing fields
  repeated ServiceMonitorTask service_monitor_tasks = 10;
}

// New message definition
message ServiceMonitorTask {
  int32 monitor_id = 1;
  string name = 2;
  string monitor_type = 3;
  string target = 4;
  int32 frequency_seconds = 5;
  string monitor_config_json = 6; // Specific config as a JSON string
  int32 timeout_seconds = 7;
}
```

---

## 4. Agent 端执行策略 (最佳实践)

-   **启动延迟 (Jitter):** 新任务启动时加入一个0到`frequency_seconds`的随机延迟，避免惊群效应。
-   **并发控制:** Agent内部使用`tokio::sync::Semaphore`限制同时执行的监控任务数量（可配置，默认如20），保护自身资源。
-   **首次立即执行:** 新任务或配置变更后立即执行一次，为用户提供即时反馈。

---

## 5. 前端 UI 与 UX

### 5.1. 任务创建/编辑弹窗UI草图

```mermaid
graph TD
    subgraph "创建/编辑监控任务 (Modal)"
        A["任务名称: [___________]"]
        B["监控类型: [HTTP ▼]"]
        C["监控目标: [https://___________]"]
        
        subgraph "基本设置"
            direction LR
            D["检查频率: [60] 秒"]
            E["超时时间: [10] 秒"]
        end

        subgraph "HTTP 详细配置 (动态显示)"
            F["期望状态码: [200, 201]"]
            G["响应内容匹配: [___________]"]
        end

        subgraph "任务分配 (Assignments)"
            direction LR
            J["<b>直接指定 Agent</b>"] --> K["[ VPS-01 [x] ]"];
            L["<b>按标签分配</b>"] --> M["[ <font color='green'>#production</font> [x] ]"];
        end

        N["[ 保存 ]"]

        A --> B --> C --> D & E
        C -- "选择HTTP" --> F & G
        D & E --> J & L --> N
    end
```

### 5.2. 核心UX逻辑

-   **动态表单:** 根据选择的`monitor_type`动态显示或隐藏特定于类型的配置区域。
-   **分配选择器:** 提供两个独立的多选框，分别用于“直接指定Agent”和“按标签分配”。
-   **状态显示:** 在UI上明确区分 **"Down"** (服务不可达) 和 **"No Data / Agent Offline"** (监控中断) 两种状态。

---

## 6. 后端API接口

-   **监控任务管理:**
    -   `POST /api/monitors` (创建)
    -   `GET /api/monitors` (列表)
    -   `GET /api/monitors/{id}` (详情)
    -   `PUT /api/monitors/{id}` (更新)
    -   `DELETE /api/monitors/{id}` (删除)
-   **监控结果查询:**
    -   `GET /api/monitors/{id}/results` (获取历史数据)