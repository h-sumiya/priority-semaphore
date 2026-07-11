use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use priority_semaphore::PrioritySemaphore;
use std::hint::black_box;
use std::sync::Arc;

fn uncontended(c: &mut Criterion) {
    let mut group = c.benchmark_group("uncontended_acquire_release");

    let priority = Arc::new(PrioritySemaphore::new(1));
    group.bench_function("priority_semaphore", |b| {
        b.iter(|| drop(black_box(priority.try_acquire(0).unwrap())))
    });

    let tokio = Arc::new(tokio::sync::Semaphore::new(1));
    group.bench_function("tokio_semaphore", |b| {
        b.iter(|| drop(black_box(tokio.clone().try_acquire_owned().unwrap())))
    });
    group.finish();
}

fn contended_handoff(c: &mut Criterion) {
    const WAITERS: usize = 128;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();
    let mut group = c.benchmark_group("contended_handoff");
    group.throughput(Throughput::Elements(WAITERS as u64));

    group.bench_with_input(
        BenchmarkId::new("priority_semaphore", WAITERS),
        &WAITERS,
        |b, &waiters| {
            b.iter(|| {
                runtime.block_on(async {
                    let semaphore = Arc::new(PrioritySemaphore::new(1));
                    let gate = semaphore.acquire(0).await.unwrap();
                    let mut tasks = Vec::with_capacity(waiters);
                    for priority in 0..waiters {
                        let semaphore = semaphore.clone();
                        tasks.push(tokio::spawn(async move {
                            drop(semaphore.acquire(priority as i32).await.unwrap());
                        }));
                    }
                    while semaphore.queued() != waiters {
                        tokio::task::yield_now().await;
                    }
                    drop(gate);
                    for task in tasks {
                        task.await.unwrap();
                    }
                })
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("tokio_semaphore", WAITERS),
        &WAITERS,
        |b, &waiters| {
            b.iter(|| {
                runtime.block_on(async {
                    let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
                    let gate = semaphore.clone().acquire_owned().await.unwrap();
                    let (started_tx, mut started_rx) = tokio::sync::mpsc::unbounded_channel();
                    let mut tasks = Vec::with_capacity(waiters);
                    for _ in 0..waiters {
                        let semaphore = semaphore.clone();
                        let started_tx = started_tx.clone();
                        tasks.push(tokio::spawn(async move {
                            started_tx.send(()).unwrap();
                            drop(semaphore.acquire_owned().await.unwrap());
                        }));
                    }
                    drop(started_tx);
                    for _ in 0..waiters {
                        started_rx.recv().await.unwrap();
                    }
                    tokio::task::yield_now().await;
                    drop(gate);
                    for task in tasks {
                        task.await.unwrap();
                    }
                })
            });
        },
    );
    group.finish();
}

criterion_group!(benches, uncontended, contended_handoff);
criterion_main!(benches);
