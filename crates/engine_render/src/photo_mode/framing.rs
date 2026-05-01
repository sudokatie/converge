//! Shot framing and composition helpers.
//!
//! Provides composition guides, framing calculations, and
//! subject positioning utilities for photo mode.

use glam::{Mat4, Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};

/// Composition guide overlay types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum CompositionGuide {
    /// No overlay.
    #[default]
    None = 0,
    /// Rule of thirds grid.
    RuleOfThirds = 1,
    /// Golden ratio grid.
    GoldenRatio = 2,
    /// Center crosshair.
    Center = 3,
    /// Diagonal lines.
    Diagonals = 4,
    /// Golden spiral (Fibonacci).
    GoldenSpiral = 5,
    /// Symmetry guide.
    Symmetry = 6,
    /// Safe area (broadcast safe).
    SafeArea = 7,
}

impl CompositionGuide {
    /// All available composition guides.
    pub const ALL: [CompositionGuide; 8] = [
        CompositionGuide::None,
        CompositionGuide::RuleOfThirds,
        CompositionGuide::GoldenRatio,
        CompositionGuide::Center,
        CompositionGuide::Diagonals,
        CompositionGuide::GoldenSpiral,
        CompositionGuide::Symmetry,
        CompositionGuide::SafeArea,
    ];

    /// Get guide name for display.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            CompositionGuide::None => "None",
            CompositionGuide::RuleOfThirds => "Rule of Thirds",
            CompositionGuide::GoldenRatio => "Golden Ratio",
            CompositionGuide::Center => "Center",
            CompositionGuide::Diagonals => "Diagonals",
            CompositionGuide::GoldenSpiral => "Golden Spiral",
            CompositionGuide::Symmetry => "Symmetry",
            CompositionGuide::SafeArea => "Safe Area",
        }
    }

    /// Get the guide lines in normalized screen space (0-1).
    #[must_use]
    pub fn guide_lines(&self) -> Vec<(Vec2, Vec2)> {
        match self {
            CompositionGuide::None => vec![],
            CompositionGuide::RuleOfThirds => rule_of_thirds_lines(),
            CompositionGuide::GoldenRatio => golden_ratio_lines(),
            CompositionGuide::Center => center_lines(),
            CompositionGuide::Diagonals => diagonal_lines(),
            CompositionGuide::GoldenSpiral => golden_spiral_lines(),
            CompositionGuide::Symmetry => symmetry_lines(),
            CompositionGuide::SafeArea => safe_area_lines(),
        }
    }

    /// Get power points (intersection points of interest).
    #[must_use]
    pub fn power_points(&self) -> Vec<Vec2> {
        match self {
            CompositionGuide::RuleOfThirds => rule_of_thirds_points(),
            CompositionGuide::GoldenRatio => golden_ratio_points(),
            CompositionGuide::Center => vec![Vec2::splat(0.5)],
            _ => vec![],
        }
    }
}

fn rule_of_thirds_lines() -> Vec<(Vec2, Vec2)> {
    vec![
        (Vec2::new(1.0 / 3.0, 0.0), Vec2::new(1.0 / 3.0, 1.0)),
        (Vec2::new(2.0 / 3.0, 0.0), Vec2::new(2.0 / 3.0, 1.0)),
        (Vec2::new(0.0, 1.0 / 3.0), Vec2::new(1.0, 1.0 / 3.0)),
        (Vec2::new(0.0, 2.0 / 3.0), Vec2::new(1.0, 2.0 / 3.0)),
    ]
}

fn rule_of_thirds_points() -> Vec<Vec2> {
    vec![
        Vec2::new(1.0 / 3.0, 1.0 / 3.0),
        Vec2::new(2.0 / 3.0, 1.0 / 3.0),
        Vec2::new(1.0 / 3.0, 2.0 / 3.0),
        Vec2::new(2.0 / 3.0, 2.0 / 3.0),
    ]
}

const PHI: f32 = 1.618_034;

fn golden_ratio_lines() -> Vec<(Vec2, Vec2)> {
    let phi_inv = 1.0 / PHI;
    vec![
        (Vec2::new(phi_inv, 0.0), Vec2::new(phi_inv, 1.0)),
        (Vec2::new(1.0 - phi_inv, 0.0), Vec2::new(1.0 - phi_inv, 1.0)),
        (Vec2::new(0.0, phi_inv), Vec2::new(1.0, phi_inv)),
        (Vec2::new(0.0, 1.0 - phi_inv), Vec2::new(1.0, 1.0 - phi_inv)),
    ]
}

