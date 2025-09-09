use std::env;
use std::io;
use std::path::PathBuf;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct SdNotify {
    socket_path: PathBuf,
    #[cfg(unix)]
    socket: Option<std::os::unix::net::UnixDatagram>,
}

#[derive(Debug)]
pub enum NotifyState {
    Ready,
    Reloading,
    Stopping,
    Watchdog,
    Status(String),
    MainPid(u32),
    Errno(i32),
}

impl SdNotify {
    pub fn new() -> Result<Self, io::Error> {
        // Check if we're running under systemd
        let socket_path = match env::var("NOTIFY_SOCKET") {
            Ok(path) if !path.is_empty() => PathBuf::from(path),
            _ => return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "NOTIFY_SOCKET not set, not running under systemd"
            )),
        };
        
        // Create Unix datagram socket on Unix platforms
        #[cfg(unix)]
        let socket = Some(std::os::unix::net::UnixDatagram::unbound()?);
        
        Ok(Self {
            socket_path,
            #[cfg(unix)]
            socket,
        })
    }
    
    pub fn notify(&self, state: NotifyState) -> Result<(), io::Error> {
        let message = match state {
            NotifyState::Ready => "READY=1",
            NotifyState::Reloading => "RELOADING=1",
            NotifyState::Stopping => "STOPPING=1",
            NotifyState::Watchdog => "WATCHDOG=1",
            NotifyState::Status(ref status) => {
                return self.notify_with_message(&format!("STATUS={}", status));
            }
            NotifyState::MainPid(pid) => {
                return self.notify_with_message(&format!("MAINPID={}", pid));
            }
            NotifyState::Errno(errno) => {
                return self.notify_with_message(&format!("ERRNO={}", errno));
            }
        };
        
        self.notify_with_message(message)
    }
    
    fn notify_with_message(&self, message: &str) -> Result<(), io::Error> {
        #[cfg(unix)]
        {
            if let Some(ref socket) = self.socket {
                debug!("Sending sd_notify: {}", message);
                
                // Handle abstract socket paths (starting with @)
                let path = if self.socket_path.to_string_lossy().starts_with('@') {
                    // Abstract socket - replace @ with null byte
                    let path_str = self.socket_path.to_string_lossy();
                    let abstract_path = format!("\0{}", &path_str[1..]);
                    PathBuf::from(abstract_path)
                } else {
                    self.socket_path.clone()
                };
                
                match socket.send_to(message.as_bytes(), &path) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        debug!("Failed to send sd_notify message: {}", e);
                        Err(e)
                    }
                }
            } else {
                Ok(())
            }
        }
        
        #[cfg(not(unix))]
        {
            // On non-Unix systems, just log the message
            debug!("sd_notify (simulated): {}", message);
            Ok(())
        }
    }
    
    pub fn notify_with_fds(&self, state: NotifyState, _fds: &[i32]) -> Result<(), io::Error> {
        // This would require SCM_RIGHTS support for passing file descriptors
        // For now, just notify without FDs
        self.notify(state)
    }
    
    pub fn is_available(&self) -> bool {
        #[cfg(unix)]
        return self.socket.is_some();
        
        #[cfg(not(unix))]
        return false;
    }
    
    pub fn watchdog_enabled() -> bool {
        env::var("WATCHDOG_USEC").is_ok()
    }
    
    pub fn watchdog_timeout() -> Option<std::time::Duration> {
        if let Ok(usec_str) = env::var("WATCHDOG_USEC") {
            if let Ok(usec) = usec_str.parse::<u64>() {
                return Some(std::time::Duration::from_micros(usec));
            }
        }
        None
    }
}

impl Clone for NotifyState {
    fn clone(&self) -> Self {
        match self {
            NotifyState::Ready => NotifyState::Ready,
            NotifyState::Reloading => NotifyState::Reloading,
            NotifyState::Stopping => NotifyState::Stopping,
            NotifyState::Watchdog => NotifyState::Watchdog,
            NotifyState::Status(s) => NotifyState::Status(s.clone()),
            NotifyState::MainPid(pid) => NotifyState::MainPid(*pid),
            NotifyState::Errno(errno) => NotifyState::Errno(*errno),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_notify_state_formatting() {
        // Test that notify states format correctly
        let ready = NotifyState::Ready;
        let status = NotifyState::Status("Test status".to_string());
        let pid = NotifyState::MainPid(1234);
        
        // These should not panic
        format!("{:?}", ready);
        format!("{:?}", status);
        format!("{:?}", pid);
    }
    
    #[test]
    fn test_watchdog_timeout_parsing() {
        // Set test environment variable
        env::set_var("WATCHDOG_USEC", "30000000");
        
        let timeout = SdNotify::watchdog_timeout();
        assert!(timeout.is_some());
        assert_eq!(timeout.unwrap(), std::time::Duration::from_secs(30));
        
        // Clean up
        env::remove_var("WATCHDOG_USEC");
    }
    
    #[test]
    fn test_notify_without_systemd() {
        // When NOTIFY_SOCKET is not set, new() should fail
        env::remove_var("NOTIFY_SOCKET");
        let result = SdNotify::new();
        assert!(result.is_err());
    }
}