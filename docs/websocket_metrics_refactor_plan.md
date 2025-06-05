# 改造计划：实时指标 WebSocket 化 (v5 - 小驼峰JSON & 自动重连)

**目标：** 将现有的 HTTP 轮询方式获取服务器实时指标，改造成通过 WebSocket 从后端实时推送。后端在内存中维护一份包含所有服务器完整信息和最新指标的列表副本。一个独立的定时任务会定期（例如每2秒）从内存副本读取数据，将其序列化为 **JSON** 字符串 (字段为**小驼峰命名**)，并通过 WebSocket 推送给所有连接的客户端。当 Agent 上报数据时，更新数据库和内存副本。前端使用 Zustand 维护这份数据，供列表页和详情页使用，并且前端 WebSocket 客户端在连接断开时会**自动尝试重连**。

**技术选型确认：**

*   **WebSocket 数据格式：** **JSON** (字段统一为**小驼峰**命名)
*   **WebSocket 推送内容：** 包含所有服务器完整信息和最新指标的**完整列表**。
*   **WebSocket 认证：** **需要认证**，复用现有 HTTP API 的认证机制 (如 JWT Token)。
*   **前端状态管理：** **Zustand**。
*   **前端 WebSocket 重连：** **自动重连机制** (例如，带退避策略)。
*   **后端数据源：** **内存缓存** (`LiveServerDataCache`)。
*   **后端推送机制：** **独立的 Tokio 定时任务** + **`tokio::sync::broadcast` channel**。

---

## 一、后端改造 (Rust - Axum & Tonic)

### 1. 定义 WebSocket 消息的 Rust 结构体 (小驼峰JSON)
    *   在后端代码中（例如 `backend/src/websocket_models.rs`）定义结构体，并为它们派生 `serde::Serialize`。
    *   **关键**：在结构体或模块级别使用 `#[serde(rename_all = "camelCase")]` 来确保序列化为 JSON 时字段名为小驼峰。
        ```rust
        // backend/src/websocket_models.rs
        use serde::Serialize;
        use chrono::{DateTime, Utc};

        #[derive(Serialize, Clone, Debug)]
        #[serde(rename_all = "camelCase")] // 应用于所有字段
        pub struct ServerBasicInfo {
            pub id: i32,
            pub name: String,
            pub ip_address: Option<String>, // Rust: snake_case -> JSON: ipAddress
            pub status: String,
            // ...
        }

        #[derive(Serialize, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct ServerMetricsSnapshot {
            pub cpu_usage_percent: f32,    // Rust: snake_case -> JSON: cpuUsagePercent
            pub memory_usage_bytes: u64,   // Rust: snake_case -> JSON: memoryUsageBytes
            pub memory_total_bytes: u64,   // Rust: snake_case -> JSON: memoryTotalBytes
            pub network_rx_instant_bps: Option<u64>, // Rust: snake_case -> JSON: networkRxInstantBps
            pub network_tx_instant_bps: Option<u64>, // Rust: snake_case -> JSON: networkTxInstantBps
            pub uptime_seconds: Option<u64>, // Rust: snake_case -> JSON: uptimeSeconds
            // ...
        }

        #[derive(Serialize, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct ServerWithDetails {
            #[serde(flatten)]
            pub basic_info: ServerBasicInfo,
            pub latest_metrics: Option<ServerMetricsSnapshot>,
            pub os_type: Option<String>,      // Rust: snake_case -> JSON: osType
            pub created_at: DateTime<Utc>,  // Rust: snake_case -> JSON: createdAt
        }

        #[derive(Serialize, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct FullServerListPush {
            pub servers: Vec<ServerWithDetails>,
        }
        ```

