#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickKind {
    Normal,
    Snap,
}

/// Longest elapsed time still treated as a normal tick rather than a snap.
const MAX_NORMAL_GAP_MILLIS: i64 = 2_000;

/// Classifies the elapsed time since the previous tick. The first tick and any
/// gap outside `0..=2000` ms snap without animation.
pub fn classify_tick(previous_millis: Option<i64>, current_millis: i64) -> TickKind {
    let Some(previous) = previous_millis else {
        return TickKind::Snap;
    };
    let gap = current_millis.saturating_sub(previous);
    if (0..=MAX_NORMAL_GAP_MILLIS).contains(&gap) {
        TickKind::Normal
    } else {
        TickKind::Snap
    }
}

/// Milliseconds from `now_millis` to the next whole second, never zero.
pub fn millis_until_next_second(now_millis: i64) -> u64 {
    const SECOND_MILLIS: u64 = 1_000;
    let remainder = now_millis.rem_euclid(SECOND_MILLIS as i64) as u64;
    if remainder == 0 {
        SECOND_MILLIS
    } else {
        SECOND_MILLIS - remainder
    }
}
