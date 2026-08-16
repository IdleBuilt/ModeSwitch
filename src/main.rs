#![windows_subsystem = "windows"]

use std::mem::size_of;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const WM_TRAYICON: u32 = WM_APP + 1;
const ID_TOGGLE_STARTUP: u32 = 1001;
const ID_EXIT: u32 = 1002;
const TRAY_UID: u32 = 1;

const PERSONALIZE_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize";
const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const STARTUP_VALUE_NAME: &str = "ModeSwitch";

struct AppState {
    hwnd: HWND,
    dark_icon: HICON,
    light_icon: HICON,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn make_int_resource(id: u16) -> *const u16 {
    id as usize as *const u16
}

fn main() {
    unsafe {
        let mutex_name = wide("ModeSwitch_SingleInstance");
        CreateMutexW(null(), 1, mutex_name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            message_box(
                "ModeSwitch is already running.",
                "ModeSwitch",
                MB_OK | MB_ICONINFORMATION,
            );
            return;
        }

        let hinstance = GetModuleHandleW(null());

        let class_name = wide("ModeSwitchWndClass");
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: null_mut(),
            hCursor: null_mut(),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: null_mut(),
        };
        RegisterClassExW(&wc);

        let title = wide("ModeSwitch");
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            hinstance,
            null(),
        );

        let dark_icon = load_icon(hinstance, 2);
        let light_icon = load_icon(hinstance, 3);

