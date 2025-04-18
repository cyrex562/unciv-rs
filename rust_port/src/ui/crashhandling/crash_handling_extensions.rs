use std::panic::catch_unwind;
use crate::game::UncivGame;

/// Extension trait for functions that can be wrapped with crash handling
pub trait CrashHandlingExt<R> {
    /// Returns a wrapped version of a function that automatically handles an uncaught exception or error.
    /// In case of an uncaught exception or error, the return will be None.
    ///
    /// The UncivStage, UncivGame.render and Concurrency already use this to wrap nearly everything
    /// that can happen during the lifespan of the Unciv application.
    /// Therefore, it usually shouldn't be necessary to manually use this.
    fn wrap_crash_handling(&self) -> Box<dyn Fn() -> Option<R>>;
}

/// Extension trait for functions that return unit (void) that can be wrapped with crash handling
pub trait CrashHandlingUnitExt {
    /// Returns a wrapped version of a function that automatically handles an uncaught exception or error.
    ///
    /// The UncivStage, UncivGame.render and Concurrency already use this to wrap nearly everything
    /// that can happen during the lifespan of the Unciv application.
    /// Therefore, it usually shouldn't be necessary to manually use this.
    fn wrap_crash_handling_unit(&self) -> Box<dyn Fn()>;
}

impl<F, R> CrashHandlingExt<R> for F
where
    F: Fn() -> R + 'static,
    R: 'static,
{
    fn wrap_crash_handling(&self) -> Box<dyn Fn() -> Option<R>> {
        let func = self.clone();
        Box::new(move || {
            match catch_unwind(|| func()) {
                Ok(result) => Some(result),
                Err(e) => {
                    UncivGame::current().handle_uncaught_panic(e);
                    None
                }
            }
        })
    }
}

impl<F> CrashHandlingUnitExt for F
where
    F: Fn() + 'static,
{
    fn wrap_crash_handling_unit(&self) -> Box<dyn Fn()> {
        let wrapped_returning = self.wrap_crash_handling();
        // Don't instantiate a new lambda every time the return get called.
        Box::new(move || {
            let _ = wrapped_returning();
        })
    }
}