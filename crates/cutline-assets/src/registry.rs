use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetInfo {
    pub id: String,
    pub name: String,
    pub kind: String, // "template", "svg_animation", "image", "video_cutaway"
    pub description: String,
    pub default_anchor: String,
    pub default_duration_frames: i64,
}

pub struct AssetRegistry {
    assets: Vec<AssetInfo>,
}

impl Default for AssetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetRegistry {
    pub fn new() -> Self {
        let mut reg = Self { assets: Vec::new() };
        reg.register_builtins();
        reg
    }

    fn register_builtins(&mut self) {
        self.assets.push(AssetInfo {
            id: "template:lower-third".to_string(),
            name: "Lower Third Pill".to_string(),
            kind: "template".to_string(),
            description: "Studio-grade lower third with speaker name and subtitle tag".to_string(),
            default_anchor: "lower_third".to_string(),
            default_duration_frames: 90, // 3 seconds at 30fps
        });

        self.assets.push(AssetInfo {
            id: "template:keyword-badge".to_string(),
            name: "Keyword Badge".to_string(),
            kind: "template".to_string(),
            description: "High-contrast callout badge in upper safe area".to_string(),
            default_anchor: "top_center".to_string(),
            default_duration_frames: 60, // 2 seconds
        });

        self.assets.push(AssetInfo {
            id: "template:animated-progress".to_string(),
            name: "Progress Indicator Bar".to_string(),
            kind: "template".to_string(),
            description: "Subtle 6px playback progress bar along top safe margin".to_string(),
            default_anchor: "top_center".to_string(),
            default_duration_frames: 0, // Follows entire video duration
        });

        self.assets.push(AssetInfo {
            id: "template:punch-in-zoom".to_string(),
            name: "Punch-in Virtual Zoom".to_string(),
            kind: "template".to_string(),
            description: "Virtual camera zoom (1.08x) to emphasize key spoken moments".to_string(),
            default_anchor: "center".to_string(),
            default_duration_frames: 45, // 1.5 seconds
        });

        self.assets.push(AssetInfo {
            id: "template:camera-viewfinder".to_string(),
            name: "Camera Viewfinder Overlay".to_string(),
            kind: "template".to_string(),
            description: "Cinematic recording viewfinder with REC blinker, battery, and focus reticle".to_string(),
            default_anchor: "center".to_string(),
            default_duration_frames: 90,
        });

        self.assets.push(AssetInfo {
            id: "svg:bird-flying".to_string(),
            name: "Animated Flying Bird".to_string(),
            kind: "svg_animation".to_string(),
            description:
                "Vector animated bird with looping flapping wings proving dynamic illustration"
                    .to_string(),
            default_anchor: "top_center".to_string(),
            default_duration_frames: 75, // 2.5 seconds
        });
    }

    pub fn list(&self) -> &[AssetInfo] {
        &self.assets
    }

    pub fn find(&self, id: &str) -> Option<&AssetInfo> {
        self.assets
            .iter()
            .find(|a| a.id == id || a.id.split(':').next_back() == Some(id))
    }

    pub fn register(&mut self, asset: AssetInfo) {
        if let Some(pos) = self.assets.iter().position(|a| a.id == asset.id) {
            self.assets[pos] = asset;
        } else {
            self.assets.push(asset);
        }
    }
}
