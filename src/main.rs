use game::Cell;
use libremarkable::appctx::ApplicationContext;
use libremarkable::cgmath::{Point2, Vector2};
use libremarkable::framebuffer::common::{
    color, display_temp, dither_mode, mxcfb_rect, waveform_mode, DRAWING_QUANT_BIT,
};
use libremarkable::framebuffer::refresh::PartialRefreshMode;
use libremarkable::framebuffer::{FramebufferDraw, FramebufferRefresh};
use libremarkable::input::wacom::WacomPen;
use libremarkable::input::{wacom::WacomEvent, InputEvent};
use libremarkable::ui_extensions::element::{UIConstraintRefresh, UIElement};
use log::*;
use once_cell::sync::Lazy;
use std::error::Error;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

static X_OFFSET: i32 = 30;
static Y_OFFSET: i32 = 264;
static REAL_FIELD_SIZE: u32 = 1344;

static GAME_RUNNING: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
static GAME_SPEED: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(500));
static DRAW_MODE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(true));

mod game;
fn on_wacom_input(ctx: &mut ApplicationContext, input: WacomEvent, game: Arc<RwLock<game::Frame>>) {
    match input {
        WacomEvent::Draw { position, .. } => {
            if let Some((x, y)) = get_cell_from_absolute(position) {
                let new_state = if DRAW_MODE.load(Ordering::Relaxed) {
                    Cell::Alive
                } else {
                    Cell::Dead
                };
                let needs_update = {
                    let mut game = game.write().unwrap();
                    game.set_cell(x, y, new_state.clone())
                };
                if needs_update {
                    draw_cell(ctx, (x, y), new_state);
                    partial_update(
                        ctx,
                        &mxcfb_rect {
                            left: x * 32 + X_OFFSET as u32,
                            top: y * 32 + Y_OFFSET as u32,
                            width: 32,
                            height: 32,
                        },
                    )
                }
            }
        }
        WacomEvent::InstrumentChange { pen, state: _ } => match pen {
            WacomPen::ToolPen => {
                DRAW_MODE.store(true, Ordering::Relaxed);
            }
            WacomPen::ToolRubber => {
                DRAW_MODE.store(false, Ordering::Relaxed);
            }
            _ => {}
        },
        _ => {}
    }
}

fn get_cell_from_absolute(position: Point2<f32>) -> Option<(u32, u32)> {
    let x = position.x.floor() as u32;
    let y = position.y.floor() as u32;

    if x >= X_OFFSET as u32
        && X_OFFSET as u32 + REAL_FIELD_SIZE >= x
        && y >= Y_OFFSET as u32
        && Y_OFFSET as u32 + REAL_FIELD_SIZE >= y
    {
        Some(((x - X_OFFSET as u32) / 32, (y - Y_OFFSET as u32) / 32))
    } else {
        None
    }
}

fn draw_cell(app: &mut ApplicationContext, (x, y): (u32, u32), cell: game::Cell) {
    let fb = app.get_framebuffer_ref();
    let cell_color = match cell {
        game::Cell::Dead => color::WHITE,
        game::Cell::Alive => color::BLACK,
    };
    fb.fill_rect(
        Point2 {
            x: x as i32 * 32 + X_OFFSET + 3,
            y: y as i32 * 32 + Y_OFFSET + 3,
        },
        Vector2 { x: 26, y: 26 },
        cell_color,
    );
    partial_update(
        app,
        &mxcfb_rect {
            left: x * 32 + X_OFFSET as u32,
            top: y * 32 + Y_OFFSET as u32,
            width: 32,
            height: 32,
        },
    )
}

fn draw_grid(app: &mut ApplicationContext, game: Arc<RwLock<game::Frame>>) {
    // With a playing field of 1344px
    let fb = app.get_framebuffer_ref();
    let game = game.read().unwrap();
    for x in 0..42 {
        for y in 0..42 {
            fb.draw_rect(
                Point2 {
                    x: x * 32 + X_OFFSET,
                    y: y * 32 + Y_OFFSET,
                },
                Vector2 { x: 32, y: 32 },
                1,
                color::GRAY(100),
            );

            let cell_color = match game.get_cell(x as u32, y as u32) {
                game::Cell::Dead => color::WHITE,
                game::Cell::Alive => color::BLACK,
            };
            fb.fill_rect(
                Point2 {
                    x: x * 32 + X_OFFSET + 3,
                    y: y * 32 + Y_OFFSET + 3,
                },
                Vector2 { x: 26, y: 26 },
                cell_color,
            );
        }
    }

    refresh_canvas(app);
}

