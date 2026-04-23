use tokio::io::{AsyncBufRead, AsyncBufReadExt};

use super::types::StreamEvent;

pub struct EventStream<R> {
    reader: R,
    buf: String,
}

impl<R: AsyncBufRead + Unpin> EventStream<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buf: String::new(),
        }
    }

    pub async fn next_event(&mut self) -> Option<StreamEvent> {
        loop {
            self.buf.clear();
            let bytes_read = match self.reader.read_line(&mut self.buf).await {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("warning: stream read error: {e}");
                    return None;
                }
            };

            if bytes_read == 0 {
                return None;
            }

            let trimmed = self.buf.trim();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<StreamEvent>(trimmed) {
                Ok(event) => return Some(event),
                Err(e) => {
                    eprintln!("warning: skipping unparseable stream line: {e}");
                    continue;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncWriteExt, BufReader};

    async fn events_from_bytes(data: &[u8]) -> Vec<StreamEvent> {
        let (mut writer, reader) = tokio::io::duplex(data.len().max(1));
        writer.write_all(data).await.unwrap();
        drop(writer);

        let buf_reader = BufReader::new(reader);
        let mut stream = EventStream::new(buf_reader);
        let mut events = Vec::new();
        while let Some(event) = stream.next_event().await {
            events.push(event);
        }
        events
    }

    #[tokio::test]
    async fn parse_multiple_events() {
        let input = b"{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"abc\"}\n{\"type\":\"stream_event\",\"event\":{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}},\"session_id\":\"abc\"}\n";
        let events = events_from_bytes(input).await;
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], StreamEvent::System { .. }));
        assert!(matches!(events[1], StreamEvent::StreamDelta { .. }));
    }

    #[tokio::test]
    async fn skip_unparseable_lines() {
        let input =
            b"not json at all\n{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"abc\"}\n";
        let events = events_from_bytes(input).await;
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], StreamEvent::System { .. }));
    }

    #[tokio::test]
    async fn skip_empty_lines() {
        let input = b"\n\n{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"abc\"}\n\n";
        let events = events_from_bytes(input).await;
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], StreamEvent::System { .. }));
    }

    #[tokio::test]
    async fn empty_input_yields_none() {
        let events = events_from_bytes(b"").await;
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn full_session_stream() {
        let input = b"{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"s1\"}\n{\"type\":\"system\",\"subtype\":\"status\",\"status\":\"requesting\",\"session_id\":\"s1\"}\n{\"type\":\"rate_limit_event\",\"rate_limit_info\":{\"status\":\"allowed\"},\"session_id\":\"s1\"}\n{\"type\":\"stream_event\",\"event\":{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}},\"session_id\":\"s1\"}\n{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"hi\",\"total_cost_usd\":0.01,\"duration_ms\":100,\"num_turns\":1,\"session_id\":\"s1\"}\n";
        let events = events_from_bytes(input).await;
        assert_eq!(events.len(), 5);
        assert!(matches!(events[0], StreamEvent::System { .. }));
        assert!(matches!(events[1], StreamEvent::System { .. }));
        assert!(matches!(events[2], StreamEvent::RateLimit { .. }));
        assert!(matches!(events[3], StreamEvent::StreamDelta { .. }));
        assert!(matches!(events[4], StreamEvent::Completion { .. }));
    }
}
