# 注册与登录接口开发计划

## 1. 目标

为 Web UI 添加用户注册和登录功能，通过 HTTP/JSON 接口与后端进行交互。

## 2. 技术选型

*   **后端语言**: Rust
*   **HTTP Web 框架**: `axum` (推荐，与 `tokio` 生态集成良好)
*   **密码哈希算法**: `bcrypt`
*   **会话管理**: JWT (JSON Web Tokens)，使用 `jsonwebtoken` 库
*   **数据库**: PostgreSQL (通过 `sqlx` 访问现有 `users` 表)
*   **序列化/反序列化**: `serde`, `serde_json`

## 3. 后端实现方案

### 3.1. 认证逻辑模块 (`backend/src/auth_logic.rs`)

创建一个新的 Rust 模块，专门负责处理认证的核心业务逻辑。

*   **主要功能**:
    *   用户注册：
        *   验证输入 (用户名、邮箱、密码格式)。
        *   检查用户名或邮箱是否已在数据库中存在。
        *   使用 `bcrypt` 对密码进行哈希处理。
        *   将新用户信息存入 `users` 表。
    *   用户登录：
        *   验证输入。
        *   根据邮箱或用户名从 `users` 表查询用户信息。
        *   使用 `bcrypt` 比较提供的密码和存储的哈希密码。
        *   如果密码匹配，生成 JWT。
*   **主要函数 (示例)**:
    ```rust
    // backend/src/auth_logic.rs
    use sqlx::PgPool;
    // ... other imports for bcrypt, jsonwebtoken, serde, custom error types

    pub struct RegisterRequest { /* ... fields ... */ }
    pub struct LoginRequest { /* ... fields ... */ }
    pub struct UserResponse { /* ... fields ... */ } // e.g., user_id, username, email
    pub struct LoginResponse { /* ... fields ... */ } // e.g., token, user_id, username

    pub async fn register_user(db_pool: &PgPool, req: RegisterRequest) -> Result<UserResponse, AuthError> {
        // ... implementation ...
    }

    pub async fn login_user(db_pool: &PgPool, req: LoginRequest) -> Result<LoginResponse, AuthError> {
        // ... implementation ...
    }
    ```
*   **错误处理**: 定义自定义错误类型 (`AuthError`) 以便在不同认证阶段返回具体错误信息。

### 3.2. HTTP 接口层 (使用 `axum`)

在主服务应用中 (例如 `backend/src/bin/server.rs`) 集成 `axum` 来处理 HTTP 请求。

*   **依赖项 (`Cargo.toml`)**:
    *   `axum`
    *   `tokio` (通常已存在)
    *   `serde`, `serde_json`
    *   `jsonwebtoken`
    *   `bcrypt`
    *   `sqlx` (通常已存在)
    *   `chrono` (用于 JWT 过期时间等)
*   **API 端点**:
    *   `POST /api/auth/register`: 处理用户注册请求。
    *   `POST /api/auth/login`: 处理用户登录请求。
*   **请求/响应格式**: JSON。
*   **处理流程**:
    1.  HTTP 处理函数接收 JSON 请求体。
    2.  使用 `serde` 将 JSON 反序列化为对应的 Rust 请求结构体。
    3.  调用 `auth_logic.rs` 中的相应业务逻辑函数。
    4.  将业务逻辑函数的返回结果 (成功响应或错误) 序列化为 JSON 并作为 HTTP 响应返回。

### 3.3. JWT (JSON Web Tokens)

*   **生成**: 登录成功后，生成 JWT。
    *   **声明 (Claims)**: 至少包含 `user_id` 和 `exp` (过期时间)。
    *   **密钥 (Secret Key)**: 从环境变量或安全配置文件中读取，**严禁硬编码**。
*   **验证**: (可选，用于受保护的路由) 创建 `axum` 中间件来验证传入请求 `Authorization` header 中的 JWT。

### 3.4. 数据库交互

*   使用现有的 `users` 表 (包含 `id`, `username`, `password_hash`, `email`, `created_at`, `updated_at`)。
*   通过 `sqlx` 与 PostgreSQL 数据库进行异步操作。

## 4. 安全考虑

*   **HTTPS**: 生产环境必须使用 HTTPS。
*   **密码存储**: 使用 `bcrypt` 哈希密码，确保为每个密码使用唯一的 salt。
*   **JWT 安全**:
    *   使用强壮的、保密的密钥。
    *   设置合理的过期时间。
    *   考虑 JWT 的吊销机制 (如果需要更高级别的安全性，但这会增加复杂性)。
