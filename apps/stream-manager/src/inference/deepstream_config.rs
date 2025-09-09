use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// DeepStream configuration parser and generator
/// Supports parsing and creating DeepStream config files

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepStreamConfig {
    pub primary_gie: PrimaryGieConfig,
    pub tracker: Option<TrackerConfig>,
    pub secondary_gies: Vec<SecondaryGieConfig>,
    pub osd: Option<OsdConfig>,
    pub tiler: Option<TilerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimaryGieConfig {
    pub enable: bool,
    pub gpu_id: u32,
    pub model_engine_file: Option<String>,
    pub config_file_path: String,
    pub batch_size: u32,
    pub interval: u32,
    pub gie_unique_id: u32,
    pub nvbuf_memory_type: u32, // 0: NVBUF_MEM_DEFAULT, 1: NVBUF_MEM_CUDA_PINNED, etc.
    pub cluster_mode: u32, // 0: Group rectangles, 1: DBSCAN, 2: NMS, 3: OpenCV grouping, 4: No clustering
    pub maintain_aspect_ratio: bool,
    pub symmetric_padding: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecondaryGieConfig {
    pub enable: bool,
    pub gpu_id: u32,
    pub model_engine_file: Option<String>,
    pub config_file_path: String,
    pub batch_size: u32,
    pub gie_unique_id: u32,
    pub operate_on_gie_id: u32,
    pub operate_on_class_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerConfig {
    pub enable: bool,
    pub tracker_width: u32,
    pub tracker_height: u32,
    pub ll_lib_file: String,
    pub ll_config_file: String,
    pub gpu_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsdConfig {
    pub enable: bool,
    pub gpu_id: u32,
    pub border_width: u32,
    pub text_size: u32,
    pub text_color: Color,
    pub text_bg_color: Color,
    pub font: String,
    pub show_clock: bool,
    pub clock_x: u32,
    pub clock_y: u32,
    pub clock_text_size: u32,
    pub clock_color: Color,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilerConfig {
    pub enable: bool,
    pub rows: u32,
    pub columns: u32,
    pub width: u32,
    pub height: u32,
    pub gpu_id: u32,
}

impl DeepStreamConfig {
    /// Load configuration from a DeepStream config file
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        // DeepStream configs are typically in INI format
        // For now, we'll create a default config
        // In production, you'd parse the actual INI file
        Ok(Self::default())
    }

    /// Generate a DeepStream config file
    pub fn to_file(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let config_str = self.generate_config_string();
        fs::write(path, config_str)?;
        Ok(())
    }

    /// Generate config string in DeepStream INI format
    fn generate_config_string(&self) -> String {
        let mut config = String::new();

        // Primary GIE section
        config.push_str("[primary-gie]\n");
        config.push_str(&format!("enable={}\n", if self.primary_gie.enable { 1 } else { 0 }));
        config.push_str(&format!("gpu-id={}\n", self.primary_gie.gpu_id));
        if let Some(ref engine) = self.primary_gie.model_engine_file {
            config.push_str(&format!("model-engine-file={}\n", engine));
        }
        config.push_str(&format!("config-file={}\n", self.primary_gie.config_file_path));
        config.push_str(&format!("batch-size={}\n", self.primary_gie.batch_size));
        config.push_str(&format!("interval={}\n", self.primary_gie.interval));
        config.push_str(&format!("gie-unique-id={}\n", self.primary_gie.gie_unique_id));
        config.push_str(&format!("nvbuf-memory-type={}\n", self.primary_gie.nvbuf_memory_type));
        config.push_str(&format!("cluster-mode={}\n", self.primary_gie.cluster_mode));
        config.push_str(&format!("maintain-aspect-ratio={}\n", if self.primary_gie.maintain_aspect_ratio { 1 } else { 0 }));
        config.push_str(&format!("symmetric-padding={}\n", if self.primary_gie.symmetric_padding { 1 } else { 0 }));
        config.push_str("\n");

        // Tracker section
        if let Some(ref tracker) = self.tracker {
            config.push_str("[tracker]\n");
            config.push_str(&format!("enable={}\n", if tracker.enable { 1 } else { 0 }));
            config.push_str(&format!("tracker-width={}\n", tracker.tracker_width));
            config.push_str(&format!("tracker-height={}\n", tracker.tracker_height));
            config.push_str(&format!("ll-lib-file={}\n", tracker.ll_lib_file));
            config.push_str(&format!("ll-config-file={}\n", tracker.ll_config_file));
            config.push_str(&format!("gpu-id={}\n", tracker.gpu_id));
            config.push_str("\n");
        }

        // Secondary GIE sections
        for (i, sgie) in self.secondary_gies.iter().enumerate() {
            config.push_str(&format!("[secondary-gie{}]\n", i));
            config.push_str(&format!("enable={}\n", if sgie.enable { 1 } else { 0 }));
            config.push_str(&format!("gpu-id={}\n", sgie.gpu_id));
            if let Some(ref engine) = sgie.model_engine_file {
                config.push_str(&format!("model-engine-file={}\n", engine));
            }
            config.push_str(&format!("config-file={}\n", sgie.config_file_path));
            config.push_str(&format!("batch-size={}\n", sgie.batch_size));
            config.push_str(&format!("gie-unique-id={}\n", sgie.gie_unique_id));
            config.push_str(&format!("operate-on-gie-id={}\n", sgie.operate_on_gie_id));
            let class_ids: Vec<String> = sgie.operate_on_class_ids.iter().map(|id| id.to_string()).collect();
            config.push_str(&format!("operate-on-class-ids={}\n", class_ids.join(";")));
            config.push_str("\n");
        }

        // OSD section
        if let Some(ref osd) = self.osd {
            config.push_str("[osd]\n");
            config.push_str(&format!("enable={}\n", if osd.enable { 1 } else { 0 }));
            config.push_str(&format!("gpu-id={}\n", osd.gpu_id));
            config.push_str(&format!("border-width={}\n", osd.border_width));
            config.push_str(&format!("text-size={}\n", osd.text_size));
            config.push_str(&format!("text-color={};{};{};{}\n", 
                osd.text_color.red, osd.text_color.green, 
                osd.text_color.blue, osd.text_color.alpha));
            config.push_str(&format!("text-bg-color={};{};{};{}\n",
                osd.text_bg_color.red, osd.text_bg_color.green,
                osd.text_bg_color.blue, osd.text_bg_color.alpha));
            config.push_str(&format!("font={}\n", osd.font));
            config.push_str(&format!("show-clock={}\n", if osd.show_clock { 1 } else { 0 }));
            config.push_str(&format!("clock-x-offset={}\n", osd.clock_x));
            config.push_str(&format!("clock-y-offset={}\n", osd.clock_y));
            config.push_str(&format!("clock-text-size={}\n", osd.clock_text_size));
            config.push_str(&format!("clock-color={};{};{};{}\n",
                osd.clock_color.red, osd.clock_color.green,
                osd.clock_color.blue, osd.clock_color.alpha));
            config.push_str("\n");
        }

        // Tiler section
        if let Some(ref tiler) = self.tiler {
            config.push_str("[tiler]\n");
            config.push_str(&format!("enable={}\n", if tiler.enable { 1 } else { 0 }));
            config.push_str(&format!("rows={}\n", tiler.rows));
            config.push_str(&format!("columns={}\n", tiler.columns));
            config.push_str(&format!("width={}\n", tiler.width));
            config.push_str(&format!("height={}\n", tiler.height));
            config.push_str(&format!("gpu-id={}\n", tiler.gpu_id));
            config.push_str("\n");
        }

        config
    }
}

impl Default for DeepStreamConfig {
    fn default() -> Self {
        Self {
            primary_gie: PrimaryGieConfig {
                enable: true,
                gpu_id: 0,
                model_engine_file: None,
                config_file_path: "/opt/nvidia/deepstream/samples/configs/deepstream-app/config_infer_primary.txt".to_string(),
                batch_size: 1,
                interval: 0,
                gie_unique_id: 1,
                nvbuf_memory_type: 0,
                cluster_mode: 2,
                maintain_aspect_ratio: true,
                symmetric_padding: true,
            },
            tracker: Some(TrackerConfig {
                enable: true,
                tracker_width: 640,
                tracker_height: 384,
                ll_lib_file: "/opt/nvidia/deepstream/lib/libnvds_nvmultiobjecttracker.so".to_string(),
                ll_config_file: "/opt/nvidia/deepstream/samples/configs/deepstream-app/config_tracker_NvDCF_perf.yml".to_string(),
                gpu_id: 0,
            }),
            secondary_gies: vec![],
            osd: Some(OsdConfig {
                enable: true,
                gpu_id: 0,
                border_width: 3,
                text_size: 15,
                text_color: Color { red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0 },
                text_bg_color: Color { red: 0.0, green: 0.0, blue: 0.0, alpha: 1.0 },
                font: "Serif".to_string(),
                show_clock: false,
                clock_x: 0,
                clock_y: 0,
                clock_text_size: 12,
                clock_color: Color { red: 1.0, green: 0.0, blue: 0.0, alpha: 1.0 },
            }),
            tiler: None,
        }
    }
}

/// Model configuration for nvinfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NvInferModelConfig {
    pub net_scale_factor: f32,
    pub offsets: Vec<f32>,
    pub model_color_format: String, // "RGB" or "BGR"
    pub custom_lib_path: Option<String>,
    pub parse_func: Option<String>,
    pub parse_bbox_func: Option<String>,
    pub parse_classifier_func: Option<String>,
    pub network_type: u32, // 0: Detector, 1: Classifier, 2: Segmentation, 3: Instance Segmentation
    pub num_detected_classes: u32,
    pub cluster_mode: u32,
    pub maintain_aspect_ratio: bool,
    pub input_blob_names: Vec<String>,
    pub output_blob_names: Vec<String>,
    pub output_bbox_layer_names: Option<Vec<String>>,
    pub output_mask_layer_names: Option<Vec<String>>,
    pub classifier_threshold: Option<f32>,
}

impl NvInferModelConfig {
    /// Generate config file content for nvinfer
    pub fn generate_config(&self) -> String {
        let mut config = String::new();

        config.push_str("[property]\n");
        config.push_str(&format!("net-scale-factor={}\n", self.net_scale_factor));
        
        let offsets_str = self.offsets.iter()
            .map(|o| o.to_string())
            .collect::<Vec<_>>()
            .join(";");
        config.push_str(&format!("offsets={}\n", offsets_str));
        
        config.push_str(&format!("model-color-format={}\n", self.model_color_format));
        
        if let Some(ref lib_path) = self.custom_lib_path {
            config.push_str(&format!("custom-lib-path={}\n", lib_path));
        }
        
        if let Some(ref func) = self.parse_func {
            config.push_str(&format!("parse-func={}\n", func));
        }
        
        if let Some(ref func) = self.parse_bbox_func {
            config.push_str(&format!("parse-bbox-func-name={}\n", func));
        }
        
        if let Some(ref func) = self.parse_classifier_func {
            config.push_str(&format!("parse-classifier-func-name={}\n", func));
        }
        
        config.push_str(&format!("network-type={}\n", self.network_type));
        config.push_str(&format!("num-detected-classes={}\n", self.num_detected_classes));
        config.push_str(&format!("cluster-mode={}\n", self.cluster_mode));
        config.push_str(&format!("maintain-aspect-ratio={}\n", if self.maintain_aspect_ratio { 1 } else { 0 }));
        
        for (i, name) in self.input_blob_names.iter().enumerate() {
            config.push_str(&format!("input-blob-names{}{}\n", if i > 0 { i.to_string() } else { "".to_string() }, name));
        }
        
        for (i, name) in self.output_blob_names.iter().enumerate() {
            config.push_str(&format!("output-blob-names{}{}\n", if i > 0 { i.to_string() } else { "".to_string() }, name));
        }
        
        if let Some(ref names) = self.output_bbox_layer_names {
            for name in names {
                config.push_str(&format!("output-bbox-layer-names={}\n", name));
            }
        }
        
        if let Some(ref names) = self.output_mask_layer_names {
            for name in names {
                config.push_str(&format!("output-mask-layer-names={}\n", name));
            }
        }
        
        if let Some(threshold) = self.classifier_threshold {
            config.push_str(&format!("classifier-threshold={}\n", threshold));
        }
        
        config
    }
}

/// Common DeepStream model types
pub enum ModelType {
    Yolo,
    SSD,
    FasterRCNN,
    RetinaNet,
    FRCNN,
    Custom(String),
}

impl ModelType {
    pub fn default_config(&self) -> NvInferModelConfig {
        match self {
            ModelType::Yolo => NvInferModelConfig {
                net_scale_factor: 0.00392157,
                offsets: vec![0.0, 0.0, 0.0],
                model_color_format: "RGB".to_string(),
                custom_lib_path: Some("/opt/nvidia/deepstream/lib/libnvdsinfer_custom_impl_Yolo.so".to_string()),
                parse_func: None,
                parse_bbox_func: Some("NvDsInferParseYolo".to_string()),
                parse_classifier_func: None,
                network_type: 0,
                num_detected_classes: 80,
                cluster_mode: 2,
                maintain_aspect_ratio: true,
                input_blob_names: vec!["data".to_string()],
                output_blob_names: vec!["prob".to_string()],
                output_bbox_layer_names: None,
                output_mask_layer_names: None,
                classifier_threshold: None,
            },
            ModelType::SSD => NvInferModelConfig {
                net_scale_factor: 1.0,
                offsets: vec![0.0, 0.0, 0.0],
                model_color_format: "RGB".to_string(),
                custom_lib_path: None,
                parse_func: None,
                parse_bbox_func: Some("NvDsInferParseSSD".to_string()),
                parse_classifier_func: None,
                network_type: 0,
                num_detected_classes: 91,
                cluster_mode: 1,
                maintain_aspect_ratio: false,
                input_blob_names: vec!["Input".to_string()],
                output_blob_names: vec!["NMS".to_string()],
                output_bbox_layer_names: None,
                output_mask_layer_names: None,
                classifier_threshold: None,
            },
            _ => NvInferModelConfig {
                net_scale_factor: 1.0,
                offsets: vec![0.0, 0.0, 0.0],
                model_color_format: "RGB".to_string(),
                custom_lib_path: None,
                parse_func: None,
                parse_bbox_func: None,
                parse_classifier_func: None,
                network_type: 0,
                num_detected_classes: 1,
                cluster_mode: 2,
                maintain_aspect_ratio: true,
                input_blob_names: vec!["input".to_string()],
                output_blob_names: vec!["output".to_string()],
                output_bbox_layer_names: None,
                output_mask_layer_names: None,
                classifier_threshold: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DeepStreamConfig::default();
        assert!(config.primary_gie.enable);
        assert_eq!(config.primary_gie.gpu_id, 0);
        assert_eq!(config.primary_gie.batch_size, 1);
    }

    #[test]
    fn test_config_generation() {
        let config = DeepStreamConfig::default();
        let config_str = config.generate_config_string();
        assert!(config_str.contains("[primary-gie]"));
        assert!(config_str.contains("enable=1"));
        assert!(config_str.contains("gpu-id=0"));
    }

    #[test]
    fn test_model_config() {
        let yolo_config = ModelType::Yolo.default_config();
        assert_eq!(yolo_config.network_type, 0);
        assert_eq!(yolo_config.num_detected_classes, 80);
        
        let config_str = yolo_config.generate_config();
        assert!(config_str.contains("network-type=0"));
        assert!(config_str.contains("num-detected-classes=80"));
    }

    #[test]
    fn test_color_serialization() {
        let color = Color {
            red: 1.0,
            green: 0.5,
            blue: 0.0,
            alpha: 1.0,
        };
        
        let json = serde_json::to_string(&color).unwrap();
        let deserialized: Color = serde_json::from_str(&json).unwrap();
        
        assert_eq!(color.red, deserialized.red);
        assert_eq!(color.green, deserialized.green);
        assert_eq!(color.blue, deserialized.blue);
        assert_eq!(color.alpha, deserialized.alpha);
    }
}