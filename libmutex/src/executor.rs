use crate::completable::{Completable, Outcome};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, SyncSender, TrySendError};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use crate::remedy::Remedy;

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

enum SenderImpl {
    Unbounded(Sender<Task>),
    Bounded(SyncSender<Task>)
}

impl SenderImpl {
    #[inline]
    fn send(&self, task: Task) {
        match self {
            SenderImpl::Unbounded(sender) => sender.send(task).unwrap(),
            SenderImpl::Bounded(sender) => sender.send(task).unwrap()
        }
    }

    #[inline]
    fn try_send(&self, task: Task) -> bool {
        match self {
            SenderImpl::Unbounded(sender) => {
                sender.send(task).unwrap();
                true
            }
            SenderImpl::Bounded(sender) => {
                match sender.try_send(task) {
                    Ok(_) => true,
                    Err(TrySendError::Full(_)) => false,
                    Err(_) => unreachable!()
                }
            }
        }
    }
}

pub struct ThreadPool {
    running: Arc<AtomicBool>,
    sender: Option<SenderImpl>,
    threads: Option<Vec<JoinHandle<()>>>,
}

#[derive(Debug, Clone)]
pub enum Queue {
    Unbounded,
    Bounded(usize)
}

impl ThreadPool {
    #[inline]
    pub fn new(threads: usize, queue: Queue) -> Self {
        assert!(threads > 0);
        let running = Arc::new(AtomicBool::new(true));
        let (sender, receiver) = {
            match queue {
                Queue::Unbounded => {
                    let (tx, rx) = mpsc::channel::<Task>();
                    (SenderImpl::Unbounded(tx), rx)
                }
                Queue::Bounded(bound) => {
                    let (tx, rx) = mpsc::sync_channel::<Task>(bound);
                    (SenderImpl::Bounded(tx), rx)
                }
            }
        };
        let receiver = Arc::new(Mutex::new(receiver));
        let threads = (0..threads)
            .into_iter()
            .map(|_| {
                let receiver = receiver.clone();
                thread::spawn(move || loop {
                    let receiver = receiver.lock().remedy();
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
    #[inline]
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

#[inline]
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
            let outcome = if running {
                Outcome::Success(f())
            } else {
                Outcome::Abort
            };
            comp.complete(outcome);
            // ---
        })
    };
    (comp, task)
}

impl Executor for ThreadPool {
    #[inline]
    fn submit<F, G>(&self, f: F) -> SubmissionOutcome<G>
    where
        F: FnOnce() -> G + Send + 'static,
        G: Send + 'static,
    {
        let (comp, task) = prepare_task(self, f);
        self.sender.as_ref().unwrap().send(task);
        comp
    }

    #[inline]
    fn try_submit<F, G>(&self, f: F) -> Option<SubmissionOutcome<G>>
    where
        F: FnOnce() -> G + Send + 'static,
        G: Send + 'static,
    {
        let (comp, task) = prepare_task(self, f);
        let enqueued = self.sender.as_ref().unwrap().try_send(task);
        if enqueued { Some(comp) } else { None }
    }
}

#[cfg(test)]
mod tests;
