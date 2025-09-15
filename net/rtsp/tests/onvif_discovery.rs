#![allow(unused)]
// ONVIF Discovery Module
// Discovers ONVIF-compliant IP cameras on the network

use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::Duration;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct OnvifDevice {
    pub name: String,
    pub manufacturer: String,
    pub model: String,
    pub firmware: Option<String>,
    pub serial_number: Option<String>,
    pub hardware_id: Option<String>,
    pub ip_address: IpAddr,
    pub onvif_port: u16,
    pub rtsp_url: Option<String>,
    pub profiles: Vec<OnvifProfile>,
    pub capabilities: OnvifCapabilities,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct OnvifProfile {
    pub name: String,
    pub token: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct OnvifCapabilities {
    pub analytics: bool,
    pub device: bool,
    pub events: bool,
    pub imaging: bool,
    pub media: bool,
    pub ptz: bool,
}

impl Default for OnvifCapabilities {
    fn default() -> Self {
        Self {
            analytics: false,
            device: true,
            events: false,
            imaging: false,
            media: true,
            ptz: false,
        }
    }
}

#[allow(dead_code)]
pub struct OnvifDiscovery {
    multicast_addr: SocketAddr,
    timeout: Duration,
}

impl OnvifDiscovery {
    pub fn new() -> Self {
        Self {
            // WS-Discovery multicast address
            multicast_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(239, 255, 255, 250)), 3702),
            timeout: Duration::from_secs(5),
        }
    }

    pub fn discover_devices(&self) -> Result<Vec<OnvifDevice>, Box<dyn std::error::Error>> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_read_timeout(Some(self.timeout))?;

        // Send WS-Discovery probe message
        let probe_message = self.create_probe_message();
        socket.send_to(probe_message.as_bytes(), self.multicast_addr)?;

        // Collect responses
        let mut devices = Vec::new();
        let mut buffer = [0u8; 4096];

        loop {
            match socket.recv_from(&mut buffer) {
                Ok((size, addr)) => {
                    if let Some(device) = self.parse_probe_response(&buffer[..size], addr) {
                        devices.push(device);
                    }
                }
                Err(_) => break, // Timeout reached
            }
        }

        Ok(devices)
    }

    fn create_probe_message(&self) -> String {
        // Simplified WS-Discovery probe message
        // In production, use proper SOAP/XML library
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <soap:Envelope 
            xmlns:soap="http://www.w3.org/2003/05/soap-envelope"
            xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing"
            xmlns:wsd="http://schemas.xmlsoap.org/ws/2005/04/discovery"
            xmlns:dn="http://www.onvif.org/ver10/network/wsdl">
            <soap:Header>
                <wsa:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe</wsa:Action>
                <wsa:MessageID>urn:uuid:probe-message-id</wsa:MessageID>
                <wsa:To>urn:schemas-xmlsoap-org:ws:2005:04:discovery</wsa:To>
            </soap:Header>
            <soap:Body>
                <wsd:Probe>
                    <wsd:Types>dn:NetworkVideoTransmitter</wsd:Types>
                </wsd:Probe>
            </soap:Body>
        </soap:Envelope>"#
            .to_string()
    }

    fn parse_probe_response(&self, _data: &[u8], addr: SocketAddr) -> Option<OnvifDevice> {
        // Simplified parsing - in production, use proper XML parser
        // For now, return a mock device for demonstration

        // This would normally parse the SOAP response
        Some(OnvifDevice {
            name: format!("Camera at {}", addr.ip()),
            manufacturer: "Unknown".to_string(),
            model: "Unknown".to_string(),
            firmware: None,
            serial_number: None,
            hardware_id: None,
            ip_address: addr.ip(),
            onvif_port: 80, // Default ONVIF port
            rtsp_url: Some(format!("rtsp://{}:554/", addr.ip())),
            profiles: vec![OnvifProfile {
                name: "Profile_1".to_string(),
                token: "profile_1_h264".to_string(),
            }],
            capabilities: OnvifCapabilities::default(),
        })
    }

    pub fn get_device_info(
        &self,
        device: &OnvifDevice,
    ) -> Result<OnvifDevice, Box<dyn std::error::Error>> {
        // Would send GetDeviceInformation SOAP request
        // For now, return the device as-is
        Ok(device.clone())
    }

    pub fn get_rtsp_url(
        &self,
        device: &OnvifDevice,
        profile: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Would send GetStreamUri SOAP request
        // For now, construct a generic RTSP URL
        Ok(format!(
            "rtsp://{}:554/onvif-media/media.amp?profile={}",
            device.ip_address, profile
        ))
    }
}

// Simulator for testing without real cameras
#[allow(dead_code)]
pub struct OnvifSimulator {
    devices: Vec<OnvifDevice>,
}

impl OnvifSimulator {
    pub fn new() -> Self {
        Self {
            devices: Self::create_simulated_devices(),
        }
    }

