use crate::grpc::health_grpc::health_check_response::ServingStatus;
use crate::grpc::health_grpc::health_grpc_service_server::HealthGrpcService;
use crate::grpc::health_grpc::{HealthCheckRequest, HealthCheckResponse};
use std::collections::HashMap;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

type ResponseStream = ReceiverStream<Result<HealthCheckResponse, Status>>;

// #[derive(Clone)]
pub struct HealthService {
    pub serving_status: HashMap<String, ServingStatus>,
}

#[tonic::async_trait]
impl HealthGrpcService for HealthService {
    async fn health_check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let req = request.into_inner();
        let service = req.service.to_lowercase();
        if service.is_empty() {
            let not_serving = self
                .serving_status
                .values()
                .any(|&x| x.eq(&ServingStatus::NotServing));
            if not_serving {
                let status = ServingStatus::NotServing as i32;
                let rep = HealthCheckResponse { status };
                Ok(Response::new(rep))
            } else {
                let status = ServingStatus::Serving as i32;
                let rep = HealthCheckResponse { status };
                Ok(Response::new(rep))
            }
        } else {
            // currently supporting:
            // - common (Common gRPC)
            // - typed (Typed gRPC)
            let serving_status = self.serving_status.get(service.as_str());
            match serving_status {
                Some(_s) => {
                    let serving_status = *serving_status.unwrap();
                    let status = serving_status as i32;
                    let rep = HealthCheckResponse { status };
                    Ok(Response::new(rep))
                }
                None => {
                    let status = ServingStatus::Unknown as i32;
                    let rep = HealthCheckResponse { status };
                    Ok(Response::new(rep))
                }
            }
        }
    }

    type healthWatchStream = ResponseStream;

    async fn health_watch(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<Self::healthWatchStream>, Status> {
        // TODO: support streaming health watch
        let req = request.into_inner();
        let _service = req.service.as_str();
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}