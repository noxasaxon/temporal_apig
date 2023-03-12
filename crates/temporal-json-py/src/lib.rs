//! Python bindings for the `temporal-json` crate
//! Provides an encoder & decoder interface for the temporal api gateway

use pyo3::{exceptions::PyTypeError, prelude::*};
use temporal_json::TemporalInteraction;
pub use temporal_json::{Encoder, SignalTemporal};

/// A Python module implemented in Rust.
#[pymodule]
fn saxorg_temporal_json(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(encode_signal_no_args_default, m)?)?;
    m.add_function(wrap_pyfunction!(encode_signal_no_args_with_version, m)?)?;
    m.add_function(wrap_pyfunction!(encode_default_from_json_string, m)?)?;
    m.add_function(wrap_pyfunction!(decode_to_json_string, m)?)?;
    m.add_class::<TemporalSignalWithoutInput>()?;
    m.add_class::<Encoder>()?;

    Ok(())
}

#[pyclass]
#[derive(Clone)]
/// A Signal struct without the Input Payload.
///
/// This is useful for routing webhook events back to the workflow,
/// and the event itself will be sent as the Signal's Input to the workflow.
pub struct TemporalSignalWithoutInput {
    pub namespace: String,
    pub task_queue: String,
    pub workflow_id: Option<String>,
    pub run_id: Option<String>,
    pub signal_name: String,
}

impl Into<SignalTemporal> for TemporalSignalWithoutInput {
    fn into(self) -> SignalTemporal {
        SignalTemporal {
            namespace: self.namespace,
            task_queue: self.task_queue,
            workflow_id: self.workflow_id,
            run_id: self.run_id,
            signal_name: self.signal_name,
            ..Default::default()
        }
    }
}

#[pyfunction]
/// Convert workflow metadata into an encoded string for embedding into a webhook event, using the default encoding.
///
/// Example: use this string as the `callback_id` for a slack interaction, and when
/// the Temporal API Gateway receives the event it can decode the string and route the event to your workflow.
pub fn encode_signal_no_args_default(signal: TemporalSignalWithoutInput) -> String {
    Encoder::default().encode(TemporalInteraction::Signal(signal.into()))
}

#[pyfunction]
/// Convert workflow metadata into an encoded string for embedding into a webhook event, with a specific encoding version.
///
/// Example: use this string as the `callback_id` for a slack interaction, and when
/// the Temporal API Gateway receives the event it can decode the string and route the event to your workflow.
pub fn encode_signal_no_args_with_version(
    encoder_version: Encoder,
    signal: TemporalSignalWithoutInput,
) -> String {
    encoder_version.encode(TemporalInteraction::Signal(signal.into()))
}

#[pyfunction]
/// Encode a TemporalInteraction struct provided as a JSON string.
///
/// panics if JSON structure is not correct.
pub fn encode_default_from_json_string(json_string: String) -> Result<String, PyErr> {
    Encoder::encode_default_from_json_string(&json_string).map_err(|err| {
        PyErr::new::<PyTypeError, _>(format!("failed to encode from json string, {}", err))
    })
}

#[pyfunction]
/// Decode an encoded string into a JSON string representing a TemporalInteraction struct.
pub fn decode_to_json_string(encoded_string: String) -> Result<String, PyErr> {
    Encoder::decode_to_json_string(&encoded_string).map_err(|err| {
        PyErr::new::<PyTypeError, _>(format!("failed to decode to json string, {}", err))
    })
}
