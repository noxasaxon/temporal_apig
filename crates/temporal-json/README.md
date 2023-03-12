# temporal-json
cross-language models and encoder for serializing temporal sdk calls into JSON, for usage in temporal_apig

Used to export in crates at `/crates/temporal-json-*`

## Encoder

The encoder can transform a TemporalInteraction enum (as JSON) like this:
```json
{
    "type" : "Signal",
    "namespace" : "my-namespace",
    "task_queue": "my-taskqueue",
    "run_id": "some-run-id",
    "workflow_id":"some-workflow-id",
    "signal_name": "my_signal_name"
}
```

into a shorter encoded string like this:
```rs
"A~E:Signal,W:some-workflow-id,N:my-namespace,T:my-taskqueue,R:some-run-id,S:my_signal_name"
```


The encoded string is formatted into 3 sections, each separated by a special delimiter character: `~`
1. Encoder Version
2. The TemporalInteraction enum converted to a UTF-8 string & drastically reduced in size
3. (optional) user-provided custom data, which we ignore.
    1. **Why?** For platforms like Slack, our users may only have one field that can hold hidden custom data sent from the workflow all the way to the customer and back. We want our encoded string to not only be as small as possible but also allow the field to be used in workflows if needed.
