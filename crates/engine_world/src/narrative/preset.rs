//! Built-in narrative event presets.

use serde::{Deserialize, Serialize};

use super::{
    CooldownConfig, EventDefinition, NarrativeEventKind, NarrativeTrigger, OutputPriority,
};

/// Trait for preset event templates.
pub trait Preset {
    /// Convert to an event definition.
    fn to_definition(&self) -> EventDefinition;

    /// Get the preset's unique ID.
    fn id(&self) -> &str;

    /// Get the event kind.
    fn kind(&self) -> NarrativeEventKind;
}

/// Disaster event presets (meteor strikes, earthquakes, volcanic eruptions).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DisasterPreset {
    /// Meteor strike at a location.
    MeteorStrike {
        id: String,
        intensity: u8,
        warning_ticks: u64,
    },
    /// Earthquake event.
    Earthquake {
        id: String,
        magnitude: u8,
        duration_ticks: u64,
    },
    /// Volcanic eruption.
    VolcanicEruption {
        id: String,
        eruption_phase_ticks: u64,
    },
    /// Radiation storm.
    RadiationStorm {
        id: String,
        severity: u8,
        duration_ticks: u64,
    },
    /// Custom disaster.
    Custom {
        id: String,
        display_name: String,
        text: String,
        audio_cue: Option<String>,
        duration_ticks: u64,
    },
}

impl DisasterPreset {
    /// Create a meteor strike preset.
    #[must_use]
    pub fn meteor_strike(id: impl Into<String>, intensity: u8, warning_ticks: u64) -> Self {
        Self::MeteorStrike {
            id: id.into(),
            intensity,
            warning_ticks,
        }
    }

    /// Create an earthquake preset.
    #[must_use]
    pub fn earthquake(id: impl Into<String>, magnitude: u8, duration_ticks: u64) -> Self {
        Self::Earthquake {
            id: id.into(),
            magnitude,
            duration_ticks,
        }
    }

    /// Create a volcanic eruption preset.
    #[must_use]
    pub fn volcanic_eruption(id: impl Into<String>, eruption_phase_ticks: u64) -> Self {
        Self::VolcanicEruption {
            id: id.into(),
            eruption_phase_ticks,
        }
    }

    /// Create a radiation storm preset.
    #[must_use]
    pub fn radiation_storm(id: impl Into<String>, severity: u8, duration_ticks: u64) -> Self {
        Self::RadiationStorm {
            id: id.into(),
            severity,
            duration_ticks,
        }
    }
}

