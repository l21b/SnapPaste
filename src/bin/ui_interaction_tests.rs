use super::*;
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, PlatformError, WindowAdapter, WindowEvent};
use slint::{LogicalPosition, Model, PhysicalSize};
use std::cell::Cell;

struct TestPlatform {
    window: Rc<MinimalSoftwareWindow>,
    started_at: std::time::Instant,
    time_offset: Rc<Cell<Duration>>,
}
impl Platform for TestPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> Duration {
        self.started_at.elapsed() + self.time_offset.get()
    }
}

#[test]
fn visible_controls_receive_pointer_events() {
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    let time_offset = Rc::new(Cell::new(Duration::ZERO));
    slint::platform::set_platform(Box::new(TestPlatform {
        window: window.clone(),
        started_at: std::time::Instant::now(),
        time_offset: time_offset.clone(),
    }))
    .unwrap();
    let ui = MainWindow::new().unwrap();
    window.set_size(PhysicalSize::new(264, 421));
    ui.show().unwrap();
    let render = || {
        ui.window().request_redraw();
        let mut pixels = vec![slint::Rgb8Pixel::default(); 264 * 421];
        window.draw_if_needed(|renderer| {
            renderer.render(&mut pixels, 264);
        });
    };
    let click = |x, y| {
        render();
        let position = LogicalPosition::new(x, y);
        ui.window()
            .dispatch_event(WindowEvent::PointerMoved { position });
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position,
            button: slint::platform::PointerEventButton::Left,
        });
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position,
            button: slint::platform::PointerEventButton::Left,
        });
    };
    let drags = Rc::new(Cell::new(0));
    let observed = drags.clone();
    ui.on_window_drag_requested(move || observed.set(observed.get() + 1));
    // A newly shown popup must accept typing without an activation click.
    ui.invoke_focus_search();
    ui.window()
        .dispatch_event(WindowEvent::KeyPressed { text: "x".into() });
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text: "x".into() });
    assert_eq!(
        ui.get_search_text(),
        "x",
        "typing immediately after opening"
    );
    ui.set_search_text("".into());
    let toggled = Rc::new(Cell::new(0));
    let observed = toggled.clone();
    ui.on_favorites_toggled(move || observed.set(observed.get() + 1));
    click(229., 31.);
    assert_eq!(toggled.get(), 1, "favorite toolbar button");
    assert_eq!(
        drags.get(),
        0,
        "toolbar clicks must not start native dragging"
    );
    click(186., 31.);
    assert_eq!(ui.get_active_panel(), "add", "add toolbar button");
    click(138., 285.);
    assert_eq!(ui.get_active_panel(), "main", "cancel add dialog");
    click(144., 31.);
    assert_eq!(ui.get_active_panel(), "clear", "clear toolbar button");
    ui.set_active_panel("main".into());
    ui.set_records(ModelRc::new(VecModel::from(vec![ClipboardItemData {
        id: "7".into(),
        content: "click me".into(),
        type_label: "文本".into(),
        ..Default::default()
    }])));
    // Hover remains visible over row content and controls without taking text focus.
    let row_pixel = || {
        render();
        std::thread::sleep(Duration::from_millis(180));
        slint::platform::update_timers_and_animations();
        window.request_redraw();
        let mut pixels = vec![slint::Rgb8Pixel::default(); 264 * 421];
        window.draw_if_needed(|renderer| {
            renderer.render(&mut pixels, 264);
        });
        pixels[190 * 264 + 10]
    };
    ui.invoke_focus_search();
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: LogicalPosition::new(50., 95.),
    });
    let normal = row_pixel();
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: LogicalPosition::new(50., 175.),
    });
    let hovered = row_pixel();
    assert!(
        hovered.b as i16 - hovered.r as i16 > 8,
        "row hover must be visibly blue"
    );
    assert_ne!(normal, hovered);
    for x in [184., 214., 244.] {
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(x, 151.),
        });
        assert_eq!(
            row_pixel(),
            hovered,
            "hover remains while crossing action buttons"
        );
    }
    ui.window()
        .dispatch_event(WindowEvent::KeyPressed { text: "x".into() });
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text: "x".into() });
    assert_eq!(
        ui.get_search_text(),
        "x",
        "hover must not steal search focus"
    );
    ui.set_search_text("".into());
    let activated = Rc::new(RefCell::new(String::new()));
    let observed = activated.clone();
    ui.on_record_activated(move |id| *observed.borrow_mut() = id.to_string());
    click(50., 175.);
    assert_eq!(&*activated.borrow(), "7", "record click");
    let actions = Rc::new(RefCell::new(Vec::new()));
    let observed = actions.clone();
    ui.on_pinned_changed(move |id, _| observed.borrow_mut().push(format!("pin:{id}")));
    let observed = actions.clone();
    ui.on_record_removed(move |id| observed.borrow_mut().push(format!("delete:{id}")));
    let observed = actions.clone();
    ui.on_favorite_changed(move |id, _| observed.borrow_mut().push(format!("favorite:{id}")));
    activated.borrow_mut().clear();
    for (x, action) in [(184., "pin:7"), (214., "delete:7"), (244., "favorite:7")] {
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(50., 175.),
        });
        render();
        for _ in 0..6 {
            ui.window().dispatch_event(WindowEvent::PointerMoved {
                position: LogicalPosition::new(x, 151.),
            });
            render();
        }
        click(x, 151.);
        assert_eq!(
            actions.borrow().last().map(String::as_str),
            Some(action),
            "action remains clickable while hovered"
        );
        assert!(
            activated.borrow().is_empty(),
            "action must not paste the record"
        );
    }
    // Enter directly from the search field, then cross between adjacent actions.
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: LogicalPosition::new(184., 95.),
    });
    render();
    let before = actions.borrow().len();
    for (index, (x, action)) in [(184., "pin:7"), (214., "delete:7"), (244., "favorite:7")]
        .into_iter()
        .enumerate()
    {
        click(x, 151.);
        assert_eq!(actions.borrow().len(), before + index + 1);
        assert_eq!(actions.borrow().last().map(String::as_str), Some(action));
        assert!(activated.borrow().is_empty());
    }
    ui.set_active_panel("settings".into());
    // A dropdown must open with the first click, without a separate focus click.
    ui.set_settings_theme("light".into());
    click(120., 182.);
    click(90., 281.);
    assert_eq!(
        ui.get_settings_theme(),
        "dark",
        "first click opens theme menu"
    );
    click(70., 118.);
    click(90., 348.);
    assert_eq!(
        ui.get_settings_modifier(),
        "Ctrl+Alt+Shift",
        "first click opens modifier menu"
    );
    assert!(ui.get_settings_hotkey().starts_with("Ctrl+Alt+Shift+"));
    click(228., 43.);
    assert_eq!(ui.get_active_panel(), "main", "settings close");

    // Finishing a native drag also restores clicks without hiding the window.
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position: LogicalPosition::new(50., 31.),
        button: slint::platform::PointerEventButton::Left,
    });
    cancel_pointer_capture(ui.window());
    let before = toggled.get();
    click(229., 31.);
    assert_eq!(toggled.get(), before + 1, "click after drag completion");

    // Native title-bar dragging consumes the release instead of returning it to Slint.
    ui.window().dispatch_event(WindowEvent::PointerPressed {
        position: LogicalPosition::new(50., 31.),
        button: slint::platform::PointerEventButton::Left,
    });
    ui.hide().unwrap();
    show_window(&ui).unwrap();
    let before = toggled.get();
    click(229., 31.);
    assert_eq!(
        toggled.get(),
        before + 1,
        "first click after native drag and reopen"
    );

    // Windows can discard the native pixels on hide without changing buffer age.
    // Match the Windows backend full-frame policy while keeping the same renderer.
    use slint::platform::software_renderer::PremultipliedRgbaColor;
    let mut pixels = vec![PremultipliedRgbaColor::default(); 264 * 421];
    window.set_size(PhysicalSize::new(264, 421));
    window.draw_if_needed(|renderer| {
        renderer.render(&mut pixels, 264);
    });
    ui.set_search_text("ss".into());
    window.request_redraw();
    window.draw_if_needed(|renderer| {
        renderer.set_repaint_buffer_type(RepaintBufferType::NewBuffer);
        renderer.render(&mut pixels, 264);
    });
    for _ in 0..3 {
        ui.hide().unwrap();
        pixels.fill(PremultipliedRgbaColor::default());
        show_window(&ui).unwrap();
        ui.invoke_focus_search();
        window.request_redraw();
        window.draw_if_needed(|renderer| {
            renderer.render(&mut pixels, 264);
        });
        assert!(
            pixels[10 * 264 + 10].alpha > 0,
            "header must repaint after reopen"
        );
        assert!(
            pixels[400 * 264 + 10].alpha > 0,
            "list background must repaint after reopen"
        );
    }
    // Losing pixels AFTER the first show repaint was missed by the old one-shot fix.
    // A small hover/caret update still needs to restore the entire native surface.
    for _ in 0..20 {
        pixels.fill(PremultipliedRgbaColor::default());
        ui.window().dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(145., 31.),
        });
        window.request_redraw();
        window.draw_if_needed(|renderer| {
            let region = renderer.render(&mut pixels, 264);
            assert_eq!(region.bounding_box_size(), PhysicalSize::new(264, 421));
        });
        assert!(
            pixels[10 * 264 + 10].alpha > 0,
            "header after late surface loss"
        );
        assert!(
            pixels[400 * 264 + 10].alpha > 0,
            "list after late surface loss"
        );
    }

    // Reopening clears both the field and its filter; an empty search does no work.
    let searches = Rc::new(Cell::new(0));
    let observed = searches.clone();
    let query = Rc::new(RefCell::new(String::new()));
    let observed_query = query.clone();
    let weak = ui.as_weak();
    ui.on_search_changed(move |keyword| {
        observed.set(observed.get() + 1);
        *observed_query.borrow_mut() = keyword.to_string();
        let ui = weak.upgrade().unwrap();
        let records = if keyword.is_empty() {
            vec![ClipboardItemData {
                id: "restored".into(),
                ..Default::default()
            }]
        } else {
            vec![]
        };
        ui.set_records(ModelRc::new(VecModel::from(records)));
    });
    for favorites in [false, true] {
        ui.set_favorites_only(favorites);
        ui.set_search_text("missing".into());
        ui.invoke_search_changed("missing".into());
        ui.hide().unwrap();
        reset_search_for_reopen(&ui);
        show_window(&ui).unwrap();
        ui.invoke_focus_search();
        assert!(ui.get_search_text().is_empty());
        assert!(query.borrow().is_empty());
        assert_eq!(ui.get_records().row_count(), 1, "unfiltered list restored");
        assert_eq!(ui.get_favorites_only(), favorites, "keep the current tab");
        let before = searches.get();
        reset_search_for_reopen(&ui);
        assert_eq!(searches.get(), before, "empty search avoids another reload");
        ui.window()
            .dispatch_event(WindowEvent::KeyPressed { text: "x".into() });
        ui.window()
            .dispatch_event(WindowEvent::KeyReleased { text: "x".into() });
        assert_eq!(ui.get_search_text(), "x", "typing immediately after reset");
    }

    // Toasts expire without clicks, and newer messages get their own full duration.
    let advance_time = |duration| {
        time_offset.set(time_offset.get() + duration);
        slint::platform::update_timers_and_animations();
        render();
    };
    ui.set_status_text("已清除 8 条记录".into());
    advance_time(Duration::ZERO);
    advance_time(Duration::from_millis(2000));
    assert_eq!(ui.get_status_text(), "已清除 8 条记录");
    ui.set_status_text("收藏已添加".into());
    advance_time(Duration::ZERO);
    advance_time(Duration::from_millis(1500));
    assert_eq!(
        ui.get_status_text(),
        "收藏已添加",
        "old deadline must not clear new toast"
    );
    advance_time(Duration::from_millis(1600));
    assert!(
        ui.get_status_text().is_empty(),
        "toast expires without another click"
    );
    ui.set_status_text("已清除 8 条记录".into());
    advance_time(Duration::ZERO);
    advance_time(Duration::from_millis(3100));
    assert!(
        ui.get_status_text().is_empty(),
        "timer restarts after an earlier expiry"
    );

    // Focus loss hides immediately, including directly after show: no timer tick
    // or startup grace period is needed. Focus gain never hides the popup.
    for _ in 0..10 {
        show_window(&ui).unwrap();
        ui.set_active_panel("main".into());
        handle_window_focus(&ui, true);
        assert!(ui.window().is_visible());
        handle_window_focus(&ui, false);
        assert!(
            !ui.window().is_visible(),
            "focus loss must hide synchronously"
        );
    }
    for panel in ["settings", "about"] {
        show_window(&ui).unwrap();
        ui.set_active_panel(panel.into());
        handle_window_focus(&ui, false);
        assert!(
            ui.window().is_visible(),
            "persistent panels survive focus loss"
        );
    }
    for panel in ["main", "add", "clear"] {
        show_window(&ui).unwrap();
        ui.set_active_panel(panel.into());
        handle_window_focus(&ui, false);
        assert!(
            !ui.window().is_visible(),
            "transient panels hide on focus loss"
        );
    }
}
