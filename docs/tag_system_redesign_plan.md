# 新标签系统设计与实施计划 (最终版 V2)

本计划旨在重新设计和实现VPS标签系统，以支持更丰富的功能，并优化用户体验。

---

### 1. 数据库设计

数据库结构将进行重构，以支持灵活的、可扩展的标签功能。

-   **`tags` 表**: 存储标签的详细定义。
-   **`vps_tags` 表**: 存储 VPS 和标签之间的多对多关联关系。
-   **`vps` 表**: 移除旧的、基于文本的 `tags` 字段。

**数据库关系图 (ERD):**
```mermaid
erDiagram
    users {
        int id PK
        string username
    }

    vps {
        int id PK
        int user_id FK
        string name
    }

    tags {
        int id PK
        int user_id FK
        string "name"
        string "color"
        string "icon"
        string "url"
    }

    vps_tags {
        int vps_id PK, FK
        int tag_id PK, FK
    }

    users ||--o{ vps : "拥有"
    users ||--o{ tags : "创建"
    vps ||--|{ vps_tags : "拥有"
    tags ||--|{ vps_tags : "关联"
```

---

### 2. 后端实现 (Rust)

后端主要负责数据持久化、业务逻辑处理和API接口。

-   **数据库迁移**: 创建新的SQL迁移文件以应用上述数据库结构变更。
-   **数据模型**: 在 `backend/src/db/models.rs` 中创建新的 `Tag` 和 `VpsTag` 结构体。
-   **API接口**:
    -   `GET /api/tags`: 获取当前用户的所有标签，并在返回数据中包含 `vps_count` 字段，用于**标签使用统计**。
    -   `POST /api/tags`: 创建一个新标签。
    -   `PUT /api/tags/:id`: 更新指定ID的标签。
    -   `DELETE /api/tags/:id`: 删除指定ID的标签。
    -   `POST /api/vps/:vps_id/tags`: 为单个VPS添加标签。
    -   `DELETE /api/vps/:vps_id/tags/:tag_id`: 移除单个VPS的标签。
    -   `POST /api/vps/bulk-actions`: 为多个VPS**批量添加/移除**标签。
-   **WebSocket服务**: 保持现有逻辑，当VPS信息（包括其关联的标签）发生变化时，通过WebSocket实时推送更新后的完整数据给前端。

---

### 3. 前端实现 (React)

前端负责UI展示、用户交互和本地状态管理。

-   **状态管理 (`serverListStore.ts`)**:
    -   负责接收并管理通过WebSocket推送的完整的、实时的VPS列表数据。
-   **标签管理页面 (`/tags`)**:
    -   提供完整的标签CRUD（创建、读取、更新、删除）功能。
    -   在标签列表中展示每个标签的**使用统计**。
    -   UI组件包括表单、颜色选择器、Lucide Icons图标选择器等。
-   **VPS列表页面 (`/`)**:
    -   **数据来源**: 页面直接从 `serverListStore` 获取并展示VPS列表，实现实时更新。
    -   **标签过滤与搜索 (前端实现)**:
        -   提供标签选择器。
        -   过滤操作在前端本地进行，直接操作 `serverListStore` 中的数据，无需发送API请求，实现瞬时响应。
    -   **批量操作**:
        -   提供复选框以选择多个VPS。
        -   通过“批量操作”弹窗，调用 `POST /api/vps/bulk-actions` 接口完成操作。
-   **组件更新**:
    -   更新 `VpsCard.tsx`, `VpsDetailPage.tsx` 等组件，以展示带有颜色、图标、可点击链接的新标签样式。

---

### 4. 开发流程概览

```mermaid
graph TD
    subgraph "用户操作"
        A["登录"] --> B["访问 /tags 页面管理标签"];
        A --> C["访问VPS列表页"];
        C --> D["使用标签选择器进行过滤"];
        C --> E["勾选多个VPS进行批量操作"];
    end

    subgraph "前端 (React)"
        subgraph "WebSocket"
            WS["WebSocket连接"] --> F["实时更新 serverListStore"];
        end
        subgraph "REST API"
             B -- "API请求" --> G["/api/tags (CRUD & 统计)"];
             E -- "API请求" --> H["/api/vps/bulk-actions"];
        end
        subgraph "本地操作"
            C --> I["从 serverListStore 读取数据并展示"];
            D --> J["在本地过滤 serverListStore 的数据"];
        end
    end

    subgraph "后端 (Rust)"
        G --> K["数据库: tags 表"];
        H --> L["数据库: vps_tags 表"];
        M["数据变更"] --> WS_Push["通过WebSocket推送更新"];
        K --> M;
        L --> M;
    end