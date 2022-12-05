use tokio::sync::watch;
use tracing::info;

/// Builds a shutdown channel consisting of
/// a (['ShutdownNotifier'], ['ShutdownListener']) combination.
pub fn shutdown_channel() -> (ShutdownNotifier, ShutdownListener) {
    let (stop_tx, stop_rx) = watch::channel(false);
    (ShutdownNotifier::new(stop_tx), ShutdownListener::new(stop_rx))
}

/// Sends shutdown signal.
///
/// Shutdown is signalled using a ['watch::Sender<bool>']. Only a single shutdown signal
/// may be sent.
///
/// Based on https://github.com/tokio-rs/mini-redis/blob/master/src/shutdown.rs
#[derive(Debug)]
pub struct ShutdownNotifier {
    pub shutdown: bool,
    notifier: watch::Sender<bool>,
}

impl ShutdownNotifier {
    pub fn new(notifier: watch::Sender<bool>) -> Self {
        Self { shutdown: false, notifier }
    }

    pub fn send(&mut self) {
        if self.shutdown { return; }
        info!("ShutdownNotifier sending shutdown signal...");
        let _ = self.notifier.send(true);
        self.shutdown = true;
    }
}

/// Listens for shutdown signal.
///
/// Shutdown signal is received using a ['watch::Receiver<bool>']. Only a single shutdown
/// signal may be received.
///
/// Based on https://github.com/tokio-rs/mini-redis/blob/master/src/shutdown.rs
#[derive(Debug, Clone)]
pub struct ShutdownListener {
    pub shutdown: bool,  // false until shutdown signal is received
    listener: watch::Receiver<bool>,
}

impl ShutdownListener {
    pub fn new(listener: watch::Receiver<bool>) -> Self {
        Self { shutdown: false, listener }
    }

    pub async fn recv(&mut self) {
        // if shutdown signal has been sent before, return immediately.
        if self.shutdown { return; }
        let _ = self.listener.changed().await;
        self.shutdown = true;
    }
}