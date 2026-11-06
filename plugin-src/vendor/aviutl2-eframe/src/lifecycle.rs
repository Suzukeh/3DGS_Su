//! 終了要求を検出したイベントをeframeへ流さない。
//! SDK 0.43.0 はexit()後にも再描画を転送し、破棄済みHWNDへ触れていた。
pub(crate) fn should_forward_event(stop_requested: bool, request_stop: impl FnOnce()) -> bool {
    if stop_requested {
        request_stop();
        false
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::should_forward_event;
    #[test]
    fn shutdown_event_requests_exit_without_forwarding() {
        let mut exits = 0;
        let mut forwards = 0;
        if should_forward_event(true, || exits += 1) {
            forwards += 1;
        }
        assert_eq!(exits, 1);
        assert_eq!(forwards, 0);
    }
    #[test]
    fn running_event_keeps_forwarding() {
        let mut exits = 0;
        let mut forwards = 0;
        if should_forward_event(false, || exits += 1) {
            forwards += 1;
        }
        assert_eq!(exits, 0);
        assert_eq!(forwards, 1);
    }
}
