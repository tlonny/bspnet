mod app;

use app::App;

const WINDOW_TITLE: &str = "BSPNet";

fn main() {
    App::new().run(WINDOW_TITLE);
}
