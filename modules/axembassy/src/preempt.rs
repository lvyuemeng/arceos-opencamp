use alloc::collections::BTreeMap;
use core::{
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};
use kspin::SpinNoIrq;

// u32 bitmaps
pub type Prio = u8;
pub const MAX_PRIO: Prio = 31;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct TaskId(u64);

impl TaskId {
    pub fn new() -> Self {
        static ID_COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
    /// Convert the task ID to a `u64`.
    pub const fn as_u64(&self) -> u64 {
        self.0
    }
}

pub struct PrioFuture<F> {
    inner: F,
    prio: Prio,
    id: TaskId,
    registered: bool,
}

impl<F: Future> PrioFuture<F> {
    pub fn new(fut: F, prio: Prio) -> Self {
        Self {
            inner: fut,
            prio,
            id: TaskId::new(),
            registered: false,
        }
    }

    pub fn set_prio(&mut self, prio: Prio) {
        let old_prio = self.prio;
        self.prio = prio;
        SCHEDULER.lock().update(old_prio, prio, self.id);
    }
}

impl<F: Future> Future for PrioFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        // Register on first poll
        if !this.registered {
            SCHEDULER.lock().insert(this.prio, this.id);
            this.registered = true;
        }

        let highest_prio = SCHEDULER.lock().cur_prio().unwrap_or(MAX_PRIO);

        if this.prio > highest_prio {
            return Poll::Pending;
        }

        // SAFETY: We're not moving the future, just projecting the pin
        let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
        match inner.poll(cx) {
            Poll::Ready(output) => {
                let id = this.id;
                SCHEDULER.lock().unpark_task_prio(id);
                Poll::Ready(output)
            }
            Poll::Pending => {
                let id = this.id;
                SCHEDULER.lock().park_task_prio(id);
                Poll::Pending
            }
        }
    }
}

impl<F> Drop for PrioFuture<F> {
    fn drop(&mut self) {
        if self.registered {
            SCHEDULER.lock().remove(self.id);
        }
    }
}

static SCHEDULER: SpinNoIrq<PrioScheduler> = SpinNoIrq::new(PrioScheduler::new());

struct PrioScheduler {
    prios: BTreeMap<Prio, u64>,
    tasks: BTreeMap<TaskId, Prio>,
}

impl PrioScheduler {
    pub const fn new() -> Self {
        Self {
            prios: BTreeMap::new(),
            tasks: BTreeMap::new(),
        }
    }

    pub fn park_task_prio(&mut self, id: TaskId) {
        if let Some(prio) = self.tasks.get(&id) {
            self.prios.entry(*prio).and_modify(|cnt| {
                *cnt = cnt.saturating_sub(1);
            });
        }
    }

    pub fn unpark_task_prio(&mut self, id: TaskId) {
        if let Some(prio) = self.tasks.get(&id) {
            self.prios.entry(*prio).and_modify(|cnt| {
                *cnt = cnt.saturating_add(1);
            });
        }
    }

    pub fn cur_prio(&self) -> Option<Prio> {
        self.prios
            .iter()
            .filter(|(_, cnt)| **cnt > 0)
            .next()
            .map(|(prio, _)| *prio)
    }

    pub fn insert(&mut self, prio: Prio, id: TaskId) {
        self.prios
            .entry(prio)
            .and_modify(|cnt| *cnt += 1)
            .or_insert(1);
        self.tasks.insert(id, prio);
    }

    pub fn update(&mut self, old_prio: Prio, prio: Prio, id: TaskId) {
        self.prios.entry(old_prio).and_modify(|cnt| *cnt -= 1);
        self.prios
            .entry(prio)
            .and_modify(|cnt| *cnt += 1)
            .or_insert(1);
        self.tasks.insert(id, prio);
    }

    pub fn remove(&mut self, id: TaskId) {
        if let Some(prio) = self.tasks.remove(&id) {
            self.prios.entry(prio).and_modify(|cnt| {
                *cnt = cnt.saturating_sub(1);
            });
            if *self.prios.get(&prio).unwrap_or(&0) == 0 {
                self.prios.remove(&prio);
            }
        }
    }
}