impl Preset for DisasterPreset {
    fn to_definition(&self) -> EventDefinition {
        match self {
            DisasterPreset::MeteorStrike {
                id,
                intensity,
                warning_ticks,
            } => EventDefinition::new(id, NarrativeEventKind::Disaster)
                .with_display_name("Meteor Strike")
                .with_text(format!(
                    "WARNING: Incoming meteor detected. Impact in {warning_ticks} ticks. Intensity: {intensity}"
                ))
                .with_audio("disaster_meteor_warning")
                .with_duration(*warning_ticks + 600)
                .with_priority(OutputPriority::CRITICAL)
                .with_cooldown(CooldownConfig::once())
                .with_trigger(NarrativeTrigger::always())
                .with_tag("disaster")
                .with_tag("meteor")
                .with_custom("intensity", intensity.to_string()),

            DisasterPreset::Earthquake {
                id,
                magnitude,
                duration_ticks,
            } => EventDefinition::new(id, NarrativeEventKind::Disaster)
                .with_display_name("Earthquake")
                .with_text(format!(
                    "SEISMIC ALERT: Earthquake magnitude {magnitude} detected."
                ))
                .with_audio("disaster_earthquake")
                .with_duration(*duration_ticks)
                .with_priority(OutputPriority::CRITICAL)
                .with_cooldown(CooldownConfig::once())
                .with_trigger(NarrativeTrigger::always())
                .with_tag("disaster")
                .with_tag("earthquake")
                .with_custom("magnitude", magnitude.to_string()),

            DisasterPreset::VolcanicEruption {
                id,
                eruption_phase_ticks,
            } => EventDefinition::new(id, NarrativeEventKind::Disaster)
                .with_display_name("Volcanic Eruption")
                .with_text("CRITICAL: Volcanic eruption imminent. Evacuate immediately.")
                .with_audio("disaster_volcano")
                .with_duration(*eruption_phase_ticks)
                .with_priority(OutputPriority::CRITICAL)
                .with_cooldown(CooldownConfig::once())
                .with_trigger(NarrativeTrigger::always())
                .with_tag("disaster")
                .with_tag("volcano"),

            DisasterPreset::RadiationStorm {
                id,
                severity,
                duration_ticks,
            } => EventDefinition::new(id, NarrativeEventKind::Disaster)
                .with_display_name("Radiation Storm")
                .with_text(format!(
                    "HAZARD: Radiation storm approaching. Severity level {severity}. Seek shelter."
                ))
                .with_audio("disaster_radiation")
                .with_duration(*duration_ticks)
                .with_priority(OutputPriority::CRITICAL)
                .with_cooldown(CooldownConfig::once())
                .with_trigger(NarrativeTrigger::always())
                .with_tag("disaster")
                .with_tag("radiation")
                .with_custom("severity", severity.to_string()),

            DisasterPreset::Custom {
                id,
                display_name,
                text,
                audio_cue,
                duration_ticks,
            } => {
                let mut def = EventDefinition::new(id, NarrativeEventKind::Disaster)
                    .with_display_name(display_name)
                    .with_text(text)
                    .with_duration(*duration_ticks)
                    .with_priority(OutputPriority::CRITICAL)
                    .with_cooldown(CooldownConfig::once())
                    .with_trigger(NarrativeTrigger::always())
                    .with_tag("disaster")
                    .with_tag("custom");
                if let Some(cue) = audio_cue {
                    def = def.with_audio(cue);
                }
                def
            }
        }
    }

    fn id(&self) -> &str {
        match self {
            DisasterPreset::MeteorStrike { id, .. }
            | DisasterPreset::Earthquake { id, .. }
            | DisasterPreset::VolcanicEruption { id, .. }
            | DisasterPreset::RadiationStorm { id, .. }
            | DisasterPreset::Custom { id, .. } => id,
        }
    }

    fn kind(&self) -> NarrativeEventKind {
        NarrativeEventKind::Disaster
    }
}

/// Radio chatter presets (distress calls, broadcasts, transmissions).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RadioPreset {
    /// Distress call from survivors.
    DistressCall {
        id: String,
        sender: String,
        message: String,
        urgency: u8,
    },
    /// Ambient radio chatter.
    AmbientChatter {
        id: String,
        messages: Vec<String>,
        interval_ticks: u64,
    },
    /// Intercepted transmission.
    InterceptedTransmission {
        id: String,
        source: String,
        content: String,
        encrypted: bool,
    },
    /// Emergency broadcast.
    EmergencyBroadcast { id: String, message: String },
    /// Custom radio event.
    Custom {
        id: String,
        display_name: String,
        text: String,
        audio_cue: Option<String>,
        repeat_count: Option<u32>,
        cooldown_ticks: u64,
    },
}

impl RadioPreset {
    /// Create a distress call preset.
    #[must_use]
    pub fn distress_call(
        id: impl Into<String>,
        sender: impl Into<String>,
        message: impl Into<String>,
        urgency: u8,
    ) -> Self {
        Self::DistressCall {
            id: id.into(),
            sender: sender.into(),
            message: message.into(),
            urgency,
        }
    }

    /// Create an ambient chatter preset.
    #[must_use]
    pub fn ambient_chatter(
        id: impl Into<String>,
        messages: Vec<String>,
        interval_ticks: u64,
    ) -> Self {
        Self::AmbientChatter {
            id: id.into(),
            messages,
            interval_ticks,
        }
    }

    /// Create an intercepted transmission preset.
    #[must_use]
    pub fn intercepted_transmission(
        id: impl Into<String>,
        source: impl Into<String>,
        content: impl Into<String>,
        encrypted: bool,
    ) -> Self {
        Self::InterceptedTransmission {
            id: id.into(),
            source: source.into(),
            content: content.into(),
            encrypted,
        }
    }

