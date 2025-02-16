use dioxus::prelude::*;
use walkdir::WalkDir;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS } document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        div { "testing"}
        Hero {}
    }
}

#[component]
pub fn Hero() -> Element {
    let images = use_signal(|| {
        WalkDir::new("/home/david/Desktop/pics")
            .into_iter()
            .filter_map(|e| e.ok()) // Ignore errors
            .filter(|e| e.file_type().is_file()) // Only get files
            .filter_map(|e| e.path().to_str().map(|s| s.to_string()))
            .collect::<Vec<_>>()
    });
    let mut index = use_signal(|| 0);

    let mut previous = move || {
        let mut i = index.write();
        if *i == 0 {
            *i = images.read().len() - 1; // Wrap around to last image
        } else {
            *i -= 1;
        }
    };

    let mut next = move || {
        let mut i = index.write();
        *i = (*i + 1) % images.read().len(); // Wrap around to first image
    };

    let handle_key_down_event = move |evt: KeyboardEvent| match evt.key() {
        Key::ArrowLeft => previous(),
        Key::ArrowRight => next(),
        _ => {}
    };

    rsx! {
        div {
            tabindex: "0",
            onkeydown: handle_key_down_event,
            div { "{images.read()[*index.read()]} {index}"}
            img {
                src: images.read()[index()].clone(),
                style: "max-width: 80vw; max-height: 80vh; margin-top: 20px;"
            }
            // for (i, image) in images().iter().enumerate() {
            //     img {
            //         key: "{image}",
            //         style: if index() == i { "display: block;" } else { "display: none;" },
            //         src: images.read()[i].clone(),
            //         style: "max-width: 80vw; max-height: 80vh; margin-top: 20px;"
            //     }
            // }
        }
    }
}
