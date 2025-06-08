# VPS 流量监控功能计划

**核心目标：** 为每个 VPS 实现可配置的月度或自定义周期流量监控，包括限额、重置规则、计费规则，并在前端展示使用情况及提供告警功能。

## 第一阶段：后端数据模型与核心逻辑

1.  **数据库模型更新 ([`backend/src/db/models.rs`](backend/src/db/models.rs)) - 在 `Vps` 表中添加以下字段**:
    *   `traffic_limit_bytes` (BIGINT, nullable): 流量限额，字节单位。
    *   `traffic_billing_rule` (VARCHAR(20), nullable): 计费规则 (e.g., `"sum_in_out"`, `"out_only"`, `"max_in_out"`).
    *   `traffic_current_cycle_rx_bytes` (BIGINT, NOT NULL, DEFAULT 0): 本周期已用RX。
    *   `traffic_current_cycle_tx_bytes` (BIGINT, NOT NULL, DEFAULT 0): 本周期已用TX。
    *   `last_processed_cumulative_rx` (BIGINT, NOT NULL, DEFAULT 0): 上次处理的累计RX。
    *   `last_processed_cumulative_tx` (BIGINT, NOT NULL, DEFAULT 0): 上次处理的累计TX。
    *   `traffic_last_reset_at` (TIMESTAMPTZ, nullable): 上次实际重置时间。
    *   `traffic_reset_config_type` (VARCHAR(50), nullable): 重置周期类型。
        *   初步支持: `"monthly_day_of_month"` (每月指定日和时间), `"fixed_days"` (每隔N天)。
    *   `traffic_reset_config_value` (VARCHAR(100), nullable): 配合 `config_type` 的值。
        *   若 `type` = `"monthly_day_of_month"`: `value` 存储如 `"day:15,time_offset_seconds:28800"` (每月15号，UTC时间当天偏移28800秒即08:00:00)。
        *   若 `type` = `"fixed_days"`: `value` 存储如 `"30"` (表示每30天)。
    *   `next_traffic_reset_at` (TIMESTAMPTZ, nullable): 下一个精确的重置时间点 (UTC)。

2.  **流量数据处理与计算逻辑 ([`backend/src/agent_modules/metrics.rs`](backend/src/agent_modules/metrics.rs) 或新模块)**:
    *   处理Agent上报的累计流量，计算增量，并更新 `traffic_current_cycle_rx/tx_bytes` 和 `last_processed_cumulative_rx/tx`。
    *   包含VPS重启导致计数器归零的处理逻辑 (检测到上报值变小则将当前值视为增量)。

3.  **流量重置逻辑 (后台定时任务)**:
    *   定时任务检查 `next_traffic_reset_at IS NOT NULL AND next_traffic_reset_at <= NOW()`。
    *   **执行重置**:
        1.  清零周期流量。
        2.  更新 `traffic_last_reset_at = vps.next_traffic_reset_at`。
        3.  更新 `last_processed_cumulative_rx/tx`。
        4.  **计算并更新新的 `next_traffic_reset_at`**: 根据 `traffic_reset_config_type` 和 `traffic_reset_config_value` 从当前的 `next_traffic_reset_at` 计算下一个重置点。
            *   `"monthly_day_of_month"`: 计算到下个月的指定日期和时间偏移。需处理月份天数不同的情况（例如，若指定31日，但下个月只有30日，则取月末）。
            *   `"fixed_days"`: 当前 `next_traffic_reset_at` 加上指定的天数。

4.  **API 接口 ([`backend/src/http_server/vps_routes.rs`](backend/src/http_server/vps_routes.rs) 和 [`backend/src/db/services/vps_service.rs`](backend/src/db/services/vps_service.rs))**:
    *   **更新/创建 VPS 配置**: 允许传入和保存所有流量配置字段。当用户配置或修改周期规则时，后端API负责计算并更新 `next_traffic_reset_at`。
    *   **获取 VPS 流量详情**: 返回包括所有配置、当前周期用量、计算后的总用量、剩余量、百分比以及 `next_traffic_reset_at`。

