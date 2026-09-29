//! Bluetooth devices Windows knows about, with the battery level Windows
//! itself shows in Settings -> Bluetooth & devices. Windows reads it from the
//! device (the GATT Battery Service for Bluetooth LE, the hands-free profile
//! for headsets) and stores it as a device property; reading it sends nothing
//! to the device.

use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_ALLCLASSES, DIGCF_PRESENT, HDEVINFO, SP_DEVINFO_DATA, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
    SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW, SetupDiGetDevicePropertyW,
};
use windows::Win32::Devices::Properties::DEVPROPTYPE;
use windows::Win32::Foundation::DEVPROPKEY;
use windows::core::GUID;

/// DEVPKEY_Bluetooth_Battery: the battery level in percent, a single byte.
const BATTERY: DEVPROPKEY = DEVPROPKEY { fmtid: GUID::from_u128(0x104ea319_6ee2_4701_bd47_8ddbf425bbe5), pid: 2 };
/// DEVPKEY_Device_FriendlyName.
const FRIENDLY_NAME: DEVPROPKEY =
    DEVPROPKEY { fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0), pid: 14 };
/// DEVPKEY_NAME, the name shown when there is no friendly name.
const NAME: DEVPROPKEY = DEVPROPKEY { fmtid: GUID::from_u128(0xb725f130_47ef_101a_a5f1_02608c9ebac0), pid: 10 };

/// Device instance ids of Bluetooth devices start with one of these.
const BLUETOOTH_ENUMERATORS: [&str; 2] = [r"BTHENUM\", r"BTHLE\"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BluetoothDevice {
    pub name: String,
    /// The device instance id, e.g. `BTHLE\DEV_C5AA8F2D3E11\...`.
    pub instance_id: String,
    /// The level Windows shows, when the device reports one.
    pub battery: Option<u8>,
}

/// Every present Bluetooth device node, with its battery when Windows has one.
pub fn devices() -> Vec<BluetoothDevice> {
    let Ok(set) = (unsafe { SetupDiGetClassDevsW(None, None, None, DIGCF_PRESENT | DIGCF_ALLCLASSES) }) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut data = SP_DEVINFO_DATA { cbSize: size_of::<SP_DEVINFO_DATA>() as u32, ..Default::default() };
    for index in 0.. {
        if unsafe { SetupDiEnumDeviceInfo(set, index, &mut data) }.is_err() {
            break;
        }
        let Some(instance_id) = instance_id(set, &data) else { continue };
        if !BLUETOOTH_ENUMERATORS.iter().any(|e| instance_id.to_ascii_uppercase().starts_with(e)) {
            continue;
        }
        let battery = property(set, &data, &BATTERY).and_then(|b| b.first().copied()).filter(|&b| b <= 100);
        let name = string_property(set, &data, &FRIENDLY_NAME)
            .or_else(|| string_property(set, &data, &NAME))
            .unwrap_or_default();
        out.push(BluetoothDevice { name, instance_id, battery });
    }
    let _ = unsafe { SetupDiDestroyDeviceInfoList(set) };
    out
}

fn instance_id(set: HDEVINFO, data: &SP_DEVINFO_DATA) -> Option<String> {
    let mut buf = [0u16; 512];
    let mut len = 0u32;
    unsafe { SetupDiGetDeviceInstanceIdW(set, data, Some(&mut buf), Some(&mut len)) }.ok()?;
    Some(utf16_until_nul(&buf))
}

fn property(set: HDEVINFO, data: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<Vec<u8>> {
    let mut kind = DEVPROPTYPE::default();
    let mut buf = vec![0u8; 1024];
    let mut len = 0u32;
    unsafe { SetupDiGetDevicePropertyW(set, data, key, &mut kind, Some(&mut buf), Some(&mut len), 0) }.ok()?;
    buf.truncate(len as usize);
    Some(buf)
}

fn string_property(set: HDEVINFO, data: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<String> {
    let bytes = property(set, data, key)?;
    let wide: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|&c| u16::from_le_bytes(c)).collect();
    Some(utf16_until_nul(&wide)).filter(|s| !s.is_empty())
}

fn utf16_until_nul(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}
