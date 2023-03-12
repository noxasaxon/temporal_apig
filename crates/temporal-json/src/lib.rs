use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, hash::Hash, str::FromStr};
use strum::{Display, EnumDiscriminants, EnumIter, EnumString, IntoEnumIterator};

#[cfg(feature = "js")]
use napi_derive::napi;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, EnumDiscriminants, Clone)]
#[serde(tag = "type")]
#[strum_discriminants(derive(EnumString, Display))]
pub enum TemporalInteraction {
    Execute(ExecuteTemporalWorkflow),
    Signal(SignalTemporal),
    Query(QueryTemporal),
}

impl TemporalInteraction {
    pub fn to_type_string(&self) -> String {
        match self {
            TemporalInteraction::Execute(_) => {
                TemporalInteractionDiscriminants::Execute.to_string()
            }
            TemporalInteraction::Signal(_) => TemporalInteractionDiscriminants::Signal.to_string(),
            TemporalInteraction::Query(_) => TemporalInteractionDiscriminants::Query.to_string(),
        }
    }

    pub fn to_slack_string(self) -> String {
        Encoder::A.encode(self)
    }

    pub fn workflow_id(&self) -> String {
        match self {
            TemporalInteraction::Execute(action) => action.workflow_id.clone(),
            TemporalInteraction::Signal(action) => action
                .workflow_id
                .as_ref()
                .map_or("".into(), |some| some.clone()),
            TemporalInteraction::Query(action) => action
                .workflow_id
                .as_ref()
                .map_or("".into(), |some| some.clone()),
        }
    }

    pub fn task_queue(&self) -> String {
        match self {
            TemporalInteraction::Execute(action) => action.task_queue.clone(),
            TemporalInteraction::Signal(action) => action.task_queue.clone(),
            TemporalInteraction::Query(action) => action.task_queue.clone(),
        }
    }

    pub fn namespace(&self) -> String {
        match self {
            TemporalInteraction::Execute(action) => action.namespace.clone(),
            TemporalInteraction::Signal(action) => action.namespace.clone(),
            TemporalInteraction::Query(action) => action.namespace.clone(),
        }
    }

