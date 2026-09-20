#![cfg(all(target_os = "linux", feature = "uinput-output"))]

use inputforge_core::{
    output::uinput::{Output, default_config},
    types::VJoyAxis,
};

#[test]
fn configuration_validation_and_replacement_need_no_hardware() {
    let mut output = Output::new(vec![default_config(16), default_config(1)]).unwrap();
    assert!(!output.is_active());
    assert_eq!(output.configs()[0].device_id, 1);
    assert!(output.configure(vec![default_config(0)]).is_err());
    assert_eq!(output.configs().len(), 2);
    for id in [0, 17, 255] {
        assert_eq!(
            Output::new(vec![default_config(id)]).unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput
        );
    }
    assert_eq!(
        Output::new(vec![]).unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
    assert_eq!(
        Output::new(vec![default_config(1), default_config(1)])
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidInput
    );
    for buttons in [0, 1, 54, 128, 255] {
        let mut config = default_config(1);
        config.button_count = buttons;
        assert_eq!(
            Output::new(vec![config]).unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput
        );
    }
    let mut config = default_config(1);
    config.hat_count = 5;
    assert_eq!(
        Output::new(vec![config]).unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
    let mut config = default_config(1);
    config.axes = vec![VJoyAxis::X, VJoyAxis::X];
    assert_eq!(
        Output::new(vec![config]).unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
    let mut buttons_only = default_config(1);
    buttons_only.axes.clear();
    buttons_only.hat_count = 0;
    buttons_only.button_count = 2;
    output.configure(vec![buttons_only]).unwrap();
    output.release().unwrap();
    output.release().unwrap();
    assert!(output.set_button(1, 1, true).is_err());
    assert!(output.flush().is_err());
}
