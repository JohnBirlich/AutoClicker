#![allow(non_camel_case_types)]

use crate::clicker::{interval_for_cps, parse_cps, SpeedMode};
use std::ffi::{c_void, OsString};
use std::mem::size_of;
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
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
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
    fn RegisterClassW(class: *const WndClassW) -> u16;
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
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(module_name: *const u16) -> HINSTANCE;
    fn GetModuleFileNameW(module: HINSTANCE, file_name: *mut u16, size: UINT) -> UINT;
}

#[link(name = "gdi32")]
extern "system" {
    fn GetStockObject(object: i32) -> *mut c_void;
}

const CS_VREDRAW: u32 = 0x0001;
const CS_HREDRAW: u32 = 0x0002;
const WS_CAPTION: u32 = 0x00C0_0000;
const WS_SYSMENU: u32 = 0x0008_0000;
const WS_MINIMIZEBOX: u32 = 0x0002_0000;
const WS_CHILD: u32 = 0x4000_0000;
const WS_VISIBLE: u32 = 0x1000_0000;
const WS_TABSTOP: u32 = 0x0001_0000;
const WS_GROUP: u32 = 0x0002_0000;
const WS_EX_CLIENTEDGE: u32 = 0x0000_0200;
const BS_DEFPUSHBUTTON: u32 = 0x0000_0001;
const BS_AUTORADIOBUTTON: u32 = 0x0000_0009;
const ES_AUTOHSCROLL: u32 = 0x0000_0080;
const ES_NUMBER: u32 = 0x0000_2000;
const COLOR_WINDOW: u32 = 5;
const DEFAULT_GUI_FONT: i32 = 17;
const SW_SHOW: i32 = 5;
const CW_USEDEFAULT: i32 = 0x8000_0000u32 as i32;

const WM_CREATE: UINT = 0x0001;
const WM_DESTROY: UINT = 0x0002;
const WM_CLOSE: UINT = 0x0010;
const WM_SETFONT: UINT = 0x0030;
const WM_SETICON: UINT = 0x0080;
const WM_NCDESTROY: UINT = 0x0082;
const WM_TIMER: UINT = 0x0113;
const WM_COMMAND: UINT = 0x0111;
const WM_HOTKEY: UINT = 0x0312;
const BM_GETCHECK: UINT = 0x00F0;
const BM_SETCHECK: UINT = 0x00F1;
const BN_CLICKED: u16 = 0;
const BST_CHECKED: usize = 1;
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
const INPUT_MOUSE: u32 = 0;
const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
const MOUSEEVENTF_RIGHTDOWN: u32 = 0x0008;
const MOUSEEVENTF_RIGHTUP: u32 = 0x0010;

const HOTKEY_ID: i32 = 1;
const TIMER_ID: usize = 1;
const ID_MAXIMUM: u16 = 1001;
const ID_CUSTOM: u16 = 1002;
const ID_LEFT: u16 = 1003;
const ID_RIGHT: u16 = 1004;
const ID_START: u16 = 1005;
const MAX_BATCH_CLICKS: usize = 16;
const SPIN_WINDOW: Duration = Duration::from_micros(300);
const MAX_SLEEP_SLICE: Duration = Duration::from_millis(5);

#[derive(Clone, Copy)]
enum MouseButton {
    Left,
    Right,
}

struct AppState {
    window: HWND,
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
    worker: Option<JoinHandle<()>>,
    hotkey_registered: bool,
    timer_registered: bool,
    large_icon: HICON,
    small_icon: HICON,
}

