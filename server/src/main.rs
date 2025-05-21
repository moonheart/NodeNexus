use tonic::{transport::Server, Request, Response, Status};

// This will be the module name defined in your .proto file's package statement
// e.g., if package is "agent_service", the module will be "agent_service"
// The actual generated code will be in a file like target/debug/build/server-xxxx/out/agent_service.rs
// but we include it with the name specified in the .proto package
pub mod agent_service {
    tonic::include_proto!("agent_service"); // Matches the package name in server.proto
}

use agent_service::{
    agent_service_server::{AgentService, AgentServiceServer},
    AgentRegisterRequest, AgentRegisterResponse,
    HeartbeatRequest, HeartbeatResponse,
    UploadMetricsRequest, UploadMetricsResponse,
    UploadDockerInfoRequest, UploadDockerInfoResponse,
    ExecuteCommandRequest, ExecuteCommandResponse,
    PtyStreamMessage, // ManageFileRequest, ManageFileResponse, // Will be stream later
    ManageFileRequest, ManageFileResponse, // Corrected based on proto definition
};
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::mpsc;

#[derive(Debug, Default)]
pub struct MyAgentService {}

#[tonic::async_trait]
impl AgentService for MyAgentService {
    async fn register(
        &self,
        request: Request<AgentRegisterRequest>,
    ) -> Result<Response<AgentRegisterResponse>, Status> {
        println!("Received AgentRegisterRequest: {:?}", request.into_inner());
        let response = AgentRegisterResponse {
            success: true,
            agent_secret: "generated_secret_for_agent".to_string(), // Example secret
            assigned_agent_id: 12345, // Example assigned ID
        };
        Ok(Response::new(response))
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        println!("Received HeartbeatRequest: {:?}", request.into_inner());
        let response = HeartbeatResponse {
            success: true,
            server_timestamp: chrono::Utc::now().timestamp(),
        };
        Ok(Response::new(response))
    }

    async fn upload_metrics(
        &self,
        request: Request<UploadMetricsRequest>,
    ) -> Result<Response<UploadMetricsResponse>, Status> {
        let req_data = request.into_inner();
        println!("Received UploadMetricsRequest for agent_id: {}", req_data.agent_id);
        let count = req_data.metrics.len() as u32;
        let response = UploadMetricsResponse {
            success: true,
            processed_count: count,
        };
        Ok(Response::new(response))
    }

    async fn upload_docker_info(
        &self,
        request: Request<UploadDockerInfoRequest>,
    ) -> Result<Response<UploadDockerInfoResponse>, Status> {
        let req_data = request.into_inner();
        println!("Received UploadDockerInfoRequest for agent_id: {}", req_data.agent_id);
        let count = req_data.containers_info.len() as u32;
        let response = UploadDockerInfoResponse {
            success: true,
            processed_count: count,
        };
        Ok(Response::new(response))
    }

    async fn execute_command(
        &self,
        request: Request<ExecuteCommandRequest>,
    ) -> Result<Response<ExecuteCommandResponse>, Status> {
        let req_data = request.into_inner();
        println!("Received ExecuteCommandRequest: cmd_id={}", req_data.command_id);
        // Placeholder implementation
        let response = ExecuteCommandResponse {
            command_id: req_data.command_id,
            success: true,
            exit_code: 0,
            stdout: "Command executed successfully (placeholder)".to_string(),
            stderr: "".to_string(),
            error_message: "".to_string(),
        };
        Ok(Response::new(response))
    }

    type StreamPtyStream = ReceiverStream<Result<PtyStreamMessage, Status>>;

    async fn stream_pty(
        &self,
        request_stream: Request<tonic::Streaming<PtyStreamMessage>>,
    ) -> Result<Response<Self::StreamPtyStream>, Status> {
        println!("Received StreamPty request");
        let mut in_stream = request_stream.into_inner();
        let (tx, rx) = mpsc::channel(128); // Buffer size for messages

        // Spawn a task to handle incoming messages from the client
        // and send outgoing messages to the client.
        tokio::spawn(async move {
            while let Some(result) = futures_util::StreamExt::next(&mut in_stream).await {
                match result {
                    Ok(message) => {
                        if let Some(event) = message.event {
                            match event {
                                agent_service::pty_stream_message::Event::StartRequest(start_req) => {
                                    println!("PTY Start Request: {:?}", start_req);
                                    // Echo back a confirmation or initial output
                                    let reply = PtyStreamMessage {
                                        event: Some(agent_service::pty_stream_message::Event::OutputData(
                                            format!("PTY session started for {}\n", start_req.session_id).into_bytes()
                                        ))
                                    };
                                    if tx.send(Ok(reply)).await.is_err() {
                                        eprintln!("receiver dropped");
                                        return;
                                    }
                                }
                                agent_service::pty_stream_message::Event::InputData(data) => {
                                    println!("PTY Input Data: {} bytes", data.len());
                                    // Echo input back as output for now
                                    let reply = PtyStreamMessage {
                                        event: Some(agent_service::pty_stream_message::Event::OutputData(data))
                                    };
                                    if tx.send(Ok(reply)).await.is_err() {
                                        eprintln!("receiver dropped");
                                        return;
                                    }
                                }
                                agent_service::pty_stream_message::Event::ResizeEvent(resize_event) => {
                                    println!("PTY Resize Event: {:?}", resize_event);
                                }
                                agent_service::pty_stream_message::Event::CloseSignal(close) => {
                                    println!("PTY Close Signal: {}", close);
                                    // Client wants to close, break the loop
                                    break;
                                }
                                _ => {
                                    println!("Received unhandled PTY event from client");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error receiving PTY message from client: {:?}", e);
                        break;
                    }
                }
            }
            println!("PTY client stream ended.");
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn manage_file(
        &self,
        request: Request<ManageFileRequest>,
    ) -> Result<Response<ManageFileResponse>, Status> {
        let req_data = request.into_inner();
        println!("Received ManageFileRequest: req_id={}, op={:?}", req_data.request_id, req_data.operation);
        // Placeholder implementation
        let response = ManageFileResponse {
            request_id: req_data.request_id,
            success: true,
            error_message: "".to_string(),
            ..Default::default() // Fill with default for other fields
        };
        Ok(Response::new(response))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let agent_service = MyAgentService::default();

    println!("AgentService server listening on {}", addr);

    Server::builder()
        .add_service(AgentServiceServer::new(agent_service))
        .serve(addr)
        .await?;

    Ok(())
}
