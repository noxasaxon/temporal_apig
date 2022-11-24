# temporal-json
cross-language models and encoder for serializing temporal sdk calls into JSON, for usage in temporal_apig

Used to export in crates at `/crates/temporal-json-*`

## Usage
1. start the api gateway server
2. ensure your slack bot's interaction url (configured in bot settings @ api.slack.com) is `<apig_hosted_url>/api/v1/slack/interaction`
3. start a Typescript worker with the workflow & activity code below
4. invoke the workflow
5. click the button that pops up in slack

### Workflow

```typescript
import { TemporalSignalWithoutInput } from '@saxorg/temporal_json';
import {
  proxyActivities,
  defineSignal,
  setHandler,
  condition,
  CancelledFailure,
  workflowInfo,
} from '@temporalio/workflow';
import type * as activities from '../activities';

const { slackSendMsgWithButton } = proxyActivities<typeof activities>({
  startToCloseTimeout: '10 minutes',
  retry: { maximumAttempts: 3, initialInterval: 10 },
});

export const SLACKBUTTONSIGNAL = 'slack_button_signal';
export const slackButtonResponse = defineSignal<[Record<string, unknown>]>(SLACKBUTTONSIGNAL);

export async function signalTestWorkflow(): Promise<void> {
  let slack_response: undefined | Record<string, unknown>;

  setHandler(slackButtonResponse, (resp) => {
    console.log('signal received!', resp);
    slack_response = resp;
  });

  // get workflow info for slack callback id
  const wf_info: TemporalSignalWithoutInput = {
    signalName: SLACKBUTTONSIGNAL,
    namespace: workflowInfo().namespace,
    taskQueue: workflowInfo().taskQueue,
    workflowId: workflowInfo().workflowId,
    runId: workflowInfo().runId,
  };

  // start Temporal Activity that will send a slack message
  const postMessageResp = slackSendMsgWithButton({
    wf_info,
    channel: '<my_channel_id>',
  });

  try {
    // wait for signal response
    const condition_result = await condition(() => slack_response != undefined, '10m');
    console.log('Unblocked');

    console.log(slack_response);
  } catch (err) {
    if (err instanceof CancelledFailure) {
      console.log('Cancelled');
    }
    throw err;
  }
}
```


### Temporal Activity
```typescript
import { Encoder, encodeSignalNoArgsWithVersion, TemporalSignalWithoutInput } from '@saxorg/temporal_json';
import { WebClient } from '@slack/web-api';

export async function slackSendMsgWithButton(input: {
  wf_info: TemporalSignalWithoutInput;
  channel: string;
}): Promise<string> {
  const token = process.env['SLACK_TOKEN'];
  const web = new WebClient(token);

  const encoder_version = Encoder.A;
  const encoded_temporal_info = encodeSignalNoArgsWithVersion(encoder_version, input.wf_info);

  // when using block actions, the encoded temporal info is instead provided to the action_id 
  const resp = await web.chat.postMessage({
    channel: input.channel,
    text: 'hi',
    blocks: [
      {
        type: 'actions',
        elements: [
          {
            type: 'button',
            text: {
              type: 'plain_text',
              text: 'Click to signal workflow via APIG',
              emoji: true,
            },
            value: 'click_me_for_temporal_apig',
            action_id: encoded_temporal_info,
          },
        ],
      },
    ],
  });

  return `ok: ${resp.ok}. err: ${resp.error}`;
}

```