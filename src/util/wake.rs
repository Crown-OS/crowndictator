//! The one road from the worker threads back onto the UI thread. A wake that
//! lands while nobody is waiting is kept, so none is lost between two frames.

use std::sync::Arc;

use tokio::sync::Notify;

#[derive(Clone, Default)]
pub struct Wake(Arc<Notify>);

impl Wake {
    pub fn wake(&self) {
        self.0.notify_one();
    }

    /// Resolves at the next wake, or at once if one is already pending.
    pub async fn woken(self) {
        self.0.notified().await;
    }
}