*   **输入验证**: 对所有用户输入进行严格验证，防止 XSS、SQL 注入等攻击。
*   **速率限制**: 对登录和注册接口实施速率限制，以防止暴力破解和滥用。
*   **错误信息**: 避免在错误信息中泄露过多敏感信息 (例如，登录失败时不应明确指出是“用户名不存在”还是“密码错误”，统一返回“无效凭证”)。

## 5. 前端集成

*   Web UI 通过标准的 HTTP/JSON 请求调用后端 `/api/auth/register` 和 `/api/auth/login` 接口。
*   注册成功后，引导用户登录。
*   登录成功后，前端存储从后端获取的 JWT (例如存储在 `localStorage` 或 `HttpOnly` Cookie 中，后者更安全)。
*   对于需要认证的后续 API 请求，前端应在 HTTP 请求的 `Authorization` header 中携带 JWT (通常使用 `Bearer <token>` 方案)。
*   处理 JWT 过期和刷新逻辑 (如果实现了刷新令牌机制)。

## 6. 流程图

### 6.1. 用户注册流程

```mermaid
sequenceDiagram
    participant WebUI
    participant Backend_HTTP_Layer as HTTP API (Axum + auth_logic.rs)
    participant Database

    WebUI->>+Backend_HTTP_Layer: POST /api/auth/register (username, email, password)
    Backend_HTTP_Layer->>Database: Check if user exists (via auth_logic)
    alt User already exists
        Database-->>Backend_HTTP_Layer: User exists
        Backend_HTTP_Layer-->>-WebUI: HTTP 409 Conflict (or other appropriate error)
    else User does not exist
        Database-->>Backend_HTTP_Layer: User does not exist
        Backend_HTTP_Layer->>Backend_HTTP_Layer: Hash password (bcrypt, via auth_logic)
        Backend_HTTP_Layer->>Database: Store new user (via auth_logic)
        Database-->>Backend_HTTP_Layer: User created
        Backend_HTTP_Layer-->>-WebUI: HTTP 201 Created (user_id, username, email)
    end
```

### 6.2. 用户登录流程

```mermaid
sequenceDiagram
    participant WebUI
    participant Backend_HTTP_Layer as HTTP API (Axum + auth_logic.rs)
    participant Database

    WebUI->>+Backend_HTTP_Layer: POST /api/auth/login (email/username, password)
    Backend_HTTP_Layer->>Database: Fetch user by email/username (via auth_logic)
    alt User not found
        Database-->>Backend_HTTP_Layer: User not found
        Backend_HTTP_Layer-->>-WebUI: HTTP 401 Unauthorized (Invalid credentials)
    else User found
        Database-->>Backend_HTTP_Layer: User data (hashed_password)
        Backend_HTTP_Layer->>Backend_HTTP_Layer: Verify password (bcrypt compare, via auth_logic)
        alt Password incorrect
            Backend_HTTP_Layer-->>-WebUI: HTTP 401 Unauthorized (Invalid credentials)
        else Password correct
            Backend_HTTP_Layer->>Backend_HTTP_Layer: Generate JWT (user_id, exp, via auth_logic)
            Backend_HTTP_Layer-->>-WebUI: HTTP 200 OK (token, user_id, username)
        end
    end
```

## 7. 任务分解 (初步)

1.  **环境搭建与依赖添加**:
    *   在 `backend/Cargo.toml` 中添加 `axum`, `jsonwebtoken`, `bcrypt`, `serde`, `serde_json`, `thiserror` (用于自定义错误) 等依赖。
2.  **定义数据结构与错误类型**:
    *   在 `auth_logic.rs` 或共享模块中定义请求/响应结构体 (`RegisterRequest`, `LoginRequest`, `UserResponse`, `LoginResponse` 等)。
    *   定义 `AuthError` 枚举。
3.  **实现 `auth_logic.rs`**:
    *   实现 `register_user` 函数逻辑。
    *   实现 `login_user` 函数逻辑。
    *   编写单元测试。
4.  **实现 HTTP 接口层 (`server.rs`)**:
    *   配置 `axum` 路由。
    *   实现注册和登录的 HTTP 处理函数。
    *   集成 `auth_logic`。
    *   配置 JWT 密钥和相关参数。
5.  **测试**:
    *   编写集成测试，覆盖注册和登录的各种场景。
    *   使用 Postman 或类似工具进行手动接口测试。
6.  **文档**: (可选) 更新 API 文档。