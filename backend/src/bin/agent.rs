use backend::agent_service::agent_communication_service_client::AgentCommunicationServiceClient;
use backend::agent_service::message_to_server::Payload;
use backend::agent_service::{AgentHandshake, Heartbeat, MessageToServer, OsType};
use std::time::Duration;
use sysinfo::System;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 配置参数（后续从环境变量读取）
    let server_addr = "http://127.0.0.1:50051";
    let agent_token = "secure_default_token"; // 生产环境应从安全存储读取
    
    // 创建gRPC客户端
    let mut client = AgentCommunicationServiceClient::connect(server_addr).await?;
    
    // 创建双向流
    let (tx, rx) = mpsc::channel(128);
    let response = client.establish_communication_stream(ReceiverStream::new(rx)).await?;
    let mut in_stream = response.into_inner();
    
    // 获取主机信息
    let mut sys = System::new_all();
    sys.refresh_all();
    
    // 发送握手消息
    let handshake = AgentHandshake {
        agent_id_hint: Uuid::new_v4().to_string(),
        current_agent_secret: agent_token.to_string(),
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        os_type: i32::from(OsType::Linux),
        os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
        arch: std::env::consts::ARCH.to_string(),
        hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
    };
    
    tx.send(MessageToServer {
        client_message_id: 1,
        payload: Some(Payload::AgentHandshake(handshake)),
    }).await?;
    
    // 处理服务端响应
    let mut agent_id = String::new();
    if let Some(response) = in_stream.next().await {
        match response?.payload {
            Some(backend::agent_service::message_to_agent::Payload::ServerHandshakeAck(ack)) => {
                if ack.authentication_successful {
                    agent_id = ack.assigned_agent_id.clone();
                    println!("Authenticated successfully. Agent ID: {}", agent_id);
                } else {
                    eprintln!("Authentication failed: {}", ack.error_message);
                    return Ok(());
                }
            }
            _ => {
                eprintln!("Unexpected response type");
                return Ok(());
            }
        }
    }
    
    // 启动心跳任务
    let mut client_message_id_counter = 2; // 从2开始，因为1用于握手
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        
        let heartbeat = Heartbeat {
            timestamp_unix_ms: chrono::Utc::now().timestamp_millis(),
        };
        
        let send_result = tx.send(MessageToServer {
            client_message_id: client_message_id_counter,
            payload: Some(Payload::Heartbeat(heartbeat)),
        }).await;
        
        if let Err(e) = send_result {
            eprintln!("[Agent:{}] Failed to send heartbeat: {}", agent_id, e);
            // 可考虑重连逻辑
            continue;
        }
        
        client_message_id_counter += 1;
        println!("[Agent:{}] Heartbeat sent at {}", agent_id, chrono::Utc::now());
    }
}
