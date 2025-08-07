use std::ops::{Add, Sub};

/// Cross-platform Duration wrapper
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    #[cfg(not(target_arch = "wasm32"))]
    inner: std::time::Duration,
    #[cfg(target_arch = "wasm32")]
    millis: u64,
}

impl Duration {
    pub fn as_secs_f32(&self) -> f32 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.as_secs_f32()
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.millis as f32 / 1000.0
        }
    }

    pub fn as_millis(&self) -> u128 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.as_millis()
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.millis as u128
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<std::time::Duration> for Duration {
    fn from(duration: std::time::Duration) -> Self {
        Self { inner: duration }
    }
}

#[cfg(target_arch = "wasm32")]
impl Duration {
    fn from_millis(millis: f64) -> Self {
        Self {
            millis: millis as u64,
        }
    }
}

impl Add for Duration {
    type Output = Duration;

    fn add(self, other: Duration) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Duration {
                inner: self.inner + other.inner,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Duration {
                millis: self.millis + other.millis,
            }
        }
    }
}

impl Sub for Duration {
    type Output = Duration;

    fn sub(self, other: Duration) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Duration {
                inner: self.inner - other.inner,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Duration {
                millis: self.millis.saturating_sub(other.millis),
            }
        }
    }
}

/// Cross-platform Instant wrapper
#[derive(Debug, Clone, Copy)]
pub struct Instant {
    #[cfg(not(target_arch = "wasm32"))]
    inner: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    millis: f64,
}

impl Instant {
    pub fn now() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                inner: std::time::Instant::now(),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self {
                millis: web_sys::window()
                    .expect("should have a window in this context")
                    .performance()
                    .expect("should have performance in this context")
                    .now(),
            }
        }
    }
}

impl Sub for Instant {
    type Output = Duration;

    fn sub(self, other: Instant) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Duration {
                inner: self.inner - other.inner,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Duration::from_millis((self.millis - other.millis).max(0.0))
        }
    }
}
