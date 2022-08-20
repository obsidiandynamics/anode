use crate::completable::{Completable, Outcome};
use crate::utils;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

pub type SubmissionOutcome<G> = Arc<Completable<Outcome<G>>>;

pub trait Executor {
    fn submit<F, G>(&self, f: F) -> SubmissionOutcome<G>
    where
        F: FnOnce() -> G + Send + 'static,
        G: Send + 'static;

    fn try_submit<F, G>(&self, f: F) -> Option<SubmissionOutcome<G>>
    where
        F: FnOnce() -> G + Send + 'static,
        G: Send + 'static;
}

type Task = Box<dyn FnOnce() + Send>;

pub struct ThreadPool {
    running: Arc<AtomicBool>,
    sender: Option<SyncSender<Task>>,
    threads: Option<Vec<JoinHandle<()>>>,
}

impl ThreadPool {
    pub fn new(threads: u16, bound: usize) -> Self {
        assert!(threads > 0);
        let running = Arc::new(AtomicBool::new(true));
        let (sender, receiver) = mpsc::sync_channel::<Task>(bound);
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
        let _ = self
            .threads
            .take()
            .unwrap()
            .into_iter()
            .map(JoinHandle::join)
            .map(Result::unwrap);
    }
}

fn prepare_task<F, G>(pool: &ThreadPool, f: F) -> (SubmissionOutcome<G>, Task)
where
    F: FnOnce() -> G + Send + 'static,
    G: Send + 'static,
{
    let comp = Arc::new(Completable::default());
    let task = {
        let comp = comp.clone();
        let running = pool.running.clone();
        Box::new(move || {
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
        })
    };
    (comp, task)
}

impl Executor for ThreadPool {
    fn submit<F, G>(&self, f: F) -> SubmissionOutcome<G>
    where
        F: FnOnce() -> G + Send + 'static,
        G: Send + 'static,
    {
        let (comp, task) = prepare_task(self, f);
        self.sender.as_ref().unwrap().send(task).unwrap();
        comp
    }

    fn try_submit<F, G>(&self, f: F) -> Option<SubmissionOutcome<G>>
    where
        F: FnOnce() -> G + Send + 'static,
        G: Send + 'static,
    {
        let (comp, task) = prepare_task(self, f);
        let send_res = self.sender.as_ref().unwrap().try_send(task);
        match send_res {
            Ok(_) => Some(comp),
            Err(TrySendError::Full(_)) => None,
            Err(_) => unreachable!()
        }
    }
}

#[cfg(test)]
mod tests;
