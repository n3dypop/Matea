#![forbid(unsafe_code)]
//! Deterministic motion without a runtime. Spring equations are informed by
//! Harmonica and Ryan Juckett; this is an altered Rust implementation (ADR 0010).
use std::{num::NonZeroU32, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionError {
    InvalidParameters,
    NonFiniteState,
}
impl std::fmt::Display for MotionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidParameters => {
                "motion parameters must be finite, nonnegative and representable"
            }
            Self::NonFiniteState => "motion state is not finite or the step overflowed",
        })
    }
}
impl std::error::Error for MotionError {}
/// Fixed frame duration rounded down to nanoseconds. A rate above 1GHz yields
/// zero duration; it cannot divide by zero or wrap a signed frame count.
pub fn frame_duration(fps: NonZeroU32) -> Duration {
    Duration::from_nanos(1_000_000_000 / u64::from(fps.get()))
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Motion {
    pub position: f64,
    pub velocity: f64,
}
impl Motion {
    /// Respect reduced motion without introducing a dependency on theme state.
    pub fn at_rest(target: f64) -> Result<Self, MotionError> {
        if !target.is_finite() {
            return Err(MotionError::NonFiniteState);
        }
        Ok(Self {
            position: target,
            velocity: 0.0,
        })
    }
}
/// Cached transition matrix for a fixed time step. Copy and reuse for many
/// animated values. Frequency is angular frequency (radians per second).
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    p: f64,
    q: f64,
    r: f64,
    s: f64,
}
impl Spring {
    pub fn new(step: Duration, frequency: f64, damping: f64) -> Result<Self, MotionError> {
        if !frequency.is_finite() || !damping.is_finite() || frequency < 0.0 || damping < 0.0 {
            return Err(MotionError::InvalidParameters);
        }
        let dt = step.as_secs_f64();
        if frequency == 0.0 || dt == 0.0 {
            return Ok(Self {
                p: 1.0,
                q: 0.0,
                r: 0.0,
                s: 1.0,
            });
        }
        let w = frequency;
        let a = w * damping;
        let (p, q, r, s) = if damping < 1.0 {
            let b = w * ((1.0 - damping) * (1.0 + damping)).sqrt();
            let theta = b * dt;
            let sinc = if theta.abs() < 1e-8 {
                dt * (1.0 - theta * theta / 6.0)
            } else {
                theta.sin() / b
            };
            let decay = (-a * dt).exp();
            let q = decay * sinc;
            let cos = decay * theta.cos();
            (cos + a * q, q, -w * (w * q), cos - a * q)
        } else if damping == 1.0 {
            let theta = w * dt;
            let decay = (-theta).exp();
            if decay == 0.0 {
                (0.0, 0.0, 0.0, 0.0)
            } else {
                (
                    (1.0 + theta) * decay,
                    dt * decay,
                    -w * (theta * decay),
                    (1.0 - theta) * decay,
                )
            }
        } else {
            let root = ((damping - 1.0) * (damping + 1.0)).sqrt();
            let sum = damping + root;
            let slow = -w / sum;
            let fast = -w * sum;
            let denominator = slow - fast;
            let e_s = (slow * dt).exp();
            let e_f = (fast * dt).exp();
            let q = (e_s - e_f) / denominator;
            (
                (-fast * e_s + slow * e_f) / denominator,
                q,
                -w * (w * q),
                (slow * e_s - fast * e_f) / denominator,
            )
        };
        if ![p, q, r, s].iter().all(|n| n.is_finite()) {
            return Err(MotionError::InvalidParameters);
        }
        Ok(Self { p, q, r, s })
    }
    /// Evaluate the cached matrix with IEEE floating-point semantics, as in the
    /// reference numeric primitive. Nonfinite inputs/overflow can produce NaN or
    /// infinity; use step when the application needs explicit validation errors.
    /// This operation performs no allocation, validation or global state access.
    pub fn sample(self, state: Motion, target: f64) -> Motion {
        let displacement = state.position - target;
        Motion {
            position: self.p * displacement + self.q * state.velocity + target,
            velocity: self.r * displacement + self.s * state.velocity,
        }
    }
    pub fn step(self, state: Motion, target: f64) -> Result<Motion, MotionError> {
        if ![state.position, state.velocity, target]
            .iter()
            .all(|n| n.is_finite())
        {
            return Err(MotionError::NonFiniteState);
        }
        let Motion { position, velocity } = self.sample(state, target);
        if !position.is_finite() || !velocity.is_finite() {
            return Err(MotionError::NonFiniteState);
        }
        Ok(Motion { position, velocity })
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
pub const GRAVITY: Vector3 = Vector3 {
    x: 0.0,
    y: -9.81,
    z: 0.0,
};
pub const TERMINAL_GRAVITY: Vector3 = Vector3 {
    x: 0.0,
    y: 9.81,
    z: 0.0,
};
#[derive(Clone, Copy, Debug)]
pub struct Projectile {
    position: Point3,
    velocity: Vector3,
    acceleration: Vector3,
    dt: f64,
}
impl Projectile {
    pub fn new(
        step: Duration,
        position: Point3,
        velocity: Vector3,
        acceleration: Vector3,
    ) -> Result<Self, MotionError> {
        if ![
            position.x,
            position.y,
            position.z,
            velocity.x,
            velocity.y,
            velocity.z,
            acceleration.x,
            acceleration.y,
            acceleration.z,
        ]
        .iter()
        .all(|n| n.is_finite())
        {
            return Err(MotionError::NonFiniteState);
        }
        Ok(Self {
            position,
            velocity,
            acceleration,
            dt: step.as_secs_f64(),
        })
    }
    #[inline]
    pub fn position(self) -> Point3 {
        self.position
    }
    #[inline]
    pub fn velocity(self) -> Vector3 {
        self.velocity
    }
    #[inline]
    pub fn acceleration(self) -> Vector3 {
        self.acceleration
    }
    /// Reference-compatible forward Euler integration. Errors leave state intact.
    #[inline]
    pub fn step(&mut self) -> Result<Point3, MotionError> {
        self.commit(self.sample())
    }
    /// Copy the state and take one forward Euler step with IEEE arithmetic.
    /// This matches the reference's unchecked numeric operation. Overflow may
    /// produce infinity/NaN; use step for a checked, transactional mutation.
    #[inline]
    pub fn sample(self) -> Self {
        let p = self.position;
        let v = self.velocity;
        let a = self.acceleration;
        Self {
            position: Point3 {
                x: p.x + v.x * self.dt,
                y: p.y + v.y * self.dt,
                z: p.z + v.z * self.dt,
            },
            velocity: Vector3 {
                x: v.x + a.x * self.dt,
                y: v.y + a.y * self.dt,
                z: v.z + a.z * self.dt,
            },
            ..self
        }
    }
    /// Analytical constant-acceleration step. Independent of time partition to
    /// floating-point tolerance; useful when frames arrive at varying intervals.
    pub fn step_exact(&mut self) -> Result<Point3, MotionError> {
        let dt = self.dt;
        let a = self.acceleration;
        let v = self.velocity;
        let p = self.position;
        let extra = 0.5 * dt;
        let position = Point3 {
            x: p.x + (v.x + a.x * extra) * dt,
            y: p.y + (v.y + a.y * extra) * dt,
            z: p.z + (v.z + a.z * extra) * dt,
        };
        let velocity = Vector3 {
            x: v.x + a.x * dt,
            y: v.y + a.y * dt,
            z: v.z + a.z * dt,
        };
        self.commit(Self {
            position,
            velocity,
            ..*self
        })
    }
    #[inline]
    fn commit(&mut self, next: Self) -> Result<Point3, MotionError> {
        let p = next.position;
        let v = next.velocity;
        // Independent, side-effect-free checks can be combined without branchy
        // short-circuit iteration. Keep the commit after all six checks.
        if !(p.x.is_finite()
            & p.y.is_finite()
            & p.z.is_finite()
            & v.x.is_finite()
            & v.y.is_finite()
            & v.z.is_finite())
        {
            return Err(MotionError::NonFiniteState);
        }
        *self = next;
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }
    #[test]
    fn analytical_oscillator_and_all_damping_regimes() {
        let state = Motion {
            position: 2.0,
            velocity: 0.0,
        };
        let t = Duration::from_millis(125);
        let w = 4.0;
        let undamped = Spring::new(t, w, 0.0).unwrap().step(state, 1.0).unwrap();
        near(undamped.position, 1.0 + 0.5f64.cos());
        near(undamped.velocity, -4.0 * 0.5f64.sin());
        let critical = Spring::new(t, w, 1.0).unwrap().step(state, 1.0).unwrap();
        near(critical.position, 1.0 + 1.5 * (-0.5f64).exp());
        near(critical.velocity, -2.0 * (-0.5f64).exp());
        for damping in [0.2, 0.999999, 1.0, 1.000001, 2.0, 1e6] {
            let spring = Spring::new(t, w, damping).unwrap();
            let half = Spring::new(t / 2, w, damping).unwrap();
            let once = spring.step(state, 1.0).unwrap();
            let twice = half.step(half.step(state, 1.0).unwrap(), 1.0).unwrap();
            near(once.position, twice.position);
            near(once.velocity, twice.velocity);
            assert!(once.position.is_finite());
        }
    }
    #[test]
    fn rejects_invalid_states_and_identity_and_rest_are_explicit() {
        assert!(Spring::new(Duration::from_secs(1), -1.0, 0.0).is_err());
        assert!(Spring::new(Duration::ZERO, f64::NAN, 0.0).is_err());
        let state = Motion {
            position: 3.0,
            velocity: 7.0,
        };
        assert_eq!(
            Spring::new(Duration::from_secs(1), 0.0, 1.0)
                .unwrap()
                .step(state, 0.0),
            Ok(state)
        );
        assert_eq!(
            Motion::at_rest(2.0),
            Ok(Motion {
                position: 2.0,
                velocity: 0.0
            })
        );
        assert!(Motion::at_rest(f64::INFINITY).is_err());
        let spring = Spring::new(Duration::from_millis(16), 12.0, 0.7).unwrap();
        assert_eq!(spring.step(state, 1.0).unwrap(), spring.sample(state, 1.0));
        let invalid = Motion {
            position: f64::NAN,
            velocity: 0.0,
        };
        assert!(spring.step(invalid, 1.0).is_err());
        assert!(spring.sample(invalid, 1.0).position.is_nan());
        assert_eq!(
            frame_duration(NonZeroU32::new(60).unwrap()).as_nanos(),
            16_666_666
        );
    }
    #[test]
    fn projectile_matches_forward_euler_and_exact_partition() {
        let p = Point3 {
            x: 1.0,
            y: 10.0,
            z: 2.0,
        };
        let v = Vector3 {
            x: 2.0,
            y: 0.0,
            z: 1.0,
        };
        let mut euler = Projectile::new(Duration::from_millis(100), p, v, GRAVITY).unwrap();
        near(euler.step().unwrap().y, 10.0);
        near(euler.step().unwrap().y, 9.9019);
        let mut one = Projectile::new(Duration::from_secs(1), p, v, GRAVITY).unwrap();
        let mut many = Projectile::new(Duration::from_millis(10), p, v, GRAVITY).unwrap();
        one.step_exact().unwrap();
        for _ in 0..100 {
            many.step_exact().unwrap();
        }
        near(one.position().x, many.position().x);
        near(one.position().y, many.position().y);
        near(one.velocity().y, many.velocity().y);
        near(one.position().y, 5.095);
        let mut overflow = Projectile::new(
            Duration::from_secs(100),
            p,
            Vector3 { x: f64::MAX, ..v },
            GRAVITY,
        )
        .unwrap();
        assert!(overflow.step().is_err());
        assert_eq!(overflow.position(), p);
        assert!(overflow.sample().position().x.is_infinite());
        assert_eq!(overflow.position(), p);
    }
    #[test]
    fn frame_duration_boundaries_and_zero_steps() {
        for (fps, nanos) in [
            (1, 1_000_000_000),
            (60, 16_666_666),
            (1_000_000_000, 1),
            (u32::MAX, 0),
        ] {
            assert_eq!(
                frame_duration(NonZeroU32::new(fps).unwrap()).as_nanos(),
                nanos
            );
        }
        let mut p = Projectile::new(
            Duration::ZERO,
            Point3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            Vector3 {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            },
            TERMINAL_GRAVITY,
        )
        .unwrap();
        let original = p;
        assert_eq!(p.step().unwrap(), original.position());
        assert_eq!(p.velocity(), original.velocity());
        assert_eq!(p.acceleration(), TERMINAL_GRAVITY);
        assert!(
            Projectile::new(
                Duration::ZERO,
                Point3 {
                    x: f64::NAN,
                    ..Point3::default()
                },
                Vector3::default(),
                GRAVITY
            )
            .is_err()
        );
    }
    #[test]
    fn extreme_overdamping_keeps_the_slow_decay_instead_of_cancelling_it() {
        let state = Motion {
            position: 2.0,
            velocity: 0.0,
        };
        let result = Spring::new(Duration::from_millis(16), 12.0, 1e12)
            .unwrap()
            .step(state, 1.0)
            .unwrap();
        assert!(result.position > 1.999999999999);
        // At this scale the fast transient has decayed. The slow eigenvalue is
        // approximately -omega/(2*damping), which must not cancel to zero.
        assert!((result.velocity / -6e-12 - 1.0).abs() < 1e-10);
    }
    #[test]
    fn velocity_overflow_cannot_partially_commit_position() {
        let mut projectile = Projectile::new(
            Duration::from_secs(2),
            Point3::default(),
            Vector3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            Vector3 {
                x: 0.0,
                y: f64::MAX,
                z: 0.0,
            },
        )
        .unwrap();
        let original = projectile;
        assert_eq!(projectile.step(), Err(MotionError::NonFiniteState));
        assert_eq!(projectile.position(), original.position());
        assert_eq!(projectile.velocity(), original.velocity());
        assert!(projectile.sample().velocity().y.is_infinite());
        assert_eq!(projectile.sample().position().x, 2.0);
    }
}
