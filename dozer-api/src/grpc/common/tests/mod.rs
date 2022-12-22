use tonic::Request;

use crate::grpc::{
    common_grpc::{
        common_grpc_service_server::CommonGrpcService, EventType, GetEndpointsRequest,
        GetFieldsRequest, OnEventRequest, QueryRequest,
    },
    typed::tests::service::setup_pipeline,
    types::{value, FieldDefinition, OperationType, Type, Value},
};

use super::CommonService;

fn setup_common_service() -> CommonService {
    let (pipeline_map, _, rx1) = setup_pipeline();
    CommonService {
        pipeline_map,
        event_notifier: rx1,
    }
}

#[tokio::test]
async fn test_grpc_common_query() {
    let service = setup_common_service();
    let response = service
        .query(Request::new(QueryRequest {
            endpoint: "films".to_string(),
            query: Some(r#"{ "$filter": { "film_id": 524 } }"#.to_string()),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(!response.records.is_empty());
}

#[tokio::test]
async fn test_grpc_common_get_endpoints() {
    let service = setup_common_service();
    let response = service
        .get_endpoints(Request::new(GetEndpointsRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(response.endpoints, vec!["films".to_string()]);
}

#[tokio::test]
async fn test_grpc_common_get_fields() {
    let service = setup_common_service();
    let response = service
        .get_fields(Request::new(GetFieldsRequest {
            endpoint: "films".to_string(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        response.fields,
        vec![
            FieldDefinition {
                typ: Type::UInt as i32,
                name: "film_id".to_string(),
                nullable: false
            },
            FieldDefinition {
                typ: Type::String as i32,
                name: "description".to_string(),
                nullable: true
            },
            FieldDefinition {
                typ: Type::Float as i32,
                name: "rental_rate".to_string(),
                nullable: true
            },
            FieldDefinition {
                typ: Type::Int as i32,
                name: "release_year".to_string(),
                nullable: true
            },
            FieldDefinition {
                typ: Type::Timestamp as i32,
                name: "updated_at".to_string(),
                nullable: true
            }
        ]
    );
}

#[tokio::test]
async fn test_grpc_common_on_event() {
    let service = setup_common_service();
    let mut rx = service
        .on_event(Request::new(OnEventRequest {
            endpoint: "films".to_string(),
            r#type: EventType::All as i32,
            filter: Some(r#"{ "film_id": 32 }"#.to_string()),
        }))
        .await
        .unwrap()
        .into_inner()
        .into_inner();
    let operation = rx.recv().await.unwrap().unwrap();
    assert_eq!(operation.endpoint_name, "films".to_string());
    assert_eq!(operation.typ, OperationType::Insert as i32);
    assert_eq!(
        operation.new.unwrap().values[0],
        Value {
            value: Some(value::Value::UintValue(32))
        }
    );
}