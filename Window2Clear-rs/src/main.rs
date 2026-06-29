#![windows_subsystem = "windows"]
#![allow(non_snake_case, unused)]

use std::ptr;
use std::mem;

#[cfg(windows)]
extern crate winapi;

use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::um::commctrl::*;
use winapi::um::libloaderapi::*;
use winapi::um::shellapi::*;
use winapi::um::synchapi::*;
use winapi::um::sysinfoapi::*;
use winapi::um::winbase::*;
use winapi::um::winuser::*;
use winapi::um::processthreadsapi::*;

// ── 常量 ──
const WM_TRAYICON: UINT = WM_USER + 1;
const ID_TRAY_EXIT: UINT = 1001;
const ID_TRAY_SETTINGS: UINT = 1002;
const ID_HOTKEY_TRANSPARENCY_UP: i32 = 1;
const ID_HOTKEY_TRANSPARENCY_DOWN: i32 = 2;
const ID_HOTKEY_CENTER_WINDOW: i32 = 3;
const ID_HOTKEY_SHAKE_WINDOW: i32 = 4;
const ID_HOTKEY_RESTORE_OPACITY: i32 = 5;

const IDC_TRANSPARENCY_UP_BUTTON: UINT = 2001;
const IDC_TRANSPARENCY_DOWN_BUTTON: UINT = 2002;
const IDC_CENTER_BUTTON: UINT = 2003;
const IDC_SHAKE_BUTTON: UINT = 2004;
const IDC_TRANSPARENCY_SLIDER: UINT = 2005;
const IDC_SAVE_BUTTON: UINT = 2006;
const IDC_TRANSPARENCY_LABEL: UINT = 2007;
const IDC_TRANSPARENCY_UP_DISPLAY: UINT = 2008;
const IDC_TRANSPARENCY_DOWN_DISPLAY: UINT = 2009;
const IDC_CENTER_DISPLAY: UINT = 2010;
const IDC_SHAKE_DISPLAY: UINT = 2011;
const IDC_TRANSPARENCY_ENABLE: UINT = 2012;
const IDC_CENTER_ENABLE: UINT = 2013;
const IDC_SHAKE_ENABLE: UINT = 2014;

const APP_VERSION: &str = "v0.3.0";

// ── 透明窗口跟踪 ──
struct TransparentWindow {
    hwnd: HWND,
    alpha: i32,
}

// ── 全局状态 ──
static mut G_MAIN_WND: HWND = ptr::null_mut();
static mut G_SETTINGS_WND: HWND = ptr::null_mut();
static mut G_NID: NOTIFYICONDATAW = unsafe { mem::zeroed() };
static mut G_TRANSPARENCY_STEP: i32 = 10;

// 热键
static mut G_TU_MODS: UINT = MOD_ALT as UINT;
static mut G_TU_KEY: UINT = VK_LEFT as UINT;
static mut G_TD_MODS: UINT = MOD_ALT as UINT;
static mut G_TD_KEY: UINT = VK_RIGHT as UINT;
static mut G_C_MODS: UINT = MOD_CONTROL as UINT;
static mut G_C_KEY: UINT = VK_NUMPAD5 as UINT;
static mut G_S_MODS: UINT = MOD_ALT as UINT;
static mut G_S_KEY: UINT = VK_DOWN as UINT;
static mut G_R_MODS: UINT = MOD_ALT as UINT;
static mut G_R_KEY: UINT = VK_UP as UINT;
static mut G_EN_TU: BOOL = TRUE;
static mut G_EN_TD: BOOL = TRUE;
static mut G_EN_C: BOOL = FALSE;
static mut G_EN_S: BOOL = FALSE;
static mut G_EN_R: BOOL = TRUE;

// 透明窗口列表
static mut G_TW: Vec<TransparentWindow> = Vec::new();

// 热键监听
static mut G_LISTENING: BOOL = FALSE;
static mut G_LISTEN_TYPE: i32 = 0;
static mut G_LISTEN_BTN: HWND = ptr::null_mut();
static mut G_LISTEN_DISP: HWND = ptr::null_mut();
static mut G_LISTEN_START: DWORD = 0;