### 2. 共享内存状态与 `AppState`
    *   **定义数据结构**: 在 `backend/src/server/agent_state.rs` (或 `live_data_cache.rs`) 中定义 `LiveServerDataCache = Arc<Mutex<HashMap<i32, ServerWithDetails>>>;` (key 为 `vps_id`, value 为上面定义的 `ServerWithDetails` 结构体)。
    *   **修改 `AppState`**: 在 `backend/src/http_server/mod.rs`
        *   添加 `live_server_data_cache: LiveServerDataCache`。
        *   添加 `ws_data_broadcaster_tx: broadcast::Sender<Arc<FullServerListPush>>`。
    *   **初始化**: 在 `main.rs` 中，从数据库加载数据填充 `live_server_data_cache`，并初始化 `ws_data_broadcaster_tx`。

### 3. 实现 WebSocket 服务 (`backend/src/http_server/websocket_handler.rs`)
    *   **WebSocket 处理器**:
        *   当需要推送数据时 (首次连接或通过 `broadcast::Receiver` 收到数据)：
            1.  获取 `Arc<FullServerListPush>`。
            2.  使用 `serde_json::to_string(&*full_server_list_push_arc)` 将其序列化为 JSON 字符串。
            3.  通过 `websocket.send(axum::extract::ws::Message::Text(json_string)).await` 发送。
    *   其余连接管理、认证、订阅逻辑与之前计划一致。

### 4. Agent 数据上报处理 (`backend/src/server/handlers.rs`)
    *   当 Agent 上报数据并更新数据库后，更新 `live_server_data_cache` 中的 `ServerWithDetails` 结构。确保从 `PerformanceSnapshot` (gRPC) 或数据库记录映射到 `ServerMetricsSnapshot` (JSON模型)。

### 5. 独立的定时推送任务
    *   在 `backend/src/bin/server.rs` (或一个专门的后台任务模块) 中，启动一个新的 Tokio 后台任务 (`tokio::spawn`)。
    *   这个任务将接收 `Arc<AppState>` (或至少是 `live_server_data_cache` 和 `ws_data_broadcaster_tx` 的克隆)。
    *   任务逻辑：
        ```rust
        // Pseudocode for the定时推送任务
        // async fn periodic_websocket_push_task(
        //     cache: LiveServerDataCache,
        //     broadcaster_tx: broadcast::Sender<Arc<FullServerListPush>>,
        //     push_interval_seconds: u64,
        // ) {
        //     let mut interval = tokio::time::interval(Duration::from_secs(push_interval_seconds));
        //     loop {
        //         interval.tick().await; // 等待下一个间隔点
        //
        //         let data_to_push = { // 创建一个作用域以尽快释放锁
        //             let cache_guard = cache.lock().await;
        //             // 从 cache_guard 构建 FullServerListPush
        //             // 例如: let list: Vec<ServerWithDetails> = cache_guard.values().cloned().collect();
        //             // Arc::new(FullServerListPush { servers: list })
        //         };
        //
        //         if broadcaster_tx.receiver_count() > 0 { // 仅当有订阅者时才发送
        //             if let Err(e) = broadcaster_tx.send(data_to_push) {
        //                 eprintln!("Failed to broadcast WebSocket data: {}", e);
        //             }
        //         }
        //     }
        // }
        ```
    *   从配置中读取推送间隔 (例如，默认为2秒)。

### 6. 数据获取服务 (`db/services.rs`)
    *   初始化 `live_server_data_cache` 时，确保从数据库查询的数据能正确映射到 `ServerWithDetails` 结构。

### 7. 整合到主服务 (`backend/src/bin/server.rs`)
    *   在 Axum 路由中添加 WebSocket 端点。
    *   确保 `AppState` 正确初始化。
    *   启动定时推送任务。

---

## 二、前端改造 (TypeScript - React & Zustand)

