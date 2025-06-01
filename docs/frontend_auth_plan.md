# 前端登录与注册界面开发计划

## 1. 项目准备与依赖安装

*   **安装 `react-router-dom`**:
    *   由于前端项目目前没有路由管理库，我们将首先安装 `react-router-dom` 用于页面导航。
    *   命令: `npm install react-router-dom` (或 `yarn add react-router-dom`)

## 2. 目录与文件结构 (在 `frontend/src` 下)

```
frontend/src/
├── App.tsx                 # 主应用组件，配置路由
├── main.tsx                # 应用入口
├── index.css               # 全局样式
├── vite-env.d.ts
├── assets/
├── components/             # 可复用UI组件
│   ├── AuthForm.tsx        # (可选) 登录/注册表单的通用框架
│   └── ProtectedRoute.tsx  # (可选) 用于保护需要登录才能访问的路由
├── pages/                  # 页面级组件
│   ├── LoginPage.tsx       # 登录页面
│   ├── RegisterPage.tsx    # 注册页面
│   └── HomePage.tsx        # 应用主页 (登录后跳转)
├── services/               # API请求相关
│   └── authService.ts      # 封装登录和注册的API调用
├── store/                  # Zustand状态管理
│   └── authStore.ts        # 认证相关的状态和操作
└── utils/                  # 工具函数 (例如，表单验证)
    └── validators.ts       # (可选) 表单验证逻辑
```

## 3. 路由配置 (`frontend/src/App.tsx`)

*   使用 `react-router-dom` 设置以下路由：
    *   `/login`: 指向 `LoginPage` 组件。
    *   `/register`: 指向 `RegisterPage` 组件。
    *   `/`: 指向 `HomePage` 组件 (或其他应用主页)。
    *   (可选) 未匹配路由可以导向一个 `NotFoundPage`。

## 4. 状态管理 (`frontend/src/store/authStore.ts`)

*   使用 Zustand 创建一个 `authStore` 来管理认证状态：
    *   **State**:
        *   `isAuthenticated: boolean` (用户是否已登录)
        *   `user: UserResponse | null` (当前用户信息)
        *   `token: string | null` (JWT)
        *   `isLoading: boolean` (是否正在进行API请求)
        *   `error: string | null` (API请求错误信息)
    *   **Actions**:
        *   `login(credentials: LoginRequest): Promise<void>`
        *   `register(userData: RegisterRequest): Promise<void>`
        *   `logout(): void`
        *   `setToken(token: string | null): void`
        *   `setUser(user: UserResponse | null): void`
        *   `clearAuth(): void` (用于登出或清除状态)

## 5. UI 组件实现 (使用 Material UI)

*   **`RegisterPage.tsx`**:
    *   包含一个表单，输入字段：用户名 (`username`)、邮箱 (`email`)、密码 (`password`)。
    *   使用 Material UI 的 `TextField`、`Button`、`Container`、`Typography` 等组件。
    *   表单提交时调用 `authStore` 中的 `register` action。
    *   根据 `authStore` 中的 `isLoading` 和 `error` 状态显示加载指示和内联错误信息。
    *   成功注册后，使用 `react-router-dom` 的 `useNavigate` 跳转到 `/login` 页面。
*   **`LoginPage.tsx`**:
    *   包含一个表单，输入字段：邮箱 (`email`)、密码 (`password`)。
    *   使用 Material UI 组件。
    *   表单提交时调用 `authStore` 中的 `login` action。
    *   根据 `authStore` 中的 `isLoading` 和 `error` 状态显示加载指示和内联错误信息。
    *   成功登录后，将 JWT 存储到 `localStorage` (或 `sessionStorage`)，并使用 `useNavigate` 跳转到 `/` (首页)。
*   **`HomePage.tsx`**:
    *   简单的占位页面，显示欢迎信息或应用核心内容。
    *   应为受保护路由，如果用户未登录尝试访问，应重定向到 `/login`。
*   **(可选) `AuthForm.tsx`**:
    *   如果登录和注册表单结构相似，可以提取一个通用表单组件。
*   **(可选) `ProtectedRoute.tsx`**:
    *   一个高阶组件或自定义路由组件，用于检查 `authStore` 中的 `isAuthenticated` 状态。如果未登录，则重定向到 `/login`。

## 6. API 服务 (`frontend/src/services/authService.ts`)

*   创建函数与后端 API 交互：
    *   `registerUser(data: RegisterRequest): Promise<UserResponse>`
        *   向 `POST /api/auth/register` 发送请求。
        *   处理响应和错误。
    *   `loginUser(data: LoginRequest): Promise<LoginResponse>`
        *   向 `POST /api/auth/login` 发送请求。
        *   处理响应和错误。

## 7. 错误处理与表单验证

