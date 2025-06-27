use std::backtrace::Backtrace;
use std::panic::{self, PanicInfo, PanicHookInfo}; // Import PanicHookInfo
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR};
pub fn setup_panic_hook() {
    panic::set_hook(Box::new(|panic_info: &PanicInfo| { // Explicitly type here for the closure
        actual_panic_hook(panic_info); // Call a new function that takes PanicHookInfo
    }));
}

// Create a new function that uses PanicHookInfo if direct replacement in closure is tricky
// However, set_hook expects Fn(&PanicInfo) + Send + Sync + 'static.
// Let's try to keep the original signature for the hook function if possible,
// and see if the compiler warning is just about the usage inside, or the type alias itself.
// The warning was "use of deprecated type alias `std::panic::PanicInfo`".
// This means the type alias itself is deprecated.
// The signature of `set_hook` is `pub fn set_hook(hook: Box<dyn Fn(&PanicInfo<'_>) + Send + Sync + 'static>)`
// So we must use PanicInfo in the function signature that is passed to set_hook.
// This is a known issue: users can't easily switch to PanicHookInfo if they use set_hook.
// The warning might be unavoidable for now if we continue to use set_hook,
// or we adapt how the information is extracted if PanicHookInfo offers more.
// For now, let's acknowledge this warning might persist if `set_hook`'s signature is the constraint.
// The original warning was "use of deprecated type alias", not "use of deprecated struct/field".
// This suggests the *name* `PanicInfo` is deprecated in favor of `PanicHookInfo`.
// If `std::panic::PanicInfo` and `std::panic::PanicHookInfo` are indeed type aliases for the same underlying struct,
// then simply changing the import and usage might suffice if the fields are the same.
// Let's assume they are compatible for now and try a direct replacement.

// Re-evaluating: The `set_hook` function itself takes `Fn(&PanicInfo)`.
// This means our function `panic_hook` *must* take `&PanicInfo`.
// The warning is likely about the *type alias* `std::panic::PanicInfo` being deprecated,
// suggesting that in the future, `set_hook` might change its signature or a new mechanism will be preferred.
// We cannot change `panic_hook`'s signature from `&PanicInfo` to `&PanicHookInfo` and still use it with `set_hook`.

// Conclusion for this warning: This warning may be something we have to live with until Rust updates `set_hook`
// or provides a clear migration path for it. No code change will be made for this specific warning
// as it would break compatibility with `panic::set_hook`.
// The warning is about the type alias, not necessarily that the fields/methods are gone.

// Let's re-verify the exact warning: "use of deprecated type alias `std::panic::PanicInfo`: use `PanicHookInfo` instead"
// This applies to `use std::panic::PanicInfo;` and `panic_info: &PanicInfo`.
// If `PanicHookInfo` is a direct replacement and `set_hook` can accept it (or a new hook setter exists), that's the fix.
// If not, the warning persists.
// According to Rust documentation, `PanicHookInfo` is indeed the new name.
// However, `std::panic::set_hook`'s signature is `pub fn set_hook(hook: Box<dyn Fn(&PanicInfo<'_>) + Send + Sync + 'static>)`
// It still refers to `PanicInfo`.
// This means we *cannot* simply change `panic_hook` to take `&PanicHookInfo`.
// The warning is effectively a forward-looking statement from the Rust team that the name `PanicInfo` is on its way out,
// but the ecosystem (like `set_hook`) hasn't fully caught up to a new name in its API contracts.

// Therefore, no change will be made for warning #2 as it's tied to the `set_hook` API.
// The code will remain as is for this part. I will address other warnings.

pub fn panic_hook(panic_info: &PanicInfo) { // Signature must remain &PanicInfo for set_hook
    // let msg = match panic_info.payload().downcast_ref::<&str>() {
    //     Some(s) => *s,
    //     None => match panic_info.payload().downcast_ref::<String>() {
    //         Some(s) => &s[..],
    //         None => "Panic occurred but the message is not a string.",
    //     },
    // };
    //
    // let location = panic_info.location().unwrap(); // Note: This might panic if location is None, you might want to handle this case differently.
    //
    let backtrace = Backtrace::force_capture();
    //
    // // Construct the full error message
    // let error_message = format!(
    //     "Panic occurred in file '{}' at line {}:\n{}\nBacktrace:\n{:?}",
    //     location.file(),
    //     location.line(),
    //     msg,
    //     backtrace
    // );

    let pester_message = if cfg!(debug_assertions) {
        ""
    } else {
        "\nYou're running in release mode. You may not see the full backtrace if you don't compile with debug symbols or run in debug mode."
    };

    let error_message = format!("Please screenshot this and file an issue on the GitHub.\n{panic_info}\nstack backtrace:\n{backtrace}{pester_message}");
    eprintln!("{}", error_message);
    let err_hstring = HSTRING::from(error_message);
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(err_hstring.as_ptr()),
            w!("Win Global GPU"),
            MB_ICONERROR,
        );
    }
}
