use aura_editor::App;
use iced_test::simulator;

#[test]
fn hello_world_shows_greeting_and_responds_to_click() {
    let mut app = App::new();
    let mut ui = simulator(app.view());

    assert!(
        ui.find("Hello, Aura").is_ok(),
        "greeting should be visible"
    );
    assert!(ui.find("Ready").is_ok(), "initial status should be visible");

    ui.click("Click me")
        .expect("primary button should be clickable");

    for message in ui.into_messages() {
        app.update(message);
    }

    assert_eq!(app.status(), "Button clicked");

    let mut ui = simulator(app.view());
    assert!(
        ui.find("Button clicked").is_ok(),
        "updated status should be visible after click"
    );
}
