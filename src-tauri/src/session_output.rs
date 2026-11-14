//! Switchable IPC channel. Buffering during handoff is bounded by the existing SSH credit window.

use std::sync::{Arc, Mutex, PoisonError};

use tauri::ipc::{Channel, IpcResponse};

pub(crate) struct Relay<T>(Arc<Mutex<Route<T>>>);

struct Route<T> {
    channel: Channel<T>,
    pending: Option<Vec<T>>,
}

impl<T> Clone for Relay<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: IpcResponse> Relay<T> {
    pub(crate) fn new(channel: Channel<T>) -> Self {
        Self(Arc::new(Mutex::new(Route {
            channel,
            pending: None,
        })))
    }

    pub(crate) fn send(&self, value: T) -> tauri::Result<()> {
        let mut route = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(pending) = &mut route.pending {
            pending.push(value);
            Ok(())
        } else {
            route.channel.send(value)
        }
    }

    pub(crate) fn pause(&self) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pending
            .get_or_insert_with(Vec::new);
    }

    pub(crate) fn pause_after(&self, marker: T) -> tauri::Result<()> {
        let mut route = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        route.channel.send(marker)?;
        route.pending.get_or_insert_with(Vec::new);
        Ok(())
    }

    pub(crate) fn resume(&self, channel: Option<Channel<T>>) {
        let mut route = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(channel) = channel {
            route.channel = channel;
        }
        for value in route.pending.take().unwrap_or_default() {
            let _ = route.channel.send(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::stream::RawBytes;
    use tauri::ipc::InvokeResponseBody;

    fn record(target: &Arc<Mutex<Vec<Vec<u8>>>>) -> Channel<RawBytes> {
        let target = target.clone();
        Channel::new(move |body| {
            if let InvokeResponseBody::Raw(bytes) = body {
                target.lock().unwrap().push(bytes);
            }
            Ok(())
        })
    }

    #[test]
    fn delivers_marker_then_moves_buffered_output_without_loss_or_duplication() {
        let source = Arc::new(Mutex::new(Vec::new()));
        let destination = Arc::new(Mutex::new(Vec::new()));
        let relay = Relay::new(record(&source));
        relay.send(RawBytes(vec![1])).unwrap();
        relay.pause_after(RawBytes(vec![])).unwrap();
        relay.send(RawBytes(vec![2])).unwrap();
        assert_eq!(*source.lock().unwrap(), vec![vec![1], vec![]]);
        relay.resume(Some(record(&destination)));
        relay.send(RawBytes(vec![3])).unwrap();
        assert_eq!(*destination.lock().unwrap(), vec![vec![2], vec![3]]);
        relay.pause_after(RawBytes(vec![])).unwrap();
        relay.send(RawBytes(vec![4])).unwrap();
        relay.resume(None);
        assert_eq!(
            *destination.lock().unwrap(),
            vec![vec![2], vec![3], vec![], vec![4]]
        );
    }
}
