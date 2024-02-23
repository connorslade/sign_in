#![cfg(windows)]

use std::iter;

use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{HANDLE, HINSTANCE},
        System::{
            Diagnostics::Debug::OutputDebugStringA,
            ProcessStatus::GetProcessImageFileNameA,
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        },
    },
};

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
unsafe extern "system" fn DllMain(
    dll_module: HINSTANCE,
    call_reason: u32,
    reserved: *mut (),
) -> bool {
    let mut buf: [u8; 1024] = [0u8; 1024];
    GetProcessImageFileNameA(HANDLE(-1), &mut buf);
    let name = String::from_utf8_lossy(&buf);

    match call_reason {
        DLL_PROCESS_ATTACH => {
            let msg = format!("DLL Injected into: {name}");
            let msg = msg.bytes().chain(iter::once(0)).collect::<Vec<_>>();
            OutputDebugStringA(PCSTR(msg.as_ptr()));
        }
        DLL_PROCESS_DETACH => {}
        _ => {}
    }

    true
}
