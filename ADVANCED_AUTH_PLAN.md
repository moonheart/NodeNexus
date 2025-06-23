# 项目计划：高级动态认证系统

## 1. 核心目标

构建一个灵活、安全且可扩展的认证系统，该系统：
- 支持标准的密码认证。
- 允许管理员通过 UI 动态配置和管理多个 OAuth 2.0 提供商。
- 支持通过配置好的提供商（如 GitHub）进行第三方登录。
- 允许用户禁用其密码登录，以增强安全性。
- 提供一个安全的管理员后门（命令行工具），用于在服务器上恢复用户的密码登录。
- 自动将系统中的第一个注册用户指定为管理员。

## 2. 数据库设计

### a. `users` 表

扩展 `users` 表以支持新功能。

```mermaid
erDiagram
    users {
        int id "PK"
        string username "UNIQUE"
        string password_hash "NULLABLE"
        string email "UNIQUE"
        string role "DEFAULT 'user'"
        bool password_login_disabled "DEFAULT false"
        datetime created_at
        datetime updated_at
    }
```

- **`password_hash`**: 设为 `NULLABLE`，以支持仅使用第三方登录的用户。
- **`role`**: 新增字段，用于权限控制（`user`, `admin`）。
- **`password_login_disabled`**: 新增布尔字段，控制密码登录的可用性。

### b. `user_identity_providers` 表

存储用户与第三方提供商的关联关系。

```mermaid
erDiagram
    users ||--o{ user_identity_providers : "has"

    user_identity_providers {
        int id "PK"
        int user_id "FK to users.id"
        string provider_name "e.g., 'github'"
        string provider_user_id "User ID from the provider"
        datetime created_at
        datetime updated_at
    }
```

### c. `oauth2_providers` 表

存储管理员配置的 OAuth 2.0 提供商信息，这是实现动态配置的核心。

```mermaid
erDiagram
    oauth2_providers {
        int id "PK"
        string provider_name "UNIQUE, e.g., 'github', 'google'"
        string client_id
        string client_secret "ENCRYPTED"
        string auth_url
        string token_url
        string user_info_url
        string scopes
        json user_info_mapping
        bool enabled "DEFAULT true"
        datetime created_at
        datetime updated_at
    }
```

- **`client_secret`**: **必须加密存储**，以防止数据库泄露风险。
- **`user_info_mapping`**: JSON 字段，用于动态映射不同提供商返回的用户信息字段，增强了系统的灵活性。

## 3. 后端架构

### a. 动态 OAuth 2.0 登录流程

```mermaid
sequenceDiagram
    participant User as "用户"
    participant Frontend as "前端应用"
    participant Backend as "后端服务"
    participant OAuthProvider as "OAuth 提供商"

    User->>Frontend: 点击 "使用 {Provider} 登录"
    Frontend->>Backend: 请求授权 URL (/api/auth/{provider}/login)
    Backend->>Backend: 从 `oauth2_providers` 表加载 {provider} 配置
    Backend->>Backend: 生成并暂存 `state` 参数以防 CSRF
    Backend->>Frontend: 返回基于配置构建的重定向 URL (包含 `state`)
    
    User->>GitHub: 同意授权
    GitHub-->>Frontend: 重定向回前端 (附带 `code` 和 `state`)
    Frontend->>Backend: 发送 `code` 和 `state` 到回调 URL (/api/auth/{provider}/callback)
    
    Backend->>Backend: 验证 `state` 参数的有效性
    Backend->>OAuthProvider: 使用解密后的 `client_secret` 交换 Token
    OAuthProvider-->>Backend: 返回 Access Token
    Backend->>OAuthProvider: 根据 `user_info_url` 获取用户信息
    OAuthProvider-->>Backend: 返回用户信息
    
    Backend->>Backend: 根据 `user_info_mapping` 解析用户信息
    
    alt 已存在绑定关系 (provider_name + provider_user_id)
        Backend->>Backend: 登录成功，生成 JWT
        Backend-->>Frontend: 返回 JWT 和用户信息
    else 无绑定关系
        alt 邮箱已在 `users` 表中存在
            Backend-->>Frontend: 返回错误: "该邮箱已存在，无法登录"
        else 邮箱不存在 (新用户)
            Backend->>Backend: 在 `users` 表创建新用户 (无密码)
            Backend->>Backend: 在 `user_identity_providers` 表创建记录
            Backend->>Backend: 登录成功，生成 JWT
            Backend-->>Frontend: 返回 JWT 和用户信息
        end
    end
```