### 1. 创建 WebSocket 服务 (`frontend/src/services/websocketService.ts`) - 带自动重连
    *   **连接管理**:
        *   `connect(token: string)`: 尝试建立连接。
        *   `disconnect()`: 主动断开连接，并清除重连定时器。
    *   **自动重连逻辑**:
        *   维护重连尝试次数 `reconnectAttempts` 和最大尝试次数。
        *   使用指数退避策略 (exponential backoff) 来决定重连间隔，例如 `Math.min(30000, (2 ** reconnectAttempts) * 1000)` (最小1秒，最大30秒)。
        *   在 `onclose` 事件处理器中：
            *   如果不是主动断开 (检查一个标志位，该标志位在调用 `disconnect()` 时设置)，则启动重连逻辑。
            *   如果达到最大重连次数，则停止尝试并通过回调通知 Zustand store 连接彻底失败。
            *   否则，设置一个 `setTimeout` 来调用 `connect()`。
        *   在 `onopen` 事件处理器中：
            *   重置 `reconnectAttempts` 为 0。
            *   通知 Zustand store 连接成功。
    *   **消息处理**: `JSON.parse(event.data as string)`。
    *   **回调**: 通知 Zustand store 连接状态 (open, close, error, message, permanent_failure)。

### 2. 创建 Zustand Store (`frontend/src/store/serverListStore.ts`)
    *   **State**:
        *   `servers: VpsListItemResponse[] = []`
        *   `connectionStatus: 'disconnected' | 'connecting' | 'connected' | 'error' | 'reconnecting' | 'permanently_failed' = 'disconnected'`
        *   `isLoading: boolean = true`
        *   `error: string | null = null`
    *   **Actions**:
        *   `initializeWebSocket(): void`: 调用 `websocketService.connect()`。设置 `connectionStatus = 'connecting'`。
        *   `onWebSocketMessage(data: FullServerListPushType): void`: 更新 `servers`，`isLoading = false`。
        *   `onWebSocketOpen(): void`: 设置 `connectionStatus = 'connected'`, `error = null`。
        *   `onWebSocketClose(isIntentional: boolean): void`: 如果 `!isIntentional`，设置 `connectionStatus = 'reconnecting'`。如果 `isIntentional`，设置 `connectionStatus = 'disconnected'`。
        *   `onWebSocketError(errorMessage: string): void`: 设置 `connectionStatus = 'error'`, `error = errorMessage`。
        *   `onWebSocketPermanentFailure(): void`: 设置 `connectionStatus = 'permanently_failed'`, `error = "WebSocket connection failed after multiple retries."`。
        *   `disconnectWebSocket(): void`: 调用 `websocketService.disconnect()`。

### 3. 修改页面组件 (`App.tsx`, `HomePage.tsx`, `VpsDetailPage.tsx`)
    *   UI 根据 `serverListStore` 的 `connectionStatus` 和 `servers` 状态进行渲染。
    *   例如，可以显示 "正在连接...", "重新连接中...", "连接已断开，请刷新或检查网络。" 等提示。

### 4. 类型定义 (`frontend/src/types/index.ts`)
    *   确保前端类型字段**全部为小驼峰**，以匹配后端 JSON 输出。
        ```typescript
        // frontend/src/types/index.ts
        export interface VpsListItemResponse {
          id: number;
          name: string;
          ipAddress: string | null;
          status: string;
          latestMetrics: LatestPerformanceMetric | null;
          osType: string | null;
          createdAt: string; // ISO string
          // ... 其他字段，确保小驼峰
        }

        export interface LatestPerformanceMetric {
          cpuUsagePercent: number;
          memoryUsageBytes: number;
          memoryTotalBytes: number;
          networkRxInstantBps?: number | null;
          networkTxInstantBps?: number | null;
          uptimeSeconds?: number | null;
          // ... 其他指标，确保小驼峰
        }

        export interface FullServerListPushType {
          servers: VpsListItemResponse[];
        }
        ```

---

## 三、数据流图 (Mermaid)

