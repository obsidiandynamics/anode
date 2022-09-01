use std::time::Duration;
use crate::deadline::Deadline;
use crate::monitor::{Directive, Monitor, SpeculativeMonitor};
use crate::zlock::{Moderator};

#[derive(Debug)]
pub struct WriteBiased;

pub struct WriteBiasedSync {
    monitor: SpeculativeMonitor<WriteBiasedState>,
}

#[derive(Debug)]
struct WriteBiasedState {
    readers: u32,
    writer: bool,
    writer_pending: bool,
}

impl Moderator for WriteBiased {
    type Sync = WriteBiasedSync;

    #[inline]
    fn new() -> Self::Sync {
        Self::Sync {
            monitor: SpeculativeMonitor::new(WriteBiasedState { readers: 0, writer: false, writer_pending: false }),
        }
    }

    #[inline]
    fn try_read(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut acquired = false;
        let mut saw_no_pending_writer = false;
        sync.monitor.enter(|state| {
            if !state.writer_pending {
                saw_no_pending_writer = true;
            }

            if !acquired && !state.writer && saw_no_pending_writer {
                acquired = true;
                state.readers += 1;
            }

            if acquired {
                Directive::Return
            } else {
                Directive::Wait(deadline.remaining())
            }
        });
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

#[cfg(test)]
mod tests;