### b. API 端点

| 方法   | 路径                               | 描述                                                               | 权限     |
| :----- | :--------------------------------- | :----------------------------------------------------------------- | :------- |
| `GET`  | `/api/auth/providers`              | 获取所有已启用的 OAuth 提供商列表。                                | 公开     |
| `GET`  | `/api/auth/{provider}/login`       | 获取指定提供商的授权 URL。                                         | 公开     |
| `GET`  | `/api/auth/{provider}/callback`    | 处理来自任何提供商的回调。                                         | 公开     |
| `POST` | `/api/settings/disable-password`   | 禁用当前用户的密码登录。                                           | 用户认证 |
| `POST` | `/api/settings/enable-password`    | 启用当前用户的密码登录。                                           | 用户认证 |
| `GET`  | `/api/settings/auth-providers`     | 获取用户当前关联的第三方登录提供商。                               | 用户认证 |
| `DELETE`| `/api/settings/auth-providers/{provider}` | 解绑一个第三方提供商（需进行安全检查）。                       | 用户认证 |
| `GET`  | `/api/admin/oauth-providers`       | 获取所有 OAuth 提供商的完整配置列表。                              | 管理员   |
| `POST` | `/api/admin/oauth-providers`       | 新增一个 OAuth 提供商配置。                                        | 管理员   |
| `PUT`  | `/api/admin/oauth-providers/{id}`  | 更新一个 OAuth 提供商配置。                                        | 管理员   |
| `DELETE`| `/api/admin/oauth-providers/{id}` | 删除一个 OAuth 提供商配置。                                        | 管理员   |

### c. 命令行工具

为 `server` 二进制文件添加子命令，用于紧急管理。

```bash
# 强制为指定用户启用密码登录
./server user --enable-password-login <user_email_or_username>
```

## 4. 前端界面

1.  **登录页面:**
    *   动态调用 `/api/auth/providers` 并渲染登录按钮。
2.  **用户设置页面 (`/settings`):**
    *   **安全设置:** 管理密码登录状态。
    *   **关联账户:** 管理绑定的第三方提供商，解绑时进行安全检查（防止孤儿账户）。
3.  **管理员设置页面 (新增):**
    *   提供完整的 CRUD 界面，用于管理 `oauth2_providers`。
    *   表单需要包含所有必要字段，特别是 `user_info_mapping` 的友好输入方式（例如，一个简单的 JSON 编辑器）。

## 5. 实施步骤

1.  **数据库迁移:** 创建迁移文件以应用上述所有数据库变更。
2.  **配置:** 在 `.env` 文件中增加 `APP_ENCRYPTION_KEY` 用于加密 `client_secret`。
3.  **后端实现:**
    *   更新用户模型和服务。
    *   实现通用的、安全的 OAuth 核心逻辑（处理 `state`、加解密、动态映射）。
    *   实现所有新的 API 端点，并应用正确的权限控制。
    *   使用 `clap` 或类似库实现命令行工具。
4.  **前端实现:**
    *   实现动态登录页面。
    *   实现用户设置页面中的新功能。
    *   构建全新的管理员 OAuth 配置页面。
5.  **测试:** 进行全面的端到端和集成测试，覆盖所有安全和业务逻辑。