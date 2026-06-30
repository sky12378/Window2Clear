#![windows_subsystem = "windows"]
#![allow(non_snake_case, unused)]

use std::cell::RefCell;
use std::ptr;
use std::mem;

#[cfg(windows)]
extern crate winapi;

use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::um::commctrl::*;
use winapi::um::libloaderapi::*;
use winapi::um::shellapi::*;
use winapi::um::sysinfoapi::*;
use winapi::um::winbase::*;
use winapi::um::winuser::*;

// ── 常量 ──
const WM_TRAYICON: UINT = WM_USER + 1;
const ID_TRAY_EXIT: UINT = 1001;
const ID_TRAY_SETTINGS: UINT = 1002;
const ID_HOTKEY_TRANSPARENCY_UP: i32 = 1;
const ID_HOTKEY_TRANSPARENCY_DOWN: i32 = 2;

const IDC_TRANSPARENCY_UP_BUTTON: UINT = 2001;
const IDC_TRANSPARENCY_DOWN_BUTTON: UINT = 2002;
const IDC_TRANSPARENCY_SLIDER: UINT = 2005;
const IDC_TRANSPARENCY_LABEL: UINT = 2007;
const IDC_TRANSPARENCY_UP_DISPLAY: UINT = 2008;
const IDC_TRANSPARENCY_DOWN_DISPLAY: UINT = 2009;
const IDC_TRANSPARENCY_ENABLE: UINT = 2012;
const IDC_SAVE_BUTTON: UINT = 2006;

const APP_VERSION: &str = "v3-rust";

// ── 透明窗口跟踪 ──
#[derive(Clone, Copy)]
struct TransparentWindow {
    hwnd: HWND,
    alpha: i32,
}

// ── 全局状态（thread_local 单线程模型，配合 Win32 消息循环） ──
struct State {
    main_wnd: HWND,
    settings_wnd: HWND,
    nid: NOTIFYICONDATAW,
    transparency_step: i32,

    // 热键（仅透明度增减）
    tu_mods: UINT, tu_key: UINT,
    td_mods: UINT, td_key: UINT,

    // 总开关：tu/td 同步启停
    en_trans: BOOL,

    // 透明窗口列表
    tw: Vec<TransparentWindow>,

    // 热键监听
    listening: BOOL,
    listen_type: i32,
    listen_btn: HWND,
    listen_disp: HWND,
    listen_start: DWORD,

    // 设置窗口控件
    h_slider: HWND,
    h_label: HWND,
    h_btn_up: HWND,
    h_btn_down: HWND,
    h_disp_up: HWND,
    h_disp_down: HWND,
    h_chk_trans: HWND,
}

impl Default for State {
    fn default() -> Self {
        Self {
            main_wnd: ptr::null_mut(),
            settings_wnd: ptr::null_mut(),
            nid: unsafe { mem::zeroed() },
            transparency_step: 10,

            tu_mods: MOD_ALT as UINT, tu_key: VK_LEFT as UINT,
            td_mods: MOD_ALT as UINT, td_key: VK_RIGHT as UINT,

            en_trans: TRUE,

            tw: Vec::new(),

            listening: FALSE,
            listen_type: 0,
            listen_btn: ptr::null_mut(),
            listen_disp: ptr::null_mut(),
            listen_start: 0,

            h_slider: ptr::null_mut(),
            h_label: ptr::null_mut(),
            h_btn_up: ptr::null_mut(),
            h_btn_down: ptr::null_mut(),
            h_disp_up: ptr::null_mut(),
            h_disp_down: ptr::null_mut(),
            h_chk_trans: ptr::null_mut(),
        }
    }
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
}

// ponytail: s/sm 闭包封装借用，保证不在 Win32 回调中持有跨调用借用。
// 单线程消息循环模型下不会重入 panic，但每条消息处理结束前必须释放借用。
fn s<R>(f: impl FnOnce(&State) -> R) -> R { S.with(|st| f(&st.borrow())) }
fn sm<R>(f: impl FnOnce(&mut State) -> R) -> R { S.with(|st| f(&mut st.borrow_mut())) }

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

