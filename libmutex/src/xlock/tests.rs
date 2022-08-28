// use std::ops::{Deref, DerefMut};
// use std::time::Duration;
// use crate::xlock::{LockReadGuard, LockWriteGuard, ReadBiased, Spec, XLock};
//
// pub type LockBox<T> = Box<dyn Locklike<T>>;
//
// pub trait Locklike<T: ?Sized> {
//     fn read(&self) -> DynLockReadGuard<T>;
//
//     fn try_read(&self, duration: Duration) -> Option<DynLockReadGuard<T>>;
//
//     fn write(&self) -> DynLockWriteGuard<T>;
//
//     fn try_write(&self, duration: Duration) -> Option<DynLockWriteGuard<T>>;
// }
//
// trait LockReadGuardlike<'a, T: ?Sized>: Deref<Target = T> {}
//
// impl<'a, T: ?Sized, S: Spec> LockReadGuardlike<'a, T> for LockReadGuard<'a, T, S> {}
//
// pub struct DynLockReadGuard<'a, T: ?Sized>(Box<dyn LockReadGuardlike<'a, T> + 'a>);
//
// impl<T: ?Sized> Deref for DynLockReadGuard<'_, T> {
//     type Target = T;
//
//     #[inline]
//     fn deref(&self) -> &T {
//         self.0.as_ref()
//     }
// }
//
// impl<'a, T: ?Sized, S: Spec> From<LockReadGuard<'a, T, S>> for DynLockReadGuard<'a, T> {
//     fn from(guard: LockReadGuard<'a, T, S>) -> Self {
//         DynLockReadGuard(Box::new(guard))
//     }
// }
//
// trait LockWriteGuardlike<'a, T: ?Sized>: DerefMut<Target = T> {
//     fn downgrade(self) -> DynLockReadGuard<'a, T>;
//
//     fn downgrade_box(self: Box<Self>) -> DynLockReadGuard<'a, T>;
// }
//
// impl<'a, T: ?Sized, S: Spec> LockWriteGuardlike<'a, T> for LockWriteGuard<'a, T, S> {
//     fn downgrade(self) -> DynLockReadGuard<'a, T> {
//         self.downgrade().into()
//     }
//
//     fn downgrade_box(self: Box<Self>) -> DynLockReadGuard<'a, T> {
//         self.downgrade().into()
//     }
// }
//
// pub struct DynLockWriteGuard<'a, T: ?Sized>(Option<Box<dyn LockWriteGuardlike<'a, T> + 'a>>);
//
// impl<'a, T: ?Sized> DynLockWriteGuard<'a, T> {
//     fn downgrade(self) -> DynLockReadGuard<'a, T> {
//         let mut b = self.0;
//         let g = b.take().unwrap();
//         let d = g.downgrade_box();
//         d.into()
//     }
// }
//
// impl<T: ?Sized> Deref for DynLockWriteGuard<'_, T> {
//     type Target = T;
//
//     #[inline]
//     fn deref(&self) -> &T {
//         self.0.as_ref().unwrap().as_ref()
//     }
// }
//
// impl<T: ?Sized> DerefMut for DynLockWriteGuard<'_, T> {
//     #[inline]
//     fn deref_mut(&mut self) -> &mut T {
//         self.0.as_mut().unwrap().as_mut()
//     }
// }
//
// impl<'a, T: ?Sized, S: Spec> From<LockWriteGuard<'a, T, S>> for DynLockWriteGuard<'a, T> {
//     fn from(guard: LockWriteGuard<'a, T, S>) -> Self {
//         DynLockWriteGuard(Some(Box::new(guard)))
//     }
// }
//
// impl<T: ?Sized, S: Spec> Locklike<T> for XLock<T, S> {
//     fn read(&self) -> DynLockReadGuard<T> {
//         DynLockReadGuard(Box::new(self.read()))
//     }
//
//     fn try_read(&self, duration: Duration) -> Option<DynLockReadGuard<T>> {
//         self.try_read(duration).map(DynLockReadGuard::from)
//     }
//
//     fn write(&self) -> DynLockWriteGuard<T> {
//         DynLockWriteGuard(Some(Box::new(self.write())))
//     }
//
//     fn try_write(&self, duration: Duration) -> Option<DynLockWriteGuard<T>> {
//         self.try_write(duration).map(DynLockWriteGuard::from)
//     }
// }
//
// #[test]
// fn boxed_cycle() {
//     let lock = XLock::<_, ReadBiased>::new(42);
//     let boxed: LockBox<_> = Box::new(lock);
//     let guard = boxed.read();
//     assert_eq!(42, *guard);
//     drop(guard);
//
//     let mut guard = boxed.write();
//     *guard = 69;
//     assert_eq!(69, *guard);
//     drop(guard);
//
//     let guard = boxed.try_read(Duration::ZERO).unwrap();
//     assert_eq!(69, *guard);
//     drop(guard);
//
//     let mut guard = boxed.try_write(Duration::ZERO).unwrap();
//     *guard = 1983;
//     assert_eq!(1983, *guard);
//
//     let guard = guard.downgrade(); // drops old guard
//     assert_eq!(1983, *guard);
//     drop(guard);
//
//
// }