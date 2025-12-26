//! Notification filtering and prioritization

use crate::{Category, Notification, Priority};
use chrono::Datelike;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Filter engine for notifications
pub struct FilterEngine {
    rules: Vec<FilterRule>,
    focus_mode: bool,
    quiet_hours: Option<QuietHours>,
}

impl FilterEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            focus_mode: false,
            quiet_hours: None,
        }
    }

    /// Add a filter rule
    pub fn add_rule(&mut self, rule: FilterRule) {
        self.rules.push(rule);
    }

    /// Remove a filter rule
    pub fn remove_rule(&mut self, rule_id: &str) {
        self.rules.retain(|r| r.id != rule_id);
    }

    /// Enable/disable focus mode
    pub fn set_focus_mode(&mut self, enabled: bool) {
        self.focus_mode = enabled;
    }

    /// Set quiet hours
    pub fn set_quiet_hours(&mut self, quiet_hours: Option<QuietHours>) {
        self.quiet_hours = quiet_hours;
    }

    /// Check if notification should be displayed
    pub fn should_display(&self, notification: &Notification) -> bool {
        // Check quiet hours
        if let Some(quiet) = &self.quiet_hours {
            if quiet.is_active() {
                // Only allow critical notifications during quiet hours
                if notification.priority != Priority::Critical {
                    return false;
                }
            }
        }

        // Check focus mode
        if self.focus_mode {
            // Only allow high priority and above in focus mode
            if notification.priority < Priority::High {
                return false;
            }
        }

        // Check rules
        for rule in &self.rules {
            match rule.matches(notification) {
                RuleResult::Allow => return true,
                RuleResult::Block => return false,
                RuleResult::NoMatch => continue,
            }
        }

        // Default: allow
        true
    }

    /// Check if notification should be synced
    pub fn should_sync(&self, notification: &Notification) -> bool {
        // Don't sync dismissed notifications
        if notification.dismissed {
            return false;
        }

        // Check rules for sync
        for rule in &self.rules {
            if rule.affects_sync {
                match rule.matches(notification) {
                    RuleResult::Block => return false,
                    _ => continue,
                }
            }
        }

        true
    }
}

impl Default for FilterEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// A filter rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub action: RuleAction,
    pub conditions: Vec<RuleCondition>,
    pub affects_sync: bool,
}

impl FilterRule {
    /// Create a new allow rule
    pub fn allow(name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            enabled: true,
            action: RuleAction::Allow,
            conditions: Vec::new(),
            affects_sync: false,
        }
    }

    /// Create a new block rule
    pub fn block(name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            enabled: true,
            action: RuleAction::Block,
            conditions: Vec::new(),
            affects_sync: false,
        }
    }

    /// Add a condition
    pub fn with_condition(mut self, condition: RuleCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Set affects_sync
    pub fn sync_affected(mut self, affected: bool) -> Self {
        self.affects_sync = affected;
        self
    }

    /// Check if rule matches notification
    pub fn matches(&self, notification: &Notification) -> RuleResult {
        if !self.enabled {
            return RuleResult::NoMatch;
        }

        // All conditions must match
        let all_match = self.conditions.iter().all(|c| c.matches(notification));

        if all_match && !self.conditions.is_empty() {
            match self.action {
                RuleAction::Allow => RuleResult::Allow,
                RuleAction::Block => RuleResult::Block,
            }
        } else {
            RuleResult::NoMatch
        }
    }
}

/// Rule action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    Allow,
    Block,
}

/// Rule matching result
#[derive(Debug, Clone, PartialEq)]
pub enum RuleResult {
    Allow,
    Block,
    NoMatch,
}

/// Rule condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCondition {
    /// Match app name
    AppName(StringMatch),
    /// Match title
    Title(StringMatch),
    /// Match body
    Body(StringMatch),
    /// Match category
    Category(Category),
    /// Match priority
    Priority(PriorityMatch),
    /// Match source device
    SourceDevice(String),
    /// Custom metadata match
    Metadata { key: String, value: StringMatch },
}