5.  **流量告警逻辑 ([`backend/src/alerting/evaluation_service.rs`](backend/src/alerting/evaluation_service.rs))**:
    *   `metric_type` 增加 `"traffic_usage_percent"`。
    *   评估服务根据计算出的流量使用百分比进行告警判断。

## 第二阶段：前端用户界面

1.  **VPS 配置 ([`frontend/src/components/CreateVpsModal.tsx`](frontend/src/components/CreateVpsModal.tsx), [`frontend/src/components/EditVpsModal.tsx`](frontend/src/components/EditVpsModal.tsx))**:
    *   流量限额 (数字 + 单位)。
    *   计费规则 (下拉框)。
    *   **重置规则**:
        *   周期类型选择 (下拉框: "每月指定日期", "每隔 N 天")。
        *   根据类型显示不同输入：
            *   "每月指定日期": 日期选择 (1-31)，时间选择 (时:分)。
            *   "每隔 N 天": 天数输入。
        *   **首次/下一个重置时间**: 日期时间选择器，允许用户指定或前端根据周期规则智能推荐。

2.  **VPS 详情页展示 ([`frontend/src/pages/VpsDetailPage.tsx`](frontend/src/pages/VpsDetailPage.tsx))**:
    *   展示总限额、已用/剩余流量、百分比（带进度条）、计费规则、本周期IN/OUT、下次重置时间 (`next_traffic_reset_at`)。

## 第三阶段：数据库迁移

1.  创建 SQL 迁移文件，使用 `ALTER TABLE vps ADD COLUMN ...` 添加上述新字段。

## Mermaid 流程图 (数据流与计算)

```mermaid
graph LR
    subgraph Agent
        A1["OS Network Counters"] --> A2{Agent Collector}
        A2 -- "Cumulative RX/TX Bytes" --> A3["PerformanceSnapshot Proto"]
    end

    subgraph Backend
        A3 --> B1["gRPC/HTTP Ingest"];
        B1 --> B2["Save to PerformanceMetrics Table"];

        subgraph "Traffic Calculation & Update"
            B2 -- "Trigger (on new metric)" --> C1{Get VPS Traffic Config};
            C1 --> C2["Get last_processed_cumulative_rx/tx"];
            B2 -- "Current cumulative_rx/tx" --> C2;
            C2 --> C3["Calculate delta_rx/tx (Handle counter reset)"];
            C3 --> C4["Update vps.traffic_current_cycle_rx/tx_bytes"];
            C3 --> C5["Update vps.last_processed_cumulative_rx/tx"];
        end

        subgraph "Traffic Reset (Scheduled Task)"
            D1{For each VPS with config} --> D2{"Is next_traffic_reset_at <= NOW?"};
            D2 -- Yes --> D3["Reset cycle data & update last_reset_at"];
            D3 --> D4["Calculate NEW next_traffic_reset_at based on config_type & config_value"];
            D4 --> D5["Update vps.next_traffic_reset_at"];
        end

        subgraph "API for Frontend"
            E1["Frontend Request for VPS Detail"] --> E2{API Handler};
            E2 --> E3["Read vps table (traffic config, cycle usage, next_reset_at)"];
            E3 --> E4["Calculate: used_traffic, remaining, percentage"];
            E4 --> E5["Return Enhanced VPS Details to Frontend"];
        end

        subgraph Alerting
            C4 -- "Updated cycle usage" --> F1{Alert Evaluation Service};
            F1 --> F2["Compare usage_percentage with AlertRule"];
            F2 -- Breach --> F3["Trigger Notification"];
        end
    end

    subgraph Frontend
        G1["User views VPS Detail Page"] --> G2["Display Traffic Info from E5"];
        G3["User edits VPS"] --> G4["Set Traffic Limit, Billing Rule, Reset Config (type, value, initial next_reset_at)"];
        G4 --> E2;
    end