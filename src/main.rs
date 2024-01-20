// This hides the console window when launching cube.exe,
// at the cost of suppressing println! statements.
/* #![windows_subsystem = "windows"] */
#![feature(lazy_cell)]

mod camera;
mod constants;
mod game_state;
mod gpu_state;
mod texture;

use crate::constants::{MIN_TIME_PER_RENDER_FRAME, TIME_PER_GAME_TICK};
use crate::game_state::{GameState, InputState};
use crate::gpu_state::WebGPUState;

use pollster::block_on;
use std::collections::VecDeque;
use std::mem::{self};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self};
use std::time::{Duration, Instant};
use windows::Win32::UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_LEFT, VK_RIGHT};
use windows::Win32::{Foundation::POINT, System::LibraryLoader::GetModuleHandleA};
use windows::{
    core::*,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::ValidateRect,
        UI::WindowsAndMessaging::*,
    },
};

const EVENT_QUEUE_SIZE_IN_BYTES: i32 = std::mem::size_of::<*mut Arc<Mutex<EventQueue>>>() as i32;

fn main() -> windows::core::Result<()> {
    println!("Hello world!");
    let hinstance = unsafe { GetModuleHandleA(None) }?;
    let window_class_name = s!("window");
    let wc = WNDCLASSA {
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }?,
        hInstance: hinstance.into(),
        lpszClassName: window_class_name,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        cbWndExtra: 2 * EVENT_QUEUE_SIZE_IN_BYTES,
        ..Default::default()
    };
    let atom = unsafe { RegisterClassA(&wc) };
    debug_assert!(atom != 0);

    let window = unsafe {
        CreateWindowExA(
            WINDOW_EX_STYLE::default(),
            window_class_name,
            s!("My sample window"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            500,
            500,
            None,
            None,
            hinstance,
            None,
        )
    };

    // These will get manipulated directly by wndproc.
    let gpu_event_queue = Arc::new(Mutex::new(EventQueue::new()));
    let input_event_queue = Arc::new(Mutex::new(EventQueue::new()));
    unsafe {
        SetWindowLongPtrA(
            window,
            WINDOW_LONG_PTR_INDEX(0),
            &gpu_event_queue as *const Arc<Mutex<EventQueue>> as isize,
        )
    };
    unsafe {
        SetWindowLongPtrA(
            window,
            WINDOW_LONG_PTR_INDEX(EVENT_QUEUE_SIZE_IN_BYTES),
            &input_event_queue as *const Arc<Mutex<EventQueue>> as isize,
        )
    };

    let mut gpu_state: WebGPUState = block_on(WebGPUState::new(window, hinstance.into()));
    let mut game_state = GameState::new(1.0);
    let mut input_state = InputState::new();
    let (tx, rx) = mpsc::channel();
    macro_rules! printUnexpected {
        ($event_name:expr) => {
            println!(
                "Unexpected occurrence: {} event was created with incorrect EventData",
                $event_name
            );
        };
    }
    {
        let gpu_event_queue = Arc::clone(&gpu_event_queue);
        let _gpu_thread = thread::spawn(move || {
            let mut last_render = Instant::now();
            let _ = gpu_state.render();

            let mut last_fps_print = last_render;
            let mut frames = 0;
            loop {
                {
                    let mut queue = gpu_event_queue.lock().unwrap();
                    while !(*queue).is_empty() {
                        let event = (*queue).pop_front().expect("queue somehow empty?");
                        match event.message {
                            WM_MOUSEMOVE => match event.data {
                                EventData::MouseMoveData(point) => {
                                    gpu_state.update_bg_color(&point);
                                }
                                _ => {
                                    printUnexpected!("WM_MOUSEMOVE");
                                }
                            },
                            WM_PAINT => match event.data {
                                EventData::EmptyData() => {}
                                _ => {
                                    printUnexpected!("WM_PAINT");
                                }
                            },
                            WM_SIZE => match event.data {
                                EventData::ResizeData(rect) => {
                                    gpu_state.resize(rect);
                                }
                                _ => {
                                    printUnexpected!("WM_SIZE");
                                }
                            },
                            _ => (),
                        }
                    }
                }
                let mut game_state_res = rx.try_recv();
                if game_state_res.is_ok() {
                    let mut next = rx.try_recv();
                    while next.is_ok() {
                        game_state_res = next;
                        next = rx.try_recv();
                    }
                    let game_state: GameState = game_state_res.unwrap();
                    gpu_state.update_camera(game_state.get_camera());
                }
                if Instant::now() >= last_fps_print + Duration::from_secs(2) {
                    println!("FPS = {}", frames as f32 / 2.0);
                    frames = 0;
                    last_fps_print = Instant::now();
                }
                let next = Instant::now();
                if next >= last_render + *MIN_TIME_PER_RENDER_FRAME {
                    last_render = next;
                    frames += 1;
                    let _ = gpu_state.render();
                }
            }
        });
    }
    {
        let input_event_queue = Arc::clone(&input_event_queue);
        let _game_thread = thread::spawn(move || {
            //assert!(thread_priority::set_current_thread_priority(ThreadPriority::Max).is_ok());
            let mut last_tick = Instant::now();
            loop {
                {
                    let mut queue = input_event_queue.lock().unwrap();
                    while !(*queue).is_empty() {
                        let event = (*queue).pop_front().expect("queue somehow empty?");
                        match event.message {
                            WM_KEYDOWN => match event.data {
                                EventData::KeyDownData(wparam, lparam) => {
                                    let virtual_key = VIRTUAL_KEY(wparam.0 as u16);
                                    match virtual_key {
                                        VK_LEFT => {
                                            // TODO: this does not work as expected. Some internet
                                            // discussions suggest that maybe I need to wait longer
                                            // to see these fields get populated?
                                            // https://stackoverflow.com/questions/44897991/wm-keydown-repeat-count
                                            let key_flags = lparam.0 as u32;
                                            let was_key_already_down: bool =
                                                (key_flags & KF_REPEAT) == KF_REPEAT;
                                            if !was_key_already_down {
                                                input_state.left = true;
                                            }
                                        }
                                        VK_RIGHT => {
                                            let key_flags = lparam.0 as u32;
                                            let was_key_already_down: bool =
                                                (key_flags & KF_REPEAT) == KF_REPEAT;
                                            if !was_key_already_down {
                                                input_state.right = true;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                _ => {
                                    printUnexpected!("WM_KEYDOWN");
                                }
                            },
                            WM_KEYUP => match event.data {
                                EventData::KeyUpData(wparam, _lparam) => {
                                    let virtual_key = VIRTUAL_KEY(wparam.0 as u16);
                                    match virtual_key {
                                        VK_LEFT => {
                                            input_state.left = false;
                                        }
                                        VK_RIGHT => {
                                            input_state.right = false;
                                        }
                                        _ => {}
                                    }
                                }
                                _ => {
                                    printUnexpected!("WM_KEYUP");
                                }
                            },
                            WM_SIZE => match event.data {
                                EventData::ResizeData(rect) => {
                                    let width = rect.right - rect.left;
                                    let height = rect.bottom - rect.top;
                                    game_state.change_camera_aspect(width as f32 / height as f32);
                                }
                                _ => {
                                    printUnexpected!("WM_SIZE");
                                }
                            },
                            _ => (),
                        }
                    }
                }
                let current_time = Instant::now();
                // I am assuming it's rare or impossible for current_time - last_tick to be more
                // than two frames. That means I expect this loop to always run to 0 or 1
                // iterations.
                //
                // If the time window does span multiple frames, I just pass the same input on
                // every frame.
                while current_time - last_tick >= *TIME_PER_GAME_TICK {
                    last_tick = last_tick + *TIME_PER_GAME_TICK;
                    game_state.update(&input_state, last_tick);
                }
                let _ = tx.send(game_state.clone());
                thread::sleep(Duration::from_millis(5));
            }
        });
    }

    let mut message = MSG::default();
    unsafe {
        while GetMessageA(&mut message, None, 0, 0).into() {
            DispatchMessageA(&message);
        }
    }

    Ok(())
}

#[derive(Clone, Copy)]
struct WindowsEvent {
    message: u32,
    data: EventData,
}

#[derive(Clone, Copy)]
enum EventData {
    ResizeData(RECT),
    EmptyData(),
    MouseMoveData(POINT),
    KeyDownData(WPARAM, LPARAM),
    KeyUpData(WPARAM, LPARAM),
}

type EventQueue = VecDeque<WindowsEvent>;

extern "system" fn wndproc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let gpu_queue_ptr = unsafe { GetWindowLongPtrA(window, WINDOW_LONG_PTR_INDEX(0)) }
        as *mut Arc<Mutex<EventQueue>>;
    let input_queue_ptr =
        unsafe { GetWindowLongPtrA(window, WINDOW_LONG_PTR_INDEX(EVENT_QUEUE_SIZE_IN_BYTES)) }
            as *mut Arc<Mutex<EventQueue>>;
    if gpu_queue_ptr.is_null() || input_queue_ptr.is_null() {
        println!("Exiting wndproc early due to null event queues.");
        return unsafe { DefWindowProcA(window, message, wparam, lparam) };
    }
    match message {
        WM_PAINT => {
            println!("WM_PAINT");
            {
                let mut queue = unsafe { (*gpu_queue_ptr).lock().unwrap() };
                (*queue).push_back(WindowsEvent { message, data: EventData::EmptyData() });
            }
            unsafe { ValidateRect(window, None) };
            LRESULT(0)
        }
        WM_DESTROY => {
            println!("WM_DESTROY");
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        WM_SIZE => {
            println!("WM_SIZE");
            let mut rect: RECT = unsafe { mem::zeroed() };
            let _ = unsafe { GetClientRect(window, &mut rect) };
            let event = WindowsEvent { message, data: EventData::ResizeData(rect) };
            {
                let mut gpu_queue = unsafe { (*gpu_queue_ptr).lock().unwrap() };
                (*gpu_queue).push_back(event);
            }
            {
                let mut input_queue = unsafe { (*input_queue_ptr).lock().unwrap() };
                (*input_queue).push_back(event);
            }
            LRESULT(0)
        }
        WM_MOUSEACTIVATE => {
            println!("WM_MOUSEACTIVATE");
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            // println!("WM_MOUSEMOVE");
            let mut pt: POINT = unsafe { mem::zeroed() };
            let _ = unsafe { GetCursorPos(&mut pt) };
            {
                let mut queue = unsafe { (*gpu_queue_ptr).lock().unwrap() };
                (*queue).push_back(WindowsEvent { message, data: EventData::MouseMoveData(pt) });
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            {
                let mut queue = unsafe { (*input_queue_ptr).lock().unwrap() };
                (*queue).push_back(WindowsEvent {
                    message,
                    data: EventData::KeyDownData(wparam, lparam),
                });
            }
            LRESULT(0)
        }
        WM_KEYUP => {
            println!("WM_KEYUP");
            {
                let mut queue = unsafe { (*input_queue_ptr).lock().unwrap() };
                (*queue).push_back(WindowsEvent {
                    message,
                    data: EventData::KeyUpData(wparam, lparam),
                });
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcA(window, message, wparam, lparam) },
    }
}
