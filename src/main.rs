// This hides the console window when launching cube.exe,
// at the cost of suppressing println! statements.
/* #![windows_subsystem = "windows"] */

mod camera;
mod game_state;
mod gpu_state;
mod texture;

use crate::game_state::{GameState, InputState};
use crate::gpu_state::WebGPUState;

use pollster::block_on;
use std::collections::VecDeque;
use std::mem::{self};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};
use thread_priority::ThreadPriority;
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

    let mut gpu_event_queue = Arc::new(Mutex::new(EventQueue::new()));
    let mut input_event_queue = Arc::new(Mutex::new(EventQueue::new()));

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
    assert!(thread_priority::set_current_thread_priority(ThreadPriority::Max).is_ok());
    {
        let gpu_event_queue = Arc::clone(&gpu_event_queue);
        let gpu_thread = thread::spawn(move || loop {
            {
                assert!(thread_priority::set_current_thread_priority(ThreadPriority::Max).is_ok());
                let mut queue = gpu_event_queue.lock().unwrap();
                println!("geq len {}", (*queue).len());
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
                            EventData::EmptyData() => {
                                // not necessary anymore, we re-render at the end of every loop
                                let _ = gpu_state.render();
                            }
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
                gpu_state.render();
                thread::sleep(Duration::from_millis(5));
            }
        });
    }
    {
        let input_event_queue = Arc::clone(&input_event_queue);
        let game_thread = thread::spawn(move || {
            assert!(thread_priority::set_current_thread_priority(ThreadPriority::Max).is_ok());
            let mut last_tick = Instant::now();
            const GAME_TICKS_PER_SECOND: f64 = 25.0;
            let TIME_PER_GAME_TICK: Duration = Duration::from_secs_f64(1.0 / GAME_TICKS_PER_SECOND);
            loop {
                {
                    let mut queue = input_event_queue.lock().unwrap();
                    println!("ieq len {}", (*queue).len());
                    while !(*queue).is_empty() {
                        let event = (*queue).pop_front().expect("queue somehow empty?");
                        match event.message {
                            WM_KEYDOWN => match event.data {
                                EventData::KeyDownData(wparam, lparam) => {
                                    let virtual_key = VIRTUAL_KEY(wparam.0 as u16);
                                    match virtual_key {
                                        VK_LEFT => {
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
                                EventData::KeyUpData(wparam, lparam) => {
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
                while current_time - last_tick >= TIME_PER_GAME_TICK {
                    last_tick = last_tick + TIME_PER_GAME_TICK;
                    game_state.update(&input_state, last_tick);
                }
                let _ = tx.send(game_state.clone());
                thread::sleep(Duration::from_millis(5));
            }
        });
    }
    // let game_start = std::time::Instant::now();
    // let mut last_tick = game_start.clone();
    // let mut tick_num = 0;
    // const GAME_TICKS_PER_SECOND: f64 = 1.0;
    // const MS_PER_GAME_TICK: std::time::Duration =
    //   std::time::Duration::from_micros(
    //   ((1000 * 1000) as f64 / GAME_TICKS_PER_SECOND) as u64);
    // let mut accumulated_time = std::time::Duration::from_millis(0);
    // let mut last_instant = game_start;
    // let mut message = MSG::default();
    // let mut keep_running = true;
    // loop {
    //     let next_instant = std::time::Instant::now();
    //     accumulated_time += next_instant - last_instant;
    //     if accumulated_time > MS_PER_GAME_TICK {
    //         tick_num += 1;
    //         println!("GAME_TICK {}", tick_num);
    //         last_tick += MS_PER_GAME_TICK;
    //         accumulated_time = (accumulated_time - MS_PER_GAME_TICK);
    //     }
    //     last_instant = next_instant;
    //     while PeekMessageA(&mut message, None, 0, 0, PM_REMOVE).into() {
    //         if message.message == WM_QUIT {
    //             keep_running = false;
    //             break;
    //         }
    //         DispatchMessageA(&message);
    //     }
    //     if !keep_running {
    //         break;
    //     }
    // }

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
            println!("WM_KEYDOWN");
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