    /// Create an emergency broadcast preset.
    #[must_use]
    pub fn emergency_broadcast(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::EmergencyBroadcast {
            id: id.into(),
            message: message.into(),
        }
    }
}

impl Preset for RadioPreset {
    fn to_definition(&self) -> EventDefinition {
        match self {
            RadioPreset::DistressCall {
                id,
                sender,
                message,
                urgency,
            } => EventDefinition::new(id, NarrativeEventKind::Radio)
                .with_display_name(format!("Distress Call - {sender}"))
                .with_text(format!("[{sender}]: {message}"))
                .with_audio("radio_distress")
                .with_duration(600)
                .with_priority(OutputPriority::from_level(150 + urgency.min(&100)))
                .with_cooldown(CooldownConfig::once())
                .with_trigger(NarrativeTrigger::always())
                .with_tag("radio")
                .with_tag("distress")
                .with_custom("sender", sender.clone())
                .with_custom("urgency", urgency.to_string()),

            RadioPreset::AmbientChatter {
                id,
                messages,
                interval_ticks,
            } => {
                let text = messages.first().cloned().unwrap_or_default();
                EventDefinition::new(id, NarrativeEventKind::Radio)
                    .with_display_name("Radio Chatter")
                    .with_text(text)
                    .with_audio("radio_static")
                    .with_duration(300)
                    .with_priority(OutputPriority::MINIMAL)
                    .with_cooldown(CooldownConfig::repeating(*interval_ticks))
                    .with_trigger(NarrativeTrigger::always())
                    .with_tag("radio")
                    .with_tag("ambient")
                    .with_custom("message_count", messages.len().to_string())
            }

            RadioPreset::InterceptedTransmission {
                id,
                source,
                content,
                encrypted,
            } => {
                let display_content = if *encrypted {
                    "[ENCRYPTED TRANSMISSION]".to_string()
                } else {
                    content.clone()
                };
                EventDefinition::new(id, NarrativeEventKind::Radio)
                    .with_display_name(format!("Intercepted - {source}"))
                    .with_text(display_content)
                    .with_audio("radio_intercept")
                    .with_duration(450)
                    .with_priority(OutputPriority::NORMAL)
                    .with_cooldown(CooldownConfig::once())
                    .with_trigger(NarrativeTrigger::always())
                    .with_tag("radio")
                    .with_tag("intercept")
                    .with_custom("source", source.clone())
                    .with_custom("encrypted", encrypted.to_string())
            }

            RadioPreset::EmergencyBroadcast { id, message } => {
                EventDefinition::new(id, NarrativeEventKind::Radio)
                    .with_display_name("Emergency Broadcast")
                    .with_text(format!("EMERGENCY BROADCAST: {message}"))
                    .with_audio("radio_emergency")
                    .with_duration(900)
                    .with_priority(OutputPriority::HIGH)
                    .with_cooldown(CooldownConfig::repeat_count(3, 300))
                    .with_trigger(NarrativeTrigger::always())
                    .with_tag("radio")
                    .with_tag("emergency")
            }

            RadioPreset::Custom {
                id,
                display_name,
                text,
                audio_cue,
                repeat_count,
                cooldown_ticks,
            } => {
                let cooldown = match repeat_count {
                    Some(count) => CooldownConfig::repeat_count(*count, *cooldown_ticks),
                    None => CooldownConfig::once(),
                };
                let mut def = EventDefinition::new(id, NarrativeEventKind::Radio)
                    .with_display_name(display_name)
                    .with_text(text)
                    .with_duration(300)
                    .with_priority(OutputPriority::LOW)
                    .with_cooldown(cooldown)
                    .with_trigger(NarrativeTrigger::always())
                    .with_tag("radio")
                    .with_tag("custom");
                if let Some(cue) = audio_cue {
                    def = def.with_audio(cue);
                }
                def
            }
        }
    }

    fn id(&self) -> &str {
        match self {
            RadioPreset::DistressCall { id, .. }
            | RadioPreset::AmbientChatter { id, .. }
            | RadioPreset::InterceptedTransmission { id, .. }
            | RadioPreset::EmergencyBroadcast { id, .. }
            | RadioPreset::Custom { id, .. } => id,
        }
    }

