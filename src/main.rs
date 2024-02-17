use std::mem;

use anyhow::{ensure, Result};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Input::{
            GetRawInputDeviceInfoW, GetRawInputDeviceList, RAWINPUTDEVICELIST,
            RAW_INPUT_DEVICE_INFO_COMMAND, RID_DEVICE_INFO_TYPE,
        },
        WindowsAndMessaging::{
            CallNextHookEx, DispatchMessageA, GetMessageA, SetWindowsHookExA, TranslateMessage,
            UnhookWindowsHookEx, KBDLLHOOKSTRUCT, WH_KEYBOARD_LL,
        },
    },
};

fn main() -> Result<()> {
    let _keyboards = get_keyboards()?;

    unsafe {
        let hook = SetWindowsHookExA(WH_KEYBOARD_LL, Some(keyboard_hook), None, 0)?;

        let mut message = mem::zeroed();
        while GetMessageA(&mut message, HWND::default(), 0, 0).as_bool() {
            TranslateMessage(&message); // ?
            DispatchMessageA(&message);
        }

        UnhookWindowsHookEx(hook)?;
    }

    Ok(())
}

struct Keyboard {
    name: String,
    device: RAWINPUTDEVICELIST,
}

const KEYBOARD_DEVICE_TYPE: RID_DEVICE_INFO_TYPE = RID_DEVICE_INFO_TYPE(0x01);
const UICOMMAND_RIDI_DEVICENAME: RAW_INPUT_DEVICE_INFO_COMMAND =
    RAW_INPUT_DEVICE_INFO_COMMAND(0x20000007);

unsafe extern "system" fn keyboard_hook(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let ptr = l_param.0 as *const KBDLLHOOKSTRUCT;

    if n_code >= 0 {
        let key = (*ptr).vkCode;
        println!("Key: {}", key as u8 as char);

        if key == 'A' as u32 {
            return LRESULT(1);
        }
    }

    CallNextHookEx(None, n_code, w_param, l_param)
}

fn get_keyboards() -> Result<Vec<Keyboard>> {
    let mut system_count = 0;
    let devices = unsafe {
        GetRawInputDeviceList(
            None,
            &mut system_count,
            mem::size_of::<RAWINPUTDEVICELIST>() as u32,
        )
    };
    ensure!(devices as i32 != -1, "Error getting raw input device count");

    let mut device_list = vec![RAWINPUTDEVICELIST::default(); system_count as usize];

    // Note: If a new device is added between the last call to GetRawInputDeviceList and the current, a ERROR_INSUFFICIENT_BUFFER will be returned.
    let devices = unsafe {
        GetRawInputDeviceList(
            Some(device_list.as_mut_ptr()),
            &mut system_count,
            mem::size_of::<RAWINPUTDEVICELIST>() as u32,
        )
    };
    ensure!(devices as i32 != -1, "Error getting raw input device list");

    let mut out = Vec::new();
    for device in device_list {
        if device.dwType != KEYBOARD_DEVICE_TYPE {
            continue;
        }

        let mut size = 0;
        unsafe {
            GetRawInputDeviceInfoW(device.hDevice, UICOMMAND_RIDI_DEVICENAME, None, &mut size);
        }

        let mut name = vec![0u16; size as usize];
        let result = unsafe {
            GetRawInputDeviceInfoW(
                device.hDevice,
                UICOMMAND_RIDI_DEVICENAME,
                Some(name.as_mut_ptr() as _),
                &mut size,
            )
        };
        ensure!(result as i32 != -1, "Error getting raw input device name");
        let name = String::from_utf16_lossy(&name[..size as usize - 1]);

        out.push(Keyboard { name, device });
    }

    Ok(out)
}