fn track_window(hwnd: HWND, alpha: i32) {
    sm(|st: &mut State| {
        if let Some(idx) = st.tw.iter().position(|tw| tw.hwnd == hwnd) {
            st.tw[idx].alpha = alpha;
        } else {
            st.tw.push(TransparentWindow { hwnd, alpha });
        }
    });
}

fn untrack_window(hwnd: HWND) {
    sm(|st: &mut State| {
        if let Some(idx) = st.tw.iter().position(|tw| tw.hwnd == hwnd) {
            st.tw.swap_remove(idx);
        }
    });
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
    let (main_wnd, settings_wnd, step) = s(|st| (st.main_wnd, st.settings_wnd, st.transparency_step));
    if hwnd.is_null() || hwnd == main_wnd || hwnd == settings_wnd { return; }
    let cur = get_window_transparency(hwnd);
    let delta = 255 * step / 100;
    let new_alpha = if increase { (cur - delta).max(25) } else { (cur + delta).min(255) };
    set_window_transparency(hwnd, new_alpha);
}

// ── 热键 ──
unsafe fn register_hotkeys(hwnd: HWND) {
    unregister_hotkeys(hwnd);
    let (en, tu_m, tu_k, td_m, td_k) = s(|st| (
        st.en_trans, st.tu_mods, st.tu_key, st.td_mods, st.td_key,
    ));
    if en != 0 {
        RegisterHotKey(hwnd, ID_HOTKEY_TRANSPARENCY_UP,   tu_m, tu_k);
        RegisterHotKey(hwnd, ID_HOTKEY_TRANSPARENCY_DOWN, td_m, td_k);
    }
}

unsafe fn unregister_hotkeys(hwnd: HWND) {
    for id in [ID_HOTKEY_TRANSPARENCY_UP, ID_HOTKEY_TRANSPARENCY_DOWN] {
        UnregisterHotKey(hwnd, id);
    }
}

// ── 托盘 ──
unsafe fn create_tray_icon(hwnd: HWND) {
    sm(|st: &mut State| {
        st.nid.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
        st.nid.hWnd = hwnd;
        st.nid.uID = 1;
        st.nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        st.nid.uCallbackMessage = WM_TRAYICON;
        st.nid.hIcon = LoadIconW(ptr::null_mut(), IDI_APPLICATION);
        let tip = wide(&format!("Window2Clear {} - 窗口透明度控制", APP_VERSION));
        for (i, &c) in tip.iter().take(127).enumerate() {
            st.nid.szTip[i] = c as u16;
        }
        Shell_NotifyIconW(NIM_ADD, &mut st.nid);
    });
}

unsafe fn remove_tray_icon() {
    sm(|st: &mut State| {
        Shell_NotifyIconW(NIM_DELETE, &mut st.nid);
    });
}

unsafe fn show_context_menu(hwnd: HWND) {
    let hmenu = CreatePopupMenu();
    let m_settings = wide("设置");
    let m_exit = wide("退出");
    AppendMenuW(hmenu, MF_STRING, ID_TRAY_SETTINGS as usize, m_settings.as_ptr());
    AppendMenuW(hmenu, MF_SEPARATOR, 0, ptr::null());
    AppendMenuW(hmenu, MF_STRING, ID_TRAY_EXIT as usize, m_exit.as_ptr());
    let mut pt: POINT = mem::zeroed();
    GetCursorPos(&mut pt);
    SetForegroundWindow(hwnd);
    TrackPopupMenu(hmenu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hwnd, ptr::null());
    DestroyMenu(hmenu);
}

// ── 配置 ──
// ponytail: 基于 exe 目录拼接，避免从托盘/自启动启动时 cwd=System32 导致配置丢失。
// 落到 fallback 才退回相对路径。
unsafe fn config_path() -> Vec<u16> {
    let mut buf = [0u16; 260];
    let len = GetModuleFileNameW(ptr::null_mut(), buf.as_mut_ptr(), buf.len() as u32);
    if len == 0 || len as usize >= buf.len() {
        return wide(".\\config.ini");
    }
    let mut last = 0usize;
    for i in 0..len as usize {
        if buf[i] == b'\\' as u16 { last = i + 1; }
    }
    let mut path = buf[..last].to_vec();
    path.extend_from_slice(&wide("config.ini"));
    path
}