// 设置窗口控件
static mut H_SLIDER: HWND = ptr::null_mut();
static mut H_LABEL: HWND = ptr::null_mut();
static mut H_BTN_UP: HWND = ptr::null_mut();
static mut H_BTN_DOWN: HWND = ptr::null_mut();
static mut H_BTN_CENTER: HWND = ptr::null_mut();
static mut H_BTN_SHAKE: HWND = ptr::null_mut();
static mut H_DISP_UP: HWND = ptr::null_mut();
static mut H_DISP_DOWN: HWND = ptr::null_mut();
static mut H_DISP_CENTER: HWND = ptr::null_mut();
static mut H_DISP_SHAKE: HWND = ptr::null_mut();
static mut H_CHK_TRANS: HWND = ptr::null_mut();
static mut H_CHK_CENTER: HWND = ptr::null_mut();
static mut H_CHK_SHAKE: HWND = ptr::null_mut();

// ── 工具函数 ──
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn get_modifier_name(mods: UINT) -> String {
    let mut parts = Vec::new();
    if mods & MOD_CONTROL as UINT != 0 { parts.push("CTRL"); }
    if mods & MOD_ALT as UINT != 0 { parts.push("ALT"); }
    if mods & MOD_SHIFT as UINT != 0 { parts.push("SHIFT"); }
    if mods & MOD_WIN as UINT != 0 { parts.push("WIN"); }
    parts.join("+")
}

fn get_key_name(vk: UINT) -> String {
    match vk as i32 {
        VK_F1..=VK_F12 => format!("F{}", vk as i32 - VK_F1 + 1),
        VK_LEFT => "LEFT".into(), VK_RIGHT => "RIGHT".into(),
        VK_UP => "UP".into(), VK_DOWN => "DOWN".into(),
        VK_NUMPAD0..=VK_NUMPAD9 => format!("NUM{}", vk as i32 - VK_NUMPAD0),
        VK_INSERT => "INSERT".into(), VK_DELETE => "DELETE".into(),
        VK_HOME => "HOME".into(), VK_END => "END".into(),
        VK_PRIOR => "PAGEUP".into(), VK_NEXT => "PAGEDOWN".into(),
        VK_SPACE => "SPACE".into(), VK_TAB => "TAB".into(),
        VK_RETURN => "ENTER".into(), VK_ESCAPE => "ESC".into(),
        0x41..=0x5A => char::from_u32(vk as u32).unwrap_or('?').to_string(),
        0x30..=0x39 => char::from_u32(vk as u32).unwrap_or('?').to_string(),
        _ => format!("KEY{}", vk),
    }
}

fn hotkey_text(mods: UINT, key: UINT) -> String {
    format!("{}+{}", get_modifier_name(mods), get_key_name(key))
}

// ── 透明度操作 ──
unsafe fn get_window_transparency(hwnd: HWND) -> i32 {
    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
    if ex_style & WS_EX_LAYERED as i32 != 0 {
        let mut alpha: u8 = 0;
        if GetLayeredWindowAttributes(hwnd, ptr::null_mut(), &mut alpha, ptr::null_mut()) != 0 {
            return alpha as i32;
        }
    }
    255
}

unsafe fn set_window_transparency(hwnd: HWND, alpha: i32) {
    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
    if alpha < 255 {
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED as i32);
        SetLayeredWindowAttributes(hwnd, 0, alpha as u8, LWA_ALPHA);
        track_window(hwnd, alpha);
    } else {
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style & !(WS_EX_LAYERED as i32));
        untrack_window(hwnd);
    }
}

unsafe fn find_tw(hwnd: HWND) -> Option<usize> {
    G_TW.iter().position(|tw| tw.hwnd == hwnd)
}

unsafe fn track_window(hwnd: HWND, alpha: i32) {
    if let Some(idx) = find_tw(hwnd) {
        G_TW[idx].alpha = alpha;
    } else if G_TW.len() < 64 {
        G_TW.push(TransparentWindow { hwnd, alpha });
    }
}

unsafe fn untrack_window(hwnd: HWND) {
    if let Some(idx) = find_tw(hwnd) {
        G_TW.swap_remove(idx);
    }
}

unsafe fn get_topmost_window() -> HWND {
    let hwnd = GetForegroundWindow();
    if hwnd.is_null() {
        return GetWindow(GetDesktopWindow(), GW_CHILD);
    }
    hwnd
}

unsafe fn adjust_transparency(increase: bool) {
    let hwnd = get_topmost_window();
    if hwnd.is_null() || hwnd == G_MAIN_WND || hwnd == G_SETTINGS_WND { return; }
    let cur = get_window_transparency(hwnd);
    let delta = 255 * G_TRANSPARENCY_STEP / 100;
    let new_alpha = if increase { (cur - delta).max(25) } else { (cur + delta).min(255) };
    set_window_transparency(hwnd, new_alpha);
}

