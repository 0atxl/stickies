use gtk::gdk::Display;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

const APPLICATION_ID: &str = "dev.stickies.Stickies";
const LAYER_NAMESPACE: &str = "stickies";

fn main() -> gtk::glib::ExitCode {
    let application = gtk::Application::builder()
        .application_id(APPLICATION_ID)
        .build();

    application.connect_activate(build_edge_surface);
    application.run()
}

fn build_edge_surface(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .default_width(8)
        .default_height(180)
        .decorated(false)
        .resizable(false)
        .build();

    window.init_layer_shell();
    window.set_namespace(Some(LAYER_NAMESPACE));
    window.set_layer(Layer::Overlay);
    window.set_anchor(Edge::Right, true);
    window.set_exclusive_zone(0);
    window.set_keyboard_mode(KeyboardMode::None);

    let marker = gtk::Box::new(gtk::Orientation::Vertical, 0);
    marker.add_css_class("edge-marker");
    marker.set_size_request(8, 180);
    window.set_child(Some(&marker));

    let styles = gtk::CssProvider::new();
    styles.load_from_data(".edge-marker { background: #f2c14e; border-radius: 4px 0 0 4px; }");
    gtk::style_context_add_provider_for_display(
        &Display::default().expect("GTK display must be available"),
        &styles,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    window.present();
}
