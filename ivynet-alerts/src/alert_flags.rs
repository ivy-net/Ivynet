use serde::{Deserialize, Serialize};

use crate::AlertType;

use super::bitflag::{BitFlag, BitflagError};

/// A bitflag representation of which alerts are enabled. Uses the `AlertType` enum discriminant of
/// `Alert` to determine which bit corresponds to which alert.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlertFlags(BitFlag);

/// 0 bit is currently unsued for any notification type.
impl AlertFlags {
    pub fn new(flags: BitFlag) -> Self {
        Self(flags)
    }

    pub fn toggle_alert(&mut self, alert: &AlertType) -> Result<(), BitflagError> {
        self.0.try_flip_bit(alert.id())
    }

    pub fn enable_alert(&mut self, alert: &AlertType) -> Result<(), BitflagError> {
        self.0.try_set_bit(alert.id())
    }

    pub fn disable_alert(&mut self, alert: &AlertType) -> Result<(), BitflagError> {
        self.0.try_unset_bit(alert.id())
    }

    pub fn is_alert_enabled(&self, alert: &AlertType) -> Result<bool, BitflagError> {
        self.0.try_get_bit(alert.id())
    }

    pub fn set_alert_to(&mut self, alert: &AlertType, value: bool) -> Result<(), BitflagError> {
        self.0.set_bit_to(alert.id(), value)
    }

    pub fn are_alerts_enabled(&self, alerts: &[&AlertType]) -> Result<Vec<bool>, BitflagError> {
        let enabled: Vec<bool> =
            alerts.iter().map(|a| self.is_alert_enabled(a)).collect::<Result<_, _>>()?;
        Ok(enabled)
    }

    /// Does not check for the 0 bit, which is unused, or bits larger than
    /// `NotificationType::variant_count()` which are also unused.
    pub fn to_alert_ids(&self) -> Vec<usize> {
        (1..=AlertType::variant_count())
            .filter(|&i| self.0.try_get_bit(i).unwrap_or(false))
            .collect()
    }

    pub fn to_alert_types(&self) -> Vec<AlertType> {
        self.to_alert_ids().into_iter().map(AlertType::from).collect()
    }
}

impl Default for AlertFlags {
    fn default() -> Self {
        Self::new(BitFlag::default())
    }
}

impl From<u64> for AlertFlags {
    fn from(value: u64) -> Self {
        Self(BitFlag::new(value))
    }
}

impl From<AlertFlags> for u64 {
    fn from(flags: AlertFlags) -> Self {
        flags.0.into()
    }
}

impl From<i64> for AlertFlags {
    fn from(value: i64) -> Self {
        Self(BitFlag::new(value as u64))
    }
}

impl From<&[AlertType]> for AlertFlags {
    fn from(alerts: &[AlertType]) -> Self {
        let mut flags = BitFlag::zero();
        for alert in alerts {
            flags.try_set_bit(alert.id()).expect("Failed to set bit, out of range.");
        }
        Self::new(flags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_alert_flags() -> Result<(), BitflagError> {
        let custom_alert = AlertType::Custom;
        let needs_update_alert = AlertType::NeedsUpdate;

        let mut flags = AlertFlags::default();
        assert!(!flags.is_alert_enabled(&custom_alert)?);
        assert!(!flags.is_alert_enabled(&needs_update_alert)?);
        assert!(flags.to_alert_ids().is_empty());

        flags.enable_alert(&custom_alert)?;
        flags.enable_alert(&needs_update_alert)?;

        assert!(flags.is_alert_enabled(&custom_alert)?);
        assert!(flags.is_alert_enabled(&needs_update_alert)?);

        let enabled = flags.are_alerts_enabled(&[&custom_alert, &needs_update_alert])?;
        assert_eq!(enabled, vec![true, true]);

        let ids = flags.to_alert_ids();
        assert_eq!(2, ids.len());
        assert!(ids.contains(&custom_alert.id()));
        assert!(ids.contains(&needs_update_alert.id()));

        Ok(())
    }
}
