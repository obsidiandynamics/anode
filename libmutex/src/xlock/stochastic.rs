use std::time::Duration;
use crate::deadline::Deadline;
use crate::inf_iterator::{InfIterator};
use crate::monitor::{Directive, Monitor, SpeculativeMonitor};
use crate::rand::{Rand64, Seeded, Xorshift, CyclicSeed};
use crate::xlock::{Moderator};

#[derive(Debug)]
pub struct Stochastic;

pub struct StochasticSync {
    monitor: SpeculativeMonitor<StochasticState>,
}

#[derive(Debug)]
struct StochasticState {
    readers: u32,
    writer: bool,
    writer_pending: bool,
    queued: u32,
    seed: CyclicSeed,
}

impl StochasticState {
    #[inline]
    fn enqueue(&mut self) -> u32 {
        let next = self.queued;
        self.queued = next + 1;
        next
    }
}

impl Moderator for Stochastic {
    type Sync = StochasticSync;

    #[inline]
    fn new() -> Self::Sync {
        Self::Sync {
            monitor: SpeculativeMonitor::new(StochasticState {
                readers: 0,
                writer: false,
                writer_pending: false,
                queued: 0,
                seed: CyclicSeed::default()
            }),
        }
    }

    #[inline]
    fn try_read(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut acquired = false;
        let mut saw_no_pending_writer = false;
        let mut privilege_determined = false;
        let mut position = None;
        sync.monitor.enter(|state| {
            if !acquired {
                if !saw_no_pending_writer {
                    if position.is_none() {
                        position = Some(state.enqueue());
                    }

                    if !state.writer_pending {
                        saw_no_pending_writer = true;
                    } else if !privilege_determined {
                        privilege_determined = true;
                        let position = position.unwrap();
                        let p_privileged = 1.0 / (position as f64 + 1.0);
                        let mut rng = Xorshift::seed(state.seed.next());
                        if rng.gen_bool(p_privileged.into()) {
                            saw_no_pending_writer = true
                        }
                    }
                }

                if !state.writer && saw_no_pending_writer {
                    acquired = true;
                    state.readers += 1;
                }
            }

            if acquired {
                state.queued -= 1;
                Directive::Return
            } else {
                Directive::Wait(deadline.remaining())
            }
        });


        if !acquired {
            sync.monitor.alter(|state| {
                state.queued -= 1;
            })
        }

        acquired
    }

    #[inline]
    fn read_unlock(sync: &Self::Sync) {
        let mut released = false;
        sync.monitor.enter(|state| {
            if !released {
                debug_assert!(state.readers > 0, "readers: {}", state.readers);
                debug_assert!(!state.writer);

                released = true;
                state.readers -= 1;
            }

            match state.readers {
                0 | 1 => Directive::NotifyAll,
                _ => Directive::Return
            }
        });
    }

    #[inline]
    fn try_write(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut acquired = false;
        let mut self_writer_pending = false;
        sync.monitor.enter(|state| {
            if !acquired {
                if state.readers == 0 && !state.writer {
                    state.writer = true;
                    acquired = true;
                } else if !state.writer_pending {
                    self_writer_pending = true;
                    state.writer_pending = true;
                }
            }

            if acquired {
                Directive::Return
            } else {
                Directive::Wait(deadline.remaining())
            }
        });

        if self_writer_pending {
            let mut cleared_writer_pending = false;
            sync.monitor.enter(|state| {
                if !cleared_writer_pending {
                    cleared_writer_pending = true;
                    state.writer_pending = false;
                }

                if acquired {
                    Directive::Return
                } else {
                    Directive::NotifyAll
                }
            });
        }

        acquired
    }

    #[inline]
    fn write_unlock(sync: &Self::Sync) {
        let mut released = false;
        sync.monitor.enter(|state| {
            if !released {
                debug_assert!(state.readers == 0, "readers: {}", state.readers);
                debug_assert!(state.writer);

                released = true;
                state.writer = false;
            }

            Directive::NotifyAll
        });
    }

    fn downgrade(sync: &Self::Sync) {
        let mut released = false;
        sync.monitor.enter(|state| {
            if !released {
                debug_assert!(state.readers == 0, "readers: {}", state.readers);
                debug_assert!(state.writer);

                released = true;
                state.writer = false;
                state.readers = 1;
            }

            Directive::NotifyAll
        });
    }

    fn try_upgrade(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut acquired = false;
        let mut self_writer_pending = false;
        sync.monitor.enter(|state| {
            if !acquired {
                debug_assert!(!state.writer);

                if state.readers == 1 {
                    acquired = true;
                    state.readers = 0;
                    state.writer = true;
                } else if !state.writer_pending {
                    self_writer_pending = true;
                    state.writer_pending = true;
                }
            }

            if acquired {
                Directive::Return
            } else {
                Directive::Wait(deadline.remaining())
            }
        });

        if self_writer_pending {
            let mut cleared_writer_pending = false;
            sync.monitor.enter(|state| {
                if !cleared_writer_pending {
                    cleared_writer_pending = true;
                    state.writer_pending = false;
                }

                if acquired {
                    Directive::Return
                } else {
                    Directive::NotifyAll
                }
            });
        }

        acquired
    }
}
