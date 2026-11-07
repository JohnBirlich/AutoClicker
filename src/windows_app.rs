#![allow(non_camel_case_types)]

use crate::clicker::{parse_cps, SpeedMode};
use std::ffi::{c_void, OsString};
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[allow(clippy::upper_case_acronyms)]
type HWND = *mut c_void;
type HINSTANCE = *mut c_void;
type HICON = *mut c_void;
type HCURSOR = *mut c_void;
type HBRUSH = *mut c_void;
type HPEN = *mut c_void;
type HFONT = *mut c_void;
type HDC = *mut c_void;
type HGDIOBJ = *mut c_void;
type HMENU = *mut c_void;
type WPARAM = usize;
type LPARAM = isize;
type LRESULT = isize;
type UINT = u32;
type BOOL = i32;

type WindowProc = unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT;

#[repr(C)]
struct WndClassW {
    style: u32,
    lpfn_wnd_proc: Option<WindowProc>,
    cb_cls_extra: i32,
    cb_wnd_extra: i32,
    h_instance: HINSTANCE,
    h_icon: HICON,
    h_cursor: HCURSOR,
    hbr_background: HBRUSH,
    lpsz_menu_name: *const u16,
    lpsz_class_name: *const u16,
}

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct Msg {
    hwnd: HWND,
    message: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
    time: u32,
    pt: Point,
    l_private: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct DrawItemStruct {
    ctl_type: u32,
    ctl_id: u32,
    item_id: u32,
    item_action: u32,
    item_state: u32,
    hwnd_item: HWND,
    hdc: HDC,
    rc_item: Rect,
    item_data: usize,
}

#[repr(C)]
struct PaintStruct {
    hdc: HDC,
    erase: BOOL,
    paint_rect: Rect,
    restore: BOOL,
    inc_update: BOOL,
    reserved: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MouseInput {
    dx: i32,
    dy: i32,
    mouse_data: u32,
    flags: u32,
    time: u32,
    extra_info: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
union InputData {
    mouse: MouseInput,
    padding: [usize; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Input {
    input_type: u32,
    data: InputData,
}

#[link(name = "user32")]
extern "system" {
    fn SetProcessDPIAware() -> BOOL;
    fn GetDC(window: HWND) -> HDC;
    fn ReleaseDC(window: HWND, dc: HDC) -> i32;
    fn RegisterClassW(class: *const WndClassW) -> u16;
    fn UnregisterClassW(class_name: *const u16, instance: HINSTANCE) -> BOOL;
    fn CreateWindowExW(
        ex_style: u32,
        class_name: *const u16,
        window_name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: HWND,
        menu: HMENU,
        instance: HINSTANCE,
        param: *mut c_void,
    ) -> HWND;
    fn DefWindowProcW(hwnd: HWND, message: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
    fn ShowWindow(hwnd: HWND, command: i32) -> BOOL;
    fn UpdateWindow(hwnd: HWND) -> BOOL;
    fn GetMessageW(message: *mut Msg, hwnd: HWND, min: UINT, max: UINT) -> BOOL;
    fn TranslateMessage(message: *const Msg) -> BOOL;
    fn DispatchMessageW(message: *const Msg) -> LRESULT;
    fn PostQuitMessage(exit_code: i32);
    fn RegisterHotKey(hwnd: HWND, id: i32, modifiers: UINT, virtual_key: UINT) -> BOOL;
    fn UnregisterHotKey(hwnd: HWND, id: i32) -> BOOL;
    fn SetTimer(
        hwnd: HWND,
        id: usize,
        elapsed_ms: UINT,
        callback: Option<unsafe extern "system" fn(HWND, UINT, usize, u32)>,
    ) -> usize;
    fn KillTimer(hwnd: HWND, id: usize) -> BOOL;
    fn SetWindowTextW(hwnd: HWND, text: *const u16) -> BOOL;
    fn GetWindowTextW(hwnd: HWND, text: *mut u16, max_count: i32) -> i32;
    fn EnableWindow(hwnd: HWND, enable: BOOL) -> BOOL;
    fn SendMessageW(hwnd: HWND, message: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
    fn LoadCursorW(instance: HINSTANCE, cursor_name: *const u16) -> HCURSOR;
    fn LoadIconW(instance: HINSTANCE, icon_name: *const u16) -> HICON;
    fn SetWindowLongPtrW(hwnd: HWND, index: i32, value: isize) -> isize;
    fn GetWindowLongPtrW(hwnd: HWND, index: i32) -> isize;
    fn MessageBoxW(hwnd: HWND, text: *const u16, caption: *const u16, flags: UINT) -> i32;
    fn AdjustWindowRect(rect: *mut Rect, style: u32, menu: BOOL) -> BOOL;
    fn LoadImageW(
        instance: HINSTANCE,
        name: *const u16,
        image_type: UINT,
        width: i32,
        height: i32,
        flags: UINT,
    ) -> *mut c_void;
    fn DestroyIcon(icon: HICON) -> BOOL;
    fn SendInput(count: UINT, inputs: *const Input, input_size: i32) -> UINT;
    fn BeginPaint(window: HWND, paint: *mut PaintStruct) -> HDC;
    fn EndPaint(window: HWND, paint: *const PaintStruct) -> BOOL;
    fn InvalidateRect(window: HWND, rect: *const Rect, erase: BOOL) -> BOOL;
    fn DrawIconEx(
        hdc: HDC,
        x: i32,
        y: i32,
        icon: HICON,
        width: i32,
        height: i32,
        step: UINT,
        brush: HBRUSH,
        flags: UINT,
    ) -> BOOL;
    fn GetDlgCtrlID(window: HWND) -> i32;
    fn GetWindowTextLengthW(window: HWND) -> i32;
    fn FillRect(hdc: HDC, rect: *const Rect, brush: HBRUSH) -> i32;
    fn DrawFocusRect(hdc: HDC, rect: *const Rect) -> BOOL;
}

#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(
        window: HWND,
        attribute: u32,
        value: *const c_void,
        size: u32,
    ) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(module_name: *const u16) -> HINSTANCE;
    fn GetModuleFileNameW(module: HINSTANCE, file_name: *mut u16, size: UINT) -> UINT;
    fn GetCurrentThread() -> *mut c_void;
    fn SetThreadPriority(thread: *mut c_void, priority: i32) -> BOOL;
}

#[link(name = "gdi32")]
extern "system" {
    fn GetDeviceCaps(hdc: HDC, index: i32) -> i32;
    fn CreateSolidBrush(color: u32) -> HBRUSH;
    fn CreatePen(style: i32, width: i32, color: u32) -> HPEN;
    fn CreateFontW(
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: u32,
        underline: u32,
        strike_out: u32,
        char_set: u32,
        output_precision: u32,
        clip_precision: u32,
        quality: u32,
        pitch_and_family: u32,
        face_name: *const u16,
    ) -> HFONT;
    fn SelectObject(hdc: HDC, object: HGDIOBJ) -> HGDIOBJ;
    fn DeleteObject(object: HGDIOBJ) -> BOOL;
    fn RoundRect(
        hdc: HDC,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        ellipse_width: i32,
        ellipse_height: i32,
    ) -> BOOL;
    fn Ellipse(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> BOOL;
    fn SetTextColor(hdc: HDC, color: u32) -> u32;
    fn SetBkColor(hdc: HDC, color: u32) -> u32;
    fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    fn DrawTextW(hdc: HDC, text: *const u16, count: i32, rect: *mut Rect, format: u32) -> i32;
}

const LOGPIXELSX: i32 = 88;
const BASE_DPI: u32 = 96;
const CS_VREDRAW: u32 = 0x0001;
const CS_HREDRAW: u32 = 0x0002;
const WS_CAPTION: u32 = 0x00C0_0000;
const WS_SYSMENU: u32 = 0x0008_0000;
const WS_MINIMIZEBOX: u32 = 0x0002_0000;
const WS_CHILD: u32 = 0x4000_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
const WS_TABSTOP: u32 = 0x0001_0000;
const WS_GROUP: u32 = 0x0002_0000;
const SS_CENTER: u32 = 0x0001;
const SS_RIGHT: u32 = 0x0002;
const BS_OWNERDRAW: u32 = 0x0000_000B;
const ES_AUTOHSCROLL: u32 = 0x0000_0080;
const ES_NUMBER: u32 = 0x0000_2000;
const SW_SHOW: i32 = 5;
const CW_USEDEFAULT: i32 = 0x8000_0000u32 as i32;

const WM_CREATE: UINT = 0x0001;
const WM_DESTROY: UINT = 0x0002;
const WM_DRAWITEM: UINT = 0x002B;
const WM_CLOSE: UINT = 0x0010;
const WM_ERASEBKGND: UINT = 0x0014;
const WM_PAINT: UINT = 0x000F;
const WM_SETFONT: UINT = 0x0030;
const EM_SETLIMITTEXT: UINT = 0x00C5;
const WM_CTLCOLOREDIT: UINT = 0x0133;
const WM_CTLCOLORSTATIC: UINT = 0x0138;
const WM_SETICON: UINT = 0x0080;
const WM_NCDESTROY: UINT = 0x0082;
const WM_TIMER: UINT = 0x0113;
const WM_COMMAND: UINT = 0x0111;
const WM_HOTKEY: UINT = 0x0312;
const BN_CLICKED: u16 = 0;
const GWLP_USERDATA: i32 = -21;
const ICON_SMALL: usize = 0;
const ICON_BIG: usize = 1;
const MB_OK: UINT = 0x0000_0000;
const MB_ICONERROR: UINT = 0x0000_0010;
const MB_ICONWARNING: UINT = 0x0000_0030;
const MOD_NOREPEAT: UINT = 0x4000;
const VK_F6: UINT = 0x75;
const IMAGE_ICON: UINT = 1;
const LR_LOADFROMFILE: UINT = 0x0010;
const ODS_SELECTED: u32 = 0x0001;
const ODS_DISABLED: u32 = 0x0004;
const ODS_FOCUS: u32 = 0x0010;
const DT_CENTER: u32 = 0x0001;
const DT_VCENTER: u32 = 0x0004;
const DT_SINGLELINE: u32 = 0x0020;
const GDI_OPAQUE: i32 = 2;
const GDI_TRANSPARENT: i32 = 1;
const PS_SOLID: i32 = 0;
const FW_NORMAL: i32 = 400;
const FW_SEMIBOLD: i32 = 600;
const DEFAULT_CHARSET: u32 = 1;
const OUT_DEFAULT_PRECIS: u32 = 0;
const CLIP_DEFAULT_PRECIS: u32 = 0;
const CLEARTYPE_QUALITY: u32 = 5;
const DEFAULT_PITCH: u32 = 0;
const FF_DONTCARE: u32 = 0;
const INPUT_MOUSE: u32 = 0;
const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
const MOUSEEVENTF_RIGHTDOWN: u32 = 0x0008;
const MOUSEEVENTF_RIGHTUP: u32 = 0x0010;

const fn rgb(red: u32, green: u32, blue: u32) -> u32 {
    red | (green << 8) | (blue << 16)
}

const COLOR_BACKGROUND: u32 = rgb(22, 25, 32);
const COLOR_LEFT_PANEL: u32 = rgb(25, 27, 33);
const COLOR_RAIL: u32 = rgb(14, 16, 21);
const COLOR_CARD: u32 = rgb(35, 39, 48);
const COLOR_OPTION: u32 = rgb(45, 50, 61);
const COLOR_BORDER: u32 = rgb(58, 64, 77);
const COLOR_CYAN: u32 = rgb(34, 211, 238);
const COLOR_CYAN_DARK: u32 = rgb(8, 119, 145);
const COLOR_TEXT: u32 = rgb(240, 244, 248);
const COLOR_MUTED: u32 = rgb(148, 159, 177);

const HOTKEY_ID: i32 = 1;
const TIMER_ID: usize = 1;
const ID_MAXIMUM: u16 = 1001;
const ID_CUSTOM: u16 = 1002;
const ID_LEFT: u16 = 1003;
const ID_RIGHT: u16 = 1004;
const ID_START: u16 = 1005;
const ID_HEADING: u16 = 1101;
const ID_SUBTITLE: u16 = 1102;
const ID_SPEED_LABEL: u16 = 1103;
const ID_CPS_LABEL: u16 = 1104;
const ID_CPS_RANGE: u16 = 1105;
const ID_MOUSE_LABEL: u16 = 1106;
const ID_STATUS: u16 = 1107;
const ID_COUNTER: u16 = 1108;
const ID_CPS_EDIT: u16 = 1110;
const ID_TARGET: u16 = 1111;
const ID_HINT: u16 = 1112;
const MAX_BATCH_CLICKS: usize = 64;
const THREAD_PRIORITY_NORMAL: i32 = 0;
const THREAD_PRIORITY_ABOVE_NORMAL: i32 = 1;
const CPS_SAMPLE_WINDOW: Duration = Duration::from_secs(1);
const SPIN_WINDOW: Duration = Duration::from_micros(300);
const MAX_SLEEP_SLICE: Duration = Duration::from_millis(5);

#[derive(Clone, Copy)]
enum MouseButton {
    Left,
    Right,
}

struct AppState {
    window: HWND,
    dpi: u32,
    maximum_radio: HWND,
    custom_radio: HWND,
    cps_edit: HWND,
    left_radio: HWND,
    right_radio: HWND,
    start_button: HWND,
    status_label: HWND,
    counter_label: HWND,
    running: Arc<AtomicBool>,
    click_count: Arc<AtomicU64>,
    worker_failed: Arc<AtomicBool>,
    last_sample_clicks: u64,
    last_sample_at: Instant,
    worker: Option<JoinHandle<()>>,
    hotkey_registered: bool,
    timer_registered: bool,
    is_maximum: bool,
    mouse_button: MouseButton,
    background_brush: HBRUSH,
    left_brush: HBRUSH,
    rail_brush: HBRUSH,
    card_brush: HBRUSH,
    option_brush: HBRUSH,
    accent_brush: HBRUSH,
    stop_brush: HBRUSH,
    border_pen: HPEN,
    accent_pen: HPEN,
    title_font: HFONT,
    section_font: HFONT,
    body_font: HFONT,
    small_font: HFONT,
    button_font: HFONT,
    value_font: HFONT,
    power_font: HFONT,
    nav_font: HFONT,
    large_icon: HICON,
    small_icon: HICON,
}

pub fn run() {
    unsafe {
        SetProcessDPIAware();
        let dpi = system_dpi();
        let instance = GetModuleHandleW(null());
        let class_name = wide("AutoClickerMainWindow");
        let window_title = wide("AutoClicker");
        let embedded_icon = LoadIconW(instance, resource_id(1));
        let class_icon = if embedded_icon.is_null() {
            LoadIconW(null_mut(), resource_id(32512))
        } else {
            embedded_icon
        };
        let class_background = CreateSolidBrush(COLOR_BACKGROUND);
        let window_class = WndClassW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfn_wnd_proc: Some(window_proc),
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance: instance,
            h_icon: class_icon,
            h_cursor: LoadCursorW(null_mut(), resource_id(32512)),
            hbr_background: class_background,
            lpsz_menu_name: null(),
            lpsz_class_name: class_name.as_ptr(),
        };

        if RegisterClassW(&window_class) == 0 {
            DeleteObject(class_background);
            show_message(
                null_mut(),
                "Не удалось зарегистрировать класс главного окна.",
                MB_OK | MB_ICONERROR,
            );
            return;
        }

        let style = WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
        let mut rect = Rect {
            left: 0,
            top: 0,
            right: scale_value(980, dpi),
            bottom: scale_value(620, dpi),
        };
        AdjustWindowRect(&mut rect, style, 0);

        let window = CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_title.as_ptr(),
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            rect.right - rect.left,
            rect.bottom - rect.top,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        );

        if window.is_null() {
            UnregisterClassW(class_name.as_ptr(), instance);
            DeleteObject(class_background);
            show_message(
                null_mut(),
                "Не удалось создать главное окно AutoClicker.",
                MB_OK | MB_ICONERROR,
            );
            return;
        }

        ShowWindow(window, SW_SHOW);
        UpdateWindow(window);

        let mut message = Msg {
            hwnd: null_mut(),
            message: 0,
            w_param: 0,
            l_param: 0,
            time: 0,
            pt: Point { x: 0, y: 0 },
            l_private: 0,
        };
        loop {
            let result = GetMessageW(&mut message, null_mut(), 0, 0);
            if result <= 0 {
                break;
            }
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }

        UnregisterClassW(class_name.as_ptr(), instance);
        DeleteObject(class_background);
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => {
            let instance = unsafe { GetModuleHandleW(null()) };
            let dark_mode = 1i32;
            let dark_result = unsafe {
                DwmSetWindowAttribute(
                    window,
                    20,
                    &dark_mode as *const i32 as *const c_void,
                    size_of::<i32>() as u32,
                )
            };
            if dark_result < 0 {
                unsafe {
                    DwmSetWindowAttribute(
                        window,
                        19,
                        &dark_mode as *const i32 as *const c_void,
                        size_of::<i32>() as u32,
                    );
                }
            }
            let state = Box::new(AppState::new(window, instance, system_dpi()));
            let state_ptr = Box::into_raw(state);
            unsafe {
                SetWindowLongPtrW(window, GWLP_USERDATA, state_ptr as isize);
            }
            0
        }
        WM_COMMAND => {
            let command_id = (w_param & 0xFFFF) as u16;
            let notification = ((w_param >> 16) & 0xFFFF) as u16;
            if notification == BN_CLICKED {
                let state_ptr = unsafe { app_state_ptr(window) };
                if !state_ptr.is_null() {
                    let state = unsafe { &mut *state_ptr };
                    match command_id {
                        ID_START => state.toggle_clicking(),
                        ID_MAXIMUM | ID_CUSTOM | ID_LEFT | ID_RIGHT => {
                            state.select_option(command_id);
                        }
                        _ => {}
                    }
                }
            }
            0
        }
        WM_DRAWITEM => {
            let state_ptr = unsafe { app_state_ptr(window) };
            if !state_ptr.is_null() && l_param != 0 {
                unsafe { &mut *state_ptr }.draw_item(l_param as *const DrawItemStruct);
            }
            1
        }
        WM_PAINT => {
            let state_ptr = unsafe { app_state_ptr(window) };
            if !state_ptr.is_null() {
                unsafe { &mut *state_ptr }.paint();
                0
            } else {
                unsafe { DefWindowProcW(window, message, w_param, l_param) }
            }
        }
        WM_ERASEBKGND => 1,
        WM_CTLCOLORSTATIC => {
            let state_ptr = unsafe { app_state_ptr(window) };
            if !state_ptr.is_null() {
                unsafe { &*state_ptr }.color_static(w_param as HDC, l_param as HWND) as LRESULT
            } else {
                0
            }
        }
        WM_CTLCOLOREDIT => {
            let state_ptr = unsafe { app_state_ptr(window) };
            if !state_ptr.is_null() {
                unsafe { &*state_ptr }.color_edit(w_param as HDC) as LRESULT
            } else {
                0
            }
        }
        WM_HOTKEY => {
            if w_param == HOTKEY_ID as usize {
                let state_ptr = unsafe { app_state_ptr(window) };
                if !state_ptr.is_null() {
                    unsafe { &mut *state_ptr }.toggle_clicking();
                }
            }
            0
        }
        WM_TIMER => {
            if w_param == TIMER_ID {
                let state_ptr = unsafe { app_state_ptr(window) };
                if !state_ptr.is_null() {
                    unsafe { &mut *state_ptr }.refresh();
                }
            }
            0
        }
        WM_CLOSE => {
            unsafe { DestroyWindow(window) };
            0
        }
        WM_DESTROY => {
            let state_ptr = unsafe { app_state_ptr(window) };
            if !state_ptr.is_null() {
                unsafe { &mut *state_ptr }.shutdown();
            }
            unsafe { PostQuitMessage(0) };
            0
        }
        WM_NCDESTROY => {
            let state_ptr = unsafe { app_state_ptr(window) };
            if !state_ptr.is_null() {
                unsafe {
                    SetWindowLongPtrW(window, GWLP_USERDATA, 0);
                    drop(Box::from_raw(state_ptr));
                }
            }
            unsafe { DefWindowProcW(window, message, w_param, l_param) }
        }
        _ => unsafe { DefWindowProcW(window, message, w_param, l_param) },
    }
}

#[link(name = "user32")]
extern "system" {
    fn DestroyWindow(window: HWND) -> BOOL;
}

impl AppState {
    fn new(window: HWND, instance: HINSTANCE, dpi: u32) -> Self {
        let heading = create_control(
            window, instance, dpi, "STATIC", "Управление", WS_CHILD | WS_VISIBLE, 0,
            ID_HEADING, 104, 34, 360, 44,
        );
        let subtitle = create_control(
            window, instance, dpi, "STATIC", "Автокликер · быстрый ввод",
            WS_CHILD | WS_VISIBLE, 0, ID_SUBTITLE, 104, 79, 360, 24,
        );
        let speed_label = create_control(
            window, instance, dpi, "STATIC", "Скорость кликов", WS_CHILD | WS_VISIBLE, 0,
            ID_SPEED_LABEL, 112, 136, 300, 24,
        );
        let maximum_radio = create_control(
            window, instance, dpi, "BUTTON", "Максимум",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | BS_OWNERDRAW, 0,
            ID_MAXIMUM, 112, 170, 166, 42,
        );
        let custom_radio = create_control(
            window, instance, dpi, "BUTTON", "Своя частота",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW, 0,
            ID_CUSTOM, 292, 170, 170, 42,
        );
        let cps_label = create_control(
            window, instance, dpi, "STATIC", "Кликов в секунду", WS_CHILD | WS_VISIBLE, 0,
            ID_CPS_LABEL, 112, 231, 160, 20,
        );
        let cps_range = create_control(
            window, instance, dpi, "STATIC", "CPS · без лимита",
            WS_CHILD | WS_VISIBLE | SS_RIGHT, 0, ID_CPS_RANGE, 286, 231, 176, 20,
        );
        let cps_edit = create_control(
            window, instance, dpi, "EDIT", "1000",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_NUMBER | ES_AUTOHSCROLL,
            0, ID_CPS_EDIT, 126, 264, 322, 38,
        );
        let mouse_label = create_control(
            window, instance, dpi, "STATIC", "Кнопка мыши", WS_CHILD | WS_VISIBLE, 0,
            ID_MOUSE_LABEL, 112, 350, 300, 24,
        );
        let left_radio = create_control(
            window, instance, dpi, "BUTTON", "Левая",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | BS_OWNERDRAW, 0,
            ID_LEFT, 112, 385, 166, 38,
        );
        let right_radio = create_control(
            window, instance, dpi, "BUTTON", "Правая",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW, 0,
            ID_RIGHT, 292, 385, 170, 38,
        );
        let start_button = create_control(
            window, instance, dpi, "BUTTON", "Питание",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW, 0,
            ID_START, 600, 136, 280, 280,
        );
        let target_label = create_control(
            window, instance, dpi, "STATIC", "Windows · SendInput     ⌄",
            WS_CHILD | WS_VISIBLE | SS_CENTER, 0, ID_TARGET, 596, 26, 288, 42,
        );
        let status_label = create_control(
            window, instance, dpi, "STATIC", "● ГОТОВ · F6",
            WS_CHILD | WS_VISIBLE | SS_CENTER, 0, ID_STATUS, 596, 444, 288, 24,
        );
        let counter_label = create_control(
            window, instance, dpi, "STATIC", "Кликов: 0",
            WS_CHILD | WS_VISIBLE | SS_CENTER, 0, ID_COUNTER, 596, 474, 288, 22,
        );
        let hint = create_control(
            window, instance, dpi, "STATIC", "F6 — запуск / остановка",
            WS_CHILD | WS_VISIBLE | SS_CENTER, 0, ID_HINT, 596, 526, 288, 22,
        );

        let title_font = create_ui_font(-30, FW_SEMIBOLD, dpi);
        let section_font = create_ui_font(-17, FW_SEMIBOLD, dpi);
        let body_font = create_ui_font(-16, FW_NORMAL, dpi);
        let small_font = create_ui_font(-14, FW_NORMAL, dpi);
        let button_font = create_ui_font(-16, FW_SEMIBOLD, dpi);
        let value_font = create_ui_font(-24, FW_SEMIBOLD, dpi);
        let power_font = create_ui_font_family(-62, FW_NORMAL, dpi, "Segoe UI Symbol");
        let nav_font = create_ui_font_family(-24, FW_NORMAL, dpi, "Segoe UI Symbol");
        unsafe {
            SendMessageW(cps_edit, EM_SETLIMITTEXT, 0x7FFF_FFFE, 0);
            SendMessageW(heading, WM_SETFONT, title_font as usize, 1);
            SendMessageW(subtitle, WM_SETFONT, body_font as usize, 1);
            SendMessageW(speed_label, WM_SETFONT, section_font as usize, 1);
            SendMessageW(mouse_label, WM_SETFONT, section_font as usize, 1);
            for control in [cps_label, cps_range, status_label, counter_label, hint] {
                SendMessageW(control, WM_SETFONT, small_font as usize, 1);
            }
            SendMessageW(cps_edit, WM_SETFONT, value_font as usize, 1);
            SendMessageW(target_label, WM_SETFONT, body_font as usize, 1);
        }

        let mut state = Self {
            window,
            dpi,
            maximum_radio,
            custom_radio,
            cps_edit,
            left_radio,
            right_radio,
            start_button,
            status_label,
            counter_label,
            running: Arc::new(AtomicBool::new(false)),
            click_count: Arc::new(AtomicU64::new(0)),
            worker_failed: Arc::new(AtomicBool::new(false)),
            last_sample_clicks: 0,
            last_sample_at: Instant::now(),
            worker: None,
            hotkey_registered: false,
            timer_registered: false,
            is_maximum: true,
            mouse_button: MouseButton::Left,
            background_brush: unsafe { CreateSolidBrush(COLOR_BACKGROUND) },
            left_brush: unsafe { CreateSolidBrush(COLOR_LEFT_PANEL) },
            rail_brush: unsafe { CreateSolidBrush(COLOR_RAIL) },
            card_brush: unsafe { CreateSolidBrush(COLOR_CARD) },
            option_brush: unsafe { CreateSolidBrush(COLOR_OPTION) },
            accent_brush: unsafe { CreateSolidBrush(COLOR_CYAN) },
            stop_brush: unsafe { CreateSolidBrush(COLOR_CYAN_DARK) },
            border_pen: unsafe { CreatePen(PS_SOLID, scale_value(1, dpi).max(1), COLOR_BORDER) },
            accent_pen: unsafe { CreatePen(PS_SOLID, scale_value(2, dpi).max(1), COLOR_CYAN) },
            title_font,
            section_font,
            body_font,
            small_font,
            button_font,
            value_font,
            power_font,
            nav_font,
            large_icon: null_mut(),
            small_icon: null_mut(),
        };

        state.load_application_icon();
        state.hotkey_registered = unsafe {
            RegisterHotKey(window, HOTKEY_ID, MOD_NOREPEAT, VK_F6) != 0
        };
        state.timer_registered = unsafe { SetTimer(window, TIMER_ID, 100, None) != 0 };
        state.update_mode_controls();
        state.update_idle_status();
        state.repaint();
        state
    }

    fn toggle_clicking(&mut self) {
        if self.worker.is_some() {
            self.finish_worker(false);
            return;
        }

        let speed = if self.is_maximum {
            SpeedMode::Maximum
        } else {
            match parse_cps(&read_text(self.cps_edit)) {
                Ok(rate) => SpeedMode::FixedCps(rate),
                Err(error) => {
                    show_message(self.window, &error.to_string(), MB_OK | MB_ICONWARNING);
                    return;
                }
            }
        };
        let button = self.mouse_button;

        self.click_count.store(0, Ordering::Relaxed);
        self.last_sample_clicks = 0;
        self.last_sample_at = Instant::now();
        self.worker_failed.store(false, Ordering::Relaxed);
        self.running.store(true, Ordering::Release);

        let running = Arc::clone(&self.running);
        let click_count = Arc::clone(&self.click_count);
        let worker_failed = Arc::clone(&self.worker_failed);
        match thread::Builder::new()
            .name("autoclicker-input".to_owned())
            .spawn(move || click_worker(speed, button, running, click_count, worker_failed))
        {
            Ok(worker) => {
                self.worker = Some(worker);
                self.set_running_controls(true);
                self.set_status("● ЗАПУЩЕНО");
                self.repaint();
            }
            Err(_) => {
                self.running.store(false, Ordering::Release);
                self.set_running_controls(false);
                self.set_status("● ОШИБКА ПОТОКА");
                show_message(
                    self.window,
                    "Не удалось запустить рабочий поток автокликера.",
                    MB_OK | MB_ICONERROR,
                );
            }
        }
    }

    fn finish_worker(&mut self, shutting_down: bool) {
        self.running.store(false, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let failed = self.worker_failed.swap(false, Ordering::AcqRel);
        self.set_running_controls(false);

        if !shutting_down {
            if failed {
                self.set_status("● ОШИБКА · ПРАВА");
            } else {
                self.update_idle_status();
            }
            self.repaint();
        }
    }

    fn refresh(&mut self) {
        let clicks = self.click_count.load(Ordering::Relaxed);
        self.set_counter(&format!("Кликов: {clicks}"));

        if self.worker.is_some() && self.running.load(Ordering::Acquire) {
            let now = Instant::now();
            let elapsed = now.saturating_duration_since(self.last_sample_at);
            if elapsed >= CPS_SAMPLE_WINDOW {
                let delta = clicks.saturating_sub(self.last_sample_clicks);
                let cps = (u128::from(delta) * 1_000_000_000 / elapsed.as_nanos()) as u64;
                self.set_status(&format!("● {cps} CPS"));
                self.last_sample_clicks = clicks;
                self.last_sample_at = now;
            }
        }

        if self.worker.is_some() && !self.running.load(Ordering::Acquire) {
            self.finish_worker(false);
        }
    }

    fn update_mode_controls(&self) {
        let is_running = self.worker.is_some();
        unsafe {
            EnableWindow(self.cps_edit, win_bool(!self.is_maximum && !is_running));
            EnableWindow(self.maximum_radio, win_bool(!is_running));
            EnableWindow(self.custom_radio, win_bool(!is_running));
            EnableWindow(self.left_radio, win_bool(!is_running));
            EnableWindow(self.right_radio, win_bool(!is_running));
        }
        self.repaint();
    }

    fn select_option(&mut self, command_id: u16) {
        if self.worker.is_some() {
            return;
        }
        match command_id {
            ID_MAXIMUM => self.is_maximum = true,
            ID_CUSTOM => self.is_maximum = false,
            ID_LEFT => self.mouse_button = MouseButton::Left,
            ID_RIGHT => self.mouse_button = MouseButton::Right,
            _ => return,
        }
        self.update_mode_controls();
    }

    fn set_running_controls(&self, is_running: bool) {
        set_control_text(
            self.start_button,
            if is_running {
                "Остановить · F6"
            } else {
                "Запустить · F6"
            },
        );
        unsafe {
            EnableWindow(self.maximum_radio, win_bool(!is_running));
            EnableWindow(self.custom_radio, win_bool(!is_running));
            EnableWindow(
                self.cps_edit,
                win_bool(!is_running && !self.is_maximum),
            );
            EnableWindow(self.left_radio, win_bool(!is_running));
            EnableWindow(self.right_radio, win_bool(!is_running));
        }
        self.repaint();
    }

    fn update_idle_status(&self) {
        let status = if self.hotkey_registered {
            "● ГОТОВ · F6"
        } else {
            "● ГОТОВ · КНОПКА"
        };
        self.set_status(status);
    }

    fn set_status(&self, text: &str) {
        set_control_text(self.status_label, text);
    }

    fn set_counter(&self, text: &str) {
        set_control_text(self.counter_label, text);
    }

    fn repaint(&self) {
        unsafe {
            InvalidateRect(self.window, null(), 1);
            for control in [
                self.maximum_radio,
                self.custom_radio,
                self.left_radio,
                self.right_radio,
                self.start_button,
            ] {
                InvalidateRect(control, null(), 0);
            }
        }
    }

    fn paint(&self) {
        unsafe {
            let px = |value| scale_value(value, self.dpi);
            let mut paint: PaintStruct = zeroed();
            let hdc = BeginPaint(self.window, &mut paint);
            let client = Rect {
                left: 0,
                top: 0,
                right: px(980),
                bottom: px(620),
            };
            FillRect(hdc, &client, self.background_brush);

            let left_panel = Rect {
                left: px(68),
                top: 0,
                right: px(500),
                bottom: px(620),
            };
            FillRect(hdc, &left_panel, self.left_brush);
            let navigation = Rect {
                left: 0,
                top: 0,
                right: px(68),
                bottom: px(620),
            };
            FillRect(hdc, &navigation, self.rail_brush);
            let divider = Rect {
                left: px(498),
                top: 0,
                right: px(500),
                bottom: px(620),
            };
            FillRect(hdc, &divider, self.card_brush);

            let old_brush = SelectObject(hdc, self.card_brush as HGDIOBJ);
            let old_pen = SelectObject(hdc, self.border_pen as HGDIOBJ);
            RoundRect(hdc, px(88), px(118), px(482), px(324), px(16), px(16));
            RoundRect(hdc, px(88), px(336), px(482), px(446), px(16), px(16));
            RoundRect(hdc, px(596), px(26), px(884), px(68), px(12), px(12));
            SelectObject(hdc, old_pen);
            SelectObject(hdc, old_brush);

            let edit_brush = SelectObject(hdc, self.option_brush as HGDIOBJ);
            let edit_pen = SelectObject(hdc, self.border_pen as HGDIOBJ);
            RoundRect(hdc, px(112), px(258), px(462), px(310), px(10), px(10));
            SelectObject(hdc, edit_pen);
            SelectObject(hdc, edit_brush);

            let nav_brush = SelectObject(hdc, self.card_brush as HGDIOBJ);
            let nav_pen = SelectObject(hdc, self.border_pen as HGDIOBJ);
            RoundRect(hdc, px(10), px(126), px(58), px(174), px(12), px(12));
            SelectObject(hdc, nav_pen);
            SelectObject(hdc, nav_brush);
            let active_mark = Rect {
                left: 0,
                top: px(137),
                right: px(3),
                bottom: px(163),
            };
            FillRect(hdc, &active_mark, self.accent_brush);

            if !self.large_icon.is_null() {
                DrawIconEx(hdc, px(16), px(18), self.large_icon, px(36), px(36), 0, null_mut(), 0);
            }
            let old_font = SelectObject(hdc, self.nav_font as HGDIOBJ);
            SetTextColor(hdc, COLOR_CYAN);
            let power_glyph = wide("⏻");
            let mut nav_icon_rect = Rect {
                left: px(10),
                top: px(126),
                right: px(58),
                bottom: px(174),
            };
            DrawTextW(
                hdc,
                power_glyph.as_ptr(),
                -1,
                &mut nav_icon_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );
            SetTextColor(hdc, COLOR_MUTED);
            let mut info_rect = Rect {
                left: px(18),
                top: px(566),
                right: px(50),
                bottom: px(598),
            };
            let info_glyph = wide("ⓘ");
            DrawTextW(
                hdc,
                info_glyph.as_ptr(),
                -1,
                &mut info_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );
            SelectObject(hdc, old_font);
            EndPaint(self.window, &paint);
        }
    }

    fn color_static(&self, hdc: HDC, control: HWND) -> HBRUSH {
        let control_id = unsafe { GetDlgCtrlID(control) } as u16;
        if control_id == ID_CPS_EDIT {
            unsafe {
                SetBkMode(hdc, GDI_OPAQUE);
                SetBkColor(hdc, COLOR_CARD);
                SetTextColor(hdc, COLOR_MUTED);
            }
            return self.card_brush;
        }

        let color = match control_id {
            ID_HEADING | ID_SPEED_LABEL | ID_MOUSE_LABEL | ID_TARGET => COLOR_TEXT,
            ID_STATUS => COLOR_CYAN,
            _ => COLOR_MUTED,
        };
        unsafe {
            SetBkMode(hdc, GDI_TRANSPARENT);
            SetTextColor(hdc, color);
        }
        self.background_brush
    }

    fn color_edit(&self, hdc: HDC) -> HBRUSH {
        unsafe {
            SetBkMode(hdc, GDI_OPAQUE);
            SetTextColor(hdc, COLOR_TEXT);
            SetBkColor(hdc, COLOR_CARD);
        }
        self.card_brush
    }

    fn draw_item(&self, draw_item: *const DrawItemStruct) {
        if draw_item.is_null() {
            return;
        }
        let item = unsafe { &*draw_item };
        let id = item.ctl_id as u16;
        let hdc = item.hdc;
        let bounds = item.rc_item;
        let px = |value| scale_value(value, self.dpi);

        unsafe {
            let old_font = SelectObject(hdc, self.button_font as HGDIOBJ);
            SetBkMode(hdc, GDI_TRANSPARENT);

            if id == ID_START {
                let running = self.worker.is_some();
                let pressed = item.item_state & ODS_SELECTED != 0;
                let outer_brush = if running {
                    self.accent_brush
                } else if pressed {
                    self.stop_brush
                } else {
                    self.option_brush
                };
                let outer_pen = if running { self.accent_pen } else { self.border_pen };
                let old_brush = SelectObject(hdc, outer_brush as HGDIOBJ);
                let old_pen = SelectObject(hdc, outer_pen as HGDIOBJ);
                Ellipse(
                    hdc,
                    bounds.left + px(2),
                    bounds.top + px(2),
                    bounds.right - px(2),
                    bounds.bottom - px(2),
                );

                SelectObject(hdc, self.background_brush as HGDIOBJ);
                SelectObject(hdc, self.border_pen as HGDIOBJ);
                Ellipse(
                    hdc,
                    bounds.left + px(12),
                    bounds.top + px(12),
                    bounds.right - px(12),
                    bounds.bottom - px(12),
                );

                SelectObject(hdc, self.card_brush as HGDIOBJ);
                Ellipse(
                    hdc,
                    bounds.left + px(26),
                    bounds.top + px(26),
                    bounds.right - px(26),
                    bounds.bottom - px(26),
                );

                SelectObject(hdc, self.option_brush as HGDIOBJ);
                let center_pen = if running { self.accent_pen } else { self.border_pen };
                SelectObject(hdc, center_pen as HGDIOBJ);
                Ellipse(
                    hdc,
                    bounds.left + px(44),
                    bounds.top + px(44),
                    bounds.right - px(44),
                    bounds.bottom - px(44),
                );

                let center_x = (bounds.left + bounds.right) / 2;
                let center_y = (bounds.top + bounds.bottom) / 2;
                SelectObject(hdc, self.power_font as HGDIOBJ);
                SetTextColor(hdc, COLOR_CYAN);
                let power_glyph = wide("⏻");
                let mut icon_rect = Rect {
                    left: center_x - px(48),
                    top: center_y - px(76),
                    right: center_x + px(48),
                    bottom: center_y + px(8),
                };
                DrawTextW(
                    hdc,
                    power_glyph.as_ptr(),
                    -1,
                    &mut icon_rect,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );

                SelectObject(hdc, self.button_font as HGDIOBJ);
                SetTextColor(hdc, COLOR_TEXT);
                let caption = wide(if running { "ОСТАНОВИТЬ" } else { "НАЖМИТЕ ДЛЯ СТАРТА" });
                let mut caption_rect = Rect {
                    left: bounds.left + px(48),
                    top: center_y + px(20),
                    right: bounds.right - px(48),
                    bottom: center_y + px(46),
                };
                DrawTextW(
                    hdc,
                    caption.as_ptr(),
                    -1,
                    &mut caption_rect,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );

                let mode_text = if self.is_maximum {
                    "МАКСИМУМ".to_owned()
                } else {
                    let requested = read_text(self.cps_edit);
                    let short_rate = if requested.len() > 10 {
                        format!("{}…", &requested[..9])
                    } else {
                        requested
                    };
                    format!("{short_rate} CPS")
                };
                SelectObject(hdc, self.small_font as HGDIOBJ);
                SetTextColor(hdc, COLOR_MUTED);
                let mode_text = wide(&mode_text);
                let mut mode_rect = Rect {
                    left: bounds.left + px(52),
                    top: center_y + px(48),
                    right: bounds.right - px(52),
                    bottom: center_y + px(70),
                };
                DrawTextW(
                    hdc,
                    mode_text.as_ptr(),
                    -1,
                    &mut mode_rect,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );
                SelectObject(hdc, old_pen);
                SelectObject(hdc, old_brush);
            } else {
                let (caption, selected) = match id {
                    ID_MAXIMUM => ("Максимум", self.is_maximum),
                    ID_CUSTOM => ("Своя частота", !self.is_maximum),
                    ID_LEFT => ("Левая", matches!(self.mouse_button, MouseButton::Left)),
                    ID_RIGHT => ("Правая", matches!(self.mouse_button, MouseButton::Right)),
                    _ => ("", false),
                };
                let disabled = item.item_state & ODS_DISABLED != 0;
                let pressed = item.item_state & ODS_SELECTED != 0;
                let fill = if pressed {
                    self.stop_brush
                } else if selected && disabled {
                    self.stop_brush
                } else if selected {
                    self.accent_brush
                } else {
                    self.option_brush
                };
                let pen = if selected { self.accent_pen } else { self.border_pen };
                let old_brush = SelectObject(hdc, fill as HGDIOBJ);
                let old_pen = SelectObject(hdc, pen as HGDIOBJ);
                RoundRect(
                    hdc,
                    bounds.left + px(1),
                    bounds.top + px(1),
                    bounds.right - px(1),
                    bounds.bottom - px(1),
                    px(12),
                    px(12),
                );
                SetTextColor(
                    hdc,
                    if selected && !disabled {
                        COLOR_BACKGROUND
                    } else if disabled {
                        COLOR_MUTED
                    } else {
                        COLOR_TEXT
                    },
                );
                let caption = wide(caption);
                let mut text_rect = Rect {
                    left: bounds.left + px(8),
                    top: bounds.top,
                    right: bounds.right - px(8),
                    bottom: bounds.bottom,
                };
                DrawTextW(
                    hdc,
                    caption.as_ptr(),
                    -1,
                    &mut text_rect,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );
                SelectObject(hdc, old_pen);
                SelectObject(hdc, old_brush);
            }

            if id != ID_START && item.item_state & ODS_FOCUS != 0 {
                let mut focus_rect = bounds;
                focus_rect.left += px(4);
                focus_rect.top += px(4);
                focus_rect.right -= px(4);
                focus_rect.bottom -= px(4);
                DrawFocusRect(hdc, &focus_rect);
            }
            SelectObject(hdc, old_font);
        }
    }

    fn load_application_icon(&mut self) {
        let Some(icon_path) = find_icon_path() else {
            return;
        };
        let path_wide: Vec<u16> = icon_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        self.large_icon = unsafe {
            LoadImageW(
                null_mut(),
                path_wide.as_ptr(),
                IMAGE_ICON,
                32,
                32,
                LR_LOADFROMFILE,
            )
        };
        self.small_icon = unsafe {
            LoadImageW(
                null_mut(),
                path_wide.as_ptr(),
                IMAGE_ICON,
                16,
                16,
                LR_LOADFROMFILE,
            )
        };

        if !self.large_icon.is_null() {
            unsafe {
                SendMessageW(
                    self.window,
                    WM_SETICON,
                    ICON_BIG,
                    self.large_icon as isize,
                );
            }
        }
        if !self.small_icon.is_null() {
            unsafe {
                SendMessageW(
                    self.window,
                    WM_SETICON,
                    ICON_SMALL,
                    self.small_icon as isize,
                );
            }
        }
    }

    fn shutdown(&mut self) {
        self.finish_worker(true);
        if self.hotkey_registered {
            unsafe {
                UnregisterHotKey(self.window, HOTKEY_ID);
            }
            self.hotkey_registered = false;
        }
        if self.timer_registered {
            unsafe {
                KillTimer(self.window, TIMER_ID);
            }
            self.timer_registered = false;
        }
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        unsafe {
            if !self.large_icon.is_null() {
                DestroyIcon(self.large_icon);
            }
            if !self.small_icon.is_null() && self.small_icon != self.large_icon {
                DestroyIcon(self.small_icon);
            }
            for object in [
                self.background_brush as HGDIOBJ,
                self.left_brush as HGDIOBJ,
                self.rail_brush as HGDIOBJ,
                self.card_brush as HGDIOBJ,
                self.option_brush as HGDIOBJ,
                self.accent_brush as HGDIOBJ,
                self.stop_brush as HGDIOBJ,
                self.border_pen as HGDIOBJ,
                self.accent_pen as HGDIOBJ,
                self.title_font as HGDIOBJ,
                self.section_font as HGDIOBJ,
                self.body_font as HGDIOBJ,
                self.small_font as HGDIOBJ,
                self.button_font as HGDIOBJ,
                self.value_font as HGDIOBJ,
                self.power_font as HGDIOBJ,
                self.nav_font as HGDIOBJ,
            ] {
                if !object.is_null() {
                    DeleteObject(object);
                }
            }
        }
    }
}

fn click_worker(
    speed: SpeedMode,
    button: MouseButton,
    running: Arc<AtomicBool>,
    click_count: Arc<AtomicU64>,
    worker_failed: Arc<AtomicBool>,
) {
    let priority = match &speed {
        SpeedMode::Maximum => THREAD_PRIORITY_ABOVE_NORMAL,
        SpeedMode::FixedCps(rate) if rate.interval().is_zero() => THREAD_PRIORITY_ABOVE_NORMAL,
        SpeedMode::FixedCps(_) => THREAD_PRIORITY_NORMAL,
    };
    unsafe {
        SetThreadPriority(GetCurrentThread(), priority);
    }
    match speed {
        SpeedMode::Maximum => run_at_maximum(button, &running, &click_count, &worker_failed),
        SpeedMode::FixedCps(rate) => {
            let interval = rate.interval();
            if interval.is_zero() {
                run_at_maximum(button, &running, &click_count, &worker_failed);
            } else {
                run_at_fixed_rate(button, interval, &running, &click_count, &worker_failed);
            }
        }
    }
    running.store(false, Ordering::Release);
}

fn run_at_maximum(
    button: MouseButton,
    running: &AtomicBool,
    click_count: &AtomicU64,
    worker_failed: &AtomicBool,
) {
    let events = maximum_click_batch(button);
    let event_total = events.len() as u32;

    while running.load(Ordering::Acquire) {
        let sent = unsafe { SendInput(event_total, events.as_ptr(), size_of::<Input>() as i32) };
        let completed_clicks = sent / 2;
        if completed_clicks != 0 {
            click_count.fetch_add(u64::from(completed_clicks), Ordering::Relaxed);
        }
        if sent != event_total {
            release_if_needed(button, sent);
            worker_failed.store(true, Ordering::Release);
            break;
        }
    }
}

fn run_at_fixed_rate(
    button: MouseButton,
    interval: Duration,
    running: &AtomicBool,
    click_count: &AtomicU64,
    worker_failed: &AtomicBool,
) {
    let events = click_events(button);
    let mut deadline = Instant::now();

    while running.load(Ordering::Acquire) {
        if !wait_until(deadline, running) || !running.load(Ordering::Acquire) {
            break;
        }

        let sent = unsafe { SendInput(2, events.as_ptr(), size_of::<Input>() as i32) };
        if sent == 2 {
            click_count.fetch_add(1, Ordering::Relaxed);
        } else {
            release_if_needed(button, sent);
            worker_failed.store(true, Ordering::Release);
            break;
        }

        deadline += interval;
        let now = Instant::now();
        if deadline <= now {
            // Do not issue catch-up bursts after the OS or target application falls behind.
            deadline = now + interval;
        }
    }
}

fn wait_until(deadline: Instant, running: &AtomicBool) -> bool {
    loop {
        if !running.load(Ordering::Acquire) {
            return false;
        }
        let now = Instant::now();
        if now >= deadline {
            return true;
        }

        let remaining = deadline.saturating_duration_since(now);
        if remaining > SPIN_WINDOW {
            let sleep_for = remaining
                .saturating_sub(SPIN_WINDOW)
                .min(MAX_SLEEP_SLICE);
            thread::sleep(sleep_for);
        } else {
            std::hint::spin_loop();
        }
    }
}

fn click_events(button: MouseButton) -> [Input; 2] {
    let (down, up) = button_flags(button);
    [mouse_input(down), mouse_input(up)]
}

fn maximum_click_batch(button: MouseButton) -> [Input; MAX_BATCH_CLICKS * 2] {
    let [down, up] = click_events(button);
    let mut events = [down; MAX_BATCH_CLICKS * 2];
    for click in 0..MAX_BATCH_CLICKS {
        events[click * 2] = down;
        events[click * 2 + 1] = up;
    }
    events
}

fn button_flags(button: MouseButton) -> (u32, u32) {
    match button {
        MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
    }
}

fn mouse_input(flags: u32) -> Input {
    Input {
        input_type: INPUT_MOUSE,
        data: InputData {
            mouse: MouseInput {
                dx: 0,
                dy: 0,
                mouse_data: 0,
                flags,
                time: 0,
                extra_info: 0,
            },
        },
    }
}

fn release_if_needed(button: MouseButton, sent_events: u32) {
    if sent_events % 2 == 1 {
        let (_, up_flag) = button_flags(button);
        let release = mouse_input(up_flag);
        unsafe {
            SendInput(1, &release, size_of::<Input>() as i32);
        }
    }
}

fn system_dpi() -> u32 {
    let dc = unsafe { GetDC(null_mut()) };
    if dc.is_null() {
        return BASE_DPI;
    }
    let dpi = unsafe { GetDeviceCaps(dc, LOGPIXELSX) };
    unsafe {
        ReleaseDC(null_mut(), dc);
    }
    if dpi > 0 { dpi as u32 } else { BASE_DPI }
}

fn scale_value(value: i32, dpi: u32) -> i32 {
    let scaled = i64::from(value) * i64::from(dpi);
    let rounding = if scaled >= 0 { 48 } else { -48 };
    ((scaled + rounding) / i64::from(BASE_DPI)) as i32
}

fn create_ui_font(height: i32, weight: i32, dpi: u32) -> HFONT {
    create_ui_font_family(height, weight, dpi, "Segoe UI")
}

fn create_ui_font_family(height: i32, weight: i32, dpi: u32, family: &str) -> HFONT {
    let face_name = wide(family);
    unsafe {
        CreateFontW(
            scale_value(height, dpi),
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            DEFAULT_PITCH | FF_DONTCARE,
            face_name.as_ptr(),
        )
    }
}

fn create_control(
    parent: HWND,
    instance: HINSTANCE,
    dpi: u32,
    class_name: &str,
    text: &str,
    style: u32,
    ex_style: u32,
    id: u16,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> HWND {
    let class_name = wide(class_name);
    let text = wide(text);
    let x = scale_value(x, dpi);
    let y = scale_value(y, dpi);
    let width = scale_value(width, dpi);
    let height = scale_value(height, dpi);
    unsafe {
        CreateWindowExW(
            ex_style,
            class_name.as_ptr(),
            text.as_ptr(),
            style,
            x,
            y,
            width,
            height,
            parent,
            id as usize as HMENU,
            instance,
            null_mut(),
        )
    }
}

fn set_control_text(control: HWND, text: &str) {
    let text = wide(text);
    unsafe {
        SetWindowTextW(control, text.as_ptr());
    }
}

fn read_text(control: HWND) -> String {
    let length = unsafe { GetWindowTextLengthW(control) }.max(0) as usize;
    let mut buffer = vec![0u16; length + 1];
    let written = unsafe { GetWindowTextW(control, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..written.max(0) as usize])
}

fn win_bool(value: bool) -> BOOL {
    if value { 1 } else { 0 }
}

fn show_message(owner: HWND, message: &str, flags: UINT) {
    let message = wide(message);
    let title = wide("AutoClicker");
    unsafe {
        MessageBoxW(owner, message.as_ptr(), title.as_ptr(), flags);
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn resource_id(id: usize) -> *const u16 {
    id as *const u16
}

unsafe fn app_state_ptr(window: HWND) -> *mut AppState {
    unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) as *mut AppState }
}

fn find_icon_path() -> Option<PathBuf> {
    let mut buffer = vec![0u16; 32_768];
    let length = unsafe {
        GetModuleFileNameW(
            null_mut(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
        )
    } as usize;
    if length == 0 || length >= buffer.len() {
        return None;
    }

    let executable = PathBuf::from(OsString::from_wide(&buffer[..length]));
    let mut directory = executable.parent()?.to_path_buf();
    loop {
        let icon = directory.join("assets").join("autoclicker.ico");
        if icon.is_file() {
            return Some(icon);
        }
        if !directory.pop() {
            return None;
        }
    }
}
