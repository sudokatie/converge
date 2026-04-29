//! Overlay marker types for diagnostic visualization.

use serde::{Deserialize, Serialize};

use super::channel::DiagnosticChannel;
use super::color::DiagnosticColor;

/// Kind of overlay marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarkerKind {
    Point,
    Arrow,
    Box,
    Sphere,
    Line,
}

/// A single overlay marker with position, kind, and style.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OverlayMarker {
    pub world_pos: [i32; 3],
    pub kind: MarkerKind,
    pub channel: DiagnosticChannel,
    pub color: DiagnosticColor,
    pub scale: f32,
    pub label: Option<String>,
}

impl OverlayMarker {
    #[must_use]
    pub fn new(world_pos: [i32; 3], kind: MarkerKind, channel: DiagnosticChannel) -> Self {
        Self {
            world_pos,
            kind,
            channel,
            color: DiagnosticColor::WHITE,
            scale: 1.0,
            label: None,
        }
    }

    #[must_use]
    pub fn with_color(mut self, color: DiagnosticColor) -> Self {
        self.color = color;
        self
    }

    #[must_use]
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Specification for an overlay layer.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OverlaySpec {
    pub markers: Vec<OverlayMarker>,
    pub enabled: bool,
    pub opacity: f32,
}

impl OverlaySpec {
    #[must_use]
    pub fn new() -> Self {
        Self {
            markers: Vec::new(),
            enabled: true,
            opacity: 1.0,
        }
    }

    pub fn push(&mut self, marker: OverlayMarker) {
        self.markers.push(marker);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.markers.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.markers.is_empty()
    }

    pub fn clear(&mut self) {
        self.markers.clear();
    }
}