    fn kind(&self) -> NarrativeEventKind {
        NarrativeEventKind::Radio
    }
}

/// Objective presets (missions with deadlines).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ObjectivePreset {
    /// Rescue mission with time limit.
    RescueMission {
        id: String,
        target: String,
        deadline_ticks: u64,
    },
    /// Resource collection objective.
    CollectResources {
        id: String,
        resource_type: String,
        target_count: u32,
        deadline_ticks: u64,
    },
    /// Evacuation objective.
    Evacuation {
        id: String,
        location: String,
        deadline_ticks: u64,
    },
    /// Defense objective.
    DefendLocation {
        id: String,
        location: String,
        duration_ticks: u64,
    },
    /// Custom objective.
    Custom {
        id: String,
        title: String,
        description: String,
        deadline_ticks: u64,
        target_count: Option<u32>,
    },
}

impl ObjectivePreset {
    /// Create a rescue mission preset.
    #[must_use]
    pub fn rescue_mission(
        id: impl Into<String>,
        target: impl Into<String>,
        deadline_ticks: u64,
    ) -> Self {
        Self::RescueMission {
            id: id.into(),
            target: target.into(),
            deadline_ticks,
        }
    }

    /// Create a collect resources preset.
    #[must_use]
    pub fn collect_resources(
        id: impl Into<String>,
        resource_type: impl Into<String>,
        target_count: u32,
        deadline_ticks: u64,
    ) -> Self {
        Self::CollectResources {
            id: id.into(),
            resource_type: resource_type.into(),
            target_count,
            deadline_ticks,
        }
    }

    /// Create an evacuation preset.
    #[must_use]
    pub fn evacuation(
        id: impl Into<String>,
        location: impl Into<String>,
        deadline_ticks: u64,
    ) -> Self {
        Self::Evacuation {
            id: id.into(),
            location: location.into(),
            deadline_ticks,
        }
    }

    /// Create a defend location preset.
    #[must_use]
    pub fn defend_location(
        id: impl Into<String>,
        location: impl Into<String>,
        duration_ticks: u64,
    ) -> Self {
        Self::DefendLocation {
            id: id.into(),
            location: location.into(),
            duration_ticks,
        }
    }
}

impl Preset for ObjectivePreset {
    fn to_definition(&self) -> EventDefinition {
        match self {
            ObjectivePreset::RescueMission {
                id,
                target,
                deadline_ticks,
            } => EventDefinition::new(id, NarrativeEventKind::Objective)
                .with_display_name(format!("Rescue: {target}"))
                .with_text(format!("MISSION: Rescue {target} before time runs out."))
                .with_audio("objective_rescue")
                .with_duration(*deadline_ticks)
                .with_priority(OutputPriority::HIGH)
                .with_cooldown(CooldownConfig::once())
                .with_trigger(NarrativeTrigger::always())
                .with_tag("objective")
                .with_tag("rescue")
                .with_custom("target", target.clone()),

            ObjectivePreset::CollectResources {
                id,
                resource_type,
                target_count,
                deadline_ticks,
            } => EventDefinition::new(id, NarrativeEventKind::Objective)
                .with_display_name(format!("Collect: {resource_type} x{target_count}"))
                .with_text(format!(
                    "OBJECTIVE: Collect {target_count} units of {resource_type}."
                ))
                .with_audio("objective_collect")
                .with_duration(*deadline_ticks)
                .with_priority(OutputPriority::NORMAL)
                .with_cooldown(CooldownConfig::once())
                .with_trigger(NarrativeTrigger::always())
                .with_tag("objective")
                .with_tag("collect")
                .with_custom("resource", resource_type.clone())
                .with_custom("target", target_count.to_string()),

            ObjectivePreset::Evacuation {
                id,
                location,
                deadline_ticks,
            } => EventDefinition::new(id, NarrativeEventKind::Objective)
                .with_display_name(format!("Evacuate: {location}"))
                .with_text(format!(
                    "URGENT: Evacuate {location} immediately. Time is critical."
                ))
                .with_audio("objective_evacuate")
                .with_duration(*deadline_ticks)
                .with_priority(OutputPriority::HIGH)
                .with_cooldown(CooldownConfig::once())
                .with_trigger(NarrativeTrigger::always())
                .with_tag("objective")
                .with_tag("evacuation")
                .with_custom("location", location.clone()),

            ObjectivePreset::DefendLocation {
                id,
                location,
                duration_ticks,
            } => EventDefinition::new(id, NarrativeEventKind::Objective)
                .with_display_name(format!("Defend: {location}"))
                .with_text(format!(
                    "DEFEND: Hold {location} for the required duration."
                ))
                .with_audio("objective_defend")
                .with_duration(*duration_ticks)
                .with_priority(OutputPriority::HIGH)
                .with_cooldown(CooldownConfig::once())
                .with_trigger(NarrativeTrigger::always())
                .with_tag("objective")
                .with_tag("defense")
                .with_custom("location", location.clone()),

            ObjectivePreset::Custom {
                id,
                title,
                description,
                deadline_ticks,
                target_count,
            } => {
                let mut def = EventDefinition::new(id, NarrativeEventKind::Objective)
                    .with_display_name(title)
                    .with_text(description)
                    .with_duration(*deadline_ticks)
                    .with_priority(OutputPriority::NORMAL)
                    .with_cooldown(CooldownConfig::once())
                    .with_trigger(NarrativeTrigger::always())
                    .with_tag("objective")
                    .with_tag("custom");
                if let Some(count) = target_count {
                    def = def.with_custom("target_count", count.to_string());
                }
                def
            }
        }
    }

