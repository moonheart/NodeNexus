#!/bin/bash
# build-docker.sh

# 当任何命令失败时，立即退出脚本
set -e

# --- 定义变量 ---
IMAGE_NAME="mjjer-server"
TAG="latest"
DOCKERFILE_PATH="backend/Dockerfile"

# --- 步骤 1: 构建 Docker 镜像 ---
echo "Building Docker image: $IMAGE_NAME:$TAG..."
docker build -t "$IMAGE_NAME:$TAG" -f "$DOCKERFILE_PATH" .

# --- 步骤 2: 完成 ---
echo "----------------------------------------"
echo "Docker image build process finished successfully!"
echo "Image created: $IMAGE_NAME:$TAG"
echo "To run the container, use the following command:"
echo "docker run -p 8080:8080 --name mjjer-container $IMAGE_NAME:$TAG"
echo "----------------------------------------"