*   **前端表单验证**: 在用户提交表单前进行基本的客户端验证 (例如，字段是否为空，邮箱格式是否正确，密码长度等)。可以使用简单的逻辑或引入如 `yup`、`zod` 等库。错误信息以内联形式显示在对应输入框下方。
*   **API 错误处理**:
    *   捕获 `authService.ts` 中 API 调用返回的错误。
    *   将错误信息更新到 `authStore` 的 `error` 状态。
    *   在 `LoginPage` 和 `RegisterPage` 中根据 `error` 状态显示相应的错误提示 (例如，“用户名已存在”，“无效的凭证”等)。

## 8. 导航与重定向逻辑

*   **注册成功**: `RegisterPage` -> `/login`
*   **登录成功**: `LoginPage` -> `/` (HomePage)
*   **已登录用户访问 `/login` 或 `/register`**: 重定向到 `/` (HomePage)
*   **未登录用户访问受保护路由 (如 `/`)**: 重定向到 `/login`

## 9. JWT 处理

*   **存储**: 登录成功后，从 `LoginResponse` 中获取 `token`，并将其存储在 `localStorage`。同时更新 `authStore`。
*   **发送**: 对于需要认证的 API 请求 (后续功能)，从 `localStorage` 读取 `token` 并将其包含在请求的 `Authorization` header 中 (例如 `Bearer <token>`)。
*   **初始化**: 应用加载时，检查 `localStorage` 中是否存在 `token`。如果存在，可以尝试验证 `token` (例如，通过一个 `/api/auth/me` 或 `/api/auth/verify-token` 的后端接口，如果后端提供的话) 并恢复用户登录状态到 `authStore`。如果后端没有提供此类接口，可以简单地将 `token` 和用户信息（如果也存储了）加载到 `authStore`，并假设 `token` 有效，直到下一次 API 调用失败因 `token` 无效。

## 10. 流程图

### a. 整体导航流程

```mermaid
graph TD
    A[未登录用户] -->|访问 /login| B(登录页面 LoginPage);
    A -->|访问 /register| C(注册页面 RegisterPage);
    A -->|访问 / (首页)| B;
    B -->|登录成功| D(首页 HomePage);
    C -->|注册成功| B;
    D -->|登出| B;
    E[已登录用户] -->|访问 /login 或 /register| D;
    E -->|访问 / (首页)| D;
```

### b. 用户注册流程 (前端)

```mermaid
sequenceDiagram
    participant User
    participant RegisterPage as 注册页面 (UI)
    participant authStore as Zustand Store
    participant authService as API Service
    participant BackendAPI as /api/auth/register

    User->>+RegisterPage: 输入用户名、邮箱、密码
    User->>RegisterPage: 点击注册按钮
    RegisterPage->>+authStore: 调用 register(userData) action
    authStore->>+authService: 调用 registerUser(userData)
    authService->>+BackendAPI: POST {username, email, password}
    BackendAPI-->>-authService: 返回成功/失败响应
    alt 注册成功
        authService-->>-authStore: 返回 UserResponse
        authStore->>authStore: 更新 state (isLoading: false, error: null)
        authStore-->>-RegisterPage: 注册成功
        RegisterPage->>RegisterPage: 跳转到 /login
    else 注册失败 (例如，用户已存在)
        authService-->>-authStore: 返回错误信息
        authStore->>authStore: 更新 state (isLoading: false, error: "错误信息")
        authStore-->>-RegisterPage: 注册失败
        RegisterPage->>RegisterPage: 显示内联错误信息
    end
```

### c. 用户登录流程 (前端)

```mermaid
sequenceDiagram
    participant User
    participant LoginPage as 登录页面 (UI)
    participant authStore as Zustand Store
    participant authService as API Service
    participant BackendAPI as /api/auth/login

    User->>+LoginPage: 输入邮箱、密码
    User->>LoginPage: 点击登录按钮
    LoginPage->>+authStore: 调用 login(credentials) action
    authStore->>+authService: 调用 loginUser(credentials)
    authService->>+BackendAPI: POST {email, password}
    BackendAPI-->>-authService: 返回成功/失败响应
    alt 登录成功
        authService-->>-authStore: 返回 LoginResponse (token, user)
        authStore->>authStore: 存储 token (localStorage), 更新 state (isAuthenticated: true, user, token, isLoading: false, error: null)
        authStore-->>-LoginPage: 登录成功
        LoginPage->>LoginPage: 跳转到 / (HomePage)
    else 登录失败 (例如，无效凭证)
        authService-->>-authStore: 返回错误信息
        authStore->>authStore: 更新 state (isLoading: false, error: "错误信息")
        authStore-->>-LoginPage: 登录失败
        LoginPage->>LoginPage: 显示内联错误信息
    end