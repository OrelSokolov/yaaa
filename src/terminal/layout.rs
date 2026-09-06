use std::time::{Duration, Instant};

/// How long to wait after the last layout change before flushing it to the
/// settings file. Resizes generate a stream of changes; this debounces them.
const SAVE_DEBOUNCE: Duration = Duration::from_millis(600);

/// Resolution (in pixels) below which layout changes are treated as noise
/// from floating-point rounding rather than a real resize.
const LAYOUT_EPSILON: f32 = 1.0;

/// Tracks the persisted terminal content size and font cell metrics so new
/// terminals boot at the correct column/row count on cold start, and debounces
/// writing them to disk during a resize.
pub struct TerminalLayoutTracker {
    last_layout: Option<[f32; 2]>,
    last_cell_metrics: Option<[f32; 2]>,
    save_at: Option<Instant>,
}

impl TerminalLayoutTracker {
    pub fn new(
        last_layout: Option<[f32; 2]>,
        last_cell_metrics: Option<[f32; 2]>,
    ) -> Self {
        Self {
            last_layout,
            last_cell_metrics,
            save_at: None,
        }
    }

    /// Last observed terminal content size [w, h] (to persist in settings).
    pub fn last_layout(&self) -> Option<[f32; 2]> {
        self.last_layout
    }

    /// Last observed font cell metrics [w, h] (to persist in settings).
    pub fn last_cell_metrics(&self) -> Option<[f32; 2]> {
        self.last_cell_metrics
    }

    /// Record the current terminal content size. Returns `true` when it counts
    /// as a real resize (first observation or moved by more than
    /// `LAYOUT_EPSILON` on either axis); a real resize restarts the save
    /// debounce timer.
    pub fn note_layout(&mut self, current: [f32; 2]) -> bool {
        let changed = self.last_layout.is_none_or(|prev| {
            (prev[0] - current[0]).abs() > LAYOUT_EPSILON
                || (prev[1] - current[1]).abs() > LAYOUT_EPSILON
        });
        if changed {
            self.last_layout = Some(current);
            self.save_at = Some(Instant::now());
        }
        changed
    }

    /// Record font cell metrics (first measurement or after a font/theme
    /// change) and schedule a debounced save.
    pub fn note_cell_metrics(&mut self, metrics: [f32; 2]) {
        self.last_cell_metrics = Some(metrics);
        self.save_at = Some(Instant::now());
    }

    /// Consume the debounce timer once it has expired. Returns `true` exactly
    /// once per change burst, when the values should be flushed to disk.
    pub fn should_flush(&mut self) -> bool {
        if let Some(when) = self.save_at {
            if when.elapsed() > SAVE_DEBOUNCE {
                self.save_at = None;
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_layout_observation_counts_as_change() {
        let mut t = TerminalLayoutTracker::new(None, None);
        assert!(t.note_layout([640.0, 480.0]));
        assert_eq!(t.last_layout(), Some([640.0, 480.0]));
    }

    #[test]
    fn subpixel_drift_is_not_a_change() {
        let mut t = TerminalLayoutTracker::new(Some([640.0, 480.0]), None);
        assert!(!t.note_layout([640.4, 479.6]));
        assert_eq!(t.last_layout(), Some([640.0, 480.0]));
    }

    #[test]
    fn real_resize_is_a_change_and_updates_last() {
        let mut t = TerminalLayoutTracker::new(Some([640.0, 480.0]), None);
        assert!(t.note_layout([800.0, 480.0]));
        assert_eq!(t.last_layout(), Some([800.0, 480.0]));
    }

    #[test]
    fn cell_metrics_are_stored() {
        let mut t = TerminalLayoutTracker::new(None, None);
        t.note_cell_metrics([7.2, 14.0]);
        assert_eq!(t.last_cell_metrics(), Some([7.2, 14.0]));
    }

    #[test]
    fn flush_waits_for_debounce_and_fires_once() {
        let mut t = TerminalLayoutTracker::new(None, None);
        t.note_layout([100.0, 100.0]);
        // Freshly stamped timer: not yet.
        assert!(!t.should_flush());
        // Simulate the timer having been stamped in the past.
        t.save_at = Instant::now().checked_sub(Duration::from_secs(1));
        assert!(t.should_flush());
        // Fires exactly once per burst.
        assert!(!t.should_flush());
    }

    #[test]
    fn resize_restarts_the_debounce() {
        let mut t = TerminalLayoutTracker::new(None, None);
        t.note_layout([100.0, 100.0]);
        t.save_at = Instant::now().checked_sub(Duration::from_secs(1));
        // A new resize arrives before the flush: the timer restarts.
        t.note_layout([200.0, 100.0]);
        assert!(!t.should_flush());
    }
}