unsafe fn load_config() {
    let path = config_path();
    let p = path.as_ptr();
    macro_rules! gi {
        ($sec:expr, $key:expr, $def:expr) => {{
            let s = wide($sec);
            let k = wide($key);
            GetPrivateProfileIntW(s.as_ptr(), k.as_ptr(), $def, p) as i32
        }};
    }
    let step   = gi!("Settings", "TransparencyStep", 10).clamp(1, 50);
    let tu_mods = gi!("Hotkeys", "TransparencyUpModifiers",   MOD_ALT as i32) as UINT;
    let tu_key  = gi!("Hotkeys", "TransparencyUpKey",         VK_LEFT)         as UINT;
    let td_mods = gi!("Hotkeys", "TransparencyDownModifiers", MOD_ALT as i32)  as UINT;
    let td_key  = gi!("Hotkeys", "TransparencyDownKey",       VK_RIGHT)        as UINT;
    // ponytail: 配置迁移。旧版用 EnableTransparencyUp/Down 分开存储，
    // 新版合并为 EnableTransparency。新键缺失时回退读旧键 EnableTransparencyUp，
    // 保留旧版"禁用透明度"的用户偏好，避免静默重新启用。
    let en_trans = {
        let raw = gi!("Switches", "EnableTransparency", -1);
        if raw == -1 {
            gi!("Switches", "EnableTransparencyUp", 1)
        } else {
            raw
        }
    };
    sm(|st: &mut State| {
        st.transparency_step = step;
        st.tu_mods = tu_mods; st.tu_key = tu_key;
        st.td_mods = td_mods; st.td_key = td_key;
        st.en_trans = en_trans;
    });
}

unsafe fn save_config() {
    let path = config_path();
    let p = path.as_ptr();
    macro_rules! si {
        ($sec:expr, $key:expr, $val:expr) => {{
            let s = wide($sec);
            let k = wide($key);
            let v = wide(&format!("{}", $val));
            WritePrivateProfileStringW(s.as_ptr(), k.as_ptr(), v.as_ptr(), p);
        }};
    }
    let (step, tu_m, tu_k, td_m, td_k, en) = s(|st| (
        st.transparency_step,
        st.tu_mods, st.tu_key, st.td_mods, st.td_key,
        st.en_trans,
    ));
    si!("Settings", "TransparencyStep", step);
    si!("Hotkeys", "TransparencyUpModifiers",   tu_m);
    si!("Hotkeys", "TransparencyUpKey",         tu_k);
    si!("Hotkeys", "TransparencyDownModifiers", td_m);
    si!("Hotkeys", "TransparencyDownKey",       td_k);
    si!("Switches", "EnableTransparency", if en != 0 { 1 } else { 0 });
}

// ── 设置窗口 ──
unsafe fn child(parent: HWND, class: &str, text: &str, style: DWORD, x: i32, y: i32, w: i32, h: i32, id: UINT) -> HWND {
    let cls = wide(class);
    let txt = wide(text);
    CreateWindowExW(0, cls.as_ptr(), txt.as_ptr(), style,
        x, y, w, h, parent, id as HMENU, GetModuleHandleW(ptr::null_mut()), ptr::null_mut())
}

unsafe fn show_settings_window() {
    let existing = s(|st| st.settings_wnd);
    if !existing.is_null() {
        SetForegroundWindow(existing);
        return;
    }
    let mut wc: WNDCLASSEXW = mem::zeroed();
    wc.cbSize = mem::size_of::<WNDCLASSEXW>() as u32;
    wc.lpfnWndProc = Some(settings_proc);
    wc.hInstance = GetModuleHandleW(ptr::null_mut());
    let cls_name = wide("SettingsWindowClass");
    wc.lpszClassName = cls_name.as_ptr();
    wc.hCursor = LoadCursorW(ptr::null_mut(), IDC_ARROW);
    wc.hbrBackground = (COLOR_WINDOW + 1) as HBRUSH;
    RegisterClassExW(&wc);

    let (ww, wh) = (380i32, 270i32);
    let sw = GetSystemMetrics(SM_CXSCREEN);
    let sh = GetSystemMetrics(SM_CYSCREEN);
    let title = wide(&format!("Window2Clear {} 设置", APP_VERSION));
    let hwnd = CreateWindowExW(WS_EX_CONTROLPARENT, cls_name.as_ptr(), title.as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
        (sw - ww) / 2, (sh - wh) / 2, ww, wh,
        ptr::null_mut(), ptr::null_mut(), GetModuleHandleW(ptr::null_mut()), ptr::null_mut());
    sm(|st: &mut State| st.settings_wnd = hwnd);
    if !hwnd.is_null() {
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
    }
}