    fn create_simulated_devices() -> Vec<OnvifDevice> {
        vec![
            OnvifDevice {
                name: "Simulated Axis Camera".to_string(),
                manufacturer: "Axis".to_string(),
                model: "M3045-V".to_string(),
                firmware: Some("9.80.1".to_string()),
                serial_number: Some("ACCC8E123456".to_string()),
                hardware_id: Some("1234567890".to_string()),
                ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                onvif_port: 80,
                rtsp_url: Some("rtsp://192.168.1.100/axis-media/media.amp".to_string()),
                profiles: vec![
                    OnvifProfile {
                        name: "Profile_1_h264".to_string(),
                        token: "profile_1_h264".to_string(),
                    },
                    OnvifProfile {
                        name: "Profile_2_h265".to_string(),
                        token: "profile_2_h265".to_string(),
                    },
                ],
                capabilities: OnvifCapabilities {
                    analytics: true,
                    device: true,
                    events: true,
                    imaging: true,
                    media: true,
                    ptz: false,
                },
            },
            OnvifDevice {
                name: "Simulated Hikvision Camera".to_string(),
                manufacturer: "Hikvision".to_string(),
                model: "DS-2CD2132F".to_string(),
                firmware: Some("V5.4.5".to_string()),
                serial_number: Some("DS-2CD2132F20170101AACH123456789".to_string()),
                hardware_id: Some("0987654321".to_string()),
                ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 101)),
                onvif_port: 80,
                rtsp_url: Some("rtsp://192.168.1.101:554/Streaming/Channels/101".to_string()),
                profiles: vec![
                    OnvifProfile {
                        name: "MainStream".to_string(),
                        token: "Profile_1".to_string(),
                    },
                    OnvifProfile {
                        name: "SubStream".to_string(),
                        token: "Profile_2".to_string(),
                    },
                ],
                capabilities: OnvifCapabilities {
                    analytics: false,
                    device: true,
                    events: true,
                    imaging: true,
                    media: true,
                    ptz: true,
                },
            },
            OnvifDevice {
                name: "Simulated Dahua Camera".to_string(),
                manufacturer: "Dahua".to_string(),
                model: "IPC-HFW4431E".to_string(),
                firmware: Some("2.800.0000000.16.R".to_string()),
                serial_number: Some("6G07D3DPAG00001".to_string()),
                hardware_id: Some("1122334455".to_string()),
                ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 102)),
                onvif_port: 8080, // Dahua often uses 8080 for ONVIF
                rtsp_url: Some(
                    "rtsp://192.168.1.102:554/cam/realmonitor?channel=1&subtype=0".to_string(),
                ),
                profiles: vec![
                    OnvifProfile {
                        name: "Main".to_string(),
                        token: "000".to_string(),
                    },
                    OnvifProfile {
                        name: "Sub".to_string(),
                        token: "001".to_string(),
                    },
                ],
                capabilities: OnvifCapabilities {
                    analytics: false,
                    device: true,
                    events: false,
                    imaging: true,
                    media: true,
                    ptz: false,
                },
            },
        ]
    }

    pub fn discover_devices(&self) -> Vec<OnvifDevice> {
        self.devices.clone()
    }

    pub fn get_device_by_ip(&self, ip: &IpAddr) -> Option<OnvifDevice> {
        self.devices.iter().find(|d| &d.ip_address == ip).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onvif_discovery_creation() {
        let discovery = OnvifDiscovery::new();
        assert_eq!(discovery.timeout, Duration::from_secs(5));
    }

    #[test]
    fn test_probe_message_generation() {
        let discovery = OnvifDiscovery::new();
        let message = discovery.create_probe_message();
        assert!(message.contains("Probe"));
        assert!(message.contains("NetworkVideoTransmitter"));
    }

    #[test]
    fn test_simulated_devices() {
        let simulator = OnvifSimulator::new();
        let devices = simulator.discover_devices();

        assert_eq!(devices.len(), 3);

        // Check Axis device
        let axis = &devices[0];
        assert_eq!(axis.manufacturer, "Axis");
        assert_eq!(axis.model, "M3045-V");
        assert_eq!(axis.profiles.len(), 2);

        // Check Hikvision device
        let hik = &devices[1];
        assert_eq!(hik.manufacturer, "Hikvision");
        assert!(hik.capabilities.ptz);

        // Check Dahua device
        let dahua = &devices[2];
        assert_eq!(dahua.manufacturer, "Dahua");
        assert_eq!(dahua.onvif_port, 8080);
    }

    #[test]
    fn test_get_device_by_ip() {
        let simulator = OnvifSimulator::new();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));

        let device = simulator.get_device_by_ip(&ip);
        assert!(device.is_some());

        let device = device.unwrap();
        assert_eq!(device.manufacturer, "Axis");
    }

    #[test]
    #[ignore] // Only run when testing with real network
    fn test_real_discovery() {
        let discovery = OnvifDiscovery::new();
        match discovery.discover_devices() {
            Ok(devices) => {
                println!("Found {} ONVIF devices", devices.len());
                for device in devices {
                    println!("Device: {:?}", device);
                }
            }
            Err(e) => {
                println!("Discovery failed: {}", e);
            }
        }
    }
}
