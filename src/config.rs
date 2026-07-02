//! Signaling method configuration.
//!
//! Attitude and count methods are independent knobs so a lesson can teach
//! either convention. All four combinations are supported and tested, even
//! though upside-down count with standard attitude is essentially unplayed
//! in practice — the bot must behave sensibly for any configuration a
//! teacher can express.

/// Attitude signal method (partner leads, you signal like/dislike).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AttitudeMethod {
    /// High card encourages, low card discourages.
    #[default]
    Standard,
    /// Low card encourages, high card discourages (UDCA).
    UpsideDown,
}

/// Count signal method (declarer leads, you show suit length).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CountMethod {
    /// High-low shows an even number of cards, low-high shows odd.
    #[default]
    Standard,
    /// Low-high shows an even number of cards, high-low shows odd.
    UpsideDown,
}

/// The bot's carding agreements. `Default` is standard/standard, matching
/// what Bridge Classroom lessons teach first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SignalConfig {
    /// Attitude method on partner's leads and on discards.
    pub attitude: AttitudeMethod,
    /// Count method on declarer's leads.
    pub count: CountMethod,
}
