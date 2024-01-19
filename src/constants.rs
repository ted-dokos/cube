use std::{cell::LazyCell, time::Duration};

const GAME_TICKS_PER_SECOND: f32 = 25.0;
pub const TIME_PER_GAME_TICK: LazyCell<Duration> =
    LazyCell::new(|| Duration::from_secs_f32(1.0 / GAME_TICKS_PER_SECOND));
const MAX_RENDER_FPS: f32 = 100.0;
pub const MIN_TIME_PER_RENDER_FRAME: LazyCell<Duration> =
    LazyCell::new(|| Duration::from_secs_f32(1.0 / MAX_RENDER_FPS));
