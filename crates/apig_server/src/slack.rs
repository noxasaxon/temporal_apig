use crate::{versions::ApiVersion, AppError};
use anyhow::{anyhow, Result};
use axum::{response::IntoResponse, Form, Json};
use serde::{Deserialize, Serialize};
use slack_morphism::prelude::*;
use temporal_sdk_helpers::{execute_interaction, Encoder};
use tracing::log::error;

pub async fn axum_apig_handler_slack_interactions_api(
    api_version: ApiVersion,
    Form(body): Form<SlackInteractionWrapper>,
) -> Result<impl IntoResponse, AppError> {
    match api_version {
        ApiVersion::V1 => handle_slack_interaction(body).await,
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SlackInteractionWrapper {
    // payload: SlackInteractionEvent, // but as a Form
    payload: String,
}

pub async fn handle_slack_interaction(
    wrapper: SlackInteractionWrapper,
) -> Result<impl IntoResponse, AppError> {
    if let Ok(interaction_event) = serde_json::from_str::<SlackInteractionEvent>(&wrapper.payload) {
        let callback_id = get_callback_id_from_slack_interaction_event(interaction_event.clone())?;
        let temporal_info_no_inputs = Encoder::decode(&callback_id)?;

        let input_data = serde_json::to_value(&interaction_event)?;

        let temporal_info = temporal_info_no_inputs.add_data_args(Some(vec![input_data]));

        let temporal_response = execute_interaction(temporal_info).await?;

        Ok(())
    } else {
        error!("Interaction event `payload` key is not valid json or does not deserialize to existing struct");
        error!("{:?}", &wrapper);

        Err(anyhow!("failed to read slack interaction event"))?
    }
}

// https://api.slack.com/interactivity/handling#payloads
fn get_callback_id_from_slack_interaction_event(
    slack_event: SlackInteractionEvent,
) -> Result<String> {
    let callback_id = match slack_event {
        SlackInteractionEvent::BlockActions(block_action_event) => block_action_event
            .actions
            .expect("No actions in block action event")
            .first()
            .expect("Actions vector is empty, from block actions event")
            .action_id
            .to_string(),
        SlackInteractionEvent::DialogSubmission(dialog_submission_event) => dialog_submission_event
            .callback_id
            .expect("callback id not provided in dialog")
            .to_string(),
        SlackInteractionEvent::MessageAction(msg_action_event) => msg_action_event
            .callback_id
            .to_string(),
        SlackInteractionEvent::Shortcut(_shortcut_event) => todo!(),
        SlackInteractionEvent::ViewSubmission(view_submission_event) => {
            let callback_id_option = match view_submission_event.view.view {
                SlackView::Home(home_view) => home_view.callback_id,
                SlackView::Modal(modal_view) => modal_view.callback_id,
            };

            callback_id_option
                .expect("callback_id not provided to view submission")
                .to_string()
        }
        SlackInteractionEvent::ViewClosed(view_closed_event) => {
            let callback_id_option = match view_closed_event.view.view {
                SlackView::Home(home_view) => home_view.callback_id,
                SlackView::Modal(modal_view) => modal_view.callback_id,
            };

            callback_id_option
                .expect("callback_id not provided to view submission")
                .to_string()
        }
    };

    Ok(callback_id)
}