    pub fn add_data_args(self, args: Option<Vec<serde_json::Value>>) -> Self {
        match self {
            Self::Execute(exec) => Self::Execute(ExecuteTemporalWorkflow { args, ..exec }),
            Self::Signal(signal) => Self::Signal(SignalTemporal {
                input: args,
                ..signal
            }),
            Self::Query(query) => Self::Query(QueryTemporal {
                query_args: args,
                ..query
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default, Clone)]
pub struct ExecuteTemporalWorkflow {
    pub namespace: String,
    pub task_queue: String,
    pub workflow_id: String,
    /// the Workflow's Function name
    pub workflow_type: String,
    pub args: Option<Vec<serde_json::Value>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default, Clone)]

pub struct SignalTemporal {
    pub namespace: String,
    pub task_queue: String,
    pub workflow_id: Option<String>,
    pub run_id: Option<String>,
    pub signal_name: String,
    pub input: Option<Vec<serde_json::Value>>,
    pub identity: Option<String>,
    pub request_id: Option<String>,
    pub control: Option<String>,
}

impl SignalTemporal {
    pub fn run_id(&self) -> String {
        self.run_id.as_ref().map_or("".into(), |some| some.clone())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default, Clone)]
pub struct TemporalWorkflowExecutionInfo {
    pub workflow_id: String,
    pub run_id: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct QueryTemporal {
    pub namespace: String,
    pub task_queue: String,
    pub workflow_id: Option<String>,
    pub run_id: Option<String>,
    pub query_type: String,
    pub query_args: Option<Vec<serde_json::Value>>,
}

impl QueryTemporal {
    pub fn run_id(&self) -> String {
        self.run_id.as_ref().map_or("".into(), |some| some.clone())
    }

    pub fn query_args(&self) -> String {
        self.query_args.as_ref().map_or("".into(), |some| {
            some.first()
                .map_or_else(|| "".to_string(), |arg| arg.to_string())
        })
    }
}

pub const SLACK_INFO_DELIMITER: &str = ",";
pub const TEMPORAL_KEY_DELIMITER: &str = ":";
pub const ENCODER_SECTION_DELIMITER: &str = "~";
pub const ENCODER_HELP_MSG: &str =
    "Encoder string format: version~temporal_key:temporal_value~user_data";

#[derive(EnumIter, EnumString, Display, PartialEq, Eq, Hash, Debug, Clone)]
#[cfg_attr(feature = "js", napi)]
#[cfg_attr(feature = "python", pyo3::pyclass)]
pub enum Encoder {
    A,
}

impl Default for Encoder {
    fn default() -> Self {
        Self::A
    }
}

impl Encoder {
    pub fn from_encoded_str(encoded: &str) -> Result<(Self, &str)> {
        let (version_str, encoded_without_version) = encoded
            .split_once(ENCODER_SECTION_DELIMITER)
            .ok_or_else(|| anyhow!("Malformed version in encoder string: {}", ENCODER_HELP_MSG))?;

        // return tuple of (Encoder, rest_of_string_without_version)
        Self::from_str(version_str)
            .context("invalid version string")
            .map(|version| (version, encoded_without_version))
    }

    pub fn encode(&self, temporal_interaction: TemporalInteraction) -> String {
        match self {
            Encoder::A => {
                let mut kv_pairs = Vec::new();

                let namespace = temporal_interaction.namespace();
                let task_queue = temporal_interaction.task_queue();
                let workflow_id = temporal_interaction.workflow_id();

                // set event type from outer enum variant
                kv_pairs
                    .push(KeysToTemporalAction::E.to_kv(&temporal_interaction.to_type_string()));

                match temporal_interaction {
                    TemporalInteraction::Execute(action) => {
                        for key in KeysToTemporalAction::iter() {
                            kv_pairs.push(key.to_kv(match key {
                                KeysToTemporalAction::W => &workflow_id,
                                KeysToTemporalAction::N => &namespace,
                                KeysToTemporalAction::T => &task_queue,
                                KeysToTemporalAction::Y => &action.workflow_type,
                                _ => continue,
                            }))
                        }
                    }
                    TemporalInteraction::Signal(action) => {
                        for key in KeysToTemporalAction::iter() {
                            kv_pairs.push(match key {
                                KeysToTemporalAction::W => key.to_kv(&workflow_id),
                                KeysToTemporalAction::N => key.to_kv(&namespace),
                                KeysToTemporalAction::T => key.to_kv(&task_queue),
                                KeysToTemporalAction::R => key.to_kv(&action.run_id()),
                                KeysToTemporalAction::S => key.to_kv(&action.signal_name),
                                _ => continue,
                            })
                        }
                    }
                    TemporalInteraction::Query(action) => {
                        for key in KeysToTemporalAction::iter() {
                            match key {
                                KeysToTemporalAction::W => key.to_kv(&workflow_id),
                                KeysToTemporalAction::N => key.to_kv(&namespace),
                                KeysToTemporalAction::T => key.to_kv(&task_queue),
                                KeysToTemporalAction::Q => key.to_kv(&action.query_type),
                                KeysToTemporalAction::U => key.to_kv(&action.query_args()),
                                _ => continue,
                            };
                        }
                    }
                }

                format!(
                    "{}{}{}",
                    self,
                    ENCODER_SECTION_DELIMITER,
                    kv_pairs.join(",")
                )
            }
        }
    }

    /// an encoded callback_id is a string with key:value pairs, with comma separation between pairs.
    /// Users can supply their own data after the encoding by adding a delimiter `~` after the encoding before their data.
    /// character limit for entire string is 255, and the temporal info takes up around 170 chars.
    ///
    /// `"A~E:Signal,W:some-super-long-uuid-string,N:test-namespace,T:test-task-queue-rs,R:some-equally-long-uuid-string,S:signal_name_thats_defined_in_workflow~Some User Defined Data Under 80 chars"`
    pub fn decode(encoded_str: &str) -> Result<TemporalInteraction> {
        let (encoder_version, encoded_str_without_version) = Self::from_encoded_str(encoded_str)?;

        match encoder_version {
            Encoder::A => {
                // a comma separated string of key:value pairs. keys are KeysToTemporalAction variants
                let temporal_encoded_str = encoded_str_without_version
                    .split_once(ENCODER_SECTION_DELIMITER)
                    .map_or_else(
                        || encoded_str_without_version,
                        |(temporal_str, _user_str)| temporal_str,
                    );

                let kv_pairs = temporal_encoded_str
                    .split(SLACK_INFO_DELIMITER)
                    .map(|kv_pair| {
                        kv_pair
                            .split_once(TEMPORAL_KEY_DELIMITER)
                            .ok_or_else(|| anyhow!("not a formatted kv pair"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let mut encoder_map: HashMap<KeysToTemporalAction, &str> = HashMap::default();
                for (k, v) in kv_pairs {
                    let formatted_key = KeysToTemporalAction::from_str(k)?;
                    encoder_map.insert(formatted_key, v);
                }

                let temporal_event_type_str = encoder_map
                    .remove(&KeysToTemporalAction::E)
                    .ok_or_else(|| anyhow!("temporal event type not supplied in callback_id"))?;

                let temporal_event_type =
                    TemporalInteractionDiscriminants::from_str(temporal_event_type_str)?;

                let namespace = KeysToTemporalAction::N.get_value(&mut encoder_map)?.into();
                let task_queue = KeysToTemporalAction::T.get_value(&mut encoder_map)?.into();

                let temporal_event_without_payload = match temporal_event_type {
                    TemporalInteractionDiscriminants::Execute => {
                        TemporalInteraction::Execute(ExecuteTemporalWorkflow {
                            namespace,
                            task_queue,
                            workflow_id: KeysToTemporalAction::W
                                .get_value(&mut encoder_map)?
                                .into(),
                            workflow_type: KeysToTemporalAction::Y
                                .get_value(&mut encoder_map)?
                                .into(),
                            args: None,
                        })
                    }
                    TemporalInteractionDiscriminants::Signal => {
                        TemporalInteraction::Signal(SignalTemporal {
                            namespace,
                            task_queue,
                            workflow_id: KeysToTemporalAction::W
                                .get_value(&mut encoder_map)
                                .ok()
                                .map(|s| s.into()),
                            run_id: KeysToTemporalAction::R
                                .get_value(&mut encoder_map)
                                .ok()
                                .map(|s| s.into()),
                            signal_name: KeysToTemporalAction::S
                                .get_value(&mut encoder_map)?
                                .into(),
                            input: None,
                            ..Default::default()
                        })
                    }
                    TemporalInteractionDiscriminants::Query => {
                        TemporalInteraction::Query(QueryTemporal {
                            namespace,
                            task_queue,
                            workflow_id: KeysToTemporalAction::W
                                .get_value(&mut encoder_map)
                                .ok()
                                .map(|s| s.into()),
                            run_id: KeysToTemporalAction::R
                                .get_value(&mut encoder_map)
                                .ok()
                                .map(|s| s.into()),
                            query_type: KeysToTemporalAction::Q.get_value(&mut encoder_map)?.into(),
                            query_args: None,
                        })
                    }
                };

                Ok(temporal_event_without_payload)
            }
        }
    }

    pub fn encode_default_from_json_string(temporal_action_as_json_str: &str) -> Result<String> {
        let interaction: TemporalInteraction = serde_json::from_str(temporal_action_as_json_str)?;

        Ok(Self::default().encode(interaction))
    }

    pub fn decode_to_json_string(encoded_str: &str) -> Result<String> {
        let as_temporal_struct = Self::decode(encoded_str)?;

        serde_json::to_string(&as_temporal_struct)
            .with_context(|| "unable to convert temporal interaction to json")
    }
}

#[derive(EnumIter, EnumString, Display, PartialEq, Eq, Hash, Debug)]
pub enum KeysToTemporalAction {
    /// Temporal Event Type (signal, query, execute)
    E,
    /// Workflow_id
    W,
    /// Namespace
    N,
    /// Taskqueue
    T,
    /// workflow tYpe aka fn name
    Y,
    /// workflow Run_id
    R,
    /// Signal name
    S,
    /// Query type
    Q,
    /// qUery args
    U,
}

impl KeysToTemporalAction {
    pub fn to_kv(&self, value: &str) -> String {
        format!("{}{}{}", self, TEMPORAL_KEY_DELIMITER, value)
    }

    pub fn get_value<'a>(&self, encoder_map: &mut HashMap<Self, &'a str>) -> Result<&'a str> {
        encoder_map.remove(self).ok_or_else(|| {
            anyhow!(
                "temporal key: `{:?}` not supplied in callback_id. encoder_map =  {:?}",
                self,
                encoder_map
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{SignalTemporal, TemporalInteraction};

    fn build_mock_signal() -> TemporalInteraction {
        TemporalInteraction::Signal(SignalTemporal {
            namespace: "test-namespace".into(),
            task_queue: "test-task-queue-rs".into(),
            workflow_id: Some("some-super-long-uuid-string".into()),
            run_id: Some("some-equally-long-uuid-string".into()),
            signal_name: "signal_name_thats_defined_in_workflow".into(),
            input: None,
            ..Default::default()
        })
    }

    fn build_mock_wf_exec() -> TemporalInteraction {
        TemporalInteraction::Execute(ExecuteTemporalWorkflow {
            namespace: "test-namespace".into(),
            task_queue: "test-task-queue-rs".into(),
            workflow_id: "some-super-long-uuid-string".into(),
            workflow_type: "some-wf-function-name".into(),
            args: Some(vec![json!({
                    "arg1" : "value1"
            })]),
        })
    }

    #[test]
    fn test_encode_slack_callback_id() {
        let temporal_interaction = build_mock_signal();
        let encoder = Encoder::A;
        let callback_id = encoder.encode(temporal_interaction);

        dbg!(&callback_id);
        dbg!(&callback_id.len()); // 143 as ordered csv, 153 as kv_pairs
    }

    #[test]
    fn test_decode_slack_callback_id() {
        let temporal_interaction = build_mock_signal();

        let encoder = Encoder::A;
        let callback_id = encoder.encode(temporal_interaction);

        let parsed = Encoder::decode(&callback_id).unwrap();

        assert_eq!(build_mock_signal(), parsed)
    }

    #[test]
    fn test_encode_decode_all_encoder_versions() {
        for encoder_version in Encoder::iter() {
            for temporal_event in [build_mock_signal(), build_mock_wf_exec()] {
                // get expected decoded item for each event type
                let expected_output = match &temporal_event {
                    TemporalInteraction::Execute(exec_wf) => {
                        TemporalInteraction::Execute(ExecuteTemporalWorkflow {
                            args: None,
                            ..exec_wf.to_owned()
                        })
                    }
                    TemporalInteraction::Signal(_sig_wf) => temporal_event.to_owned(),
                    TemporalInteraction::Query(_query_wf) => temporal_event.to_owned(),
                };

                // as struct
                let callback_id = encoder_version.encode(temporal_event.clone());
                let parsed = Encoder::decode(&callback_id).expect(&format!(
                    "failed to decode. version {encoder_version} for string {callback_id}"
                ));
                assert_eq!(expected_output, parsed);

                // as json string
                let as_string = serde_json::to_string(&temporal_event).unwrap();
                let callback_id = Encoder::encode_default_from_json_string(&as_string).unwrap();
                let parsed = Encoder::decode(&callback_id).expect(&format!(
                    "failed to decode. version {encoder_version} for string {callback_id}"
                ));
                assert_eq!(expected_output, parsed)
            }
        }
    }
}
