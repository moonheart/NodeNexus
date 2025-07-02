# build-docker.ps1

# 当任何命令失败时，立即退出脚本
$ErrorActionPreference = "Stop"

# --- 定义变量 ---
$ImageName = "moonheartmoon/node-nexus-server"
$Tag = "latest"
$Dockerfile = "Dockerfile"

# --- 步骤 1: 构建 Docker 镜像 ---
Write-Host "Building Docker image: $($ImageName):$($Tag)..."
docker build -t "$($ImageName):$($Tag)" -f $Dockerfile .
docker push "$($ImageName):$($Tag)" | Out-Null

# --- 步骤 2: 完成 ---
Write-Host "----------------------------------------"
Write-Host "Docker image build process finished successfully!"
Write-Host "Image created: $($ImageName):$($Tag)"
Write-Host "To run the container, use the following command:"
Write-Host "docker run -p 8080:8080 --name node-nexus-container $($ImageName):$($Tag)"
Write-Host "----------------------------------------"