    fn id(&self) -> &str {
        match self {
            ObjectivePreset::RescueMission { id, .. }
            | ObjectivePreset::CollectResources { id, .. }
            | ObjectivePreset::Evacuation { id, .. }
            | ObjectivePreset::DefendLocation { id, .. }
            | ObjectivePreset::Custom { id, .. } => id,
        }
    }

    fn kind(&self) -> NarrativeEventKind {
        NarrativeEventKind::Objective
    }
}

/// Anomaly sighting presets (strange phenomena, unknown signals).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AnomalyPreset {
    /// Unknown signal detected.
    UnknownSignal {
        id: String,
        frequency: String,
        strength: u8,
    },
    /// Strange phenomena observed.
    StrangePhenomena {
        id: String,
        description: String,
        threat_level: u8,
    },
    /// Unidentified object sighting.
    UnidentifiedObject { id: String, classification: String },
    /// Energy fluctuation.
    EnergyFluctuation {
        id: String,
        magnitude: u8,
        pattern: String,
    },
    /// Custom anomaly.
    Custom {
        id: String,
        display_name: String,
        description: String,
        duration_ticks: u64,
    },
}

impl AnomalyPreset {
    /// Create an unknown signal preset.
    #[must_use]
    pub fn unknown_signal(
        id: impl Into<String>,
        frequency: impl Into<String>,
        strength: u8,
    ) -> Self {
        Self::UnknownSignal {
            id: id.into(),
            frequency: frequency.into(),
            strength,
        }
    }

    /// Create a strange phenomena preset.
    #[must_use]
    pub fn strange_phenomena(
        id: impl Into<String>,
        description: impl Into<String>,
        threat_level: u8,
    ) -> Self {
        Self::StrangePhenomena {
            id: id.into(),
            description: description.into(),
            threat_level,
        }
    }

    /// Create an unidentified object preset.
    #[must_use]
    pub fn unidentified_object(id: impl Into<String>, classification: impl Into<String>) -> Self {
        Self::UnidentifiedObject {
            id: id.into(),
            classification: classification.into(),
        }
    }

    /// Create an energy fluctuation preset.
    #[must_use]
    pub fn energy_fluctuation(
        id: impl Into<String>,
        magnitude: u8,
        pattern: impl Into<String>,
    ) -> Self {
        Self::EnergyFluctuation {
            id: id.into(),
            magnitude,
            pattern: pattern.into(),
        }
    }
}

