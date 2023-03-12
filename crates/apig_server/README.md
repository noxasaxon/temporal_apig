# temporal_apig

## Routes

**All routes are versioned**

### /api/:version/temporal subroutes
- `/`  JSON to Temporal SDK call (Execute, Signal, Query) **Not available in PROD**
- `/encode` JSON to encoded string
- `/decode` encoded string to JSON

### /api/:version/slack subroutes
- `/interaction` Parse Slack interaction events, find the encoded string and trigger Temporal


## Slack Interaction events
All Slack interaction events have a `callback_id` field except for `block_actions` events, in which case the `action_id` is used. The encoder is used to embed the running workflow's info into the `callback_id` so that it can be routed back to the same workflow.


