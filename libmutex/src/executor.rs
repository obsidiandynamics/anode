use crate::completable::{Completable, Outcome};
use crate::utils;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::thread::{JoinHandle};

pub trait Executor {
    fn submit<F, G>(&self, f: F) -> Arc<Completable<Outcome<G>>>
    where
        F: FnOnce() -> G + Send + 'static,
        G: Send + 'static;
}

type Task = Box<dyn FnOnce() + Send>;

pub struct ThreadPool {
    running: Arc<AtomicBool>,
    sender: Option<Sender<Task>>,
    threads: Option<Vec<JoinHandle<()>>>,
}

impl ThreadPool {
    pub fn new(threads: u16) -> Self {
        assert!(threads > 0);
        let running = Arc::new(AtomicBool::default());
        let (sender, receiver) = mpsc::channel::<Task>();
        let receiver = Arc::new(Mutex::new(receiver));
        let threads = (0..threads)
            .into_iter()
            .map(|_| {
                let receiver = receiver.clone();
                thread::spawn(move || loop {
                    let receiver = utils::remedy(receiver.lock());
                    let task = receiver.recv();
                    drop(receiver);
                    match task {
                        Ok(task) => task(),
                        Err(_) => return,
                    }
                })
            })
            .collect::<Vec<_>>();

        Self {
            running,
            sender: Some(sender),
            threads: Some(threads),
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.sender = None;
        let _ = self.threads.take().unwrap().into_iter().map(JoinHandle::join).map(Result::unwrap);
    }
}

impl Executor for ThreadPool {
    fn submit<F, G>(&self, f: F) -> Arc<Completable<Outcome<G>>>
    where
        F: FnOnce() -> G + Send + 'static,
        G: Send + 'static,
    {
        let comp = Arc::new(Completable::default());
        {
            let comp = comp.clone();
            let running = self.running.clone();
            self.sender.as_ref().unwrap()
                .send(Box::new(move || {
                    // --- code that is run on the worker thread
                    let running = running.load(Ordering::Relaxed);
                    comp.complete_exclusive(|| {
                        if running {
                            Outcome::Success(f())
                        } else {
                            Outcome::Abort
                        }
                    });
                    // ---
                }))
                .unwrap();
        }
        comp
    }
}

#[cfg(test)]
mod tests;