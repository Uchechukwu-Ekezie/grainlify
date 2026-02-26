// contracts/program-escrow/src/threshold_monitor.rs
//
// Threshold Monitor Module
//
// Implements automatic circuit breaker triggers based on configurable thresholds
// for failure rates and token outflow volumes. Monitors operations in sliding
// time windows and opens the circuit breaker when abnormal patterns are detected.

use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

// ─────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────

/// Configuration for threshold-based circuit breaking
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThresholdConfig {
    /// Maximum failures allowed per time window
    pub failure_rate_threshold: u32,
    /// Maximum outflow amount per time window
    pub outflow_volume_threshold: i128,
    /// Maximum amount for a single payout transaction
    pub max_single_payout: i128,
    /// Time window duration in seconds
    pub time_window_secs: u64,
    /// Minimum cooldown period before reopening (seconds)
    pub cooldown_period_secs: u64,
    /// Backoff multiplier for repeated breaches
    pub cooldown_multiplier: u32,
}

impl ThresholdConfig {
    /// Default configuration with conservative thresholds
    pub fn default() -> Self {
        ThresholdConfig {
            failure_rate_threshold: 10,
            outflow_volume_threshold: 5_000_000_0000000, // 5M tokens (7 decimals)
            max_single_payout: 500_000_0000000,          // 500K tokens
            time_window_secs: 600,                       // 10 minutes
            cooldown_period_secs: 300,                   // 5 minutes
            cooldown_multiplier: 2,
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.failure_rate_threshold == 0 || self.failure_rate_threshold > 1000 {
            return Err("Failure threshold must be between 1 and 1000");
        }
        if self.outflow_volume_threshold <= 0 {
            return Err("Outflow threshold must be greater than zero");
        }

        if self.max_single_payout <= 0 {
            return Err("Max single payout must be greater than zero");
        }
        if self.time_window_secs < 10 || self.time_window_secs > 86400 {
            return Err("Time window must be between 10 and 86400 seconds");
        }
        if self.cooldown_period_secs < 60 || self.cooldown_period_secs > 3600 {
            return Err("Cooldown period must be between 60 and 3600 seconds");
        }
        Ok(())
    }
}

/// Current metrics for a time window
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowMetrics {
    /// Window start timestamp
    pub window_start: u64,
    /// Failures in current window
    pub failure_count: u32,
    /// Successes in current window
    pub success_count: u32,
    /// Total outflow in current window
    pub total_outflow: i128,
    /// Largest single outflow in window
    pub max_single_outflow: i128,
    /// Number of times thresholds breached
    pub breach_count: u32,
}

impl WindowMetrics {
    pub fn new(window_start: u64) -> Self {
        WindowMetrics {
            window_start,
            failure_count: 0,
            success_count: 0,
            total_outflow: 0,
            max_single_outflow: 0,
            breach_count: 0,
        }
    }
}

/// Threshold breach information
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThresholdBreach {
    /// Type of metric that breached ("failure" or "outflow")
    pub metric_type: Symbol,
    /// Configured threshold value
    pub threshold_value: i128,
    /// Actual value that breached
    pub actual_value: i128,
    /// When breach occurred
    pub timestamp: u64,
    /// Total breaches in this window
    pub breach_count: u32,
}

/// Storage keys for threshold monitoring
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ThresholdKey {
    Config,
    CurrentMetrics,
    PreviousMetrics,
    LastCooldownEnd,
    CooldownMultiplier,
}

// ─────────────────────────────────────────────────────────
// Error codes
// ─────────────────────────────────────────────────────────

pub const ERR_THRESHOLD_BREACHED: u32 = 2001;
pub const ERR_INVALID_THRESHOLD_CONFIG: u32 = 2002;
pub const ERR_COOLDOWN_ACTIVE: u32 = 2003;
pub const ERR_WINDOW_NOT_EXPIRED: u32 = 2004;
