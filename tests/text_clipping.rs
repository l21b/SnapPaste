use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, PlatformError, WindowAdapter};
use slint::{ComponentHandle, PhysicalSize};
use std::rc::Rc;

slint::slint! {
    export component TextProbe inherits Window {
        width: 180px;
        height: 80px;
        background: #ffffff;
        default-font-family: "Segoe UI";
        Text { x: 10px; y: 10px; width: 150px; height: 24px; text: "快捷键 Alt 0"; font-size: 12px; color: #000000; }
        Text { x: 10.75px; y: 42px; width: 150px; height: 24px; text: "快捷键 Alt 0"; font-size: 12px; color: #000000; }
    }
}

struct TestPlatform(Rc<MinimalSoftwareWindow>);
impl Platform for TestPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.0.clone())
    }
}

#[test]
fn fractional_text_origin_preserves_the_same_glyph_pixels() {
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(TestPlatform(window.clone()))).unwrap();
    let ui = TextProbe::new().unwrap();
    ui.show().unwrap();
    for scale in [1.0_f32, 1.25, 1.5] {
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged {
                scale_factor: scale,
            });
        let width = (180.0 * scale) as usize;
        let height = (80.0 * scale) as usize;
        window.set_size(PhysicalSize::new(width as u32, height as u32));
        window.request_redraw();
        let mut pixels = vec![slint::Rgb8Pixel::default(); width * height];
        window.draw_if_needed(|renderer| {
            renderer.render(&mut pixels, width);
        });
        let x1 = (10.0 * scale).round() as usize;
        let x2 = (10.75 * scale).round() as usize;
        let y1 = (10.0 * scale).round() as usize;
        let y2 = (42.0 * scale).round() as usize;
        let mut ink = 0;
        for y in 0..(23.0 * scale) as usize {
            for x in 0..(140.0 * scale) as usize {
                let integer = pixels[(y1 + y) * width + x1 + x];
                let fractional = pixels[(y2 + y) * width + x2 + x];
                ink += usize::from(integer.r < 200);
                assert_eq!(
                    integer, fractional,
                    "glyph differs at ({x}, {y}), scale={scale}"
                );
            }
        }
        assert!(ink > 20, "the comparison must contain rendered text");
    }
}
