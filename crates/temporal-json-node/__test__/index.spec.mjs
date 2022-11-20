import test from 'ava'

import { encodeDefaultFromJsonString, decodeToJsonString } from '../index.js'

function build_temporal_interaction_exec_wf(){
  return {
    "type" : "Execute",
    "namespace" : "test-namespace",
    "task_queue": "template-taskqueue",
    "workflow_id" : "1",
    "workflow_type" : "GreetingWorkflow",
    "args":[{
        "name" : "saxon",
        "team" : "noxasaxon"
    }]
  }
}


function build_temporal_interaction_signal(){
  return {
    "type": "Signal",
    "namespace": "test-namespace",
    "task_queue": "test-task-queue-rs",
    "workflow_id": "some-super-long-uuid-string",
    "run_id": "some-equally-long-uuid-string",
    "signal_name": "signal_name_thats_defined_in_workflow",
  }
}



test('encodeDefaultFromJsonString', (t) => {
  const temporal_interaction = build_temporal_interaction_signal();

  const as_string = JSON.stringify(temporal_interaction);

  t.notThrows(() => encodeDefaultFromJsonString(as_string));
})

test('decodeFromJsonString', (t) => {
  const temporal_interaction = build_temporal_interaction_signal();

  const as_string = JSON.stringify(temporal_interaction);

  const encoded_string = encodeDefaultFromJsonString(as_string);

  const decoded_json_string = decodeToJsonString(encoded_string);

  t.notThrows(() => encodeDefaultFromJsonString(decoded_json_string));
})


test('test all event types', (t) => {
  for (const temporal_event_json of [build_temporal_interaction_exec_wf(), build_temporal_interaction_signal()]) {
  
    const as_string = JSON.stringify(temporal_event_json);
  
    const encoded_string = encodeDefaultFromJsonString(as_string);
  
    const decoded_json_string = decodeToJsonString(encoded_string);

    t.notThrows(() => encodeDefaultFromJsonString(decoded_json_string));
  }
})


