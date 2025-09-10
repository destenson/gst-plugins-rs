// Camera Configuration Loader
// Loads camera test configurations from TOML/JSON files

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfigFile {
    pub cameras: Vec<CameraConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    pub name: String,
    pub vendor: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware: Option<String>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default = "default_transport")]
    pub transport: String,
    #[serde(default = "default_auth")]
    pub auth_type: String,
    #[serde(default)]
    pub features: CameraFeaturesConfig,
    #[serde(default)]
    pub known_quirks: Vec<String>,
    #[serde(default)]
    pub test_categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CameraFeaturesConfig {
    #[serde(default)]
    pub h264: bool,
    #[serde(default)]
    pub h265: bool,
    #[serde(default)]
    pub mjpeg: bool,
    #[serde(default)]
    pub audio: bool,
    #[serde(default)]
    pub ptz: bool,
    #[serde(default)]
    pub onvif: bool,
    #[serde(default)]
    pub backchannel: bool,
    #[serde(default)]
    pub events: bool,
    #[serde(default)]
    pub seeking: bool,
}

fn default_transport() -> String {
    "auto".to_string()
}

fn default_auth() -> String {
    "none".to_string()
}

impl CameraConfigFile {
    pub fn load_from_toml<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let config: CameraConfigFile = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn load_from_json<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let config: CameraConfigFile = serde_json::from_str(&contents)?;
        Ok(config)
    }

    pub fn save_to_toml<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let contents = toml::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }

    pub fn save_to_json<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
}

pub fn create_example_config() -> CameraConfigFile {
    CameraConfigFile {
        cameras: vec![
            CameraConfig {
                name: "Axis M3045-V Test".to_string(),
                vendor: "Axis".to_string(),
                model: "M3045-V".to_string(),
                firmware: Some("9.80.1".to_string()),
                url: "rtsp://192.168.1.100/axis-media/media.amp".to_string(),
                username: Some("root".to_string()),
                password: Some("password".to_string()),
                transport: "auto".to_string(),
                auth_type: "digest".to_string(),
                features: CameraFeaturesConfig {
                    h264: true,
                    h265: true,
                    audio: true,
                    onvif: true,
                    ..Default::default()
                },
                known_quirks: vec![
                    "High default bitrate".to_string(),
                    "Requires digest authentication".to_string(),
                ],
                test_categories: vec![
                    "connectivity".to_string(),
                    "stream_formats".to_string(),
                    "performance".to_string(),
                ],
            },
            CameraConfig {
                name: "Hikvision DS-2CD2132F".to_string(),
                vendor: "Hikvision".to_string(),
                model: "DS-2CD2132F".to_string(),
                firmware: Some("V5.4.5".to_string()),
                url: "rtsp://192.168.1.101:554/Streaming/Channels/101".to_string(),
                username: Some("admin".to_string()),
                password: Some("admin123".to_string()),
                transport: "tcp".to_string(),
                auth_type: "digest".to_string(),
                features: CameraFeaturesConfig {
                    h264: true,
                    h265: true,
                    audio: true,
                    onvif: true,
                    ..Default::default()
                },
                known_quirks: vec![
                    "Channel 101 for main stream".to_string(),
                    "Channel 102 for sub stream".to_string(),
                    "Requires ONVIF account setup".to_string(),
                ],
                test_categories: vec![
                    "connectivity".to_string(),
                    "stream_formats".to_string(),
                    "onvif".to_string(),
                ],
            },
            CameraConfig {
                name: "Dahua IPC-HFW4431E".to_string(),
                vendor: "Dahua".to_string(),
                model: "IPC-HFW4431E".to_string(),
                firmware: Some("2.800.0000000.16.R".to_string()),
                url: "rtsp://192.168.1.102:554/cam/realmonitor?channel=1&subtype=0".to_string(),
                username: Some("admin".to_string()),
                password: Some("admin".to_string()),
                transport: "auto".to_string(),
                auth_type: "digest".to_string(),
                features: CameraFeaturesConfig {
                    h264: true,
                    audio: true,
                    onvif: true,
                    ..Default::default()
                },
                known_quirks: vec![
                    "subtype=0 for main stream".to_string(),
                    "subtype=1 for sub stream".to_string(),
                    "Port 8080 for ONVIF".to_string(),
                ],
                test_categories: vec![
                    "connectivity".to_string(),
                    "stream_formats".to_string(),
                    "reliability".to_string(),
                ],
            },
            CameraConfig {
                name: "Wowza Test Stream".to_string(),
                vendor: "Wowza".to_string(),
                model: "Test Server".to_string(),
                firmware: None,
                url: "rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2".to_string(),
                username: None,
                password: None,
                transport: "auto".to_string(),
                auth_type: "none".to_string(),
                features: CameraFeaturesConfig {
                    h264: true,
                    ..Default::default()
                },
                known_quirks: vec!["Static looping test video".to_string()],
                test_categories: vec![
                    "connectivity".to_string(),
                    "stream_formats".to_string(),
                ],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_save_and_load_toml() {
        let config = create_example_config();
        let path = "test_cameras.toml";
        
        // Save to TOML
        config.save_to_toml(path).unwrap();
        
        // Load from TOML
        let loaded = CameraConfigFile::load_from_toml(path).unwrap();
        
        assert_eq!(loaded.cameras.len(), config.cameras.len());
        assert_eq!(loaded.cameras[0].name, config.cameras[0].name);
        
        // Cleanup
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_save_and_load_json() {
        let config = create_example_config();
        let path = "test_cameras.json";
        
        // Save to JSON
        config.save_to_json(path).unwrap();
        
        // Load from JSON
        let loaded = CameraConfigFile::load_from_json(path).unwrap();
        
        assert_eq!(loaded.cameras.len(), config.cameras.len());
        assert_eq!(loaded.cameras[0].vendor, config.cameras[0].vendor);
        
        // Cleanup
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_generate_example_config_file() {
        let config = create_example_config();
        
        // Save example TOML config
        config.save_to_toml("camera_test_config_example.toml").unwrap();
        
        // Save example JSON config
        config.save_to_json("camera_test_config_example.json").unwrap();
        
        println!("Example configuration files generated:");
        println!("- camera_test_config_example.toml");
        println!("- camera_test_config_example.json");
    }
}