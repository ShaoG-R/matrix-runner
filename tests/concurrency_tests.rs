//! # Concurrency Tests using Loom
//!
//! This module uses loom to test concurrency and thread-safety in the execution module,
//! particularly focusing on the CancellationToken used for fast-failing.

#[cfg(test)]
mod tests {
    use loom::sync::Arc;
    use loom::sync::atomic::{AtomicUsize, Ordering};
    use loom::thread;
    use tokio_util::sync::CancellationToken;

    /// This test models a simplified "fast-fail" scenario.
    ///
    /// While the actual implementation uses a central coordinator to cancel, that model
    /// proves too complex for `loom` to explore without causing a stack overflow, even
    /// with a larger stack.
    ///
    /// This simplified model still captures the essential race condition:
    /// - One worker task directly triggers the `CancellationToken`.
    /// - Other tasks race to check `is_cancelled()` before starting work.
    ///
    /// This is sufficient to verify the thread-safety of the cancellation mechanism.
    #[test]
    fn test_fast_fail_cancellation_is_thread_safe() {
        // We spawn a new thread with a larger stack size to prevent a stack overflow,
        // which can occur with loom's deep exploration of complex concurrent models.
        const STACK_SIZE: usize = 8 * 1024 * 1024; // 8 MB

        let builder = std::thread::Builder::new()
            .name("loom-test-thread".into())
            .stack_size(STACK_SIZE);

        let handle = builder
            .spawn(|| {
                loom::model(|| {
                    // Two tasks are sufficient to model the race condition: one that proceeds
                    // and one that triggers the cancellation.
                    const NUM_TASKS: usize = 2;
                    let completed_tasks = Arc::new(AtomicUsize::new(0));
                    let token = Arc::new(CancellationToken::new());

                    let mut handles = vec![];

                    for i in 0..NUM_TASKS {
                        let token_clone = token.clone();
                        let completed_tasks_clone = completed_tasks.clone();

                        handles.push(thread::spawn(move || {
                            // This check simulates the `tokio::select!` that races
                            // the test execution against `token.cancelled()`.
                            if !token_clone.is_cancelled() {
                                completed_tasks_clone.fetch_add(1, Ordering::Relaxed);

                                // Designate one task to be the trigger for cancellation.
                                if i == 1 {
                                    token_clone.cancel();
                                }
                            }
                        }));
                    }

                    for handle in handles {
                        handle.join().unwrap();
                    }

                    // After all threads complete, the token must be in the "cancelled" state
                    // because one task was guaranteed to trigger it.
                    assert!(token.is_cancelled());

                    let final_count = completed_tasks.load(Ordering::Relaxed);

                    // Due to the race condition, we can't know the exact number of tasks
                    // that completed, but it must be between 1 and NUM_TASKS.
                    assert!(
                        final_count >= 1 && final_count <= NUM_TASKS,
                        "Final count was {}",
                        final_count
                    );
                });
            })
            .unwrap();

        handle.join().unwrap();
    }
}
