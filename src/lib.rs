#![cfg(windows)]

use std::{collections::HashMap, ffi::c_void, iter, mem, panic, process};

use anyhow::{ensure, Result};
use once_cell::sync::Lazy;
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{
            GetLastError, BOOL, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, MAX_PATH, WPARAM,
        },
        System::{
            Diagnostics::Debug::OutputDebugStringA,
            ProcessStatus::GetProcessImageFileNameA,
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        },
        UI::{
            Input::{
                GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE,
                RAWINPUTDEVICE_FLAGS, RAWINPUTHEADER, RID_INPUT, RIM_TYPEKEYBOARD,
            },
            WindowsAndMessaging::{
                EnumChildWindows, GetForegroundWindow, GetWindowLongPtrW, SetWindowLongPtrW,
                GWL_WNDPROC, WM_CHAR, WM_INPUT, WM_KEYDOWN, WM_KEYUP,
            },
        },
    },
};

macro_rules! log {
    ($($arg:tt)*) => {{
        let msg = to_pcstr(&format!($($arg)*));
        OutputDebugStringA(PCSTR(msg.as_ptr()));
    }};
}

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
            panic::set_hook(Box::new(|info| {
                log!("== Panic ==");
                log!("{info}");
                process::abort();
            }));
            process_attach()
        }
        DLL_PROCESS_DETACH => {
            log!("DLL Unloaded from: {name}");
            process_detach()
        }
        _ => return true,
    };

    handle_error(result);
    true
}

unsafe fn handle_error(result: Result<()>) {
    if let Err(err) = result {
        log!("Error: {err}");
        if let Err(last_err) = GetLastError() {
            log!("Last Error: {last_err}");
        }
        log!("{}", err.backtrace());
    }
}

type WNDPROC = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;
static mut OLD_HOOK: Lazy<HashMap<isize, WNDPROC>> = Lazy::new(|| HashMap::new());

unsafe fn process_attach() -> Result<()> {
    let window = GetForegroundWindow();
    // inject_window(window)?;

    unsafe extern "system" fn child_window_callback(hwnd: HWND, _: LPARAM) -> BOOL {
        handle_error(inject_window(hwnd));
        BOOL::from(true)
    }

    EnumChildWindows(window, Some(child_window_callback), LPARAM::default());

    // Hook into the keyboard
    // Note: if the application already has a raw hook, this will fail
    let dev = RAWINPUTDEVICE {
        usUsagePage: 0x01,
        usUsage: 0x06,
        dwFlags: RAWINPUTDEVICE_FLAGS::default(),
        hwndTarget: window,
    };

    RegisterRawInputDevices(&[dev], mem::size_of::<RAWINPUTDEVICE>() as u32)?;

    Ok(())
}

unsafe fn process_detach() -> Result<()> {
    Ok(())
}

unsafe fn inject_window(window: HWND) -> Result<()> {
    // let module_handle = GetModuleHandleA(s!("user32"))?;
    // let default_window = GetProcAddress(module_handle, s!("DefWindowProcW"))
    //     .context("Error getting default window procedure address")?;

    log!("Hooking into: {window:?}");
    let window_proc = GetWindowLongPtrW(window, GWL_WNDPROC);

    OLD_HOOK.insert(window.0, mem::transmute(window_proc));
    ensure!(window_proc != 0, "Error getting window procedure");
    log!("Old Window Proc: {window_proc:x}");
    log!("{:?}", OLD_HOOK);

    let result = SetWindowLongPtrW(window, GWL_WNDPROC, window_proc_hook as _);
    ensure!(result != 0, "Error setting window procedure");

    Ok(())
}

unsafe extern "system" fn window_proc_hook(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    // WM_CHAR and WM_KEYDOWN
    match msg {
        WM_INPUT => {
            let mut size = 0;
            let result = GetRawInputData(
                HRAWINPUT(l_param.0),
                RID_INPUT,
                None,
                &mut size,
                mem::size_of::<RAWINPUTHEADER>() as u32,
            );
            assert_eq!(result as i32, 0);

            let mut data = vec![0u8; size as usize];
            let result = GetRawInputData(
                HRAWINPUT(l_param.0),
                RID_INPUT,
                Some(data.as_mut_ptr() as *mut c_void),
                &mut size,
                mem::size_of::<RAWINPUTHEADER>() as u32,
            );
            assert_eq!(result as i32, size as i32);

            let input = &*(data.as_ptr() as *const RAWINPUT);
            if (*input).header.dwType == RIM_TYPEKEYBOARD.0 {
                let device = (*input).header.hDevice;
                let key = (*input).data.keyboard.VKey as u8 as char;
                log!("[{:x}] [{:?}]: {}", hwnd.0, device, key);
            }

            return LRESULT(0);
        }
        WM_KEYDOWN => {
            log!("[{:x}] WM_KEYDOWN", hwnd.0);
            return LRESULT(0);
        }
        WM_KEYUP => {
            log!("[{:x}] WM_KEYUP", hwnd.0);
            return LRESULT(0);
        }
        WM_CHAR => {
            log!("[{:x}] WM_CHAR", hwnd.0);
            return LRESULT(0);
        }
        _ => {}
    }

    let hook = OLD_HOOK.get(&hwnd.0).unwrap();
    (hook)(hwnd, msg, w_param, l_param)
}

fn to_pcstr(s: &str) -> Vec<u8> {
    s.bytes().chain(iter::once(0)).collect()
}
