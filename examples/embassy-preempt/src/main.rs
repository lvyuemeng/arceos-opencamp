#![feature(impl_trait_in_assoc_type)]
#![feature(type_alias_impl_trait)]
#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;

use axasync::executor::PrioFuture;
use axasync::executor::spawner;
use axasync::executor::yield_now;
use axasync::time::Timer;
use core::hint::black_box;
use std::boxed::Box;
use std::thread::{self, sleep};
use std::time::Duration;

fn busy_work(iters: u64) -> u64 {
    let mut total = 0;
    for _ in 0..iters {
        total = black_box(total + 1);
        total = black_box(total * 3 / 2);
    }
    black_box(total)
}

async fn async_busy_work(iters: u64) -> u64 {
    let mut total = 0;
    for _ in 0..iters {
        total = black_box(total + 1);
        total = black_box(total * 3 / 2);
        yield_now().await;
    }
    black_box(total)
}

macro_rules! work_loop {
    (
        task_type: $task_type:literal,
        id: $id:expr,
        sleep: $sleep_fn:expr,
        // millis restriction
        millis: $millis:expr,
        $(is_await: $await_tt:tt,)?
        busy_work: $busy_work:expr,
        busy_iters: $busy_iters:expr,
    ) => {
        use std::time::{Instant,Duration};
        use log;
        use core::hint::black_box;

        let mut cnt = 0;
        let mut last_report = Instant::now();
        let millis = Duration::from_millis($millis);

        loop {
            let iter_start = Instant::now();
            cnt += 1;

            if $busy_iters > 0 {
                let busy_start = Instant::now();
                let _res = black_box($busy_work($busy_iters));
                let busy_dur = Instant::now() - busy_start;
                log::info!(
                    "{} {}: duration {}/ns",
                    $task_type,
                    $id,
                    busy_dur.as_nanos()
                )
            }

            $sleep_fn($millis)$(.$await_tt)?;

            let iter_end = Instant::now();
            let iter_dur = iter_end - iter_start;
            let full_dur = iter_end - last_report;

            log::info!(
                "{} {}: Iteration {}, expected {}/ns, actual {}/ns, full {}/ns",
                $task_type,
                $id,
                cnt,
                millis.as_nanos(),
                iter_dur.as_nanos(),
                full_dur.as_nanos(),
            );

            last_report = iter_end;
        }
    };
}

macro_rules! priority_task {
    (
        $task_type:expr
        $prio_fn_name:ident,
        $async_fn_name:ident,
        $prios:expr,
        $pool_size:expr,
    ) => {
        #[axasync::executor::task(pool_size = $pool_size)]
        async fn $prio_fn_name(id: u64, millis: u64, busy_iters: u64) {
            let prio_fut = PrioFuture::new($async_fn_name(id, millis, busy_iters), $prios as u8);
            prio_fut.await;
        }

        async fn stringify!($prio_fn_name + "__task")(id: u64, millis: u64, busy_iters: u64) {
            task_loop! {
                task_type: $task_type,
                id: id,
                sleep: |millis| async move {Timer::after_millis(millis).await},
                millis: millis,
                is_await: await,
                busy_work: async_busy_work,
                busy_iters: busy_iters,
            }
        }
    };
}

define_priority_tasks! {
    "ASYNC_TASK_REPORT_HIGH"
    prio_tick_high,
    NUM_HIGH_TASKS,
    NUM_HIGH_PRIOS,
}

define_priority_tasks! {
    "ASYNC_TASK_REPORT_LOW"
    prio_tick_low,
    NUM_LOW_TASKS,
    NUM_LOW_PRIOS,
}

fn prio_tick_raw(
    id: u64,
    millis: u64,
    busy_iters: u64,
    prio: u8,
) -> embassy_executor::SpawnToken<impl Sized> {
    type Fut = PrioFuture<impl ::core::future::Future + 'static>;
    const POOL_SIZE: usize = 5;
    static POOL: embassy_executor::raw::TaskPool<Fut, POOL_SIZE> =
        embassy_executor::raw::TaskPool::new();

    let prio_fut = PrioFuture::new(async_tick(id, millis, busy_iters), prio);
    POOL.spawn(|| prio_fut)
}

fn thread_tick(id: u64, millis: u64, busy_iters: u64) {
    work_loop! {
        task_type: "NATIVE_THREAD",
        id:id,
        sleep: |millis| sleep(Duration::from_millis(millis)),
        millis:millis,
        busy_work: busy_work,
        busy_iters:busy_iters,
    }
}

const NUM_THREADS: u64 = 5;
const NUM_HIGH_TASKS: u64 = 2;
const NUM_LOW_TASKS: u64 = 6;

const NUM_HIGH_PRIOS: u64 = 1;
const NUM_LOW_PRIOS: u64 = 3;

// const NUM_ITERS_THREADS: u64 = 0;
// const NUM_ITERS_THREADS: u64 = 1_000;
const NUM_ITERS_THREADS: u64 = 1_000_000;
// const NUM_ITERS_TASKS: u64 = 0;
// const NUM_ITERS_TASKS: u64 = 1000;
const NUM_ITERS_TASKS: u64 = 1_000_000;

#[cfg_attr(feature = "axstd", unsafe(no_mangle))]
fn main() {
    log::info!("Embassy Test");
    for i in 1..NUM_THREADS {
        thread::spawn(move || {
            thread_tick(i, i * 1000, NUM_ITERS_THREADS);
        });
    }

    for i in 1..NUM_HIGH_TASKS {
        spawner()
            .spawn(prio_tick_high(i, i * 1000, NUM_ITERS_TASKS))
            .unwrap();
    }
    for i in 1..NUM_LOW_TASKS {
        spawner()
            .spawn(prio_tick_low(i, i * 1000, NUM_ITERS_TASKS))
            .unwrap();
    }
    // Avoid shut down immediately
    sleep(Duration::from_millis(1000 * 15 as u64));
}