fn main() -> Result<(), Box<dyn Error>> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();
    debug!("starting...");
    let mut app = ApplicationContext::default();
    app.clear(true);

    let game = game::Frame::new(42, 42);
    let game_handle = Arc::new(RwLock::new(game));
    let game_thread = {
        let game_handle = game_handle.clone();
        let appref = app.upgrade_ref();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(GAME_SPEED.load(Ordering::Relaxed)));
            if GAME_RUNNING.load(Ordering::Relaxed) {
                let mut game = game_handle.write().unwrap();
                let diffs = game.step();
                for (x, y, cell) in diffs {
                    draw_cell(appref, (x, y), cell);
                }
            }
        })
    };
    draw_grid(&mut app, game_handle.clone());

    app.add_element(
        "toggleGame",
        libremarkable::ui_extensions::element::UIElementWrapper {
            position: Point2 { x: 20, y: 50 },
            refresh: UIConstraintRefresh::Refresh,
            onclick: Some(|_, _| {
                let running = GAME_RUNNING.load(Ordering::Relaxed);
                GAME_RUNNING.store(!running, Ordering::Relaxed)
            }),
            inner: UIElement::Text {
                text: "Toggle Game".to_owned(),
                scale: 50.,
                foreground: color::BLACK,
                border_px: 5,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "gameSpeedPlusPlus",
        libremarkable::ui_extensions::element::UIElementWrapper {
            position: Point2 { x: 300, y: 50 },
            refresh: UIConstraintRefresh::Refresh,
            onclick: Some(|_, _| {
                let speed = GAME_SPEED.load(Ordering::Relaxed);
                GAME_SPEED.store(speed + 50, Ordering::Relaxed)
            }),
            inner: UIElement::Text {
                text: "++".to_owned(),
                scale: 50.,
                foreground: color::BLACK,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "gameSpeedPlus",
        libremarkable::ui_extensions::element::UIElementWrapper {
            position: Point2 { x: 400, y: 50 },
            refresh: UIConstraintRefresh::Refresh,
            onclick: Some(|_, _| {
                let speed = GAME_SPEED.load(Ordering::Relaxed);
                GAME_SPEED.store(speed + 10, Ordering::Relaxed)
            }),
            inner: UIElement::Text {
                text: "+".to_owned(),
                scale: 50.,
                foreground: color::BLACK,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "gameSpeedMinus",
        libremarkable::ui_extensions::element::UIElementWrapper {
            position: Point2 { x: 450, y: 50 },
            refresh: UIConstraintRefresh::Refresh,
            onclick: Some(|_, _| {
                let speed = GAME_SPEED.load(Ordering::Relaxed);
                GAME_SPEED.store(speed - 10, Ordering::Relaxed)
            }),
            inner: UIElement::Text {
                text: "-".to_owned(),
                scale: 50.,
                foreground: color::BLACK,
                border_px: 0,
            },
            ..Default::default()
        },
    );
    app.add_element(
        "gameSpeedMinusMinus",
        libremarkable::ui_extensions::element::UIElementWrapper {
            position: Point2 { x: 500, y: 50 },
            refresh: UIConstraintRefresh::Refresh,
            onclick: Some(|_, _| {
                let speed = GAME_SPEED.load(Ordering::Relaxed);
                GAME_SPEED.store(speed - 50, Ordering::Relaxed)
            }),
            inner: UIElement::Text {
                text: "--".to_owned(),
                scale: 50.,
                foreground: color::BLACK,
                border_px: 0,
            },
            ..Default::default()
        },
    );

    app.draw_elements();
    // Blocking call to process events from digitizer + touchscreen + physical buttons
    app.start_event_loop(true, true, true, |ctx, evt| match evt {
        InputEvent::WacomEvent { event } => on_wacom_input(ctx, event, game_handle.clone()),
        _e => {
            //debug!("{e:?}")
        }
    });
    game_thread.join().unwrap();

    Ok(())
}

fn refresh_canvas(app: &mut ApplicationContext) {
    let fb = app.get_framebuffer_ref();
    fb.partial_refresh(
        &mxcfb_rect {
            top: Y_OFFSET as u32,
            left: 0,
            width: 1404,
            height: 1344,
        },
        PartialRefreshMode::Async,
        waveform_mode::WAVEFORM_MODE_AUTO,
        display_temp::TEMP_USE_REMARKABLE_DRAW,
        dither_mode::EPDC_FLAG_EXP1,
        DRAWING_QUANT_BIT,
        false,
    );
}

fn partial_update(app: &mut ApplicationContext, rect: &mxcfb_rect) {
    let fb = app.get_framebuffer_ref();
    fb.partial_refresh(
        rect,
        PartialRefreshMode::Async,
        waveform_mode::WAVEFORM_MODE_DU,
        display_temp::TEMP_USE_REMARKABLE_DRAW,
        dither_mode::EPDC_FLAG_EXP1,
        DRAWING_QUANT_BIT,
        false,
    );
}