fn golden_ratio_points() -> Vec<Vec2> {
    let phi_inv = 1.0 / PHI;
    vec![
        Vec2::new(phi_inv, phi_inv),
        Vec2::new(1.0 - phi_inv, phi_inv),
        Vec2::new(phi_inv, 1.0 - phi_inv),
        Vec2::new(1.0 - phi_inv, 1.0 - phi_inv),
    ]
}

fn center_lines() -> Vec<(Vec2, Vec2)> {
    vec![
        (Vec2::new(0.5, 0.0), Vec2::new(0.5, 1.0)),
        (Vec2::new(0.0, 0.5), Vec2::new(1.0, 0.5)),
    ]
}

fn diagonal_lines() -> Vec<(Vec2, Vec2)> {
    vec![
        (Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0)),
        (Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)),
    ]
}

fn golden_spiral_lines() -> Vec<(Vec2, Vec2)> {
    let mut lines = Vec::new();
    let phi_inv = 1.0 / PHI;

    let mut x = 0.0;
    let mut y = 0.0;
    let mut w = 1.0;
    let mut h = 1.0;

    for i in 0..6 {
        match i % 4 {
            0 => {
                lines.push((
                    Vec2::new(x + w * phi_inv, y),
                    Vec2::new(x + w * phi_inv, y + h),
                ));
                w *= phi_inv;
            }
            1 => {
                lines.push((
                    Vec2::new(x, y + h * phi_inv),
                    Vec2::new(x + w, y + h * phi_inv),
                ));
                let old_h = h;
                h *= phi_inv;
                y += old_h - h;
            }
            2 => {
                lines.push((
                    Vec2::new(x + w * (1.0 - phi_inv), y),
                    Vec2::new(x + w * (1.0 - phi_inv), y + h),
                ));
                let old_w = w;
                w *= phi_inv;
                x += old_w - w;
            }
            3 => {
                lines.push((
                    Vec2::new(x, y + h * (1.0 - phi_inv)),
                    Vec2::new(x + w, y + h * (1.0 - phi_inv)),
                ));
                h *= phi_inv;
            }
            _ => unreachable!(),
        }
    }

    lines
}

fn symmetry_lines() -> Vec<(Vec2, Vec2)> {
    vec![(Vec2::new(0.5, 0.0), Vec2::new(0.5, 1.0))]
}

fn safe_area_lines() -> Vec<(Vec2, Vec2)> {
    let action_margin = 0.05;
    let title_margin = 0.10;

    vec![
        (
            Vec2::new(action_margin, action_margin),
            Vec2::new(1.0 - action_margin, action_margin),
        ),
        (
            Vec2::new(1.0 - action_margin, action_margin),
            Vec2::new(1.0 - action_margin, 1.0 - action_margin),
        ),
        (
            Vec2::new(1.0 - action_margin, 1.0 - action_margin),
            Vec2::new(action_margin, 1.0 - action_margin),
        ),
        (
            Vec2::new(action_margin, 1.0 - action_margin),
            Vec2::new(action_margin, action_margin),
        ),
        (
            Vec2::new(title_margin, title_margin),
            Vec2::new(1.0 - title_margin, title_margin),
        ),
        (
            Vec2::new(1.0 - title_margin, title_margin),
            Vec2::new(1.0 - title_margin, 1.0 - title_margin),
        ),
        (
            Vec2::new(1.0 - title_margin, 1.0 - title_margin),
            Vec2::new(title_margin, 1.0 - title_margin),
        ),
        (
            Vec2::new(title_margin, 1.0 - title_margin),
            Vec2::new(title_margin, title_margin),
        ),
    ]
}

/// Shot type based on framing distance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ShotType {
    /// Extreme close-up (face detail).
    ExtremeCloseUp = 0,
    /// Close-up (face).
    CloseUp = 1,
    /// Medium close-up (head and shoulders).
    MediumCloseUp = 2,
    /// Medium shot (waist up).
    #[default]
    Medium = 3,
    /// Medium long shot (knees up).
    MediumLong = 4,
    /// Long shot (full body).
    Long = 5,
    /// Extreme long shot (environment).
    ExtremeLong = 6,
}

impl ShotType {
    /// Approximate screen coverage for this shot type (0.0 to 1.0).
    #[must_use]
    pub const fn screen_coverage(&self) -> f32 {
        match self {
            ShotType::ExtremeCloseUp => 0.8,
            ShotType::CloseUp => 0.6,
            ShotType::MediumCloseUp => 0.4,
            ShotType::Medium => 0.3,
            ShotType::MediumLong => 0.2,
            ShotType::Long => 0.15,
            ShotType::ExtremeLong => 0.05,
        }
    }

