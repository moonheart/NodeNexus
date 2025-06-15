# release.ps1

# 当任何命令遇到错误时，立即停止执行
$ErrorActionPreference = "Stop"

# --- 步骤 1: 构建前端 ---
Write-Host "Building frontend application..."
Push-Location -Path "frontend"
# 安装依赖 (如果需要)
npm install
# 执行构建
npm run build
Pop-Location
Write-Host "Frontend build complete. Assets are in frontend/dist/"

# --- 步骤 2: 构建后端 ---
Write-Host "Building backend application..."
Push-Location -Path "backend"
# 执行 release 构建，这将会把前端文件嵌入
cargo build --bin server --release
Pop-Location
Write-Host "Backend build complete."

# --- 步骤 3: 整理发布文件 ---
Write-Host "Copying executable to release folder..."
# 创建 release 目录 (如果不存在)
if (-not (Test-Path -Path "release")) {
    New-Item -ItemType Directory -Path "release" | Out-Null
}
# 复制可执行文件
Copy-Item -Path "backend/target/release/server.exe" -Destination "release/mjjer-server.exe" -Force

Write-Host "----------------------------------------"
Write-Host "Release process finished successfully!"
Write-Host "Your application is ready at: release/mjjer-server.exe"
Write-Host "----------------------------------------"