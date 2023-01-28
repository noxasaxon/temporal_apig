use anyhow::{anyhow, Context, Result};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use temporal_client::{self, ConfiguredClient, RetryClient, TemporalServiceClientWithMetrics};
pub use temporal_json::{Encoder, TemporalInteraction};
use temporal_json::{ExecuteTemporalWorkflow, QueryTemporal, SignalTemporal};
use temporal_sdk_core_protos::{
    coresdk::AsJsonPayloadExt,
    temporal::api::{
        common::v1::{Payloads, WorkflowExecution, WorkflowType},
        enums::v1::TaskQueueKind,
        query::v1::WorkflowQuery,
        taskqueue::v1::TaskQueue,
        workflowservice::v1::{
            QueryWorkflowRequest, QueryWorkflowResponse, SignalWorkflowExecutionRequest,
            SignalWorkflowExecutionResponse, StartWorkflowExecutionRequest,
            StartWorkflowExecutionResponse,
        },
    },
};
use uuid::Uuid;

pub const DEFAULT_NAMESPACE: &str = "test-namespace";
pub type TemporalSDKClient = RetryClient<ConfiguredClient<TemporalServiceClientWithMetrics>>;

pub static TEMPORAL_HOST_PORT_PAIR: OnceCell<(String, String)> = OnceCell::new();

pub async fn build_temporal_client_without_namespace() -> Result<TemporalSDKClient> {
    let (host, port) = TEMPORAL_HOST_PORT_PAIR
        .get()
        .ok_or_else(|| anyhow!("Temporal host and port not set!"))?;

    let temporal_url = url::Url::parse(&format!("http://{host}:{port}"))?;

    let client_options = temporal_client::ClientOptionsBuilder::default()
        .identity("custom_rust_apig".into())
        .client_name("")
        .client_version("")
        .target_url(temporal_url.clone())
        .build()
        .unwrap();

    client_options
        .connect_no_namespace(None, None)
        .await
        .with_context(|| format!("Failed to create Temporal Client at url {temporal_url}"))
}

pub async fn signal_temporal(
    signal_info: SignalTemporal,
) -> Result<SignalWorkflowExecutionResponse> {
    let mut client = build_temporal_client_without_namespace().await?;

    let input = signal_info.input.map(|inputs| Payloads {
        payloads: inputs
            .into_iter()
            .map(|arg| arg.as_json_payload().unwrap())
            .collect(),
    });

    let workflow_execution = signal_info
        .workflow_id
        .map(|workflow_id| WorkflowExecution {
            workflow_id,
            run_id: signal_info.run_id.unwrap_or_default(),
        });

    let signal_response = client
        .get_client_mut()
        .workflow_svc_mut()
        .signal_workflow_execution(SignalWorkflowExecutionRequest {
            namespace: signal_info.namespace,
            workflow_execution,
            signal_name: signal_info.signal_name,
            input,
            identity: signal_info
                .identity
                .unwrap_or_else(|| "TemporalAPIG".into()),
            request_id: signal_info
                .request_id
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            control: signal_info
                .control
                .unwrap_or_else(|| "placeholder_control".into()),
            header: None,
        })
        .await?;

    Ok(signal_response.into_inner())
}

pub async fn start_temporal_workflow(
    workflow_info: ExecuteTemporalWorkflow,
) -> Result<StartWorkflowExecutionResponse> {
    let mut client = build_temporal_client_without_namespace().await?;

    let workflow_execution_request = build_workflow_execution_request(
        workflow_info.namespace,
        workflow_info.args,
        workflow_info.task_queue,
        workflow_info.workflow_id,
        workflow_info.workflow_type,
        None,
    );

    let execution_response = client
        .get_client_mut()
        .workflow_svc_mut()
        .start_workflow_execution(workflow_execution_request)
        .await?;

    Ok(execution_response.into_inner())
}

pub fn to_json_payloads(args: Vec<serde_json::Value>) -> Payloads {
    Payloads {
        payloads: args
            .iter()
            .map(|arg| arg.as_json_payload().unwrap())
            .collect(),
    }
}

pub fn build_workflow_execution_request(
    namespace: String,
    input: Option<Vec<serde_json::Value>>,
    task_queue: String,
    workflow_id: String,
    workflow_type: String,
    options: Option<temporal_client::WorkflowOptions>,
) -> StartWorkflowExecutionRequest {
    let options = options.unwrap_or_default();

    let input = input.map(to_json_payloads);

    StartWorkflowExecutionRequest {
        namespace,
        input,
        workflow_id,
        workflow_type: Some(WorkflowType {
            name: workflow_type,
        }),
        task_queue: Some(TaskQueue {
            name: task_queue,
            kind: TaskQueueKind::Unspecified as i32,
        }),
        request_id: Uuid::new_v4().to_string(),
        workflow_id_reuse_policy: options.id_reuse_policy as i32,
        workflow_execution_timeout: options.execution_timeout.and_then(|d| d.try_into().ok()),
        workflow_run_timeout: options.execution_timeout.and_then(|d| d.try_into().ok()),
        workflow_task_timeout: options.task_timeout.and_then(|d| d.try_into().ok()),
        search_attributes: options.search_attributes.and_then(|d| d.try_into().ok()),
        cron_schedule: options.cron_schedule.unwrap_or_default(),
        ..Default::default()
    }
}

