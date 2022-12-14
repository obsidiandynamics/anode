// Copyright 2016 Amanieu d'Antras
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use anode::zlock::{ArrivalOrdered, ReadBiased, Stochastic, WriteBiased, ZLock};
use crate::pl_harness::RwLock;

pub struct ReadBiasedLock<T>(ZLock<T, ReadBiased>);
pub struct WriteBiasedLock<T>(ZLock<T, WriteBiased>);
pub struct ArrivalOrderedLock<T>(ZLock<T, ArrivalOrdered>);
pub struct StochasticLock<T>(ZLock<T, Stochastic>);
pub struct ParkingLotLock<T>(parking_lot::RwLock<T>);
pub struct StdLock<T>(std::sync::RwLock<T>);

impl<T> RwLock<T> for StdLock<T> {
    fn new(v: T) -> Self {
        Self(std::sync::RwLock::new(v))
    }

    fn read<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&T) -> R,
    {
        f(&*self.0.read().unwrap())
    }

    fn write<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&mut T) -> R,
    {
        f(&mut *self.0.write().unwrap())
    }

    fn name() -> &'static str {
        "std::sync::RwLock"
    }
}

impl<T> RwLock<T> for ReadBiasedLock<T> {
    fn new(v: T) -> Self {
        Self(ZLock::new(v))
    }

    fn read<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&T) -> R,
    {
        f(&*self.0.read())
    }

    fn write<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&mut T) -> R,
    {
        f(&mut *self.0.write())
    }

    fn name() -> &'static str {
        "anode::rwlock::RwLock<ReadBiased>"
    }
}

impl<T> RwLock<T> for WriteBiasedLock<T> {
    fn new(v: T) -> Self {
        Self(ZLock::new(v))
    }

    fn read<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&T) -> R,
    {
        f(&*self.0.read())
    }

    fn write<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&mut T) -> R,
    {
        f(&mut *self.0.write())
    }

    fn name() -> &'static str {
        "anode::rwlock::RwLock<WriteBiased>"
    }
}

impl<T> RwLock<T> for ArrivalOrderedLock<T> {
    fn new(v: T) -> Self {
        Self(ZLock::new(v))
    }

    fn read<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&T) -> R,
    {
        f(&*self.0.read())
    }

    fn write<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&mut T) -> R,
    {
        f(&mut *self.0.write())
    }

    fn name() -> &'static str {
        "anode::rwlock::RwLock<ArrivalOrdered>"
    }
}

impl<T> RwLock<T> for StochasticLock<T> {
    fn new(v: T) -> Self {
        Self(ZLock::new(v))
    }

    fn read<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&T) -> R,
    {
        f(&*self.0.read())
    }

    fn write<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&mut T) -> R,
    {
        f(&mut *self.0.write())
    }

    fn name() -> &'static str {
        "anode::rwlock::RwLock<Stochastic>"
    }
}

impl<T> RwLock<T> for ParkingLotLock<T> {
    fn new(v: T) -> Self {
        Self(parking_lot::RwLock::new(v))
    }

    fn read<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&T) -> R,
    {
        f(&*self.0.read())
    }

    fn write<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&mut T) -> R,
    {
        f(&mut *self.0.write())
    }

    fn name() -> &'static str {
        "parking_lot::RwLock"
    }
}