impl Preset for AnomalyPreset {
    fn to_definition(&self) -> EventDefinition {
        match self {
            AnomalyPreset::UnknownSignal {
                id,
                frequency,
                strength,
            } => EventDefinition::new(id, NarrativeEventKind::Anomaly)
                .with_display_name("Unknown Signal")
                .with_text(format!(
                    "ANOMALY: Unknown signal detected at {frequency} Hz. Strength: {strength}"
                ))
                .with_audio("anomaly_signal")
                .with_duration(900)
                .with_priority(OutputPriority::NORMAL)
                .with_cooldown(CooldownConfig::repeating(3000).with_jitter(500))
                .with_trigger(NarrativeTrigger::always())
                .with_tag("anomaly")
                .with_tag("signal")
                .with_custom("frequency", frequency.clone())
                .with_custom("strength", strength.to_string()),

            AnomalyPreset::StrangePhenomena {
                id,
                description,
                threat_level,
            } => {
                let priority = if *threat_level > 7 {
                    OutputPriority::HIGH
                } else {
                    OutputPriority::NORMAL
                };
                EventDefinition::new(id, NarrativeEventKind::Anomaly)
                    .with_display_name("Strange Phenomena")
                    .with_text(format!("ANOMALY DETECTED: {description}"))
                    .with_audio("anomaly_phenomena")
                    .with_duration(1200)
                    .with_priority(priority)
                    .with_cooldown(CooldownConfig::once())
                    .with_trigger(NarrativeTrigger::always())
                    .with_tag("anomaly")
                    .with_tag("phenomena")
                    .with_custom("threat_level", threat_level.to_string())
            }

            AnomalyPreset::UnidentifiedObject { id, classification } => {
                EventDefinition::new(id, NarrativeEventKind::Anomaly)
                    .with_display_name("Unidentified Object")
                    .with_text(format!(
                        "VISUAL CONTACT: Unidentified object. Classification: {classification}"
                    ))
                    .with_audio("anomaly_ufo")
                    .with_duration(600)
                    .with_priority(OutputPriority::NORMAL)
                    .with_cooldown(CooldownConfig::once())
                    .with_trigger(NarrativeTrigger::always())
                    .with_tag("anomaly")
                    .with_tag("ufo")
                    .with_custom("classification", classification.clone())
            }

            AnomalyPreset::EnergyFluctuation {
                id,
                magnitude,
                pattern,
            } => EventDefinition::new(id, NarrativeEventKind::Anomaly)
                .with_display_name("Energy Fluctuation")
                .with_text(format!(
                    "SENSOR ALERT: Energy fluctuation detected. Magnitude: {magnitude}. Pattern: {pattern}"
                ))
                .with_audio("anomaly_energy")
                .with_duration(450)
                .with_priority(OutputPriority::NORMAL)
                .with_cooldown(CooldownConfig::repeating(1800))
                .with_trigger(NarrativeTrigger::always())
                .with_tag("anomaly")
                .with_tag("energy")
                .with_custom("magnitude", magnitude.to_string())
                .with_custom("pattern", pattern.clone()),

            AnomalyPreset::Custom {
                id,
                display_name,
                description,
                duration_ticks,
            } => EventDefinition::new(id, NarrativeEventKind::Anomaly)
                .with_display_name(display_name)
                .with_text(description)
                .with_duration(*duration_ticks)
                .with_priority(OutputPriority::NORMAL)
                .with_cooldown(CooldownConfig::once())
                .with_trigger(NarrativeTrigger::always())
                .with_tag("anomaly")
                .with_tag("custom"),
        }
    }

    fn id(&self) -> &str {
        match self {
            AnomalyPreset::UnknownSignal { id, .. }
            | AnomalyPreset::StrangePhenomena { id, .. }
            | AnomalyPreset::UnidentifiedObject { id, .. }
            | AnomalyPreset::EnergyFluctuation { id, .. }
            | AnomalyPreset::Custom { id, .. } => id,
        }
    }