unsafe extern "system" fn settings_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            // ponytail: State 是 thread_local 单例，跨设置窗口开关复用；
            // 上次关闭若残留 listening=TRUE，再次打开后首次按键会走"监听超时"分支
            // 且 SetWindowTextW 对已销毁控件 HWND 静默失败。在此重置为初始态。
            sm(|st: &mut State| {
                st.listening = FALSE;
                st.listen_type = 0;
                st.listen_btn = ptr::null_mut();
                st.listen_disp = ptr::null_mut();
                st.listen_start = 0;
            });
            let main_wnd = s(|st| st.main_wnd);
            unregister_hotkeys(main_wnd);
            let (en_trans, step, tu_m, tu_k, td_m, td_k) = s(|st| (
                st.en_trans, st.transparency_step,
                st.tu_mods, st.tu_key, st.td_mods, st.td_key,
            ));
            let mut y = 20;

            child(hwnd, "STATIC", "透明度控制:", WS_VISIBLE | WS_CHILD, 20, y, 120, 20, 0);
            let chk_trans = child(hwnd, "BUTTON", "启用", WS_VISIBLE | WS_CHILD | BS_AUTOCHECKBOX, 150, y, 60, 20, IDC_TRANSPARENCY_ENABLE);
            SendMessageW(chk_trans, BM_SETCHECK, if en_trans != 0 { BST_CHECKED } else { BST_UNCHECKED } as WPARAM, 0);
            y += 30;

            child(hwnd, "STATIC", "增加透明度:", WS_VISIBLE | WS_CHILD, 30, y, 80, 20, 0);
            let disp_up = child(hwnd, "EDIT", &hotkey_text(tu_m, tu_k),
                WS_VISIBLE | WS_CHILD | WS_BORDER | ES_AUTOHSCROLL | ES_READONLY, 120, y, 140, 20, IDC_TRANSPARENCY_UP_DISPLAY);
            let btn_up = child(hwnd, "BUTTON", "设置", WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON, 270, y, 50, 20, IDC_TRANSPARENCY_UP_BUTTON);
            y += 30;

            child(hwnd, "STATIC", "减少透明度:", WS_VISIBLE | WS_CHILD, 30, y, 80, 20, 0);
            let disp_down = child(hwnd, "EDIT", &hotkey_text(td_m, td_k),
                WS_VISIBLE | WS_CHILD | WS_BORDER | ES_AUTOHSCROLL | ES_READONLY, 120, y, 140, 20, IDC_TRANSPARENCY_DOWN_DISPLAY);
            let btn_down = child(hwnd, "BUTTON", "设置", WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON, 270, y, 50, 20, IDC_TRANSPARENCY_DOWN_BUTTON);
            y += 40;

            let label = child(hwnd, "STATIC", &format!("透明度步长: {}%", step),
                WS_VISIBLE | WS_CHILD, 20, y, 200, 20, IDC_TRANSPARENCY_LABEL);
            y += 25;
            let slider = child(hwnd, "msctls_trackbar32", "",
                WS_VISIBLE | WS_CHILD | TBS_HORZ | TBS_AUTOTICKS, 20, y, 250, 30, IDC_TRANSPARENCY_SLIDER);
            SendMessageW(slider, TBM_SETRANGE, TRUE as WPARAM, MAKELONG(1, 50) as LPARAM);
            SendMessageW(slider, TBM_SETPOS, TRUE as WPARAM, step as LPARAM);
            y += 50;

            child(hwnd, "BUTTON", "保存设置", WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON, 20, y, 100, 30, IDC_SAVE_BUTTON);
            child(hwnd, "BUTTON", "关闭", WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON, 140, y, 100, 30, IDCANCEL as UINT);

            sm(|st: &mut State| {
                st.h_chk_trans = chk_trans;
                st.h_disp_up = disp_up;
                st.h_disp_down = disp_down;
                st.h_btn_up = btn_up;
                st.h_btn_down = btn_down;
                st.h_label = label;
                st.h_slider = slider;
            });
        }

        WM_HSCROLL => {
            let (slider, label) = s(|st| (st.h_slider, st.h_label));
            if lparam as HWND == slider {
                let pos = SendMessageW(slider, TBM_GETPOS, 0, 0) as i32;
                let text = wide(&format!("透明度步长: {}%", pos));
                SetWindowTextW(label, text.as_ptr());
            }
        }

        WM_COMMAND => {
            let id = LOWORD(wparam as DWORD);
            match id as UINT {
                IDC_TRANSPARENCY_UP_BUTTON | IDC_TRANSPARENCY_DOWN_BUTTON => {
                    let type_id = match id as UINT {
                        IDC_TRANSPARENCY_UP_BUTTON => 1,
                        IDC_TRANSPARENCY_DOWN_BUTTON => 2,
                        _ => 0,
                    };
                    let (listening, listen_type) = s(|st| (st.listening, st.listen_type));
                    if listening != 0 && listen_type == type_id {
                        // 取消监听
                        let (tu_m, tu_k, td_m, td_k) = s(|st| (
                            st.tu_mods, st.tu_key, st.td_mods, st.td_key,
                        ));
                        let text = match type_id {
                            1 => hotkey_text(tu_m, tu_k),
                            2 => hotkey_text(td_m, td_k),
                            _ => String::new(),
                        };
                        let (btn, disp) = s(|st| match type_id {
                            1 => (st.h_btn_up,   st.h_disp_up),
                            2 => (st.h_btn_down, st.h_disp_down),
                            _ => (ptr::null_mut(), ptr::null_mut()),
                        });
                        let w_set = wide("设置");
                        let w_text = wide(&text);
                        SetWindowTextW(btn, w_set.as_ptr());
                        SetWindowTextW(disp, w_text.as_ptr());
                        sm(|st: &mut State| { st.listening = FALSE; st.listen_type = 0; });
                    } else {
                        // 切换监听目标前，先把旧目标按钮/显示框复位
                        if listening != 0 && listen_type != type_id {
                            let (old_type, old_btn, old_disp,
                                tu_m, tu_k, td_m, td_k) = s(|st| (
                                st.listen_type, st.listen_btn, st.listen_disp,
                                st.tu_mods, st.tu_key, st.td_mods, st.td_key,
                            ));
                            let old_text = match old_type {
                                1 => hotkey_text(tu_m, tu_k),
                                2 => hotkey_text(td_m, td_k),
                                _ => String::new(),
                            };
                            let w_set = wide("设置");
                            let w_old = wide(&old_text);
                            SetWindowTextW(old_btn, w_set.as_ptr());
                            SetWindowTextW(old_disp, w_old.as_ptr());
                        }
                        let (btn, disp) = s(|st| match type_id {
                            1 => (st.h_btn_up,   st.h_disp_up),
                            2 => (st.h_btn_down, st.h_disp_down),
                            _ => (ptr::null_mut(), ptr::null_mut()),
                        });
                        let w_cancel = wide("取消");
                        let w_prompt = wide("请按下组合键...");
                        SetWindowTextW(btn, w_cancel.as_ptr());
                        SetWindowTextW(disp, w_prompt.as_ptr());
                        sm(|st: &mut State| {
                            st.listening = TRUE;
                            st.listen_type = type_id;
                            st.listen_btn = btn;
                            st.listen_disp = disp;
                            st.listen_start = GetTickCount();
                        });
                    }
                }
                IDC_TRANSPARENCY_ENABLE => {
                    let chk_trans = s(|st| st.h_chk_trans);
                    let chk = SendMessageW(chk_trans, BM_GETCHECK, 0, 0) == BST_CHECKED as LRESULT;
                    sm(|st: &mut State| st.en_trans = if chk { TRUE } else { FALSE });
                    // 取消勾选时恢复所有已透明窗口，与"总开关"语义一致
                    if !chk {
                        let snapshot: Vec<HWND> = s(|st| st.tw.iter().map(|tw| tw.hwnd).collect());
                        for hwnd in &snapshot {
                            set_window_transparency(*hwnd, 255);
                        }
                    }
                }
                IDC_SAVE_BUTTON => {
                    let (slider, main_wnd) = s(|st| (st.h_slider, st.main_wnd));
                    let pos = SendMessageW(slider, TBM_GETPOS, 0, 0) as i32;
                    sm(|st: &mut State| st.transparency_step = pos);
                    save_config();
                    register_hotkeys(main_wnd);
                    let w_msg = wide("设置已保存！");
                    let w_title = wide("提示");
                    MessageBoxW(hwnd, w_msg.as_ptr(), w_title.as_ptr(), MB_OK | MB_ICONINFORMATION);
                }
                _ if id as i32 == IDCANCEL => {
                    let main_wnd = s(|st| st.main_wnd);
                    register_hotkeys(main_wnd);
                    DestroyWindow(hwnd);
                    sm(|st: &mut State| st.settings_wnd = ptr::null_mut());
                }
                _ => {}
            }
        }

        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let listening = s(|st| st.listening);
            // 未监听时走默认处理，恢复 Tab/Enter 等对话框语义
            if listening == 0 {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            let (listen_start, listen_btn, listen_disp, listen_type) = s(|st| (
                st.listen_start, st.listen_btn, st.listen_disp, st.listen_type
            ));
            if GetTickCount().wrapping_sub(listen_start) > 10000 {
                let w_set = wide("设置");
                let w_to = wide("监听超时，请重试");
                SetWindowTextW(listen_btn, w_set.as_ptr());
                SetWindowTextW(listen_disp, w_to.as_ptr());
                sm(|st: &mut State| { st.listening = FALSE; st.listen_type = 0; });
                return 0;
            }
            let mut mods: UINT = 0;
            if GetKeyState(VK_CONTROL) as u16 & 0x8000 != 0 { mods |= MOD_CONTROL as UINT; }
            if GetKeyState(VK_MENU) as u16 & 0x8000 != 0 { mods |= MOD_ALT as UINT; }
            if GetKeyState(VK_SHIFT) as u16 & 0x8000 != 0 { mods |= MOD_SHIFT as UINT; }
            if GetKeyState(VK_LWIN) as u16 & 0x8000 != 0 || GetKeyState(VK_RWIN) as u16 & 0x8000 != 0 {
                mods |= MOD_WIN as UINT;
            }

            let vk = wparam as UINT;
            // ponytail: 排除裸修饰键，否则会被记成 Ctrl+VK_CONTROL 导致 RegisterHotKey 静默失败
            if matches!(vk, 0x10..=0x12 | 0x5B..=0x5C | 0xA0..=0xA5) {
                return 0;
            }
            if mods == 0 && !(vk >= 0x70 && vk <= 0x87) {
                return 0;
            }

            sm(|st: &mut State| match listen_type {
                1 => { st.tu_mods = mods; st.tu_key = vk; }
                2 => { st.td_mods = mods; st.td_key = vk; }
                _ => {}
            });

            let w_text = wide(&format!("{}", hotkey_text(mods, vk)));
            SetWindowTextW(listen_disp, w_text.as_ptr());
            let w_set = wide("设置");
            SetWindowTextW(listen_btn, w_set.as_ptr());
            sm(|st: &mut State| { st.listening = FALSE; st.listen_type = 0; });
        }

        WM_CLOSE => {
            let main_wnd = s(|st| st.main_wnd);
            register_hotkeys(main_wnd);
            DestroyWindow(hwnd);
            sm(|st: &mut State| st.settings_wnd = ptr::null_mut());
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
        }

        WM_TIMER => {
            if wparam != 1 { return 0; }
            // ponytail: 守护透明窗口的 layered + alpha。无条件重设，避免目标应用在
            // 鼠标 hover/重绘等事件里短暂改 exstyle 或 alpha 后，原版基于
            // GetLayeredWindowAttributes 漏判导致透明度被覆盖丢失。
            // 上限：跟踪窗口数量稀少（通常 <10），每 10ms 一次 SetLayeredWindowAttributes
            // 调用 CPU 开销可忽略；若未来跟踪数百窗口需改为 dirty-flag 模式。
            let snapshot: Vec<TransparentWindow> = s(|st| st.tw.clone());
            if snapshot.is_empty() { return 0; }
            let mut dead: Vec<HWND> = Vec::new();
            for tw in &snapshot {
                if IsWindow(tw.hwnd) == 0 {
                    dead.push(tw.hwnd);
                    continue;
                }
                let es = GetWindowLongW(tw.hwnd, GWL_EXSTYLE);
                if es & WS_EX_LAYERED as i32 == 0 {
                    SetWindowLongW(tw.hwnd, GWL_EXSTYLE, es | WS_EX_LAYERED as i32);
                }
                SetLayeredWindowAttributes(tw.hwnd, 0, tw.alpha as u8, LWA_ALPHA);
            }
            if !dead.is_empty() {
                sm(|st: &mut State| st.tw.retain(|tw| !dead.contains(&tw.hwnd)));
            }
        }

        WM_HOTKEY => {
            match wparam as i32 {
                ID_HOTKEY_TRANSPARENCY_UP => adjust_transparency(true),
                ID_HOTKEY_TRANSPARENCY_DOWN => adjust_transparency(false),
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
        let class_name = wide("Window2ClearClass");
        let existing = FindWindowW(class_name.as_ptr(), ptr::null());
        if !existing.is_null() {
            let w_msg = wide("Window2Clear 已在运行中！");
            let w_title = wide("提示");
            MessageBoxW(ptr::null_mut(), w_msg.as_ptr(), w_title.as_ptr(), MB_OK | MB_ICONINFORMATION);
            return;
        }

        let mut wc: WNDCLASSEXW = mem::zeroed();
        wc.cbSize = mem::size_of::<WNDCLASSEXW>() as u32;
        wc.lpfnWndProc = Some(window_proc);
        wc.hInstance = GetModuleHandleW(ptr::null_mut());
        wc.lpszClassName = class_name.as_ptr();
        wc.hCursor = LoadCursorW(ptr::null_mut(), IDC_ARROW);
        wc.hbrBackground = (COLOR_WINDOW + 1) as HBRUSH;
        RegisterClassExW(&wc);

        load_config();

        let wnd_title = wide("Window2Clear");
        let main_wnd = CreateWindowExW(0, class_name.as_ptr(), wnd_title.as_ptr(),
            WS_OVERLAPPEDWINDOW, CW_USEDEFAULT, CW_USEDEFAULT, 400, 300,
            ptr::null_mut(), ptr::null_mut(), GetModuleHandleW(ptr::null_mut()), ptr::null_mut());
        sm(|st: &mut State| st.main_wnd = main_wnd);

        create_tray_icon(main_wnd);
        register_hotkeys(main_wnd);

        let msg = format!(
            "Window2Clear {} 已启动！\n\n默认热键：\n- Alt+\u{2190} 增加窗口透明度\n- Alt+\u{2192} 减少窗口透明度\n\n右键点击托盘图标进行设置",
            APP_VERSION
        );
        let w_msg = wide(&msg);
        let w_title = wide("Window2Clear 启动成功");
        MessageBoxW(ptr::null_mut(), w_msg.as_ptr(), w_title.as_ptr(), MB_OK | MB_ICONINFORMATION);

        let mut msg: MSG = mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) != 0 {
            // 设置窗口存在时走 IsDialogMessage，恢复 Tab/Enter 等对话框语义
            let settings_wnd = s(|st| st.settings_wnd);
            if !settings_wnd.is_null() && IsDialogMessageW(settings_wnd, &mut msg) != 0 {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        unregister_hotkeys(main_wnd);
        remove_tray_icon();
    }
}
