//! 会话事件总线
//!
//! 提供统一的运行时事件发布、批量追加与按会话回放能力。

use std::collections::HashMap;
use std::sync::RwLock;

use crate::models::execution::SessionEvent;

use super::{OrchestrationResult, SessionEventPublisher};

/// 会话事件总线
#[derive(Default)]
pub struct SessionEventBus {
    events: RwLock<HashMap<String, Vec<SessionEvent>>>,
}

impl SessionEventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&self, session_id: &str, event: SessionEvent) -> OrchestrationResult<()> {
        let mut events = self
            .events
            .write()
            .map_err(|_| super::OrchestrationError::EventBus(String::from("事件总线写锁失败")))?;
        events
            .entry(session_id.to_string())
            .or_default()
            .push(event);
        Ok(())
    }

    pub fn append_many(
        &self,
        session_id: &str,
        new_events: &[SessionEvent],
    ) -> OrchestrationResult<()> {
        let mut events = self
            .events
            .write()
            .map_err(|_| super::OrchestrationError::EventBus(String::from("事件总线写锁失败")))?;
        events
            .entry(session_id.to_string())
            .or_default()
            .extend(new_events.iter().cloned());
        Ok(())
    }

    pub fn replay(&self, session_id: &str) -> OrchestrationResult<Vec<SessionEvent>> {
        let events = self
            .events
            .read()
            .map_err(|_| super::OrchestrationError::EventBus(String::from("事件总线读锁失败")))?;
        Ok(events.get(session_id).cloned().unwrap_or_default())
    }
}

impl SessionEventPublisher for SessionEventBus {
    fn publish(&self, session_id: &str, events: &[SessionEvent]) -> OrchestrationResult<()> {
        self.append_many(session_id, events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::execution::SESSION_EVENT_VERSION;

    /// 验证流式事件顺序可回放
    #[test]
    fn test_session_event_replay_order_for_streaming() {
        let bus = SessionEventBus::new();
        bus.append(
            "session-1",
            SessionEvent::Start {
                version: SESSION_EVENT_VERSION,
                message_id: String::from("msg-1"),
            },
        )
        .unwrap();
        bus.append(
            "session-1",
            SessionEvent::Chunk {
                version: SESSION_EVENT_VERSION,
                content: String::from("你好"),
            },
        )
        .unwrap();
        bus.append(
            "session-1",
            SessionEvent::Complete {
                version: SESSION_EVENT_VERSION,
            },
        )
        .unwrap();

        let replay = bus.replay("session-1").unwrap();
        assert!(matches!(replay.first(), Some(SessionEvent::Start { .. })));
        assert!(matches!(replay.get(1), Some(SessionEvent::Chunk { .. })));
        assert!(matches!(replay.last(), Some(SessionEvent::Complete { .. })));
    }

    /// 验证搜索摘要与推理事件可同时写入总线
    #[test]
    fn test_session_event_replay_search_and_reasoning() {
        let bus = SessionEventBus::new();
        bus.publish(
            "session-2",
            &[
                SessionEvent::SearchSummary {
                    version: SESSION_EVENT_VERSION,
                    sources: vec![String::from("memory")],
                    evidence_count: 2,
                },
                SessionEvent::Reasoning {
                    version: SESSION_EVENT_VERSION,
                    summary: String::from("先看证据后给建议"),
                },
            ],
        )
        .unwrap();

        let replay = bus.replay("session-2").unwrap();
        assert_eq!(replay.len(), 2);
        assert!(matches!(replay[0], SessionEvent::SearchSummary { .. }));
        assert!(matches!(replay[1], SessionEvent::Reasoning { .. }));
    }

    /// 验证错误事件会进入回放路径
    #[test]
    fn test_session_event_replay_error_path() {
        let bus = SessionEventBus::new();
        bus.append(
            "session-3",
            SessionEvent::Error {
                version: SESSION_EVENT_VERSION,
                message: String::from("模型超时"),
            },
        )
        .unwrap();

        let replay = bus.replay("session-3").unwrap();
        assert!(matches!(replay.first(), Some(SessionEvent::Error { .. })));
    }
}