unsafe fn restore_opacity() {
    let hwnd = get_topmost_window();
    if hwnd.is_null() || hwnd == G_MAIN_WND || hwnd == G_SETTINGS_WND { return; }
    if get_window_transparency(hwnd) < 255 {
        set_window_transparency(hwnd, 255);
    }
}

unsafe fn center_window() {
    let hwnd = get_topmost_window();
    if hwnd.is_null() || hwnd == G_MAIN_WND || hwnd == G_SETTINGS_WND { return; }
    let sw = GetSystemMetrics(SM_CXSCREEN);
    let sh = GetSystemMetrics(SM_CYSCREEN);
    let mut rect: RECT = mem::zeroed();
    GetWindowRect(hwnd, &mut rect);
    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    SetWindowPos(hwnd, ptr::null_mut(), (sw - w) / 2, (sh - h) / 2, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
}

unsafe fn shake_window() {
    let hwnd = get_topmost_window();
    if hwnd.is_null() || hwnd == G_MAIN_WND || hwnd == G_SETTINGS_WND { return; }
    let mut rect: RECT = mem::zeroed();
    GetWindowRect(hwnd, &mut rect);
    let (ox, oy) = (rect.left, rect.top);
    for i in 0..6 {
        let dx = if i % 2 == 0 { 5 } else { -5 };
        let dy = if i % 4 < 2 { 5 } else { -5 };
        SetWindowPos(hwnd, ptr::null_mut(), ox + dx, oy + dy, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
        Sleep(50);
    }
    SetWindowPos(hwnd, ptr::null_mut(), ox, oy, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
}

// ── 热键 ──
unsafe fn register_hotkeys(hwnd: HWND) {
    unregister_hotkeys(hwnd);
    if G_EN_TU != 0 { RegisterHotKey(hwnd, ID_HOTKEY_TRANSPARENCY_UP, G_TU_MODS, G_TU_KEY); }
    if G_EN_TD != 0 { RegisterHotKey(hwnd, ID_HOTKEY_TRANSPARENCY_DOWN, G_TD_MODS, G_TD_KEY); }
    if G_EN_C != 0 { RegisterHotKey(hwnd, ID_HOTKEY_CENTER_WINDOW, G_C_MODS, G_C_KEY); }
    if G_EN_S != 0 { RegisterHotKey(hwnd, ID_HOTKEY_SHAKE_WINDOW, G_S_MODS, G_S_KEY); }
    if G_EN_R != 0 { RegisterHotKey(hwnd, ID_HOTKEY_RESTORE_OPACITY, G_R_MODS, G_R_KEY); }
}

unsafe fn unregister_hotkeys(hwnd: HWND) {
    for id in [ID_HOTKEY_TRANSPARENCY_UP, ID_HOTKEY_TRANSPARENCY_DOWN,
               ID_HOTKEY_CENTER_WINDOW, ID_HOTKEY_SHAKE_WINDOW, ID_HOTKEY_RESTORE_OPACITY] {
        UnregisterHotKey(hwnd, id);
    }
}

// ── 托盘 ──
unsafe fn create_tray_icon(hwnd: HWND) {
    G_NID.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
    G_NID.hWnd = hwnd;
    G_NID.uID = 1;
    G_NID.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    G_NID.uCallbackMessage = WM_TRAYICON;
    G_NID.hIcon = LoadIconW(ptr::null_mut(), IDI_APPLICATION);
    let tip = wide(&format!("Window2Clear {} - 窗口透明度控制", APP_VERSION));
    for (i, &c) in tip.iter().take(127).enumerate() {
        G_NID.szTip[i] = c as u16;
    }
    Shell_NotifyIconW(NIM_ADD, &mut G_NID);
}

unsafe fn remove_tray_icon() {
    Shell_NotifyIconW(NIM_DELETE, &mut G_NID);
}

unsafe fn show_context_menu(hwnd: HWND) {
    let hmenu = CreatePopupMenu();
    AppendMenuW(hmenu, MF_STRING, ID_TRAY_SETTINGS as usize, wide("设置\0").as_ptr());
    AppendMenuW(hmenu, MF_SEPARATOR, 0, ptr::null());
    AppendMenuW(hmenu, MF_STRING, ID_TRAY_EXIT as usize, wide("退出\0").as_ptr());
    let mut pt: POINT = mem::zeroed();
    GetCursorPos(&mut pt);
    SetForegroundWindow(hwnd);
    TrackPopupMenu(hmenu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, ptr::null());
    DestroyMenu(hmenu);
}

// ── 配置 ──
unsafe fn config_path() -> Vec<u16> { wide(".\\config.ini") }

unsafe fn load_config() {
    let p = config_path().as_ptr();
    macro_rules! gi {
        ($sec:expr, $key:expr, $def:expr) => {{
            GetPrivateProfileIntW(wide($sec).as_ptr(), wide($key).as_ptr(), $def, p) as i32
        }};
    }
    G_TRANSPARENCY_STEP = gi!("Settings", "TransparencyStep", 10).clamp(1, 50);
    G_TU_MODS = gi!("Hotkeys", "TransparencyUpModifiers", MOD_ALT as i32) as UINT;
    G_TU_KEY = gi!("Hotkeys", "TransparencyUpKey", VK_LEFT) as UINT;
    G_TD_MODS = gi!("Hotkeys", "TransparencyDownModifiers", MOD_ALT as i32) as UINT;
    G_TD_KEY = gi!("Hotkeys", "TransparencyDownKey", VK_RIGHT) as UINT;
    G_C_MODS = gi!("Hotkeys", "CenterModifiers", MOD_CONTROL as i32) as UINT;
    G_C_KEY = gi!("Hotkeys", "CenterKey", VK_NUMPAD5) as UINT;
    G_S_MODS = gi!("Hotkeys", "ShakeModifiers", MOD_ALT as i32) as UINT;
    G_S_KEY = gi!("Hotkeys", "ShakeKey", VK_DOWN) as UINT;
    G_R_MODS = gi!("Hotkeys", "RestoreModifiers", MOD_ALT as i32) as UINT;
    G_R_KEY = gi!("Hotkeys", "RestoreKey", VK_UP) as UINT;
    G_EN_TU = gi!("Switches", "EnableTransparencyUp", 1);
    G_EN_TD = gi!("Switches", "EnableTransparencyDown", 1);
    G_EN_C = gi!("Switches", "EnableCenter", 0);
    G_EN_S = gi!("Switches", "EnableShake", 0);
    G_EN_R = gi!("Switches", "EnableRestore", 1);
}

unsafe fn save_config() {
    let p = config_path().as_ptr();
    macro_rules! si {
        ($sec:expr, $key:expr, $val:expr) => {{
            let v = wide(&format!("{}", $val));
            WritePrivateProfileStringW(wide($sec).as_ptr(), wide($key).as_ptr(), v.as_ptr(), p);
        }};
    }
    si!("Settings", "TransparencyStep", G_TRANSPARENCY_STEP);
    si!("Hotkeys", "TransparencyUpModifiers", G_TU_MODS);
    si!("Hotkeys", "TransparencyUpKey", G_TU_KEY);
    si!("Hotkeys", "TransparencyDownModifiers", G_TD_MODS);
    si!("Hotkeys", "TransparencyDownKey", G_TD_KEY);
    si!("Hotkeys", "CenterModifiers", G_C_MODS);
    si!("Hotkeys", "CenterKey", G_C_KEY);
    si!("Hotkeys", "ShakeModifiers", G_S_MODS);
    si!("Hotkeys", "ShakeKey", G_S_KEY);
    si!("Hotkeys", "RestoreModifiers", G_R_MODS);
    si!("Hotkeys", "RestoreKey", G_R_KEY);
    si!("Switches", "EnableTransparencyUp", if G_EN_TU != 0 { 1 } else { 0 });
    si!("Switches", "EnableTransparencyDown", if G_EN_TD != 0 { 1 } else { 0 });
    si!("Switches", "EnableCenter", if G_EN_C != 0 { 1 } else { 0 });
    si!("Switches", "EnableShake", if G_EN_S != 0 { 1 } else { 0 });
    si!("Switches", "EnableRestore", if G_EN_R != 0 { 1 } else { 0 });
}

// ── 设置窗口 ──
unsafe fn child(parent: HWND, class: &str, text: &str, style: DWORD, x: i32, y: i32, w: i32, h: i32, id: UINT) -> HWND {
    CreateWindowExW(0, wide(class).as_ptr(), wide(text).as_ptr(), style,
        x, y, w, h, parent, id as HMENU, GetModuleHandleW(ptr::null_mut()), ptr::null_mut())
}

unsafe fn show_settings_window() {
    if !G_SETTINGS_WND.is_null() {
        SetForegroundWindow(G_SETTINGS_WND);
        return;
    }
    let mut wc: WNDCLASSEXW = mem::zeroed();
    wc.cbSize = mem::size_of::<WNDCLASSEXW>() as u32;
    wc.lpfnWndProc = Some(settings_proc);
    wc.hInstance = GetModuleHandleW(ptr::null_mut());
    wc.lpszClassName = wide("SettingsWindowClass\0").as_ptr();
    wc.hCursor = LoadCursorW(ptr::null_mut(), IDC_ARROW);
    wc.hbrBackground = (COLOR_WINDOW + 1) as HBRUSH;
    RegisterClassExW(&wc);

    let (ww, wh) = (380i32, 420i32);
    let sw = GetSystemMetrics(SM_CXSCREEN);
    let sh = GetSystemMetrics(SM_CYSCREEN);
    let title = wide(&format!("Window2Clear {} 设置\0", APP_VERSION));
    G_SETTINGS_WND = CreateWindowExW(0, wide("SettingsWindowClass\0").as_ptr(), title.as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
        (sw - ww) / 2, (sh - wh) / 2, ww, wh,
        ptr::null_mut(), ptr::null_mut(), GetModuleHandleW(ptr::null_mut()), ptr::null_mut());
    if !G_SETTINGS_WND.is_null() {
        ShowWindow(G_SETTINGS_WND, SW_SHOW);
        UpdateWindow(G_SETTINGS_WND);
    }
}

unsafe extern "system" fn settings_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            unregister_hotkeys(G_MAIN_WND);
            let mut y = 20;

            child(hwnd, "STATIC", "透明度控制:", WS_VISIBLE | WS_CHILD, 20, y, 120, 20, 0);
            H_CHK_TRANS = child(hwnd, "BUTTON", "启用", WS_VISIBLE | WS_CHILD | BS_AUTOCHECKBOX, 150, y, 60, 20, IDC_TRANSPARENCY_ENABLE);
            SendMessageW(H_CHK_TRANS, BM_SETCHECK, if G_EN_TU != 0 || G_EN_TD != 0 { BST_CHECKED } else { BST_UNCHECKED } as WPARAM, 0);
            y += 30;

            child(hwnd, "STATIC", "增加透明度:", WS_VISIBLE | WS_CHILD, 30, y, 80, 20, 0);
            H_DISP_UP = child(hwnd, "EDIT", &hotkey_text(G_TU_MODS, G_TU_KEY),
                WS_VISIBLE | WS_CHILD | WS_BORDER | ES_AUTOHSCROLL | ES_READONLY, 120, y, 140, 20, IDC_TRANSPARENCY_UP_DISPLAY);
            H_BTN_UP = child(hwnd, "BUTTON", "设置", WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON, 270, y, 50, 20, IDC_TRANSPARENCY_UP_BUTTON);
            y += 30;

            child(hwnd, "STATIC", "减少透明度:", WS_VISIBLE | WS_CHILD, 30, y, 80, 20, 0);
            H_DISP_DOWN = child(hwnd, "EDIT", &hotkey_text(G_TD_MODS, G_TD_KEY),
                WS_VISIBLE | WS_CHILD | WS_BORDER | ES_AUTOHSCROLL | ES_READONLY, 120, y, 140, 20, IDC_TRANSPARENCY_DOWN_DISPLAY);
            H_BTN_DOWN = child(hwnd, "BUTTON", "设置", WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON, 270, y, 50, 20, IDC_TRANSPARENCY_DOWN_BUTTON);
            y += 40;

            child(hwnd, "STATIC", "窗口居中:", WS_VISIBLE | WS_CHILD, 20, y, 80, 20, 0);
            H_CHK_CENTER = child(hwnd, "BUTTON", "启用", WS_VISIBLE | WS_CHILD | BS_AUTOCHECKBOX, 150, y, 60, 20, IDC_CENTER_ENABLE);
            SendMessageW(H_CHK_CENTER, BM_SETCHECK, if G_EN_C != 0 { BST_CHECKED } else { BST_UNCHECKED } as WPARAM, 0);
            y += 30;
            H_DISP_CENTER = child(hwnd, "EDIT", &hotkey_text(G_C_MODS, G_C_KEY),
                WS_VISIBLE | WS_CHILD | WS_BORDER | ES_AUTOHSCROLL | ES_READONLY, 30, y, 140, 20, IDC_CENTER_DISPLAY);
            H_BTN_CENTER = child(hwnd, "BUTTON", "设置", WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON, 180, y, 50, 20, IDC_CENTER_BUTTON);
            y += 40;

            child(hwnd, "STATIC", "窗口抖动:", WS_VISIBLE | WS_CHILD, 20, y, 80, 20, 0);
            H_CHK_SHAKE = child(hwnd, "BUTTON", "启用", WS_VISIBLE | WS_CHILD | BS_AUTOCHECKBOX, 150, y, 60, 20, IDC_SHAKE_ENABLE);
            SendMessageW(H_CHK_SHAKE, BM_SETCHECK, if G_EN_S != 0 { BST_CHECKED } else { BST_UNCHECKED } as WPARAM, 0);
            y += 30;
            H_DISP_SHAKE = child(hwnd, "EDIT", &hotkey_text(G_S_MODS, G_S_KEY),
                WS_VISIBLE | WS_CHILD | WS_BORDER | ES_AUTOHSCROLL | ES_READONLY, 30, y, 140, 20, IDC_SHAKE_DISPLAY);
            H_BTN_SHAKE = child(hwnd, "BUTTON", "设置", WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON, 180, y, 50, 20, IDC_SHAKE_BUTTON);
            y += 40;

            H_LABEL = child(hwnd, "STATIC", &format!("透明度步长: {}%", G_TRANSPARENCY_STEP),
                WS_VISIBLE | WS_CHILD, 20, y, 200, 20, IDC_TRANSPARENCY_LABEL);
            y += 25;
            H_SLIDER = child(hwnd, "msctls_trackbar32", "",
                WS_VISIBLE | WS_CHILD | TBS_HORZ | TBS_AUTOTICKS, 20, y, 250, 30, IDC_TRANSPARENCY_SLIDER);
            SendMessageW(H_SLIDER, TBM_SETRANGE, TRUE as WPARAM, MAKELONG(1, 50) as LPARAM);
            SendMessageW(H_SLIDER, TBM_SETPOS, TRUE as WPARAM, G_TRANSPARENCY_STEP as LPARAM);
            y += 50;

            child(hwnd, "BUTTON", "保存设置", WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON, 20, y, 100, 30, IDC_SAVE_BUTTON);
            child(hwnd, "BUTTON", "关闭", WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON, 140, y, 100, 30, IDCANCEL as UINT);
        }

        WM_HSCROLL => {
            if lparam as HWND == H_SLIDER {
                let pos = SendMessageW(H_SLIDER, TBM_GETPOS, 0, 0) as i32;
                SetWindowTextW(H_LABEL, wide(&format!("透明度步长: {}%\0", pos)).as_ptr());
            }
        }

        WM_COMMAND => {
            let id = LOWORD(wparam as DWORD);
            match id as UINT {
                IDC_TRANSPARENCY_UP_BUTTON | IDC_TRANSPARENCY_DOWN_BUTTON |
                IDC_CENTER_BUTTON | IDC_SHAKE_BUTTON => {
                    let type_id = match id as UINT {
                        IDC_TRANSPARENCY_UP_BUTTON => 1,
                        IDC_TRANSPARENCY_DOWN_BUTTON => 2,
                        IDC_CENTER_BUTTON => 3,
                        IDC_SHAKE_BUTTON => 4,
                        _ => 0,
                    };
                    if G_LISTENING != 0 && G_LISTEN_TYPE == type_id {
                        G_LISTENING = FALSE;
                        G_LISTEN_TYPE = 0;
                        let text = match type_id {
                            1 => hotkey_text(G_TU_MODS, G_TU_KEY),
                            2 => hotkey_text(G_TD_MODS, G_TD_KEY),
                            3 => hotkey_text(G_C_MODS, G_C_KEY),
                            4 => hotkey_text(G_S_MODS, G_S_KEY),
                            _ => String::new(),
                        };
                        let disp = match type_id { 1 => H_DISP_UP, 2 => H_DISP_DOWN, 3 => H_DISP_CENTER, 4 => H_DISP_SHAKE, _ => ptr::null_mut() };
                        let btn = match type_id { 1 => H_BTN_UP, 2 => H_BTN_DOWN, 3 => H_BTN_CENTER, 4 => H_BTN_SHAKE, _ => ptr::null_mut() };
                        SetWindowTextW(btn, wide("设置\0").as_ptr());
                        SetWindowTextW(disp, wide(&format!("{}\0", text)).as_ptr());
                    } else {
                        G_LISTENING = TRUE;
                        G_LISTEN_TYPE = type_id;
                        let btn = match type_id { 1 => H_BTN_UP, 2 => H_BTN_DOWN, 3 => H_BTN_CENTER, 4 => H_BTN_SHAKE, _ => ptr::null_mut() };
                        let disp = match type_id { 1 => H_DISP_UP, 2 => H_DISP_DOWN, 3 => H_DISP_CENTER, 4 => H_DISP_SHAKE, _ => ptr::null_mut() };
                        G_LISTEN_BTN = btn;
                        G_LISTEN_DISP = disp;
                        G_LISTEN_START = GetTickCount();
                        SetWindowTextW(btn, wide("取消\0").as_ptr());
                        SetWindowTextW(disp, wide("请按下组合键...\0").as_ptr());
                    }
                }
                IDC_TRANSPARENCY_ENABLE => {
                    let chk = SendMessageW(H_CHK_TRANS, BM_GETCHECK, 0, 0) == BST_CHECKED as LRESULT;
                    G_EN_TU = if chk { TRUE } else { FALSE };
                    G_EN_TD = G_EN_TU;
                }
                IDC_CENTER_ENABLE => {
                    G_EN_C = if SendMessageW(H_CHK_CENTER, BM_GETCHECK, 0, 0) == BST_CHECKED as LRESULT { TRUE } else { FALSE };
                }
                IDC_SHAKE_ENABLE => {
                    G_EN_S = if SendMessageW(H_CHK_SHAKE, BM_GETCHECK, 0, 0) == BST_CHECKED as LRESULT { TRUE } else { FALSE };
                }
                IDC_SAVE_BUTTON => {
                    G_TRANSPARENCY_STEP = SendMessageW(H_SLIDER, TBM_GETPOS, 0, 0) as i32;
                    save_config();
                    register_hotkeys(G_MAIN_WND);
                    MessageBoxW(hwnd, wide("设置已保存！\0").as_ptr(), wide("提示\0").as_ptr(), MB_OK | MB_ICONINFORMATION);
                }
                _ if id as i32 == IDCANCEL => {
                    register_hotkeys(G_MAIN_WND);
                    DestroyWindow(hwnd);
                    G_SETTINGS_WND = ptr::null_mut();
                }
                _ => {}
            }
        }

        WM_KEYDOWN | WM_SYSKEYDOWN => {
            if G_LISTENING != 0 {
                if GetTickCount().wrapping_sub(G_LISTEN_START) > 10000 {
                    G_LISTENING = FALSE;
                    SetWindowTextW(G_LISTEN_BTN, wide("设置\0").as_ptr());
                    SetWindowTextW(G_LISTEN_DISP, wide("监听超时，请重试\0").as_ptr());
                    G_LISTEN_TYPE = 0;
                    return 0;
                }
                let mut mods: UINT = 0;
                if GetKeyState(VK_CONTROL) as u16 & 0x8000 != 0 { mods |= MOD_CONTROL as UINT; }
                if GetKeyState(VK_MENU) as u16 & 0x8000 != 0 { mods |= MOD_ALT as UINT; }
                if GetKeyState(VK_SHIFT) as u16 & 0x8000 != 0 { mods |= MOD_SHIFT as UINT; }
                if GetKeyState(VK_LWIN) as u16 & 0x8000 != 0 { mods |= MOD_WIN as UINT; }

                let vk = wparam as UINT;
                if mods == 0 && !(vk >= 0x70 && vk <= 0x87) {
                    return 0;
                }

                match G_LISTEN_TYPE {
                    1 => { G_TU_MODS = mods; G_TU_KEY = vk; }
                    2 => { G_TD_MODS = mods; G_TD_KEY = vk; }
                    3 => { G_C_MODS = mods; G_C_KEY = vk; }
                    4 => { G_S_MODS = mods; G_S_KEY = vk; }
                    _ => {}
                }

                SetWindowTextW(G_LISTEN_DISP, wide(&format!("{}\0", hotkey_text(mods, vk))).as_ptr());
                G_LISTENING = FALSE;
                G_LISTEN_TYPE = 0;
                SetWindowTextW(G_LISTEN_BTN, wide("设置\0").as_ptr());
            }
        }

        WM_CLOSE => {
            register_hotkeys(G_MAIN_WND);
            DestroyWindow(hwnd);
            G_SETTINGS_WND = ptr::null_mut();
        }

        _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
    }
    0
}

