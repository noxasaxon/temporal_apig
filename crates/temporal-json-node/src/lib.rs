#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

pub mod encoder {
  use napi::Status;
  use temporal_json::TemporalInteraction;
  pub use temporal_json::{Encoder, SignalTemporal};

  #[napi(object)]
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

  #[napi]
  /// Convert workflow metadata into an encoded string for embedding into a webhook event, using the default encoding.
  ///
  /// Example: use this string as the `callback_id` for a slack interaction, and when
  /// the Temporal API Gateway receives the event it can decode the string and route the event to your workflow.
  fn encode_signal_no_args_default(signal: TemporalSignalWithoutInput) -> String {
    Encoder::default().encode(TemporalInteraction::Signal(signal.into()))
  }

  #[napi]
  /// Convert workflow metadata into an encoded string for embedding into a webhook event, with a specific encoding version.
  ///
  /// Example: use this string as the `callback_id` for a slack interaction, and when
  /// the Temporal API Gateway receives the event it can decode the string and route the event to your workflow.
  fn encode_signal_no_args_with_version(
    encoder_version: Encoder,
    signal: TemporalSignalWithoutInput,
  ) -> String {
    encoder_version.encode(TemporalInteraction::Signal(signal.into()))
  }

  #[napi]
  /// Encode a TemporalInteraction struct provided as a JSON string.
  ///
  /// panics if JSON structure is not correct.
  fn encode_default_from_json_string(json_string: String) -> napi::Result<String> {
    Encoder::encode_default_from_json_string(&json_string).map_err(|err| {
      napi::Error::new(
        Status::GenericFailure,
        format!("failed to encode from json string, {}", err),
      )
    })
  }

  #[napi]
  /// Decode an encoded string into a JSON string representing a TemporalInteraction struct.
  fn decode_to_json_string(encoded_string: String) -> napi::Result<String> {
    Encoder::decode_to_json_string(&encoded_string).map_err(|err| {
      napi::Error::new(
        Status::GenericFailure,
        format!("failed to decode from encoded string, {}", err),
      )
    })
  }
}