pub async fn query_temporal(query_info: QueryTemporal) -> Result<QueryWorkflowResponse> {
    let mut client = build_temporal_client_without_namespace().await?;

    let input = query_info.query_args.map(|inputs| Payloads {
        payloads: inputs
            .into_iter()
            .map(|arg| arg.as_json_payload().unwrap())
            .collect(),
    });

    let workflow_execution = query_info.workflow_id.map(|workflow_id| WorkflowExecution {
        workflow_id,
        run_id: query_info.run_id.unwrap_or_default(),
    });

    let query_response = client
        .get_client_mut()
        .workflow_svc_mut()
        .query_workflow(QueryWorkflowRequest {
            namespace: query_info.namespace,
            execution: workflow_execution,
            query: Some(WorkflowQuery {
                query_type: query_info.query_type,
                query_args: input,
                header: None,
            }),
            ..Default::default() // query_reject_condition: todo!(),
        })
        .await?;

    Ok(query_response.into_inner())
}

/// Data Models ///////////////////////////////////////////////////

// {
//     "type" : "Execute",
//     "namespace" : "test-namespace",
//     "task_queue": "template-taskqueue",
//     "workflow_id" : "1",
//     "workflow_type" : "GreetingWorkflow",
//     "args":[{
//         "name" : "saxon",
//         "team" : "test-team"
//     }]
// }

// {
//     "type": "Signal",
//     "namespace": "test-namespace",
//     "task_queue": "test-task-queue",
//     "workflow_id": "some-super-long-uuid-string",
//     "run_id": "some-equally-long-uuid-string",
//     "signal_name": "signal_name_thats_defined_in_workflow",
//   }

pub async fn execute_interaction(
    interaction: TemporalInteraction,
) -> Result<TemporalInteractionResponse> {
    Ok(match interaction {
        TemporalInteraction::Execute(wf_info) => {
            TemporalInteractionResponse::from(start_temporal_workflow(wf_info).await?)
        }
        TemporalInteraction::Signal(signal_info) => {
            TemporalInteractionResponse::from(signal_temporal(signal_info).await?)
        }
        TemporalInteraction::Query(query_info) => {
            // we need `try_from` here because queries can return arbitrary data from the workflow,
            // which requires a fallible attempt at JSON conversion via serde
            TemporalInteractionResponse::try_from(query_temporal(query_info).await?)?
        }
    })
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum TemporalInteractionResponse {
    ExecuteWorkflow(TemporalExecuteWorkflowResponse),
    Signal(TemporalSignalResponse),
    Query(TemporalQueryResponse),
}

impl From<StartWorkflowExecutionResponse> for TemporalInteractionResponse {
    fn from(exec_response: StartWorkflowExecutionResponse) -> Self {
        Self::ExecuteWorkflow(TemporalExecuteWorkflowResponse {
            run_id: exec_response.run_id,
        })
    }
}

impl From<SignalWorkflowExecutionResponse> for TemporalInteractionResponse {
    fn from(_signal_response: SignalWorkflowExecutionResponse) -> Self {
        Self::Signal(TemporalSignalResponse {})
    }
}

impl TryFrom<QueryWorkflowResponse> for TemporalInteractionResponse {
    type Error = serde_json::Error;

    fn try_from(query_response: QueryWorkflowResponse) -> Result<Self, Self::Error> {
        // if we have results (payloads), convert them to JSON for HTTP transmission
        let query_result = match query_response.query_result {
            Some(payload_container) => Some(
                payload_container
                    .payloads
                    .iter()
                    .map(|payload| serde_json::to_value(&payload.data))
                    .collect::<Result<Vec<Value>, _>>()?,
            ),
            None => None,
        };

        Ok(Self::Query(TemporalQueryResponse {
            query_result,
            query_rejected: query_response
                .query_rejected
                .map(|rejected| rejected.status),
        }))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct TemporalExecuteWorkflowResponse {
    run_id: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct TemporalSignalResponse {}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct TemporalQueryResponse {
    pub query_rejected: Option<i32>,
    pub query_result: Option<Vec<Value>>,
}
