use std::mem;

use anyhow::{ensure, Result};
use windows::Win32::{
    Foundation::HWND,
    UI::Input::{
        GetRawInputDeviceInfoW, GetRawInputDeviceList, RegisterRawInputDevices, RAWINPUTDEVICE,
        RAWINPUTDEVICELIST, RAWINPUTDEVICE_FLAGS, RAW_INPUT_DEVICE_INFO_COMMAND,
        RID_DEVICE_INFO_TYPE,
    },
};

fn main() -> Result<()> {
    let keyboards = get_keyboards()?;

    for keyboard in keyboards {
        println!("Registering {}", keyboard.name);
        unsafe { keyboard.register()? };
    }

    Ok(())
}

struct Keyboard {
    name: String,
    device: RAWINPUTDEVICELIST,
}

impl Keyboard {
    unsafe fn register(&self) -> Result<()> {
        RegisterRawInputDevices(
            &mut [RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage: 0x06,
                dwFlags: RAWINPUTDEVICE_FLAGS(0), // RIDEV_INPUTSINK,
                hwndTarget: HWND(0),
            }],
            mem::size_of::<RAWINPUTDEVICE>() as u32,
        )?;
        Ok(())
    }
}

const KEYBOARD_DEVICE_TYPE: RID_DEVICE_INFO_TYPE = RID_DEVICE_INFO_TYPE(0x01);
const UICOMMAND_RIDI_DEVICENAME: RAW_INPUT_DEVICE_INFO_COMMAND =
    RAW_INPUT_DEVICE_INFO_COMMAND(0x20000007);

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
