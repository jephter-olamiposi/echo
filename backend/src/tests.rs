#[cfg(test)]
mod rate_limit_tests {
    use crate::state::SyncEngine;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn allows_first_message() {
        let engine = SyncEngine::default();
        assert!(engine.check_rate_limit("device_1"));
    }

    #[test]
    fn blocks_rapid_fire_messages() {
        let engine = SyncEngine::default();
        let device = "rapid_device";

        assert!(engine.check_rate_limit(device));
        assert!(!engine.check_rate_limit(device));
    }

    #[test]
    fn allows_message_after_interval() {
        let engine = SyncEngine::default();
        let device = "slow_device";

        assert!(engine.check_rate_limit(device));
        thread::sleep(Duration::from_millis(150));
        assert!(engine.check_rate_limit(device));
    }

    #[test]
    fn blocks_after_window_limit() {
        let engine = SyncEngine::default();
        let device = "spam_device";

        for i in 0..30 {
            {
                let mut entry = engine.rate_limits.entry(device.to_string()).or_default();
                entry.value_mut().last_message = None;
            }
            assert!(
                engine.check_rate_limit(device),
                "Message {} should be allowed",
                i + 1
            );
        }

        {
            let mut entry = engine.rate_limits.entry(device.to_string()).or_default();
            entry.value_mut().last_message = None;
        }
        assert!(
            !engine.check_rate_limit(device),
            "Message 31 should be blocked"
        );
    }

    #[test]
    fn different_devices_independent() {
        let engine = SyncEngine::default();

        assert!(engine.check_rate_limit("device_a"));
        assert!(engine.check_rate_limit("device_b"));
        assert!(!engine.check_rate_limit("device_a"));

        thread::sleep(Duration::from_millis(150));
        assert!(engine.check_rate_limit("device_b"));
    }
}

#[cfg(test)]
mod history_tests {
    use crate::models::ClipboardMessage;
    use crate::state::SyncEngine;
    use uuid::Uuid;

    #[test]
    fn adds_to_history() {
        let engine = SyncEngine::default();
        let user_id = Uuid::new_v4();

        engine.add_to_history(user_id, ClipboardMessage::new("device_1", "Hello"));

        let history = engine.get_history(&user_id);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content, "Hello");
    }

    #[test]
    fn history_newest_first() {
        let engine = SyncEngine::default();
        let user_id = Uuid::new_v4();

        engine.add_to_history(user_id, ClipboardMessage::new("d1", "First"));
        engine.add_to_history(user_id, ClipboardMessage::new("d1", "Second"));
        engine.add_to_history(user_id, ClipboardMessage::new("d1", "Third"));

        let history = engine.get_history(&user_id);
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].content, "Third");
        assert_eq!(history[1].content, "Second");
        assert_eq!(history[2].content, "First");
    }

    #[test]
    fn history_truncates_at_50() {
        let engine = SyncEngine::default();
        let user_id = Uuid::new_v4();

        for i in 0..60 {
            engine.add_to_history(user_id, ClipboardMessage::new("d1", format!("msg_{}", i)));
        }

        let history = engine.get_history(&user_id);
        assert_eq!(history.len(), 50);
        assert_eq!(history[0].content, "msg_59");
        assert_eq!(history[49].content, "msg_10");
    }

    #[test]
    fn empty_history_for_unknown_user() {
        let engine = SyncEngine::default();
        let unknown_user = Uuid::new_v4();

        assert!(engine.get_history(&unknown_user).is_empty());
    }

    #[test]
    fn users_have_separate_histories() {
        let engine = SyncEngine::default();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();

        engine.add_to_history(user_a, ClipboardMessage::new("d1", "User A message"));
        engine.add_to_history(user_b, ClipboardMessage::new("d2", "User B message"));

        let history_a = engine.get_history(&user_a);
        let history_b = engine.get_history(&user_b);

        assert_eq!(history_a.len(), 1);
        assert_eq!(history_b.len(), 1);
        assert_eq!(history_a[0].content, "User A message");
        assert_eq!(history_b[0].content, "User B message");
    }
}

#[cfg(test)]
mod models_tests {
    use crate::models::ClipboardMessage;

    #[test]
    fn clipboard_message_new_sets_defaults() {
        let msg = ClipboardMessage::new("device_123", "test content");

        assert_eq!(msg.device_id, "device_123");
        assert_eq!(msg.content, "test content");
        assert!(!msg.encrypted);
        assert!(msg.nonce.is_none());
        assert!(msg.timestamp > 0);
    }

    #[test]
    fn clipboard_message_accepts_string_and_str() {
        let msg1 = ClipboardMessage::new("device", "content");
        let msg2 = ClipboardMessage::new(String::from("device"), String::from("content"));

        assert_eq!(msg1.device_id, msg2.device_id);
        assert_eq!(msg1.content, msg2.content);
    }
}

#[cfg(test)]
mod channel_tests {
    use crate::state::SyncEngine;
    use uuid::Uuid;

    #[test]
    fn creates_new_channel() {
        let engine = SyncEngine::default();
        let user_id = Uuid::new_v4();

        assert!(!engine.hub.contains_key(&user_id));
        let _tx = engine.get_or_create_channel(user_id);
        assert!(engine.hub.contains_key(&user_id));
    }

    #[test]
    fn reuses_existing_channel() {
        let engine = SyncEngine::default();
        let user_id = Uuid::new_v4();

        let tx1 = engine.get_or_create_channel(user_id);
        let tx2 = engine.get_or_create_channel(user_id);

        assert_eq!(tx1.receiver_count(), tx2.receiver_count());
    }

    #[test]
    fn cleanup_removes_empty_channel() {
        let engine = SyncEngine::default();
        let user_id = Uuid::new_v4();

        let tx = engine.get_or_create_channel(user_id);
        assert!(engine.hub.contains_key(&user_id));

        engine.cleanup_channel_if_empty(&user_id, &tx);
        assert!(!engine.hub.contains_key(&user_id));
    }

    #[test]
    fn cleanup_keeps_channel_with_subscribers() {
        let engine = SyncEngine::default();
        let user_id = Uuid::new_v4();

        let tx = engine.get_or_create_channel(user_id);
        let _rx = tx.subscribe();

        engine.cleanup_channel_if_empty(&user_id, &tx);
        assert!(engine.hub.contains_key(&user_id));
    }
}