impl RuleCondition {
    pub fn matches(&self, notification: &Notification) -> bool {
        match self {
            RuleCondition::AppName(m) => m.matches(&notification.app_name),
            RuleCondition::Title(m) => m.matches(&notification.title),
            RuleCondition::Body(m) => m.matches(&notification.body),
            RuleCondition::Category(c) => &notification.category == c,
            RuleCondition::Priority(p) => p.matches(notification.priority),
            RuleCondition::SourceDevice(d) => &notification.source_device == d,
            RuleCondition::Metadata { key, value } => {
                notification
                    .metadata
                    .get(key)
                    .map(|v| value.matches(v))
                    .unwrap_or(false)
            }
        }
    }
}

/// String matching options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StringMatch {
    Exact(String),
    Contains(String),
    StartsWith(String),
    EndsWith(String),
    Regex(String),
}

impl StringMatch {
    pub fn matches(&self, value: &str) -> bool {
        match self {
            StringMatch::Exact(s) => value == s,
            StringMatch::Contains(s) => value.contains(s),
            StringMatch::StartsWith(s) => value.starts_with(s),
            StringMatch::EndsWith(s) => value.ends_with(s),
            StringMatch::Regex(pattern) => {
                Regex::new(pattern)
                    .map(|r| r.is_match(value))
                    .unwrap_or(false)
            }
        }
    }
}

/// Priority matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PriorityMatch {
    Exact(Priority),
    AtLeast(Priority),
    AtMost(Priority),
}

impl PriorityMatch {
    pub fn matches(&self, priority: Priority) -> bool {
        match self {
            PriorityMatch::Exact(p) => priority == *p,
            PriorityMatch::AtLeast(p) => priority >= *p,
            PriorityMatch::AtMost(p) => priority <= *p,
        }
    }
}

/// Quiet hours configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuietHours {
    pub start_hour: u8,
    pub start_minute: u8,
    pub end_hour: u8,
    pub end_minute: u8,
    pub days: Vec<chrono::Weekday>,
}

impl QuietHours {
    pub fn new(start_hour: u8, start_minute: u8, end_hour: u8, end_minute: u8) -> Self {
        Self {
            start_hour,
            start_minute,
            end_hour,
            end_minute,
            days: vec![
                chrono::Weekday::Mon,
                chrono::Weekday::Tue,
                chrono::Weekday::Wed,
                chrono::Weekday::Thu,
                chrono::Weekday::Fri,
                chrono::Weekday::Sat,
                chrono::Weekday::Sun,
            ],
        }
    }

    pub fn is_active(&self) -> bool {
        use chrono::Timelike;

        let now = chrono::Local::now();
        let weekday = now.weekday();

        if !self.days.contains(&weekday) {
            return false;
        }

        let current_minutes = now.hour() * 60 + now.minute();
        let start_minutes = self.start_hour as u32 * 60 + self.start_minute as u32;
        let end_minutes = self.end_hour as u32 * 60 + self.end_minute as u32;

        if start_minutes < end_minutes {
            // Same day range
            current_minutes >= start_minutes && current_minutes < end_minutes
        } else {
            // Overnight range
            current_minutes >= start_minutes || current_minutes < end_minutes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_match() {
        assert!(StringMatch::Exact("test".to_string()).matches("test"));
        assert!(!StringMatch::Exact("test".to_string()).matches("Test"));

        assert!(StringMatch::Contains("est".to_string()).matches("test"));
        assert!(!StringMatch::Contains("xyz".to_string()).matches("test"));

        assert!(StringMatch::StartsWith("te".to_string()).matches("test"));
        assert!(StringMatch::EndsWith("st".to_string()).matches("test"));
    }

    #[test]
    fn test_filter_rule() {
        let rule = FilterRule::block("Block Spam")
            .with_condition(RuleCondition::AppName(StringMatch::Contains("Spam".to_string())));

        let spam_notification = Notification::new("Spam App", "Buy Now!", "Click here");
        let normal_notification = Notification::new("GitHub", "PR Review", "Review requested");

        assert_eq!(rule.matches(&spam_notification), RuleResult::Block);
        assert_eq!(rule.matches(&normal_notification), RuleResult::NoMatch);
    }

    #[test]
    fn test_priority_match() {
        assert!(PriorityMatch::AtLeast(Priority::High).matches(Priority::Critical));
        assert!(PriorityMatch::AtLeast(Priority::High).matches(Priority::High));
        assert!(!PriorityMatch::AtLeast(Priority::High).matches(Priority::Normal));
    }
}
