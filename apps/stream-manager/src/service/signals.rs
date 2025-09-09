use tokio::signal;
use tokio::sync::mpsc;
use tracing::{info, error, debug};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalType {
    Terminate,
    Interrupt,
    Reload,
    User1,
    User2,
}

#[derive(Clone)]
pub struct SignalHandler {
    signal_tx: mpsc::UnboundedSender<SignalType>,
    signal_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<SignalType>>>,
}

impl SignalHandler {
    pub fn new() -> Self {
        let (signal_tx, signal_rx) = mpsc::unbounded_channel();
        
        let handler = Self {
            signal_tx: signal_tx.clone(),
            signal_rx: Arc::new(tokio::sync::Mutex::new(signal_rx)),
        };
        
        // Spawn signal listeners
        handler.spawn_signal_listeners();
        
        handler
    }
    
    fn spawn_signal_listeners(&self) {
        let tx = self.signal_tx.clone();
        
        // SIGTERM handler
        #[cfg(unix)]
        {
            let tx_term = tx.clone();
            tokio::spawn(async move {
                let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                    .expect("Failed to install SIGTERM handler");
                
                loop {
                    sigterm.recv().await;
                    debug!("Received SIGTERM");
                    let _ = tx_term.send(SignalType::Terminate);
                }
            });
        }
        
        // SIGINT handler (Ctrl+C)
        let tx_int = tx.clone();
        tokio::spawn(async move {
            loop {
                signal::ctrl_c().await.expect("Failed to install SIGINT handler");
                debug!("Received SIGINT");
                let _ = tx_int.send(SignalType::Interrupt);
            }
        });
        
        // SIGHUP handler (reload)
        #[cfg(unix)]
        {
            let tx_hup = tx.clone();
            tokio::spawn(async move {
                let mut sighup = signal::unix::signal(signal::unix::SignalKind::hangup())
                    .expect("Failed to install SIGHUP handler");
                
                loop {
                    sighup.recv().await;
                    debug!("Received SIGHUP");
                    let _ = tx_hup.send(SignalType::Reload);
                }
            });
        }
        
        // SIGUSR1 handler (status dump)
        #[cfg(unix)]
        {
            let tx_usr1 = tx.clone();
            tokio::spawn(async move {
                let mut sigusr1 = signal::unix::signal(signal::unix::SignalKind::user_defined1())
                    .expect("Failed to install SIGUSR1 handler");
                
                loop {
                    sigusr1.recv().await;
                    debug!("Received SIGUSR1");
                    let _ = tx_usr1.send(SignalType::User1);
                }
            });
        }
        
        // SIGUSR2 handler (log rotation)
        #[cfg(unix)]
        {
            let tx_usr2 = tx.clone();
            tokio::spawn(async move {
                let mut sigusr2 = signal::unix::signal(signal::unix::SignalKind::user_defined2())
                    .expect("Failed to install SIGUSR2 handler");
                
                loop {
                    sigusr2.recv().await;
                    debug!("Received SIGUSR2");
                    let _ = tx_usr2.send(SignalType::User2);
                }
            });
        }
    }
    
    pub async fn wait_for_signal(&self) -> SignalType {
        let mut rx = self.signal_rx.lock().await;
        rx.recv().await.unwrap_or(SignalType::Terminate)
    }
    
    pub fn try_recv(&self) -> Option<SignalType> {
        // Non-blocking receive for polling
        let mut rx = match self.signal_rx.try_lock() {
            Ok(rx) => rx,
            Err(_) => return None,
        };
        
        rx.try_recv().ok()
    }
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_signal_handler_creation() {
        let handler = SignalHandler::new();
        
        // Should be able to try receiving without blocking
        let signal = handler.try_recv();
        assert!(signal.is_none());
    }
    
    #[test]
    fn test_signal_type_equality() {
        assert_eq!(SignalType::Terminate, SignalType::Terminate);
        assert_ne!(SignalType::Terminate, SignalType::Interrupt);
        assert_ne!(SignalType::Reload, SignalType::User1);
    }
}