```mermaid
sequenceDiagram
    participant Agent
    participant Backend_gRPC_Service
    participant Backend_DB_Service
    participant Backend_LiveServerData_Cache as "内存缓存 (Rust Structs with camelCase serde)"
    participant Backend_Periodic_Push_Task as "定时推送任务 (每2s)"
    participant Backend_WS_Data_Broadcaster as "WS数据广播器 (broadcast::Sender)"
    participant Backend_WebSocket_Handler
    participant Frontend_WebSocket_Service as "Frontend WS Service (with Auto-Reconnect)"
    participant Frontend_Zustand_Store
    participant Frontend_UI_Components

    Note over Backend_gRPC_Service, Backend_LiveServerData_Cache: 服务启动时初始化内存缓存
    Backend_gRPC_Service->>Backend_DB_Service: Get All VPS Info & Latest Metrics
    Backend_DB_Service-->>Backend_gRPC_Service: Return Initial Data
    Backend_gRPC_Service->>Backend_LiveServerData_Cache: Populate Cache (ServerWithDetails structs)

    Note over Backend_Periodic_Push_Task, Backend_WS_Data_Broadcaster: 服务启动时启动定时推送任务
    Backend_Periodic_Push_Task->>Backend_Periodic_Push_Task: Loop every 2s

    Note over Agent, Backend_gRPC_Service: Agent上报数据
    Agent->>+Backend_gRPC_Service: Send PerformanceBatch (gRPC)
    Backend_gRPC_Service->>Backend_DB_Service: Save PerformanceBatch
    Backend_DB_Service-->>Backend_gRPC_Service: Success
    Backend_gRPC_Service->>-Backend_LiveServerData_Cache: Update metrics in Cache

    Note over Frontend_UI_Components, Frontend_Zustand_Store: 用户登录/App启动
    Frontend_UI_Components->>Frontend_Zustand_Store: initializeWebSocket()
    Frontend_Zustand_Store->>Frontend_WebSocket_Service: connect(jwtToken)
    Frontend_WebSocket_Service->>+Backend_WebSocket_Handler: WebSocket Handshake (with JWT)
    Backend_WebSocket_Handler->>Backend_WebSocket_Handler: Authenticate JWT
    Note over Backend_WebSocket_Handler: 认证成功
    Backend_WebSocket_Handler->>Backend_LiveServerData_Cache: Read FullServerList from Cache
    Backend_LiveServerData_Cache-->>Backend_WebSocket_Handler: Return Cached Data (FullServerListPush struct)
    Backend_WebSocket_Handler->>Backend_WebSocket_Handler: Serialize to JSON string (camelCase)
    Backend_WebSocket_Handler->>Frontend_WebSocket_Service: Push Initial FullServerList (JSON String)
    Backend_WebSocket_Handler->>Backend_WS_Data_Broadcaster: Subscribe to broadcaster
    Backend_WebSocket_Handler-->>-Frontend_WebSocket_Service: WebSocket Connection Established
    Frontend_WebSocket_Service-->>Frontend_Zustand_Store: onWebSocketOpen()

    Note over Backend_Periodic_Push_Task, Frontend_WebSocket_Service: 定时推送逻辑
    Backend_Periodic_Push_Task->>Backend_LiveServerData_Cache: Read FullServerList from Cache
    Backend_LiveServerData_Cache-->>Backend_Periodic_Push_Task: Return Cached Data (FullServerListPush struct)
    Backend_Periodic_Push_Task->>Backend_WS_Data_Broadcaster: Send(Arc<FullServerListPush struct>)

    Backend_WS_Data_Broadcaster-->>Backend_WebSocket_Handler: Receives Arc<FullServerListPush struct>
    Backend_WebSocket_Handler->>Backend_WebSocket_Handler: Serialize to JSON string (camelCase)
    Backend_WebSocket_Handler->>Frontend_WebSocket_Service: Push FullServerList (JSON String)
    
    Frontend_WebSocket_Service->>Frontend_Zustand_Store: onWebSocketMessage(JSON.parse(data))
    Frontend_Zustand_Store->>Frontend_Zustand_Store: Update 'servers' state & 'connectionStatus'
    Frontend_Zustand_Store-->>Frontend_UI_Components: Notify UI of state change
    Frontend_UI_Components->>Frontend_UI_Components: Re-render with new server list & connection status

    Note over Frontend_WebSocket_Service: If connection drops
    Frontend_WebSocket_Service->>Frontend_WebSocket_Service: Attempt auto-reconnect (with backoff)
    Frontend_WebSocket_Service-->>Frontend_Zustand_Store: Update 'connectionStatus' (reconnecting, error, permanently_failed)