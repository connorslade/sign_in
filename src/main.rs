use std::{
    ffi::{c_void, OsStr},
    mem,
    os::windows::ffi::OsStrExt,
};

use anyhow::{ensure, Context, Result};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::{
                GetRawInputData, GetRawInputDeviceInfoW, GetRawInputDeviceList,
                RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTDEVICELIST,
                RAWINPUTHEADER, RAW_INPUT_DEVICE_INFO_COMMAND, RIDEV_INPUTSINK,
                RID_DEVICE_INFO_TYPE, RID_INPUT, RIM_TYPEKEYBOARD,
            },
            WindowsAndMessaging::{
                CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageA,
                GetMessageA, RegisterClassExW, CW_USEDEFAULT, KBDLLHOOKSTRUCT, WINDOW_EX_STYLE,
                WINDOW_STYLE, WM_INPUT, WM_KEYDOWN, WNDCLASSEXW,
            },
        },
    },
};

fn main() -> Result<()> {
    let keyboards = get_keyboards()?;
    for keyboard in keyboards {
        println!("[{:?}]: {}", keyboard.device.hDevice, keyboard.name);
    }

    // unsafe {
    //     let hook = SetWindowsHookExA(WH_KEYBOARD_LL, Some(keyboard_hook), None, 0)?;

    //     let mut message = mem::zeroed();
    //     while GetMessageA(&mut message, HWND::default(), 0, 0).as_bool() {
    //         DispatchMessageA(&message);
    //     }

    //     UnhookWindowsHookEx(hook)?;
    // }

    let hwnd = init_window()?;

    let dev = RAWINPUTDEVICE {
        usUsagePage: 0x01,
        usUsage: 0x06,
        dwFlags: RIDEV_INPUTSINK,
        hwndTarget: hwnd,
    };

    unsafe { RegisterRawInputDevices(&[dev], mem::size_of::<RAWINPUTDEVICE>() as u32)? };

    unsafe {
        let mut message = mem::zeroed();
        while GetMessageA(&mut message, hwnd, 0, 0).as_bool() {
            DispatchMessageA(&message);
        }
    }

    unsafe { DestroyWindow(hwnd)? };

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
        if w_param.0 == WM_KEYDOWN as usize {
            println!("Key: {}", key as u8 as char);
        }

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

fn init_window() -> Result<HWND> {
    let class_name = OsStr::new("SignIn")
        .encode_wide()
        .chain(Some(0))
        .collect::<Vec<_>>();

    let hinstance = unsafe { GetModuleHandleW(None)? };

    let mut wnd = WNDCLASSEXW::default();
    wnd.cbSize = mem::size_of::<WNDCLASSEXW>() as u32;
    wnd.lpfnWndProc = Some(window_proc);
    wnd.hInstance = HINSTANCE(hinstance.0);
    wnd.lpszClassName = PCWSTR(class_name.as_ptr());

    let reg = unsafe { RegisterClassExW(&wnd) };
    ensure!(reg != 0, "Error registering window class");

    let window = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR(class_name.as_ptr()),
            None,
            WINDOW_STYLE(0),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            hinstance,
            None,
        )
    };

    unsafe { GetLastError().context(window.0)? };

    ensure!(window.0 != 0, "Error creating window");

    Ok(window)
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
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
            if (*input).header.dwType == RIM_TYPEKEYBOARD.0
                && (*input).data.keyboard.Message == WM_KEYDOWN as u32
            {
                let device = (*input).header.hDevice;
                let key = (*input).data.keyboard.VKey as u8 as char;
                println!("[{:?}]: {}", device, key);
            }

            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}