        let state = Box::new(AppState {
            hwnd,
            dark_icon,
            light_icon,
        });
        let state_ptr = Box::into_raw(state);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);

        add_tray_icon(hwnd, current_icon(&*state_ptr));

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        remove_tray_icon(hwnd);
        drop(Box::from_raw(state_ptr));
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            match lparam as u32 {
                WM_LBUTTONUP => {
                    if let Some(state) = get_state(hwnd) {
                        toggle_theme(state);
                    }
                }
                WM_RBUTTONUP => {
                    show_context_menu(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as u32;
            match id {
                ID_TOGGLE_STARTUP => {
                    toggle_startup();
                }
                ID_EXIT => {
                    DestroyWindow(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_SETTINGCHANGE => {
            if let Some(state) = get_state(hwnd) {
                refresh_icon(state);
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn get_state(hwnd: HWND) -> Option<&'static mut AppState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
    if ptr.is_null() {
        None
    } else {
        Some(&mut *ptr)
    }
}

unsafe fn message_box(text: &str, caption: &str, flags: MESSAGEBOX_STYLE) {
    let t = wide(text);
    let c = wide(caption);
    MessageBoxW(null_mut(), t.as_ptr(), c.as_ptr(), flags);
}

unsafe fn load_icon(hinstance: HINSTANCE, id: u16) -> HICON {
    let cx = GetSystemMetrics(SM_CXSMICON);
    let cy = GetSystemMetrics(SM_CYSMICON);
    LoadImageW(
        hinstance,
        make_int_resource(id),
        IMAGE_ICON,
        cx,
        cy,
        LR_DEFAULTCOLOR,
    ) as HICON
}

unsafe fn current_icon(state: &AppState) -> HICON {
    if is_light_mode() {
        state.light_icon
    } else {
        state.dark_icon
    }
}

unsafe fn apply_icon(state: &mut AppState, is_light: bool) {
    let icon = if is_light { state.light_icon } else { state.dark_icon };
    update_tray_icon(state.hwnd, icon);
}

unsafe fn toggle_theme(state: &mut AppState) {
    let new_is_light = !is_light_mode();
    set_light_mode(new_is_light);
    broadcast_theme_changed();
    apply_icon(state, new_is_light);
}

unsafe fn refresh_icon(state: &mut AppState) {
    let is_light = is_light_mode();
    apply_icon(state, is_light);
}

fn build_nid(hwnd: HWND, icon: HICON) -> NOTIFYICONDATAW {
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_UID;
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = WM_TRAYICON;
    nid.hIcon = icon;
    let tip = wide("ModeSwitch");
    let n = tip.len().min(nid.szTip.len());
    nid.szTip[..n].copy_from_slice(&tip[..n]);
    nid
}

unsafe fn add_tray_icon(hwnd: HWND, icon: HICON) {
    let nid = build_nid(hwnd, icon);
    Shell_NotifyIconW(NIM_ADD, &nid);
}

unsafe fn update_tray_icon(hwnd: HWND, icon: HICON) {
    let nid = build_nid(hwnd, icon);
    Shell_NotifyIconW(NIM_MODIFY, &nid);
}

unsafe fn remove_tray_icon(hwnd: HWND) {
    let nid = build_nid(hwnd, null_mut());
    Shell_NotifyIconW(NIM_DELETE, &nid);
}

unsafe fn show_context_menu(hwnd: HWND) {
    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);

    let hmenu = CreatePopupMenu();
    let startup_label = wide("Start with Windows");
    let exit_label = wide("Exit");

    let flags_startup: MENU_ITEM_FLAGS = if is_startup_enabled() {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    AppendMenuW(hmenu, flags_startup, ID_TOGGLE_STARTUP as usize, startup_label.as_ptr());
    AppendMenuW(hmenu, MF_SEPARATOR, 0, null());
    AppendMenuW(hmenu, MF_STRING, ID_EXIT as usize, exit_label.as_ptr());

    SetForegroundWindow(hwnd);
    TrackPopupMenu(hmenu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, null());
    PostMessageW(hwnd, WM_NULL, 0, 0);

    DestroyMenu(hmenu);
}

unsafe fn is_light_mode() -> bool {
    let mut hkey: HKEY = null_mut();
    let subkey = wide(PERSONALIZE_KEY);
    let res = RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey);
    if res != ERROR_SUCCESS {
        return true;
    }

    let value_name = wide("AppsUseLightTheme");
    let mut data: u32 = 0;
    let mut data_len: u32 = size_of::<u32>() as u32;
    let mut value_type: u32 = 0;
    let qres = RegQueryValueExW(
        hkey,
        value_name.as_ptr(),
        null_mut(),
        &mut value_type,
        &mut data as *mut u32 as *mut u8,
        &mut data_len,
    );
    RegCloseKey(hkey);

    if qres == ERROR_SUCCESS {
        data != 0
    } else {
        true
    }
}

unsafe fn set_light_mode(is_light: bool) {
    let mut hkey: HKEY = null_mut();
    let subkey = wide(PERSONALIZE_KEY);
    let mut disposition: u32 = 0;
    let res = RegCreateKeyExW(
        HKEY_CURRENT_USER,
        subkey.as_ptr(),
        0,
        null(),
        REG_OPTION_NON_VOLATILE,
        KEY_WRITE,
        null(),
        &mut hkey,
        &mut disposition,
    );
    if res != ERROR_SUCCESS {
        return;
    }

    let value: u32 = if is_light { 1 } else { 0 };
    let value_bytes = value.to_le_bytes();

    for name in ["AppsUseLightTheme", "SystemUsesLightTheme"] {
        let value_name = wide(name);
        RegSetValueExW(
            hkey,
            value_name.as_ptr(),
            0,
            REG_DWORD,
            value_bytes.as_ptr(),
            value_bytes.len() as u32,
        );
    }

    RegCloseKey(hkey);
}

unsafe fn broadcast_theme_changed() {
    for setting in ["ImmersiveColorSet", "WindowsThemeElement"] {
        let s = wide(setting);
        let mut result: usize = 0;
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            s.as_ptr() as LPARAM,
            SMTO_ABORTIFHUNG,
            100,
            &mut result,
        );
    }
}

unsafe fn is_startup_enabled() -> bool {
    let mut hkey: HKEY = null_mut();
    let subkey = wide(RUN_KEY);
    if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != ERROR_SUCCESS {
        return false;
    }

    let value_name = wide(STARTUP_VALUE_NAME);
    let mut value_type: u32 = 0;
    let res = RegQueryValueExW(
        hkey,
        value_name.as_ptr(),
        null_mut(),
        &mut value_type,
        null_mut(),
        null_mut(),
    );
    RegCloseKey(hkey);

    res == ERROR_SUCCESS
}

unsafe fn toggle_startup() {
    if is_startup_enabled() {
        let mut hkey: HKEY = null_mut();
        let subkey = wide(RUN_KEY);
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_WRITE, &mut hkey) == ERROR_SUCCESS {
            let value_name = wide(STARTUP_VALUE_NAME);
            RegDeleteValueW(hkey, value_name.as_ptr());
            RegCloseKey(hkey);
        }
    } else {
        let mut hkey: HKEY = null_mut();
        let subkey = wide(RUN_KEY);
        let mut disposition: u32 = 0;
        let res = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            null(),
            &mut hkey,
            &mut disposition,
        );
        if res == ERROR_SUCCESS {
            let mut buf = [0u16; 260];
            GetModuleFileNameW(null_mut(), buf.as_mut_ptr(), buf.len() as u32);
            let len = buf.iter().position(|&c| c == 0).unwrap_or(0);

            // Quote the path: the install dir lives under "Program Files", which has a space.
            let quote = '"' as u16;
            let mut exe_path: Vec<u16> = Vec::with_capacity(len + 3);
            exe_path.push(quote);
            exe_path.extend_from_slice(&buf[..len]);
            exe_path.push(quote);
            exe_path.push(0);

            let value_name = wide(STARTUP_VALUE_NAME);
            RegSetValueExW(
                hkey,
                value_name.as_ptr(),
                0,
                REG_SZ,
                exe_path.as_ptr() as *const u8,
                (exe_path.len() * 2) as u32,
            );
            RegCloseKey(hkey);
        }
    }
}
