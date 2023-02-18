#![allow(non_snake_case, non_camel_case_types)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/*
// outgoing
registerMixingServer (mixer -> api) (edited)
createdFrameAudioMixer (mixer -> api)
destroyedFrameAudioMixer (mixer->api)
// incoming
createFrameAudioMixer (api -> mixer) (edited)
destroyFrameAudioMixer (api -> mixer)
heartbeat (mixer -> api)
createFrameAudioMixer will open a port for incoming RTP from mediasoup, mediasoup would then send rtp packets when received createdFrameAudioMixer
 */

// server message sent to client
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum RequestMessage {
    IncomingServer {
        node: Option<Uuid>,
        wsid: Option<String>,
        message: MessageRequest,
    },
}

// server message sent to client
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum MessageRequest {
    #[serde(rename_all = "camelCase")]
    createFrameAudioMixer { hello: String },
    #[serde(rename_all = "camelCase")]
    destroyFrameAudioMixer {},
}

// client message sent to server
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ResponseMessage {
    OutgoingServer {
        node: Option<Uuid>,
        message: MessageResponse,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum MessageResponse {
    #[serde(rename_all = "camelCase")]
    registerMixingServer { mode: String, region: String },
    #[serde(rename_all = "camelCase")]
    serverLoad {
        mode: String,
        region: String,
        load: f32,
    },
    #[serde(rename_all = "camelCase")]
    createdFrameAudioMixer {},
    #[serde(rename_all = "camelCase")]
    destroyFrameAudioMixer {},
}