    fn kind(&self) -> NarrativeEventKind {
        NarrativeEventKind::Anomaly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disaster_meteor_strike_to_definition() {
        let preset = DisasterPreset::meteor_strike("meteor_1", 8, 300);
        let def = preset.to_definition();

        assert_eq!(def.id, "meteor_1");
        assert_eq!(def.kind, NarrativeEventKind::Disaster);
        assert!(def.has_tag("disaster"));
        assert!(def.has_tag("meteor"));
        assert_eq!(def.custom_data.get("intensity"), Some(&"8".to_string()));
    }

    #[test]
    fn disaster_earthquake_to_definition() {
        let preset = DisasterPreset::earthquake("quake_1", 6, 1200);
        let def = preset.to_definition();

        assert_eq!(def.id, "quake_1");
        assert_eq!(def.duration, 1200);
        assert!(def.has_tag("earthquake"));
    }

    #[test]
    fn radio_distress_call_to_definition() {
        let preset = RadioPreset::distress_call("sos_1", "Station Alpha", "Need help!", 75);
        let def = preset.to_definition();

        assert_eq!(def.id, "sos_1");
        assert_eq!(def.kind, NarrativeEventKind::Radio);
        assert!(def.has_tag("distress"));
        assert!(def.text_template.unwrap().contains("Station Alpha"));
    }

    #[test]
    fn radio_ambient_chatter_repeats() {
        let preset =
            RadioPreset::ambient_chatter("chatter_1", vec!["Hello".into(), "World".into()], 1000);
        let def = preset.to_definition();

        assert!(matches!(
            def.cooldown.repeat_mode,
            super::super::RepeatMode::Forever
        ));
        assert_eq!(def.cooldown.cooldown_ticks, 1000);
    }

    #[test]
    fn objective_rescue_to_definition() {
        let preset = ObjectivePreset::rescue_mission("rescue_1", "Survivor Team", 6000);
        let def = preset.to_definition();

        assert_eq!(def.id, "rescue_1");
        assert_eq!(def.kind, NarrativeEventKind::Objective);
        assert!(def.has_tag("rescue"));
        assert_eq!(def.duration, 6000);
    }

    #[test]
    fn objective_collect_resources_to_definition() {
        let preset = ObjectivePreset::collect_resources("collect_1", "Crystals", 50, 3000);
        let def = preset.to_definition();

        assert_eq!(
            def.custom_data.get("resource"),
            Some(&"Crystals".to_string())
        );
        assert_eq!(def.custom_data.get("target"), Some(&"50".to_string()));
    }

    #[test]
    fn anomaly_unknown_signal_to_definition() {
        let preset = AnomalyPreset::unknown_signal("signal_1", "432.5", 85);
        let def = preset.to_definition();

        assert_eq!(def.id, "signal_1");
        assert_eq!(def.kind, NarrativeEventKind::Anomaly);
        assert!(def.has_tag("signal"));
        assert!(def.cooldown.jitter_ticks > 0);
    }

    #[test]
    fn anomaly_strange_phenomena_high_threat() {
        let preset = AnomalyPreset::strange_phenomena("phenom_1", "Reality distortion", 9);
        let def = preset.to_definition();

        assert_eq!(def.priority, OutputPriority::HIGH);
    }

    #[test]
    fn preset_trait_id_and_kind() {
        let disaster: &dyn Preset = &DisasterPreset::earthquake("eq", 5, 100);
        assert_eq!(disaster.id(), "eq");
        assert_eq!(disaster.kind(), NarrativeEventKind::Disaster);

        let radio: &dyn Preset = &RadioPreset::emergency_broadcast("eb", "Test broadcast");
        assert_eq!(radio.id(), "eb");
        assert_eq!(radio.kind(), NarrativeEventKind::Radio);
    }

    #[test]
    fn serde_round_trip_disaster() {
        let preset = DisasterPreset::meteor_strike("test", 5, 100);
        let json = serde_json::to_string(&preset).unwrap();
        let recovered: DisasterPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, preset);
    }

    #[test]
    fn serde_round_trip_radio() {
        let preset = RadioPreset::distress_call("test", "sender", "message", 50);
        let json = serde_json::to_string(&preset).unwrap();
        let recovered: RadioPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, preset);
    }

    #[test]
    fn serde_round_trip_objective() {
        let preset = ObjectivePreset::rescue_mission("test", "target", 1000);
        let json = serde_json::to_string(&preset).unwrap();
        let recovered: ObjectivePreset = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, preset);
    }

    #[test]
    fn serde_round_trip_anomaly() {
        let preset = AnomalyPreset::unknown_signal("test", "100", 50);
        let json = serde_json::to_string(&preset).unwrap();
        let recovered: AnomalyPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, preset);
    }
}
