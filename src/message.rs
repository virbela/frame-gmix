#![allow(non_snake_case, non_camel_case_types)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// server message sent to client
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum RequestMessage {
    Incoming {
        wsid: String,
        message: MessageRequest,
    },
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
    Ping,
    #[serde(rename_all = "camelCase")]
    createRouterGroup {},
}

// client message sent to server
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ResponseMessage {
    Outgoing {
        ws: Option<String>,
        message: MessageResponse,
    },
    OutgoingCommunication {
        ws: Option<String>,
        communication: MessageResponse,
    },
    OutgoingServer {
        // wsid: Option<String>,
        node: Option<Uuid>,
        message: MessageResponse,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum MessageResponse {
    Ping,
    #[serde(rename_all = "camelCase")]
    registerMediaServer {
        mode: String,
        region: String,
    },
}