// ── 主窗口 ──
unsafe extern "system" fn window_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            SetTimer(hwnd, 1, 10, None);
            SetTimer(hwnd, 2, 5000, None);
        }

        WM_TIMER => {
            match wparam {
                1 => {
                    // 定时守护：读优先，只在需要时写
                    let mut i = 0;
                    while i < G_TW.len() {
                        let tw = &G_TW[i];
                        if IsWindow(tw.hwnd) == 0 {
                            G_TW.swap_remove(i);
                            continue;
                        }
                        let es = GetWindowLongW(tw.hwnd, GWL_EXSTYLE);
                        let has_layered = es & WS_EX_LAYERED as i32 != 0;
                        if !has_layered {
                            SetWindowLongW(tw.hwnd, GWL_EXSTYLE, es | WS_EX_LAYERED as i32);
                            SetLayeredWindowAttributes(tw.hwnd, 0, tw.alpha as u8, LWA_ALPHA);
                        } else {
                            let mut alpha: u8 = 0;
                            if GetLayeredWindowAttributes(tw.hwnd, ptr::null_mut(), &mut alpha, ptr::null_mut()) != 0 {
                                if alpha as i32 != tw.alpha {
                                    SetLayeredWindowAttributes(tw.hwnd, 0, tw.alpha as u8, LWA_ALPHA);
                                }
                            }
                        }
                        i += 1;
                    }
                }
                2 => {
                    // 清理已销毁窗口
                    G_TW.retain(|tw| IsWindow(tw.hwnd) != 0);
                }
                _ => {}
            }
        }

        WM_HOTKEY => {
            match wparam as i32 {
                ID_HOTKEY_TRANSPARENCY_UP => adjust_transparency(true),
                ID_HOTKEY_TRANSPARENCY_DOWN => adjust_transparency(false),
                ID_HOTKEY_CENTER_WINDOW => center_window(),
                ID_HOTKEY_SHAKE_WINDOW => shake_window(),
                ID_HOTKEY_RESTORE_OPACITY => restore_opacity(),
                _ => {}
            }
        }

        WM_TRAYICON => {
            if lparam as UINT == WM_RBUTTONUP {
                show_context_menu(hwnd);
            }
        }

        WM_COMMAND => {
            match LOWORD(wparam as DWORD) as UINT {
                ID_TRAY_SETTINGS => show_settings_window(),
                ID_TRAY_EXIT => { PostQuitMessage(0); }
                _ => {}
            }
        }

        WM_DESTROY => {
            KillTimer(hwnd, 1);
            KillTimer(hwnd, 2);
            PostQuitMessage(0);
        }

        _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
    }
    0
}

