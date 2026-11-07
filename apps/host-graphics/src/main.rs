use embedded_graphics as eg;
use embedded_graphics::prelude::*;
use embedded_graphics_simulator as eg_sim;

fn main() -> Result<(), core::convert::Infallible> {
    let mut target = eg_sim::SimulatorDisplay::<eg::pixelcolor::Rgb888>::new(Size::new(320, 240));
    let output_settings = eg_sim::OutputSettingsBuilder::new().scale(2).build();
    let mut window = eg_sim::Window::new("text_fonts", &output_settings);
    window.show_static(&target);
    Ok(())
}
