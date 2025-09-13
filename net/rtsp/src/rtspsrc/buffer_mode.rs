
// Buffer mode enum (matching original rtspsrc)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferMode {
    None,   // Only use RTP timestamps
    Slave,  // Slave receiver to sender clock
    Buffer, // Do low/high watermark buffering
    Auto,   // Choose mode depending on stream live
    Synced, // Synchronized sender and receiver clocks
}

impl Default for BufferMode {
    fn default() -> Self {
        BufferMode::Auto // Matches DEFAULT_BUFFER_MODE (BUFFER_MODE_AUTO) from original
    }
}

impl BufferMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            BufferMode::None => "none",
            BufferMode::Slave => "slave",
            BufferMode::Buffer => "buffer",
            BufferMode::Auto => "auto",
            BufferMode::Synced => "synced",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "none" => Ok(BufferMode::None),
            "slave" => Ok(BufferMode::Slave),
            "buffer" => Ok(BufferMode::Buffer),
            "auto" => Ok(BufferMode::Auto),
            "synced" => Ok(BufferMode::Synced),
            _ => Err(format!("Invalid buffer mode: {}", s)),
        }
    }

    pub fn as_int(&self) -> u32 {
        match self {
            BufferMode::None => 0,
            BufferMode::Slave => 1,
            BufferMode::Buffer => 2,
            BufferMode::Auto => 3,
            BufferMode::Synced => 4,
        }
    }

    pub fn from_int(i: u32) -> Result<Self, String> {
        match i {
            0 => Ok(BufferMode::None),
            1 => Ok(BufferMode::Slave),
            2 => Ok(BufferMode::Buffer),
            3 => Ok(BufferMode::Auto),
            4 => Ok(BufferMode::Synced),
            _ => Err(format!("Invalid buffer mode value: {}", i)),
        }
    }
}