// ── 入口 ──
fn main() {
    unsafe {
        InitCommonControls();

        // 单实例
        let existing = FindWindowW(wide("Window2ClearClass\0").as_ptr(), ptr::null());
        if !existing.is_null() {
            MessageBoxW(ptr::null_mut(), wide("Window2Clear 已在运行中！\0").as_ptr(),
                wide("提示\0").as_ptr(), MB_OK | MB_ICONINFORMATION);
            return;
        }

        let mut wc: WNDCLASSEXW = mem::zeroed();
        wc.cbSize = mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(window_proc);
        wc.hInstance = GetModuleHandleW(ptr::null_mut());
        wc.lpszClassName = wide("Window2ClearClass\0").as_ptr();
        wc.hCursor = LoadCursorW(ptr::null_mut(), IDC_ARROW);
        wc.hbrBackground = (COLOR_WINDOW + 1) as HBRUSH;
        RegisterClassExW(&wc);

        load_config();

        G_MAIN_WND = CreateWindowExW(0, wide("Window2ClearClass\0").as_ptr(), wide("Window2Clear\0").as_ptr(),
            WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, CW_USEDEFAULT, 400, 300,
            ptr::null_mut(), ptr::null_mut(), GetModuleHandleW(ptr::null_mut()), ptr::null_mut());

        create_tray_icon(G_MAIN_WND);
        register_hotkeys(G_MAIN_WND);

        let msg = format!(
            "Window2Clear {} 已启动！\n\n默认热键：\n- Alt+\u{2190}/\u{2192} 调整窗口透明度\n- Alt+\u{2191} 恢复窗口不透明\n- Ctrl+数字键5 窗口居中（需开启）\n- Alt+\u{2193} 窗口抖动（需开启）\n\n右键点击托盘图标进行设置\0",
            APP_VERSION
        );
        MessageBoxW(ptr::null_mut(), wide(&msg).as_ptr(), wide("Window2Clear 启动成功\0").as_ptr(), MB_OK | MB_ICONINFORMATION);

        let mut msg: MSG = mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        unregister_hotkeys(G_MAIN_WND);
        remove_tray_icon();
    }
}
