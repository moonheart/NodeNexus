#!/bin/bash
# release.sh

# 当任何命令失败时，立即退出脚本
set -e

# --- 步骤 1: 构建前端 ---
echo "Building frontend application..."
cd frontend
# 安装依赖 (如果需要)
npm install
# 执行构建
npm run build
cd ..
echo "Frontend build complete. Assets are in frontend/dist/"

# --- 步骤 2: 构建后端 ---
echo "Building backend application..."
cd backend
# 执行 release 构建，这将会把前端文件嵌入
cargo build --release
cd ..
echo "Backend build complete."

# --- 步骤 3: 整理发布文件 ---
echo "Copying executable to release folder..."
# 创建 release 目录 (如果不存在)
mkdir -p release
# 复制可执行文件
# 注意: 在Windows上，可执行文件是 .exe 后缀
cp backend/target/release/server.exe release/mjjer-server.exe

echo "----------------------------------------"
echo "Release process finished successfully!"
echo "Your application is ready at: release/mjjer-server.exe"
echo "----------------------------------------"