use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "system")]
    System {
        subtype: String,
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        status: Option<String>,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        cwd: Option<String>,
        #[serde(default)]
        error: Option<String>,
        #[serde(default)]
        message: Option<String>,
    },

    #[serde(rename = "assistant")]
    Assistant {
        message: AssistantMessage,
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        error: Option<String>,
    },

    #[serde(rename = "user")]
    User {
        #[serde(default)]
        message: serde_json::Value,
        #[serde(default)]
        session_id: Option<String>,
    },

    #[serde(rename = "result")]
    Completion {
        #[serde(default)]
        subtype: Option<String>,
        #[serde(default)]
        is_error: bool,
        #[serde(default)]
        result: String,
        #[serde(default)]
        total_cost_usd: f64,
        #[serde(default)]
        duration_ms: u64,
        #[serde(default)]
        num_turns: u32,
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        stop_reason: Option<String>,
        #[serde(default, rename = "modelUsage")]
        model_usage: Option<serde_json::Value>,
        #[serde(default)]
        usage: Option<serde_json::Value>,
    },

    #[serde(rename = "stream_event")]
    StreamDelta {
        event: DeltaEvent,
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        ttft_ms: Option<u64>,
    },

    #[serde(rename = "rate_limit_event")]
    RateLimit {
        #[serde(default)]
        rate_limit_info: serde_json::Value,
        #[serde(default)]
        session_id: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum DeltaEvent {
    #[serde(rename = "message_start")]
    MessageStart {
        #[serde(default)]
        message: Option<serde_json::Value>,
    },

    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: u32,
        content_block: ContentBlock,
    },

    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: u32, delta: Delta },

    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: u32 },

    #[serde(rename = "message_delta")]
    MessageDelta {
        #[serde(default)]
        delta: serde_json::Value,
        #[serde(default)]
        usage: Option<serde_json::Value>,
    },

    #[serde(rename = "message_stop")]
    MessageStop {},

    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text {
        #[serde(default)]
        text: String,
    },

    #[serde(rename = "tool_use")]
    ToolUse {
        #[serde(default)]
        id: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },

    #[serde(rename = "thinking")]
    Thinking {
        #[serde(default)]
        thinking: String,
    },

    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Delta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },

    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },

    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },

    #[serde(rename = "signature_delta")]
    SignatureDelta { signature: String },

    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct AssistantMessage {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub content: Vec<ContentBlock>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub usage: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_system_init() {
        let line =
            r#"{"type":"system","subtype":"init","cwd":"/tmp","session_id":"abc","model":"opus"}"#;
        let event: StreamEvent = serde_json::from_str(line).unwrap();
        match event {
            StreamEvent::System { subtype, model, .. } => {
                assert_eq!(subtype, "init");
                assert_eq!(model.as_deref(), Some("opus"));
            }
            _ => panic!("expected System event"),
        }
    }

    #[test]
    fn parse_system_status() {
        let line =
            r#"{"type":"system","subtype":"status","status":"requesting","session_id":"abc"}"#;
        let event: StreamEvent = serde_json::from_str(line).unwrap();
        match event {
            StreamEvent::System {
                subtype, status, ..
            } => {
                assert_eq!(subtype, "status");
                assert_eq!(status.as_deref(), Some("requesting"));
            }
            _ => panic!("expected System event"),
        }
    }

    #[test]
    fn parse_text_delta() {
        let line = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hello"}},"session_id":"abc"}"#;
        let event: StreamEvent = serde_json::from_str(line).unwrap();
        match event {
            StreamEvent::StreamDelta {
                event:
                    DeltaEvent::ContentBlockDelta {
                        delta: Delta::TextDelta { text },
                        ..
                    },
                ..
            } => {
                assert_eq!(text, "hello");
            }
            _ => panic!("expected StreamDelta with text delta"),
        }
    }

    #[test]
    fn parse_result_success() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":4441,"num_turns":1,"result":"hello","session_id":"abc","total_cost_usd":0.0426145}"#;
        let event: StreamEvent = serde_json::from_str(line).unwrap();
        match event {
            StreamEvent::Completion {
                is_error,
                result,
                total_cost_usd,
                ..
            } => {
                assert!(!is_error);
                assert_eq!(result, "hello");
                assert!((total_cost_usd - 0.0426145).abs() < 0.0001);
            }
            _ => panic!("expected Completion event"),
        }
    }

    #[test]
    fn parse_result_error() {
        let line = r#"{"type":"result","subtype":"success","is_error":true,"result":"Not logged in","session_id":"abc","total_cost_usd":0}"#;
        let event: StreamEvent = serde_json::from_str(line).unwrap();
        match event {
            StreamEvent::Completion {
                is_error, result, ..
            } => {
                assert!(is_error);
                assert_eq!(result, "Not logged in");
            }
            _ => panic!("expected Completion event"),
        }
    }

    #[test]
    fn parse_rate_limit() {
        let line = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed"},"session_id":"abc"}"#;
        let event: StreamEvent = serde_json::from_str(line).unwrap();
        assert!(matches!(event, StreamEvent::RateLimit { .. }));
    }

    #[test]
    fn parse_content_block_start_tool_use() {
        let line = r#"{"type":"stream_event","event":{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_123","name":"Bash","input":{}}},"session_id":"abc"}"#;
        let event: StreamEvent = serde_json::from_str(line).unwrap();
        match event {
            StreamEvent::StreamDelta {
                event:
                    DeltaEvent::ContentBlockStart {
                        content_block: ContentBlock::ToolUse { name, .. },
                        ..
                    },
                ..
            } => {
                assert_eq!(name, "Bash");
            }
            _ => panic!("expected tool use content block start"),
        }
    }

    #[test]
    fn parse_assistant_message() {
        let line = r#"{"type":"assistant","message":{"id":"msg_123","model":"opus","role":"assistant","content":[{"type":"text","text":"hello"}]},"session_id":"abc"}"#;
        let event: StreamEvent = serde_json::from_str(line).unwrap();
        match event {
            StreamEvent::Assistant { message, .. } => {
                assert_eq!(message.content.len(), 1);
            }
            _ => panic!("expected Assistant event"),
        }
    }

    #[test]
    fn unknown_event_type_fails() {
        let line = r#"{"type":"unknown_future_event","data":123}"#;
        assert!(serde_json::from_str::<StreamEvent>(line).is_err());
    }

    #[test]
    fn extra_fields_ignored() {
        let line = r#"{"type":"system","subtype":"init","session_id":"abc","unknown_field":"value","nested":{"deep":true}}"#;
        assert!(serde_json::from_str::<StreamEvent>(line).is_ok());
    }

    #[test]
    fn parse_real_captured_output() {
        let lines = [
            r#"{"type":"system","subtype":"init","cwd":"/Users/personal/os-projects/yoke","session_id":"4e2a7dc0-100a-4e9e-b8f2-a4b4dac118b2","tools":["Task"],"model":"claude-opus-4-6"}"#,
            r#"{"type":"system","subtype":"status","status":"requesting","uuid":"6faa653b","session_id":"4e2a7dc0"}"#,
            r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1777003200},"uuid":"83714cbd","session_id":"4e2a7dc0"}"#,
            r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hello"}},"session_id":"4e2a7dc0","parent_tool_use_id":null,"uuid":"ef4acf2c"}"#,
            r#"{"type":"result","subtype":"success","is_error":false,"duration_ms":4441,"duration_api_ms":5265,"num_turns":1,"result":"hello","stop_reason":"end_turn","session_id":"4e2a7dc0","total_cost_usd":0.0426145}"#,
        ];
        for line in lines {
            let result = serde_json::from_str::<StreamEvent>(line);
            assert!(
                result.is_ok(),
                "failed to parse: {line}\nerror: {}",
                result.unwrap_err()
            );
        }
    }

    #[test]
    fn parse_message_start() {
        let line = r#"{"type":"stream_event","event":{"type":"message_start","message":{"model":"opus","id":"msg_123","type":"message","role":"assistant","content":[]}},"session_id":"abc"}"#;
        let event: StreamEvent = serde_json::from_str(line).unwrap();
        assert!(matches!(
            event,
            StreamEvent::StreamDelta {
                event: DeltaEvent::MessageStart { .. },
                ..
            }
        ));
    }

    #[test]
    fn parse_message_stop() {
        let line = r#"{"type":"stream_event","event":{"type":"message_stop"},"session_id":"abc"}"#;
        let event: StreamEvent = serde_json::from_str(line).unwrap();
        assert!(matches!(
            event,
            StreamEvent::StreamDelta {
                event: DeltaEvent::MessageStop {},
                ..
            }
        ));
    }

    #[test]
    fn parse_content_block_stop() {
        let line = r#"{"type":"stream_event","event":{"type":"content_block_stop","index":0},"session_id":"abc"}"#;
        let event: StreamEvent = serde_json::from_str(line).unwrap();
        assert!(matches!(
            event,
            StreamEvent::StreamDelta {
                event: DeltaEvent::ContentBlockStop { index: 0 },
                ..
            }
        ));
    }
}