pub fn run() {
    unsafe {
        let instance = GetModuleHandleW(null());
        let class_name = wide("AutoClickerMainWindow");
        let window_title = wide("AutoClicker");
        let embedded_icon = LoadIconW(instance, resource_id(1));
        let class_icon = if embedded_icon.is_null() {
            LoadIconW(null_mut(), resource_id(32512))
        } else {
            embedded_icon
        };
        let window_class = WndClassW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfn_wnd_proc: Some(window_proc),
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance: instance,
            h_icon: class_icon,
            h_cursor: LoadCursorW(null_mut(), resource_id(32512)),
            hbr_background: (COLOR_WINDOW as usize + 1) as HBRUSH,
            lpsz_menu_name: null(),
            lpsz_class_name: class_name.as_ptr(),
        };

        if RegisterClassW(&window_class) == 0 {
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
            right: 480,
            bottom: 380,
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
            let state = Box::new(AppState::new(window, instance));
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
                        ID_MAXIMUM | ID_CUSTOM => state.update_mode_controls(),
                        ID_LEFT | ID_RIGHT => {}
                        _ => {}
                    }
                }
            }
            0
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
    fn new(window: HWND, instance: HINSTANCE) -> Self {
        let heading = create_control(
            window,
            instance,
            "STATIC",
            "AutoClicker",
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            24,
            18,
            430,
            32,
        );
        let subtitle = create_control(
            window,
            instance,
            "STATIC",
            "Быстрые клики мышью — управление клавишей F6",
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            24,
            54,
            430,
            22,
        );
        let speed_label = create_control(
            window,
            instance,
            "STATIC",
            "Скорость",
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            24,
            94,
            160,
            20,
        );
        let maximum_radio = create_control(
            window,
            instance,
            "BUTTON",
            "Максимум",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | BS_AUTORADIOBUTTON,
            0,
            ID_MAXIMUM,
            24,
            120,
            145,
            26,
        );
        let custom_radio = create_control(
            window,
            instance,
            "BUTTON",
            "Своя частота",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_AUTORADIOBUTTON,
            0,
            ID_CUSTOM,
            184,
            120,
            160,
            26,
        );
        let cps_label = create_control(
            window,
            instance,
            "STATIC",
            "Кликов в секунду:",
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            24,
            158,
            132,
            24,
        );
        let cps_edit = create_control(
            window,
            instance,
            "EDIT",
            "1000",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_NUMBER | ES_AUTOHSCROLL,
            WS_EX_CLIENTEDGE,
            0,
            164,
            155,
            106,
            28,
        );
        let cps_range = create_control(
            window,
            instance,
            "STATIC",
            "от 1 до 100000",
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            282,
            159,
            170,
            24,
        );
        let button_label = create_control(
            window,
            instance,
            "STATIC",
            "Кнопка мыши",
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            24,
            205,
            160,
            20,
        );
        let left_radio = create_control(
            window,
            instance,
            "BUTTON",
            "Левая",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_GROUP | BS_AUTORADIOBUTTON,
            0,
            ID_LEFT,
            24,
            230,
            110,
            26,
        );
        let right_radio = create_control(
            window,
            instance,
            "BUTTON",
            "Правая",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_AUTORADIOBUTTON,
            0,
            ID_RIGHT,
            145,
            230,
            110,
            26,
        );
        let start_button = create_control(
            window,
            instance,
            "BUTTON",
            "Запустить (F6)",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_DEFPUSHBUTTON,
            0,
            ID_START,
            24,
            278,
            210,
            44,
        );
        let status_label = create_control(
            window,
            instance,
            "STATIC",
            "Остановлен",
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            256,
            278,
            204,
            24,
        );
        let counter_label = create_control(
            window,
            instance,
            "STATIC",
            "Кликов: 0",
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            256,
            304,
            204,
            24,
        );
        let hint = create_control(
            window,
            instance,
            "STATIC",
            "Максимум использует CPU; остановка — F6 или кнопкой.",
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            24,
            345,
            440,
            20,
        );

        let font = unsafe { GetStockObject(DEFAULT_GUI_FONT) };
        for control in [
            heading,
            subtitle,
            speed_label,
            maximum_radio,
            custom_radio,
            cps_label,
            cps_edit,
            cps_range,
            button_label,
            left_radio,
            right_radio,
            start_button,
            status_label,
            counter_label,
            hint,
        ] {
            unsafe {
                SendMessageW(control, WM_SETFONT, font as usize, 1);
            }
        }
        unsafe {
            SendMessageW(maximum_radio, BM_SETCHECK, BST_CHECKED, 0);
            SendMessageW(left_radio, BM_SETCHECK, BST_CHECKED, 0);
        }

        let mut state = Self {
            window,
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
            worker: None,
            hotkey_registered: false,
            timer_registered: false,
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
        state
    }

    fn toggle_clicking(&mut self) {
        if self.worker.is_some() {
            self.finish_worker(false);
            return;
        }

        let speed = if is_checked(self.maximum_radio) {
            SpeedMode::Maximum
        } else {
            let cps = read_text(self.cps_edit);
            match parse_cps(&cps) {
                Ok(cps) => SpeedMode::FixedCps(cps),
                Err(error) => {
                    show_message(self.window, &error.to_string(), MB_OK | MB_ICONWARNING);
                    return;
                }
            }
        };
        let button = if is_checked(self.left_radio) {
            MouseButton::Left
        } else {
            MouseButton::Right
        };

        self.click_count.store(0, Ordering::Relaxed);
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
                match speed {
                    SpeedMode::Maximum => self.set_status("Работает · максимальная скорость"),
                    SpeedMode::FixedCps(cps) => {
                        self.set_status(&format!("Работает · {cps} клик/с"));
                    }
                }
            }
            Err(_) => {
                self.running.store(false, Ordering::Release);
                self.set_running_controls(false);
                self.set_status("Не удалось запустить поток кликов");
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
                self.set_status("Ошибка отправки ввода (права/цель)");
            } else {
                self.update_idle_status();
            }
        }
    }

    fn refresh(&mut self) {
        let clicks = self.click_count.load(Ordering::Relaxed);
        self.set_counter(&format!("Кликов: {clicks}"));

        if self.worker.is_some() && !self.running.load(Ordering::Acquire) {
            self.finish_worker(false);
        }
    }

    fn update_mode_controls(&self) {
        let custom_mode = is_checked(self.custom_radio);
        let is_running = self.worker.is_some();
        unsafe {
            EnableWindow(self.cps_edit, win_bool(custom_mode && !is_running));
            EnableWindow(self.maximum_radio, win_bool(!is_running));
            EnableWindow(self.custom_radio, win_bool(!is_running));
            EnableWindow(self.left_radio, win_bool(!is_running));
            EnableWindow(self.right_radio, win_bool(!is_running));
        }
    }

    fn set_running_controls(&self, is_running: bool) {
        set_control_text(
            self.start_button,
            if is_running {
                "Остановить (F6)"
            } else {
                "Запустить (F6)"
            },
        );
        unsafe {
            EnableWindow(self.maximum_radio, win_bool(!is_running));
            EnableWindow(self.custom_radio, win_bool(!is_running));
            EnableWindow(
                self.cps_edit,
                win_bool(!is_running && is_checked(self.custom_radio)),
            );
            EnableWindow(self.left_radio, win_bool(!is_running));
            EnableWindow(self.right_radio, win_bool(!is_running));
        }
    }

    fn update_idle_status(&self) {
        let status = if self.hotkey_registered {
            "Остановлен · F6 — старт/стоп"
        } else {
            "Остановлен · используйте кнопку"
        };
        self.set_status(status);
    }

    fn set_status(&self, text: &str) {
        set_control_text(self.status_label, text);
    }

    fn set_counter(&self, text: &str) {
        set_control_text(self.counter_label, text);
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
    match speed {
        SpeedMode::Maximum => run_at_maximum(button, &running, &click_count, &worker_failed),
        SpeedMode::FixedCps(cps) => run_at_fixed_rate(
            button,
            cps,
            &running,
            &click_count,
            &worker_failed,
        ),
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
    cps: u32,
    running: &AtomicBool,
    click_count: &AtomicU64,
    worker_failed: &AtomicBool,
) {
    let Some(interval) = interval_for_cps(cps) else {
        worker_failed.store(true, Ordering::Release);
        return;
    };
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

fn create_control(
    parent: HWND,
    instance: HINSTANCE,
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
    let mut buffer = [0u16; 64];
    let length = unsafe { GetWindowTextW(control, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

fn is_checked(control: HWND) -> bool {
    unsafe { SendMessageW(control, BM_GETCHECK, 0, 0) == BST_CHECKED as isize }
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