    /// Get shot type name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            ShotType::ExtremeCloseUp => "Extreme Close-Up",
            ShotType::CloseUp => "Close-Up",
            ShotType::MediumCloseUp => "Medium Close-Up",
            ShotType::Medium => "Medium",
            ShotType::MediumLong => "Medium Long",
            ShotType::Long => "Long",
            ShotType::ExtremeLong => "Extreme Long",
        }
    }
}

/// Subject framing information.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SubjectFraming {
    /// Subject world position.
    pub position: Vec3,
    /// Subject bounding sphere radius.
    pub radius: f32,
    /// Subject screen-space center (normalized).
    pub screen_center: Vec2,
    /// Subject screen-space size (normalized).
    pub screen_size: Vec2,
    /// Distance from camera.
    pub distance: f32,
    /// Current shot type.
    pub shot_type: ShotType,
    /// Whether subject is in frame.
    pub in_frame: bool,
}

impl SubjectFraming {
    /// Create new subject framing.
    #[must_use]
    pub fn new(position: Vec3, radius: f32) -> Self {
        Self {
            position,
            radius,
            ..Default::default()
        }
    }

    /// Calculate framing from camera matrices.
    #[must_use]
    pub fn calculate(position: Vec3, radius: f32, view_proj: Mat4, _screen_size: Vec2) -> Self {
        let clip_pos = view_proj * Vec4::new(position.x, position.y, position.z, 1.0);

        if clip_pos.w <= 0.0 {
            return Self {
                position,
                radius,
                in_frame: false,
                ..Default::default()
            };
        }

        let ndc = clip_pos.truncate() / clip_pos.w;
        let screen_center = Vec2::new((ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5);

        let edge_offset = Vec4::new(position.x + radius, position.y, position.z, 1.0);
        let edge_clip = view_proj * edge_offset;
        let edge_ndc = if edge_clip.w > 0.0 {
            edge_clip.truncate() / edge_clip.w
        } else {
            ndc
        };

        let screen_radius = ((edge_ndc.x - ndc.x).abs() + (edge_ndc.y - ndc.y).abs()) * 0.5;
        let screen_size_val = Vec2::splat(screen_radius * 2.0);

        let in_frame = screen_center.x >= 0.0
            && screen_center.x <= 1.0
            && screen_center.y >= 0.0
            && screen_center.y <= 1.0
            && ndc.z >= -1.0
            && ndc.z <= 1.0;

        let coverage = screen_radius.max(screen_size_val.x).max(screen_size_val.y);
        let shot_type = shot_type_from_coverage(coverage);

        Self {
            position,
            radius,
            screen_center,
            screen_size: screen_size_val,
            distance: clip_pos.w,
            shot_type,
            in_frame,
        }
    }

    /// Check if subject is at a power point.
    #[must_use]
    pub fn at_power_point(&self, guide: CompositionGuide, tolerance: f32) -> bool {
        let points = guide.power_points();
        points
            .iter()
            .any(|p| (self.screen_center - *p).length() < tolerance)
    }

    /// Get suggested camera adjustment to frame subject at a power point.
    #[must_use]
    pub fn suggest_adjustment(&self, target_point: Vec2) -> Vec2 {
        target_point - self.screen_center
    }
}

fn shot_type_from_coverage(coverage: f32) -> ShotType {
    if coverage >= 0.7 {
        ShotType::ExtremeCloseUp
    } else if coverage >= 0.5 {
        ShotType::CloseUp
    } else if coverage >= 0.35 {
        ShotType::MediumCloseUp
    } else if coverage >= 0.25 {
        ShotType::Medium
    } else if coverage >= 0.15 {
        ShotType::MediumLong
    } else if coverage >= 0.08 {
        ShotType::Long
    } else {
        ShotType::ExtremeLong
    }
}

/// Calculate optimal camera distance for a desired shot type.
#[must_use]
pub fn distance_for_shot_type(
    subject_radius: f32,
    fov_degrees: f32,
    shot_type: ShotType,
    aspect_ratio: f32,
) -> f32 {
    let target_coverage = shot_type.screen_coverage();
    let fov_rad = fov_degrees.to_radians();
    let half_fov = fov_rad / 2.0;

    let vertical_size = subject_radius * 2.0 / target_coverage;
    let horizontal_size = vertical_size / aspect_ratio.max(0.1);

    let distance_v = vertical_size / (2.0 * half_fov.tan());
    let distance_h = horizontal_size / (2.0 * (half_fov * aspect_ratio).tan());

    distance_v.max(distance_h)
}

/// Calculate FOV required to achieve a desired shot type at a fixed distance.
#[must_use]
pub fn fov_for_shot_type(subject_radius: f32, distance: f32, shot_type: ShotType) -> f32 {
    let target_coverage = shot_type.screen_coverage();
    let apparent_size = subject_radius * 2.0 / target_coverage;
    let half_fov = (apparent_size / (2.0 * distance)).atan();
    (half_fov * 2.0).to_degrees().clamp(10.0, 120.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_composition_guides() {
        for guide in CompositionGuide::ALL {
            let lines = guide.guide_lines();
            assert!(!guide.name().is_empty());

            for (start, end) in &lines {
                assert!((0.0..=1.0).contains(&start.x));
                assert!((0.0..=1.0).contains(&start.y));
                assert!((0.0..=1.0).contains(&end.x));
                assert!((0.0..=1.0).contains(&end.y));
            }
        }
    }

    #[test]
    fn test_rule_of_thirds_points() {
        let points = rule_of_thirds_points();
        assert_eq!(points.len(), 4);

        for point in &points {
            assert!((0.0..=1.0).contains(&point.x));
            assert!((0.0..=1.0).contains(&point.y));
        }
    }

    #[test]
    fn test_golden_ratio_points() {
        let points = golden_ratio_points();
        assert_eq!(points.len(), 4);
    }

    #[test]
    fn test_shot_type_coverage() {
        assert!(ShotType::ExtremeCloseUp.screen_coverage() > ShotType::CloseUp.screen_coverage());
        assert!(ShotType::CloseUp.screen_coverage() > ShotType::Medium.screen_coverage());
        assert!(ShotType::Medium.screen_coverage() > ShotType::Long.screen_coverage());
    }

    #[test]
    fn test_subject_framing_in_frame() {
        let view_proj = Mat4::perspective_lh(70.0_f32.to_radians(), 16.0 / 9.0, 0.1, 1000.0)
            * Mat4::look_at_lh(Vec3::new(0.0, 0.0, -10.0), Vec3::ZERO, Vec3::Y);

        let framing =
            SubjectFraming::calculate(Vec3::ZERO, 1.0, view_proj, Vec2::new(1920.0, 1080.0));

        assert!(framing.in_frame);
        assert!(framing.distance > 0.0);
    }

    #[test]
    fn test_subject_framing_out_of_frame() {
        let view_proj = Mat4::perspective_lh(70.0_f32.to_radians(), 16.0 / 9.0, 0.1, 1000.0)
            * Mat4::look_at_lh(Vec3::new(0.0, 0.0, -10.0), Vec3::ZERO, Vec3::Y);

        let far_away = Vec3::new(100.0, 100.0, 0.0);
        let framing =
            SubjectFraming::calculate(far_away, 1.0, view_proj, Vec2::new(1920.0, 1080.0));

        assert!(!framing.in_frame);
    }

    #[test]
    fn test_distance_for_shot_type() {
        let close = distance_for_shot_type(1.0, 70.0, ShotType::CloseUp, 16.0 / 9.0);
        let medium = distance_for_shot_type(1.0, 70.0, ShotType::Medium, 16.0 / 9.0);
        let long = distance_for_shot_type(1.0, 70.0, ShotType::Long, 16.0 / 9.0);

        assert!(close < medium);
        assert!(medium < long);
    }

    #[test]
    fn test_fov_for_shot_type() {
        let close_fov = fov_for_shot_type(1.0, 5.0, ShotType::CloseUp);
        let medium_fov = fov_for_shot_type(1.0, 5.0, ShotType::Medium);

        assert!(close_fov < medium_fov);
        assert!((10.0..=120.0).contains(&close_fov));
    }

    #[test]
    fn test_at_power_point() {
        let framing = SubjectFraming {
            screen_center: Vec2::new(1.0 / 3.0, 1.0 / 3.0),
            ..Default::default()
        };

        assert!(framing.at_power_point(CompositionGuide::RuleOfThirds, 0.01));
        assert!(!framing.at_power_point(CompositionGuide::Center, 0.01));
    }

    #[test]
    fn test_suggest_adjustment() {
        let framing = SubjectFraming {
            screen_center: Vec2::new(0.5, 0.5),
            ..Default::default()
        };

        let adjustment = framing.suggest_adjustment(Vec2::new(1.0 / 3.0, 1.0 / 3.0));
        assert_relative_eq!(adjustment.x, 1.0 / 3.0 - 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_serde_roundtrip() {
        for guide in CompositionGuide::ALL {
            let bytes = bincode::serialize(&guide).expect("serialize");
            let restored: CompositionGuide = bincode::deserialize(&bytes).expect("deserialize");
            assert_eq!(guide, restored);
        }

        for shot in [ShotType::CloseUp, ShotType::Medium, ShotType::Long] {
            let bytes = bincode::serialize(&shot).expect("serialize");
            let restored: ShotType = bincode::deserialize(&bytes).expect("deserialize");
            assert_eq!(shot, restored);
        }
    }
}
